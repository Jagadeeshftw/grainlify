# Program Escrow Insurance Reserve

## Overview

The Program Escrow Contract now supports an **insurance reserve** feature that allows programs to carve out a configurable portion of collected fees into a segregated on-chain reserve balance. This provides a lightweight self-insurance buffer without requiring a separate claims contract.

## Architecture

### Fee Configuration

The `FeeConfig` struct includes a new field:

```rust
/// Basis-point share of each collected fee that is carved out into the
/// on-chain insurance reserve instead of being forwarded to `fee_recipient`.
///
/// Range: `0` (disabled, default) – `MAX_FEE_RATE` (1,000 basis points = 10%).
pub insurance_reserve_bps: u32,
```

### Fee Splitting Logic

When fees are collected, they are split according to this formula:

```
total_fee = combined_fee_amount(gross, rate, fixed, enabled)
reserve_share = ceil(total_fee * insurance_reserve_bps / BASIS_POINTS)
recipient_share = total_fee - reserve_share
```

**Invariant**: `reserve_share + recipient_share == total_fee` (no value leakage or double-counting)

### Storage

The insurance reserve balance is stored separately from the program's `remaining_balance` using:

- **Storage Key**: `DataKey::InsuranceReserve`
- **Storage Type**: Instance storage (shares contract TTL)
- **Data Type**: `i128` (native token units)

## API Functions

### View Functions

#### `get_insurance_reserve_balance(env: Env) -> i128`

Returns the current insurance reserve balance in native token units.

- **Authorization**: None (read-only)
- **Returns**: Current reserve balance, defaults to 0 if not initialized

#### `get_fee_config(env: Env) -> FeeConfig`

Returns the current fee configuration including `insurance_reserve_bps`.

- **Authorization**: None (read-only)
- **Returns**: Complete fee configuration

### Admin Functions

#### `update_fee_config(..., insurance_reserve_bps: Option<u32>)`

Updates the fee configuration. The `insurance_reserve_bps` parameter is validated:

- **Range**: 0 to `MAX_FEE_RATE` (1,000 basis points = 10%)
- **Authorization**: Admin only
- **Validation**: Must not exceed `MAX_FEE_RATE`
- **Error**: `InvalidInsuranceReserveBps` (704) if validation fails

#### `withdraw_insurance_reserve(env: Env, target: Address, amount: i128)`

Withdraws funds from the insurance reserve.

- **Authorization**: Admin only (same level as `emergency_withdraw`)
- **Parameters**:
  - `target`: Destination address
  - `amount`: Amount to withdraw (must be > 0)
- **Validation**:
  - Amount must be positive
  - Amount must not exceed current reserve balance
- **Effects**:
  - Decrements reserve balance
  - Transfers tokens to target address
  - Emits `InsuranceReserveWithdrawnEvent`
- **Errors**:
  - `InvalidAmount` if amount ≤ 0
  - `InsufficientInsuranceReserve` (705) if amount exceeds balance

## Integration Points

The insurance reserve is automatically integrated into all fee collection operations:

1. **Program Lock Fees**: Applied during `init_program` with initial liquidity
2. **Single Payout Fees**: Applied during `single_payout` operations  
3. **Batch Payout Fees**: Applied during `batch_payout` operations

### Zero-Impact Behavior

When `insurance_reserve_bps == 0`:
- Full fee amount goes to `fee_recipient`
- Reserve balance remains unchanged
- No performance overhead

## Events

### InsuranceReserveWithdrawnEvent

Emitted when funds are withdrawn from the reserve:

```rust
pub struct InsuranceReserveWithdrawnEvent {
    pub version: u32,
    pub admin: Address,           // Admin who authorized withdrawal
    pub target: Address,          // Destination address
    pub amount: i128,             // Amount withdrawn
    pub balance_before: i128,     // Reserve balance before withdrawal
    pub balance_after: i128,      // Reserve balance after withdrawal
    pub timestamp: u64,           // Ledger timestamp
}
```

## Security Considerations

### Arithmetic Safety

- **Overflow Protection**: All arithmetic operations use `checked_add()` and `checked_mul()`
- **Ceiling Division**: Reserve calculation uses ceiling division to prevent value leakage
- **Invariant Enforcement**: `reserve_share + recipient_share == total_fee` is guaranteed

### Access Control

- **Admin Authorization**: Only admin can update configuration or withdraw funds
- **Rate Limits**: `insurance_reserve_bps` cannot exceed `MAX_FEE_RATE` (10%)
- **Validation**: All inputs validated before state changes

### Audit Trail

- **Withdrawal Events**: All reserve withdrawals emit audit events
- **Fee Events**: Standard fee collection events include reserve information
- **Balance Tracking**: Reserve balance tracked independently from program funds

## Error Codes

| Code | Error | Description |
|------|-------|-------------|
| 704  | `InvalidInsuranceReserveBps` | Reserve rate exceeds `MAX_FEE_RATE` |
| 705  | `InsufficientInsuranceReserve` | Withdrawal amount exceeds reserve balance |

## Usage Examples

### Configure Reserve Rate

```rust
// Set 5% (500 basis points) reserve rate
client.update_fee_config(
    &None,                    // lock_fee_rate unchanged
    &None,                    // payout_fee_rate unchanged  
    &None,                    // lock_fixed_fee unchanged
    &None,                    // payout_fixed_fee unchanged
    &None,                    // fee_recipient unchanged
    &None,                    // fee_enabled unchanged
    &Some(500_u32),           // insurance_reserve_bps = 5%
);
```

### Query Reserve Balance

```rust
let balance = client.get_insurance_reserve_balance();
println!("Insurance reserve: {} tokens", balance);
```

### Withdraw Reserve Funds

```rust
// Admin withdraws 1000 tokens from reserve
client.withdraw_insurance_reserve(
    &target_address,
    &1000_i128,
);
```

## Testing

Comprehensive test coverage is provided in `contracts/program-escrow/src/test_insurance_reserve.rs`:

- Field initialization and persistence
- Split invariant verification across all fee paths
- Accumulation accuracy across multiple operations
- Query function correctness
- Withdrawal authorization and validation
- Zero-basis-point passthrough behavior
- Edge cases and error conditions
