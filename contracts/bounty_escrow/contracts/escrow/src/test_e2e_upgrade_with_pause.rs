#![cfg(test)]

//! # End-to-End Upgrade with Pause Tests
//!
//! Validates the complete operational playbook for safely upgrading the
//! bounty escrow contract under pause:
//!
//! 1. **Pause** — Admin pauses all operations before upgrade.
//! 2. **Safety check** — `upgrade_safety::simulate_upgrade` validates state.
//! 3. **Upgrade** — Contract WASM is replaced (simulated by state persistence).
//! 4. **Unpause** — Admin resumes operations; existing escrows complete normally.
//!
//! ## Security assumptions
//!
//! - Only admin can toggle pause flags and trigger upgrades.
//! - Paused state is preserved across the upgrade boundary.
//! - No funds can move (lock/release/refund) while paused.
//! - Emergency withdraw is gated on `lock_paused = true`.
//! - Escrow data (amounts, statuses, deadlines) survives the upgrade.
//! - Balances remain consistent before, during, and after upgrade.

use crate::{
    upgrade_safety, BountyEscrowContract, BountyEscrowContractClient, Error, EscrowStatus,
};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Create a Stellar asset token and return both the standard and admin clients.
fn create_token_contract<'a>(
    e: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let contract_address = e
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    (
        token::Client::new(e, &contract_address),
        token::StellarAssetClient::new(e, &contract_address),
    )
}

/// Register a new bounty escrow contract instance.
fn create_escrow_contract<'a>(e: &Env) -> (BountyEscrowContractClient<'a>, Address) {
    let contract_id = e.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(e, &contract_id);
    (client, contract_id)
}

/// Full test harness: env + admin + depositor + contributor + token + escrow.
struct TestSetup<'a> {
    env: Env,
    admin: Address,
    depositor: Address,
    contributor: Address,
    token_client: token::Client<'a>,
    token_admin_client: token::StellarAssetClient<'a>,
    escrow_client: BountyEscrowContractClient<'a>,
    escrow_id: Address,
}

impl<'a> TestSetup<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);
        let token_admin = Address::generate(&env);

        let (token_client, token_admin_client) = create_token_contract(&env, &token_admin);
        let (escrow_client, escrow_id) = create_escrow_contract(&env);

        escrow_client.init(&admin, &token_client.address);
        token_admin_client.mint(&depositor, &500_000);
        env.ledger().set_timestamp(1);

        TestSetup {
            env,
            admin,
            depositor,
            contributor,
            token_client,
            token_admin_client,
            escrow_client,
            escrow_id,
        }
    }

    /// Lock a bounty and return its deadline.
    fn lock_bounty(&self, bounty_id: u64, amount: i128, deadline_offset: u64) -> u64 {
        let deadline = self.env.ledger().timestamp() + deadline_offset;
        self.escrow_client
            .lock_funds(&self.depositor, &bounty_id, &amount, &deadline);
        deadline
    }

    /// Pause all three operation classes.
    fn pause_all(&self, reason: &str) {
        let r = soroban_sdk::String::from_str(&self.env, reason);
        self.escrow_client
            .set_paused(&Some(true), &Some(true), &Some(true), &Some(r));
    }

    /// Unpause all three operation classes.
    fn unpause_all(&self) {
        self.escrow_client
            .set_paused(&Some(false), &Some(false), &Some(false), &None);
    }

    /// Advance ledger time to bypass rate limits (cooldown is 60s).
    fn advance_time(&self) {
        let current = self.env.ledger().timestamp();
        self.env.ledger().set_timestamp(current + 61);
    }
}

// ── Core E2E Upgrade Playbook ───────────────────────────────────────────────

