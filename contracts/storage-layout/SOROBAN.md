# Soroban workspace storage layout snapshots

These snapshots pin every `#[contracttype]` encoding under `soroban/contracts/*`
so CI fails when a persisted type is added, removed, reordered or retyped.

| Package | Snapshot | Sources |
|---------|----------|---------|
| `escrow` | `soroban-escrow.json` | `soroban/contracts/escrow/src/lib.rs`, `soroban/contracts/escrow/src/identity.rs` |
| `soroban-program-escrow` | `soroban-program-escrow.json` | `soroban/contracts/program-escrow/src/lib.rs` |
| `grainlify-stream` | `soroban-stream.json` | none (gas-regression fixture crate; no persisted `#[contracttype]` types) |

## Regenerating (documented command)

After an intentional layout change to a soroban contract:

```bash
python3 contracts/scripts/gen_storage_layout_types.py \
  contracts/storage-layout/soroban-escrow.json \
  contracts/storage-layout/soroban-program-escrow.json \
  contracts/storage-layout/soroban-stream.json
```

Then update `contracts/storage-layout-manifest.json` if a package was added, and
run the same verify command CI runs:

```bash
python3 scripts/verify-storage-layout-manifest.py --require-soroban
```

CI (`.github/workflows/storage-layout.yml`) triggers on `soroban/**` changes and
fails when any `#[contracttype]` under `soroban/contracts/*/src` is missing from
a soroban snapshot, or when a recorded field encoding no longer matches source.
