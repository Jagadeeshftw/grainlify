#![cfg(test)]

extern crate std;

use super::*;
use crate::test_support::*;
use soroban_sdk::{testutils::{Address as _, Events, Ledger, MockAuth, MockAuthInvoke}, token, vec, Address, Env, IntoVal, Map, String, Symbol, TryFromVal, Val};

#[test]
fn test_admin_rotation() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);

    env.mock_all_auths();

    client.set_admin(&admin);
    assert_eq!(client.get_admin(), Some(admin.clone()));

    client.set_admin(&new_admin);
    assert_eq!(client.get_admin(), Some(new_admin));
}

/// After admin rotation, new admin can update rate limit config.
#[test]
fn test_new_admin_can_update_config() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);

    env.mock_all_auths();

    client.set_admin(&admin);
    client.set_admin(&new_admin);

    client.update_rate_limit_config(&3600, &10, &30);

    let config = client.get_rate_limit_config();
    assert_eq!(config.window_size, 3600);
    assert_eq!(config.max_operations, 10);
    assert_eq!(config.cooldown_period, 30);
}

/// Non-admin cannot update rate limit config.
#[test]
#[should_panic]
fn test_non_admin_cannot_update_config() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
}

// ==================== ROLE SEPARATION TESTS ====================

/// Test admin proposal with deterministic behavior and explicit errors.
#[test]
fn test_admin_rotation_proposal_success() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);

    // Initialize contract with admin
    client.initialize_contract(&admin);

    // Propose new admin
    client.propose_admin(&new_admin);

    // Check that proposal was stored
    let events = env.events().all();
    assert!(events.len() >= 2); // init + propose

    // Verify schema version is set
    let schema_version = client.get_role_management_schema_version();
    assert_eq!(schema_version, 1);
}

/// Test that a newer admin proposal overwrites any existing pending proposal.
#[test]
fn test_admin_rotation_latest_proposal_overwrites_previous() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let new_admin1 = Address::generate(&env);
    let new_admin2 = Address::generate(&env);

    // Initialize contract with admin
    client.initialize_contract(&admin);

    // Propose first admin, then replace it with a second proposal.
    client.propose_admin(&new_admin1);
    client.propose_admin(&new_admin2);

    // The pending proposal should now point at the newer candidate.
    let pending_admin: Address = env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .get(&DataKey::PendingAdmin)
            .unwrap()
    });
    assert_eq!(pending_admin, new_admin2);
}

/// Test admin rotation acceptance success.
#[test]
fn test_admin_rotation_acceptance_success() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);

    // Initialize contract with admin
    client.initialize_contract(&admin);

    // Propose new admin at timestamp T
    env.ledger().with_mut(|li| li.timestamp = 1_000_000);
    client.propose_admin(&new_admin);

    // Advance time past the 24h timelock before accepting
    env.ledger().with_mut(|li| li.timestamp = 1_000_000 + ROTATION_TIMELOCK_DELAY);

    // Accept admin role
    client.accept_admin();

    // Verify admin was updated
    let current_admin = client.get_admin().unwrap();
    assert_eq!(current_admin, new_admin);

    // Check events
    let events = env.events().all();
    assert!(events.len() >= 3); // init + propose + accept
}

/// Test admin rotation acceptance without proposal.
#[test]
#[should_panic(expected = "No admin rotation in progress")]
fn test_admin_acceptance_without_proposal() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);

    // Initialize contract with admin
    client.initialize_contract(&admin);

    // Try to accept without proposal - should fail
    client.accept_admin();
}

/// Test admin rotation cancellation success.
#[test]
fn test_admin_rotation_cancellation_success() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);

    // Initialize contract with admin
    client.initialize_contract(&admin);

    // Propose new admin
    client.propose_admin(&new_admin);

    // Cancel admin rotation
    client.cancel_admin_rotation();

    // Verify original admin is still in place
    let current_admin = client.get_admin().unwrap();
    assert_eq!(current_admin, admin);
}

