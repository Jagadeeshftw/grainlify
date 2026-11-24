//! Integration tests for insurance and reserve pool hooks
//!
//! These tests exercise the hook infrastructure end-to-end using a test escrow
//! contract, [`crate::mock_hook::MockHook`], and a panicking hook contract to
//! verify best-effort semantics (hook failures never revert core flow).

#[cfg(test)]
mod hook_integration_tests {
    use crate::hooks::{
        call_dispute_opened_hook, call_dispute_resolved_hook, call_large_release_hook,
        call_refund_hook, get_hook_address, get_large_release_threshold,
        set_hook_address_internal, set_large_release_threshold_internal, HookCall, HookCallEvent,
        HookCallStatus, HookEventType, RefundReason,
    };
    use crate::mock_hook::{MockHook, MockHookClient};
    use soroban_sdk::{
        contract, contractimpl, symbol_short,
        testutils::{Address as _, Events},
        Address, Env, String, Symbol, TryFromVal,
    };

    /// Minimal escrow contract used to exercise hook invocation and verify
    /// that core state mutations persist even when hooks fail or panic.
    #[contract]
    struct TestEscrow;

    #[contractimpl]
    impl TestEscrow {
        pub fn set_hook_address(env: Env, hook_address: Option<Address>) {
            set_hook_address_internal(&env, hook_address);
        }

        pub fn get_hook_address(env: Env) -> Option<Address> {
            get_hook_address(&env)
        }

        pub fn set_large_release_threshold(env: Env, threshold: Option<i128>) {
            set_large_release_threshold_internal(&env, threshold);
        }

        pub fn get_large_release_threshold(env: Env) -> Option<i128> {
            get_large_release_threshold(&env)
        }

        pub fn get_ops_completed(env: Env) -> u32 {
            env.storage()
                .instance()
                .get(&symbol_short!("ops"))
                .unwrap_or(0)
        }

        fn record_op(env: &Env) {
            let count: u32 = env
                .storage()
                .instance()
                .get(&symbol_short!("ops"))
                .unwrap_or(0);
            env.storage()
                .instance()
                .set(&symbol_short!("ops"), &(count + 1));
        }

        pub fn open_dispute(
            env: Env,
            bounty_id: u64,
            disputer: Address,
            amount: i128,
            reason: String,
            deadline: u64,
        ) {
            Self::record_op(&env);
            call_dispute_opened_hook(&env, bounty_id, &disputer, amount, &reason, deadline);
        }

        pub fn release_funds(
            env: Env,
            bounty_id: u64,
            recipient: Address,
            amount: i128,
            release_count: u32,
        ) {
            Self::record_op(&env);
            call_large_release_hook(&env, bounty_id, &recipient, amount, release_count);
        }

        pub fn refund(
            env: Env,
            bounty_id: u64,
            refund_to: Address,
            amount: i128,
            reason: RefundReason,
        ) {
            Self::record_op(&env);
            call_refund_hook(&env, bounty_id, &refund_to, amount, reason);
        }

        pub fn resolve_dispute(
            env: Env,
            bounty_id: u64,
            outcome: String,
            amount_released: i128,
            amount_refunded: i128,
        ) {
            Self::record_op(&env);
            call_dispute_resolved_hook(
                &env,
                bounty_id,
                &outcome,
                amount_released,
                amount_refunded,
            );
        }
    }

    /// Hook contract that always panics/aborts, used to verify best-effort
    /// isolation against trapping hooks.
    #[contract]
    struct PanickingHook;

    #[contractimpl]
    impl PanickingHook {
        pub fn handle_hook(_env: Env, _call: HookCall) {
            panic!("hook trapped");
        }
    }

    fn collect_hook_call_events(env: &Env) -> std::vec::Vec<HookCallEvent> {
        let expected_topic = symbol_short!("hook_call");
        env.events()
            .all()
            .iter()
            .filter_map(|(_contract, topics, data)| {
                let first_topic = topics
                    .get(0)
                    .and_then(|v| Symbol::try_from_val(env, &v).ok());
                if topics.len() == 1 && first_topic == Some(expected_topic.clone()) {
                    HookCallEvent::try_from_val(env, &data).ok()
                } else {
                    None
                }
            })
            .collect()
    }

