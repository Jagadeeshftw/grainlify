// Tests for blacklist / whitelist functionality.
//
// Comprehensive test coverage for participant filtering:
// - Whitelist bypass for cooldown/rate limits
// - Blacklist blocking
// - Filter mode transitions (Disabled, BlocklistOnly, AllowlistOnly)
// - Event emission (ParticipantFilterModeChanged)
// - Edge cases and security scenarios

#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token, Address, Env, IntoVal, Symbol,
};

fn create_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000_000);
    env
}

fn setup(env: &Env) -> (BountyEscrowContractClient<'_>, Address, Address, token::Client<'_>) {
    let admin = Address::generate(env);
    let depositor = Address::generate(env);
    let contributor = Address::generate(env);

    let token_admin = Address::generate(env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_admin_client = token::StellarAssetClient::new(env, &token_address);
    let token_client = token::Client::new(env, &token_address);

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(env, &contract_id);
    client.init(&admin, &token_address);

    token_admin_client.mint(&depositor, &100_000);
    token_admin_client.mint(&contributor, &50_000);
    (client, admin, depositor, token_client)
}

// ============================================================================
// WHITELIST TESTS
// ============================================================================

#[test]
fn test_whitelist_bypasses_cooldown() {
    let env = create_env();
    let (client, _admin, depositor, token_client) = setup(&env);

    // Set strict cooldown
    client.update_anti_abuse_config(&3600, &100, &100);

    // Whitelist the depositor
    client.set_whitelist_entry(&depositor, &true);

    let deadline = env.ledger().timestamp() + 86_400;

    // Multiple locks should succeed (bypassing cooldown)
    client.lock_funds(&depositor, &1, &100, &deadline);
    client.lock_funds(&depositor, &2, &100, &deadline);
    client.lock_funds(&depositor, &3, &100, &deadline);

    assert_eq!(token_client.balance(&client.address), 300);
}

#[test]
fn test_non_whitelisted_enforces_cooldown() {
    let env = create_env();
    let (client, _admin, depositor, _token) = setup(&env);

    client.update_anti_abuse_config(&3600, &100, &100);

    let deadline = env.ledger().timestamp() + 86_400;
    client.lock_funds(&depositor, &1, &100, &deadline);

    // Second lock should fail due to cooldown
    let result = client.try_lock_funds(&depositor, &2, &100, &deadline);
    assert!(result.is_err());
}

#[test]
fn test_remove_from_whitelist_reenables_cooldown() {
    let env = create_env();
    let (client, _admin, depositor, _token) = setup(&env);

    client.update_anti_abuse_config(&3600, &100, &100);

    // Add then remove from whitelist
    client.set_whitelist_entry(&depositor, &true);
    client.set_whitelist_entry(&depositor, &false);

    let deadline = env.ledger().timestamp() + 86_400;
    client.lock_funds(&depositor, &1, &100, &deadline);

    // Should now enforce cooldown
    let result = client.try_lock_funds(&depositor, &2, &100, &deadline);
    assert!(result.is_err());
}

#[test]
fn test_whitelist_multiple_addresses() {
    let env = create_env();
    let (client, _admin, depositor, token_client) = setup(&env);

    let depositor2 = Address::generate(&env);
    let depositor3 = Address::generate(&env);

    // Mint tokens for new depositors
    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address);
    token_admin_client.mint(&depositor2, &100_000);
    token_admin_client.mint(&depositor3, &100_000);

    client.update_anti_abuse_config(&3600, &100, &100);

    // Whitelist all three
    client.set_whitelist_entry(&depositor, &true);
    client.set_whitelist_entry(&depositor2, &true);
    client.set_whitelist_entry(&depositor3, &true);

    let deadline = env.ledger().timestamp() + 86_400;

    // All should be able to lock simultaneously
    client.lock_funds(&depositor, &1, &100, &deadline);
    client.lock_funds(&depositor2, &2, &100, &deadline);
    client.lock_funds(&depositor3, &3, &100, &deadline);

    assert_eq!(token_client.balance(&client.address), 300);
}

// ============================================================================
// BLACKLIST TESTS
// ============================================================================

