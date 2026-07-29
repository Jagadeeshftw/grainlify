# Dynamic Pricing Mechanism Documentation

## Overview

The Dynamic Pricing Mechanism is a sophisticated fee adjustment system that automatically adapts pricing based on market conditions, demand patterns, supply constraints, and time-based factors. This ensures fair and responsive pricing while preventing manipulation through comprehensive validation and smoothing mechanisms.

## Architecture

### Pricing Pipeline

```
┌─────────────────────────────────────────────────────────────┐
│                    Dynamic Pricing Engine                    │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │   Demand     │  │   Supply     │  │   Time       │     │
│  │   Calculator │  │   Calculator │  │   Decay      │     │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘     │
│         │                  │                  │             │
│         └──────────────────┼──────────────────┘             │
│                            ▼                                │
│                 ┌──────────────────┐                      │
│                 │  Price Aggregator│                      │
│                 └────────┬─────────┘                      │
│                          ▼                                 │
│                 ┌──────────────────┐                      │
│                 │  Price Smoother  │                      │
│                 │  (EMA Filter)    │                      │
│                 └────────┬─────────┘                      │
│                          ▼                                 │
│                 ┌──────────────────┐                      │
│                 │  Change Limiter   │                      │
│                 │  (Max Δ%)        │                      │
│                 └────────┬─────────┘                      │
│                          ▼                                 │
│                 ┌──────────────────┐                      │
│                 │  Oracle Validator│                      │
│                 └────────┬─────────┘                      │
│                          ▼                                 │
│                 ┌──────────────────┐                      │
│                 │  Final Price     │                      │
│                 └──────────────────┘                      │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Components

### 1. Demand-Based Pricing

Adjusts fees based on transaction volume and user activity patterns.

**Metrics Tracked:**
- Transaction count in current window
- Total transaction volume
- Unique user count
- Average transaction size
- Growth rate vs previous window

**Calculation:**
```
demand_score = (tx_count * 10 + volume_score * 2 + growth_score) / 6
demand_adjustment = (demand_score - 5000) * DEMAND_SENSITIVITY / 10000
```

**Sensitivity:** 50 basis points multiplier for demand impact

### 2. Supply-Based Pricing

Adjusts fees based on liquidity availability and utilization rates.

**Metrics Tracked:**
- Total liquidity available
- Utilization rate (basis points)
- Available liquidity
- Locked liquidity

**Calculation:**
```
if utilization > 50%:
    supply_adjustment = (utilization - 5000) * SUPPLY_SENSITIVITY / 10000
else:
    supply_adjustment = -(5000 - utilization) * SUPPLY_SENSITIVITY / 10000
