#![cfg(test)]
//! Parity tests: lock, release, refund, and edge cases (double release, double refund).
//! Behavior aligned with main contracts/bounty_escrow where applicable.

use super::*;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{testutils::Events, token, Address, Env, String, Symbol};

fn create_token<'a>(
    env: &'a Env,
    admin: &Address,
) -> (Address, token::Client<'a>, token::StellarAssetClient<'a>) {
    let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
    let addr = token_contract.address();
    let client = token::Client::new(env, &addr);
    let admin_client = token::StellarAssetClient::new(env, &addr);
    (addr, client, admin_client)
}

fn setup<'a>(
    env: &'a Env,
    initial_balance: i128,
) -> (
    EscrowContractClient<'a>,
    Address, // contract_id
    Address,
    Address,
    Address,
    token::Client<'a>,
) {
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(env, &contract_id);

    let admin = Address::generate(env);
    let depositor = Address::generate(env);
    let contributor = Address::generate(env);
    let (token_addr, token_client, token_admin) = create_token(env, &admin);

    client.init(&admin, &token_addr);
    token_admin.mint(&depositor, &initial_balance);

    (
        client,
        contract_id,
        admin,
        depositor,
        contributor,
        token_client,
    )
}

fn has_event_topic(env: &Env, topic_name: &str) -> bool {
    let expected = Symbol::new(env, topic_name);
    for event in env.events().all().iter() {
        for topic in event.1.iter() {
            if topic.get_payload() == expected.to_val().get_payload() {
                return true;
            }
        }
    }
    false
}

// --- Parity: lock flow ---
#[test]
fn parity_lock_flow() {
    let env = Env::default();
    let amount = 10_000i128;
    let (client, contract_id, _admin, depositor, _contributor, token_client) = setup(&env, amount);

    let bounty_id = 1u64;
    let deadline = env.ledger().timestamp() + 1000;
    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

    let escrow = client.get_escrow(&bounty_id);
    assert_eq!(escrow.depositor, depositor);
    assert_eq!(escrow.amount, amount);
    assert_eq!(escrow.remaining_amount, amount);
    assert_eq!(escrow.status, EscrowStatus::Locked);
    assert_eq!(token_client.balance(&contract_id), amount);
}

// --- Parity: release flow ---
#[test]
fn parity_release_flow() {
    let env = Env::default();
    let amount = 10_000i128;
    let (client, contract_id, _admin, depositor, contributor, token_client) = setup(&env, amount);

    let bounty_id = 1u64;
    let deadline = env.ledger().timestamp() + 1000;
    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);
    client.release_funds(&bounty_id, &contributor);

    assert_eq!(token_client.balance(&contributor), amount);
    assert_eq!(token_client.balance(&contract_id), 0);
    let escrow = client.get_escrow(&bounty_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
    assert_eq!(escrow.remaining_amount, 0);
}

// --- Parity: refund flow ---
#[test]
fn parity_refund_flow() {
    let env = Env::default();
    let amount = 10_000i128;
    let (client, contract_id, _admin, depositor, _contributor, token_client) = setup(&env, amount);

    let bounty_id = 1u64;
    let deadline = env.ledger().timestamp() + 10;
    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

    env.ledger().set_timestamp(deadline + 1);
    client.refund(&bounty_id);

    assert_eq!(token_client.balance(&depositor), amount);
    assert_eq!(token_client.balance(&contract_id), 0);
    let escrow = client.get_escrow(&bounty_id);
    assert_eq!(escrow.status, EscrowStatus::Refunded);
    assert_eq!(escrow.remaining_amount, 0);
}

// --- Edge case: double release (must fail) ---
#[test]
fn parity_double_release_fails() {
    let env = Env::default();
    let amount = 10_000i128;
    let (client, _cid, _admin, depositor, contributor, _token_client) = setup(&env, amount);

    let bounty_id = 1u64;
    let deadline = env.ledger().timestamp() + 1000;
    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);
    client.release_funds(&bounty_id, &contributor);

    let res = client.try_release_funds(&bounty_id, &contributor);
    assert!(res.is_err());
}

// --- Edge case: double refund (must fail) ---
#[test]
fn parity_double_refund_fails() {
    let env = Env::default();
    let amount = 10_000i128;
    let (client, _cid, _admin, depositor, _contributor, _token_client) = setup(&env, amount);

    let bounty_id = 1u64;
    let deadline = env.ledger().timestamp() + 10;
    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);
    env.ledger().set_timestamp(deadline + 1);
    client.refund(&bounty_id);

    let res = client.try_refund(&bounty_id);
    assert!(res.is_err());
}

