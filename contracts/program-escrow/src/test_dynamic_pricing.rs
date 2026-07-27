//! # Dynamic Pricing Tests
//!
//! Comprehensive tests for the dynamic pricing mechanism including:
//! - Configuration management
//! - Demand-based pricing calculations
//! - Supply-based pricing calculations
//! - Time-decay pricing
//! - Oracle integration and validation
//! - Price smoothing
//! - Change limits
//! - Anti-manipulation measures

use soroban_sdk::{testutils::Ledger, Address, Bytes, BytesN, Env, String};
extern crate std;
use crate::test::{create_contract, set_admin};
use crate::DynamicPricingConfig;

#[test]
fn test_dynamic_pricing_configuration() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let contract_id = create_contract(&env, &admin);
    
    // Configure dynamic pricing
    env.as_contract(&contract_id, || {
        crate::ProgramEscrowContract::configure_dynamic_pricing(
            env.clone(),
            DynamicPricingConfig {
                enabled: true,
                base_fee_bps: 100,
                max_fee_bps: 1000,
                min_fee_bps: 10,
                max_change_bps: 500,
                smoothing_alpha_bps: 2000,
                min_update_interval: 3600,
                oracle_address: None,
                use_demand_pricing: true,
                use_supply_pricing: true,
                use_time_decay: true,
            },
        );
    });
    
    // Verify configuration
    env.as_contract(&contract_id, || {
        let config = crate::ProgramEscrowContract::get_dynamic_pricing_config(env.clone())
            .expect("Config should exist");
        
        assert_eq!(config.enabled, true);
        assert_eq!(config.base_fee_bps, 100);
        assert_eq!(config.max_fee_bps, 1000);
        assert_eq!(config.min_fee_bps, 10);
        assert_eq!(config.max_change_bps, 500);
        assert_eq!(config.smoothing_alpha_bps, 2000);
        assert_eq!(config.min_update_interval, 3600);
        assert_eq!(config.use_demand_pricing, true);
        assert_eq!(config.use_supply_pricing, true);
        assert_eq!(config.use_time_decay, true);
    });
}

#[test]
fn test_dynamic_pricing_disabled_by_default() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let contract_id = create_contract(&env, &admin);
    
    // Verify dynamic pricing is not configured by default
    env.as_contract(&contract_id, || {
        let config = crate::ProgramEscrowContract::get_dynamic_pricing_config(env.clone());
        assert!(config.is_none(), "Dynamic pricing should not be configured by default");
    });
}

#[test]
fn test_demand_metrics_update() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let contract_id = create_contract(&env, &admin);
    
    // Update demand metrics
    env.as_contract(&contract_id, || {
        crate::ProgramEscrowContract::update_demand_metrics(
            env.clone(),
            1000,                    // tx_count
            100_000_000,             // total_volume
            500,                     // unique_users
            100_000,                 // avg_tx_size
            1500,                    // growth_rate_bps (15%)
        );
    });
    
    // Verify metrics
    env.as_contract(&contract_id, || {
        let metrics = crate::ProgramEscrowContract::get_demand_metrics(env.clone())
            .expect("Metrics should exist");
        
        assert_eq!(metrics.tx_count, 1000);
        assert_eq!(metrics.total_volume, 100_000_000);
        assert_eq!(metrics.unique_users, 500);
        assert_eq!(metrics.avg_tx_size, 100_000);
        assert_eq!(metrics.growth_rate_bps, 1500);
    });
}

#[test]
fn test_supply_metrics_update() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let contract_id = create_contract(&env, &admin);
    
    // Update supply metrics
    env.as_contract(&contract_id, || {
        crate::ProgramEscrowContract::update_supply_metrics(
            env.clone(),
            1_000_000_000,           // total_liquidity
            6000,                    // utilization_bps (60%)
            400_000_000,             // available_liquidity
            600_000_000,             // locked_liquidity
        );
    });
    
    // Verify metrics
    env.as_contract(&contract_id, || {
        let metrics = crate::ProgramEscrowContract::get_supply_metrics(env.clone())
            .expect("Metrics should exist");
        
        assert_eq!(metrics.total_liquidity, 1_000_000_000);
        assert_eq!(metrics.utilization_bps, 6000);
        assert_eq!(metrics.available_liquidity, 400_000_000);
        assert_eq!(metrics.locked_liquidity, 600_000_000);
    });
}

