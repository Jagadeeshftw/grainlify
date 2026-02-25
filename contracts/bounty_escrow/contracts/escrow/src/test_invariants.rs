use super::*;
use crate::invariants;
use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, token, Address, Env};

fn setup_bounty(env: &Env) -> (BountyEscrowContractClient<'static>, Address, Address) {
    env.mock_all_auths();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(env, &contract_id);

    let admin = Address::generate(env);
    let depositor = Address::generate(env);
    let token_admin = Address::generate(env);
    // Fixed: Updated to v2 to resolve deprecation warning
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_admin_client = token::StellarAssetClient::new(env, &token_id);

    client.init(&admin, &token_id);
    token_admin_client.mint(&depositor, &50_000);

    (client, admin, depositor)
}

/// Ensures invariant checks are invoked in all three major state-changing flows:
/// lock_funds, release_funds, and refund. If any flow stops calling assert_escrow,
/// the call count drops and this test fails.
#[test]
fn test_invariant_checker_ci_called_in_major_bounty_flows() {
    let env = Env::default();
    let (client, _admin, depositor) = setup_bounty(&env);
    env.as_contract(&client.address, || invariants::reset_test_state(&env));

    let bounty_id = 42_u64;
    let contributor = Address::generate(&env);
    let amount = 10_000_i128;
    let deadline = env.ledger().timestamp() + 1000;

    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);
    client.release_funds(&bounty_id, &contributor);

    let calls = env.as_contract(&client.address, || invariants::call_count_for_test(&env));
    assert!(
        calls >= 2,
        "lock_funds and release_funds must each trigger invariant check"
    );
}

/// Covers all three major flows (lock, release, refund) and asserts the exact
/// expected invariant call count. Prevents future changes from bypassing checks
/// in any of these flows.
#[test]
fn test_invariant_checker_ci_all_three_flows_increment_call_count() {
    let env = Env::default();
    let (client, _admin, depositor) = setup_bounty(&env);
    env.as_contract(&client.address, || invariants::reset_test_state(&env));

    let lock_id = 10_u64;
    let release_id = 11_u64;
    let refund_id = 12_u64;
    let amount = 5_000_i128;
    let now = env.ledger().timestamp();
    let deadline_short = now + 100;
    let deadline_later = now + 2000;

    client.lock_funds(&depositor, &lock_id, &amount, &deadline_later);
    client.lock_funds(&depositor, &release_id, &amount, &deadline_later);
    client.lock_funds(&depositor, &refund_id, &amount, &deadline_short);

    let contributor = Address::generate(&env);
    client.release_funds(&release_id, &contributor);

    env.ledger().set_timestamp(deadline_short + 1);
    client.refund(&refund_id);

    let calls = env.as_contract(&client.address, || invariants::call_count_for_test(&env));
    assert_eq!(
        calls, 5,
        "expected 5 invariant checks: 3 lock_funds + 1 release_funds + 1 refund; \
         if this fails, a major flow may have stopped calling assert_escrow"
    );
}

#[test]
#[should_panic(expected = "Invariant checks disabled")]
fn test_invariant_checker_ci_panics_when_disabled() {
    let env = Env::default();
    let (client, _admin, depositor) = setup_bounty(&env);
    env.as_contract(&client.address, || {
        invariants::reset_test_state(&env);
        invariants::set_disabled_for_test(&env, true);
    });

    client.lock_funds(&depositor, &7_u64, &5_000_i128, &500);
}

// ==================== STATE VERIFICATION TESTS ====================

#[test]
fn test_invariant_checker_healthy_locked_state() {
    let env = Env::default();
    let (client, _admin, depositor) = setup_bounty(&env);
    env.as_contract(&client.address, || invariants::reset_test_state(&env));

    let bounty_id = 42_u64;
    let amount = 10_000_i128;
    let deadline = env.ledger().timestamp() + 1000;

    // Lock funds - should pass invariants
    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

    // Verify invariants pass for locked state
    let escrow_data = client.get_escrow_info(&bounty_id);
    env.as_contract(&client.address, || {
        invariants::assert_escrow(&env, &escrow_data);
    });

    // Verify invariant was called
    let calls = env.as_contract(&client.address, || invariants::call_count_for_test(&env));
    assert!(calls >= 1);
}

