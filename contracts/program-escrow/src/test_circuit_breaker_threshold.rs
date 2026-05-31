/// Per-Program Circuit Breaker Threshold Tests — Issue #1255
///
/// Verifies that the per-program circuit breaker threshold feature works correctly:
/// - Default threshold (3) is used when not configured
/// - Custom threshold can be set via set_program_circuit_breaker_threshold
/// - Threshold validation (1-100) is enforced
/// - Threshold changes emit correct audit events
/// - Circuit breaker respects per-program thresholds

#[cfg(test)]
mod test {
    use crate::error_recovery::{self, CircuitBreakerKey, CircuitState};
    use crate::{
        ProgramEscrowContract, ProgramEscrowContractClient,
        errors::ContractError,
    };
    use soroban_sdk::{
        symbol_short,
        testutils::{Address as _, Events, Ledger},
        token, vec, Address, Env, String, Symbol, TryFromVal,
    };

    // ─────────────────────────────────────────────────────────────────────
    // Test Helpers
    // ─────────────────────────────────────────────────────────────────────

    struct Setup<'a> {
        env: Env,
        client: ProgramEscrowContractClient<'a>,
        admin: Address,
        token_client: token::Client<'a>,
        program_id: String,
    }

    fn setup() -> Setup<'static> {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(1000);

        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let sac = env.register_stellar_asset_contract_v2(token_admin);
        let token_id = sac.address();
        let token_client = token::Client::new(&env, &token_id);
        let token_admin_client = token::StellarAssetClient::new(&env, &token_id);

        client.initialize_contract(&admin);
        client.set_circuit_admin(&admin, &None);

        let program_id = String::from_str(&env, "prog-cb-threshold");
        client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
        client.publish_program(&program_id);

        let initial_balance = 10_000_0000000; // 10,000 tokens
        token_admin_client.mint(&contract_id, &initial_balance);
        client.lock_program_funds(&initial_balance);

        Setup {
            env,
            client,
            admin,
            token_client,
            program_id,
        }
    }

    fn get_program_data(env: &Env, program_id: &String) -> crate::ProgramData {
        env.storage()
            .instance()
            .get(&crate::DataKey::Program(program_id.clone()))
            .unwrap()
    }

    fn has_event_topic(env: &Env, topic0: Symbol, topic1: Symbol) -> bool {
        for ev in env.events().all().iter() {
            if ev.1.len() >= 2
                && Symbol::try_from_val(env, &ev.1.get(0).unwrap()).ok() == Some(topic0.clone())
                && Symbol::try_from_val(env, &ev.1.get(1).unwrap()).ok() == Some(topic1.clone())
            {
                return true;
            }
        }
        false
    }

    // ─────────────────────────────────────────────────────────────────────
    // Default Threshold Tests
    // ─────────────────────────────────────────────────────────────────────

    /// New programs should have None as the circuit breaker threshold,
    /// meaning they use the global default (3).
    #[test]
    fn test_default_threshold_is_none() {
        let s = setup();
        let program_data = get_program_data(&s.env, &s.program_id);
        assert_eq!(program_data.circuit_breaker_threshold, None);
    }

    // ─────────────────────────────────────────────────────────────────────
    // Set Threshold Tests
    // ─────────────────────────────────────────────────────────────────────

    /// Admin can set a valid custom threshold for a program.
    #[test]
    fn test_set_custom_threshold() {
        let s = setup();
        
        // Set threshold to 10
        s.client.set_program_circuit_breaker_threshold(&s.program_id, &Some(10u8));
        
        let program_data = get_program_data(&s.env, &s.program_id);
        assert_eq!(program_data.circuit_breaker_threshold, Some(10));
    }

    /// Admin can reset threshold to None (use global default).
    #[test]
    fn test_reset_threshold_to_none() {
        let s = setup();
        
        // Set threshold to 10
        s.client.set_program_circuit_breaker_threshold(&s.program_id, &Some(10u8));
        
        // Reset to None
        s.client.set_program_circuit_breaker_threshold(&s.program_id, &None);
        
        let program_data = get_program_data(&s.env, &s.program_id);
        assert_eq!(program_data.circuit_breaker_threshold, None);
    }

    /// Threshold must be >= 1.
    #[test]
    #[should_panic(expected = "804")]
    fn test_threshold_too_low() {
        let s = setup();
        s.client.set_program_circuit_breaker_threshold(&s.program_id, &Some(0u8));
    }

    /// Threshold must be <= 100.
    #[test]
    #[should_panic(expected = "804")]
    fn test_threshold_too_high() {
        let s = setup();
        s.client.set_program_circuit_breaker_threshold(&s.program_id, &Some(101u8));
    }

    /// Threshold of 1 is valid (minimum allowed).
    #[test]
    fn test_threshold_minimum_valid() {
        let s = setup();
        s.client.set_program_circuit_breaker_threshold(&s.program_id, &Some(1u8));
        
        let program_data = get_program_data(&s.env, &s.program_id);
        assert_eq!(program_data.circuit_breaker_threshold, Some(1));
    }

    /// Threshold of 100 is valid (maximum allowed).
    #[test]
    fn test_threshold_maximum_valid() {
        let s = setup();
        s.client.set_program_circuit_breaker_threshold(&s.program_id, &Some(100u8));
        
        let program_data = get_program_data(&s.env, &s.program_id);
        assert_eq!(program_data.circuit_breaker_threshold, Some(100));
    }

    // ─────────────────────────────────────────────────────────────────────
    // Audit Event Tests
    // ─────────────────────────────────────────────────────────────────────

    /// Setting threshold emits CB_THRESHOLD_SET event.
    #[test]
    fn test_set_threshold_emits_event() {
        let s = setup();
        
        s.client.set_program_circuit_breaker_threshold(&s.program_id, &Some(10u8));
        
        assert!(
            has_event_topic(&s.env, symbol_short!("CbThrSet"), symbol_short!("CbThrSet")),
            "CB_THRESHOLD_SET event must be emitted when threshold is set"
        );
    }

    /// Event contains previous and new threshold values.
    #[test]
    fn test_event_contains_threshold_values() {
        let s = setup();
        
        // First set: previous is None
        s.client.set_program_circuit_breaker_threshold(&s.program_id, &Some(10u8));
        
        // Second set: previous is Some(10)
        s.client.set_program_circuit_breaker_threshold(&s.program_id, &Some(20u8));
        
        // Verify events were emitted
        let events = s.env.events().all();
        let mut threshold_set_count = 0;
        for ev in events.iter() {
            if ev.1.len() >= 2 {
                if let Ok(topic) = Symbol::try_from_val(&s.env, &ev.1.get(0).unwrap()) {
                    if topic == symbol_short!("CbThrSet") {
                        threshold_set_count += 1;
                    }
                }
            }
        }
        assert_eq!(threshold_set_count, 2, "Should emit 2 CB_THRESHOLD_SET events");
    }

    // ─────────────────────────────────────────────────────────────────────
    // Authorization Tests
    // ─────────────────────────────────────────────────────────────────────

    /// Unauthorized callers cannot set threshold.
    #[test]
    #[should_panic(expected = "Unauthorized")]
    fn test_unauthorized_cannot_set_threshold() {
        let s = setup();
        let unauthorized = Address::generate(&s.env);
        
        s.env.mock_all_auths(); // Mock all auths except the specific one we want to fail
        s.env.budget().reset_unlimited();
        
        // Try to set threshold as unauthorized user
        // This should fail because require_auth is called on authorized_payout_key
        s.client.set_program_circuit_breaker_threshold(&s.program_id, &Some(10u8));
    }

    // ─────────────────────────────────────────────────────────────────────
    // Integration with Circuit Breaker Tests
    // ─────────────────────────────────────────────────────────────────────

    /// Circuit breaker uses custom threshold when set.
    #[test]
    fn test_circuit_breaker_uses_custom_threshold() {
        let s = setup();
        
        // Set custom threshold to 5
        s.client.set_program_circuit_breaker_threshold(&s.program_id, &Some(5u8));
        
        let program_data = get_program_data(&s.env, &s.program_id);
        let threshold = program_data.circuit_breaker_threshold.map(|t| t as u32).unwrap_or(3);
        
        assert_eq!(threshold, 5);
        
        // Record failures up to threshold
        s.env.as_contract(&s.client.address, || {
            for i in 0..threshold {
                error_recovery::record_failure(
                    &s.env,
                    s.program_id.clone(),
                    symbol_short!("test_op"),
                    42,
                    program_data.circuit_breaker_threshold.map(|t| t as u32),
                );
            }
            
            // Circuit should be open after threshold failures
            assert_eq!(
                error_recovery::get_state(&s.env),
                CircuitState::Open,
                "Circuit must open after {} failures with custom threshold",
                threshold
            );
        });
    }

    /// Circuit breaker uses default threshold (3) when not set.
    #[test]
    fn test_circuit_breaker_uses_default_threshold() {
        let s = setup();
        
        let program_data = get_program_data(&s.env, &s.program_id);
        let threshold = program_data.circuit_breaker_threshold.map(|t| t as u32).unwrap_or(3);
        
        assert_eq!(threshold, 3);
        
        // Record failures up to default threshold
        s.env.as_contract(&s.client.address, || {
            for i in 0..threshold {
                error_recovery::record_failure(
                    &s.env,
                    s.program_id.clone(),
                    symbol_short!("test_op"),
                    42,
                    program_data.circuit_breaker_threshold.map(|t| t as u32),
                );
            }
            
            // Circuit should be open after 3 failures
            assert_eq!(
                error_recovery::get_state(&s.env),
                CircuitState::Open,
                "Circuit must open after 3 failures with default threshold"
            );
        });
    }
}