/// Full operational playbook: lock → pause → safety check → (upgrade) → unpause → release.
///
/// This is the primary happy-path test that mirrors the procedure an operator
/// would follow in production.
#[test]
fn test_e2e_upgrade_with_pause() {
    let s = TestSetup::new();

    // Step 1: Pre-upgrade — lock funds into two bounties
    s.lock_bounty(1, 10_000, 1_000);
    s.env
        .ledger()
        .set_timestamp(s.env.ledger().timestamp() + 100);
    s.lock_bounty(2, 20_000, 1_000);

    let balance_before = s.token_client.balance(&s.escrow_id);
    assert_eq!(balance_before, 30_000);

    // Step 2: Pause all operations before upgrade
    s.advance_time();
    s.pause_all("Contract upgrade v2");

    let flags = s.escrow_client.get_pause_flags();
    assert!(flags.lock_paused);

    // Step 3: Run upgrade safety simulation while paused
    let report = s
        .env
        .as_contract(&s.escrow_id, || upgrade_safety::simulate_upgrade(&s.env));
    assert!(report.is_safe);
    assert_eq!(report.checks_failed, 0);

    // Step 4: Verify operations are blocked during the paused window
    s.advance_time();
    let deadline = s.env.ledger().timestamp() + 5_000;
    assert_eq!(
        s.escrow_client
            .try_lock_funds(&s.depositor, &3, &5_000, &deadline),
        Err(Ok(Error::FundsPaused))
    );

    assert_eq!(
        s.escrow_client.try_release_funds(&1, &s.contributor),
        Err(Ok(Error::FundsPaused))
    );

    assert_eq!(s.escrow_client.try_refund(&2), Err(Ok(Error::FundsPaused)));

    // Ensure balance hasn't moved
    let balance_during_pause = s.token_client.balance(&s.escrow_id);
    assert_eq!(balance_during_pause, balance_before);

    // Step 5: (Simulated) Upgrade happens here

    // Step 6: Post-upgrade — Unpause operations
    s.advance_time();
    s.unpause_all();

    let flags_after = s.escrow_client.get_pause_flags();
    assert!(!flags_after.lock_paused);

    // Step 7: Complete the original escrows successfully
    s.advance_time();
    s.escrow_client.release_funds(&1, &s.contributor);

    // Advance time past deadline to allow refund on bounty 2
    s.env
        .ledger()
        .set_timestamp(s.env.ledger().timestamp() + 2_000);
    s.escrow_client.refund(&2);

    // Verify final states
    assert_eq!(
        s.escrow_client.get_escrow_info(&1).status,
        EscrowStatus::Released
    );
    assert_eq!(
        s.escrow_client.get_escrow_info(&2).status,
        EscrowStatus::Refunded
    );

    assert_eq!(s.token_client.balance(&s.escrow_id), 0);
    assert_eq!(s.token_client.balance(&s.contributor), 10_000);
    assert_eq!(s.token_client.balance(&s.depositor), 490_000);
}

// ── Invariant: Balance Consistency ──────────────────────────────────────────

/// The token balance of the contract must not change during the pause/upgrade window.
#[test]
fn test_e2e_upgrade_with_pause_preserves_balance() {
    let s = TestSetup::new();

    s.lock_bounty(1, 15_000, 1_000);
    let balance_before = s.token_client.balance(&s.escrow_id);

    s.advance_time();
    s.pause_all("upgrade prep");

    let _report = s
        .env
        .as_contract(&s.escrow_id, || upgrade_safety::simulate_upgrade(&s.env));

    let balance_during_pause = s.token_client.balance(&s.escrow_id);
    assert_eq!(balance_during_pause, balance_before);

    s.advance_time();
    s.unpause_all();

    let balance_after = s.token_client.balance(&s.escrow_id);
    assert_eq!(balance_after, balance_before);
}

// ── Full Lifecycle: Lock → Pause → Upgrade → Unpause → Release → New Lock ──

