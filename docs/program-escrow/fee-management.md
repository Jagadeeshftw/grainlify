# Fee Management in Program Escrow

## Overview

The Program Escrow contract supports configurable fee deduction on both lock and payout operations. Fees are collected to sustain the platform and are sent to a designated fee recipient address.

## Fee Configuration

Fee parameters are managed through the [`FeeConfig`](contracts/program-escrow/src/lib.rs#L256) struct stored under the `FEE_CONFIG` storage key.

| Field | Type | Description |
|-------|------|-------------|
| `lock_fee_rate` | `i128` | Percentage fee on lock operations (basis points, max `MAX_FEE_RATE`) |
| `payout_fee_rate` | `i128` | Percentage fee on each payout (basis points, max `MAX_FEE_RATE`) |
| `lock_fixed_fee` | `i128` | Flat fee on lock (token base units), capped to lock amount |
| `payout_fixed_fee` | `i128` | Flat fee per payout (token base units), capped to gross payout |
| `fee_recipient` | `Address` | Address that receives collected fees |
| `fee_enabled` | `bool` | Global on/off switch for fee deduction |
| `fee_waivers` | `u32` | Bitmask for per-payout-type waivers (see [fee-arithmetic.md](fee-arithmetic.md)) |

## Maximum Fee Rate Cap

### Constant

`MAX_FEE_RATE = 1000` (defined in `lib.rs:197`)

This corresponds to **10%** in basis points (1000 bps). No `lock_fee_rate` or `payout_fee_rate` may exceed this value.

### Derivation

Soroban contracts operate within a per-transaction CPU instruction budget of 100 M instructions. Empirical benchmarking shows that fee-calculation overhead is negligible compared to token transfers and storage writes, so the cap is set conservatively at 10 % to prevent economic attacks while remaining flexible for most use cases.

### Enforcement

The cap is enforced in [`update_fee_config`](contracts/program-escrow/src/lib.rs#L2848), the only public entry point that modifies `FeeConfig` after initialization:

```rust
if r > MAX_FEE_RATE {
    panic_with_error!(&env, ContractError::InvalidFeeRate);
}
```

> **Audit note**: Initialization paths (`init_program`, `init_program_with_dependencies`, `batch_init_programs`) set both rates to `0`, which is below `MAX_FEE_RATE` by construction and requires no guard.

## Updating Fees

Call `update_fee_config` with `Option<i128>` parameters — a `None` leaves the current value unchanged:

```rust
client.update_fee_config(
    &Some(500),   // lock_fee_rate: 5%
    &None,        // payout_fee_rate: unchanged
    &None,        // lock_fixed_fee: unchanged
    &None,        // payout_fixed_fee: unchanged
    &None,        // fee_recipient: unchanged
    &Some(true),  // fee_enabled: true
);
```

### Security Properties

1. **Admin-only**: Only the contract admin can call `update_fee_config`.
2. **Atomic update**: All changes are written in a single storage `set` call — partial failure is impossible.
3. **Range validation**: `lock_fee_rate` and `payout_fee_rate` are validated against `MAX_FEE_RATE` (panics with `ContractError::InvalidFeeRate` on violation).
4. **Non-negative fixed fees**: `lock_fixed_fee` and `payout_fixed_fee` must be non-negative.
5. **Preservation**: Fields set to `None` are preserved unchanged.

## Fee Calculation

See [fee-arithmetic.md](fee-arithmetic.md) for the exact rounding policy and implementation details.

## Testing

Comprehensive boundary-value tests are located in [`test_payout_splits.rs::fee_enforcement`](contracts/program-escrow/src/test_payout_splits.rs). The test matrix covers:

| Scenario | Expected Outcome |
|----------|------------------|
| `rate = MAX_FEE_RATE + 1` | Rejected with `ContractError::InvalidFeeRate` |
| `rate = MAX_FEE_RATE` | Accepted |
| `rate = 0` | Accepted |
| `rate < 0` | Rejected |
| Both rates at `MAX_FEE_RATE` | Accepted |
| One rate valid, other invalid | Rejected |
| Fixed fee negative | Rejected |
| Partial update preserves other fields | Preserved |
