#[cfg(test)]
mod test_conditional_refund {
    use crate::{
        BountyEscrowContract, BountyEscrowContractClient, DisputeReason, Error, EscrowStatus,
    };
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token, Address, Env, IntoVal,
    };

    // ── Test harness ──────────────────────────────────────────────────────────

    struct Setup {
        env: Env,
        admin: Address,
        depositor: Address,
        oracle: Address,
        contract_id: Address,
    }

    fn setup() -> Setup {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let oracle = Address::generate(&env);

        let token_id = env.register_stellar_asset_contract_v2(admin.clone());
        let token_admin = token::StellarAssetClient::new(&env, &token_id);

        let contract_id = env.register_contract(None, BountyEscrowContract);
        {
            let client = BountyEscrowContractClient::new(&env, &contract_id);
            client.init(&admin, &token_id);
        }

        token_admin.mint(&depositor, &100_000);

        Setup {
            env,
            admin,
            depositor,
            oracle,
            contract_id,
        }
    }

    fn client<'a>(s: &'a Setup) -> BountyEscrowContractClient<'a> {
        BountyEscrowContractClient::new(&s.env, &s.contract_id)
    }

    fn lock(s: &Setup, bounty_id: u64, deadline_offset: u64) {
        let deadline = s.env.ledger().timestamp() + deadline_offset;
        client(s).lock_funds(&s.depositor, &bounty_id, &1_000, &deadline);
    }

    fn refund_event(s: &Setup) -> crate::events::FundsRefunded {
        for (_, topics, data) in s.env.events().all().iter() {
            if topics.get(0).unwrap().into_val(&s.env)
                == soroban_sdk::symbol_short!("f_ref").into_val(&s.env)
            {
                return data.into_val(&s.env);
            }
        }
        panic!("refund event was not emitted");
    }

    // ── Oracle-triggered refund tests ─────────────────────────────────────────

    #[test]
    fn test_oracle_refund_succeeds() {
        let s = setup();
        lock(&s, 1, 1000);
        client(&s).set_oracle(&s.oracle, &true);
        client(&s).oracle_refund(&1u64, &s.oracle);

        let escrow = client(&s).get_escrow_info(&1u64).unwrap();
        assert_eq!(escrow.status, EscrowStatus::Refunded);
        assert_eq!(escrow.remaining_amount, 0);
        // Verify trigger type in history
        assert_eq!(
            refund_event(&s).trigger_type,
            crate::events::RefundTriggerType::OracleAttestation
        );
    }

    #[test]
    fn test_oracle_refund_fails_not_configured() {
        let s = setup();
        lock(&s, 1, 1000);
        // No set_oracle call
        let result = client(&s).try_oracle_refund(&1u64, &s.oracle);
        assert_eq!(result.unwrap_err().unwrap(), Error::OracleNotConfigured);
    }

    #[test]
    fn test_oracle_refund_fails_oracle_disabled() {
        let s = setup();
        lock(&s, 1, 1000);
        client(&s).set_oracle(&s.oracle, &false); // disabled
        let result = client(&s).try_oracle_refund(&1u64, &s.oracle);
        assert_eq!(result.unwrap_err().unwrap(), Error::OracleNotConfigured);
    }

    #[test]
    fn test_oracle_refund_fails_wrong_caller() {
        let s = setup();
        lock(&s, 1, 1000);
        client(&s).set_oracle(&s.oracle, &true);
        let wrong_caller = Address::generate(&s.env);
        let result = client(&s).try_oracle_refund(&1u64, &wrong_caller);
        assert_eq!(result.unwrap_err().unwrap(), Error::CallerNotOracle);
    }

    #[test]
    fn test_oracle_refund_fails_already_released() {
        let s = setup();
        lock(&s, 1, 1000);
        let contributor = Address::generate(&s.env);
        client(&s).release_funds(&1u64, &contributor);
        client(&s).set_oracle(&s.oracle, &true);
        let result = client(&s).try_oracle_refund(&1u64, &s.oracle);
        assert_eq!(result.unwrap_err().unwrap(), Error::FundsNotLocked);
    }

    #[test]
    fn test_oracle_refund_fails_already_refunded() {
        let s = setup();
        lock(&s, 1, 1000);
        client(&s).set_oracle(&s.oracle, &true);
        // First oracle refund succeeds
        client(&s).oracle_refund(&1u64, &s.oracle);
        // Second oracle refund must fail with double-refund error
        let result = client(&s).try_oracle_refund(&1u64, &s.oracle);
        assert_eq!(
            result.unwrap_err().unwrap(),
            Error::OracleRefundAlreadyProcessed
        );
    }

    #[test]
    fn test_oracle_refund_with_pending_claim_blocked() {
        let s = setup();
        lock(&s, 1, 1000);
        let recipient = Address::generate(&s.env);
        client(&s).authorize_claim(&1u64, &recipient, &DisputeReason::Other);
        client(&s).set_oracle(&s.oracle, &true);
        let result = client(&s).try_oracle_refund(&1u64, &s.oracle);
        assert_eq!(result.unwrap_err().unwrap(), Error::ClaimPending);
    }

    // ── Time-triggered auto-refund tests ──────────────────────────────────────

    #[test]
    fn test_auto_refund_permissionless_after_deadline() {
        let s = setup();
        lock(&s, 1, 500);
        // Advance past deadline
        s.env
            .ledger()
            .set_timestamp(s.env.ledger().timestamp() + 600);
        // Any address (not admin, not depositor) can trigger
        client(&s).auto_refund(&1u64);

        let escrow = client(&s).get_escrow_info(&1u64).unwrap();
        assert_eq!(escrow.status, EscrowStatus::Refunded);
        assert_eq!(escrow.remaining_amount, 0);
        assert_eq!(
            refund_event(&s).trigger_type,
            crate::events::RefundTriggerType::DeadlineExpired
        );
    }

    #[test]
    fn test_auto_refund_fails_before_deadline() {
        let s = setup();
        lock(&s, 1, 1000);
        // Do NOT advance time — deadline not yet passed
        let result = client(&s).try_auto_refund(&1u64);
        assert_eq!(result.unwrap_err().unwrap(), Error::DeadlineNotPassed);
    }

    #[test]
    fn test_auto_refund_fails_already_released() {
        let s = setup();
        lock(&s, 1, 500);
        let contributor = Address::generate(&s.env);
        client(&s).release_funds(&1u64, &contributor);
        s.env
            .ledger()
            .set_timestamp(s.env.ledger().timestamp() + 600);
        let result = client(&s).try_auto_refund(&1u64);
        assert_eq!(result.unwrap_err().unwrap(), Error::FundsNotLocked);
    }

    #[test]
    fn test_auto_refund_fails_already_refunded() {
        let s = setup();
        lock(&s, 1, 500);
        s.env
            .ledger()
            .set_timestamp(s.env.ledger().timestamp() + 600);
        client(&s).auto_refund(&1u64);
        // Second attempt
        let result = client(&s).try_auto_refund(&1u64);
        assert_eq!(result.unwrap_err().unwrap(), Error::FundsNotLocked);
    }

    // ── Mutual exclusion tests ────────────────────────────────────────────────

    #[test]
    fn test_release_then_refund_fails() {
        let s = setup();
        lock(&s, 1, 500);
        let contributor = Address::generate(&s.env);
        client(&s).release_funds(&1u64, &contributor);
        // Advance past deadline
        s.env
            .ledger()
            .set_timestamp(s.env.ledger().timestamp() + 600);
        // auto_refund must fail — already released
        let result = client(&s).try_auto_refund(&1u64);
        assert_eq!(result.unwrap_err().unwrap(), Error::FundsNotLocked);
    }

    #[test]
    fn test_oracle_refund_then_release_fails() {
        let s = setup();
        lock(&s, 1, 1000);
        client(&s).set_oracle(&s.oracle, &true);
        client(&s).oracle_refund(&1u64, &s.oracle);
        // release_funds must fail — already refunded
        let contributor = Address::generate(&s.env);
        let result = client(&s).try_release_funds(&1u64, &contributor);
        assert_eq!(result.unwrap_err().unwrap(), Error::FundsNotLocked);
    }

    #[test]
    fn test_auto_refund_then_release_fails() {
        let s = setup();
        lock(&s, 1, 500);
        s.env
            .ledger()
            .set_timestamp(s.env.ledger().timestamp() + 600);
        client(&s).auto_refund(&1u64);
        let contributor = Address::generate(&s.env);
        let result = client(&s).try_release_funds(&1u64, &contributor);
        assert_eq!(result.unwrap_err().unwrap(), Error::FundsNotLocked);
    }

    // ── Event / trigger_type verification ────────────────────────────────────

    #[test]
    fn test_refund_event_includes_trigger_type_deadline() {
        let s = setup();
        lock(&s, 1, 500);
        s.env
            .ledger()
            .set_timestamp(s.env.ledger().timestamp() + 600);
        client(&s).auto_refund(&1u64);

        let escrow = client(&s).get_escrow_info(&1u64).unwrap();
        assert_eq!(
            refund_event(&s).trigger_type,
            crate::events::RefundTriggerType::DeadlineExpired
        );
    }

    #[test]
    fn test_refund_event_includes_trigger_type_oracle() {
        let s = setup();
        lock(&s, 1, 1000);
        client(&s).set_oracle(&s.oracle, &true);
        client(&s).oracle_refund(&1u64, &s.oracle);

        let escrow = client(&s).get_escrow_info(&1u64).unwrap();
        assert_eq!(
            refund_event(&s).trigger_type,
            crate::events::RefundTriggerType::OracleAttestation
        );
    }

    #[test]
    fn test_refund_event_includes_trigger_type_admin() {
        let s = setup();
        lock(&s, 1, 1000);
        // Admin approve + refund (existing path)
        client(&s).approve_refund(&1u64, &1_000i128, &s.depositor, &crate::RefundMode::Full);
        client(&s).refund(&1u64);

        let escrow = client(&s).get_escrow_info(&1u64).unwrap();
        assert_eq!(
            refund_event(&s).trigger_type,
            crate::events::RefundTriggerType::AdminApproval
        );
    }
}
