# Pause Management — `program-escrow`

## Overview

The `ProgramEscrowContract` supports granular pause controls for three operation classes:

| Flag | Blocks |
|------|--------|
| `lock_paused` | `lock_program_funds` |
| `release_paused` | `single_payout`, `batch_payout`, `trigger_scheduled_releases` |
| `refund_paused` | `refund_program` |

Only the contract admin can toggle pause flags via `set_paused`.

---

## `set_paused` Function

```rust
pub fn set_paused(
    env: Env,
    lock: Option<bool>,
    release: Option<bool>,
    refund: Option<bool>,
    reason: Option<String>,
)
```

### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `lock` | `Option<bool>` | `Some(true)` pauses lock operations; `Some(false)` unpauses. `None` leaves unchanged. |
| `release` | `Option<bool>` | `Some(true)` pauses release/payout operations; `Some(false)` unpauses. `None` leaves unchanged. |
| `refund` | `Option<bool>` | `Some(true)` pauses refund operations; `Some(false)` unpauses. `None` leaves unchanged. |
| `reason` | `Option<String>` | Optional human-readable reason string. **Bounded to 256 characters.** |

### Authorization

Requires the contract admin to authorize the call.

### Reason String Bound

The `reason` parameter is bounded to **256 characters** (`PAUSE_REASON_MAX_LEN = 256`) to prevent storage abuse. Providing a reason longer than 256 characters will cause the transaction to panic.

---

## Events

### `PauseStateChanged` (V1)

Emitted for backward compatibility. Contains:

| Field | Type | Description |
|-------|------|-------------|
| `operation` | `Symbol` | `"lock"`, `"release"`, or `"refund"` |
| `paused` | `bool` | New pause state |
| `admin` | `Address` | Admin address that triggered the change |
| `reason` | `Option<String>` | Optional reason string |
| `timestamp` | `u64` | Ledger timestamp |
| `receipt_id` | `u64` | Monotonic receipt ID |

### `PauseStateChangedV2` (V2) — Preferred

Emitted alongside V1 for every `set_paused` call. Adds audit trail fields:

| Field | Type | Description |
|-------|------|-------------|
| `version` | `u32` | Event schema version (`EVENT_VERSION_V2`) |
| `operation` | `Symbol` | `"lock"`, `"release"`, or `"refund"` |
| `previous_paused` | `bool` | Pause state **before** this call |
| `paused` | `bool` | New pause state |
| `actor` | `Address` | **The address that triggered the pause state change** |
| `reason` | `Option<String>` | Optional human-readable reason, bounded to 256 chars |
| `timestamp` | `u64` | Ledger timestamp |
| `receipt_id` | `u64` | Monotonic receipt ID |

#### Topics

```
(PAUSE_STATE_CHANGED_V2, operation_symbol)
```

#### Security Notes

- `actor` is always the authenticated admin address — it cannot be spoofed.
- `previous_paused` is read from storage **before** the mutation, accurately reflecting the old → new transition.
- `reason` is bounded to 256 characters to prevent storage abuse.

---

## Audit Trail

The V2 event provides a complete audit trail for incident post-mortems:

- **Who** paused: `actor` field
- **What** was paused: `operation` field
- **Why**: `reason` field
- **When**: `timestamp` field
- **State transition**: `previous_paused` → `paused`

Indexers should prefer `PauseStateChangedV2` over `PauseStateChanged` for new integrations.

---

## Example

```rust
// Pause lock operations with a reason
contract.set_paused(
    &Some(true),   // pause lock
    &None,         // leave release unchanged
    &None,         // leave refund unchanged
    &Some(String::from_str(&env, "Security incident — investigating")),
);
```

This emits a `PauseStateChangedV2` event with:
- `actor` = admin address
- `operation` = `"lock"`
- `previous_paused` = `false`
- `paused` = `true`
- `reason` = `Some("Security incident — investigating")`

---

## Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `PAUSE_REASON_MAX_LEN` | `256` | Maximum characters allowed in a pause reason string |