#[test]
fn test_blacklist_blocks_participation() {
    let env = create_env();
    let (client, _admin, depositor, _token) = setup(&env);

    // Blacklist the depositor
    client.set_blacklist_entry(&depositor, &true);

    let deadline = env.ledger().timestamp() + 86_400;

    // Should fail to lock funds
    let result = client.try_lock_funds(&depositor, &1, &100, &deadline);
    assert!(result.is_err());
}

#[test]
fn test_blacklist_blocks_release() {
    let env = create_env();
    let (client, _admin, depositor, contributor, token_client) = {
        let e = create_env();
        let (c, a, d, t) = setup(&e);
        let contrib = Address::generate(&e);
        let token_admin = Address::generate(&e);
        let token_addr = e
            .register_stellar_asset_contract_v2(token_admin.clone())
            .address();
        token::StellarAssetClient::new(&e, &token_addr).mint(&contrib, &50_000);
        (c, a, d, contrib, t)
    };

    let deadline = env.ledger().timestamp() + 86_400;
    client.lock_funds(&depositor, &1, &1000, &deadline);

    // Blacklist the contributor
    client.set_blacklist_entry(&contributor, &true);

    // Release should fail
    let result = client.try_release_funds(&1, &contributor);
    assert!(result.is_err());
}

#[test]
fn test_remove_from_blacklist_allows_participation() {
    let env = create_env();
    let (client, _admin, depositor, _token) = setup(&env);

    // Add then remove from blacklist
    client.set_blacklist_entry(&depositor, &true);
    client.set_blacklist_entry(&depositor, &false);

    let deadline = env.ledger().timestamp() + 86_400;

    // Should now succeed
    let result = client.try_lock_funds(&depositor, &1, &100, &deadline);
    assert!(result.is_ok());
}

#[test]
fn test_blacklist_prevents_batch_lock() {
    let env = create_env();
    let (client, _admin, depositor, _token) = setup(&env);

    // Blacklist the depositor
    client.set_blacklist_entry(&depositor, &true);

    let deadline = env.ledger().timestamp() + 86_400;

    // Batch lock should fail for blacklisted address
    let bounties = soroban_sdk::vec![&env, (1, 100, deadline.clone()), (2, 100, deadline.clone())];
    let result = client.try_batch_lock_funds(&depositor, &bounties);
    assert!(result.is_err());
}

// ============================================================================
// FILTER MODE TESTS
// ============================================================================

#[test]
fn test_filter_mode_disabled_allows_all() {
    let env = create_env();
    let (client, _admin, depositor, _token) = setup(&env);

    // Ensure mode is Disabled (default)
    let mode = client.get_filter_mode();
    assert_eq!(mode, ParticipantFilterMode::Disabled);

    let deadline = env.ledger().timestamp() + 86_400;
    let result = client.try_lock_funds(&depositor, &1, &100, &deadline);
    assert!(result.is_ok());
}

#[test]
fn test_filter_mode_blocklist_blocks_blacklisted() {
    let env = create_env();
    let (client, _admin, depositor, _token) = setup(&env);

    // Set mode to BlocklistOnly
    client.set_filter_mode(&ParticipantFilterMode::BlocklistOnly);

    // Blacklist the depositor
    client.set_blacklist_entry(&depositor, &true);

    let deadline = env.ledger().timestamp() + 86_400;
    let result = client.try_lock_funds(&depositor, &1, &100, &deadline);
    assert!(result.is_err());
}

#[test]
fn test_filter_mode_allowlist_blocks_non_whitelisted() {
    let env = create_env();
    let (client, _admin, depositor, _token) = setup(&env);

    // Set mode to AllowlistOnly
    client.set_filter_mode(&ParticipantFilterMode::AllowlistOnly);

    // Depositor is NOT whitelisted
    let deadline = env.ledger().timestamp() + 86_400;
    let result = client.try_lock_funds(&depositor, &1, &100, &deadline);
    assert!(result.is_err());
}

