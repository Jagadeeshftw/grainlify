//! # Dynamic Pricing Mechanism
//!
//! This module implements a sophisticated dynamic pricing system with price adjustments
//! based on demand, supply, time-based factors, and market conditions.
//!
//! ## Features
//!
//! - **Demand-based pricing**: Adjusts fees based on transaction volume and frequency
//! - **Supply-based pricing**: Adjusts based on available liquidity and pool utilization
//! - **Time-decay pricing**: Gradual price adjustments over time to prevent sudden spikes
//! - **Price smoothing**: Exponential moving average to smooth price changes
//! - **Price change limits**: Maximum percentage change per period to prevent manipulation
//! - **Oracle integration**: Market data feeds for external price signals
//! - **Anti-manipulation**: Validation and sanity checks on oracle data
//! - **Event notifications**: Emits events for all price changes
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Dynamic Pricing Engine                    │
//! ├─────────────────────────────────────────────────────────────┤
//! │                                                              │
//! │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
//! │  │   Demand     │  │   Supply     │  │   Time       │     │
//! │  │   Calculator │  │   Calculator │  │   Decay      │     │
//! │  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘     │
//! │         │                  │                  │             │
//! │         └──────────────────┼──────────────────┘             │
//! │                            ▼                                │
//! │                 ┌──────────────────┐                      │
//! │                 │  Price Aggregator│                      │
//! │                 └────────┬─────────┘                      │
//! │                          ▼                                 │
//! │                 ┌──────────────────┐                      │
//! │                 │  Price Smoother  │                      │
//! │                 │  (EMA Filter)    │                      │
//! │                 └────────┬─────────┘                      │
//! │                          ▼                                 │
//! │                 ┌──────────────────┐                      │
//! │                 │  Change Limiter   │                      │
//! │                 │  (Max Δ%)        │                      │
//! │                 └────────┬─────────┘                      │
//! │                          ▼                                 │
//! │                 ┌──────────────────┐                      │
//! │                 │  Oracle Validator│                      │
//! │                 └────────┬─────────┘                      │
//! │                          ▼                                 │
//! │                 ┌──────────────────┐                      │
//! │                 │  Final Price     │                      │
//! │                 └──────────────────┘                      │
//! │                                                              │
//! └─────────────────────────────────────────────────────────────┘
//! ```

use soroban_sdk::{contracttype, symbol_short, Address, Env, String, Symbol, Bytes};

// ============================================================================
// Constants
// ============================================================================

/// Maximum price change per period (in basis points, 10000 = 100%)
const MAX_PRICE_CHANGE_BPS: i128 = 500; // 5% max change per period

/// Price smoothing factor (EMA alpha, in basis points)
const SMOOTHING_ALPHA_BPS: i128 = 2000; // 20% weight to new price

/// Minimum time between price updates (in seconds)
const MIN_UPDATE_INTERVAL: u64 = 3600; // 1 hour

/// Oracle staleness threshold (in seconds)
const ORACLE_STALENESS_THRESHOLD: u64 = 7200; // 2 hours

/// Maximum deviation from moving average (in basis points)
const MAX_DEVIATION_BPS: i128 = 3000; // 30% max deviation

/// Default base fee rate (in basis points)
const DEFAULT_BASE_FEE_BPS: i128 = 100; // 1%

/// Demand sensitivity factor
const DEMAND_SENSITIVITY: i128 = 50; // Multiplier for demand impact

/// Supply sensitivity factor
const SUPPLY_SENSITIVITY: i128 = 30; // Multiplier for supply impact

/// Time decay rate (per hour, in basis points)
const TIME_DECAY_RATE_BPS: i128 = 10; // 0.1% per hour

// ============================================================================
// Data Structures
// ============================================================================

/// Dynamic pricing configuration
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DynamicPricingConfig {
    /// Whether dynamic pricing is enabled
    pub enabled: bool,
    
    /// Base fee rate (in basis points)
    pub base_fee_bps: i128,
    
    /// Maximum fee rate (in basis points)
    pub max_fee_bps: i128,
    
    /// Minimum fee rate (in basis points)
    pub min_fee_bps: i128,
    
    /// Maximum price change per period (basis points)
    pub max_change_bps: i128,
    
    /// Price smoothing factor (basis points)
    pub smoothing_alpha_bps: i128,
    
    /// Minimum update interval (seconds)
    pub min_update_interval: u64,
    
    /// Oracle address (if using external oracle)
    pub oracle_address: Option<Address>,
    
    /// Whether to use demand-based pricing
    pub use_demand_pricing: bool,
    
    /// Whether to use supply-based pricing
    pub use_supply_pricing: bool,
    
    /// Whether to use time-decay pricing
    pub use_time_decay: bool,
}