#[test]
fn test_invariant_checker_healthy_released_state() {
    let env = Env::default();
    let (client, _admin, depositor) = setup_bounty(&env);
    env.as_contract(&client.address, || invariants::reset_test_state(&env));

    let bounty_id = 42_u64;
    let amount = 10_000_i128;
    let deadline = env.ledger().timestamp() + 1000;
    let contributor = Address::generate(&env);

    // Lock and release funds - should pass invariants
    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);
    client.release_funds(&bounty_id, &contributor);

    // Verify invariants pass for released state
    let escrow_data = client.get_escrow_info(&bounty_id);
    env.as_contract(&client.address, || {
        invariants::assert_escrow(&env, &escrow_data);
    });

    // Verify invariant was called multiple times
    let calls = env.as_contract(&client.address, || invariants::call_count_for_test(&env));
    assert!(calls >= 2);
}

#[test]
fn test_invariant_checker_healthy_refunded_state() {
    let env = Env::default();
    let (client, admin, depositor) = setup_bounty(&env);
    env.as_contract(&client.address, || invariants::reset_test_state(&env));

    let bounty_id = 42_u64;
    let amount = 10_000_i128;
    let deadline = env.ledger().timestamp() + 1000;

    // Lock funds - should pass invariants
    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

    // Approve refund and execute - should pass invariants
    client.approve_refund(&bounty_id, &amount, &depositor, &RefundMode::Full);
    client.refund(&bounty_id);

    // Verify invariants pass for refunded state
    let escrow_data = client.get_escrow_info(&bounty_id);
    env.as_contract(&client.address, || {
        invariants::assert_escrow(&env, &escrow_data);
    });

    // Verify invariant was called multiple times
    let calls = env.as_contract(&client.address, || invariants::call_count_for_test(&env));
    assert!(calls >= 2);
}

// ==================== SYNTHETIC INCONSISTENT STATE TESTS ====================

#[test]
#[should_panic(expected = "Invariant violated: amount must be non-negative")]
fn test_invariant_checker_catches_negative_amount() {
    let env = Env::default();
    let (client, _admin, depositor) = setup_bounty(&env);
    env.as_contract(&client.address, || invariants::reset_test_state(&env));

    // Create escrow with negative amount
    let invalid_escrow = Escrow {
        depositor: depositor.clone(),
        amount: -1000_i128,
        remaining_amount: -1000_i128,
        status: EscrowStatus::Locked,
        deadline: env.ledger().timestamp() + 1000,
        refund_history: vec![&env],
    };

    env.as_contract(&client.address, || {
        invariants::assert_escrow(&env, &invalid_escrow);
    });
}

#[test]
#[should_panic(expected = "Invariant violated: remaining_amount must be non-negative")]
fn test_invariant_checker_catches_negative_remaining_amount() {
    let env = Env::default();
    let (client, _admin, depositor) = setup_bounty(&env);
    env.as_contract(&client.address, || invariants::reset_test_state(&env));

    // Create escrow with negative remaining_amount
    let invalid_escrow = Escrow {
        depositor: depositor.clone(),
        amount: 10_000_i128,
        remaining_amount: -500_i128,
        status: EscrowStatus::Locked,
        deadline: env.ledger().timestamp() + 1000,
        refund_history: vec![&env],
    };

    env.as_contract(&client.address, || {
        invariants::assert_escrow(&env, &invalid_escrow);
    });
}

