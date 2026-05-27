# Protection Layer Precedence

This document defines the canonical evaluation order for the three overlapping
protection layers in `contracts/program-escrow/src/lib.rs`: **circuit breaker**,
**pause modes**, and **read-only mode**.

## TL;DR — Evaluation Order
Each layer is checked in sequence. If a higher-priority layer blocks the
operation, the lower layers are never evaluated.

---

## Layer Definitions

### 1. Pause Mode (highest priority among the three)

Controlled by `PauseFlags` (stored at `DataKey::PauseFlags`) and the
`MaintenanceMode` flag (stored at `DataKey::MaintenanceMode`).

- `lock_paused` blocks `lock_program_funds` and `lock_program_funds_v2`
- `release_paused` blocks `batch_payout`, `single_payout`,
  `trigger_program_releases`, `execute_claim`, and all `*_by` variants
- `refund_paused` blocks `cancel_claim`
- `MaintenanceMode = true` is treated as `lock_paused` for the `lock` operation
  (see `check_paused` in `lib.rs`)

**Error message:** `"Funds Paused"` or `"Contract is in read-only maintenance mode"`

**Why first?** Pause is the operator's emergency stop. It must be respected
before any other logic runs, including circuit-breaker state checks, so that
operators can halt activity even when the circuit breaker is closed.

### 2. Read-only Mode

Controlled by `DataKey::ReadOnlyMode` (bool, defaults to `false`).

When `true`, all state-mutating operations are blocked via `require_not_read_only`.
Currently applied explicitly in `lock_program_funds_v2` and `single_payout_v2`.

**Error message:** `"Read-only mode"` or `"Contract is in read-only maintenance mode"`

**Why second?** Read-only mode is a softer administrative freeze. It runs after
pause (the hard emergency stop) but before the circuit breaker (the automated
failure-rate guard).

### 3. Circuit Breaker (lowest priority of the three)

Implemented in `error_recovery.rs`. Three states:

| State      | Behaviour                                                   |
|------------|-------------------------------------------------------------|
| `Closed`   | Normal operation. Requests pass through.                    |
| `Open`     | All payout operations are rejected immediately.             |
| `HalfOpen` | Admin has initiated reset; next success closes the circuit. |

The circuit opens automatically when `failure_count >= failure_threshold`
(default: 3). It can also be opened by a threshold monitor breach via
`check_and_allow_with_thresholds`.

**Error message:** `"Circuit breaker is OPEN"` or `"Operation rejected by circuit breaker"`

**Why last?** The circuit breaker is an automated guard against cascading
failures. It should only be consulted after the operator's explicit controls
(pause, read-only) have been satisfied, so that manual overrides always take
precedence over automated state.

---

## Full Guard Chain for Payout Operations

The complete precedence chain in `batch_payout_internal` and
`single_payout_internal` is:
Read-only mode (LAYER 2) is enforced at the top of v2 entry-points
(`lock_program_funds_v2`, `single_payout_v2`) before this chain begins.

Steps 3 and 7 are where the three protection layers appear. Step 3 (pause)
always runs before step 7 (circuit breaker).

---

## Operator Decision Matrix

| Scenario                                 | Expected behaviour                             |
|------------------------------------------|------------------------------------------------|
| Pause active, circuit closed             | Rejected at step 3 — "Funds Paused"            |
| Pause active, circuit open               | Rejected at step 3 — pause wins               |
| Pause inactive, read-only active         | Rejected by `require_not_read_only`            |
| Pause inactive, circuit open             | Rejected at step 7 — "Circuit breaker is OPEN" |
| All three active simultaneously          | Rejected at step 3 — pause wins               |
| None active                              | Operation proceeds normally                    |

---

## Resetting Each Layer

| Layer        | How to disable / reset                                          |
|--------------|-----------------------------------------------------------------|
| Pause        | `set_paused(lock: Some(false), release: Some(false), ...)`      |
| Maintenance  | `set_maintenance_mode(false)`                                   |
| Read-only    | `set_read_only_mode(false, None)`                               |
| Circuit open | `reset_circuit_breaker(caller)` -> Open -> HalfOpen -> Closed   |

Only the contract admin can perform any of these operations.

---

## Security Assumptions

1. **Pause is authoritative.** If an operator pauses operations, the circuit
   breaker state is irrelevant — no payouts will go through.
2. **Circuit breaker is automated.** It responds to observed failure rates and
   threshold breaches without requiring admin action.
3. **Read-only mode is additive.** It does not replace pause — both can be
   active simultaneously, and pause still wins.
4. **No layer bypasses another.** The guard chain in `lib.rs` must not be
   reordered without updating this document and the integration tests in
   `test_circuit_breaker_enforcement.rs`.

---

## Related Files

| File                                                              | Role                                    |
|-------------------------------------------------------------------|-----------------------------------------|
| `contracts/program-escrow/src/lib.rs`                            | Guard chain implementation              |
| `contracts/program-escrow/src/error_recovery.rs`                 | Circuit breaker state machine           |
| `contracts/program-escrow/src/reentrancy_guard.rs`               | Reentrancy protection                   |
| `contracts/program-escrow/src/test_circuit_breaker_enforcement.rs` | Integration tests for precedence      |