/// End-to-end lifecycle covering every phase of the upgrade playbook with
/// multiple bounties and all operation types.
#[test]
fn test_full_upgrade_lifecycle() {
    let s = TestSetup::new();
    let contributor_2 = Address::generate(&s.env);

    // Phase 1: Normal operations — create bounties
    s.lock_bounty(10, 10_000, 5_000);
    s.env
        .ledger()
        .set_timestamp(s.env.ledger().timestamp() + 100);
    s.lock_bounty(20, 20_000, 5_000);
    s.env
        .ledger()
        .set_timestamp(s.env.ledger().timestamp() + 100);
    s.lock_bounty(30, 30_000, 100); // short deadline for refund test

    let total_locked = s.token_client.balance(&s.escrow_id);
    assert_eq!(total_locked, 60_000);

    // Phase 2: Pause for upgrade
    s.advance_time();
    s.pause_all("Upgrade to v3");

    // Phase 3: Safety check
    let report = s
        .env
        .as_contract(&s.escrow_id, || upgrade_safety::simulate_upgrade(&s.env));
    assert_eq!(report.checks_failed, 0);

    // Phase 4: All ops blocked
    let deadline = s.env.ledger().timestamp() + 10_000;
    assert!(s
        .escrow_client
        .try_lock_funds(&s.depositor, &99, &1_000, &deadline)
        .is_err());
    assert!(s
        .escrow_client
        .try_release_funds(&10, &s.contributor)
        .is_err());
    assert!(s.escrow_client.try_refund(&30).is_err());

    // Phase 5: Unpause
    s.unpause_all();

    // Phase 6: Resume normal operations
    s.advance_time();
    // Release bounty 10
    s.escrow_client.release_funds(&10, &s.contributor);
    assert_eq!(
        s.escrow_client.get_escrow_info(&10).status,
        EscrowStatus::Released
    );

    s.advance_time();
    // Release bounty 20 to a different contributor
    s.escrow_client.release_funds(&20, &contributor_2);
    assert_eq!(
        s.escrow_client.get_escrow_info(&20).status,
        EscrowStatus::Released
    );

    // Advance time and refund bounty 30
    s.env
        .ledger()
        .set_timestamp(s.env.ledger().timestamp() + 200);
    s.escrow_client.refund(&30);
    assert_eq!(
        s.escrow_client.get_escrow_info(&30).status,
        EscrowStatus::Refunded
    );

    // Phase 7: New lock after upgrade
    s.advance_time();
    s.lock_bounty(40, 8_000, 1_000);
    assert_eq!(
        s.escrow_client.get_escrow_info(&40).status,
        EscrowStatus::Locked
    );

    // Balance accounting correct
    assert_eq!(s.token_client.balance(&s.contributor), 10_000);
    assert_eq!(s.token_client.balance(&contributor_2), 20_000);
}

// ── Partial Release Across Upgrade ──────────────────────────────────────────

/// Partial releases started pre-pause can be completed post-unpause.
#[test]
fn test_partial_release_across_upgrade_boundary() {
    let s = TestSetup::new();

    s.lock_bounty(1, 10_000, 1_000);

    // Partial release before pause
    s.advance_time();
    s.escrow_client.partial_release(&1, &s.contributor, &3_000);
    let escrow = s.escrow_client.get_escrow_info(&1);
    assert_eq!(escrow.remaining_amount, 7_000);
    assert_eq!(escrow.status, EscrowStatus::Locked);

    // Pause → "upgrade" → unpause
    s.advance_time();
    s.pause_all("mid-release upgrade");
    s.advance_time();
    s.unpause_all();

    // Complete the remaining release
    s.advance_time();
    s.escrow_client.partial_release(&1, &s.contributor, &7_000);
    let escrow_after_second_partial = s.escrow_client.get_escrow_info(&1); // Renamed variable
    assert_eq!(escrow_after_second_partial.remaining_amount, 0); // Used renamed variable
    assert_eq!(escrow_after_second_partial.status, EscrowStatus::Released); // Used renamed variable
                                                                            // 3,000 + 7,000 = 10,000
    assert_eq!(s.token_client.balance(&s.contributor), 10_000);
}

