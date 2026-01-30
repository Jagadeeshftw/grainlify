#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

use crate::{
    AdminActionType, BountyEscrowContract, BountyEscrowContractClient, ConfigLimits, FeeConfig,
};

fn create_test_env() -> (Env, BountyEscrowContractClient<'static>, Address) {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    (env, client, contract_id)
}

// ============================================================================
// Admin Update Tests
// ============================================================================

#[test]
fn test_update_admin_without_timelock() {
    let (env, client, _contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    // Update admin (no time-lock set)
    client.update_admin(&new_admin);

    // Verify admin was updated
    let state = client.get_contract_state();
    assert_eq!(state.admin, new_admin);
}

#[test]
fn test_update_admin_with_timelock() {
    let (env, client, _contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    // Set time-lock duration (1000 seconds)
    client.set_time_lock_duration(&1000);

    // Propose admin update
    client.update_admin(&new_admin);

    // Verify action was proposed
    let action = client.get_admin_action(&1);
    assert_eq!(action.action_id, 1);
    
    // UPDATED: Check the Enum variant which now carries the data
    assert_eq!(action.action_type, AdminActionType::UpdateAdmin(new_admin.clone()));
    
    // UPDATED: Removed checks for deleted Option fields (action.new_admin, etc.)
    assert!(!action.executed);

    // Try to execute before time-lock expires (should fail)
    let result = client.try_execute_admin_action(&1);
    assert!(result.is_err());

    // Advance time past time-lock
    env.ledger().set_timestamp(2000);

    // Execute action
    client.execute_admin_action(&1);

    // Verify admin was updated
    let state = client.get_contract_state();
    assert_eq!(state.admin, new_admin);
}

#[test]
fn test_cancel_admin_action() {
    let (env, client, _contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    // Set time-lock duration
    client.set_time_lock_duration(&1000);

    // Propose admin update
    client.update_admin(&new_admin);

    // Cancel the action
    client.cancel_admin_action(&1);

    // Verify action was cancelled (getting it should fail)
    let result = client.try_get_admin_action(&1);
    assert!(result.is_err());
}

// ============================================================================
// Payout Key Tests
// ============================================================================

#[test]
fn test_update_payout_key() {
    let (env, client, _contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let payout_key = Address::generate(&env);

    client.init(&admin, &token);

    // Update payout key
    client.update_payout_key(&payout_key);

    // Verify payout key was set
    let state = client.get_contract_state();
    assert_eq!(state.payout_key, Some(payout_key));
}

#[test]
fn test_update_payout_key_multiple_times() {
    let (env, client, _contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let payout_key1 = Address::generate(&env);
    let payout_key2 = Address::generate(&env);

    client.init(&admin, &token);

    // Set first payout key
    client.update_payout_key(&payout_key1);

    // Verify first key
    let state = client.get_contract_state();
    assert_eq!(state.payout_key, Some(payout_key1));

    // Update to second payout key
    client.update_payout_key(&payout_key2);

    // Verify second key
    let state = client.get_contract_state();
    assert_eq!(state.payout_key, Some(payout_key2));
}

// ============================================================================
// Config Limits Tests
// ============================================================================

#[test]
fn test_update_config_limits() {
    let (env, client, _contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    // Update config limits
    client.update_config_limits(
        &Some(1_000_000i128), // max_bounty_amount
        &Some(1_000i128),     // min_bounty_amount
        &Some(7_776_000u64),  // max_deadline_duration (90 days)
        &Some(86_400u64),     // min_deadline_duration (1 day)
    );

    // Verify limits were set
    let state = client.get_contract_state();
    assert_eq!(state.config_limits.max_bounty_amount, Some(1_000_000));
    assert_eq!(state.config_limits.min_bounty_amount, Some(1_000));
    assert_eq!(state.config_limits.max_deadline_duration, Some(7_776_000));
    assert_eq!(state.config_limits.min_deadline_duration, Some(86_400));
}

#[test]
fn test_update_config_limits_partial() {
    let (env, client, _contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    // Update only some limits
    client.update_config_limits(
        &Some(1_000_000i128), // max_bounty_amount
        &None,                // min_bounty_amount (not set)
        &None,                // max_deadline_duration (not set)
        &Some(86_400u64),     // min_deadline_duration
    );

    // Verify only specified limits were set
    let state = client.get_contract_state();
    assert_eq!(state.config_limits.max_bounty_amount, Some(1_000_000));
    assert_eq!(state.config_limits.min_bounty_amount, None);
    assert_eq!(state.config_limits.max_deadline_duration, None);
    assert_eq!(state.config_limits.min_deadline_duration, Some(86_400));
}

// ============================================================================
// Contract State View Tests
// ============================================================================

#[test]
fn test_get_contract_state() {
    let (env, client, _contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    // Get contract state
    let state = client.get_contract_state();

    // Verify state
    assert_eq!(state.admin, admin);
    assert_eq!(state.token, token);
    assert_eq!(state.payout_key, None);
    assert_eq!(state.is_paused, false);
    assert_eq!(state.time_lock_duration, 0);
    assert_eq!(state.contract_version, 1);
}

#[test]
fn test_get_contract_state_with_updates() {
    let (env, client, _contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let payout_key = Address::generate(&env);

    client.init(&admin, &token);

    // Make various updates
    client.update_payout_key(&payout_key);
    client.set_time_lock_duration(&1000);
    client.update_config_limits(
        &Some(1_000_000i128),
        &Some(1_000i128),
        &Some(7_776_000u64),
        &Some(86_400u64),
    );

    // Get contract state
    let state = client.get_contract_state();

    // Verify all updates are reflected
    assert_eq!(state.admin, admin);
    assert_eq!(state.token, token);
    assert_eq!(state.payout_key, Some(payout_key));
    assert_eq!(state.time_lock_duration, 1000);
    assert_eq!(state.config_limits.max_bounty_amount, Some(1_000_000));
}

// ============================================================================
// Authorization Tests
// ============================================================================

#[test]
#[should_panic]
fn test_update_admin_unauthorized() {
    let (env, client, _contract_id) = create_test_env();

    let admin = Address::generate(&env);
    let unauthorized = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let token = Address::generate(&env);

    // Mock auth only for init
    env.mock_all_auths();
    client.init(&admin, &token);

    // Remove mock auth and try to update as unauthorized user
    env.mock_auths(&[]);
    unauthorized.require_auth();

    // This should panic
    client.update_admin(&new_admin);
}

#[test]
#[should_panic]
fn test_update_payout_key_unauthorized() {
    let (env, client, _contract_id) = create_test_env();

    let admin = Address::generate(&env);
    let unauthorized = Address::generate(&env);
    let payout_key = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();
    client.init(&admin, &token);

    env.mock_auths(&[]);
    unauthorized.require_auth();

    // This should panic
    client.update_payout_key(&payout_key);
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_complete_admin_workflow() {
    let (env, client, _contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let payout_key = Address::generate(&env);
    let token = Address::generate(&env);

    // 1. Initialize
    client.init(&admin, &token);

    // 2. Configure time-lock
    client.set_time_lock_duration(&1000);

    // 3. Set payout key
    client.update_payout_key(&payout_key);

    // 4. Update config limits
    client.update_config_limits(
        &Some(1_000_000i128),
        &Some(1_000i128),
        &Some(7_776_000u64),
        &Some(86_400u64),
    );

    // 5. Update fee config
    client.update_fee_config(&Some(100), &Some(50), &Some(payout_key.clone()), &Some(true));

    // 6. Propose admin update
    client.update_admin(&new_admin);

    // 7. Verify state before execution
    let state = client.get_contract_state();
    assert_eq!(state.admin, admin); // Still old admin
    assert_eq!(state.payout_key, Some(payout_key.clone()));
    assert_eq!(state.time_lock_duration, 1000);

    // 8. Advance time and execute
    env.ledger().set_timestamp(2000);
    client.execute_admin_action(&1);

    // 9. Verify final state
    let final_state = client.get_contract_state();
    assert_eq!(final_state.admin, new_admin);
    assert_eq!(final_state.payout_key, Some(payout_key));
    assert_eq!(final_state.fee_config.lock_fee_rate, 100);
    assert_eq!(final_state.fee_config.release_fee_rate, 50);
}

#[test]
fn test_multiple_admin_actions() {
    let (env, client, _contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let payout_key = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);
    client.set_time_lock_duration(&1000);

    // Propose multiple actions
    client.update_admin(&new_admin);
    client.update_payout_key(&payout_key); // This should execute immediately (no time-lock for payout key)

    // Verify first action is pending
    let action = client.get_admin_action(&1);
    
    // UPDATED: Check for Enum variant data
    assert_eq!(action.action_type, AdminActionType::UpdateAdmin(new_admin.clone()));

    // Verify payout key was updated immediately
    let state = client.get_contract_state();
    assert_eq!(state.payout_key, Some(payout_key.clone()));

    // Execute pending admin action
    env.ledger().set_timestamp(2000);
    client.execute_admin_action(&1);

    // Verify both updates are complete
    let final_state = client.get_contract_state();
    assert_eq!(final_state.admin, new_admin);
    assert_eq!(final_state.payout_key, Some(payout_key.clone()));
}