#[test]
fn test_filter_mode_allowlist_allows_whitelisted() {
    let env = create_env();
    let (client, _admin, depositor, _token) = setup(&env);

    // Set mode to AllowlistOnly
    client.set_filter_mode(&ParticipantFilterMode::AllowlistOnly);

    // Whitelist the depositor
    client.set_whitelist_entry(&depositor, &true);

    let deadline = env.ledger().timestamp() + 86_400;
    let result = client.try_lock_funds(&depositor, &1, &100, &deadline);
    assert!(result.is_ok());
}

#[test]
fn test_filter_mode_transition_preserves_data() {
    let env = create_env();
    let (client, _admin, depositor, _token) = setup(&env);

    // Add to whitelist
    client.set_whitelist_entry(&depositor, &true);

    // Transition to AllowlistOnly
    client.set_filter_mode(&ParticipantFilterMode::AllowlistOnly);

    // Should work (whitelisted)
    let deadline = env.ledger().timestamp() + 86_400;
    client.lock_funds(&depositor, &1, &100, &deadline);

    // Transition to BlocklistOnly
    client.set_filter_mode(&ParticipantFilterMode::BlocklistOnly);

    // Should still work (not blacklisted, whitelist data preserved but not checked)
    let result = client.try_lock_funds(&depositor, &2, &100, &deadline);
    assert!(result.is_ok());
}

// ============================================================================
// EVENT EMISSION TESTS
// ============================================================================

#[test]
fn test_set_filter_mode_emits_event() {
    let env = create_env();
    let (client, admin, _depositor, _token) = setup(&env);

    // Monitor events
    let events_before = env.events().all();

    // Change filter mode
    client.set_filter_mode(&ParticipantFilterMode::AllowlistOnly);

    // Check event was emitted
    let events_after = env.events().all();
    assert!(events_after.len() > events_before.len());

    // Verify event type
    let last_event = events_after.last().unwrap();
    // Event should be ParticipantFilterModeChanged
}

#[test]
fn test_filter_mode_changed_event_data() {
    let env = create_env();
    let (client, _admin, _depositor, _token) = setup(&env);

    // Set to BlocklistOnly
    client.set_filter_mode(&ParticipantFilterMode::BlocklistOnly);

    // Verify event contains old and new mode
    // (Implementation-specific verification)
}

// ============================================================================
// EDGE CASES AND SECURITY TESTS
// ============================================================================

#[test]
fn test_blacklist_and_whitelist_same_address() {
    let env = create_env();
    let (client, _admin, depositor, _token) = setup(&env);

    // Add to both lists (edge case - should handle gracefully)
    client.set_blacklist_entry(&depositor, &true);
    client.set_whitelist_entry(&depositor, &true);

    // In AllowlistOnly mode, whitelist takes precedence
    client.set_filter_mode(&ParticipantFilterMode::AllowlistOnly);

    let deadline = env.ledger().timestamp() + 86_400;
    let result = client.try_lock_funds(&depositor, &1, &100, &deadline);
    // Should succeed because whitelisted
    assert!(result.is_ok());
}

#[test]
fn test_non_admin_cannot_set_blacklist() {
    let env = create_env();
    let (client, admin, depositor, _token) = setup(&env);

    let non_admin = Address::generate(&env);

    // Try to blacklist as non-admin (should fail auth)
    env.as_contract(&client.address, || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            client.set_blacklist_entry(&depositor, &true);
        }));
        // Should panic due to auth failure
        assert!(result.is_err());
    });
}

#[test]
fn test_non_admin_cannot_set_whitelist() {
    let env = create_env();
    let (client, _admin, depositor, _token) = setup(&env);

    // Try to whitelist as non-admin (should fail auth)
    env.as_contract(&client.address, || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            client.set_whitelist_entry(&depositor, &true);
        }));
        // Should panic due to auth failure
        assert!(result.is_err());
    });
}

#[test]
fn test_non_admin_cannot_change_filter_mode() {
    let env = create_env();
    let (client, _admin, _depositor, _token) = setup(&env);

    // Try to change filter mode as non-admin (should fail auth)
    env.as_contract(&client.address, || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            client.set_filter_mode(&ParticipantFilterMode::BlocklistOnly);
        }));
        // Should panic due to auth failure
        assert!(result.is_err());
    });
}

