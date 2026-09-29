# Deployable Artifacts Inventory

This document lists every deployable WebAssembly (wasm) artifact produced by this repository, its source crate, and its workspace.

| Artifact | Source Crate | Workspace | Status |
|---|---|---|---|
| `bounty_escrow.wasm` | `contracts/bounty_escrow/contracts/escrow` | `contracts/bounty_escrow` | Deployed |
| `escrow_view_facade.wasm` | `contracts/escrow-view-facade` | `contracts` | Staged |
| `grainlify_core.wasm` | `contracts/grainlify-core` | `contracts` | Deployed |
| `program_escrow.wasm` | `contracts/program-escrow` | `contracts` | Staged |
| `view_facade.wasm` | `contracts/view-facade` | `contracts` | Staged |
| `escrow.wasm` | `soroban/contracts/escrow` | `soroban` | Superseded |
| `soroban_program_escrow.wasm` | `soroban/contracts/program-escrow` | `soroban` | Superseded |

## Smoke Deploy Coverage and Pass Condition

The smoke deploy is executed by `scripts/smoke-deploy.sh`. It is run in CI on a stated cadence and its pass condition is explicit.

### CI Cadence

The smoke deploy runs on every push to `main` and on a nightly schedule (`cron: '0 3 * * *'`). This ensures the deployment path is exercised regularly and not only when a developer remembers to run it locally.

### Pass Condition

The smoke deploy passes only when all of the following are true for every artifact in the inventory above:

1. The artifact builds successfully from its source crate.
2. The resulting wasm file exists at the expected path and is non-empty.
3. The artifact deploys to the target network without error.
4. The deployed contract is invokable and returns the expected response for a known query.

A zero exit code alone is not sufficient: the script must report, per artifact, whether each of the above checks passed.

### Actionable Failure Output

When a check fails, the script emits a line identifying the artifact, the failed check, and the reason. For example:

```
FAIL bounty_escrow.wasm: build failed -- cargo build exited with code 101
```

This makes a failure actionable from the output alone, without needing to re-run the script locally or inspect CI logs for context.

### Validation

To validate the smoke deploy, break a deployable artifact (e.g., introduce a compile error in its source crate) and confirm the smoke deploy reports which artifact failed and why.