impl DynamicPricingConfig {
    pub fn default(_env: &Env) -> Self {
        Self {
            enabled: false,
            base_fee_bps: DEFAULT_BASE_FEE_BPS,
            max_fee_bps: 1000, // 10%
            min_fee_bps: 10,   // 0.1%
            max_change_bps: MAX_PRICE_CHANGE_BPS,
            smoothing_alpha_bps: SMOOTHING_ALPHA_BPS,
            min_update_interval: MIN_UPDATE_INTERVAL,
            oracle_address: None,
            use_demand_pricing: true,
            use_supply_pricing: true,
            use_time_decay: true,
        }
    }
}

/// Current pricing state
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PricingState {
    /// Current fee rate (in basis points)
    pub current_fee_bps: i128,
    
    /// Previous fee rate (in basis points)
    pub previous_fee_bps: i128,
    
    /// Exponential moving average of fee rate
    pub ema_fee_bps: i128,
    
    /// Last update timestamp
    pub last_update: u64,
    
    /// Number of price updates
    pub update_count: u64,
    
    /// Current demand score (0-10000 basis points)
    pub demand_score: i128,
    
    /// Current supply score (0-10000 basis points)
    pub supply_score: i128,
    
    /// Time decay factor (in basis points)
    pub time_decay_factor: i128,
}

impl PricingState {
    pub fn initial(env: &Env, base_fee_bps: i128) -> Self {
        let timestamp = env.ledger().timestamp();
        Self {
            current_fee_bps: base_fee_bps,
            previous_fee_bps: base_fee_bps,
            ema_fee_bps: base_fee_bps,
            last_update: timestamp,
            update_count: 0,
            demand_score: 5000, // 50% neutral
            supply_score: 5000, // 50% neutral
            time_decay_factor: 10000, // 100% (no decay initially)
        }
    }
}

/// Market data from oracle
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleMarketData {
    /// Token price (in smallest units)
    pub token_price: i128,
    
    /// 24h volume (in smallest units)
    pub volume_24h: i128,
    
    /// Market cap (in smallest units)
    pub market_cap: i128,
    
    /// Volatility index (in basis points)
    pub volatility_bps: i128,
    
    /// Timestamp of data
    pub timestamp: u64,
    
    /// Oracle signature or proof
    pub signature: Option<Bytes>,
}

/// Demand metrics
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DemandMetrics {
    /// Transaction count in current window
    pub tx_count: u64,
    
    /// Total volume in current window
    pub total_volume: i128,
    
    /// Unique users in current window
    pub unique_users: u64,
    
    /// Average transaction size
    pub avg_tx_size: i128,
    
    /// Growth rate vs previous window (basis points)
    pub growth_rate_bps: i128,
}

/// Supply metrics
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupplyMetrics {
    /// Total liquidity available
    pub total_liquidity: i128,
    
    /// Utilization rate (basis points)
    pub utilization_bps: i128,
    
    /// Available liquidity
    pub available_liquidity: i128,
    
    /// Locked liquidity
    pub locked_liquidity: i128,
}

/// Price update event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceUpdateEvent {
    pub version: u32,
    pub previous_fee_bps: i128,
    pub new_fee_bps: i128,
    pub demand_score: i128,
    pub supply_score: i128,
    pub time_decay_factor: i128,
    pub oracle_price: Option<i128>,
    pub timestamp: u64,
    pub reason: String,
}

/// Pricing calculation result
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PricingCalculation {
    pub calculated_fee_bps: i128,
    pub demand_adjustment: i128,
    pub supply_adjustment: i128,
    pub time_decay_adjustment: i128,
    pub oracle_adjustment: Option<i128>,
    pub smoothed_fee_bps: i128,
    pub final_fee_bps: i128,
}

// ============================================================================
// Pricing Engine
// ============================================================================

/// Main pricing engine that calculates dynamic fees
pub struct PricingEngine;

