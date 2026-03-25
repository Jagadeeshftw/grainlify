# Token Math — `grainlify_core::token_math`

> **Module**: `grainlify_core/src/token_math.rs`  
> **Test file**: `grainlify_core/src/test_token_math.rs` (or `token_math` inline tests)  
> **Feature branch**: relevant to any escrow fee or multi-token work

---

## Table of Contents

1. [Overview](#overview)
2. [Constants](#constants)
3. [Public Interface](#public-interface)
   - [`calculate_fee`](#calculate_fee)
   - [`split_amount`](#split_amount)
   - [`scale_amount`](#scale_amount)
   - [`to_base_units`](#to_base_units)
4. [Rounding Contract](#rounding-contract)
5. [Decimal Model](#decimal-model)
6. [Error and Overflow Behaviour](#error-and-overflow-behaviour)
7. [Security Assumptions](#security-assumptions)
8. [Test Coverage Summary](#test-coverage-summary)
9. [Integration Guide](#integration-guide)

---

## Overview

`token_math` is a **pure arithmetic utility module** — no storage, no events, no SDK environment required. It provides the three numeric primitives that the escrow contract relies on wherever money changes hands:

- **Fee calculation** — how many stroops to withhold at a given basis-point rate.
- **Amount splitting** — decompose a gross amount into `(fee, net)` with the invariant `fee + net == amount` guaranteed.
- **Decimal scaling** — convert a raw integer between two different decimal precisions.
- **Base-unit conversion** — turn a human-readable whole-token count into its on-chain integer representation.

All functions use **floor (truncating) integer division**. This favours the depositor on fractional amounts — no depositor ever pays a rounded-up fee — but it means the fee can be zero when the amount is too small relative to the rate. Callers that require a minimum fee floor must enforce that policy themselves.

---

## Constants

### `BASIS_POINTS`

```rust
pub const BASIS_POINTS: i128 = 10_000;
```

The denominator for all fee rates. One basis point equals 0.01 %. A rate of `500` means 5 %.

### `MAX_FEE_RATE`

```rust
pub const MAX_FEE_RATE: i128 = 5_000;
```

The maximum permitted fee rate: 5 000 basis points = 50 %. No escrow function accepts a rate above this value. It is exported so callers can validate inputs before submitting a transaction.

---

## Public Interface

### `calculate_fee`

```rust
pub fn calculate_fee(amount: i128, fee_rate: i128) -> i128
```

Returns the fee amount for a given gross `amount` at `fee_rate` basis points, using **floor division**.

**Formula**

```
fee = floor(amount × fee_rate / BASIS_POINTS)
    = (amount × fee_rate) / 10_000          // integer division truncates
```

**Parameters**

| Parameter | Type | Description |
|-----------|------|-------------|
| `amount` | `i128` | Gross amount in the token's smallest unit (stroops, micro-units, etc.). |
| `fee_rate` | `i128` | Rate in basis points. `0` = no fee, `10_000` = 100 %. |

**Returns** `i128` — the fee in the same unit as `amount`. Always `≥ 0` and always `≤ amount`.

**Short-circuit rules**

- If `fee_rate == 0` → returns `0` immediately.
- If `amount == 0` → returns `0` immediately.

**Examples**

```
calculate_fee(10_000, 500)  = 500     // 5 % of 10 000
calculate_fee(999,    100)  = 9       // 9.99 → floor = 9
calculate_fee(1,      100)  = 0       // 0.01 → floor = 0 (sub-unit)
calculate_fee(10_000, 1)    = 1       // 0.01 % of 10 000
```

---

### `split_amount`

```rust
pub fn split_amount(amount: i128, fee_rate: i128) -> (i128, i128)
```

Splits `amount` into `(fee, net)` such that `fee + net == amount` is always exactly true.

**Parameters**

| Parameter | Type | Description |
|-----------|------|-------------|
| `amount` | `i128` | Gross amount to split. |
| `fee_rate` | `i128` | Rate in basis points. |

**Returns** `(fee: i128, net: i128)` — a tuple where `fee` is computed via `calculate_fee` and `net` is the remainder: `amount - fee`. Neither value is ever negative.

**Invariant (formally)**

```
∀ amount ≥ 0, ∀ fee_rate ∈ [0, MAX_FEE_RATE]:
    let (fee, net) = split_amount(amount, fee_rate)
    fee + net == amount
    fee ≥ 0
    net ≥ 0
```

The invariant holds even when floor division introduces a remainder, because `net` absorbs all fractional amounts that `calculate_fee` truncates away. This means the depositor always retains any unrepresentable fractional fee — there is no stroop "lost" in the split.

**Examples**

```
split_amount(10_000, 500)  = (500,  9_500)   // 500 + 9_500 == 10_000 ✓
split_amount(999,    100)  = (9,    990)     // 9   + 990   == 999    ✓
split_amount(5_000,  0)    = (0,    5_000)   // 0   + 5_000 == 5_000  ✓
split_amount(1_001,  5000) = (500,  501)     // 500 + 501   == 1_001  ✓
```

---

### `scale_amount`

```rust
pub fn scale_amount(amount: i128, from_decimals: u32, to_decimals: u32) -> Option<i128>
```

Converts `amount` from one decimal precision to another.

**Parameters**

| Parameter | Type | Description |
|-----------|------|-------------|
| `amount` | `i128` | Raw integer amount in units of `from_decimals` precision. |
| `from_decimals` | `u32` | Number of decimal places the input uses (e.g. `7` for XLM, `6` for USDC). |
| `to_decimals` | `u32` | Number of decimal places the output should use. |

**Returns** `Option<i128>`:

- `Some(scaled)` on success.
- `None` on arithmetic overflow (when scaling up by a large factor would exceed `i128::MAX`).

**Behaviour**

- When `from_decimals == to_decimals`: returns `Some(amount)` unchanged.
- When `from_decimals < to_decimals` (scaling up): multiplies by `10^(to - from)`. Can overflow for very large amounts — returns `None` in that case.
- When `from_decimals > to_decimals` (scaling down): divides by `10^(from - to)` using **floor division**. Sub-unit amounts that do not survive the scaling become `0`.

**Examples**

```
scale_amount(1_000_000,  6, 7) = Some(10_000_000)  // USDC → XLM precision
scale_amount(10_000_005, 7, 6) = Some(1_000_000)   // XLM → USDC precision (floor)
scale_amount(19,         7, 6) = Some(1)            // 1.9 → floor = 1
scale_amount(9,          7, 6) = Some(0)            // 0.9 → floor = 0
scale_amount(1,          0, 7) = Some(10_000_000)   // 1 whole token → stroops
scale_amount(12345,      7, 7) = Some(12345)        // identity
```

---

### `to_base_units`

```rust
pub fn to_base_units(whole_tokens: i128, decimals: u32) -> Option<i128>
```

Converts a whole-token count into its on-chain integer representation by multiplying by `10^decimals`.

This is a convenience wrapper around `scale_amount(whole_tokens, 0, decimals)`.

**Parameters**

| Parameter | Type | Description |
|-----------|------|-------------|
| `whole_tokens` | `i128` | Number of whole tokens (e.g. `100` for 100 XLM). |
| `decimals` | `u32` | Token precision (e.g. `7` for XLM, `6` for USDC). |

**Returns** `Option<i128>`:

- `Some(base_units)` on success.
- `None` on overflow.

**Examples**

```
to_base_units(100, 7) = Some(1_000_000_000)  // 100 XLM in stroops
to_base_units(50,  6) = Some(50_000_000)     // 50 USDC
to_base_units(42,  0) = Some(42)             // integer token, no scaling
to_base_units(0,   7) = Some(0)
```

---

## Rounding Contract

Every function in this module uses **floor (truncating) integer division**. The consequences are:

| Situation | Result |
|-----------|--------|
| Fee on a dust amount (e.g. 1 stroop at 1 bp) | Fee = 0; full amount goes to depositor |
| `scale_amount` losing sub-unit precision on scale-down | Truncated; caller receives the floor |
| `split_amount` with a fractional fee | Remainder stays in `net`, not lost |

**Why floor and not ceiling?**

The escrow contract's single-item `lock_funds` path uses **ceiling** division to ensure the fee is never zero when `fee_rate > 0` (closing the dust-splitting principal-drain vector). The `token_math` module intentionally uses floor division as its primitive, and the higher-level contract layer applies ceiling where required. This separation keeps the math module predictable and composable — callers opt in to ceiling behaviour by adjusting the formula at the call site, rather than having the module impose a rounding policy.

Do not assume that `token_math::calculate_fee` is interchangeable with the in-contract `BountyEscrowContract::calculate_fee`. They differ in rounding direction. Always use the contract-level function for escrow fee deductions.

---

## Decimal Model

Stellar tokens carry a decimal precision that is intrinsic to the token contract, not encoded in the integer value itself. `token_math` operates on raw integers and treats decimal precision as an input parameter. The caller is responsible for passing the correct precision for the token in question.

Common precisions:

| Token | Decimals | 1 whole token |
|-------|----------|---------------|
| XLM (Lumens) | 7 | 10 000 000 stroops |
| USDC (Circle) | 6 | 1 000 000 micro-USDC |
| Generic 2-decimal | 2 | 100 base units |

When comparing or combining amounts across different tokens, always scale to a common precision first using `scale_amount`. Mixing raw integers of different precisions without scaling will silently produce incorrect results — the module has no way to detect mismatched precisions.

---

## Error and Overflow Behaviour

`calculate_fee` and `split_amount` do not return `Option` or `Result`. They are designed for amounts and rates that fit within `i128`. The practical upper bound on Stellar is the total XLM supply in stroops (`~500 billion XLM × 10^7 = ~5 × 10^18`), which is well within `i128::MAX` (~1.7 × 10^38). Overflow in these functions is not possible under any realistic Stellar token supply.

`scale_amount` and `to_base_units` return `Option<i128>` specifically for the scale-up path, where multiplying by a large power of 10 could in principle overflow `i128`. Callers must handle `None` — treating it as an invalid input or returning an error to the user.

---

## Security Assumptions

### 1. Floor rounding favours the depositor, not the protocol

`calculate_fee` floors the result. At sub-unit amounts the fee is zero. A caller who relies on fee revenue for protocol sustainability should be aware that a user can lock a single-stroop amount and pay no fee. The contract-level `BountyEscrowContract::calculate_fee` uses ceiling division to close this gap for the primary lock path; but any secondary use of `token_math::calculate_fee` directly inherits the floor behaviour.

### 2. `split_amount` never loses a stroop

The `fee + net == amount` invariant is structural — `net` is computed as `amount - fee`, not by a separate formula. No arithmetic path exists that loses value. This is verifiable by inspection and covered by the property-based boundary tests.

### 3. No negative outputs

Both `fee` and `net` from `split_amount` are always non-negative given non-negative inputs. If `amount < 0` is passed, the results will be mathematically consistent but semantically meaningless. The escrow layer validates `amount > 0` before calling these utilities; the module itself does not re-validate.

### 4. Decimal mismatch is a silent logic error

`scale_amount` has no way to verify that `from_decimals` matches the token's actual precision. Passing the wrong `from_decimals` produces a wrong-but-successful result. Token decimal lookup and validation is the caller's responsibility.

### 5. `MAX_FEE_RATE` is advisory at this layer

The module exports `MAX_FEE_RATE` as a constant but does not enforce it inside `calculate_fee` or `split_amount`. Passing `fee_rate > MAX_FEE_RATE` will compute a mathematically correct but economically invalid fee (more than 50 % of the principal). Enforcement happens in `update_fee_config` and `set_token_fee_config` at the contract level.

---

## Test Coverage Summary

The test suite covers the following scenarios across six groups:

### Group 1 — `calculate_fee` basic behaviour

| Test | Scenario |
|------|----------|
| `fee_zero_rate_returns_zero` | Rate = 0 → fee = 0 regardless of amount |
| `fee_zero_amount_returns_zero` | Amount = 0 → fee = 0 regardless of rate |
| `fee_exact_division` | Amount and rate produce no remainder |
| `fee_floor_rounds_down` | Fractional result truncates toward zero |
| `fee_one_basis_point` | Minimum meaningful rate (1 bp) |
| `fee_max_rate` | Rate = `MAX_FEE_RATE` (50 %) |
| `fee_single_unit_amount` | Smallest possible amount (1) with non-zero rate floors to 0 |

### Group 2 — `calculate_fee` decimal scenarios

| Test | Scenario |
|------|----------|
| `fee_xlm_7_decimals` | 100 XLM (stroops) at 2 % = 2 XLM |
| `fee_usdc_6_decimals` | 100 USDC at 2 % = 2 USDC |
| `fee_low_decimal_token_2_decimals` | 2-decimal token at 3 % |
| `fee_small_amount_high_decimals_floors_correctly` | 1 stroop at 1 % floors to 0 |

### Group 3 — `split_amount` invariant

| Test | Scenario |
|------|----------|
| `split_invariant_exact` | Clean split with no remainder |
| `split_invariant_with_remainder` | Fractional fee — remainder stays in `net` |
| `split_invariant_zero_fee` | Rate = 0 → all goes to net |
| `split_invariant_max_rate` | Rate = `MAX_FEE_RATE` with odd amount |
| `split_invariant_prime_amount` | Prime amount to stress-test remainder handling |
| `split_invariant_large_amount` | 1 billion XLM in stroops |

### Group 4 — `scale_amount`

| Test | Scenario |
|------|----------|
| `scale_same_decimals_is_identity` | 7 → 7, no change |
| `scale_up_6_to_7` | USDC precision to XLM precision |
| `scale_down_7_to_6` | XLM precision to USDC, floors sub-unit |
| `scale_down_floors` | 1.9 → 1 |
| `scale_down_sub_unit_floors_to_zero` | 0.9 → 0 |
| `scale_large_gap` | 0-decimal to 7-decimal |
| `scale_zero_amount` | 0 input always yields 0 |

### Group 5 — `to_base_units`

| Test | Scenario |
|------|----------|
| `to_base_units_xlm` | 100 XLM → stroops |
| `to_base_units_usdc` | 50 USDC → micro-USDC |
| `to_base_units_zero_decimals` | Integer token (no scaling) |
| `to_base_units_zero_amount` | 0 input |

### Group 6 — Boundary and property tests

| Test | Scenario |
|------|----------|
| `fee_never_exceeds_amount` | Property: fee ≤ amount for a range of amounts at `MAX_FEE_RATE` |
| `split_net_never_negative` | Property: both `fee` and `net` are ≥ 0 for a range |
| `fee_monotonic_with_amount` | Property: fee is non-decreasing as amount increases at fixed rate |

### Coverage gaps to consider

| Suggested test | Rationale |
|----------------|-----------|
| `scale_amount_overflow_returns_none` | The `None` path of `scale_amount` is not yet covered; verify that a very large amount scaled up by a large factor returns `None` rather than panicking |
| `to_base_units_overflow_returns_none` | Same gap for `to_base_units` |
| `fee_monotonic_with_rate` | Mirror of `fee_monotonic_with_amount` — fee should be non-decreasing as rate increases at fixed amount |
| `split_invariant_single_stroop` | `split_amount(1, MAX_FEE_RATE)` — verifies `(0, 1)` and not `(-1, 2)` or similar |
| `calculate_fee_large_amount_high_rate` | Stress test near `i128::MAX / MAX_FEE_RATE` to confirm no overflow in the intermediate multiply |

---

## Integration Guide

### Using `calculate_fee` for display purposes

```rust
use grainlify_core::token_math;

let gross_amount: i128 = 1_000_0000000; // 1000 XLM in stroops
let fee_rate: i128 = 250;               // 2.5 %
let fee = token_math::calculate_fee(gross_amount, fee_rate);
let net = gross_amount - fee;

// fee = 25_0000000 (25 XLM)
// net = 975_0000000 (975 XLM)
```

### Using `split_amount` for atomic deductions

Prefer `split_amount` over calling `calculate_fee` and subtracting manually. It guarantees the invariant and removes the risk of an off-by-one from a separate subtraction:

```rust
let (fee, net) = token_math::split_amount(gross_amount, fee_rate);
// fee + net is guaranteed to equal gross_amount
token_client.transfer(&depositor, &contract, &gross_amount);
token_client.transfer(&contract, &fee_recipient, &fee);
token_client.transfer(&contract, &contributor, &net);
```

### Converting between token precisions

When an escrow holds XLM and a fee report is needed in USDC display units:

```rust
let xlm_stroops: i128 = 1_000_000_000; // 100 XLM
let in_usdc_units = token_math::scale_amount(xlm_stroops, 7, 6);
// Some(100_000_000) — 100 USDC units (assuming 1:1 peg, display only)
```

### Accepting user input in whole tokens

When an off-chain UI takes a human-readable amount like "50 USDC" and needs to pass it to the contract:

```rust
let whole_usdc: i128 = 50;
let base_units = token_math::to_base_units(whole_usdc, 6)
    .ok_or("amount overflow")?;
// base_units = 50_000_000
contract.lock_funds(&depositor, &bounty_id, &base_units, &deadline);
```

---

*Last updated: based on `token_math` module and `test_token_math.rs` test suite.*