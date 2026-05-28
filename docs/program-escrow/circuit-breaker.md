# Program Escrow — Circuit Breaker

This document describes the circuit breaker, admin reset, and security model for the `program-escrow` contract.

## Overview

The circuit breaker protects payout operations from cascading failures by tracking consecutive failures and moving through three states:

- `Closed` — normal operation
- `Open` — all protected operations rejected (requires admin action)
- `HalfOpen` — trial period; successes move to `Closed`, failures re-open

The circuit breaker lives in persistent storage under keys defined in `error_recovery::CircuitBreakerKey`.

## Admin Hard Reset: `reset_circuit_breaker`

We added an admin-only entrypoint to immediately reset the circuit breaker to `Closed` and clear failure counters.

- Signature: `reset_circuit_breaker(env: Env, program_id: String) -> Result<(), Error>`
- Authorization: caller must be the contract admin (value stored at `DataKey::Admin`) and must sign the transaction.
- Effects:
  - Calls into `error_recovery::close_circuit()` which sets state to `Closed`, clears `FailureCount` and `SuccessCount`, and clears `OpenedAt`.
  - Additionally ensures `FailureCount` is zero (defensive).
  - Emits an audit event `(circuit, admin_reset)` with payload `(program_id, admin_address, timestamp)`.

## Security Notes

- Only the contract admin can call this entrypoint; the admin address is read from instance storage and `require_auth()` is enforced.
- The function performs a defensive clear of failure counters and emits a deterministic, auditable event so off-chain monitors can verify the manual reset.
- Prefer manual resets only after a root-cause investigation; use `HalfOpen` trial behavior for staged recovery when appropriate.

## Tests

Unit tests were added under `contracts/program-escrow/src/test_admin_reset.rs` verifying that:

- An admin-authorized reset transitions an `Open` circuit to `Closed` and clears counters.
- A non-authorized caller cannot reset the circuit (panics / is rejected).

Note: the repository contains many existing tests; depending on the environment some test suites may not compile or run fully.

## Implementation Notes

- The implementation reuses `error_recovery::close_circuit()` for the state transition and counter resets.
- The audit event topic `admin_reset` is stable and includes the `program_id` to help indexers correlate the reset to a program context.

If you want, I can also update integration tests to exercise the end-to-end flow for `batch_payout`/`single_payout` interactions with the breaker.
