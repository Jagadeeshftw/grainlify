#[cfg(test)]
mod test {
    use crate::threshold_monitor::{self, ThresholdConfig, WindowMetrics};
    use crate::{ProgramEscrowContract, ProgramEscrowContractClient};
    use soroban_sdk::{testutils::Address as _, Address, Env};

    fn setup_test(env: &Env) -> (ProgramEscrowContractClient, Address) {
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        client.initialize_contract(&admin);
        client.set_circuit_admin(&admin, &None);
        (client, admin)
    }

    #[test]
    fn test_threshold_config_initialization() {
        let env = Env::default();
        let (client, _admin) = setup_test(&env);
        
        // Initialize threshold monitoring
        client.init_threshold_monitoring();
        
        // Get config and verify defaults
        let config = client.get_threshold_config();
        assert_eq!(config.failure_rate_threshold(), 10);
        assert!(config.outflow_volume_threshold > 0);
        assert!(config.max_single_payout > 0);
        assert_eq!(config.time_window_secs(), 600);
        assert_eq!(config.cooldown_period_secs(), 300);
    }

    #[test]
    fn test_threshold_config_validation() {
        let env = Env::default();
        
        // Test invalid failure threshold (too high)
        let mut config = ThresholdConfig::default();
        config.set_failure_rate_threshold(2000);
        assert!(config.validate().is_err());
        
        // Test invalid failure threshold (zero)
        config.set_failure_rate_threshold(0);
        assert!(config.validate().is_err());
        
        // Test invalid time window (too short)
        config.set_failure_rate_threshold(10);
        config.set_time_window_secs(5);
        assert!(config.validate().is_err());
        
        // Test invalid time window (too long)
        config.set_time_window_secs(100000);
        assert!(config.validate().is_err());
        
        // Test valid configuration
        config.set_time_window_secs(600);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_metrics_tracking() {
        let env = Env::default();
        
        // Record some operations
        threshold_monitor::init_threshold_monitor(&env);
        threshold_monitor::record_operation_success(&env);
        threshold_monitor::record_operation_success(&env);
        threshold_monitor::record_operation_failure(&env);
        
        let metrics = threshold_monitor::get_current_metrics(&env);
        assert_eq!(metrics.success_count, 2);
        assert_eq!(metrics.failure_count, 1);
    }

    #[test]
    fn test_outflow_tracking() {
        let env = Env::default();
        
        threshold_monitor::init_threshold_monitor(&env);
        threshold_monitor::record_outflow(&env, 1000);
        threshold_monitor::record_outflow(&env, 2000);
        threshold_monitor::record_outflow(&env, 500);
        
        let metrics = threshold_monitor::get_current_metrics(&env);
        assert_eq!(metrics.total_outflow, 3500);
        assert_eq!(metrics.max_single_outflow, 2000);
    }

    #[test]
    fn test_failure_threshold_breach() {
        let env = Env::default();
        
        let mut config = ThresholdConfig::default();
        config.set_failure_rate_threshold(3);
        threshold_monitor::init_threshold_monitor(&env);
        threshold_monitor::set_threshold_config(&env, config).unwrap();
        
        // Record failures up to threshold
        threshold_monitor::record_operation_failure(&env);
        threshold_monitor::record_operation_failure(&env);
        
        // Should not breach yet
        assert!(threshold_monitor::check_thresholds(&env).is_ok());
        
        // One more failure should breach
        threshold_monitor::record_operation_failure(&env);
        assert!(threshold_monitor::check_thresholds(&env).is_err());
    }

    #[test]
    fn test_outflow_threshold_breach() {
        let env = Env::default();
        
        let mut config = ThresholdConfig::default();
        config.outflow_volume_threshold = 5000;
        threshold_monitor::init_threshold_monitor(&env);
        threshold_monitor::set_threshold_config(&env, config).unwrap();
        
        // Record outflows below threshold
        threshold_monitor::record_outflow(&env, 2000);
        assert!(threshold_monitor::check_thresholds(&env).is_ok());
        
        // Exceed threshold
        threshold_monitor::record_outflow(&env, 4000);
        assert!(threshold_monitor::check_thresholds(&env).is_err());
    }

    #[test]
    fn test_single_payout_threshold() {
        let env = Env::default();
        
        let mut config = ThresholdConfig::default();
        config.max_single_payout = 1000;
        threshold_monitor::init_threshold_monitor(&env);
        threshold_monitor::set_threshold_config(&env, config).unwrap();
        
        // Check amount below threshold
        assert!(threshold_monitor::check_single_payout_threshold(&env, 500).is_ok());
        
        // Check amount at threshold
        assert!(threshold_monitor::check_single_payout_threshold(&env, 1000).is_err());
        
        // Check amount above threshold
        assert!(threshold_monitor::check_single_payout_threshold(&env, 1500).is_err());
    }

    #[test]
    fn test_metrics_reset() {
        let env = Env::default();
        let (_client, admin) = setup_test(&env);
        
        threshold_monitor::init_threshold_monitor(&env);
        
        // Record some metrics
        threshold_monitor::record_operation_failure(&env);
        threshold_monitor::record_operation_failure(&env);
        threshold_monitor::record_outflow(&env, 1000);
        
        let metrics_before = threshold_monitor::get_current_metrics(&env);
        assert_eq!(metrics_before.failure_count, 2);
        assert_eq!(metrics_before.total_outflow, 1000);
        
        // Reset metrics
        threshold_monitor::reset_metrics(&env, &admin);
        
        let metrics_after = threshold_monitor::get_current_metrics(&env);
        assert_eq!(metrics_after.failure_count, 0);
        assert_eq!(metrics_after.success_count, 0);
        assert_eq!(metrics_after.total_outflow, 0);
    }

    #[test]
    fn test_precedence_per_program_threshold_overrides_global_rate_limit() {
        let env = Env::default();
        env.mock_all_auths();
        
        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract(token_admin.clone());
        let (client, admin) = setup_test(&env);
        
        let creator = Address::generate(&env);
        let initial_balance = 5000;
        soroban_sdk::token::StellarAssetClient::new(&env, &token_id).mint(&creator, &initial_balance);
        
        let program_id = soroban_sdk::String::from_str(&env, "PROG1");
        client.init_program(
            &program_id,
            &admin,
            &token_id,
            &creator,
            &Some(initial_balance),
            &None,
        );
        client.publish_program();
        
        // Set global rate limit config to be very restrictive (max 1 operation).
        client.update_rate_limit_config(&3600, &1, &60);
        
        // Set per-program spend threshold to 1000.
        client.set_program_spend_threshold(&program_id, &1000);
        
        // Scenario 1: Violates global rate limit (batch size 2 > max_operations 1)
        // but satisfies per-program threshold (2 * 100 = 200 <= 1000)
        let mut recipients = soroban_sdk::Vec::new(&env);
        let mut amounts = soroban_sdk::Vec::new(&env);
        recipients.push_back(Address::generate(&env));
        recipients.push_back(Address::generate(&env));
        amounts.push_back(100);
        amounts.push_back(100);
        
        // This should SUCCEED because global RateLimitConfig is not strictly enforced on payout sizes.
        // The per-program threshold is the only effectively enforced limit.
        let result = client.try_batch_payout(&recipients, &amounts, &None);
        assert!(result.is_ok(), "Payout should succeed, global rate limit is not enforced for batch size");
        
        // Scenario 2: Satisfies global rate limit (batch size 1 <= max_operations 1)
        // but violates per-program threshold (1 * 2000 = 2000 > 1000)
        let mut recipients2 = soroban_sdk::Vec::new(&env);
        let mut amounts2 = soroban_sdk::Vec::new(&env);
        recipients2.push_back(Address::generate(&env));
        amounts2.push_back(2000);
        
        // This should FAIL because the per-program spend threshold is strictly enforced.
        let result2 = client.try_batch_payout(&recipients2, &amounts2, &None);
        assert!(result2.is_err(), "Payout should fail, per-program spend threshold is strictly enforced");
    }

    #[test]
    fn test_threshold_config_packing_boundary_values() {
        let mut config = ThresholdConfig::default();

        // 1. Minimum boundaries
        config.set_failure_rate_threshold(0);
        config.set_time_window_secs(0);
        config.set_cooldown_period_secs(0);
        config.set_cooldown_multiplier(0);

        assert_eq!(config.failure_rate_threshold(), 0);
        assert_eq!(config.time_window_secs(), 0);
        assert_eq!(config.cooldown_period_secs(), 0);
        assert_eq!(config.cooldown_multiplier(), 0);

        // 2. Maximum boundaries (based on bit allocation)
        // failure_rate_threshold: 16 bits (max 65535)
        config.set_failure_rate_threshold(65535);
        // time_window_secs: 24 bits (max 16777215)
        config.set_time_window_secs(16777215);
        // cooldown_period_secs: 16 bits (max 65535)
        config.set_cooldown_period_secs(65535);
        // cooldown_multiplier: 8 bits (max 255)
        config.set_cooldown_multiplier(255);

        assert_eq!(config.failure_rate_threshold(), 65535);
        assert_eq!(config.time_window_secs(), 16777215);
        assert_eq!(config.cooldown_period_secs(), 65535);
        assert_eq!(config.cooldown_multiplier(), 255);

        // 3. Ensure they do not overwrite each other
        config.set_failure_rate_threshold(0);
        assert_eq!(config.failure_rate_threshold(), 0);
        assert_eq!(config.time_window_secs(), 16777215);
        assert_eq!(config.cooldown_period_secs(), 65535);
        assert_eq!(config.cooldown_multiplier(), 255);

        config.set_time_window_secs(0);
        assert_eq!(config.time_window_secs(), 0);
        assert_eq!(config.cooldown_period_secs(), 65535);
        assert_eq!(config.cooldown_multiplier(), 255);

        config.set_cooldown_period_secs(0);
        assert_eq!(config.cooldown_period_secs(), 0);
        assert_eq!(config.cooldown_multiplier(), 255);

        config.set_cooldown_multiplier(0);
        assert_eq!(config.cooldown_multiplier(), 0);
        
        // 4. Over-boundary (should truncate or mask safely without panicking)
        config.set_failure_rate_threshold(65536); // one over max (16 bits)
        assert_eq!(config.failure_rate_threshold(), 0); // masked to 0

        config.set_time_window_secs(16777216); // one over max (24 bits)
        assert_eq!(config.time_window_secs(), 0); // masked to 0
    }
}

