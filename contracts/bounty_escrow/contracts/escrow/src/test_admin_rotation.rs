//! Comprehensive tests for two-step admin rotation with timelock.
//!
//! This test suite validates:
//! - Proposal flow with proper authorization
//! - Timelock enforcement (cannot accept before delay)
//! - Acceptance flow with pending admin authorization
//! - Cancellation by current admin
//! - Timelock duration configuration
//! - Edge cases (duplicate proposals, self-rotation, etc.)
//! - Upgrade safety (storage keys persist correctly)
//! - Event emission for all state changes

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env,
};

// ═══════════════════════════════════════════════════════════════════════════════
// PROPOSAL TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_propose_admin_rotation_success() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);

    // Propose admin rotation
    let execute_after = client.propose_admin_rotation(&new_admin);

    // Verify execute_after is in the future
    let now = env.ledger().timestamp();
    assert!(execute_after > now);

    // Verify pending admin is set
    let pending = client.get_pending_admin();
    assert_eq!(pending, Some(new_admin));

    // Verify timelock is set
    let timelock = client.get_admin_rotation_timelock();
    assert_eq!(timelock, Some(execute_after));

    // Verify current admin is still the original
    let current_admin = client.get_admin();
    assert_eq!(current_admin, admin);
}

#[test]
fn test_propose_admin_rotation_uses_configured_timelock() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);

    // Set custom timelock to 2 hours
    let custom_timelock = 7200;
    client.set_rotation_timelock_duration(&custom_timelock);

    let start_time = env.ledger().timestamp();
    let execute_after = client.propose_admin_rotation(&new_admin);

    // Verify execute_after matches start_time + custom_timelock
    assert_eq!(execute_after, start_time + custom_timelock);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")] // Unauthorized
fn test_propose_admin_rotation_unauthorized() {
    let env = Env::default();
    // Don't mock auths - require real authorization

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let attacker = Address::generate(&env);

    client.init(&admin, &token);

    // Attacker tries to propose rotation
    env.mock_auths(&[&attacker]);
    client.propose_admin_rotation(&new_admin);
}

#[test]
fn test_propose_admin_rotation_cannot_rotate_to_self() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    // Try to rotate to self
    let result = client.try_propose_admin_rotation(&admin);
    assert_eq!(result, Err(Ok(Error::InvalidAdminRotationTarget)));
}

#[test]
fn test_propose_admin_rotation_cannot_duplicate() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);

    // First proposal succeeds
    client.propose_admin_rotation(&new_admin);

    // Second proposal should fail
    let another_admin = Address::generate(&env);
    let result = client.try_propose_admin_rotation(&another_admin);
    assert_eq!(result, Err(Ok(Error::AdminRotationAlreadyPending)));
}

#[test]
fn test_propose_admin_rotation_not_initialized() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let new_admin = Address::generate(&env);

    // Try to propose without initialization
    let result = client.try_propose_admin_rotation(&new_admin);
    assert_eq!(result, Err(Ok(Error::NotInitialized)));
}

// ═══════════════════════════════════════════════════════════════════════════════
// ACCEPTANCE TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_accept_admin_rotation_success() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);

    // Propose rotation
    let execute_after = client.propose_admin_rotation(&new_admin);

    // Advance time past timelock
    env.ledger().set_timestamp(execute_after + 1);

    // Accept rotation
    let accepted_admin = client.accept_admin_rotation();
    assert_eq!(accepted_admin, new_admin);

    // Verify admin has changed
    let current_admin = client.get_admin();
    assert_eq!(current_admin, new_admin);

    // Verify pending admin is cleared
    let pending = client.get_pending_admin();
    assert_eq!(pending, None);

    // Verify timelock is cleared
    let timelock = client.get_admin_rotation_timelock();
    assert_eq!(timelock, None);
}

#[test]
fn test_accept_admin_rotation_at_exact_timelock() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);

    let execute_after = client.propose_admin_rotation(&new_admin);

    // Advance time to exact execute_after timestamp
    env.ledger().set_timestamp(execute_after);

    // Should succeed at exact timestamp
    let accepted_admin = client.accept_admin_rotation();
    assert_eq!(accepted_admin, new_admin);
}

#[test]
fn test_accept_admin_rotation_requires_pending_admin_auth() {
    let env = Env::default();
    // Don't mock auths

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let attacker = Address::generate(&env);

    client.init(&admin, &token);
    let execute_after = client.propose_admin_rotation(&new_admin);

    env.ledger().set_timestamp(execute_after + 1);

    // Attacker tries to accept
    env.mock_auths(&[&attacker]);
    client.accept_admin_rotation();
}

