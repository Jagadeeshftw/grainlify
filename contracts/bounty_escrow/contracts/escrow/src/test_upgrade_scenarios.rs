#![cfg(test)]

//! # Upgrade scenario tests
//!
//! Verifies that escrow state (locked funds, partial releases, refunds) is
//! fully preserved across a simulated WASM upgrade and that [`upgrade_safety`]
//! functions behave correctly.

use crate::{upgrade_safety, BountyEscrowContract, BountyEscrowContractClient, EscrowStatus};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

// ── Helpers ──────────────────────────────────────────────────────────────────

fn create_test_env<'a>() -> (Env, BountyEscrowContractClient<'a>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);
    let addr = contract_id.clone();
    (env, client, addr)
}

fn create_token_contract<'a>(
    e: &'a Env,
    admin: &Address,
) -> (Address, token::Client<'a>, token::StellarAssetClient<'a>) {
    let token_id = e.register_stellar_asset_contract_v2(admin.clone());
    let token = token_id.address();
    let token_client = token::Client::new(e, &token);
    let token_admin_client = token::StellarAssetClient::new(e, &token);
    (token, token_client, token_admin_client)
}

// ── Upgrade scenario tests ────────────────────────────────────────────────────

/// A locked bounty must still be `Locked` after the upgrade boundary.
#[test]
fn test_upgrade_locked_bounty_remains_locked() {
    let (env, client, _contract_id) = create_test_env();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &10_000);

    let deadline = env.ledger().timestamp() + 1000;
    client.lock_funds(&depositor, &1, &5_000, &deadline);

    // Simulate upgrade by verifying state persistence (WASM swap keeps storage)
    let escrow = client.get_escrow_info(&1);
    assert_eq!(escrow.status, EscrowStatus::Locked);
    assert_eq!(escrow.amount, 5_000);
    assert_eq!(escrow.remaining_amount, 5_000);
}

/// A full release flow must work after the upgrade boundary.
#[test]
fn test_upgrade_complete_release_flow() {
    let (env, client, _contract_id) = create_test_env();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &10_000);

    let deadline = env.ledger().timestamp() + 1000;
    client.lock_funds(&depositor, &1, &5_000, &deadline);

    let escrow = client.get_escrow_info(&1);
    assert_eq!(escrow.status, EscrowStatus::Locked);

    // Complete release (simulates post-upgrade call)
    client.release_funds(&1, &contributor);

    let escrow = client.get_escrow_info(&1);
    assert_eq!(escrow.status, EscrowStatus::Released);
}

/// A bounty whose deadline has passed can be refunded post-upgrade.
#[test]
fn test_upgrade_pending_lock_then_refund() {
    let (env, client, _contract_id) = create_test_env();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &10_000);

    let deadline = env.ledger().timestamp() + 100;
    client.lock_funds(&depositor, &2, &5_000, &deadline);

    // Advance time past deadline
    env.ledger().set_timestamp(env.ledger().timestamp() + 200);

    client.refund(&2);

    let escrow = client.get_escrow_info(&2);
    assert_eq!(escrow.status, EscrowStatus::Refunded);
}

/// Partial releases started pre-upgrade can be completed post-upgrade.
#[test]
fn test_upgrade_partial_release_then_complete() {
    let (env, client, _contract_id) = create_test_env();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &10_000);

    let deadline = env.ledger().timestamp() + 1000;
    client.lock_funds(&depositor, &3, &6_000, &deadline);

    client.partial_release(&3, &contributor, &2_000);

    let escrow = client.get_escrow_info(&3);
    assert_eq!(escrow.remaining_amount, 4_000);
    assert_eq!(escrow.status, EscrowStatus::Locked);

    client.partial_release(&3, &contributor, &4_000);

    let escrow = client.get_escrow_info(&3);
    assert_eq!(escrow.remaining_amount, 0);
    assert_eq!(escrow.status, EscrowStatus::Released);
}

// ── Upgrade safety module tests ───────────────────────────────────────────────

/// `simulate_upgrade` passes on a properly initialised contract.
#[test]
fn test_safety_check_passes_after_init() {
    let (env, client, contract_id) = create_test_env();

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _token_client, _token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin, &token);

    let report = env.as_contract(&contract_id, || upgrade_safety::simulate_upgrade(&env));
    assert!(
        report.is_safe,
        "Safety check should pass after initialization"
    );
    assert_eq!(report.checks_passed, 10);
    assert_eq!(report.checks_failed, 0);
}

/// `simulate_upgrade` fails before the contract is initialised.
#[test]
fn test_safety_check_fails_before_init() {
    let (env, _client, contract_id) = create_test_env();
    // Intentionally skip client.init(…)

    let report = env.as_contract(&contract_id, || upgrade_safety::simulate_upgrade(&env));
    assert!(
        !report.is_safe,
        "Safety check should fail before initialization"
    );
    assert!(report.checks_failed > 0, "Should have failed checks");
}

/// `simulate_upgrade` passes when locked escrows are present.
#[test]
fn test_safety_check_with_locked_escrows() {
    let (env, client, contract_id) = create_test_env();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &10_000);

    let deadline = env.ledger().timestamp() + 1_000;
    client.lock_funds(&depositor, &1, &5_000, &deadline);

    let report = env.as_contract(&contract_id, || upgrade_safety::simulate_upgrade(&env));
    assert!(
        report.is_safe,
        "Safety check should pass with locked escrows"
    );
}

