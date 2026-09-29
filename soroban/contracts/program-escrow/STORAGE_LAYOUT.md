# Grainlify Program Escrow Storage Layout

This document defines the definitive storage layout for the `program-escrow` contract. All state mutations and future upgrades must preserve compatibility with this layout or include explicit data migration logic.

## Storage Schema Version: 2

Circuit breaker storage uses its own marker: `CIRCUIT_BREAKER_SCHEMA_VERSION_V2 = 2`.

Below are all storage keys utilized by the contract.

| Key | Variant/Constant | Tier | Type | Notes |
|-----|-----------------|------|------|-------|
| `DataKey::Admin` | `Admin` | Instance | `Address` | Set once at init |
| `DataKey::Token` | `Token` | Instance | `Address` | Token contract address |
| `DataKey::Program(u64)` | `Program(program_id)` | Instance | `ProgramData` | Per-program configuration and state |
| `DataKey::ProgramJurisdiction(u64)` | `ProgramJurisdiction(program_id)` | Persistent | `JurisdictionConfig` | Jurisdiction config stored separately from main program record |
| `DataKey::ProgramIndex` | `ProgramIndex` | Instance | `u64` | Stable index used by `get_programs` and `get_program_count` |
| `DataKey::DeprecationState` | `DeprecationState` | Instance | `DeprecationState` | State tracking deprecation |
| `DataKey::LabelConfig` | `LabelConfig` | Persistent | `LabelConfig` | Label configuration |
| `DataKey::ReentrancyGuard` | `ReentrancyGuard` | Instance | `ReentrancyGuard` | Reentrancy protection state |
| `// Ownership transfer` | | | | |
| `DataKey::PendingAdmin` | `PendingAdmin` | Instance | `Address` | Pending admin transfer |
| `DataKey::ProgramPendingAdmin(u64)` | `ProgramPendingAdmin(program_id)` | Instance | `Address` | Pending admin for specific program |

## Migration Rules
- When a type definition changes, the `STORAGE_SCHEMA_VERSION` constant within `lib.rs` MUST be incremented.
- Upgrades must provide a migration path that reads the old struct format and writes the new one, or leave old struct variants and add V2 keys.
- Deleting an `Instance` tier key can cause `verify_storage_layout` to fail.