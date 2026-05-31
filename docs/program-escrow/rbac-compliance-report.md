# RBAC Permission Audit Report for Program Escrow

This document describes the new audit query for program delegate permissions in the `program-escrow` contract.

## Purpose

Compliance teams require a point-in-time snapshot of all currently granted delegate permissions for a program. The new view function exposes a clean, read-only report of a program's active delegate and permission bitmask.

## New Interface

### `ProgramEscrowContract::query_all_delegates(env, program_id) -> Vec<ProgramDelegateInfo>`

- `env` — the Soroban environment
- `program_id` — the program identifier to query
- returns a vector of active delegate records

The returned vector contains at most one `ProgramDelegateInfo`, because this contract stores a single delegate and permission bitmask per program.

### `EscrowViewFacade::query_all_delegates(env, program_contract, program_id) -> Vec<ProgramDelegateInfo>`

- `program_contract` — on-chain address of a `ProgramEscrow` contract
- `program_id` — the program identifier to query
- returns a vector of delegate audit records from the target contract

## `ProgramDelegateInfo`

Fields:

- `program_id: String`
- `delegate: Option<Address>`
- `permissions: u32`

A missing or revoked delegate is represented by an empty result set.

## Security Notes

- This is a read-only query and does not modify contract state.
- The facade wrapper uses `try_query_all_delegates` and returns an empty list on contract call failure, preserving safe audit semantics.
- No authorization is required for read access.

## Example Usage

```rust
let delegates = ProgramEscrowContract::query_all_delegates(env.clone(), program_id.clone());
for delegate_info in delegates.iter() {
    // Inspect delegate_info.delegate and delegate_info.permissions
}
```

```rust
let facade_delegates = EscrowViewFacadeClient::new(&env, &facade_contract)
    .query_all_delegates(&program_contract, &program_id);
```