// --- Refund before deadline fails ---
#[test]
fn parity_refund_before_deadline_fails() {
    let env = Env::default();
    let amount = 10_000i128;
    let (client, _cid, _admin, depositor, _contributor, _token_client) = setup(&env, amount);

    let bounty_id = 1u64;
    let deadline = env.ledger().timestamp() + 1000;
    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

    let res = client.try_refund(&bounty_id);
    assert!(res.is_err());
}

// --- Jurisdiction: generic escrows remain untagged ---
#[test]
fn test_generic_escrow_has_no_jurisdiction_config() {
    let env = Env::default();
    let amount = 10_000i128;
    let (client, _cid, _admin, depositor, _contributor, _token_client) = setup(&env, amount);

    let bounty_id = 50u64;
    let deadline = env.ledger().timestamp() + 1000;
    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

    let cfg = client.get_escrow_jurisdiction(&bounty_id);
    assert_eq!(cfg, OptionalJurisdiction::None);
}

// --- Jurisdiction: tagged escrows can override identity-limit enforcement ---
#[test]
fn test_jurisdiction_tagged_escrow_can_skip_identity_limits() {
    let env = Env::default();
    let amount = 2_000_0000000i128; // above default unverified limit
    let (client, _cid, _admin, depositor, _contributor, _token_client) = setup(&env, amount);

    let bounty_id = 51u64;
    let deadline = env.ledger().timestamp() + 1000;
    let cfg = EscrowJurisdictionConfig {
        tag: Some(String::from_str(&env, "US-only")),
        requires_kyc: false,
        enforce_identity_limits: false,
        lock_paused: false,
        release_paused: false,
        refund_paused: false,
        max_lock_amount: Some(3_000_0000000),
    };

    client.lock_funds_with_jurisdiction(
        &depositor,
        &bounty_id,
        &amount,
        &deadline,
        &OptionalJurisdiction::Some(cfg.clone()),
    );

    let stored = client.get_escrow_jurisdiction(&bounty_id);
    assert_eq!(stored, OptionalJurisdiction::Some(cfg));
}

#[test]
fn test_generic_escrow_still_enforces_identity_limits() {
    let env = Env::default();
    let amount = 2_000_0000000i128; // above default unverified limit
    let (client, _cid, _admin, depositor, _contributor, _token_client) = setup(&env, amount);

    let bounty_id = 52u64;
    let deadline = env.ledger().timestamp() + 1000;
    let res = client.try_lock_funds(&depositor, &bounty_id, &amount, &deadline);
    assert!(res.is_err());
}

#[test]
fn test_jurisdiction_lock_pause_blocks_new_locks() {
    let env = Env::default();
    let amount = 10_000i128;
    let (client, _cid, _admin, depositor, _contributor, _token_client) = setup(&env, amount);

    let bounty_id = 53u64;
    let deadline = env.ledger().timestamp() + 1000;
    let cfg = EscrowJurisdictionConfig {
        tag: Some(String::from_str(&env, "EU-only")),
        requires_kyc: false,
        enforce_identity_limits: true,
        lock_paused: true,
        release_paused: false,
        refund_paused: false,
        max_lock_amount: Some(20_000),
    };

    let res = client.try_lock_funds_with_jurisdiction(
        &depositor,
        &bounty_id,
        &amount,
        &deadline,
        &OptionalJurisdiction::Some(cfg),
    );
    assert!(res.is_err());
}

#[test]
fn test_jurisdiction_events_emitted() {
    let env = Env::default();
    let amount = 10_000i128;
    let (client, _cid, _admin, depositor, contributor, _token_client) = setup(&env, amount);

    let bounty_id = 54u64;
    let deadline = env.ledger().timestamp() + 1000;
    let cfg = EscrowJurisdictionConfig {
        tag: Some(String::from_str(&env, "pilot-zone")),
        requires_kyc: false,
        enforce_identity_limits: false,
        lock_paused: false,
        release_paused: false,
        refund_paused: false,
        max_lock_amount: Some(100_000),
    };

    client.lock_funds_with_jurisdiction(
        &depositor,
        &bounty_id,
        &amount,
        &deadline,
        &OptionalJurisdiction::Some(cfg),
    );
    // Assert jurisdiction event emitted immediately; events are cleared on the
    // next invocation so we must check before calling release_funds.
    assert!(has_event_topic(&env, "juris"));

    client.release_funds(&bounty_id, &contributor);
}
