# Deployable Artifacts Inventory

This document lists every deployable WebAssembly (wasm) artifact produced by this repository, its source crate, and its workspace.

| Artifact | Source Crate | Workspace | Status |
||---|---|---|---|
| `bounty_escrow.wasm` | `contracts/bounty_escrow/contracts/escrow` | `contracts/bounty_escrow' | Deployed |
| `escrow-view-facade.wasm` | `contracts/escrow-view-facade` | `contracts` | Staged |
| `grainlify_core.wasm` | `contracts/grainlify-core` | `contracts` | Deployed |
| `program-escrow.wasm` | `contracts/program-escrow` | `contracts` | Staged |
| `view-facade.wasm` | `contracts/view-facade` | `contracts` | Staged |
| `escrow.wasm` | `soroban/contracts/escrow` | `soroban` | Superseded |
| `soroban_program_escrow.wasm` | `soroban/contracts/program-escrow` | `soroban` | Superseded |

## Smoke Deploy Coverage

The smoke deploy is executed by `scripts/smoke-deploy.sh` and is run on a stated cadence in CI. The cadence and the explicit pass condition are defined below so that a failure is actionable from the script output alone.

### Cadence

The smoke deploy runs on every push to `main` and on a nightly schedule (`0 2 * * *`). This ensures the deployable artifacts are exercised regularly and not only when a maintainer manually runs the script.

### Pass Condition

A smoke deploy passes only when all of the following hold for every artifact in the inventory:

1. The artifact exists at the expected path and is non-empty.
2. The artifact is a valid wasm binary (the `wasm` magic bytes are present).
3. The artifact can be deployed to the target network and the deployment returns a contract ID.
4. The deployed contract responds to a minimal read/metadata call without error.

A zero exit code alone is not sufficient; the script must report the per-artifact result and fail if any check above fails.

### Actionable Failure Output

When a check fails, the script must name the failing artifact, the check that failed, and the underlying error or stderr output. Example:

```
FAIL bounty_escrow.wasm: wasm magic bytes not found (expected 0061sm01 at offset 0)
FAIL grainlify_core.wasm: deploy failed: host error: insufficient balance
```

### Inventory Coverage

The smoke deploy must cover every artifact listed in the inventory table above. A new artifact added to the inventory must be added to the smoke deploy in the same change.

## Validation

To validate the smoke deploy, break a deployable artifact (e.g., truncate its bytes or corrupt its wasm magic) and confirm the smoke deploy reports which artifact failed and why.
