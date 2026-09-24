# Contract Deprecation and Migration (Kill Switch)

This document describes the controlled kill-switch mechanism used to deprecate older contract versions while allowing existing escrows to complete or migrate, without locking funds.

## Overview

- **Deprecated flag**: When set by the admin, the contract stops accepting **new** locks/registrations.
- **Migration target**: Optional address (e.g. new contract) exposed to users for migration guidance.
- **Existing escrows**: Release, refund, and other unwind flows remain available; only **new** escrows/registrations are blocked.

## Affected Contracts

- **Bounty escrow** (`contracts/bounty_escrow`): `lock_funds` and `batch_lock_funds` are blocked when deprecated; release and refund still work.
- **Soroban escrow** (`soroban/contracts/escrow`): `lock_funds` blocked when deprecated.
- **Soroban program-escrow** (`soroban/contracts/program-escrow`, crate `soroban-program-escrow`): `register_program` and `batch_register_programs` blocked when deprecated.

## Admin Operations

### Set deprecation (admin only)

- **Bounty escrow**: `set_deprecated(deprecated: bool, migration_target: Option<Address>)`  
  Call with `deprecated = true` to enable the kill switch; optionally set `migration_target` to the new contract address.
- **Soroban escrow / soroban-program-escrow**: Same signature and behavior.

### View deprecation status (anyone)

- **Bounty escrow**: `get_deprecation_status() -> DeprecationStatus { deprecated, migration_target }`
- **Soroban contracts**: `get_deprecation_status() -> DeprecationState { deprecated, migration_target }`

Clients and UIs should call this before attempting new locks or registrations and, if deprecated, show the migration target and block new actions.

## Events

When deprecation state changes, contracts emit a deprecation event (e.g. topic `deprec`) with:

- `deprecated`: bool  
- `migration_target`: Option<Address>  
- `admin`: Address  
- `timestamp`: u64  

Use these events for indexing and notifications.

## Migration Procedure

1. **Deploy** the new contract version and configure it as needed.
2. **Set migration target** on the old contract:  
   `set_deprecated(true, Some(new_contract_address))`.  
   This enables the kill switch and advertises the new contract.
3. **Communicate** to users (UI, docs, events): no new locks/registrations on the old contract; use the new contract and, if applicable, migrate existing positions per the new contract’s design.
4. **Let existing escrows unwind** on the old contract (release, refund, etc.) until empty.
5. Optionally **unset** deprecation with `set_deprecated(false, None)` if you need to re-open (e.g. for testing); in production, deprecation is typically left on.

## Behavior Summary

| Action                    | When not deprecated | When deprecated      |
|---------------------------|---------------------|----------------------|
| New lock / register       | Allowed             | **Blocked** (error)  |
| Release / refund / unwind | Allowed             | Allowed              |
| View deprecation status   | Allowed             | Allowed              |
| Set deprecation           | Admin only          | Admin only           |

Funds are never locked by deprecation alone; only new operations are disabled.
