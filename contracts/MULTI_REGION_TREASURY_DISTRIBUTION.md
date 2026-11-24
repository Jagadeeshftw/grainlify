# Multi-Region Treasury Distribution

## Overview

This document describes the Multi-Region Treasury Distribution feature (Issue #597) that enables automatic or configured distribution of treasury funds across multiple treasury addresses, organized by region or business unit, while maintaining transparent and auditable accounting.

## Feature Description

### Problem Statement

The original implementation used a single `fee_recipient` address for all fee collection. Organizations with regional or business unit segmentation requirements needed a way to automatically distribute fees to multiple treasury destinations based on configurable weights.

### Solution

The feature introduces:

1. **TreasuryDestination** - A new data structure containing:
   - `address`: The destination wallet address
   - `weight`: A u32 value representing the proportion (weights are relative, e.g., 5000 = 50% of total)
   - `region`: A string identifier for the region (e.g., "north_america", "europe", "asia_pacific")

2. **Extended FeeConfig** - The existing FeeConfig now includes:
   - `treasury_destinations`: Vec<TreasuryDestination>
   - `distribution_enabled`: bool flag to enable/disable multi-region distribution

3. **New Contract Functions**:
   - `set_treasury_distributions(destinations: Vec<TreasuryDestination>, enabled: bool)` - Admin function to configure treasury destinations
   - `get_treasury_distributions() -> (Vec<TreasuryDestination>, bool)` - View function to retrieve current configuration
   - Internal `distribute_treasury_fees()` - Handles the fee distribution logic

4. **New Events**:
   - `TreasuryDistributionUpdated` - Emitted when treasury configuration changes
   - `TreasuryDistribution` - Emitted for each fee distribution with full details

## Accounting Expectations

### Fee Calculation

Fees are calculated using the existing `token_math::calculate_fee` function which uses floor rounding:

```
fee = floor(amount * fee_rate / BASIS_POINTS)
```

Where:
- `BASIS_POINTS = 10,000` (1 bp = 0.01%)
- `MAX_FEE_RATE = 5,000` (50% maximum)

### Distribution Logic

When `distribution_enabled` is true:

1. **Lock Fees**: Deducted from depositor's transfer, distributed to treasury destinations
2. **Release Fees**: Deducted from release amount, distributed to treasury destinations
3. **Weight Calculation**: Each non-final destination receives `floor(fee_amount * destination_weight / total_weight)` via integer division.
4. **Rounding Remainder Rule**: To preserve exact fee conservation (`sum(all_destination_amounts) == fee_amount`), any rounding remainder from integer division is deterministically assigned to the **last destination** in the configured `treasury_destinations` order (`last_share = fee_amount - distributed_so_far`). For example, distributing a fee of `100` across three equal-weight destinations `[1, 1, 1]` yields `33`, `33`, and `34`.

### Backward Compatibility

The feature maintains backward compatibility:

- When `distribution_enabled = false` (default): Falls back to legacy single-recipient mode
- When `treasury_destinations` is empty or `distribution_enabled = false`: Fees go to `fee_recipient`
- Existing integrations continue to work without modification

### Invariants

1. **Total Conservation**: `fee = sum(all_destination_amounts)` - All collected fees are distributed
2. **Proportional Distribution**: Each destination receives amount proportional to its weight
3. **No Partial Distribution**: Either all fees are distributed or none (atomic operation)
4. **Audit Trail**: Every distribution emits an event with full details

## Usage Examples

### Example 1: Three-Region Distribution

```rust
// Configure 50% NA, 30% EU, 20% APAC
let mut destinations = Vec::new(&env);
destinations.push(TreasuryDestination {
    address: na_treasury,
    weight: 5000,
    region: String::from_str(&env, "north_america"),
});
destinations.push(TreasuryDestination {
    address: eu_treasury,
    weight: 3000,
    region: String::from_str(&env, "europe"),
});
destinations.push(TreasuryDestination {
    address: apac_treasury,
    weight: 2000,
    region: String::from_str(&env, "asia_pacific"),
});

client.set_treasury_distributions(&destinations, &true);
client.update_fee_config(&Some(1000), &Some(500), &None, &Some(true));
```

With a 1000 XLM lock:
- Lock fee: 100 XLM (10%)
- NA receives: 50 XLM (100 * 5000/10000)
- EU receives: 30 XLM (100 * 3000/10000)
- APAC receives: 20 XLM (100 * 2000/10000)

### Example 2: Single Treasury (100%)

```rust
// Configure single treasury with 100% weight
let mut destinations = Vec::new(&env);
destinations.push(TreasuryDestination {
    address: global_treasury,
    weight: 10000,  // 100%
    region: String::from_str(&env, "global"),
});

client.set_treasury_distributions(&destinations, &true);
```

### Example 3: Disable Distribution (Legacy Mode)

```rust
// Disable distribution to use single fee_recipient
client.set_treasury_distributions(&destinations, &false);
// Or simply don't call set_treasury_distributions at all
```

## Events

### TreasuryDistributionUpdated

Emitted when treasury configuration is updated:
```rust
TreasuryDistributionUpdated {
    destinations_count: u32,
    total_weight: u32,
    distribution_enabled: bool,
    timestamp: u64,
}
```

### TreasuryDistribution

Emitted for each fee distribution:
```rust
TreasuryDistribution {
    version: u32,
    operation_type: FeeOperationType, // Lock or Release
    total_amount: i128,
    distributions: Vec<TreasuryDistributionDetail>,
    timestamp: u64,
}

TreasuryDistributionDetail {
    destination_address: Address,
    region: String,
    amount: i128,
    weight: u32,
}
```

## Security Considerations

1. **Admin Only**: Only the contract admin can configure treasury destinations
2. **Weight Validation**: Total weight must be > 0 when distribution is enabled
3. **Destination Validation**: At least one destination required when distribution is enabled
4. **Reentrancy Protection**: Distribution uses the existing reentrancy guard

## Testing

See `test_multi_region_treasury.rs` for comprehensive test coverage including:

- Configuration of treasury destinations
- Proportional fee distribution
- Backward compatibility with single recipient
- Edge cases (zero weight, empty destinations)
- Lock and release fee distribution

## Migration Guide

### For Existing Deployments

1. **Automatic Backward Compatibility**: Existing deployments continue to work
2. **Opt-in Migration**: Call `set_treasury_distributions()` to enable multi-region distribution
3. **Gradual Rollout**: Start with single destination (100% weight), then add more regions

### Configuration Checklist

- [ ] Deploy contract upgrade
- [ ] Configure treasury destinations using `set_treasury_distributions()`
- [ ] Enable distribution with `distribution_enabled = true`
- [ ] Verify fee collection in event logs
- [ ] Update off-chain accounting systems to listen for `TreasuryDistribution` events