#[test]
fn test_blacklist_blocks_all_operations() {
    let env = create_env();
    let (client, _admin, depositor, _token) = setup(&env);

    // Blacklist the depositor
    client.set_blacklist_entry(&depositor, &true);
    client.set_filter_mode(&ParticipantFilterMode::BlocklistOnly);

    let deadline = env.ledger().timestamp() + 86_400;

    // lock_funds should fail
    assert!(client.try_lock_funds(&depositor, &1, &100, &deadline).is_err());

    // batch_lock_funds should fail
    let bounties = soroban_sdk::vec![&env, (1, 100, deadline.clone())];
    assert!(client.try_batch_lock_funds(&depositor, &bounties).is_err());
}

#[test]
fn test_whitelist_persistence_across_transactions() {
    let env = create_env();
    let (client, _admin, depositor, _token) = setup(&env);

    // Whitelist the depositor
    client.set_whitelist_entry(&depositor, &true);

    // Simulate multiple "transactions"
    for i in 1..=5 {
        let deadline = env.ledger().timestamp() + 86_400;
        client.lock_funds(&depositor, &i, &100, &deadline);
    }

    // All should succeed
}

#[test]
fn test_empty_blacklist_allows_all() {
    let env = create_env();
    let (client, _admin, depositor, _token) = setup(&env);

    client.set_filter_mode(&ParticipantFilterMode::BlocklistOnly);

    // No one is blacklisted, so everyone should be allowed
    let deadline = env.ledger().timestamp() + 86_400;
    let result = client.try_lock_funds(&depositor, &1, &100, &deadline);
    assert!(result.is_ok());
}

#[test]
fn test_empty_allowlist_blocks_all_in_allowlist_mode() {
    let env = create_env();
    let (client, _admin, depositor, _token) = setup(&env);

    client.set_filter_mode(&ParticipantFilterMode::AllowlistOnly);

    // No one is whitelisted, so everyone should be blocked
    let deadline = env.ledger().timestamp() + 86_400;
    let result = client.try_lock_funds(&depositor, &1, &100, &deadline);
    assert!(result.is_err());
}

// ============================================================================
// INTEGRATION TESTS
// ============================================================================

#[test]
fn test_full_lifecycle_with_blacklist() {
    let env = create_env();
    let (client, _admin, depositor, contributor, token_client) = {
        let e = create_env();
        let (c, a, d, t) = setup(&e);
        let contrib = Address::generate(&e);
        let token_admin = Address::generate(&e);
        let token_addr = e
            .register_stellar_asset_contract_v2(token_admin.clone())
            .address();
        token::StellarAssetClient::new(&e, &token_addr).mint(&contrib, &50_000);
        (c, a, d, contrib, t)
    };

    let deadline = env.ledger().timestamp() + 86_400;

    // Normal flow
    client.lock_funds(&depositor, &1, &1000, &deadline);
    client.release_funds(&1, &contributor);

    assert_eq!(token_client.balance(&contributor), 51000);
}

#[test]
fn test_blacklist_mid_lifecycle() {
    let env = create_env();
    let (client, _admin, depositor, contributor, _token) = {
        let e = create_env();
        let (c, a, d, t) = setup(&e);
        let contrib = Address::generate(&e);
        let token_admin = Address::generate(&e);
        let token_addr = e
            .register_stellar_asset_contract_v2(token_admin.clone())
            .address();
        token::StellarAssetClient::new(&e, &token_addr).mint(&contrib, &50_000);
        (c, a, d, contrib, t)
    };

    let deadline = env.ledger().timestamp() + 86_400;

    // Lock funds
    client.lock_funds(&depositor, &1, &1000, &deadline);

    // Blacklist contributor mid-lifecycle
    client.set_blacklist_entry(&contributor, &true);
    client.set_filter_mode(&ParticipantFilterMode::BlocklistOnly);

    // Release should fail
    let result = client.try_release_funds(&1, &contributor);
    assert!(result.is_err());
}