/// Test controller rotation proposal with deterministic behavior.
#[test]
fn test_controller_rotation_proposal_success() {
    let env = Env::default();
    let (client, admin, _token, _token_admin) = setup_program(&env, 0);

    let new_controller = Address::generate(&env);
    let program_id = String::from_str(&env, "hack-2026");

    // Propose new controller
    client.propose_controller(&program_id, &admin, &new_controller);

    // Check that proposal was stored
    let events = env.events().all();
    assert!(events.len() >= 3); // init + publish + propose

    // Verify schema version is set
    let schema_version = client.get_role_management_schema_version();
    assert_eq!(schema_version, 1);
}

/// Test controller rotation already in progress error.
#[test]
#[should_panic(expected = "Controller rotation already in progress")]
fn test_controller_rotation_already_in_progress() {
    let env = Env::default();
    let (client, admin, _token, _token_admin) = setup_program(&env, 0);

    let new_controller1 = Address::generate(&env);
    let new_controller2 = Address::generate(&env);
    let program_id = String::from_str(&env, "hack-2026");

    // Propose first controller
    client.propose_controller(&program_id, &admin, &new_controller1);

    // Try to propose second controller - should fail
    client.propose_controller(&program_id, &admin, &new_controller2);
}

/// Test controller rotation acceptance success.
#[test]
fn test_controller_rotation_acceptance_success() {
    let env = Env::default();
    let (client, admin, _token, _token_admin) = setup_program(&env, 0);

    let new_controller = Address::generate(&env);
    let program_id = String::from_str(&env, "hack-2026");

    // Propose new controller at timestamp T
    env.ledger().with_mut(|li| li.timestamp = 1_000_000);
    client.propose_controller(&program_id, &admin, &new_controller);

    // Advance time past the 24h timelock before accepting
    env.ledger().with_mut(|li| li.timestamp = 1_000_000 + ROTATION_TIMELOCK_DELAY);

    // Accept controller role
    client.accept_controller(&program_id);

    // Verify controller was updated
    let program_info = client.get_program_info();
    assert_eq!(program_info.authorized_payout_key, new_controller);

    // Check events
    let events = env.events().all();
    assert!(events.len() >= 4); // init + publish + propose + accept
}

/// Test controller rotation cancellation success.
#[test]
fn test_controller_rotation_cancellation_success() {
    let env = Env::default();
    let (client, admin, _token, _token_admin) = setup_program(&env, 0);

    let new_controller = Address::generate(&env);
    let program_id = String::from_str(&env, "hack-2026");
    let original_controller = client.get_program_info().authorized_payout_key;

    // Propose new controller
    client.propose_controller(&program_id, &admin, &new_controller);

    // Cancel controller rotation
    client.cancel_controller_rotation(&program_id, &admin);

    // Verify original controller is still in place
    let program_info = client.get_program_info();
    assert_eq!(program_info.authorized_payout_key, original_controller);
}

/// Test role rotation blocked during emergency mode.
#[test]
#[should_panic(expected = "Role rotation not allowed")]
fn test_role_rotation_blocked_during_emergency() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);

    // Initialize contract with admin
    client.initialize_contract(&admin);

    // Enable read-only mode (emergency)
    client.set_read_only_mode(&true);

    // Try to propose admin - should fail
    client.propose_admin(&new_admin);
}

/// Test invalid role proposal error.
#[test]
#[should_panic(expected = "Invalid role proposal")]
fn test_invalid_role_proposal() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);

    // Initialize contract with admin
    client.initialize_contract(&admin);

    // Try to propose same admin - should fail
    client.propose_admin(&admin);
}

/// Non-admins cannot update the global rate-limit configuration.
#[test]
#[should_panic(expected = "Unauthorized")]
fn test_non_admin_cannot_update_rate_limit_config() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let non_admin = Address::generate(&env);

    env.mock_all_auths();
    client.set_admin(&admin);

    // Mock only non_admin so that update_rate_limit_config sees non_admin as caller;
    // contract requires admin.require_auth(), so this must panic.
    env.mock_auths(&[MockAuth {
        address: &non_admin,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "update_rate_limit_config",
            args: (3600u64, 10u32, 30u64).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    client.update_rate_limit_config(&3600, &10, &30);
}

// ============================================================================
// TESTS FOR batch_initialize_programs
// ============================================================================