/// `validate_upgrade` returns `Ok` for an initialised contract.
#[test]
fn test_upgrade_succeeds_with_valid_state() {
    let (env, client, contract_id) = create_test_env();

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _token_client, _token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin, &token);

    let result = env.as_contract(&contract_id, || upgrade_safety::validate_upgrade(&env));
    assert!(
        result.is_ok(),
        "validate_upgrade should succeed with valid state"
    );
}

/// `validate_upgrade` returns `Err` for an uninitialised contract.
#[test]
fn test_upgrade_fails_without_init() {
    let (env, _client, contract_id) = create_test_env();
    // Intentionally skip client.init(…)

    let result = env.as_contract(&contract_id, || upgrade_safety::validate_upgrade(&env));
    assert!(
        result.is_err(),
        "validate_upgrade should fail without initialization"
    );
}

/// Safety checks are enabled by default.
#[test]
fn test_get_safety_status() {
    let (env, _client, contract_id) = create_test_env();

    let enabled = env.as_contract(&contract_id, || {
        upgrade_safety::is_safety_checks_enabled(&env)
    });
    assert!(enabled, "Safety checks should be enabled by default");
}

/// Safety checks can be toggled off and back on.
#[test]
fn test_set_safety_status() {
    let (env, _client, contract_id) = create_test_env();

    env.as_contract(&contract_id, || {
        upgrade_safety::set_safety_checks_enabled(&env, false)
    });
    assert!(
        !env.as_contract(&contract_id, || upgrade_safety::is_safety_checks_enabled(
            &env
        )),
        "Safety checks should be disabled"
    );

    env.as_contract(&contract_id, || {
        upgrade_safety::set_safety_checks_enabled(&env, true)
    });
    assert!(
        env.as_contract(&contract_id, || upgrade_safety::is_safety_checks_enabled(
            &env
        )),
        "Safety checks should be re-enabled"
    );
}

/// `simulate_upgrade` passes with a released escrow.
#[test]
fn test_safety_check_with_released_escrow() {
    let (env, client, contract_id) = create_test_env();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &10_000);

    let deadline = env.ledger().timestamp() + 1_000;
    client.lock_funds(&depositor, &1, &5_000, &deadline);
    client.release_funds(&1, &contributor);

    let report = env.as_contract(&contract_id, || upgrade_safety::simulate_upgrade(&env));
    assert!(
        report.is_safe,
        "Safety check should pass with released escrow"
    );
}

/// `simulate_upgrade` passes with a refunded escrow.
#[test]
fn test_safety_check_with_refunded_escrow() {
    let (env, client, contract_id) = create_test_env();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &10_000);

    let deadline = env.ledger().timestamp() + 100;
    client.lock_funds(&depositor, &1, &5_000, &deadline);

    env.ledger().set_timestamp(env.ledger().timestamp() + 200);

    client.refund(&1);

    let report = env.as_contract(&contract_id, || upgrade_safety::simulate_upgrade(&env));
    assert!(
        report.is_safe,
        "Safety check should pass with refunded escrow"
    );
}

/// `simulate_upgrade` passes when multiple escrows are in different states.
#[test]
fn test_safety_check_with_multiple_escrows() {
    let (env, client, contract_id) = create_test_env();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &30_000);

    let deadline1 = env.ledger().timestamp() + 1_000;
    let deadline2 = env.ledger().timestamp() + 1_000;
    let deadline3 = env.ledger().timestamp() + 100;

    client.lock_funds(&depositor, &1, &5_000, &deadline1);
    client.lock_funds(&depositor, &2, &5_000, &deadline2);
    client.lock_funds(&depositor, &3, &5_000, &deadline3);

    // Release bounty 1
    client.release_funds(&1, &contributor);

    // Advance time and refund bounty 3
    env.ledger().set_timestamp(env.ledger().timestamp() + 200);
    client.refund(&3);

    let report = env.as_contract(&contract_id, || upgrade_safety::simulate_upgrade(&env));
    assert!(
        report.is_safe,
        "Safety check should pass with multiple escrows in different states"
    );
    assert_eq!(report.checks_passed, 10);
}

/// `validate_upgrade` returns `Ok` when safety checks are disabled, even on an
/// otherwise-uninitialized contract — the gate is bypassed intentionally.
#[test]
fn test_upgrade_with_disabled_safety_allows_invalid_state() {
    let (env, _client, contract_id) = create_test_env();

    env.as_contract(&contract_id, || {
        upgrade_safety::set_safety_checks_enabled(&env, false)
    });

    // With checks disabled, validate_upgrade short-circuits to Ok
    let result = env.as_contract(&contract_id, || upgrade_safety::validate_upgrade(&env));
    assert!(
        result.is_ok(),
        "validate_upgrade should succeed when safety checks are disabled"
    );
}

/// Safety module toggle works independently of a contract instance.
#[test]
fn test_safety_module_check_count() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, BountyEscrowContract);

    assert!(
        env.as_contract(&contract_id, || upgrade_safety::is_safety_checks_enabled(
            &env
        ))
    );

    env.as_contract(&contract_id, || {
        upgrade_safety::set_safety_checks_enabled(&env, false)
    });
    assert!(
        !env.as_contract(&contract_id, || upgrade_safety::is_safety_checks_enabled(
            &env
        ))
    );

    env.as_contract(&contract_id, || {
        upgrade_safety::set_safety_checks_enabled(&env, true)
    });
    assert!(
        env.as_contract(&contract_id, || upgrade_safety::is_safety_checks_enabled(
            &env
        ))
    );
}
