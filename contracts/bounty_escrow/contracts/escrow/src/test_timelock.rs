use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

#[test]
#[ignore]
fn test_configure_timelock() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    // Initialize contract
    client.init(&admin, &token);

    // Test configuring timelock with valid delay
    let delay = 7200; // 2 hours
    client.configure_timelock(&delay, &true);

    let config = client.get_timelock_config();
    assert_eq!(config.delay, delay);
    assert_eq!(config.is_enabled, true);

    // Test configuring timelock with disabled
    client.configure_timelock(&86400, &false);
    let config = client.get_timelock_config();
    assert_eq!(config.delay, 86400);
    assert_eq!(config.is_enabled, false);
}

#[test]
#[should_panic(expected = "Error(Contract, #57)")] // DelayBelowMinimum
fn test_configure_timelock_below_minimum() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    // Try to configure with delay below minimum (3599 < 3600)
    client.configure_timelock(&3599, &true);
}

#[test]
#[should_panic(expected = "Error(Contract, #58)")] // DelayAboveMaximum
fn test_configure_timelock_above_maximum() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    // Try to configure with delay above maximum (2592001 > 2592000)
    client.configure_timelock(&2592001, &true);
}

#[test]
#[ignore]
#[should_panic(expected = "Error(Contract, #7)")] // Unauthorized
fn test_configure_timelock_unauthorized() {
    let env = Env::default();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let non_admin = Address::generate(&env);

    client.init(&admin, &token);

    // Try to configure timelock as non-admin
    env.mock_auths(&[&non_admin]);
    client.configure_timelock(&7200, &true);
}

#[test]
#[ignore]
fn test_propose_admin_action_immediate_execution_when_disabled() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);

    // Ensure timelock is disabled
    client.configure_timelock(&86400, &false);

    // Propose admin change - should execute immediately
    let action_id = client.propose_admin_action(
        &ActionType::ChangeAdmin,
        &ActionPayload::ChangeAdmin(new_admin.clone()),
    );

    // Should return 0 to signal immediate execution
    assert_eq!(action_id, 0);

    // Admin should be changed immediately
    let current_admin = client.get_admin();
    assert_eq!(current_admin, new_admin);

    // No pending actions should exist
    let pending = client.get_pending_actions();
    assert_eq!(pending.len(), 0);
}

#[test]
#[ignore]
fn test_propose_admin_action_creates_pending_when_enabled() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);

    // Enable timelock with 2-hour delay
    let delay = 7200;
    client.configure_timelock(&delay, &true);

    let current_timestamp = env.ledger().timestamp();

    // Propose admin change
    let action_id = client.propose_admin_action(
        &ActionType::ChangeAdmin,
        &ActionPayload::ChangeAdmin(new_admin.clone()),
    );

    // Should return a non-zero action ID
    assert!(action_id > 0);

    // Verify pending action
    let action = client.get_action(&action_id);
    assert_eq!(action.action_id, action_id);
    assert_eq!(action.action_type, ActionType::ChangeAdmin);
    assert_eq!(action.proposed_by, admin);
    assert_eq!(action.proposed_at, current_timestamp);
    assert_eq!(action.execute_after, current_timestamp + delay);
    assert_eq!(action.status, ActionStatus::Pending);

    // Admin should not be changed yet
    let current_admin = client.get_admin();
    assert_eq!(current_admin, admin);
}

#[test]
#[should_panic(expected = "Error(Contract, #51)")] // TimelockNotElapsed
fn test_execute_before_delay_reverts() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);

    // Enable timelock with 2-hour delay
    let delay = 7200;
    client.configure_timelock(&delay, &true);

    // Propose admin change
    let action_id = client.propose_admin_action(
        &ActionType::ChangeAdmin,
        &ActionPayload::ChangeAdmin(new_admin),
    );

    // Try to execute immediately (before delay)
    client.execute_after_delay(&action_id);
}

#[test]
#[ignore]
fn test_execute_at_exact_delay_succeeds() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);

    // Enable timelock with 2-hour delay
    let delay = 7200;
    client.configure_timelock(&delay, &true);

    let start_timestamp = env.ledger().timestamp();

    // Propose admin change
    let action_id = client.propose_admin_action(
        &ActionType::ChangeAdmin,
        &ActionPayload::ChangeAdmin(new_admin.clone()),
    );

    // Advance time exactly to the execute_after timestamp
    env.ledger().set_timestamp(start_timestamp + delay);

    // Execute should succeed
    client.execute_after_delay(&action_id);

    // Admin should be changed
    let current_admin = client.get_admin();
    assert_eq!(current_admin, new_admin);

    // Action should be marked as executed
    let action = client.get_action(&action_id);
    assert_eq!(action.status, ActionStatus::Executed);
}