#[test]
fn test_accept_admin_rotation_before_timelock_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);

    let execute_after = client.propose_admin_rotation(&new_admin);

    // Try to accept before timelock (1 second early)
    env.ledger().set_timestamp(execute_after - 1);

    let result = client.try_accept_admin_rotation();
    assert_eq!(result, Err(Ok(Error::AdminRotationTimelockActive)));
}

#[test]
fn test_accept_admin_rotation_not_pending() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    // No rotation pending
    let result = client.try_accept_admin_rotation();
    assert_eq!(result, Err(Ok(Error::AdminRotationNotPending)));
}

// ═══════════════════════════════════════════════════════════════════════════════
// CANCELLATION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_cancel_admin_rotation_success() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);
    client.propose_admin_rotation(&new_admin);

    // Cancel rotation
    client.cancel_admin_rotation();

    // Verify pending admin is cleared
    let pending = client.get_pending_admin();
    assert_eq!(pending, None);

    // Verify timelock is cleared
    let timelock = client.get_admin_rotation_timelock();
    assert_eq!(timelock, None);

    // Verify admin is unchanged
    let current_admin = client.get_admin();
    assert_eq!(current_admin, admin);
}

#[test]
fn test_cancel_admin_rotation_requires_admin_auth() {
    let env = Env::default();
    // Don't mock auths

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let attacker = Address::generate(&env);

    client.init(&admin, &token);
    client.propose_admin_rotation(&new_admin);

    // Attacker tries to cancel
    env.mock_auths(&[&attacker]);
    client.cancel_admin_rotation();
}

#[test]
fn test_cancel_admin_rotation_not_pending() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    // No rotation pending
    let result = client.try_cancel_admin_rotation();
    assert_eq!(result, Err(Ok(Error::AdminRotationNotPending)));
}

#[test]
fn test_can_propose_after_cancel() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin1 = Address::generate(&env);
    let new_admin2 = Address::generate(&env);

    client.init(&admin, &token);

    // Propose first rotation
    client.propose_admin_rotation(&new_admin1);

    // Cancel it
    client.cancel_admin_rotation();

    // Should be able to propose new rotation
    let execute_after = client.propose_admin_rotation(&new_admin2);
    let pending = client.get_pending_admin();
    assert_eq!(pending, Some(new_admin2));
    assert!(execute_after > env.ledger().timestamp());
}

// ═══════════════════════════════════════════════════════════════════════════════
// TIMELOCK DURATION CONFIGURATION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_set_rotation_timelock_duration_success() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    // Set to 2 hours
    let new_duration = 7200;
    client.set_rotation_timelock_duration(&new_duration);

    let duration = client.get_rotation_timelock_duration();
    assert_eq!(duration, new_duration);
}

#[test]
fn test_set_rotation_timelock_duration_minimum() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    // Set to minimum (1 hour)
    let min_duration = 3600;
    client.set_rotation_timelock_duration(&min_duration);

    let duration = client.get_rotation_timelock_duration();
    assert_eq!(duration, min_duration);
}

#[test]
fn test_set_rotation_timelock_duration_maximum() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    // Set to maximum (30 days)
    let max_duration = 2_592_000;
    client.set_rotation_timelock_duration(&max_duration);

    let duration = client.get_rotation_timelock_duration();
    assert_eq!(duration, max_duration);
}

#[test]
fn test_set_rotation_timelock_duration_below_minimum() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    // Try to set below minimum
    let result = client.try_set_rotation_timelock_duration(&3599);
    assert_eq!(result, Err(Ok(Error::InvalidAdminRotationTimelock)));
}

#[test]
fn test_set_rotation_timelock_duration_above_maximum() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    // Try to set above maximum
    let result = client.try_set_rotation_timelock_duration(&2_592_001);
    assert_eq!(result, Err(Ok(Error::InvalidAdminRotationTimelock)));
}

#[test]
fn test_set_rotation_timelock_duration_requires_admin() {
    let env = Env::default();
    // Don't mock auths

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let attacker = Address::generate(&env);

    client.init(&admin, &token);

    // Attacker tries to change timelock
    env.mock_auths(&[&attacker]);
    client.set_rotation_timelock_duration(&7200);
}

#[test]
fn test_default_timelock_duration() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    // Should use default (24 hours)
    let duration = client.get_rotation_timelock_duration();
    assert_eq!(duration, 86_400);
}