// ── Multiple Bounties in Mixed States ───────────────────────────────────────

/// Upgrade with bounties in Locked, Released, and Refunded states.
#[test]
fn test_upgrade_with_mixed_escrow_states() {
    let s = TestSetup::new();

    // Bounty 1: will be released pre-pause
    s.lock_bounty(1, 10_000, 1_000);
    s.advance_time();
    s.escrow_client.release_funds(&1, &s.contributor);

    // Bounty 2: will be refunded pre-pause
    let short_deadline = 50;
    s.lock_bounty(2, 5_000, short_deadline);
    s.env
        .ledger()
        .set_timestamp(s.env.ledger().timestamp() + short_deadline + 1);
    s.advance_time();
    s.escrow_client.refund(&2);

    // Bounty 3: still locked when pause begins
    s.advance_time();
    s.lock_bounty(3, 20_000, 5_000);

    // Pause + safety check
    s.advance_time();
    s.pause_all("mixed state upgrade");

    let report = s
        .env
        .as_contract(&s.escrow_id, || upgrade_safety::simulate_upgrade(&s.env));
    assert_eq!(report.checks_failed, 0);

    // Verify all states preserved
    assert_eq!(
        s.escrow_client.get_escrow_info(&1).status,
        EscrowStatus::Released
    );
    assert_eq!(
        s.escrow_client.get_escrow_info(&2).status,
        EscrowStatus::Refunded
    );
    assert_eq!(
        s.escrow_client.get_escrow_info(&3).status,
        EscrowStatus::Locked
    );

    s.advance_time();
    s.unpause_all();

    // Bounty 3 can be released after upgrade
    s.advance_time();
    s.escrow_client.release_funds(&3, &s.contributor);
    assert_eq!(
        s.escrow_client.get_escrow_info(&3).status,
        EscrowStatus::Released
    );
}

// ── Safety Check Before Init ────────────────────────────────────────────────

/// Safety simulation on an uninitialized contract must report failure.
#[test]
fn test_safety_check_fails_before_init() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, BountyEscrowContract);

    let report = env.as_contract(&contract_id, || upgrade_safety::simulate_upgrade(&env));
    assert!(!report.is_safe, "Uninitialized contract should fail safety");
    assert!(report.checks_failed > 0);
}

/// Safety simulation on a properly initialized contract passes.
#[test]
fn test_safety_check_passes_after_init() {
    let s = TestSetup::new();
    let report = s
        .env
        .as_contract(&s.escrow_id, || upgrade_safety::simulate_upgrade(&s.env));
    assert!(report.is_safe);
    assert_eq!(report.checks_failed, 0);
}

// ── Safety Check with Locked Escrows ────────────────────────────────────────

/// Locked escrows are expected state — safety check should still pass.
#[test]
fn test_safety_check_passes_with_locked_escrows() {
    let s = TestSetup::new();
    s.lock_bounty(1, 5_000, 1_000);
    s.advance_time();
    s.lock_bounty(2, 15_000, 1_000);

    let report = s
        .env
        .as_contract(&s.escrow_id, || upgrade_safety::simulate_upgrade(&s.env));
    assert!(report.is_safe);
    assert_eq!(report.checks_passed, 10);
}

// ── Double Pause/Unpause Idempotency ────────────────────────────────────────

/// Pausing an already-paused contract is idempotent.
#[test]
fn test_double_pause_is_idempotent() {
    let s = TestSetup::new();

    s.pause_all("first pause");
    s.pause_all("second pause");

    let flags = s.escrow_client.get_pause_flags();
    assert!(flags.lock_paused);
    assert!(flags.release_paused);
    assert!(flags.refund_paused);

    s.unpause_all();
    s.unpause_all();

    let flags = s.escrow_client.get_pause_flags();
    assert!(!flags.lock_paused);
    assert!(!flags.release_paused);
    assert!(!flags.refund_paused);
}

