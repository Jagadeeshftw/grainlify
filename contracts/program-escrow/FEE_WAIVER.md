# Fee Waiver — Per-PayoutType Fee Exemption

## Overview

The fee waiver feature lets an admin mark specific payout types (`Single`, `Batch`)
as exempt from the payout fee configured in `FeeConfig`. When a payout type is
waived, `single_payout` or `batch_payout` transfers the full gross amount to the
recipient(s) without deducting any fee, and nothing is transferred to
`fee_recipient`.

---

## Motivation

Some payment flows — grants, payroll runs, rebates — should be fee-free even when
a fee is configured for other flows. Rather than requiring a separate contract
deployment or temporary `update_fee_config` calls (which introduce race conditions
and extra admin overhead), admins can permanently or temporarily mark a payout
type as waived.

---

## Data Model

### `FEE_WAIVER_SINGLE` / `FEE_WAIVER_BATCH` constants

```rust
pub const FEE_WAIVER_SINGLE: u32 = 1 << 0;  // 0b01
pub const FEE_WAIVER_BATCH:  u32 = 1 << 1;  // 0b10
```

### `FeeConfig.fee_waivers: u32`

A bitmask stored inside `FeeConfig` in instance storage. A set bit means the
corresponding payout type is waived.

| Bit | Constant            | Meaning                             |
|-----|---------------------|-------------------------------------|
| 0   | `FEE_WAIVER_SINGLE` | `single_payout` skips the fee       |
| 1   | `FEE_WAIVER_BATCH`  | `batch_payout` skips the fee        |

Default value: `0` (no waivers; all fees charge normally).

### `FeeWaiverUpdatedEvent`

Emitted by `set_fee_waiver` with topic `("FeeWaivr",)`.

| Field             | Type      | Description                              |
|-------------------|-----------|------------------------------------------|
| `version`         | `u32`     | Always `2` (`EVENT_VERSION_V2`)          |
| `payout_type_bit` | `u32`     | Bitmask constant for the affected type   |
| `waived`          | `bool`    | `true` if waiver enabled, `false` if removed |
| `updated_by`      | `Address` | Admin address that made the change       |
| `timestamp`       | `u64`     | Ledger timestamp at the time of the call |

---

## API

### `set_fee_waiver(payout_type: PayoutType, waived: bool)`

- **Caller**: Admin only (`require_admin` enforces `admin.require_auth()`).
- **Effect**:
  - `waived = true` → sets the corresponding bit in `fee_waivers`.
  - `waived = false` → clears the corresponding bit in `fee_waivers`.
- **Emits**: `FeeWaiverUpdatedEvent`.
- **Failure**: Panics if caller is not the admin (Soroban auth enforcement).

> **Note on `PayoutType::Batch(u32)` payload:** The inner `u32` value of the
> `Batch` variant is ignored for waiver lookup. Any `Batch(_)` value maps to the
> `FEE_WAIVER_BATCH` bit, and any `Batch(_)` value queries that same bit.

---

## Fee Calculation

The helper `is_fee_waived(fee_waivers, payout_type)` is called before
`combined_fee_amount` in both payout paths:

```
single_payout_internal:
  pay_fee = if is_fee_waived(cfg.fee_waivers, &PayoutType::Single) { 0 }
            else { combined_fee_amount(...) }

batch_payout_internal:
  batch_fee_waived = is_fee_waived(cfg.fee_waivers, &PayoutType::Batch(0))
  for each recipient:
    pay_fee = if batch_fee_waived { 0 }
              else { combined_fee_amount(...) }
```

When `pay_fee == 0` the token transfer to `fee_recipient` is skipped entirely —
no zero-amount transfer is issued.

---

## Security Assumptions

| Property | Guarantee |
|----------|-----------|
| **Admin-only mutation** | `set_fee_waiver` calls `require_admin`, which calls `admin.require_auth()`. Any non-admin call panics and reverts atomically. |
| **No race condition** | Waivers are stored atomically in the same `FeeConfig` instance entry as the fee rates. There is no window where fee_waivers is partially written. |
| **Waiver is per-type, not per-recipient** | Waivers apply to the entire payout type. Per-recipient exemptions are not supported. |
| **`fee_enabled = false` still respected** | When `fee_enabled` is `false`, `combined_fee_amount` already returns `0`. Waivers are logically redundant in that case but do not cause errors. |
| **Bitmask overflow** | `u32` provides 32 bits; only bits 0 and 1 are currently used. Future variants can claim additional bits without a storage migration. |

---

## Configuration Example

```rust
// Enable a 5% payout fee
client.update_fee_config(
    &None, &Some(500), &None, &None,
    &Some(fee_recipient), &Some(true),
);

// Waive the fee for batch payouts (e.g., payroll runs)
client.set_fee_waiver(&PayoutType::Batch(0), &true);

// Later, restore batch fee
client.set_fee_waiver(&PayoutType::Batch(0), &false);
```

---

## Test Coverage

Tests are in `src/test_fee_waiver.rs` (10 cases):

| # | Test | What it verifies |
|---|------|-----------------|
| 1 | `test_single_waiver_delivers_full_amount` | Single waiver → recipient gets gross, fee_recipient gets 0 |
| 2 | `test_batch_waiver_delivers_full_amounts` | Batch waiver → all recipients get gross |
| 3 | `test_single_waiver_does_not_affect_batch` | Single waiver only — batch fee still charged |
| 4 | `test_batch_waiver_does_not_affect_single` | Batch waiver only — single fee still charged |
| 5 | `test_clearing_single_waiver_restores_fee` | Removing waiver → fee resumes |
| 6 | `test_set_fee_waiver_requires_admin` | Non-admin call panics |
| 7 | `test_get_fee_config_reflects_waiver_bitmask` | `get_fee_config` exposes correct `fee_waivers` |
| 8 | `test_both_types_waived_simultaneously` | Both bits set → zero fees on both types |
| 9 | `test_fee_recipient_receives_nothing_when_waived` | Token transfer to fee_recipient is skipped |
| 10 | `test_fee_waiver_event_is_emitted` | `FeeWaiverUpdatedEvent` emitted with correct fields |