```

**Sensitivity:** 30 basis points multiplier for supply impact

### 3. Time-Decay Pricing

Gradually reduces fees over time to prevent excessive pricing during low activity periods.

**Calculation:**
```
hours_passed = (current_time - last_update) / 3600
decay = (hours_passed * TIME_DECAY_RATE_BPS) / 10000
time_decay_adjustment = -decay
```

**Decay Rate:** 10 basis points per hour (0.1% per hour)

### 4. Oracle Integration

Integrates with external price oracles for market-based pricing signals.

**Oracle Data:**
- Token price
- 24h trading volume
- Market capitalization
- Volatility index (basis points)
- Data timestamp
- Optional signature for validation

**Validation:**
- Staleness threshold: 2 hours maximum
- Price sanity checks
- Volatility bounds (0-10000 bps)
- Signature verification (optional)

**Calculation:**
```
volatility_adjustment = (volatility_bps * 50) / 10000
```

### 5. Price Smoothing

Uses Exponential Moving Average (EMA) to smooth price changes and prevent volatility.

**Formula:**
```
EMA = (alpha * new_price) + ((1 - alpha) * old_EMA)
```

**Default Alpha:** 20% weight to new price (2000 basis points)

### 6. Change Limits

Enforces maximum percentage changes per update period to prevent manipulation.

**Constraints:**
- Maximum change per period: 5% (500 basis points)
- Minimum update interval: 1 hour (3600 seconds)
- Maximum deviation from moving average: 30% (3000 basis points)

## Configuration

### DynamicPricingConfig Structure

```rust
pub struct DynamicPricingConfig {
    pub enabled: bool,              // Whether dynamic pricing is active
    pub base_fee_bps: i128,         // Base fee rate (basis points)
    pub max_fee_bps: i128,          // Maximum fee rate (basis points)
    pub min_fee_bps: i128,          // Minimum fee rate (basis points)
    pub max_change_bps: i128,       // Max change per period (basis points)
    pub smoothing_alpha_bps: i128,  // EMA smoothing factor (basis points)
    pub min_update_interval: u64,   // Min time between updates (seconds)
    pub oracle_address: Option<Address>, // Optional oracle contract
    pub use_demand_pricing: bool,    // Enable demand-based pricing
    pub use_supply_pricing: bool,    // Enable supply-based pricing
    pub use_time_decay: bool,       // Enable time-decay pricing
}
```

### Default Configuration

```rust
DynamicPricingConfig {
    enabled: false,
    base_fee_bps: 100,              // 1%
    max_fee_bps: 1000,              // 10%
    min_fee_bps: 10,                // 0.1%
    max_change_bps: 500,            // 5%
    smoothing_alpha_bps: 2000,       // 20%
    min_update_interval: 3600,      // 1 hour
    oracle_address: None,
    use_demand_pricing: true,
    use_supply_pricing: true,
    use_time_decay: true,
}
```

## Contract Functions

### Admin Functions

#### `configure_dynamic_pricing`

Configure dynamic pricing settings. Admin-only.

**Parameters:**
- `enabled: bool` - Enable/disable dynamic pricing
- `base_fee_bps: i128` - Base fee rate (0-10000)
- `max_fee_bps: i128` - Maximum fee rate
- `min_fee_bps: i128` - Minimum fee rate
- `max_change_bps: i128` - Maximum change per period
- `smoothing_alpha_bps: i128` - Smoothing factor
- `min_update_interval: u64` - Minimum update interval
- `oracle_address: Option<Address>` - Oracle contract address
- `use_demand_pricing: bool` - Enable demand pricing
- `use_supply_pricing: bool` - Enable supply pricing
- `use_time_decay: bool` - Enable time decay

**Event:** `DynPricCfg`

#### `update_demand_metrics`

Update demand metrics. Admin-only.

**Parameters:**
- `tx_count: u64` - Transaction count
- `total_volume: i128` - Total volume
- `unique_users: u64` - Unique users
- `avg_tx_size: i128` - Average transaction size
- `growth_rate_bps: i128` - Growth rate

#### `update_supply_metrics`

Update supply metrics. Admin-only.

**Parameters:**
- `total_liquidity: i128` - Total liquidity
- `utilization_bps: i128` - Utilization rate
- `available_liquidity: i128` - Available liquidity
- `locked_liquidity: i128` - Locked liquidity

#### `update_oracle_data`

Update oracle data. Admin-only.

**Parameters:**
- `token_price: i128` - Token price
- `volume_24h: i128` - 24h volume
- `market_cap: i128` - Market cap
- `volatility_bps: i128` - Volatility index
- `timestamp: u64` - Data timestamp
- `signature: Option<Bytes>` - Oracle signature

#### `update_dynamic_price`

Trigger a dynamic price update. Admin-only.

**Event:** `PriceUpd`

### Query Functions

#### `get_dynamic_pricing_config`

Get current dynamic pricing configuration.

**Returns:** `Option<DynamicPricingConfig>`

#### `get_pricing_state`

Get current pricing state.

**Returns:** `Option<PricingState>`

#### `get_dynamic_fee`

Get current dynamic fee rate.

**Returns:** `Option<i128>` (fee in basis points)

#### `get_demand_metrics`

Get current demand metrics.

**Returns:** `Option<DemandMetrics>`

#### `get_supply_metrics`

Get current supply metrics.

**Returns:** `Option<SupplyMetrics>`

#### `get_oracle_data`

Get current oracle data.

**Returns:** `Option<OracleMarketData>`

## Security Features

### Anti-Manipulation

1. **Oracle Validation:**
   - Staleness checks (2-hour threshold)
   - Price sanity checks
   - Volatility bounds
   - Signature verification

2. **Change Limits:**
   - Maximum 5% change per period
   - Minimum 1-hour update interval
   - Maximum 30% deviation from moving average

3. **Price Smoothing:**
   - EMA filter prevents sudden spikes
   - 20% weight to new prices by default

4. **Authorization:**
   - All configuration functions are admin-only
   - Metrics updates require admin authorization

### Error Handling

**Error Codes (1300-1399):**

- `1300` - OracleDataStale: Oracle data exceeds staleness threshold
- `1301` - OracleDataInvalid: Oracle data fails validation
- `1302` - PriceChangeExceedsLimit: Change exceeds maximum allowed
- `1303` - UpdateTooSoon: Minimum interval not met
- `1304` - InvalidDynamicPricingConfig: Invalid configuration parameters
- `1305` - PricingCalculationOverflow: Arithmetic overflow in calculations
- `1306` - OracleNotConfigured: Oracle not configured
- `1307` - OracleCallFailed: Oracle data retrieval failed
- `1308` - DynamicPricingNotEnabled: Operation attempted while disabled

## Events

### PriceUpdateEvent

Emitted when a price update occurs.

```rust
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
```

**Topic:** `PriceUpd`

### DynamicPricingConfigUpdated

Emitted when configuration is updated.

**Topic:** `DynPricCfg`

## Usage Example

### 1. Initialize Dynamic Pricing

```rust
// Configure dynamic pricing
escrow_client.configure_dynamic_pricing(
    &env,
    true,                    // enabled
    100,                     // base_fee_bps (1%)
    1000,                    // max_fee_bps (10%)
    10,                      // min_fee_bps (0.1%)
    500,                     // max_change_bps (5%)
    2000,                    // smoothing_alpha_bps (20%)
    3600,                    // min_update_interval (1 hour)
    None,                    // oracle_address
    true,                    // use_demand_pricing
    true,                    // use_supply_pricing
    true,                    // use_time_decay
);
```

### 2. Update Metrics

```rust
// Update demand metrics
escrow_client.update_demand_metrics(
    &env,
    1000,                    // tx_count
    100_000_000,             // total_volume
    500,                     // unique_users
    100_000,                 // avg_tx_size
    1500,                    // growth_rate_bps (15%)
);