// ── Emergency Withdraw After Upgrade Preserves Pause ────────────────────────

/// After emergency withdraw, contract remains paused and new locks are blocked.
#[test]
fn test_emergency_withdraw_preserves_pause_state() {
    let s = TestSetup::new();
    let target = Address::generate(&s.env);

    s.lock_bounty(1, 25_000, 1_000);

    s.advance_time();
    s.pause_all("emergency scenario");
    s.advance_time();
    s.escrow_client.emergency_withdraw(&target);

    // Contract is still paused
    let flags = s.escrow_client.get_pause_flags();
    assert!(flags.lock_paused);

    // Cannot lock new funds while paused
    s.advance_time();
    let deadline = s.env.ledger().timestamp() + 2_000;
    assert_eq!(
        s.escrow_client
            .try_lock_funds(&s.depositor, &99, &100, &deadline),
        Err(Ok(Error::FundsPaused))
    );

    // Admin can unpause and resume after draining
    s.advance_time();
    s.unpause_all();

    // Disable invariants as the contract was manually drained
    s.env.as_contract(&s.escrow_id, || {
        crate::invariants::set_disabled_for_test(&s.env, true);
    });

    s.advance_time();
    s.escrow_client
        .lock_funds(&s.depositor, &99, &100, &deadline);
    assert_eq!(
        s.escrow_client.get_escrow_info(&99).status,
        EscrowStatus::Locked
    );
}

// ── Upgrade Safety Toggle ───────────────────────────────────────────────────

/// Safety checks can be disabled and re-enabled via the module API.
#[test]
fn test_upgrade_safety_toggle() {
    let env = Env::default();
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

// ── Validate Upgrade Gate ───────────────────────────────────────────────────

/// `validate_upgrade` returns Ok when the contract is properly initialized.
#[test]
fn test_validate_upgrade_ok_for_initialized_contract() {
    let s = TestSetup::new();
    let result = s
        .env
        .as_contract(&s.escrow_id, || upgrade_safety::validate_upgrade(&s.env));
    assert!(result.is_ok());
}

/// `validate_upgrade` returns Err for uninitialized contract.
#[test]
fn test_validate_upgrade_fails_for_uninitialized() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, BountyEscrowContract);

    let result = env.as_contract(&contract_id, || upgrade_safety::validate_upgrade(&env));
    assert!(result.is_err());
}

/// `validate_upgrade` skips checks when safety is disabled.
#[test]
fn test_validate_upgrade_skips_when_disabled() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, BountyEscrowContract);

    // Would fail — contract not initialized
    env.as_contract(&contract_id, || {
        upgrade_safety::set_safety_checks_enabled(&env, false)
    });
    let result = env.as_contract(&contract_id, || upgrade_safety::validate_upgrade(&env));
    assert!(result.is_ok(), "Should skip checks when disabled");
}

// ── Safety Check Records Timestamp ──────────────────────────────────────────

/// Running `simulate_upgrade` records the timestamp for audit trail.
#[test]
fn test_safety_check_records_timestamp() {
    let s = TestSetup::new();

    assert!(s
        .env
        .as_contract(&s.escrow_id, || upgrade_safety::get_last_safety_check(
            &s.env
        ))
        .is_none());

    s.env.ledger().set_timestamp(12345);
    let _report = s
        .env
        .as_contract(&s.escrow_id, || upgrade_safety::simulate_upgrade(&s.env));

    let ts = s.env.as_contract(&s.escrow_id, || {
        upgrade_safety::get_last_safety_check(&s.env)
    });
    assert_eq!(ts, Some(12345));
}

// ── Pause Reason Stored ─────────────────────────────────────────────────────