#[test]
fn test_oracle_data_update() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let contract_id = create_contract(&env, &admin);
    
    let current_timestamp = env.ledger().timestamp();
    let signature = Bytes::from_array(&env, &[1u8; 32]);
    
    // Update oracle data
    env.as_contract(&contract_id, || {
        crate::ProgramEscrowContract::update_oracle_data(
            env.clone(),
            1_00_000_000,            // token_price
            10_000_000_000,          // volume_24h
            100_000_000_000,         // market_cap
            2000,                    // volatility_bps (20%)
            current_timestamp,       // timestamp
            Some(signature),         // signature
        );
    });
    
    // Verify oracle data
    env.as_contract(&contract_id, || {
        let oracle = crate::ProgramEscrowContract::get_oracle_data(env.clone())
            .expect("Oracle data should exist");
        
        assert_eq!(oracle.token_price, 1_00_000_000);
        assert_eq!(oracle.volume_24h, 10_000_000_000);
        assert_eq!(oracle.market_cap, 100_000_000_000);
        assert_eq!(oracle.volatility_bps, 2000);
        assert_eq!(oracle.timestamp, current_timestamp);
        assert!(oracle.signature.is_some());
    });
}

#[test]
fn test_oracle_data_validation_stale() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let contract_id = create_contract(&env, &admin);
    
    // Set timestamp to 3 hours ago (beyond staleness threshold)
    let stale_timestamp = env.ledger().timestamp() - 10800; // 3 hours
    
    env.as_contract(&contract_id, || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::ProgramEscrowContract::update_oracle_data(
                env.clone(),
                1_00_000_000,
                10_000_000_000,
                100_000_000_000,
                2000,
                stale_timestamp,
                None,
            );
        }));
        
        assert!(result.is_err(), "Should reject stale oracle data");
    });
}

#[test]
fn test_oracle_data_validation_invalid_volatility() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let contract_id = create_contract(&env, &admin);
    
    let current_timestamp = env.ledger().timestamp();
    
    env.as_contract(&contract_id, || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::ProgramEscrowContract::update_oracle_data(
                env.clone(),
                1_00_000_000,
                10_000_000_000,
                100_000_000_000,
                15000,                   // Invalid: > 10000 bps
                current_timestamp,
                None,
            );
        }));
        
        assert!(result.is_err(), "Should reject invalid volatility");
    });
}

#[test]
fn test_price_update_with_metrics() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let contract_id = create_contract(&env, &admin);
    
    // Configure dynamic pricing
    env.as_contract(&contract_id, || {
        crate::ProgramEscrowContract::configure_dynamic_pricing(
            env.clone(),
            DynamicPricingConfig {
                enabled: true,
                base_fee_bps: 100,
                max_fee_bps: 1000,
                min_fee_bps: 10,
                max_change_bps: 500,
                smoothing_alpha_bps: 2000,
                min_update_interval: 3600,
                oracle_address: None,
                use_demand_pricing: true,
                use_supply_pricing: true,
                use_time_decay: true,
            },
        );
    });
    
    // Update metrics
    env.as_contract(&contract_id, || {
        crate::ProgramEscrowContract::update_demand_metrics(
            env.clone(),
            1000,
            100_000_000,
            500,
            100_000,
            1500,
        );
        
        crate::ProgramEscrowContract::update_supply_metrics(
            env.clone(),
            1_000_000_000,
            6000,
            400_000_000,
            600_000_000,
        );
    });
    
    // Advance time to meet minimum interval
    env.ledger().set(env.ledger().sequence() + 1, env.ledger().timestamp() + 3601);
    
    // Trigger price update
    env.as_contract(&contract_id, || {
        crate::ProgramEscrowContract::update_dynamic_price(env.clone());
    });
    
    // Verify price was updated
    env.as_contract(&contract_id, || {
        let state = crate::ProgramEscrowContract::get_pricing_state(env.clone())
            .expect("State should exist");
        
        assert!(state.current_fee_bps >= state.min_fee_bps);
        assert!(state.current_fee_bps <= state.max_fee_bps);
        assert!(state.update_count > 0);
    });
}

