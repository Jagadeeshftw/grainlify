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

### Solvency Invariant Specification

The insurance reserve maintains strict balance solvency connecting reserve debits, credits, and failed payouts:

$$R_t = R_0 + \sum_{i=1}^t \text{Credits}_i - \sum_{j=1}^t \text{Debits}_j \ge 0 \quad \forall t \ge 0$$

Where:
- **$\text{Credits}_i$**: Segregated carve-out shares collected from protocol fees during `lock_program_funds`, `single_payout`, and `batch_payout`.
- **$\text{Debits}_j$**: Authorized administrative withdrawals (`withdraw_insurance_reserve`) or potential claims coverage.
- **Failed Payouts & Aborted Transactions**: Soroban transactions execute atomically. Any failed operation (fee shortfall, underfunded debit, invalid parameter, or transfer failure) results in immediate transaction rollback:
  $$\Delta R = 0$$
  No partial state writes or phantom debits/credits are ever persisted.

### Design Decision: Prohibition of Transient Negative Balances

- **Decision**: The reserve balance is **strictly non-negative** ($R_t \ge 0$) at all times. It may **NEVER** be temporarily negative during any intermediate execution step of a transaction.
- **Rationale**: Transient negative balances introduce critical solvency risks, reentrancy vulnerabilities, off-chain ledger reconciliation errors, and unrecoverable states if downstream contract invocations fail.
- **Enforcement**: Every debit operation enforces a strict pre-condition:
  $$\text{amount} \le \text{balance\_before}$$
  If an operation attempts to debit more than the available reserve, it fails safely and panics immediately with `ContractError::InsufficientInsuranceReserve` (error code 705) prior to any state modification or token transfer.

### State Transitions Across Execution Paths

| Path | Trigger Operation | Reserve Impact ($\Delta R$) | Preconditions | Postconditions & Event Assertions |
|------|-------------------|-----------------------------|---------------|-----------------------------------|
| **Normal Payout** | `single_payout` / `batch_payout` | $+ \text{reserve\_share}$ | `fee_enabled == true`, `bps > 0` | Storage updated: $R_{t+1} = R_t + \text{reserve\_share}$. `FeeCollectedEvent` emitted. Conservation holds (`net + fee_recipient + reserve == gross`). |
| **Fee Shortfall / Underfunded** | Over-withdrawal or 0 fee payout | $0$ | Debit $> R_t$ or fee is 0 | Underfunded debits fail safely with `InsufficientInsuranceReserve` (705). Storage is strictly unchanged ($R_{after} = R_{before}$). Zero withdrawal events emitted. |
| **Refund Path** | `cancel_claim` (refund path) | $0$ | Pending claim exists | Escrow remaining balance restored; reserve storage strictly intact ($R_{t+1} = R_t$). Emits `ClmCncl`; zero reserve withdrawal events. |
| **Cancellation Path** | Claim / program cancellation | $0$ | Valid cancellation state | Unreleased funds returned; reserve storage strictly untouched ($R_{t+1} = R_t$). Zero reserve events. |
| **Repeated Failure** | Multiple sequential failing calls | $0$ | Failing preconditions | Solvency preserved across all failures ($R_0 = R_1 = R_2$). No spurious events emitted; subsequent valid operations execute cleanly. |

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
- **Solvency Invariant Regression Suite (Issue #1723)**:
  - Normal payout path ($R_{t+1} = R_t + \text{reserve\_share}$, storage and event assertions)
  - Fee shortfall & underfunded safe failure (`InsufficientInsuranceReserve` 705, zero storage modification)
  - Refund path (`cancel_claim` refund preserves reserve storage and events)
  - Cancellation path (claim cancellation cycles maintain invariant $R_t \ge 0$)
  - Repeated failure (sequential failures leave storage untouched; subsequent valid withdrawal succeeds)
  - Prohibition of transient negative balances ($R_t \ge 0$ strictly enforced at every step)
