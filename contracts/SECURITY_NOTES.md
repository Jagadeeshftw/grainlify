# Security Notes — Timelocked Upgrades & Emergency Pause (v3.0.0)

## Overview

This document covers the security model, threat mitigations, and operational
guidance for the timelocked upgrade and emergency pause features added in
`grainlify-core` v3.0.0.

---

## 1. Timelocked Upgrade Execution

### How it works

1. A registered multisig signer calls `propose_upgrade(proposer, wasm_hash, expiry)`.
2. Other signers call `approve_upgrade(proposal_id, signer)`.
3. When the approval count reaches the configured threshold, the timelock
   **starts automatically** — no separate call required.
4. After `timelock_delay` seconds have elapsed, anyone may call
   `execute_upgrade(proposal_id)` to install the new WASM.

### Constants

| Parameter | Value | Configurable |
|---|---|---|
| `DEFAULT_TIMELOCK_DELAY` | 86 400 s (24 h) | Yes, via `set_timelock_delay` |
| `MIN_TIMELOCK_DELAY` | 3 600 s (1 h) | No |
| `MAX_TIMELOCK_DELAY` | 2 592 000 s (30 d) | No |

### Threat mitigations

| Threat | Mitigation |
|---|---|
| Compromised single key installs malicious WASM immediately | Multisig threshold + timelock window gives other signers time to cancel |
| Admin sets an absurdly long delay to brick upgrades | `MAX_TIMELOCK_DELAY` (30 days) hard-coded in contract |
| Admin sets delay to 0 to bypass review window | `MIN_TIMELOCK_DELAY` (1 hour) hard-coded in contract |
| Stale proposal executed long after approval | `expiry` parameter on `propose_upgrade`; set to a reasonable future timestamp |
| Timelock bypassed by calling `execute_upgrade` before delay | Contract panics with "Timelock delay not met: N seconds remaining" |
| Timelock bypassed by calling `execute_upgrade` without any approvals | Contract panics with "Timelock not started - call approve_upgrade first" |

### Operational guidance

- Always set a non-zero `expiry` on proposals (e.g. `now + 7 days`) to prevent
  stale approvals from being executed weeks later.
- Monitor the `timelock/started` event on-chain; if an unexpected upgrade is
  proposed, cancel it immediately via `cancel_upgrade`.
- Any registered signer can cancel a proposal — this is intentional to allow
  rapid response to a compromised co-signer.

---

## 2. Emergency Pause

### How it works

- Any registered multisig signer calls `pause(signer)` to set the pause flag.
- Any registered multisig signer calls `unpause(signer)` to clear it.
- `is_paused()` is a public view function.

### What is blocked when paused

| Entrypoint | Blocked? | Rationale |
|---|---|---|
| `propose_upgrade` | ✅ Yes | Prevents new upgrade proposals during an incident |
| `approve_upgrade` | ✅ Yes | Prevents threshold from being met during an incident |
| `execute_upgrade` | ❌ No | An already-approved upgrade that passed its timelock should not be blocked by a unilateral pause |
| `cancel_upgrade` | ❌ No | Cancellation is a defensive action; must remain available |
| `migrate` / `commit_migration` | ❌ No | Blocked by `read_only_mode` instead (admin-controlled) |
| `set_timelock_delay` | ❌ No | Blocked by `read_only_mode` instead |

### Threat mitigations

| Threat | Mitigation |
|---|---|
| Attacker proposes malicious upgrade | Any signer can pause to stop further approvals, then cancel the proposal |
| Compromised signer pauses indefinitely | Any other signer can unpause — no single signer has exclusive pause authority |
| Pause used to block a legitimate in-flight upgrade | `execute_upgrade` is not blocked; a proposal that already passed its timelock can still execute |

### Operational guidance

- Pause is a **temporary** measure. Coordinate with other signers to unpause
  once the incident is resolved.
- After pausing, immediately cancel any suspicious proposals via `cancel_upgrade`.
- Do not rely on pause as a long-term security control — use `read_only_mode`
  for extended maintenance windows.

---

## 3. Commit-Reveal Migration Replay Protection

### How it works

1. Admin calls `commit_migration(target_version, hash)` — stores the hash on-chain.
2. Admin calls `migrate(target_version, hash)` — verifies hash matches commitment,
   runs migration, then **deletes the commitment** (one-time use).

### Threat mitigations

| Threat | Mitigation |
|---|---|
| Replayed migration call re-runs state transformation | Commitment is consumed on first use; idempotency guard skips re-runs to same version |
| Wrong migration data passed to `migrate` | Hash mismatch panics with `MigrationHashMismatch` |
| Migration called without prior commitment | Panics with `MigrationCommitmentNotFound` |
| Migration run in read-only mode | `require_not_read_only` guard panics before any state change |

---

## 4. Read-Only Mode vs. Pause — Comparison

| Feature | `pause` | `read_only_mode` |
|---|---|---|
| Who can toggle | Any multisig signer | Admin only |
| Scope | Blocks `propose_upgrade`, `approve_upgrade` | Blocks all state-mutating entrypoints |
| Use case | Incident response during active upgrade flow | Extended maintenance / post-incident lockdown |
| `execute_upgrade` blocked? | No | No |
| `cancel_upgrade` blocked? | No | No |

---

## 5. Test Coverage Summary

File: `contracts/grainlify-core/src/test_timelocked_pause.rs`

| Section | Tests | Coverage focus |
|---|---|---|
| Timelock delay config | 8 | default, min, max, read-only block |
| `propose_upgrade` | 7 | ID increment, wasm hash, proposer, expiry, non-signer |
| `approve_upgrade` | 6 | threshold auto-start, event, no restart, duplicate |
| `execute_upgrade` | 7 | no-timelock panic, before-delay panic, boundary, status |
| `cancel_upgrade` | 6 | marks cancelled, clears timelock, non-signer, double-cancel |
| Emergency pause | 12 | flag set/clear, cycle, non-signer, blocks propose/approve, not execute/cancel |
| `commit_migration` / `migrate` | 12 | happy path, version, from_version, idempotency, missing, mismatch, events, read-only |
| Proposal expiry | 3 | expired, before expiry, zero expiry |
| Initialization paths | 5 | multisig, reinit block, init_with_network |
| Integration flows | 3 | full propose→approve→wait→execute, pause→cancel→repropose, commit→migrate |

**Total: 69 tests** across the new entrypoints, targeting ≥95% line coverage.

---

## 6. Validation Output

```
$ python3 contracts/scripts/validate_seed_file.py
OK: validated 3 manifest(s) and 1 deployment seed file(s).
```