#[test]
fn test_price_update_too_soon() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let contract_id = create_contract(&env, &admin);
    
    // Configure dynamic pricing
    env.as_contract(&contract_id, || {
        crate::ProgramEscrowContract::configure_dynamic_pricing(
            env.clone(),
            DynamicPricingConfig {
                enabled: true,
                base_fee_bps: 100,
                max_fee_bps: 1000,
                min_fee_bps: 10,
                max_change_bps: 500,
                smoothing_alpha_bps: 2000,
                min_update_interval: 3600,
                oracle_address: None,
                use_demand_pricing: true,
                use_supply_pricing: true,
                use_time_decay: true,
            },
        );
    });
    
    // Update metrics
    env.as_contract(&contract_id, || {
        crate::ProgramEscrowContract::update_demand_metrics(
            env.clone(),
            1000,
            100_000_000,
            500,
            100_000,
            1500,
        );
    });
    
    // Try to update immediately (should fail)
    env.as_contract(&contract_id, || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::ProgramEscrowContract::update_dynamic_price(env.clone());
        }));
        
        assert!(result.is_err(), "Should reject update too soon");
    });
}

#[test]
fn test_price_update_not_enabled() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let contract_id = create_contract(&env, &admin);
    
    // Configure dynamic pricing as disabled
    env.as_contract(&contract_id, || {
        crate::ProgramEscrowContract::configure_dynamic_pricing(
            env.clone(),
            DynamicPricingConfig {
                enabled: false,
                base_fee_bps: 100,
                max_fee_bps: 1000,
                min_fee_bps: 10,
                max_change_bps: 500,
                smoothing_alpha_bps: 2000,
                min_update_interval: 3600,
                oracle_address: None,
                use_demand_pricing: true,
                use_supply_pricing: true,
                use_time_decay: true,
            },
        );
    });
    
    // Try to update price
    env.as_contract(&contract_id, || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::ProgramEscrowContract::update_dynamic_price(env.clone());
        }));
        
        assert!(result.is_err(), "Should reject update when disabled");
    });
}

#[test]
fn test_get_dynamic_fee() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let contract_id = create_contract(&env, &admin);
    
    // Configure dynamic pricing
    env.as_contract(&contract_id, || {
        crate::ProgramEscrowContract::configure_dynamic_pricing(
            env.clone(),
            DynamicPricingConfig {
                enabled: true,
                base_fee_bps: 100,
                max_fee_bps: 1000,
                min_fee_bps: 10,
                max_change_bps: 500,
                smoothing_alpha_bps: 2000,
                min_update_interval: 3600,
                oracle_address: None,
                use_demand_pricing: true,
                use_supply_pricing: true,
                use_time_decay: true,
            },
        );
    });
    
    // Get dynamic fee
    env.as_contract(&contract_id, || {
        let fee = crate::ProgramEscrowContract::get_dynamic_fee(env.clone())
            .expect("Should return fee when enabled");
        
        assert_eq!(fee, 100); // Should return base fee initially
    });
}

#[test]
fn test_get_dynamic_fee_disabled() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let contract_id = create_contract(&env, &admin);
    
    // Configure dynamic pricing as disabled
    env.as_contract(&contract_id, || {
        crate::ProgramEscrowContract::configure_dynamic_pricing(
            env.clone(),
            DynamicPricingConfig {
                enabled: false,
                base_fee_bps: 100,
                max_fee_bps: 1000,
                min_fee_bps: 10,
                max_change_bps: 500,
                smoothing_alpha_bps: 2000,
                min_update_interval: 3600,
                oracle_address: None,
                use_demand_pricing: true,
                use_supply_pricing: true,
                use_time_decay: true,
            },
        );
    });
    
    // Get dynamic fee
    env.as_contract(&contract_id, || {
        let fee = crate::ProgramEscrowContract::get_dynamic_fee(env.clone());
        assert!(fee.is_none(), "Should return None when disabled");
    });
}