#[test]
#[ignore]
fn test_execute_after_delay_succeeds() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);

    // Enable timelock with 2-hour delay
    let delay = 7200;
    client.configure_timelock(&delay, &true);

    let start_timestamp = env.ledger().timestamp();

    // Propose admin change
    let action_id = client.propose_admin_action(
        &ActionType::ChangeAdmin,
        &ActionPayload::ChangeAdmin(new_admin.clone()),
    );

    // Advance time past the delay
    env.ledger().set_timestamp(start_timestamp + delay + 100);

    // Execute should succeed
    client.execute_after_delay(&action_id);

    // Admin should be changed
    let current_admin = client.get_admin();
    assert_eq!(current_admin, new_admin);

    // Action should be marked as executed
    let action = client.get_action(&action_id);
    assert_eq!(action.status, ActionStatus::Executed);
}

#[test]
#[should_panic(expected = "Error(Contract, #54)")] // ActionAlreadyExecuted
fn test_execute_already_executed_reverts() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);

    // Enable timelock with 2-hour delay
    let delay = 7200;
    client.configure_timelock(&delay, &true);

    let start_timestamp = env.ledger().timestamp();

    // Propose admin change
    let action_id = client.propose_admin_action(
        &ActionType::ChangeAdmin,
        &ActionPayload::ChangeAdmin(new_admin),
    );

    // Advance time and execute
    env.ledger().set_timestamp(start_timestamp + delay);
    client.execute_after_delay(&action_id);

    // Try to execute again
    client.execute_after_delay(&action_id);
}

#[test]
#[ignore]
fn test_cancel_pending_action() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);

    // Enable timelock
    client.configure_timelock(&7200, &true);

    // Propose admin change
    let action_id = client.propose_admin_action(
        &ActionType::ChangeAdmin,
        &ActionPayload::ChangeAdmin(new_admin),
    );

    // Cancel the action
    client.cancel_admin_action(&action_id);

    // Action should be marked as cancelled
    let action = client.get_action(&action_id);
    assert_eq!(action.status, ActionStatus::Cancelled);

    // Admin should not be changed
    let current_admin = client.get_admin();
    assert_eq!(current_admin, admin);
}

#[test]
#[should_panic(expected = "Error(Contract, #55)")] // ActionAlreadyCancelled
fn test_execute_cancelled_action_reverts() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);

    // Enable timelock
    client.configure_timelock(&7200, &true);

    // Propose admin change
    let action_id = client.propose_admin_action(
        &ActionType::ChangeAdmin,
        &ActionPayload::ChangeAdmin(new_admin),
    );

    // Cancel the action
    client.cancel_admin_action(&action_id);

    // Try to execute cancelled action
    client.execute_after_delay(&action_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #54)")] // ActionAlreadyExecuted
fn test_cancel_executed_action_reverts() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);

    // Enable timelock
    client.configure_timelock(&7200, &true);

    let start_timestamp = env.ledger().timestamp();

    // Propose admin change
    let action_id = client.propose_admin_action(
        &ActionType::ChangeAdmin,
        &ActionPayload::ChangeAdmin(new_admin),
    );

    // Execute the action
    env.ledger().set_timestamp(start_timestamp + 7200);
    client.execute_after_delay(&action_id);

    // Try to cancel executed action
    client.cancel_admin_action(&action_id);
}

#[test]
#[ignore]
#[should_panic(expected = "Error(Contract, #7)")] // Unauthorized
fn test_only_admin_can_cancel() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let non_admin = Address::generate(&env);

    client.init(&admin, &token);

    // Enable timelock
    client.configure_timelock(&7200, &true);

    // Propose admin change
    let action_id = client.propose_admin_action(
        &ActionType::ChangeAdmin,
        &ActionPayload::ChangeAdmin(new_admin),
    );

    // Try to cancel as non-admin
    env.mock_auths(&[&non_admin]);
    client.cancel_admin_action(&action_id);
}

#[test]
#[ignore]
fn test_non_admin_can_execute_after_delay() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let executor = Address::generate(&env);

    client.init(&admin, &token);

    // Enable timelock
    client.configure_timelock(&7200, &true);

    let start_timestamp = env.ledger().timestamp();

    // Propose admin change
    let action_id = client.propose_admin_action(
        &ActionType::ChangeAdmin,
        &ActionPayload::ChangeAdmin(new_admin.clone()),
    );

    // Advance time and execute as different address
    env.ledger().set_timestamp(start_timestamp + 7200);
    env.mock_auths(&[&executor]);
    client.execute_after_delay(&action_id);

    // Admin should be changed
    let current_admin = client.get_admin();
    assert_eq!(current_admin, new_admin);
}