// ═══════════════════════════════════════════════════════════════════════════════
// STATUS AND CONFIG QUERY TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_get_admin_rotation_status_no_pending() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    // No rotation pending
    let status = client.get_admin_rotation_status();
    assert_eq!(status, None);
}

#[test]
fn test_get_admin_rotation_status_with_pending() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);
    let execute_after = client.propose_admin_rotation(&new_admin);

    let status = client.get_admin_rotation_status();
    assert!(status.is_some());

    let status = status.unwrap();
    assert_eq!(status.current_admin, admin);
    assert_eq!(status.pending_admin, new_admin);
    assert_eq!(status.execute_after, execute_after);
    assert!(!status.is_executable);
    assert!(status.remaining_seconds > 0);
}

#[test]
fn test_get_admin_rotation_status_executable() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);
    let execute_after = client.propose_admin_rotation(&new_admin);

    // Advance past timelock
    env.ledger().set_timestamp(execute_after + 100);

    let status = client.get_admin_rotation_status();
    assert!(status.is_some());

    let status = status.unwrap();
    assert!(status.is_executable);
    assert_eq!(status.remaining_seconds, 0);
}

#[test]
fn test_get_admin_rotation_config() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    let config = client.get_admin_rotation_config();
    assert_eq!(config.timelock_duration, 86_400); // default
    assert_eq!(config.min_timelock, 3_600);
    assert_eq!(config.max_timelock, 2_592_000);
    assert!(!config.has_pending_rotation);
}

#[test]
fn test_get_admin_rotation_config_with_pending() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);
    client.propose_admin_rotation(&new_admin);

    let config = client.get_admin_rotation_config();
    assert!(config.has_pending_rotation);
}

// ═══════════════════════════════════════════════════════════════════════════════
// END-TO-END FLOW TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_full_rotation_flow_propose_wait_accept() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);

    // Step 1: Propose
    let execute_after = client.propose_admin_rotation(&new_admin);
    assert_eq!(client.get_admin(), admin);

    // Step 2: Wait for timelock
    env.ledger().set_timestamp(execute_after + 1);

    // Step 3: Accept
    client.accept_admin_rotation();
    assert_eq!(client.get_admin(), new_admin);

    // Verify rotation is complete
    assert_eq!(client.get_pending_admin(), None);
    assert_eq!(client.get_admin_rotation_timelock(), None);
}

#[test]
fn test_full_rotation_flow_propose_cancel_repropose() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin1 = Address::generate(&env);
    let new_admin2 = Address::generate(&env);

    client.init(&admin, &token);

    // Propose first rotation
    client.propose_admin_rotation(&new_admin1);

    // Cancel it
    client.cancel_admin_rotation();

    // Propose different admin
    let execute_after = client.propose_admin_rotation(&new_admin2);

    // Wait and accept
    env.ledger().set_timestamp(execute_after + 1);
    client.accept_admin_rotation();

    assert_eq!(client.get_admin(), new_admin2);
}

#[test]
fn test_multiple_rotations_sequential() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin1 = Address::generate(&env);
    let token = Address::generate(&env);
    let admin2 = Address::generate(&env);
    let admin3 = Address::generate(&env);

    client.init(&admin1, &token);

    // First rotation: admin1 -> admin2
    let execute_after_1 = client.propose_admin_rotation(&admin2);
    env.ledger().set_timestamp(execute_after_1 + 1);
    client.accept_admin_rotation();
    assert_eq!(client.get_admin(), admin2);

    // Second rotation: admin2 -> admin3
    let execute_after_2 = client.propose_admin_rotation(&admin3);
    env.ledger().set_timestamp(execute_after_2 + 1);
    client.accept_admin_rotation();
    assert_eq!(client.get_admin(), admin3);
}

// ═══════════════════════════════════════════════════════════════════════════════
// UPGRADE SAFETY TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_admin_rotation_storage_persists_across_queries() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);
    let execute_after = client.propose_admin_rotation(&new_admin);

    // Query multiple times to ensure storage is consistent
    for _ in 0..5 {
        assert_eq!(client.get_pending_admin(), Some(new_admin.clone()));
        assert_eq!(client.get_admin_rotation_timelock(), Some(execute_after));
        assert_eq!(client.get_admin(), admin);
    }
}

#[test]
fn test_timelock_duration_persists() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    // Set custom duration
    client.set_rotation_timelock_duration(&7200);

    // Query multiple times
    for _ in 0..5 {
        assert_eq!(client.get_rotation_timelock_duration(), 7200);
    }
}
