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
        assert_eq!(config.failure_rate_threshold, 10);
        assert!(config.outflow_volume_threshold > 0);
        assert!(config.max_single_payout > 0);
        assert_eq!(config.time_window_secs, 600);
        assert_eq!(config.cooldown_period_secs, 300);
    }

    #[test]
    fn test_threshold_config_validation() {
        let env = Env::default();
        
        // Test invalid failure threshold (too high)
        let mut config = ThresholdConfig::default();
        config.failure_rate_threshold = 2000;
        assert!(config.validate().is_err());
        
        // Test invalid failure threshold (zero)
        config.failure_rate_threshold = 0;
        assert!(config.validate().is_err());
        
        // Test invalid time window (too short)
        config.failure_rate_threshold = 10;
        config.time_window_secs = 5;
        assert!(config.validate().is_err());
        
        // Test invalid time window (too long)
        config.time_window_secs = 100000;
        assert!(config.validate().is_err());
        
        // Test valid configuration
        config.time_window_secs = 600;
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
        config.failure_rate_threshold = 3;
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
}
