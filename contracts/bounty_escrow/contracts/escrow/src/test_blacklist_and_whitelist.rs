//! Tests for blacklist / whitelist behavior and participant filter mode events.
//!
//! The allowlist serves two purposes:
//! - It is the source of truth for `AllowlistOnly` participant filtering.
//! - It bypasses anti-abuse cooldown and window checks when filtering is disabled.
//!
//! The blocklist is only enforced when `ParticipantFilterMode::BlocklistOnly` is active.

#![cfg(test)]

use crate::events::ParticipantFilterModeChanged;
use crate::{BountyEscrowContract, BountyEscrowContractClient, Error, ParticipantFilterMode};
use soroban_sdk::testutils::{Address as _, Events, Ledger};
use soroban_sdk::{symbol_short, token, Address, Env, IntoVal, Symbol, TryIntoVal};

struct Setup<'a> {
    env: Env,
    client: BountyEscrowContractClient<'a>,
    admin: Address,
    depositor: Address,
    other: Address,
    token: token::Client<'a>,
}

impl<'a> Setup<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(1_000_000);

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let other = Address::generate(&env);

        let token_admin = Address::generate(&env);
        let token_address = env
            .register_stellar_asset_contract_v2(token_admin)
            .address();
        let token_admin_client = token::StellarAssetClient::new(&env, &token_address);
        let token_client = token::Client::new(&env, &token_address);

        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);
        client.init(&admin, &token_address);

        token_admin_client.mint(&depositor, &10_000);
        token_admin_client.mint(&other, &10_000);

        Self {
            env,
            client,
            admin,
            depositor,
            other,
            token: token_client,
        }
    }

    fn deadline(&self) -> u64 {
        self.env.ledger().timestamp() + 86_400
    }
}

fn last_filter_mode_event(env: &Env) -> ParticipantFilterModeChanged {
    let events = env.events().all();
    for event in events.iter().rev() {
        let topics = event.1;
        if topics.len() < 1 {
            continue;
        }

        let event_type: Symbol = topics.get(0).unwrap().into_val(env);
        if event_type != symbol_short!("pf_mode") {
            continue;
        }

        return event.2.try_into_val(env).expect("participant filter event");
    }

    panic!("expected a ParticipantFilterModeChanged event");
}

#[test]
fn test_non_whitelisted_address_is_rate_limited_by_cooldown() {
    let setup = Setup::new();

    setup.env.as_contract(&setup.client.address, || {
        crate::anti_abuse::set_config(
            &setup.env,
            crate::anti_abuse::AntiAbuseConfig {
                window_size: 3600,
                max_operations: 100,
                cooldown_period: 100,
            },
        );
    });

    let deadline = setup.deadline();
    setup
        .client
        .lock_funds(&setup.depositor, &1, &100, &deadline);

    assert!(setup
        .client
        .try_lock_funds(&setup.depositor, &2, &100, &deadline)
        .is_err());
}

#[test]
fn test_whitelisted_address_bypasses_cooldown_check() {
    let setup = Setup::new();

    setup.env.as_contract(&setup.client.address, || {
        crate::anti_abuse::set_config(
            &setup.env,
            crate::anti_abuse::AntiAbuseConfig {
                window_size: 3600,
                max_operations: 100,
                cooldown_period: 100,
            },
        );
    });
    setup.client.set_whitelist_entry(&setup.depositor, &true);

    let deadline = setup.deadline();
    setup
        .client
        .lock_funds(&setup.depositor, &11, &100, &deadline);
    setup
        .client
        .lock_funds(&setup.depositor, &12, &100, &deadline);

    assert_eq!(setup.token.balance(&setup.client.address), 200);
}

#[test]
fn test_removed_from_whitelist_reenables_rate_limit_checks() {
    let setup = Setup::new();

    setup.env.as_contract(&setup.client.address, || {
        crate::anti_abuse::set_config(
            &setup.env,
            crate::anti_abuse::AntiAbuseConfig {
                window_size: 3600,
                max_operations: 100,
                cooldown_period: 100,
            },
        );
    });
    setup.client.set_whitelist_entry(&setup.depositor, &true);
    setup.client.set_whitelist_entry(&setup.depositor, &false);

    let deadline = setup.deadline();
    setup
        .client
        .lock_funds(&setup.depositor, &21, &100, &deadline);

    assert!(setup
        .client
        .try_lock_funds(&setup.depositor, &22, &100, &deadline)
        .is_err());
}

#[test]
fn test_blocklisted_address_is_rejected_in_blocklist_mode() {
    let setup = Setup::new();

    setup
        .client
        .set_filter_mode(&ParticipantFilterMode::BlocklistOnly);
    setup.client.set_blocklist_entry(&setup.depositor, &true);

    let err = setup
        .client
        .try_lock_funds(&setup.depositor, &31, &100, &setup.deadline())
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::ParticipantBlocked);

    setup
        .client
        .lock_funds(&setup.other, &32, &100, &setup.deadline());
}

#[test]
fn test_allowlist_mode_rejects_non_allowlisted_address() {
    let setup = Setup::new();

    setup
        .client
        .set_filter_mode(&ParticipantFilterMode::AllowlistOnly);
    setup.client.set_whitelist_entry(&setup.depositor, &true);

    setup
        .client
        .lock_funds(&setup.depositor, &41, &100, &setup.deadline());

    let err = setup
        .client
        .try_lock_funds(&setup.other, &42, &100, &setup.deadline())
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::ParticipantNotAllowed);
}

#[test]
fn test_address_present_on_both_lists_follows_active_mode() {
    let setup = Setup::new();

    setup.client.set_whitelist_entry(&setup.depositor, &true);
    setup.client.set_blocklist_entry(&setup.depositor, &true);

    setup
        .client
        .set_filter_mode(&ParticipantFilterMode::AllowlistOnly);
    setup
        .client
        .lock_funds(&setup.depositor, &51, &100, &setup.deadline());

    setup
        .client
        .set_filter_mode(&ParticipantFilterMode::BlocklistOnly);
    let err = setup
        .client
        .try_lock_funds(&setup.depositor, &52, &100, &setup.deadline())
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::ParticipantBlocked);
}

#[test]
fn test_set_filter_mode_emits_expected_event_payload() {
    let setup = Setup::new();

    setup
        .client
        .set_filter_mode(&ParticipantFilterMode::BlocklistOnly);
    let first = last_filter_mode_event(&setup.env);
    assert_eq!(first.previous_mode, ParticipantFilterMode::Disabled);
    assert_eq!(first.new_mode, ParticipantFilterMode::BlocklistOnly);
    assert_eq!(first.admin, setup.admin);
    assert_eq!(first.timestamp, 1_000_000);

    setup
        .client
        .set_filter_mode(&ParticipantFilterMode::AllowlistOnly);
    let second = last_filter_mode_event(&setup.env);
    assert_eq!(second.previous_mode, ParticipantFilterMode::BlocklistOnly);
    assert_eq!(second.new_mode, ParticipantFilterMode::AllowlistOnly);
    assert_eq!(second.admin, setup.admin);
    assert_eq!(second.timestamp, 1_000_000);
}
