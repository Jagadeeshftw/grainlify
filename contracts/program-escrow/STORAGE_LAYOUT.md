# Program Escrow Storage Layout

This document defines the definitive storage layout for the `program-escrow` contract. All state mutations and future upgrades must preserve compatibility with this layout or include explicit data migration logic.

## Storage Schema Version: 1

Circuit breaker storage uses its own marker: `CIRCUIT_BREAKER_SCHEMA_VERSION_V2 = 2`.

Below are all storage keys utilized by the contract.

| Key | Variant/Constant | Tier | Type | Notes |
|-----|-----------------|------|------|-------|
| `DataKey::Admin` | `Admin` | Instance | `Address` | Set once at init |
| `DataKey::PauseFlags` | `PauseFlags` | Instance | `PauseFlags` | Granular pause per-operation |
| `DataKey::MaintenanceMode` | `MaintenanceMode` | Instance | `bool` | Blocks lock mutations |
| `DataKey::ReadOnlyMode` | `ReadOnlyMode` | Instance | `bool` | Prevents writes during indexer backfills |
| `DataKey::RateLimitConfig` | `RateLimitConfig` | Instance | `RateLimitConfig` | |
| `DataKey::ClaimWindow` | `ClaimWindow` | Instance | `u64` | |
| `DataKey::Program(String)` | `Program(program_id)` | Instance | `ProgramData` | Per-program configuration and state |
| `DataKey::MultisigConfig(String)` | `MultisigConfig(program_id)` | Persistent | `MultisigConfig` | |
| `DataKey::ReleaseSchedule(String, u64)` | `ReleaseSchedule(program_id, schedule_id)` | Persistent | `ProgramReleaseSchedule` | |
| `DataKey::ReleaseHistory(String)` | `ReleaseHistory(program_id)` | Instance | `Vec<ProgramReleaseHistory>` | |
| `DataKey::PendingClaim(String, u64)` | `PendingClaim(program_id, schedule_id)` | Persistent | `ClaimRecord` | |
| `DataKey::ProgramDependencies(String)` | `ProgramDependencies(program_id)` | Instance | `Vec<String>` | |
| `DataKey::DependencyStatus(String)` | `DependencyStatus(program_id)` | Instance | `DependencyStatus` | |
| `PROGRAM_DATA` | (Symbol) | Instance | `ProgramData` | Legacy single-program key |
| `RECEIPT_ID` | (Symbol) | Instance | `u64` | Monotone receipt counter |
| `SCHEDULES` | (Symbol) | Instance | `Vec<ProgramReleaseSchedule>` | Release schedules list |
| `RELEASE_HISTORY` | (Symbol) | Instance | `Vec<ProgramReleaseHistory>`| Release history list |
| `NEXT_SCHEDULE_ID` | (Symbol) | Instance | `u64` | Next schedule id counter |
| `PROGRAM_INDEX` | (Symbol) | Instance | `Vec<String>` | Program index list |
| `AUTH_KEY_INDEX` | (Symbol) | Instance | `Vec<Address>` | Auth key index |
| `FEE_CONFIG` | (Symbol) | Instance | `FeeConfig` | Fee configuration |
| `PROGRAM_REGISTRY` | (Symbol) | Instance | `Vec<String>` | Registry of program ids |
| `CircuitBreakerKey::ErrorLog` | `ErrorLog` | Persistent | `Vec<ErrorEntry>` | Hot circuit failure log capped at 50 entries |
| `CircuitBreakerKey::ErrorArchive(String)` | `ErrorArchive(program_id)` | Persistent | `CompactFailureArchive` | Per-program compact archive of pruned failure timestamps, error codes, and failure counts |

## Migration Rules
- When a type definition changes, the `STORAGE_SCHEMA_VERSION` constant within `lib.rs` MUST be incremented.
- Upgrades must provide a migration path that reads the old struct format and writes the new one, or leave old struct variants and add V2 keys.
- Deleting an `Instance` tier key can cause `verify_storage_layout` to fail.