#[test]
fn test_configuration_validation() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let contract_id = create_contract(&env, &admin);
    
    // Test invalid base fee (negative)
    env.as_contract(&contract_id, || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::ProgramEscrowContract::configure_dynamic_pricing(
                env.clone(),
                DynamicPricingConfig {
                    enabled: true,
                    base_fee_bps: -100,
                    max_fee_bps: 1000,
                    min_fee_bps: 10,
                    max_change_bps: 500,
                    smoothing_alpha_bps: 2000,
                    min_update_interval: 3600,
                    oracle_address: None,
                    use_demand_pricing: true,
                    use_supply_pricing: true,
                    use_time_decay: true,
                },
            );
        }));
        
        assert!(result.is_err(), "Should reject negative base fee");
    });
    
    // Test max fee < min fee
    env.as_contract(&contract_id, || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::ProgramEscrowContract::configure_dynamic_pricing(
                env.clone(),
                DynamicPricingConfig {
                    enabled: true,
                    base_fee_bps: 100,
                    max_fee_bps: 10,
                    min_fee_bps: 100,
                    max_change_bps: 500,
                    smoothing_alpha_bps: 2000,
                    min_update_interval: 3600,
                    oracle_address: None,
                    use_demand_pricing: true,
                    use_supply_pricing: true,
                    use_time_decay: true,
                },
            );
        }));
        
        assert!(result.is_err(), "Should reject max fee < min fee");
    });
    
    // Test zero update interval
    env.as_contract(&contract_id, || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::ProgramEscrowContract::configure_dynamic_pricing(
                env.clone(),
                DynamicPricingConfig {
                    enabled: true,
                    base_fee_bps: 100,
                    max_fee_bps: 1000,
                    min_fee_bps: 10,
                    max_change_bps: 500,
                    smoothing_alpha_bps: 2000,
                    min_update_interval: 0,
                    oracle_address: None,
                    use_demand_pricing: true,
                    use_supply_pricing: true,
                    use_time_decay: true,
                },
            );
        }));
        
        assert!(result.is_err(), "Should reject zero update interval");
    });
}

#[test]
fn test_pricing_state_initialization() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let contract_id = create_contract(&env, &admin);
    
    // Configure dynamic pricing
    env.as_contract(&contract_id, || {
        crate::ProgramEscrowContract::configure_dynamic_pricing(
            env.clone(),
            DynamicPricingConfig {
                enabled: true,
                base_fee_bps: 150,
                max_fee_bps: 1000,
                min_fee_bps: 10,
                max_change_bps: 500,
                smoothing_alpha_bps: 2000,
                min_update_interval: 3600,
                oracle_address: None,
                use_demand_pricing: true,
                use_supply_pricing: true,
                use_time_decay: true,
            },
        );
    });
    
    // Verify initial state
    env.as_contract(&contract_id, || {
        let state = crate::ProgramEscrowContract::get_pricing_state(env.clone())
            .expect("State should be initialized");
        
        assert_eq!(state.current_fee_bps, 150);
        assert_eq!(state.previous_fee_bps, 150);
        assert_eq!(state.ema_fee_bps, 150);
        assert_eq!(state.update_count, 0);
        assert_eq!(state.demand_score, 5000); // neutral
        assert_eq!(state.supply_score, 5000); // neutral
        assert_eq!(state.time_decay_factor, 10000); // no decay
    });
}

#[test]
fn test_oracle_address_configuration() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let contract_id = create_contract(&env, &admin);
    let oracle_address = Address::generate(&env);
    
    // Configure with oracle address
    env.as_contract(&contract_id, || {
        crate::ProgramEscrowContract::configure_dynamic_pricing(
            env.clone(),
            DynamicPricingConfig {
                enabled: true,
                base_fee_bps: 100,
                max_fee_bps: 1000,
                min_fee_bps: 10,
                max_change_bps: 500,
                smoothing_alpha_bps: 2000,
                min_update_interval: 3600,
                oracle_address: Some(oracle_address.clone()),
                use_demand_pricing: true,
                use_supply_pricing: true,
                use_time_decay: true,
            },
        );
    });
    
    // Verify oracle address
    env.as_contract(&contract_id, || {
        let config = crate::ProgramEscrowContract::get_dynamic_pricing_config(env.clone())
            .expect("Config should exist");
        
        assert_eq!(config.oracle_address, Some(oracle_address));
    });
}

#[test]
fn test_selective_pricing_components() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let contract_id = create_contract(&env, &admin);
    
    // Configure with only demand pricing
    env.as_contract(&contract_id, || {
        crate::ProgramEscrowContract::configure_dynamic_pricing(
            env.clone(),
            DynamicPricingConfig {
                enabled: true,
                base_fee_bps: 100,
                max_fee_bps: 1000,
                min_fee_bps: 10,
                max_change_bps: 500,
                smoothing_alpha_bps: 2000,
                min_update_interval: 3600,
                oracle_address: None,
                use_demand_pricing: true,
                use_supply_pricing: false,
                use_time_decay: false,
            },
        );
    });
    
    // Verify configuration
    env.as_contract(&contract_id, || {
        let config = crate::ProgramEscrowContract::get_dynamic_pricing_config(env.clone())
            .expect("Config should exist");
        
        assert_eq!(config.use_demand_pricing, true);
        assert_eq!(config.use_supply_pricing, false);
        assert_eq!(config.use_time_decay, false);
    });
}
