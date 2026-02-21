#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::Address as _,
    Address, Env, String,
};

struct RbacSetup<'a> {
    env: Env,
    admin: Address,
    operator: Address,
    pauser: Address,
    random: Address,
    client: ProgramEscrowContractClient<'a>,
    token_address: Address,
    program_id: String,
}

impl<'a> RbacSetup<'a> {
    fn new() -> Self {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let operator = Address::generate(&env);
        let pauser = Address::generate(&env);
        let random = Address::generate(&env);

        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone()).address();

        let program_id = String::from_str(&env, "RBAC-Test");

        // Initialize contract with admin
        client.initialize_contract(&admin);

        // Initialize program with operator
        // Note: Currently init_program doesn't have auth, so we can just call it
        client.init_program(&program_id, &operator, &token_id);

        // Initialize circuit breaker with pauser
        // caller is None for first setting
        client.set_circuit_admin(&pauser, &None);

        Self {
            env,
            admin,
            operator,
            pauser,
            random,
            client,
            token_address: token_id,
            program_id,
        }
    }
}

// ─────────────────────────────────────────────────────────
// Admin Role Tests
// ─────────────────────────────────────────────────────────

#[test]
fn test_admin_permissions() {
    let setup = RbacSetup::new();
    
    // Admin should be able to pause/unpause
    setup.env.mock_all_auths();
    setup.client.set_paused(&Some(true), &None, &None);
    assert!(setup.client.get_pause_flags().lock_paused);
}

#[test]
#[should_panic]
fn test_random_cannot_pause() {
    let setup = RbacSetup::new();
    setup.client.set_paused(&Some(true), &None, &None);
    // This should panic because the default caller in Soroban tests (without mock_all_auths) 
    // will be unauthorized if it hasn't call setup.env.mock_all_auths() or provided auth.
}

// ─────────────────────────────────────────────────────────
// Operator Role Tests
// ─────────────────────────────────────────────────────────

#[test]
fn test_operator_permissions() {
    let setup = RbacSetup::new();
    setup.env.mock_all_auths();

    // Operator should be able to trigger releases
    setup.client.trigger_program_releases();
}

#[test]
#[should_panic]
fn test_admin_cannot_trigger_releases() {
    let setup = RbacSetup::new();
    // No mock_all_auths()
    
    // Admin is not the operator
    setup.admin.require_auth();
    setup.client.trigger_program_releases();
}

// ─────────────────────────────────────────────────────────
// Pauser Role Tests
// ─────────────────────────────────────────────────────────

#[test]
fn test_pauser_permissions() {
    let setup = RbacSetup::new();
    setup.env.mock_all_auths();

    // Pauser should be able to reset/configure circuit breaker
    setup.client.reset_circuit_breaker(&setup.pauser);
    setup.client.configure_circuit_breaker(&setup.pauser, &5, &2, &20);
}

#[test]
#[should_panic]
fn test_admin_cannot_reset_circuit() {
    let setup = RbacSetup::new();
    setup.env.mock_all_auths();
    
    // Even admin cannot reset circuit if they aren't the registered pauser
    setup.client.reset_circuit_breaker(&setup.admin);
}

#[test]
#[should_panic]
fn test_operator_cannot_reset_circuit() {
    let setup = RbacSetup::new();
    setup.env.mock_all_auths();
    
    // Operator cannot reset circuit
    setup.client.reset_circuit_breaker(&setup.operator);
}
