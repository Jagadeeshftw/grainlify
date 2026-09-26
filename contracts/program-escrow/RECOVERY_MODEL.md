# Program Escrow Recovery Model

This document describes the recovery behavior implemented by `src/error_recovery.rs` and the contract methods that expose it. Recovery has two separate mechanisms: the circuit breaker controls whether protected operations may start, while batch recovery records the outcome of each item in a partially completed batch. Batch recovery helpers are currently internal to the crate; they are not `ProgramEscrowContract` entry points.

## Failure classes and restored invariants

| Failure class | Recovery path | Invariant restored or guaranteed |
| --- | --- | --- |
| Repeated transient operation failures | `record_failure` opens the circuit at the configured threshold. After the recovery window, `check_timeout_transitions` permits a `HalfOpen` probe; an authorized administrator can also call `reset_circuit_breaker`. A successful probe is recorded through `record_success`. | `Open` rejects protected operations. A `HalfOpen` circuit closes after `success_threshold` recorded successes; closing clears the failure/success counters and `opened_at`. The implementation does not clear the HalfOpen success counter for each sub-threshold failure, so it does not guarantee those successes are consecutive. A threshold-reaching failure reopens the circuit. |
| Threshold breach or security emergency | `check_and_allow_with_thresholds` opens the circuit when threshold monitoring reports a breach. The circuit admin can call `emergency_open_circuit` to halt protected payouts immediately. | The circuit is `Open`, so subsequent guarded operations are denied until the recovery window enables a `HalfOpen` probe or an authorized admin resets it. |
| Exhausted transient retries | `execute_with_retry` stops after `max_attempts`, records each failed attempt, and returns the last error. This helper is for simulation/test use; production callers use the guard and record functions around their operation. | The helper does not claim the operation succeeded. Failure state remains visible to the circuit breaker and the caller receives a failed `RetryResult`. |
| Partial batch transfer failure | `store_batch_state` checkpoints items; `mark_item_success` and `mark_item_failed` record individual outcomes. `get_failed_items` identifies failed items and `increment_retry_count` moves an eligible failed item back to `Pending`. The caller performs any actual token transfer and reports its result. | Tracked `successful_amount` equals the sum of `Success` item amounts, and `total_amount` equals the sum of `Success`, `Pending`, `Failed`, and `RolledBack` item amounts. No automatic transfer or retry is performed by these helpers. |
| Partial batch success requiring compensation | An authorized batch key calls `prepare_rollback` to obtain the successful recipients and amount. The caller performs the token transfers back and records completed compensations with `mark_item_rolled_back`. | Once each compensation is recorded, rolled-back items no longer contribute to `successful_amount`, and the batch amount accounting remains balanced. `prepare_rollback` only prepares a result; it does not transfer funds or mutate item status. |
| Batch completion or explicit cancellation | `finalize_batch` verifies integrity before archiving and clearing a completed batch. `cancel_batch_recovery` lets the batch's authorized key stop a pending recovery. | Finalization requires internally consistent item accounting and no completed batch with pending items. Cancellation removes the batch from the pending list and sets its completion timestamp; it does not reverse successful transfers. |

### Accounting boundary

The batch invariant is about the checkpoint's item ledger, not an independent token-balance proof. The code stores `original_balance`, but `verify_batch_integrity` validates the status/amount totals and tracked successful amount; it does not compare current token balances with `original_balance`. Callers must report each item transition once and only after the matching transfer outcome: the mark helpers do not enforce valid prior states, and duplicate or premature updates can invalidate accounting. Soroban transaction atomicity separately rolls back state changes when an invocation traps. This module's persisted partial-batch recovery is for flows that record and retain partial outcomes, not a repair mechanism for state from a reverted invocation.

## States that recovery does not resolve

The following cases are terminal for the corresponding recovery attempt or require intervention outside this module:

- An unknown batch ID has no checkpoint to recover (`ERR_BATCH_NOT_FOUND`).
- A completed batch cannot be cancelled again (`ERR_BATCH_ALREADY_COMPLETE`).
- A failed item at `max_retries` cannot be scheduled for another retry (`ERR_BATCH_NOT_RECOVERABLE`).
- A caller other than the batch's `authorized_key` cannot cancel or prepare rollback (`ERR_UNAUTHORIZED_RECOVERY`); only the registered circuit admin may reset, configure, or emergency-open the circuit.
- Rollback cannot be prepared when rollback is disabled (`ERR_ROLLBACK_DISABLED`) or when the batch has no successful items (`ERR_NO_SUCCESSFUL_ITEMS`).
- A missing or internally inconsistent checkpoint cannot be finalized as a valid batch (`verify_batch_integrity` returns false; `finalize_batch` returns `ERR_BATCH_INTEGRITY`). It is not repaired or reconstructed by this module.
- Expiry is detectable with `is_recovery_expired` (a missing batch is also reported as expired), and `verify_batch_recovery_invariants` reports false for an expired pending recovery. The current retry, cancellation, and rollback helpers do not enforce expiry themselves; after expiry, an operator must choose an authorized disposition rather than treating expiry as automatic cleanup or a fund refund.
- An `Open` circuit with `opened_at == 0` cannot transition to `HalfOpen` through timeout because the timeout check treats zero as unset. This can occur if the circuit opens at ledger timestamp zero; an authorized admin reset is required.
- The circuit breaker does not resolve a permanently failing operation or establish that its underlying cause is fixed. It only blocks, probes, and re-enables guarded calls. Administrative hard reset to `Closed` clears breaker counters without proving external service health.

