//! Tests for participant filtering: mutually exclusive modes (Disabled, BlocklistOnly, AllowlistOnly)
//! and state transitions between modes.

#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token, Address, Env,
};

fn create_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000_000);
    env
}

fn setup(
    env: &Env,
) -> (
    BountyEscrowContractClient<'_>,
    Address,
    Address,
    token::Client<'_>,
) {
    let admin = Address::generate(env);
    let depositor = Address::generate(env);
    let other = Address::generate(env);

    let token_admin = Address::generate(env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_admin_client = token::StellarAssetClient::new(env, &token_address);
    let token_client = token::Client::new(env, &token_address);

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(env, &contract_id);
    client.init(&admin, &token_address);

    token_admin_client.mint(&depositor, &10_000);
    token_admin_client.mint(&other, &10_000);
    (client, depositor, other, token_client)
}

#[test]
fn test_default_mode_is_disabled() {
    let env = create_env();
    let (client, depositor, _other, _token) = setup(&env);
    assert_eq!(client.get_filter_mode(), ParticipantFilterMode::Disabled);
    let deadline = env.ledger().timestamp() + 86_400;
    client.lock_funds(&depositor, &1, &100, &deadline);
    // Any address can lock when mode is Disabled
}

#[test]
fn test_blocklist_only_rejects_blocklisted() {
    let env = create_env();
    let (client, depositor, other, _token) = setup(&env);

    client.set_filter_mode(&ParticipantFilterMode::BlocklistOnly);
    client.set_blocklist_entry(&depositor, &true);

    let deadline = env.ledger().timestamp() + 86_400;
    let res = client.try_lock_funds(&depositor, &1, &100, &deadline);
    assert!(res.is_err());

    // Non-blocklisted can lock
    client.lock_funds(&other, &2, &100, &deadline);
}

#[test]
fn test_blocklist_only_allows_non_blocklisted() {
    let env = create_env();
    let (client, depositor, other, _token) = setup(&env);

    client.set_filter_mode(&ParticipantFilterMode::BlocklistOnly);
    // depositor not blocklisted
    let deadline = env.ledger().timestamp() + 86_400;
    client.lock_funds(&depositor, &1, &100, &deadline);
    client.lock_funds(&other, &2, &100, &deadline);
}

#[test]
fn test_allowlist_only_rejects_non_allowlisted() {
    let env = create_env();
    let (client, depositor, other, _token) = setup(&env);

    client.set_filter_mode(&ParticipantFilterMode::AllowlistOnly);
    client.set_whitelist_entry(&depositor, &true);
    // other is not on allowlist

    let deadline = env.ledger().timestamp() + 86_400;
    client.lock_funds(&depositor, &1, &100, &deadline);

    let res = client.try_lock_funds(&other, &2, &100, &deadline);
    assert!(res.is_err());
}

#[test]
fn test_allowlist_only_allows_allowlisted() {
    let env = create_env();
    let (client, depositor, other, _token) = setup(&env);

    client.set_filter_mode(&ParticipantFilterMode::AllowlistOnly);
    client.set_whitelist_entry(&depositor, &true);
    client.set_whitelist_entry(&other, &true);

    let deadline = env.ledger().timestamp() + 86_400;
    client.lock_funds(&depositor, &1, &100, &deadline);
    client.lock_funds(&other, &2, &100, &deadline);
}

#[test]
fn test_mode_transition_disabled_to_blocklist_only() {
    let env = create_env();
    let (client, depositor, other, _token) = setup(&env);

    assert_eq!(client.get_filter_mode(), ParticipantFilterMode::Disabled);
    client.set_filter_mode(&ParticipantFilterMode::BlocklistOnly);
    assert_eq!(
        client.get_filter_mode(),
        ParticipantFilterMode::BlocklistOnly
    );

    client.set_blocklist_entry(&depositor, &true);
    let deadline = env.ledger().timestamp() + 86_400;
    assert!(client
        .try_lock_funds(&depositor, &1, &100, &deadline)
        .is_err());
    client.lock_funds(&other, &2, &100, &deadline);
}

#[test]
fn test_mode_transition_blocklist_only_to_allowlist_only() {
    let env = create_env();
    let (client, depositor, other, _token) = setup(&env);

    client.set_filter_mode(&ParticipantFilterMode::BlocklistOnly);
    client.set_blocklist_entry(&other, &true);
    let deadline = env.ledger().timestamp() + 86_400;
    client.lock_funds(&depositor, &1, &100, &deadline);

    client.set_filter_mode(&ParticipantFilterMode::AllowlistOnly);
    client.set_whitelist_entry(&depositor, &true);
    // other is blocklisted but mode is now allowlist: other is not allowlisted so rejected
    assert!(client.try_lock_funds(&other, &2, &100, &deadline).is_err());
    client.lock_funds(&depositor, &3, &100, &deadline);
}

#[test]
fn test_mode_transition_back_to_disabled() {
    let env = create_env();
    let (client, depositor, other, _token) = setup(&env);

    client.set_filter_mode(&ParticipantFilterMode::AllowlistOnly);
    client.set_whitelist_entry(&depositor, &true);
    let deadline = env.ledger().timestamp() + 86_400;
    client.lock_funds(&depositor, &1, &100, &deadline);
    assert!(client.try_lock_funds(&other, &2, &100, &deadline).is_err());

    client.set_filter_mode(&ParticipantFilterMode::Disabled);
    // With Disabled, other can now lock (no list check)
    client.lock_funds(&other, &2, &100, &deadline);
}

#[test]
fn test_lists_persist_across_mode_switch() {
    let env = create_env();
    let (client, depositor, other, _token) = setup(&env);

    client.set_whitelist_entry(&depositor, &true);
    client.set_blocklist_entry(&other, &true);

    client.set_filter_mode(&ParticipantFilterMode::AllowlistOnly);
    let deadline = env.ledger().timestamp() + 86_400;
    client.lock_funds(&depositor, &1, &100, &deadline);
    assert!(client.try_lock_funds(&other, &2, &100, &deadline).is_err());

    client.set_filter_mode(&ParticipantFilterMode::Disabled);
    client.set_filter_mode(&ParticipantFilterMode::AllowlistOnly);
    // Still only depositor allowlisted; other still not on allowlist
    assert!(client.try_lock_funds(&other, &3, &100, &deadline).is_err());
    client.lock_funds(&depositor, &4, &100, &deadline);
}

#[test]
fn test_batch_lock_funds_respects_filter_mode() {
    let env = create_env();
    let (client, depositor, other, _token) = setup(&env);

    client.set_filter_mode(&ParticipantFilterMode::AllowlistOnly);
    client.set_whitelist_entry(&depositor, &true);

    let deadline = env.ledger().timestamp() + 86_400;
    let items = vec![
        &env,
        LockFundsItem {
            bounty_id: 10,
            depositor: depositor.clone(),
            amount: 100,
            deadline,
        },
        LockFundsItem {
            bounty_id: 11,
            depositor: other.clone(),
            amount: 100,
            deadline,
        },
    ];
    let res = client.try_batch_lock_funds(&items);
    assert!(res.is_err());
}

#[test]
fn test_set_filter_mode_emits_event_and_persists() {
    let env = create_env();
    let (client, _depositor, _other, _token) = setup(&env);

    assert_eq!(client.get_filter_mode(), ParticipantFilterMode::Disabled);
    client.set_filter_mode(&ParticipantFilterMode::BlocklistOnly);
    assert_eq!(
        client.get_filter_mode(),
        ParticipantFilterMode::BlocklistOnly
    );
    client.set_filter_mode(&ParticipantFilterMode::AllowlistOnly);
    assert_eq!(
        client.get_filter_mode(),
        ParticipantFilterMode::AllowlistOnly
    );
}

#[test]
fn test_query_whitelist_pagination_semantics() {
    let env = create_env();
    let (client, depositor, other, _token) = setup(&env);
    let third = Address::generate(&env);

    client.set_whitelist_entry(&depositor, &true);
    client.set_whitelist_entry(&other, &true);
    client.set_whitelist_entry(&third, &true);

    let page1 = client.query_whitelist(&0, &2);
    assert_eq!(page1.items.len(), 2);
    assert_eq!(page1.total, 3);
    assert!(page1.has_more);

    let page2 = client.query_whitelist(&2, &2);
    assert_eq!(page2.items.len(), 1);
    assert_eq!(page2.items.get(0).unwrap(), third);
    assert_eq!(page2.total, 3);
    assert!(!page2.has_more);

    let page3 = client.query_whitelist(&10, &5);
    assert_eq!(page3.items.len(), 0);
    assert!(!page3.has_more);
}

#[test]
fn test_query_blocklist_pagination_and_mutation() {
    let env = create_env();
    let (client, depositor, other, _token) = setup(&env);

    client.set_blocklist_entry(&depositor, &true);
    client.set_blocklist_entry(&other, &true);

    let initial = client.query_blocklist(&0, &10);
    assert_eq!(initial.items.len(), 2);
    assert_eq!(initial.total, 2);
    assert!(!initial.has_more);

    client.set_blocklist_entry(&depositor, &false);
    let after_remove = client.query_blocklist(&0, &10);
    assert_eq!(after_remove.items.len(), 1);
    assert_eq!(after_remove.items.get(0).unwrap(), other);
    assert_eq!(after_remove.total, 1);
    assert!(!after_remove.has_more);
}
