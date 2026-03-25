# Maintenance Mode — `bounty_escrow/escrow`

> **Contract**: `bounty_escrow/contracts/escrow/src/lib.rs`  
> **Test file**: `bounty_escrow/contracts/escrow/src/test_maintenance_mode.rs`  
> **Feature branch**: `feature/bounty-escrow-maintenance`

---

## Table of Contents

1. [Overview](#overview)
2. [How Maintenance Mode Works](#how-maintenance-mode-works)
3. [Public Interface](#public-interface)
   - [`set_maintenance_mode`](#set_maintenance_mode)
   - [`is_maintenance_mode`](#is_maintenance_mode)
4. [Affected Operations](#affected-operations)
5. [Events](#events)
6. [Error Reference](#error-reference)
7. [Interaction with Pause Flags](#interaction-with-pause-flags)
8. [Security Assumptions](#security-assumptions)
9. [Test Coverage Summary](#test-coverage-summary)
10. [Integration Guide](#integration-guide)

---

## Overview

Maintenance mode is a **lock-only circuit breaker** that an admin can toggle at any time to temporarily block new fund deposits without disrupting in-flight escrows. When enabled, `lock_funds` and `batch_lock_funds` immediately return `FundsPaused`; all other operations — `release_funds`, `refund`, partial releases, claims — continue to work normally.

This is intentionally narrower than the granular pause flags (`set_paused`). Its purpose is routine operational windows: upgrades, configuration changes, indexer maintenance, or any situation where accepting new deposits would be unsafe but existing obligations must still be fulfilled.

```
Admin calls set_maintenance_mode(true)
         │
         ▼
MaintenanceMode flag = true  (instance storage)
         │
         ├─ lock_funds / batch_lock_funds  → FundsPaused ✗
         │
         ├─ release_funds                  → allowed    ✓
         ├─ refund                          → allowed    ✓
         ├─ partial_release                 → allowed    ✓
         ├─ claim / claim_with_capability   → allowed    ✓
         └─ all view functions              → allowed    ✓
```

---

## How Maintenance Mode Works

The flag is a single `bool` stored under `DataKey::MaintenanceMode` in **instance storage**. The `check_paused` helper reads it on every `lock` operation:

```rust
fn check_paused(env: &Env, operation: Symbol) -> bool {
    let flags = Self::get_pause_flags(env);
    if operation == symbol_short!("lock") {
        if Self::is_maintenance_mode(env.clone()) {
            return true;         // ← maintenance mode takes effect here
        }
        return flags.lock_paused;
    }
    // release and refund only check their own pause flags — not maintenance mode
    ...
}
```

The key design detail: `is_maintenance_mode` is **only checked in the `lock` branch** of `check_paused`. It has no effect on `release` or `refund` checks. This means:

- Enabling maintenance mode is equivalent to pausing locks, but without touching `PauseFlags.lock_paused`.
- The two mechanisms are additive — if both `lock_paused = true` and `MaintenanceMode = true`, disabling one alone is not enough to re-enable locking; both must be false.
- Disabling maintenance mode restores lock availability even while `PauseFlags` remain unchanged.

---

## Public Interface

### `set_maintenance_mode`

```rust
pub fn set_maintenance_mode(env: Env, enabled: bool) -> Result<(), Error>
```

Enables or disables maintenance mode. **Admin only.**

**Parameters**

| Parameter | Type | Description |
|-----------|------|-------------|
| `enabled` | `bool` | `true` to block new locks; `false` to restore normal operation. |

**Authorization**: `admin.require_auth()` — only the address stored under `DataKey::Admin` may call this.

**Side effects**

- Writes `enabled` to `DataKey::MaintenanceMode` in instance storage.
- Emits a `MaintenanceModeChanged` event (topics: `("MaintSt", "change")`) containing `(enabled, admin, timestamp)`.

**Errors**

| Error | Condition |
|-------|-----------|
| `NotInitialized` | Contract has not been initialized via `init`. |

**Example**

```rust
// Enable — blocks new locks immediately
client.set_maintenance_mode(&true);

// Disable — restores locking
client.set_maintenance_mode(&false);
```

---

### `is_maintenance_mode`

```rust
pub fn is_maintenance_mode(env: Env) -> bool
```

View function. Returns `true` when maintenance mode is active, `false` otherwise. Defaults to `false` if the key has never been written (i.e., on a freshly initialized contract).

**No authorization required** — anyone may call this.

---

## Affected Operations

| Operation | Behavior during maintenance mode |
|-----------|----------------------------------|
| `lock_funds` | Returns `Err(FundsPaused)` |
| `batch_lock_funds` | Returns `Err(FundsPaused)` |
| `lock_funds_anonymous` | Returns `Err(FundsPaused)` |
| `release_funds` | **Unaffected** — proceeds normally |
| `batch_release_funds` | **Unaffected** — proceeds normally |
| `partial_release` | **Unaffected** — proceeds normally |
| `refund` | **Unaffected** — proceeds normally |
| `refund_resolved` | **Unaffected** — proceeds normally |
| `refund_with_capability` | **Unaffected** — proceeds normally |
| `claim` | **Unaffected** — proceeds normally |
| `claim_with_capability` | **Unaffected** — proceeds normally |
| `authorize_claim` | **Unaffected** — proceeds normally |
| All view/query functions | **Unaffected** — proceeds normally |

The asymmetry is deliberate: maintenance mode protects the deposit intake path while guaranteeing that no contributor or depositor is locked out of funds they are already owed.

---

## Events

### `MaintenanceModeChanged`

Emitted every time `set_maintenance_mode` is called, whether enabling or disabling.

```rust
pub struct MaintenanceModeChanged {
    pub enabled: bool,    // true = maintenance on, false = maintenance off
    pub admin: Address,   // address that made the change
    pub timestamp: u64,   // ledger timestamp at the time of the call
}
```

**Topics**: `(Symbol("MaintSt"), Symbol("change"))`

**Data**: the `MaintenanceModeChanged` struct encoded as a Soroban contract value.

Off-chain indexers should listen for this event to maintain an accurate view of contract availability. The event fires on both enable and disable transitions, so a simple boolean field in the indexed state is sufficient.

---

## Error Reference

Maintenance mode itself introduces no new error codes. When a lock operation is attempted while maintenance is active, the existing `FundsPaused` error (code `18`) is returned — the same error emitted by the granular `lock_paused` flag. Callers cannot distinguish between maintenance mode and an explicit lock pause from the error code alone; they must call `is_maintenance_mode()` or inspect `get_pause_flags()` to determine which condition is active.

| Error | Code | Context |
|-------|------|---------|
| `FundsPaused` | 18 | Returned by `lock_funds` / `batch_lock_funds` when maintenance mode is `true` |
| `NotInitialized` | 2 | Returned by `set_maintenance_mode` if contract is not yet initialized |

---

## Interaction with Pause Flags

Maintenance mode and the granular `PauseFlags` (set via `set_paused`) are **independent** mechanisms that both gate the lock operation. The table below shows the combined effect:

| `MaintenanceMode` | `PauseFlags.lock_paused` | `lock_funds` result |
|-------------------|--------------------------|---------------------|
| `false` | `false` | Allowed |
| `true` | `false` | `FundsPaused` |
| `false` | `true` | `FundsPaused` |
| `true` | `true` | `FundsPaused` |

To fully re-enable locking after both flags are set, **both** must be cleared:

```rust
client.set_maintenance_mode(&false);
client.set_paused(&Some(false), &None, &None, &None);
```

Calling only one while the other remains active will continue to block new locks.

`set_paused` for `release` and `refund` operations is not affected by maintenance mode at all — those flags are read independently and maintenance mode has no influence on them.

---

## Security Assumptions

### 1. Single-admin authorization

`set_maintenance_mode` requires `admin.require_auth()`. The admin key is stored immutably at `init` time. If the admin key is compromised, an attacker could toggle maintenance mode to deny deposits or — more critically — disable it during an active incident response to re-open the deposit path before a fix is ready. Admin key custody must be treated as a critical operational secret.

### 2. Maintenance does not freeze existing funds

The design guarantees that enabling maintenance mode never prevents contributors or depositors from recovering their already-locked funds. Release, refund, and claim paths are deliberately excluded from the maintenance check. An admin cannot use maintenance mode alone to trap funds — they would need to separately pause release and refund operations, which would require explicit calls to `set_paused` and would emit additional audit events.

### 3. No cooldown or time-lock

There is no minimum duration for maintenance mode. An admin can enable and immediately disable it within the same ledger (or in rapid succession across ledgers). This simplicity is intentional for operational agility, but it means maintenance mode does not provide any enforceable delay guarantee to depositors. If stronger guarantees are needed (e.g., a 24-hour notice period before blocking deposits), that logic must be implemented in the calling layer or a governance contract.

### 4. Instance storage scope

`DataKey::MaintenanceMode` lives in instance storage, which is shared across all contract invocations in the same contract instance. There is no per-token or per-bounty scoping — maintenance mode is a global flag. Deployments that want independent maintenance windows for different asset types should use separate contract instances.

### 5. No reentrancy interaction

Maintenance mode is checked in `check_paused` before the reentrancy guard is acquired in `lock_funds_logic`. This ordering means a failed maintenance check exits cleanly without acquiring the guard, and there is no risk of a stuck guard if maintenance is enabled mid-call (which cannot happen within a single transaction on Soroban anyway, since ledger state is snapshotted per-transaction).

### 6. `FundsPaused` ambiguity

Because `set_maintenance_mode` and `set_paused(lock=true)` both produce `FundsPaused`, callers that only inspect the error code cannot distinguish the two. This is acceptable for transaction senders (they simply retry after the maintenance window), but monitoring systems should use `is_maintenance_mode()` and `get_pause_flags()` together to emit accurate alerts.

---

## Test Coverage Summary

The test file `test_maintenance_mode.rs` covers the following scenarios:

| Test name | Scenario |
|-----------|----------|
| `test_maintenance_mode_toggles_and_blocks_lock` | Enable maintenance → verify `is_maintenance_mode()` returns `true` → verify `MaintenanceModeChanged` event contains correct `(enabled, admin, timestamp)` → disable → verify `false` |
| `test_lock_fails_in_maintenance_mode` | Mint tokens to depositor → enable maintenance → call `lock_funds` → verify panic with `Error(Contract, ...)` (`FundsPaused`) |
| `test_release_and_refund_allowed_in_maintenance_mode` | Lock funds before maintenance → enable maintenance → call `release_funds` → verify contributor receives correct balance, no panic |

### Coverage gaps to consider adding

The existing three tests provide a solid baseline. The following additional cases would complete the coverage matrix and meet the 95% threshold required by the contribution guidelines:

| Suggested test | Rationale |
|----------------|-----------|
| `test_batch_lock_fails_in_maintenance_mode` | `batch_lock_funds` goes through the same `check_paused` path and should be verified independently |
| `test_refund_allowed_in_maintenance_mode` | Mirrors the release test for the refund path; explicitly documents the non-blocking guarantee |
| `test_maintenance_mode_does_not_affect_granular_pause` | Verify that disabling maintenance does not clear `PauseFlags.lock_paused`, and vice versa |
| `test_set_maintenance_mode_not_initialized_fails` | Contract not initialized → `NotInitialized` error |
| `test_set_maintenance_mode_unauthorized_fails` | Non-admin caller → auth failure |
| `test_maintenance_event_on_disable` | Verify `MaintenanceModeChanged` is emitted with `enabled=false` on disable, not just on enable |
| `test_lock_resumes_after_maintenance_disabled` | Enable → disable → `lock_funds` succeeds, confirming toggle is truly bidirectional |

---

## Integration Guide

### Checking maintenance status before submitting a lock

Off-chain clients should check both conditions before submitting a `lock_funds` transaction to avoid paying transaction fees on a predictably failing call:

```typescript
const isMaintenance = await contract.is_maintenance_mode();
const pauseFlags = await contract.get_pause_flags();

if (isMaintenance || pauseFlags.lock_paused) {
  throw new Error("Lock operations are currently unavailable");
}
```

### Subscribing to maintenance events

Indexers and dashboards should subscribe to `("MaintSt", "change")` events on the contract address and update a cached availability flag accordingly. This avoids polling `is_maintenance_mode()` on every page load.

### Admin runbook

**Entering maintenance mode:**
```bash
soroban contract invoke \
  --id $CONTRACT_ID \
  --source $ADMIN_SECRET \
  -- set_maintenance_mode --enabled true
```

**Exiting maintenance mode:**
```bash
soroban contract invoke \
  --id $CONTRACT_ID \
  --source $ADMIN_SECRET \
  -- set_maintenance_mode --enabled false
```

**Verifying current state:**
```bash
soroban contract invoke \
  --id $CONTRACT_ID \
  -- is_maintenance_mode
```

---

*Last updated: based on `lib.rs` implementation and `test_maintenance_mode.rs` test suite.*