#[cfg(test)]
mod test {
    //! # Bounty Escrow State Verification Tests
    //!
    //! Closes #787
    //!
    //! This module ensures that on-chain contract state matches expectations after
    //! various operation sequences. These tests are designed to support **migration
    //! audits** by verifying state invariants hold after complex multi-step workflows.
    //!
    //! ## Verification Procedure
    //!
    //! 1. **Pre-condition**: Initialize contract and lock escrows.
    //! 2. **Action sequence**: Execute a series of operations (lock, release, refund,
    //!    admin approval, partial refund, etc.).
    //! 3. **Post-condition**: Verify each escrow's state matches the expected outcome:
    //!    - `verify_state(bounty_id)` returns `true` (invariants hold)
    //!    - `get_escrow_info(bounty_id)` fields match expected values
    //!    - Token balances reconcile with escrow remaining amounts
    //!    - Aggregate stats are consistent with individual escrow states
    //!
    //! ## Invariants Checked by `verify_state`
    //!
    //! - `amount >= 0` (non-negative total)
    //! - `remaining_amount >= 0` (non-negative remainder)
    //! - `remaining_amount <= amount` (no over-allocation)
    //! - If `status == Released` then `remaining_amount == 0` (fully disbursed)
    //!
    //! ## Security Notes
    //!
    //! - Tamper detection tests write directly to contract storage to simulate
    //!   corruption and verify that `verify_state` correctly rejects invalid states.
    //! - All state mutations go through the contract's public API in normal tests,
    //!   ensuring Soroban's auth and invariant checks are exercised.
    //! - Token balance reconciliation ensures no funds are lost or created.

    use crate::{
        BountyEscrowContract, BountyEscrowContractClient, DataKey, EscrowStatus, RefundMode,
    };
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::{token, Address, Env};