    #[test]
    fn test_hook_address_configuration() {
        let env = Env::default();
        let escrow_id = env.register_contract(None, TestEscrow);
        let escrow = TestEscrowClient::new(&env, &escrow_id);
        let hook_addr = Address::generate(&env);

        assert_eq!(escrow.get_hook_address(), None);

        escrow.set_hook_address(&Some(hook_addr.clone()));
        assert_eq!(escrow.get_hook_address(), Some(hook_addr));

        escrow.set_hook_address(&None);
        assert_eq!(escrow.get_hook_address(), None);
    }

    #[test]
    fn test_dispute_opened_triggers_hook() {
        let env = Env::default();
        let escrow_id = env.register_contract(None, TestEscrow);
        let escrow = TestEscrowClient::new(&env, &escrow_id);
        let mock_id = env.register_contract(None, MockHook);
        let mock = MockHookClient::new(&env, &mock_id);

        let disputer = Address::generate(&env);
        let reason = String::from_str(&env, "Deliverable incomplete");

        escrow.set_hook_address(&Some(mock_id.clone()));
        escrow.open_dispute(&42, &disputer, &2_500_000, &reason, &9999);

        assert_eq!(escrow.get_ops_completed(), 1);
        let stats = mock.get_stats();
        assert_eq!(stats.calls_received, 1);
        assert_eq!(stats.last_event, HookEventType::DisputeOpened);
        assert_eq!(stats.last_bounty_id, 42);
        assert_eq!(stats.last_amount, 2_500_000);

        let events = collect_hook_call_events(&env);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].status, HookCallStatus::Success);
        assert_eq!(events[0].bounty_id, 42);
        assert_eq!(events[0].event_type, HookEventType::DisputeOpened);
    }

    #[test]
    fn test_core_flow_continues_if_hook_returns_error() {
        let env = Env::default();
        let escrow_id = env.register_contract(None, TestEscrow);
        let escrow = TestEscrowClient::new(&env, &escrow_id);
        let mock_id = env.register_contract(None, MockHook);
        let mock = MockHookClient::new(&env, &mock_id);

        let disputer = Address::generate(&env);
        let reason = String::from_str(&env, "Quality dispute");

        escrow.set_hook_address(&Some(mock_id.clone()));
        mock.set_fail(&true, &String::from_str(&env, "Insufficient reserve"));

        // Core operation must succeed despite hook returning an error.
        escrow.open_dispute(&7, &disputer, &1_000_000, &reason, &5000);

        assert_eq!(escrow.get_ops_completed(), 1);

        let events = collect_hook_call_events(&env);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].status, HookCallStatus::Failed);
        assert!(events[0].error_msg.is_some());
    }

    #[test]
    fn test_core_flow_continues_if_hook_panics() {
        let env = Env::default();
        let escrow_id = env.register_contract(None, TestEscrow);
        let escrow = TestEscrowClient::new(&env, &escrow_id);
        let panicking_id = env.register_contract(None, PanickingHook);

        let disputer = Address::generate(&env);
        let reason = String::from_str(&env, "Panic isolation test");

        escrow.set_hook_address(&Some(panicking_id));

        // Core operation must succeed even when the hook contract panics/traps.
        escrow.open_dispute(&99, &disputer, &500_000, &reason, &8000);

        assert_eq!(escrow.get_ops_completed(), 1);

        let events = collect_hook_call_events(&env);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].status, HookCallStatus::Panic);
        assert!(events[0].error_msg.is_some());
    }

    #[test]
    fn test_large_release_threshold() {
        let env = Env::default();
        let escrow_id = env.register_contract(None, TestEscrow);
        let escrow = TestEscrowClient::new(&env, &escrow_id);
        let mock_id = env.register_contract(None, MockHook);
        let mock = MockHookClient::new(&env, &mock_id);

        let recipient = Address::generate(&env);
        let small_amount = 100_000i128;
        let large_amount = 10_000_000i128;
        let threshold = 5_000_000i128;

        escrow.set_hook_address(&Some(mock_id));
        escrow.set_large_release_threshold(&Some(threshold));
        assert_eq!(escrow.get_large_release_threshold(), Some(threshold));

        // Small release should NOT trigger hook
        escrow.release_funds(&1, &recipient, &small_amount, &1);
        assert_eq!(mock.get_call_count(), 0);

        // Large release SHOULD trigger hook
        escrow.release_funds(&2, &recipient, &large_amount, &1);
        let stats = mock.get_stats();
        assert_eq!(stats.calls_received, 1);
        assert_eq!(stats.last_event, HookEventType::LargeRelease);
        assert_eq!(stats.last_bounty_id, 2);
        assert_eq!(stats.last_amount, large_amount);
    }

    #[test]
    fn test_multiple_operations_and_audit_trail() {
        let env = Env::default();
        let escrow_id = env.register_contract(None, TestEscrow);
        let escrow = TestEscrowClient::new(&env, &escrow_id);
        let mock_id = env.register_contract(None, MockHook);
        let mock = MockHookClient::new(&env, &mock_id);

        let recipient = Address::generate(&env);
        let disputer = Address::generate(&env);

        escrow.set_hook_address(&Some(mock_id));
        escrow.set_large_release_threshold(&Some(5_000_000));

        // Small release - no hook
        escrow.release_funds(&1, &recipient, &1_000_000, &1);
        assert_eq!(mock.get_call_count(), 0);

        // Large release - triggers hook
        escrow.release_funds(&1, &recipient, &6_000_000, &2);
        assert_eq!(mock.get_call_count(), 1);

        // Open dispute - triggers hook
        escrow.open_dispute(
            &1,
            &disputer,
            &4_000_000,
            &String::from_str(&env, "Disputed"),
            &12000,
        );
        assert_eq!(mock.get_call_count(), 2);

        // Resolve dispute - triggers hook
        escrow.resolve_dispute(
            &1,
            &String::from_str(&env, "Partial"),
            &2_000_000,
            &2_000_000,
        );
        assert_eq!(mock.get_call_count(), 3);

        let history = mock.get_call_history();
        assert_eq!(history.len(), 3);
        assert_eq!(history.get(0).unwrap().event_type, HookEventType::LargeRelease);
        assert_eq!(history.get(1).unwrap().event_type, HookEventType::DisputeOpened);
        assert_eq!(history.get(2).unwrap().event_type, HookEventType::DisputeResolved);
    }

    #[test]
    fn test_hook_idempotency() {
        let env = Env::default();
        let escrow_id = env.register_contract(None, TestEscrow);
        let escrow = TestEscrowClient::new(&env, &escrow_id);
        let mock_id = env.register_contract(None, MockHook);
        let mock = MockHookClient::new(&env, &mock_id);

        let disputer = Address::generate(&env);
        let reason = String::from_str(&env, "Repeated event");

        escrow.set_hook_address(&Some(mock_id));

        for _ in 0..3 {
            escrow.open_dispute(&10, &disputer, &1_000_000, &reason, &5000);
        }

        assert_eq!(mock.get_call_count(), 3);
        assert!(mock.last_call_matches(&HookEventType::DisputeOpened, &10, &1_000_000));
    }

    #[test]
    fn test_hook_disable() {
        let env = Env::default();
        let escrow_id = env.register_contract(None, TestEscrow);
        let escrow = TestEscrowClient::new(&env, &escrow_id);
        let mock_id = env.register_contract(None, MockHook);
        let mock = MockHookClient::new(&env, &mock_id);

        let disputer = Address::generate(&env);

        escrow.set_hook_address(&Some(mock_id.clone()));
        assert_eq!(escrow.get_hook_address(), Some(mock_id));

        escrow.set_hook_address(&None);
        assert_eq!(escrow.get_hook_address(), None);

        escrow.open_dispute(
            &1,
            &disputer,
            &1_000_000,
            &String::from_str(&env, "Should not trigger"),
            &5000,
        );
        assert_eq!(mock.get_call_count(), 0);
        assert_eq!(escrow.get_ops_completed(), 1);
    }

    #[test]
    fn test_refund_hook() {
        let env = Env::default();
        let escrow_id = env.register_contract(None, TestEscrow);
        let escrow = TestEscrowClient::new(&env, &escrow_id);
        let mock_id = env.register_contract(None, MockHook);
        let mock = MockHookClient::new(&env, &mock_id);

        let depositor = Address::generate(&env);

        escrow.set_hook_address(&Some(mock_id));
        escrow.refund(&55, &depositor, &3_000_000, &RefundReason::Expired);

        let stats = mock.get_stats();
        assert_eq!(stats.calls_received, 1);
        assert_eq!(stats.last_event, HookEventType::Refund);
        assert_eq!(stats.last_bounty_id, 55);
        assert_eq!(stats.last_amount, 3_000_000);
    }
}