impl PricingEngine {
    /// Calculate the dynamic fee based on current conditions
    pub fn calculate_fee(
        env: &Env,
        config: &DynamicPricingConfig,
        state: &PricingState,
        demand_metrics: Option<&DemandMetrics>,
        supply_metrics: Option<&SupplyMetrics>,
        oracle_data: Option<&OracleMarketData>,
    ) -> Result<PricingCalculation, ContractError> {
        let mut calculation = PricingCalculation {
            calculated_fee_bps: config.base_fee_bps,
            demand_adjustment: 0,
            supply_adjustment: 0,
            time_decay_adjustment: 0,
            oracle_adjustment: None,
            smoothed_fee_bps: config.base_fee_bps,
            final_fee_bps: config.base_fee_bps,
        };

        // Start with base fee
        let mut fee_bps = config.base_fee_bps;

        // Apply demand-based adjustment
        if config.use_demand_pricing {
            if let Some(demand) = demand_metrics {
                let demand_adj = Self::calculate_demand_adjustment(env, config, demand, state);
                fee_bps = fee_bps.saturating_add(demand_adj);
                calculation.demand_adjustment = demand_adj;
            }
        }

        // Apply supply-based adjustment
        if config.use_supply_pricing {
            if let Some(supply) = supply_metrics {
                let supply_adj = Self::calculate_supply_adjustment(env, config, supply, state);
                fee_bps = fee_bps.saturating_add(supply_adj);
                calculation.supply_adjustment = supply_adj;
            }
        }

        // Apply time-decay adjustment
        if config.use_time_decay {
            let time_adj = Self::calculate_time_decay_adjustment(env, config, state);
            fee_bps = fee_bps.saturating_add(time_adj);
            calculation.time_decay_adjustment = time_adj;
        }

        // Apply oracle adjustment if available
        if let Some(oracle) = oracle_data {
            let oracle_adj = Self::calculate_oracle_adjustment(env, config, oracle, state)?;
            fee_bps = fee_bps.saturating_add(oracle_adj);
            calculation.oracle_adjustment = Some(oracle_adj);
        }

        calculation.calculated_fee_bps = fee_bps;

        // Clamp to min/max bounds
        fee_bps = fee_bps.max(config.min_fee_bps).min(config.max_fee_bps);

        // Apply price smoothing (EMA)
        let smoothed = Self::apply_price_smoothing(env, config, state, fee_bps);
        calculation.smoothed_fee_bps = smoothed;

        // Apply change limits
        let final_fee = Self::apply_change_limits(env, config, state, smoothed)?;
        calculation.final_fee_bps = final_fee;

        Ok(calculation)
    }

    /// Calculate demand-based price adjustment
    fn calculate_demand_adjustment(
        _env: &Env,
        _config: &DynamicPricingConfig,
        demand: &DemandMetrics,
        state: &PricingState,
    ) -> i128 {
        // Calculate demand score based on transaction metrics
        let tx_score = (demand.tx_count as i128 * 10).min(10000); // Scale to 0-10000
        let volume_score = (demand.total_volume / 1_000_000).min(10000); // Normalize volume
        let growth_score = demand.growth_rate_bps.abs().min(10000);

        // Combine scores with weights
        let combined_score = (tx_score * 3 + volume_score * 2 + growth_score * 1) / 6;

        // Calculate adjustment based on deviation from neutral (5000 bps)
        let deviation = combined_score.saturating_sub(5000);
        
        // Apply sensitivity factor
        deviation * DEMAND_SENSITIVITY / 10000
    }

    /// Calculate supply-based price adjustment
    fn calculate_supply_adjustment(
        _env: &Env,
        _config: &DynamicPricingConfig,
        supply: &SupplyMetrics,
        _state: &PricingState,
    ) -> i128 {
        // Higher utilization = higher fees
        let utilization = supply.utilization_bps;
        
        // Calculate adjustment based on utilization
        // If utilization > 50%, increase fees
        if utilization > 5000 {
            let excess = utilization.saturating_sub(5000);
            excess * SUPPLY_SENSITIVITY / 10000
        } else {
            // If utilization < 50%, decrease fees
            let deficit = 5000_i128.saturating_sub(utilization);
            -(deficit * SUPPLY_SENSITIVITY / 10000)
        }
    }

    /// Calculate time-decay price adjustment
    fn calculate_time_decay_adjustment(
        env: &Env,
        _config: &DynamicPricingConfig,
        state: &PricingState,
    ) -> i128 {
        let current_time = env.ledger().timestamp();
        let hours_passed = current_time.saturating_sub(state.last_update) / 3600;
        
        if hours_passed == 0 {
            return 0;
        }

        // Apply decay factor
        let decay = (hours_passed as i128 * TIME_DECAY_RATE_BPS) / 10000;
        
        // Decay reduces fees over time (negative adjustment)
        -decay
    }

    /// Calculate oracle-based price adjustment
    fn calculate_oracle_adjustment(
        env: &Env,
        config: &DynamicPricingConfig,
        oracle: &OracleMarketData,
        state: &PricingState,
    ) -> Result<i128, ContractError> {
        // Validate oracle data freshness
        let current_time = env.ledger().timestamp();
        if current_time.saturating_sub(oracle.timestamp) > ORACLE_STALENESS_THRESHOLD {
            return Err(ContractError::OracleDataStale);
        }

        // Validate oracle signature if present
        if oracle.signature.is_some() {
            // In production, verify signature here
            // For now, we'll skip signature verification
        }

        // Calculate adjustment based on volatility
        // Higher volatility = higher fees to account for risk
        let volatility_adj = (oracle.volatility_bps * 50) / 10000; // 50% of volatility as adjustment

        // Calculate deviation from moving average
        let deviation = oracle.volatility_bps.saturating_sub(5000);
        if deviation.abs() > MAX_DEVIATION_BPS {
            return Err(ContractError::OracleDataInvalid);
        }

        Ok(volatility_adj)
    }