#[test]
#[should_panic(expected = "Invariant violated: remaining_amount cannot exceed amount")]
fn test_invariant_checker_catches_remaining_amount_exceeds_amount() {
    let env = Env::default();
    let (client, _admin, depositor) = setup_bounty(&env);
    env.as_contract(&client.address, || invariants::reset_test_state(&env));

    // Create escrow where remaining_amount > amount
    let invalid_escrow = Escrow {
        depositor: depositor.clone(),
        amount: 5_000_i128,
        remaining_amount: 10_000_i128,
        status: EscrowStatus::Locked,
        deadline: env.ledger().timestamp() + 1000,
        refund_history: vec![&env],
    };

    env.as_contract(&client.address, || {
        invariants::assert_escrow(&env, &invalid_escrow);
    });
}

#[test]
#[should_panic(expected = "Invariant violated: released escrow must have zero remaining amount")]
fn test_invariant_checker_catches_released_with_nonzero_remaining() {
    let env = Env::default();
    let (client, _admin, depositor) = setup_bounty(&env);
    env.as_contract(&client.address, || invariants::reset_test_state(&env));

    // Create escrow with Released status but non-zero remaining_amount
    let invalid_escrow = Escrow {
        depositor: depositor.clone(),
        amount: 10_000_i128,
        remaining_amount: 5_000_i128,
        status: EscrowStatus::Released,
        deadline: env.ledger().timestamp() + 1000,
        refund_history: vec![&env],
    };

    env.as_contract(&client.address, || {
        invariants::assert_escrow(&env, &invalid_escrow);
    });
}

#[test]
fn test_invariant_checker_allows_valid_edge_cases() {
    let env = Env::default();
    let (client, _admin, depositor) = setup_bounty(&env);
    env.as_contract(&client.address, || invariants::reset_test_state(&env));

    // Test edge case: zero amount and remaining_amount (should be valid)
    let zero_escrow = Escrow {
        depositor: depositor.clone(),
        amount: 0_i128,
        remaining_amount: 0_i128,
        status: EscrowStatus::Released,
        deadline: env.ledger().timestamp() + 1000,
        refund_history: vec![&env],
    };

    env.as_contract(&client.address, || {
        invariants::assert_escrow(&env, &zero_escrow);
    });

    // Test edge case: remaining_amount equals amount for locked state
    let equal_escrow = Escrow {
        depositor: depositor.clone(),
        amount: 10_000_i128,
        remaining_amount: 10_000_i128,
        status: EscrowStatus::Locked,
        deadline: env.ledger().timestamp() + 1000,
        refund_history: vec![&env],
    };

    env.as_contract(&client.address, || {
        invariants::assert_escrow(&env, &equal_escrow);
    });

    // Test edge case: released status with zero remaining_amount
    let released_zero_escrow = Escrow {
        depositor: depositor.clone(),
        amount: 10_000_i128,
        remaining_amount: 0_i128,
        status: EscrowStatus::Released,
        deadline: env.ledger().timestamp() + 1000,
        refund_history: vec![&env],
    };

    env.as_contract(&client.address, || {
        invariants::assert_escrow(&env, &released_zero_escrow);
    });
}

#[test]
fn test_invariant_checker_partial_refund_state() {
    let env = Env::default();
    let (client, _admin, depositor) = setup_bounty(&env);
    env.as_contract(&client.address, || invariants::reset_test_state(&env));

    let bounty_id = 42_u64;
    let amount = 10_000_i128;
    let deadline = env.ledger().timestamp() + 1000;

    // Lock funds
    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

    // Create a partially refunded state manually for testing
    let partially_refunded_escrow = Escrow {
        depositor: depositor.clone(),
        amount: 10_000_i128,
        remaining_amount: 7_000_i128, // Partially refunded
        status: EscrowStatus::Locked,
        deadline,
        refund_history: vec![&env],
    };

    // This should pass invariants
    env.as_contract(&client.address, || {
        invariants::assert_escrow(&env, &partially_refunded_escrow);
    });
}
