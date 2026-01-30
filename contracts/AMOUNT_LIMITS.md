# Configurable Amount Limits

This document describes the configurable amount limits feature added to both the bounty escrow and program escrow contracts.

## Overview

The amount limits feature provides configurable minimum and maximum amount constraints for lock and payout operations. This helps prevent:

- **Dust attacks**: By setting minimum amounts
- **Excessive exposure**: By capping maximum amounts per operation
- **Platform policy enforcement**: By allowing admins to set bounty/program size limits

## Configuration Structure

Both contracts use the same `AmountLimits` struct:

```rust
pub struct AmountLimits {
    pub min_lock_amount: i128,    // Minimum amount for lock operations
    pub max_lock_amount: i128,    // Maximum amount for lock operations  
    pub min_payout: i128,         // Minimum amount for payout operations
    pub max_payout: i128,         // Maximum amount for payout operations
}
```

## Default Values

When contracts are initialized, the following default limits are set:

- `min_lock_amount`: 1 (prevents zero/negative amounts)
- `max_lock_amount`: i128::MAX (no upper limit)
- `min_payout`: 1 (prevents zero/negative amounts)
- `max_payout`: i128::MAX (no upper limit)

## Admin Functions

### Update Amount Limits

**Bounty Escrow:**
```rust
pub fn update_amount_limits(
    env: Env,
    min_lock_amount: i128,
    max_lock_amount: i128,
    min_payout: i128,
    max_payout: i128,
) -> Result<(), Error>
```

**Program Escrow:**
```rust
pub fn update_amount_limits(
    env: Env,
    min_lock_amount: i128,
    max_lock_amount: i128,
    min_payout: i128,
    max_payout: i128,
)
```

**Authorization:** Only the contract admin can call this function.

**Validation:**
- All amounts must be non-negative
- Minimum amounts cannot exceed maximum amounts
- Emits configuration update event

### Get Amount Limits

**Both contracts:**
```rust
pub fn get_amount_limits(env: Env) -> AmountLimits
```

Returns the current amount limits configuration. This is a view function that doesn't require authorization.

## Enforcement Points

### Lock Operations

Amount limits are enforced in:
- `lock_funds()` - Individual bounty funding
- `lock_program_funds()` - Program funding
- `batch_lock_funds()` - Batch bounty funding

The validation checks that the lock amount is within the configured `min_lock_amount` and `max_lock_amount` bounds.

### Payout Operations

Amount limits are enforced in:
- `release_funds()` - Individual bounty release
- `single_payout()` - Individual program payout
- `batch_release_funds()` - Batch bounty release
- `batch_payout()` - Batch program payout

**Important:** For payout operations, the validation is applied to the **net amount** after fees are deducted, not the gross amount.

## Fee Interaction

When fees are enabled, payout limits are applied to the net amount that recipients receive:

```
gross_amount = 1000
fee_amount = 50 (5% fee)
net_amount = 950 (amount checked against payout limits)
```

This ensures that recipients receive amounts within the configured limits, regardless of fee structure.

## Error Handling

### Bounty Escrow
- Returns `Error::InvalidAmount` when limits are violated
- Returns `Error::InvalidAmount` for invalid limit configurations

### Program Escrow
- Panics with descriptive messages when limits are violated
- Panics with "Invalid amount" messages for invalid configurations

## Events

Both contracts emit events when amount limits are updated:

```rust
env.events().publish(
    (symbol_short!("amt_lmt"),),
    (min_lock_amount, max_lock_amount, min_payout, max_payout),
);
```

## Usage Examples

### Setting Conservative Limits
```rust
// Prevent dust attacks and limit exposure
escrow.update_amount_limits(
    &100,      // min_lock: 100 tokens minimum
    &10_000,   // max_lock: 10,000 tokens maximum
    &50,       // min_payout: 50 tokens minimum
    &5_000     // max_payout: 5,000 tokens maximum
);
```

### Setting Dust Prevention Only
```rust
// Only prevent dust, no upper limits
escrow.update_amount_limits(
    &10,           // min_lock: 10 tokens minimum
    &i128::MAX,    // max_lock: no limit
    &5,            // min_payout: 5 tokens minimum
    &i128::MAX     // max_payout: no limit
);
```

### Setting Maximum Exposure Limits
```rust
// Cap maximum amounts for risk management
escrow.update_amount_limits(
    &1,            // min_lock: allow small amounts
    &100_000,      // max_lock: 100,000 tokens maximum
    &1,            // min_payout: allow small payouts
    &50_000        // max_payout: 50,000 tokens maximum
);
```

## Testing

Comprehensive tests are included for:
- Default limit initialization
- Limit configuration updates
- Validation of invalid configurations
- Enforcement in lock operations
- Enforcement in payout operations (considering fees)
- Batch operation enforcement
- Edge cases (exactly at limits, just above/below limits)

## Migration

Existing contracts will use default limits (1 to i128::MAX) until explicitly configured by admins. This ensures backward compatibility while providing the new functionality.

## Security Considerations

1. **Admin Authorization**: Only contract admins can modify limits
2. **Validation**: All limit updates are validated for consistency
3. **Fee Interaction**: Payout limits consider net amounts after fees
4. **Event Logging**: All limit changes are logged for transparency
5. **Atomic Operations**: Batch operations respect limits for all items

## Implementation Notes

- Limits are stored in contract instance storage
- Validation occurs early in functions to fail fast
- Both contracts share the same AmountLimits structure for consistency
- Default values ensure existing functionality continues to work
