# Upgrade Governance & Timelock

## Overview

The `grainlify-core` contract implements a timelocked multisig upgrade governance
system. All WASM upgrades must pass through a proposal → approval → timelock →
execution pipeline. The timelock delay prevents immediate execution after
threshold approval, giving stakeholders time to review and react.

## Timelock Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `DEFAULT_TIMELOCK_DELAY` | 86 400 s (24 h) | Applied when no custom delay is set |
| `MIN_TIMELOCK_DELAY` | 3 600 s (1 h) | Minimum allowed delay |
| `MAX_TIMELOCK_DELAY` | 2 592 000 s (30 d) | Maximum allowed delay |

## Upgrade Flow

```
1. propose_upgrade(proposer, wasm_hash)  → proposal_id
2. approve_upgrade(signer, proposal_id)  → starts timelock when threshold met
3. [wait timelock_delay seconds]
4. execute_upgrade(proposal_id)          → installs new WASM
```

## Entrypoints

| Function | Auth | Description |
|----------|------|-------------|
| `propose_upgrade(proposer, wasm_hash)` | signer | Create upgrade proposal |
| `approve_upgrade(signer, proposal_id)` | signer | Approve; starts timelock at threshold |
| `execute_upgrade(proposal_id)` | any | Execute after delay elapsed |
| `cancel_upgrade(caller, proposal_id)` | admin/proposer | Cancel proposal |
| `set_timelock_delay(delay_seconds)` | admin | Update delay (1 h – 30 d) |
| `get_timelock_delay()` | view | Current delay in seconds |
| `get_timelock_status(proposal_id)` | view | Remaining seconds (0 = ready) |

## Boundary Enforcement

### Minimum (1 hour)

`set_timelock_delay` panics with `"Timelock delay must be at least 1 hour (3600 seconds)"` for any value < 3 600.

This prevents an admin from setting a trivially short delay that would allow
near-instant upgrades, defeating the purpose of the timelock.

### Maximum (30 days)

`set_timelock_delay` panics with `"Timelock delay cannot exceed 30 days (2592000 seconds)"` for any value > 2 592 000.

This prevents an admin from accidentally (or maliciously) setting a delay so
long that the upgrade path becomes permanently bricked.

### Execute-upgrade timing

`execute_upgrade` panics with `"Timelock delay not met: X seconds remaining"` if
called before `timelock_start + timelock_delay` seconds have elapsed.

## Security Assumptions

1. **Admin key security** — only the admin can change the timelock delay. A
   compromised admin key can reduce the delay to the minimum (1 h) but cannot
   bypass it entirely.
2. **Timelock start is immutable** — once the approval threshold is met, the
   timelock start timestamp is written to storage and cannot be changed.
3. **No bypass** — there is no emergency override that skips the timelock.
   Use `cancel_upgrade` + re-propose if an urgent fix is needed.
4. **Monotonic clock** — the contract uses `env.ledger().timestamp()` which is
   set by validators and cannot be manipulated by the contract caller.

## Boundary Test Coverage (issue #1293)

| Scenario | Expected |
|----------|----------|
| `set_timelock_delay(3600)` | ✅ succeeds |
| `set_timelock_delay(3599)` | ❌ panics |
| `set_timelock_delay(0)` | ❌ panics |
| `set_timelock_delay(2592000)` | ✅ succeeds |
| `set_timelock_delay(2592001)` | ❌ panics |
| `set_timelock_delay(u64::MAX)` | ❌ panics |
| `execute_upgrade` at t < expiry | ❌ panics |
| `execute_upgrade` at t = expiry | ✅ succeeds |
| `execute_upgrade` at t > expiry | ✅ succeeds |
