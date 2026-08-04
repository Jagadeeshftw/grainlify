# Migration System — grainlify-core

This document describes the `migrate` entrypoint and `MigrationState` tracking in the `grainlify-core` contract.

---

## Overview

The migration system provides a secure, admin-controlled mechanism for evolving on-chain state across contract versions. It is designed to be:

- **Idempotent** — calling `migrate` with the same `target_version` more than once is a safe no-op
- **Monotonic** — version numbers only increase; downgrades are rejected
- **Auditable** — every migration attempt emits an on-chain event and persists a `MigrationState` record
- **Chained** — a single call can traverse multiple version boundaries (e.g. v1 → v3 executes v1→v2 then v2→v3)

---

## Entrypoints

### `migrate(target_version: u32, migration_hash: BytesN<32>)`

Executes state migration from the current version to `target_version`.

| Parameter | Type | Description |
|---|---|---|
| `target_version` | `u32` | Version to migrate to. Must be strictly greater than the current version. |
| `migration_hash` | `BytesN<32>` | SHA-256 hash of migration data for off-chain verification and audit trail. |

**Authorization:** Admin only (`admin.require_auth()`).

**Idempotency:** If `MigrationState.to_version == target_version` already exists in storage, the call returns immediately without re-executing migrations or emitting events.

**Chaining:** If `target_version > current_version + 1`, intermediate migrations are executed in sequence:
- v1 → v3 calls `migrate_v1_to_v2` then `migrate_v2_to_v3`

**Panics:**
- `"Target version must be greater than current version"` — if `target_version <= current_version`
- `"No migration path available"` — if no migration function exists for a version step (e.g. v3 → v4)
- Auth failure — if caller is not the admin

---

### `get_migration_state() -> Option<MigrationState>`

Returns the persisted migration state, or `None` if no migration has been executed.

---

## Data Types

### `MigrationState`

Stored at `DataKey::MigrationState` (instance storage, persists across WASM upgrades).

```rust
pub struct MigrationState {
    pub from_version: u32,       // Version before migration
    pub to_version: u32,         // Version after migration
    pub migrated_at: u64,        // Ledger timestamp of migration
    pub migration_hash: BytesN<32>, // Caller-supplied verification hash
}
```

### `MigrationEvent`

Emitted on every `migrate()` call (topic: `symbol_short!("migration")`).

```rust
pub struct MigrationEvent {
    pub from_version: u32,
    pub to_version: u32,
    pub timestamp: u64,
    pub migration_hash: BytesN<32>,
    pub success: bool,
    pub error_message: Option<String>,
}
```

Failed migrations (e.g. invalid target version) also emit an event with `success: false` before panicking, enabling off-chain monitors to detect and alert on failed attempts.

---

## Storage Keys

| Key | Type | Description |
|---|---|---|
| `DataKey::MigrationState` | `MigrationState` | Persisted after each successful migration |
| `DataKey::Version` | `u32` | Updated to `target_version` after migration |
| `DataKey::Admin` | `Address` | Read for authorization; never modified by migrate |

All keys use instance storage and persist across WASM upgrades.

> **IMPORTANT:** Storage key enum variants must never be renamed or removed between contract versions. Doing so breaks access to existing on-chain data.

---

## Supported Migration Paths

| From | To | Function |
|---|---|---|
| v1 | v2 | `migrate_v1_to_v2` (no-op placeholder) |
| v2 | v3 | `migrate_v2_to_v3` (no-op placeholder) |

To add a new migration path (e.g. v3 → v4):
1. Implement `fn migrate_v3_to_v4(env: &Env)` with the required state transformations
2. Add `4 => migrate_v3_to_v4(&env)` to the `match next_version` block in `migrate()`
3. Increment `VERSION` constant
4. Write tests covering the new path, idempotency, and chaining

---

## Upgrade Workflow

```
1. Develop and test new contract version locally
2. cargo build --release --target wasm32-unknown-unknown
3. stellar contract install --wasm <path/to/new.wasm> --source ADMIN_KEY
   → returns NEW_WASM_HASH
4. stellar contract invoke --id CONTRACT_ID --source ADMIN_KEY \
     -- upgrade --new_wasm_hash NEW_WASM_HASH
5. stellar contract invoke --id CONTRACT_ID --source ADMIN_KEY \
     -- migrate --target_version 3 --migration_hash <sha256_of_migration_data>
6. Verify: stellar contract invoke -- get_migration_state
```

Steps 4 and 5 are independent. `upgrade` replaces the WASM; `migrate` transforms the state. Both require admin authorization.

---

## Security Assumptions