#[test]
#[should_panic(expected = "Error(Contract, #52)")] // TimelockEnabled
fn test_direct_admin_call_blocked_when_enabled() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    // Enable timelock
    client.configure_timelock(&7200, &true);

    // Try direct admin call - should be blocked
    client.set_maintenance_mode(&true, &None);
}

#[test]
fn test_direct_admin_call_works_when_disabled() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    // Ensure timelock is disabled
    client.configure_timelock(&7200, &false);

    // Direct admin call should work
    client.set_maintenance_mode(&true, &None);

    // Verify maintenance mode is set
    assert!(client.is_maintenance_mode());
}

#[test]
fn test_get_pending_actions_ordered_by_time() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin1 = Address::generate(&env);
    let new_admin2 = Address::generate(&env);

    client.init(&admin, &token);

    // Enable timelock
    client.configure_timelock(&7200, &true);

    // Propose first action
    let action_id1 = client.propose_admin_action(
        &ActionType::ChangeAdmin,
        &ActionPayload::ChangeAdmin(new_admin1),
    );

    // Advance time a bit
    env.ledger().set_timestamp(env.ledger().timestamp() + 100);

    // Propose second action
    let action_id2 = client.propose_admin_action(
        &ActionType::ChangeAdmin,
        &ActionPayload::ChangeAdmin(new_admin2),
    );

    // Get pending actions
    let pending = client.get_pending_actions();
    assert_eq!(pending.len(), 2);

    // Should be ordered by proposed_at (earliest first)
    assert_eq!(pending[0].action_id, action_id1);
    assert_eq!(pending[1].action_id, action_id2);
    assert!(pending[0].proposed_at <= pending[1].proposed_at);
}

#[test]
fn test_change_fee_recipient_via_timelock() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_recipient = Address::generate(&env);

    client.init(&admin, &token);

    // Enable timelock
    client.configure_timelock(&7200, &true);

    let start_timestamp = env.ledger().timestamp();

    // Propose fee recipient change
    let action_id = client.propose_admin_action(
        &ActionType::ChangeFeeRecipient,
        &ActionPayload::ChangeFeeRecipient(new_recipient.clone()),
    );

    // Advance time and execute
    env.ledger().set_timestamp(start_timestamp + 7200);
    client.execute_after_delay(&action_id);

    // Verify fee recipient changed
    let fee_config = client.get_fee_config();
    assert_eq!(fee_config.fee_recipient, new_recipient);
}

#[test]
fn test_enable_kill_switch_via_timelock() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    // Enable timelock
    client.configure_timelock(&7200, &true);

    let start_timestamp = env.ledger().timestamp();

    // Propose kill switch enable
    let action_id = client.propose_admin_action(
        &ActionType::EnableKillSwitch,
        &ActionPayload::EnableKillSwitch,
    );

    // Advance time and execute
    env.ledger().set_timestamp(start_timestamp + 7200);
    client.execute_after_delay(&action_id);

    // Verify deprecation state
    let deprecation_status = client.get_deprecation_status();
    assert!(deprecation_status.deprecated);
}

#[test]
fn test_set_paused_via_timelock() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    // Enable timelock
    client.configure_timelock(&7200, &true);

    let start_timestamp = env.ledger().timestamp();

    // Propose pause state change
    let action_id = client.propose_admin_action(
        &ActionType::SetPaused,
        &ActionPayload::SetPaused(Some(true), Some(false), Some(false)),
    );

    // Advance time and execute
    env.ledger().set_timestamp(start_timestamp + 7200);
    client.execute_after_delay(&action_id);

    // Verify pause flags
    let pause_flags = client.get_pause_flags();
    assert!(pause_flags.lock_paused);
    assert!(!pause_flags.release_paused);
    assert!(!pause_flags.refund_paused);
}

#[test]
#[should_panic(expected = "Error(Contract, #56)")] // InvalidPayload
fn test_invalid_payload_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);

    // Enable timelock
    client.configure_timelock(&7200, &true);

    // Try to propose with mismatched payload
    client.propose_admin_action(
        &ActionType::ChangeAdmin,
        &ActionPayload::ChangeFeeRecipient(new_admin), // Wrong payload type
    );
}