    /// Shared setup: initializes contract, mints tokens, returns all handles.
    fn setup_bounty(
        env: &Env,
    ) -> (
        BountyEscrowContractClient<'static>,
        Address,
        Address,
        Address,
        Address,
        token::Client<'static>,
    ) {
        env.mock_all_auths();
        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(env, &contract_id);

        let admin = Address::generate(env);
        let depositor = Address::generate(env);
        let token_admin = Address::generate(env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_id.address();
        let token_admin_client = token::StellarAssetClient::new(env, &token_address);
        let token_client = token::Client::new(env, &token_address);

        client.init(&admin, &token_address);
        token_admin_client.mint(&depositor, &100_000);
        (
            client,
            contract_id,
            admin,
            depositor,
            token_address,
            token_client,
        )
    }

    // =========================================================================
    // 1. Basic invariant verification – healthy states
    // =========================================================================

    /// Freshly locked escrow passes all invariant checks.
    #[test]
    fn test_bounty_healthy_state_passes() {
        let env = Env::default();
        let (client, _contract_id, _admin, depositor, _token_id, _token) = setup_bounty(&env);

        let bounty_id = 1u64;
        let amount = 1000i128;
        let deadline = env.ledger().timestamp() + 100;

        client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

        assert!(client.verify_state(&bounty_id));
    }

    /// Released escrow passes invariant checks (remaining_amount == 0).
    #[test]
    fn test_released_state_passes_verification() {
        let env = Env::default();
        let (client, _contract_id, _admin, depositor, _token_id, _token) = setup_bounty(&env);

        let contributor = Address::generate(&env);
        let bounty_id = 2u64;

        client.lock_funds(&depositor, &bounty_id, &1000i128, &100);
        client.release_funds(&bounty_id, &contributor);

        assert!(client.verify_state(&bounty_id));
        let info = client.get_escrow_info(&bounty_id);
        assert_eq!(info.status, EscrowStatus::Released);
        assert_eq!(info.remaining_amount, 0);
    }

    /// Refunded escrow passes invariant checks.
    #[test]
    fn test_refunded_state_passes_verification() {
        let env = Env::default();
        let (client, _contract_id, _admin, depositor, _token_id, _token) = setup_bounty(&env);

        let bounty_id = 3u64;
        let deadline = env.ledger().timestamp() + 500;

        client.lock_funds(&depositor, &bounty_id, &1000i128, &deadline);
        env.ledger().set_timestamp(deadline + 1);
        client.refund(&bounty_id);

        assert!(client.verify_state(&bounty_id));
        let info = client.get_escrow_info(&bounty_id);
        assert_eq!(info.status, EscrowStatus::Refunded);
    }

    // =========================================================================
    // 2. Tamper detection – corrupted state should fail verification
    // =========================================================================

    /// Detects remaining_amount > amount (over-allocation).
    #[test]
    fn test_bounty_tamper_amount_drift() {
        let env = Env::default();
        let (client, contract_id, _admin, depositor, _token_id, _token) = setup_bounty(&env);

        let bounty_id = 1u64;
        client.lock_funds(&depositor, &bounty_id, &1000i128, &100);

        let mut escrow = client.get_escrow_info(&bounty_id);
        escrow.remaining_amount = 2000;
        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&DataKey::Escrow(bounty_id), &escrow);
        });

        assert!(
            !client.verify_state(&bounty_id),
            "Should fail when remaining > total"
        );
    }

    /// Detects negative amount field.
    #[test]
    fn test_bounty_tamper_negative_amount() {
        let env = Env::default();
        let (client, contract_id, _admin, depositor, _token_id, _token) = setup_bounty(&env);

        let bounty_id = 1u64;
        client.lock_funds(&depositor, &bounty_id, &1000i128, &100);

        let mut escrow = client.get_escrow_info(&bounty_id);
        escrow.amount = -1;
        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&DataKey::Escrow(bounty_id), &escrow);
        });

        assert!(
            !client.verify_state(&bounty_id),
            "Should fail with negative amount"
        );
    }

    /// Detects Released status with non-zero remaining_amount.
    #[test]
    fn test_bounty_tamper_released_with_balance() {
        let env = Env::default();
        let (client, contract_id, _admin, depositor, _token_id, _token) = setup_bounty(&env);

        let bounty_id = 1u64;
        client.lock_funds(&depositor, &bounty_id, &1000i128, &100);

        let mut escrow = client.get_escrow_info(&bounty_id);
        escrow.status = EscrowStatus::Released;
        escrow.remaining_amount = 100;
        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&DataKey::Escrow(bounty_id), &escrow);
        });

        assert!(
            !client.verify_state(&bounty_id),
            "Should fail if released with remaining balance"
        );
    }

    /// Detects negative remaining_amount.
    #[test]
    fn test_bounty_tamper_negative_remaining() {
        let env = Env::default();
        let (client, contract_id, _admin, depositor, _token_id, _token) = setup_bounty(&env);

        let bounty_id = 1u64;
        client.lock_funds(&depositor, &bounty_id, &1000i128, &100);

        let mut escrow = client.get_escrow_info(&bounty_id);
        escrow.remaining_amount = -500;
        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&DataKey::Escrow(bounty_id), &escrow);
        });

        assert!(
            !client.verify_state(&bounty_id),
            "Should fail with negative remaining_amount"
        );
    }

    /// Non-existent bounty fails verification (returns false).
    #[test]
    fn test_verify_state_nonexistent_bounty_returns_false() {
        let env = Env::default();
        let (client, _contract_id, _admin, _depositor, _token_id, _token) = setup_bounty(&env);

        assert!(
            !client.verify_state(&999),
            "Non-existent bounty must return false"
        );
    }

    // =========================================================================
    // 3. Multi-step sequence verification
    //
    // These tests execute complex operation sequences and verify that the
    // entire contract state is consistent after each step.
    // =========================================================================

    /// Full lifecycle: lock → release → verify state and token balances.
    #[test]
    fn test_sequence_lock_release_state_consistent() {
        let env = Env::default();
        let (client, _contract_id, _admin, depositor, _token_id, token) = setup_bounty(&env);
        let contributor = Address::generate(&env);

        let bounty_id = 10u64;
        let amount = 2000i128;
        let deadline = env.ledger().timestamp() + 1000;

        let depositor_before = token.balance(&depositor);
        client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

        // After lock: verify state
        assert!(client.verify_state(&bounty_id));
        let info = client.get_escrow_info(&bounty_id);
        assert_eq!(info.status, EscrowStatus::Locked);
        assert_eq!(info.amount, amount);
        assert_eq!(info.remaining_amount, amount);
        assert_eq!(token.balance(&depositor), depositor_before - amount);
        assert_eq!(token.balance(&client.address), amount);

        // Release
        client.release_funds(&bounty_id, &contributor);

        // After release: verify state and balances
        assert!(client.verify_state(&bounty_id));
        let info = client.get_escrow_info(&bounty_id);
        assert_eq!(info.status, EscrowStatus::Released);
        assert_eq!(info.remaining_amount, 0);
        assert_eq!(token.balance(&contributor), amount);
        assert_eq!(token.balance(&client.address), 0);
    }

    /// Full lifecycle: lock → wait → refund → verify state and token balances.
    #[test]
    fn test_sequence_lock_refund_state_consistent() {
        let env = Env::default();
        let (client, _contract_id, _admin, depositor, _token_id, token) = setup_bounty(&env);

        let bounty_id = 11u64;
        let amount = 3000i128;
        let deadline = env.ledger().timestamp() + 500;

        let depositor_before = token.balance(&depositor);
        client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

        assert_eq!(token.balance(&depositor), depositor_before - amount);

        // Advance past deadline and refund
        env.ledger().set_timestamp(deadline + 1);
        client.refund(&bounty_id);

        // After refund: verify state
        assert!(client.verify_state(&bounty_id));
        let info = client.get_escrow_info(&bounty_id);
        assert_eq!(info.status, EscrowStatus::Refunded);
        assert_eq!(token.balance(&depositor), depositor_before);
        assert_eq!(token.balance(&client.address), 0);
    }

    /// Partial refund sequence: lock → approve partial → refund → verify remaining.
    #[test]
    fn test_sequence_partial_refund_state_consistent() {
        let env = Env::default();
        let (client, _contract_id, _admin, depositor, _token_id, token) = setup_bounty(&env);

        let bounty_id = 12u64;
        let amount = 5000i128;
        let partial_amount = 2000i128;
        let deadline = env.ledger().timestamp() + 2000;

        client.lock_funds(&depositor, &bounty_id, &amount, &deadline);
        client.approve_refund(
            &bounty_id,
            &partial_amount,
            &depositor,
            &RefundMode::Partial,
        );
        client.refund(&bounty_id);

        // After partial refund: verify state
        assert!(client.verify_state(&bounty_id));
        let info = client.get_escrow_info(&bounty_id);
        assert_eq!(info.status, EscrowStatus::PartiallyRefunded);
        assert_eq!(info.remaining_amount, amount - partial_amount);
        assert_eq!(
            token.balance(&client.address),
            amount - partial_amount,
            "contract balance must equal remaining_amount after partial refund"
        );
    }

    // =========================================================================
    // 4. Multi-bounty state verification
    //
    // Verifies state consistency across multiple bounties after mixed operations.
    // =========================================================================

    /// Multiple bounties with different outcomes all pass verification.
    #[test]
    fn test_multi_bounty_mixed_outcomes_all_verified() {
        let env = Env::default();
        let (client, _contract_id, _admin, depositor, _token_id, token) = setup_bounty(&env);
        let contributor = Address::generate(&env);

        let now = env.ledger().timestamp();
        let locked_id = 20u64;
        let released_id = 21u64;
        let refunded_id = 22u64;

        // Lock three bounties
        client.lock_funds(&depositor, &locked_id, &1000, &(now + 5000));
        client.lock_funds(&depositor, &released_id, &2000, &(now + 5000));
        client.lock_funds(&depositor, &refunded_id, &3000, &(now + 500));

        // Release one, refund one, keep one locked
        client.release_funds(&released_id, &contributor);
        env.ledger().set_timestamp(now + 501);
        client.refund(&refunded_id);

        // All three must pass state verification
        assert!(client.verify_state(&locked_id));
        assert!(client.verify_state(&released_id));
        assert!(client.verify_state(&refunded_id));

        // Verify individual states
        assert_eq!(
            client.get_escrow_info(&locked_id).status,
            EscrowStatus::Locked
        );
        assert_eq!(
            client.get_escrow_info(&released_id).status,
            EscrowStatus::Released
        );
        assert_eq!(
            client.get_escrow_info(&refunded_id).status,
            EscrowStatus::Refunded
        );

        // Token balance reconciliation: only locked bounty's amount remains
        assert_eq!(
            token.balance(&client.address),
            1000,
            "only locked bounty amount should remain in contract"
        );
    }

    // =========================================================================
    // 5. Token balance reconciliation
    //
    // Ensures the sum of remaining amounts across all escrows equals the
    // on-chain token balance of the contract — critical for migration audits.
    // =========================================================================

    /// Sum of remaining amounts matches contract token balance after mixed operations.
    #[test]
    fn test_balance_reconciliation_across_multiple_escrows() {
        let env = Env::default();
        let (client, _contract_id, _admin, depositor, _token_id, token) = setup_bounty(&env);
        let contributor = Address::generate(&env);

        let now = env.ledger().timestamp();
        // Lock various amounts
        client.lock_funds(&depositor, &30, &1000, &(now + 5000));
        client.lock_funds(&depositor, &31, &2000, &(now + 5000));
        client.lock_funds(&depositor, &32, &3000, &(now + 500));
        client.lock_funds(&depositor, &33, &4000, &(now + 5000));

        // Release 31, refund 32
        client.release_funds(&31, &contributor);
        env.ledger().set_timestamp(now + 501);
        client.refund(&32);

        // Sum remaining amounts
        let remaining_30 = client.get_escrow_info(&30).remaining_amount;
        let remaining_31 = client.get_escrow_info(&31).remaining_amount;
        let remaining_32 = client.get_escrow_info(&32).remaining_amount;
        let remaining_33 = client.get_escrow_info(&33).remaining_amount;

        let sum_remaining = remaining_30 + remaining_31 + remaining_32 + remaining_33;

        assert_eq!(
            token.balance(&client.address),
            sum_remaining,
            "contract token balance must equal sum of all remaining_amounts"
        );
        // Specifically: 1000 + 0 + 0 + 4000 = 5000
        assert_eq!(sum_remaining, 5000);
    }

    // =========================================================================
    // 6. Aggregate stats consistency with individual escrows
    //
    // For migration audits: verify that aggregate_stats matches the sum of
    // individual escrow states.
    // =========================================================================

    /// Aggregate stats match individual escrow data after mixed operations.
    #[test]
    fn test_aggregate_stats_match_individual_escrows_for_audit() {
        let env = Env::default();
        let (client, _contract_id, _admin, depositor, _token_id, _token) = setup_bounty(&env);
        let contributor = Address::generate(&env);

        let now = env.ledger().timestamp();
        client.lock_funds(&depositor, &40, &500, &(now + 5000));
        client.lock_funds(&depositor, &41, &700, &(now + 5000));
        client.lock_funds(&depositor, &42, &900, &(now + 500));

        client.release_funds(&41, &contributor);
        env.ledger().set_timestamp(now + 501);
        client.refund(&42);

        let stats = client.get_aggregate_stats();

        // Count matches
        assert_eq!(stats.count_locked, 1, "one escrow still locked");
        assert_eq!(stats.count_released, 1, "one escrow released");
        assert_eq!(stats.count_refunded, 1, "one escrow refunded");

        // Total amounts match individual escrows
        assert_eq!(stats.total_locked, 500, "locked total = escrow 40 amount");
        assert_eq!(
            stats.total_released, 700,
            "released total = escrow 41 amount"
        );
        assert_eq!(
            stats.total_refunded, 900,
            "refunded total = escrow 42 amount"
        );
    }

    // =========================================================================
    // 7. State persistence across time advancement
    //
    // Verifies that state remains valid regardless of how far time advances
    // (useful for long-running escrows and migration windows).
    // =========================================================================

    /// Locked escrow state remains valid after large time advancement.
    #[test]
    fn test_state_valid_after_large_time_advancement() {
        let env = Env::default();
        let (client, _contract_id, _admin, depositor, _token_id, _token) = setup_bounty(&env);

        let bounty_id = 50u64;
        let deadline = env.ledger().timestamp() + 1_000_000;
        client.lock_funds(&depositor, &bounty_id, &5000, &deadline);

        // Advance 10 years
        env.ledger()
            .set_timestamp(env.ledger().timestamp() + 10 * 365 * 24 * 3600);

        // State should still be valid
        assert!(client.verify_state(&bounty_id));
        let info = client.get_escrow_info(&bounty_id);
        assert_eq!(info.status, EscrowStatus::Locked);
        assert_eq!(info.amount, 5000);
        assert_eq!(info.remaining_amount, 5000);
    }

    // =========================================================================
    // 8. Sequential operations on same bounty
    //
    // Verifies state after sequential operations like lock → partial refund
    // → another partial refund (if applicable via re-approval).
    // =========================================================================

    /// State correct after lock → failed early refund → time advance → successful refund.
    #[test]
    fn test_sequence_failed_refund_then_successful_refund() {
        let env = Env::default();
        let (client, _contract_id, _admin, depositor, _token_id, token) = setup_bounty(&env);

        let bounty_id = 60u64;
        let deadline = env.ledger().timestamp() + 1000;
        client.lock_funds(&depositor, &bounty_id, &2000, &deadline);

        // Failed refund (before deadline)
        let result = client.try_refund(&bounty_id);
        assert!(result.is_err());

        // State unchanged after failed attempt
        assert!(client.verify_state(&bounty_id));
        assert_eq!(
            client.get_escrow_info(&bounty_id).status,
            EscrowStatus::Locked
        );
        assert_eq!(token.balance(&client.address), 2000);

        // Advance past deadline and succeed
        env.ledger().set_timestamp(deadline + 1);
        client.refund(&bounty_id);

        assert!(client.verify_state(&bounty_id));
        assert_eq!(
            client.get_escrow_info(&bounty_id).status,
            EscrowStatus::Refunded
        );
        assert_eq!(token.balance(&client.address), 0);
    }

    /// State correct after lock → failed release (wrong bounty) doesn't affect valid bounty.
    #[test]
    fn test_failed_operation_does_not_corrupt_other_escrow() {
        let env = Env::default();
        let (client, _contract_id, _admin, depositor, _token_id, _token) = setup_bounty(&env);
        let contributor = Address::generate(&env);

        let bounty_id = 70u64;
        client.lock_funds(
            &depositor,
            &bounty_id,
            &1500,
            &(env.ledger().timestamp() + 1000),
        );

        // Try to release a non-existent bounty
        let result = client.try_release_funds(&999, &contributor);
        assert!(result.is_err());

        // Original bounty still valid and unchanged
        assert!(client.verify_state(&bounty_id));
        let info = client.get_escrow_info(&bounty_id);
        assert_eq!(info.status, EscrowStatus::Locked);
        assert_eq!(info.remaining_amount, 1500);
    }

    // =========================================================================
    // 9. Escrow count consistency
    //
    // Verifies that escrow count always matches the number of escrows that
    // have been created (regardless of status).
    // =========================================================================

    /// Escrow count matches actual number of escrows after mixed operations.
    #[test]
    fn test_escrow_count_matches_created_escrows() {
        let env = Env::default();
        let (client, _contract_id, _admin, depositor, _token_id, _token) = setup_bounty(&env);
        let contributor = Address::generate(&env);

        let now = env.ledger().timestamp();
        client.lock_funds(&depositor, &80, &500, &(now + 5000));
        client.lock_funds(&depositor, &81, &600, &(now + 5000));
        client.lock_funds(&depositor, &82, &700, &(now + 500));

        client.release_funds(&80, &contributor);
        env.ledger().set_timestamp(now + 501);
        client.refund(&82);

        // Count should be 3 (total created, not just active)
        assert_eq!(
            client.get_escrow_count(),
            3,
            "escrow count must reflect all created escrows"
        );

        // All should pass verification
        assert!(client.verify_state(&80));
        assert!(client.verify_state(&81));
        assert!(client.verify_state(&82));
    }
}