1. **Admin key security** — The admin address is immutable after `init_admin`. Compromise of the admin key allows arbitrary migrations. Use a hardware wallet or multi-sig for production.
2. **Replay protection** — `migration_hash` is stored on-chain. Off-chain tooling should verify this hash against the expected migration script before and after execution.
3. **Version monotonicity** — The contract enforces forward-only migrations. There is no on-chain rollback of state; rollback requires deploying a previous WASM and manually reversing state changes.
4. **Idempotency scope** — Idempotency is per `to_version`. A migration to v3 followed by a migration to v4 are two distinct operations, each recorded separately.
5. **No key mutations** — Migration functions must not rename or remove `DataKey` variants. New storage keys must use new enum variants.
6. **Soroban atomicity and ledger timestamp monotonicity** — Soroban transactions are all-or-nothing. If a contract panics, every storage write in that execution is discarded. This means `migrate()` cannot consume a commitment on expiry failure: the `remove` executed before the panic is rolled back. On networks with monotonic timestamps (Stellar mainnet), this is not a problem because a commitment that expires at `T=4600` can never become valid again when time only moves forward. On test networks where timestamps periodically reset, a failed expiry check leaves the commitment in storage and a subsequent reset can make it valid again. The migration system does **not** include end-to-end defense-in-depth against this scenario; admins on test networks must treat expiry failures as requiring a fresh `commit_migration`.

### Replay Protection Mechanisms

The contract implements the following replay protections:

#### Primary: Commitment Consumption on Success

After a successful migration, the commitment is always consumed:

```rust
// Consume commitment (replay protection)
env.storage().instance().remove(&DataKey::MigrationCommitment(target_version));
```

This prevents replay of the same migration, even when `expires_at = 0`.

#### Secondary: Migration Hash Verification

The migration hash check is independent of timestamp checks:

```rust
if commitment.hash != migration_hash {
    panic!("{}", ContractError::MigrationHashMismatch as u32);
}
```

This means:
- A hash committed for version 3 cannot be used to migrate to version 4 (version isolation)
- A different hash cannot be used with an existing commitment (hash binding)
- The hash check works even when `expires_at = 0` (no expiry)

#### Tertiary: Idempotency Guard

If `MigrationState.to_version == target_version` already exists, `migrate()` returns immediately without re-executing migrations. This prevents replay of a migration that has already completed successfully.

### Expiry Semantics and Testnet Limitations

`expires_at` is an absolute ledger timestamp (seconds). `migrate()` rejects any call where the current ledger timestamp exceeds `expires_at`. This provides a *soft* time-bound window on networks with monotonic timestamps.

However, because Soroban rolls back all state changes when the contract panics, the removal of an expired commitment is not persisted. On test networks that reset ledger timestamps backward, a commitment that failed the expiry check may still be present and can become valid again after the reset.

On Stellar mainnet, ledger timestamps are strictly monotonic and this scenario cannot occur. On test networks, admins should:

- Understand that an expired commitment must be re-committed after a testnet reset
- Prefer short expiry TTLs to limit exposure
- Monitor for `"Migration commitment has expired"` panics and re-commit as needed

### Trust Assumptions by Environment

| Environment | Timestamp Monotonicity | Expiry Protection | Recommended Practice |
|-------------|------------------------|-------------------|----------------------|
| Stellar Mainnet | ✅ Strictly monotonic | Works as intended | Use `expires_at` for time-bound authorizations |
| Stellar Testnet | ❌ Periodic resets | Not persisted after failure | Re-commit after expiry failures or set `expires_at = 0` |
| Local Development | ⚠️ Depends on harness | Mirrors testnet behavior | Use short TTLs or `expires_at = 0` |

### Recommended Practices

1. **Always verify migration_hash** off-chain before and after migration execution — this is the strongest replay protection
2. **Never reuse a migration hash** across different target versions or migration attempts
3. **Treat expiry failures as requiring a fresh commitment** — a failed `migrate()` due to expiry does not consume the commitment in Soroban
4. **Use short expiry TTLs** in test environments to minimize the window for replay
5. **Use `expires_at = 0`** (no expiry) only in controlled test environments where timestamp behavior is managed
6. **Monitor expiry events** — if a migration fails due to expiry, re-commit with a new hash before retrying

---

## Testing

Run the full test suite:

```bash
# From contracts/grainlify-core/
cargo test --lib
```

Key test modules:
- `src/migration_hook_tests.rs` — idempotency, auth, event emission, edge cases
- `src/test_migration_replay.rs` — replay protection, ledger-timestamp reset safety, hash-only replay prevention
- `src/test/e2e_upgrade_migration_tests.rs` — end-to-end lifecycle scenarios
- `src/lib.rs` (inline `mod test`) — integration and regression tests
