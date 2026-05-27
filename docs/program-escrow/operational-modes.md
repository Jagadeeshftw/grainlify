# Operational Modes: Maintenance vs Read-Only

## Overview

The program-escrow contract exposes two distinct operational modes that operators activate to restrict contract behaviour during upgrades, incidents, or audits.

| Mode | Admin writes | User writes | Read queries | Emergency withdraw |
|------|-------------|-------------|--------------|-------------------|
| Normal | Yes | Yes | Yes | No |
| Maintenance | Yes | No | Yes | Yes |
| Read-Only | No | No | Yes | No |

---

## Maintenance Mode

### Definition
Maintenance mode blocks all state-mutating operations from non-admin callers while preserving admin access for emergency recovery. Read queries remain fully available.

### Permitted operations
- All view/query methods
- Admin: emergency_withdraw, set_paused, set_maintenance_mode, set_read_only_mode
- Admin: single_payout and batch_payout for graceful wind-down

### Blocked operations
- lock_program_funds panics with Funds Paused
- Non-admin payouts

### When to use
- Scheduled contract upgrades
- Suspected exploit requiring investigation
- Migrating funds to a new contract version
- Circuit breaker has tripped and admin needs controlled wind-down

### Activation
- contract.set_maintenance_mode(true)

### Events emitted
- Topic: MaintSt
- Payload: MaintenanceModeChanged { enabled, admin, timestamp }

---

## Read-Only Mode

### Definition
Read-only mode is a stricter subset of maintenance mode. It blocks ALL state-mutating operations including admin writes. Only pure view queries are permitted.

### Permitted operations
- All view/query methods only

### Blocked operations
- lock_program_funds
- single_payout, batch_payout
- set_paused
- emergency_withdraw
- Any function that mutates contract storage

### When to use
- Pre-upgrade snapshot verification
- Regulatory compliance window requiring frozen state
- Post-incident forensic hold

### Activation
- contract.set_read_only_mode(true, reason)

### Events emitted
- Topic: ROModeChg
- Payload: ReadOnlyModeChanged { enabled, admin, timestamp, reason }

---

## Interaction Matrix

### With Pause Flags

| Flag | Maintenance Mode | Read-Only Mode |
|------|-----------------|----------------|
| lock_paused=true | lock_program_funds blocked | lock_program_funds blocked |
| release_paused=true | payouts blocked | payouts blocked |
| refund_paused=true | refunds blocked | refunds blocked |
| maintenance=true | internally sets lock_paused | N/A |

When maintenance mode is enabled it internally activates the lock pause flag. This is why lock_program_funds panics with Funds Paused not Maintenance Mode. The pause flag system is the single enforcement point for fund operations.

### With Circuit Breaker

| Circuit Breaker State | Maintenance Mode | Read-Only Mode |
|----------------------|-----------------|----------------|
| Not tripped | Normal blocking applies | Normal blocking applies |
| Tripped auto-pause | Admin can override via emergency_withdraw | All writes blocked including emergency_withdraw |
| Tripped plus maintenance | Admin retains full recovery access | No recovery possible until read-only is lifted |

### Mode Priority

If both modes are active simultaneously read-only takes precedence because its check runs first in the guard chain.

---

## Security Assumptions

1. Only the contract admin can activate or deactivate either mode
2. Maintenance mode preserves emergency_withdraw so funds are never permanently trapped
3. Neither mode clears existing pause flags
4. Both modes emit auditable on-chain events with timestamp and admin address
5. Read-only mode cannot be exited except by lifting it before it was set or via contract upgrade

---

## Upgrade Checklist

1. Call set_maintenance_mode(true) to block new deposits
2. Complete all in-flight payouts
3. Call emergency_withdraw to drain remaining balance if needed
4. Deploy new contract version
5. Call set_maintenance_mode(false) on old contract

---

## FAQ

Q: Can both modes be active simultaneously?
A: Yes. Read-only takes precedence over maintenance.

Q: Does either mode affect view calls?
A: No. All read queries succeed in both modes.

Q: Why does lock_program_funds panic with Funds Paused in maintenance mode?
A: Maintenance mode internally sets the lock pause flag. The pause system is the single enforcement point.

Q: Can the circuit breaker override read-only mode?
A: No. Read-only blocks all writes including emergency_withdraw. Lift read-only first.

---

*Last updated: 2026-05-27 | Contract version: 2.0.0*