// Update supply metrics
escrow_client.update_supply_metrics(
    &env,
    1_000_000_000,           // total_liquidity
    6000,                    // utilization_bps (60%)
    400_000_000,             // available_liquidity
    600_000_000,             // locked_liquidity
);
```

### 3. Update Oracle Data

```rust
// Update oracle data
escrow_client.update_oracle_data(
    &env,
    1_00_000_000,            // token_price
    10_000_000_000,          // volume_24h
    100_000_000_000,         // market_cap
    2000,                    // volatility_bps (20%)
    current_timestamp,       // timestamp
    Some(signature_bytes),   // signature
);
```

### 4. Trigger Price Update

```rust
// Calculate and apply new price
escrow_client.update_dynamic_price(&env);
```

### 5. Get Current Fee

```rust
// Get current dynamic fee
if let Some(fee_bps) = escrow_client.get_dynamic_fee(&env) {
    let fee_percentage = fee_bps as f64 / 100.0;
    println!("Current fee: {}%", fee_percentage);
}
```

## Integration with Existing Fee System

The dynamic pricing system is designed to work alongside the existing fee mechanism:

1. **Fallback:** If dynamic pricing is disabled, the system falls back to static fee configuration
2. **Override:** Dynamic fees can override static fees when enabled
3. **Compatibility:** Existing fee collection logic remains unchanged
4. **Gradual Rollout:** Can be enabled/disabled without disrupting operations

## Best Practices

### 1. Configuration

- Start with conservative settings (low max_change_bps, high min_update_interval)
- Monitor price adjustments before increasing sensitivity
- Set appropriate min/max fee bounds based on your use case
- Test oracle integration thoroughly before production use

### 2. Metrics Updates

- Update metrics regularly (hourly or daily)
- Use reliable data sources for oracle data
- Validate metrics before submission
- Keep historical data for analysis

### 3. Price Updates

- Trigger updates at regular intervals
- Monitor price change events
- Adjust configuration if prices are too volatile
- Consider automated scheduling for updates

### 4. Security

- Keep oracle signatures secure
- Use trusted oracle providers
- Monitor for unusual price movements
- Have emergency disable procedures ready

## Monitoring and Analytics

### Key Metrics to Monitor

1. **Fee Volatility:** Track frequency and magnitude of fee changes
2. **Demand Patterns:** Analyze transaction volume trends
3. **Supply Utilization:** Monitor liquidity usage
4. **Oracle Health:** Track oracle data freshness and validity
5. **Update Frequency:** Monitor how often prices are updated

### Alert Thresholds

- Price change > 3% in single update
- Oracle data staleness > 1 hour
- Utilization > 80%
- Fee rate > max_fee_bps or < min_fee_bps

## Future Enhancements

### Potential Improvements

1. **Machine Learning Integration:**
   - Predictive pricing models
   - Anomaly detection
   - Automated parameter tuning

2. **AMM Integration:**
   - Liquidity pool-based pricing
   - Automated market maker integration
   - Dynamic liquidity provision

3. **Multi-Oracle Support:**
   - Aggregate data from multiple oracles
   - Consensus mechanisms
   - Fault tolerance

4. **Advanced Smoothing:**
   - Kalman filters
   - Adaptive smoothing parameters
   - Volatility-based adjustment

5. **Time-Based Adjustments:**
   - Peak/off-peak pricing
   - Seasonal adjustments
   - Event-based pricing

## Troubleshooting

### Common Issues

**Issue:** Prices not updating
- **Solution:** Check if dynamic pricing is enabled and minimum interval has passed

**Issue:** Oracle data rejected
- **Solution:** Verify data freshness and signature validity

**Issue:** Price changes too volatile
- **Solution:** Increase smoothing alpha or decrease max_change_bps

**Issue:** Fees too high/low
- **Solution:** Adjust base_fee_bps, min_fee_bps, and max_fee_bps

## References

- Issue #265: Implement Dynamic Pricing Mechanism
- Stellar Soroban Documentation
- AMM Pricing Mechanisms
- Oracle Best Practices