/// Pause reason is recorded and readable from flags.
#[test]
fn test_pause_reason_stored_in_flags() {
    let s = TestSetup::new();
    s.env.ledger().set_timestamp(5555);

    let reason = soroban_sdk::String::from_str(&s.env, "Upgrade to v2.0.1");
    s.escrow_client
        .set_paused(&Some(true), &Some(true), &Some(true), &Some(reason));

    let flags = s.escrow_client.get_pause_flags();
    assert!(flags.pause_reason.is_some());
    assert_eq!(flags.paused_at, 5555);
}

// ── Edge Case 1: Error Codes on Uninitialized Contract ──────────────────────

#[test]
fn test_safety_report_error_codes() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, BountyEscrowContract);

    let report = env.as_contract(&contract_id, || upgrade_safety::simulate_upgrade(&env));
    assert!(!report.is_safe);
    assert!(report.checks_failed > 0);

    // Should have INITIALIZATION error since we didn't call init()
    let init_error = report
        .errors
        .iter()
        .find(|e| e.code == crate::upgrade_safety::safety_codes::INITIALIZATION);
    assert!(
        init_error.is_some(),
        "Expected INITIALIZATION error but not found"
    );
}

// ── Edge Case 2: Balance Remains Consistent Across 5 Cycles ─────────────────

#[test]
fn test_multiple_pause_resume_cycles_balance() {
    let s = TestSetup::new();

    s.lock_bounty(1, 5_000, 1_000);
    s.advance_time();
    s.lock_bounty(2, 5_000, 1_000);

    let expected_balance = 10_000;

    for i in 0..5 {
        s.advance_time();
        s.pause_all("Cycle");
        assert_eq!(s.token_client.balance(&s.escrow_id), expected_balance);

        s.env
            .ledger()
            .set_timestamp(s.env.ledger().timestamp() + 100);
        s.advance_time();
        s.unpause_all();
        assert_eq!(s.token_client.balance(&s.escrow_id), expected_balance);
        s.advance_time();
    }

    s.advance_time();
    s.escrow_client.release_funds(&1, &s.contributor);
    s.advance_time();
    s.escrow_client.release_funds(&2, &s.contributor);

    assert_eq!(s.token_client.balance(&s.escrow_id), 0);
    assert_eq!(s.token_client.balance(&s.contributor), 10_000);
}

// ── Edge Case 3: High-Value Bounties Through Upgrade Cycle ──────────────────

#[test]
fn test_upgrade_with_high_value_bounties() {
    let s = TestSetup::new();

    let high_value: i128 = 100_000_000 * 10_000_000; // 100M tokens with 7 decimals

    // Setup rich depositor
    s.token_admin_client.mint(&s.depositor, &(high_value * 3));

    // Lock 3 high-value bounties
    let deadline = s.env.ledger().timestamp() + 100_000;
    s.escrow_client
        .lock_funds(&s.depositor, &100, &high_value, &deadline);
    s.advance_time();
    s.escrow_client
        .lock_funds(&s.depositor, &200, &high_value, &deadline);
    s.advance_time();
    s.escrow_client
        .lock_funds(&s.depositor, &300, &high_value, &deadline);

    assert_eq!(s.token_client.balance(&s.escrow_id), high_value * 3);

    // Pause
    s.advance_time();
    s.pause_all("High value upgrade prep");

    // "Upgrade" dummy step
    s.env.as_contract(&s.escrow_id, || {
        crate::upgrade_safety::simulate_upgrade(&s.env)
    });

    // Unpause
    s.advance_time();
    s.unpause_all();

    // Release
    s.advance_time();
    s.escrow_client.release_funds(&100, &s.contributor);
    s.advance_time();
    s.escrow_client.release_funds(&200, &s.contributor);
    s.advance_time();
    s.escrow_client.release_funds(&300, &s.contributor);

    assert_eq!(s.token_client.balance(&s.escrow_id), 0);
    assert_eq!(s.token_client.balance(&s.contributor), high_value * 3);
}