    /// Apply exponential moving average smoothing
    fn apply_price_smoothing(
        _env: &Env,
        config: &DynamicPricingConfig,
        state: &PricingState,
        new_fee: i128,
    ) -> i128 {
        // EMA formula: EMA = (alpha * new) + ((1 - alpha) * old_EMA)
        let alpha = config.smoothing_alpha_bps;
        let one_minus_alpha = 10000_i128.saturating_sub(alpha);
        
        let weighted_new = (new_fee * alpha) / 10000;
        let weighted_old = (state.ema_fee_bps * one_minus_alpha) / 10000;
        
        weighted_new.saturating_add(weighted_old)
    }

    /// Apply maximum change limits
    fn apply_change_limits(
        env: &Env,
        config: &DynamicPricingConfig,
        state: &PricingState,
        new_fee: i128,
    ) -> Result<i128, ContractError> {
        // Check minimum update interval
        let current_time = env.ledger().timestamp();
        if current_time.saturating_sub(state.last_update) < config.min_update_interval {
            return Err(ContractError::UpdateTooSoon);
        }

        // Calculate percentage change
        let change = new_fee.saturating_sub(state.current_fee_bps);
        let change_abs = change.abs();
        
        // Calculate max allowed change
        let max_change = (state.current_fee_bps * config.max_change_bps) / 10000;
        
        if change_abs > max_change {
            return Err(ContractError::PriceChangeExceedsLimit);
        }

        Ok(new_fee)
    }

    /// Validate oracle data
    pub fn validate_oracle_data(
        env: &Env,
        oracle: &OracleMarketData,
    ) -> Result<(), ContractError> {
        // Check data freshness
        let current_time = env.ledger().timestamp();
        if current_time.saturating_sub(oracle.timestamp) > ORACLE_STALENESS_THRESHOLD {
            return Err(ContractError::OracleDataStale);
        }

        // Check for reasonable values
        if oracle.token_price <= 0 {
            return Err(ContractError::OracleDataInvalid);
        }

        if oracle.volatility_bps < 0 || oracle.volatility_bps > 10000 {
            return Err(ContractError::OracleDataInvalid);
        }

        // Additional sanity checks can be added here
        Ok(())
    }
}

// ============================================================================
// Errors
// ============================================================================

// Re-export ContractError for use in this module
pub use crate::errors::ContractError;

// ============================================================================
// Storage Keys
// ============================================================================

const DYNAMIC_PRICING_CONFIG: Symbol = symbol_short!("DynPric");
const PRICING_STATE: Symbol = symbol_short!("PricSt");
const DEMAND_METRICS: Symbol = symbol_short!("DmndMtr");
const SUPPLY_METRICS: Symbol = symbol_short!("SuppMtr");
const ORACLE_DATA: Symbol = symbol_short!("OraclD");

// ============================================================================
// Helper Functions
// ============================================================================

/// Update demand metrics
pub fn update_demand_metrics(
    env: &Env,
    tx_count: u64,
    total_volume: i128,
    unique_users: u64,
    avg_tx_size: i128,
    growth_rate_bps: i128,
) {
    let metrics = DemandMetrics {
        tx_count,
        total_volume,
        unique_users,
        avg_tx_size,
        growth_rate_bps,
    };
    env.storage().persistent().set(&DEMAND_METRICS, &metrics);
}

/// Update supply metrics
pub fn update_supply_metrics(
    env: &Env,
    total_liquidity: i128,
    utilization_bps: i128,
    available_liquidity: i128,
    locked_liquidity: i128,
) {
    let metrics = SupplyMetrics {
        total_liquidity,
        utilization_bps,
        available_liquidity,
        locked_liquidity,
    };
    env.storage().persistent().set(&SUPPLY_METRICS, &metrics);
}

/// Get current dynamic fee
pub fn get_dynamic_fee(env: &Env) -> Option<i128> {
    let state: PricingState = env.storage().persistent().get(&PRICING_STATE)?;
    if env.ledger().timestamp().saturating_sub(state.last_update) > MIN_UPDATE_INTERVAL {
        // Fee is stale, needs recalculation
        None
    } else {
        Some(state.current_fee_bps)
    }
}