A panic, storage/serialization corruption, token-transfer failure that reverts the whole Soroban invocation, or a balance discrepancy outside the tracked item ledger has no automatic repair path here. The caller must diagnose it and use the platform's transaction semantics or an independently authorized operational process.

## Contract entry-point inventory

These are the recovery-related methods in `ProgramEscrowContract` (`src/lib.rs`). Read-only methods are listed to make the exposed surface complete; they do not perform recovery.

| Contract method | Recovery role | Failure class covered |
| --- | --- | --- |
| `set_circuit_admin` | Set or replace the circuit administrator under the module's authorization rules. | Recovery authority setup. |
| `get_circuit_admin` | Reads the registered circuit administrator. | Recovery authority diagnosis only. |
| `reset_circuit_breaker` | Admin reset: `Open` becomes `HalfOpen`; `HalfOpen` or `Closed` becomes `Closed`. | Repeated transient failures and operator-directed recovery. |
| `configure_circuit_breaker` | Admin configures failure/success thresholds, log cap, and recovery window. | Recovery policy configuration; does not itself repair an operation. |
| `set_prog_cb_threshold` | A program's authorized payout key sets or clears that program's circuit failure threshold. | Per-program threshold policy; changes when repeated failures trip the breaker. |
| `emergency_open_circuit` | Admin immediately blocks guarded payouts. | Threshold/security emergency. |
| `init_threshold_monitoring` | Initializes threshold monitoring with its defaults and resets current window metrics/cooldown state. Despite the wrapper comment, repeating it is not behaviorally idempotent. | Enables threshold-based circuit opening; does not itself recover an operation. |
| `get_threshold_config` | Reads threshold-monitoring configuration. | Diagnosis only. |
| `get_circuit_breaker_status`, `get_circuit_status` | Read the current breaker state and counters. | Diagnosis only. |
| `get_circuit_error_log`, `archive_circuit_breaker_logs`, `get_circuit_failure_archive` | Read or archive breaker failure evidence. Archiving requires the registered admin. | Diagnosis/audit only. |

The batch functions (`store_batch_state`, `mark_item_success`, `mark_item_failed`, `get_failed_items`, `increment_retry_count`, `prepare_rollback`, `mark_item_rolled_back`, `cancel_batch_recovery`, `finalize_batch`, and the integrity/expiry queries) are `pub` Rust functions in a private module, not Soroban contract entry points. Their caller must apply the authorization described above and execute any token transfers; the public Rust visibility does not expose them in the contract ABI.

## Test organization by failure class

The existing tests are in `src/error_recovery_tests.rs`. This index assigns their current groups to the failure model so each recovery class can be reviewed independently.

| Failure class | Existing groups and representative tests |
| --- | --- |
| Circuit state and threshold trips | Groups 1-4 and 18; `test_initial_state_is_closed`, `test_circuit_opens_at_threshold`, `test_circuit_open_rejects_operations`, `test_many_successes_in_closed_state_never_open`. |
| Circuit recovery after transient failures | Groups 5-8, 14, and 14b; `test_reset_open_to_half_open`, `test_success_in_half_open_closes_circuit`, `test_failure_in_half_open_reopens_circuit`, `test_circuit_stays_open_until_admin_reset`. |
| Retry exhaustion and retry success | Groups 10-11 and 19-28; includes `test_retry_exhaustion_opens_circuit`, `test_retry_success_on_second_attempt_resets_failures`, policy/backoff boundary tests, and `test_policy_stops_on_circuit_open_mid_retry`. |
| Recovery authority and policy | Groups 12-13 and 17; includes `test_unauthorized_reset_panics`, config-change tests, and admin-management tests. |
| Partial batch failure and retry eligibility | Groups 29-32 and 42; includes configuration/checkpoint/item-status tests, `test_partial_batch_failure_tracking`, and retry-count limit tests. |
| Compensation after partial success | Group 33; includes rollback amount, authorization, disabled rollback, and no-success cases. |
| Checkpoint integrity and terminal disposition | Groups 34, 38-39, and 41; includes integrity, finalization, invariant, and recovery-history tests. |
| Batch recovery timeout and pending state | Groups 35-37; includes expiry detection, cancellation, authorization, and pending-recovery list tests. |
| Boundary cases | Group 40; single-item, large amount, item status transition, and batch-isolation tests. |

`error_recovery_tests.rs` is currently not included by `lib.rs` because it is marked as pre-existing breakage. The test index describes the existing tests, but those tests do not run as part of the crate suite until that independent compilation issue is repaired.
