# Storage Key Namespace Isolation Audit

## Overview

This document is the security audit for storage key namespace collisions between
`contracts/program-escrow/` and `contracts/bounty_escrow/contracts/escrow/`
(issue #1284).

---

## Soroban Storage Isolation Guarantee

On Soroban, **every contract has its own isolated storage namespace**. The ledger
entry key for any storage operation is:

```
ledger_key = Hash(contract_id || xdr_encoded(DataKey))
```

Because `contract_id` is unique per deployment, two contracts that use the same
`DataKey` variant (e.g. `DataKey::Admin`) will produce **different ledger keys**
and therefore access **completely disjoint storage entries**.

This means:
- A write to `DataKey::Admin` in `program-escrow` does NOT affect `DataKey::Admin`
  in `bounty_escrow`, even if both are deployed in the same Soroban sandbox.
- There is no mechanism by which one contract can read or write another contract's
  storage directly (cross-contract calls are the only interaction path).

---

## Shared Variant Names

Both contracts define `DataKey` enums with overlapping variant names. The table
below lists every shared name and confirms isolation:

| Variant Name | program-escrow type | bounty_escrow type | Isolated? |
|---|---|---|---|
| `Admin` | `Address` | `Address` | ✅ Yes |
| `PauseFlags` | `PauseFlags` struct | `PauseFlags` struct | ✅ Yes |
| `MaintenanceMode` | `bool` | `bool` | ✅ Yes |
| `PendingAdmin` | `Address` | `Address` | ✅ Yes |
| `ReentrancyGuard` | `u32` | `u32` | ✅ Yes |
| `MultisigConfig(String)` | per-program config | global config | ✅ Yes |
| `ClaimWindow` | `u64` | `u64` | ✅ Yes |
| `FeeConfig` | `FeeConfig` struct | `FeeConfig` struct | ✅ Yes |
| `Metadata(key)` | per-program metadata | per-bounty metadata | ✅ Yes |
| `Version` | `u32` | `u32` | ✅ Yes |
| `PendingClaim(key)` | `(String, u64)` | `u64` | ✅ Yes |
| `ChainId` | `String` | `String` | ✅ Yes |
| `NetworkId` | `String` | `String` | ✅ Yes |

---

## Key Prefix Convention

### program-escrow

All parameterised keys use `String` (program_id) as the discriminant:

```
Program(String)           → per-program ProgramData
ReleaseSchedule(String, u64) → per-program, per-schedule
MultisigConfig(String)    → per-program multisig
SpendingConfig(String)    → per-program spend limits
Metadata(String)          → per-program metadata
IdempotencyKey(String)    → per-payout idempotency
```

Singleton keys (no parameter): `Admin`, `PauseFlags`, `MaintenanceMode`,
`TokenAllowlist`, `FeeConfig`, `ReentrancyGuard`, `ClaimWindow`, etc.

### bounty_escrow

All parameterised keys use `u64` (bounty_id) as the discriminant:

```
Escrow(u64)               → per-bounty EscrowData
Metadata(u64)             → per-bounty metadata
EscrowFreeze(u64)         → per-bounty freeze record
RefundApproval(u64)       → per-bounty refund approval
PendingClaim(u64)         → per-bounty claim record
```

Singleton keys: `Admin`, `Token`, `Version`, `FeeConfig`, `PauseFlags`,
`MaintenanceMode`, `ReentrancyGuard`, `ClaimWindow`, etc.

---

## Audit Findings

### Finding 1: No namespace collisions possible

**Severity:** Informational (no vulnerability)

Soroban's per-contract storage isolation makes cross-contract key collisions
structurally impossible. Even if both contracts use `DataKey::Admin`, the
underlying ledger entries are at different keys.

### Finding 2: Shared variant names are safe

**Severity:** Informational (no vulnerability)

The shared variant names (`Admin`, `PauseFlags`, etc.) are a natural consequence
of both contracts implementing similar governance patterns. They do not create
any security risk.

### Finding 3: Cross-contract calls are the only interaction path

**Severity:** Informational (no vulnerability)

The only way `program-escrow` and `bounty_escrow` can interact is through
explicit cross-contract calls (e.g. via the `escrow-view-facade`). These calls
are audited separately in `docs/security/view-facade-safety.md`.

---

## Test Coverage (issue #1284)

Tests in `contracts/program-escrow/src/storage_collision_tests.rs`:

| Test | Property verified |
|------|-------------------|
| `test_admin_key_isolated_between_contracts` | Admin keys are independent |
| `test_pause_flags_isolated_between_contracts` | PauseFlags are independent |
| `test_maintenance_mode_isolated_between_contracts` | MaintenanceMode is independent |
| `test_reentrancy_guard_isolated_between_contracts` | ReentrancyGuard is independent |
| `test_program_data_key_isolated_between_contracts` | Program data is independent |
| `test_same_program_id_in_two_contracts_is_independent` | Same key name → different data |
| `test_token_allowlist_isolated_between_contracts` | TokenAllowlist is independent |
| `test_token_allowlist_enforcement_independent_per_contract` | Enforcement is per-contract |
| `test_schema_version_markers_isolated_between_contracts` | Schema markers are independent |
| `test_fee_config_isolated_between_contracts` | FeeConfig is independent |
| `test_pending_admin_isolated_between_contracts` | PendingAdmin is independent |
| `test_claim_window_isolated_between_contracts` | ClaimWindow is independent |
| `test_read_only_mode_isolated_between_contracts` | ReadOnlyMode is independent |
| `test_concurrent_independent_state` | Concurrent writes don't interfere |
| `test_program_registry_isolated_between_contracts` | Program registry is independent |
| `test_soroban_storage_isolation_proof` | Definitive isolation proof |
| `test_all_shared_variant_names_are_isolated` | All shared names verified |
