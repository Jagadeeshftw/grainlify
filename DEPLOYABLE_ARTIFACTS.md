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

## Smoke Deploy Coverage

The smoke deploy (`scripts/smoke-deploy.sh`) is the executable check that every artifact above is buildable and deployable. The cadence, pass condition, and failure output contract are defined below.

### Cadence

The smoke deploy runs on a schedule in CI via `.github/workflows/smoke-deploy.yml`:

- Pushes to the default branch that touch any artifact source or the smoke script.
- Pull requests targeting the default branch that touch any artifact source or the smoke script.
- A scheduled nightly run (`cron : '0 3 * * *'`) so drift in dependencies or toolchain is caught without a code change.
- Manual dispatch (`workflow_dispatch`) for on-demand reruns.

### Pass Condition

The smoke deploy passes only when every artifact in the inventory above is built and deployed to a fresh local network and the deployed contract responds to a minimal invocation. Specifically:

1. Every artifact listed in the inventory builds to a non-empty `.wasm` file with a valid wasm magic header.
2. Every built artifact deploys to the local network without error.
3. Every deployed contract returns a successful response to a minimal invocation.
4. The script exits non-zero if any artifact fails any of the above checks.

A zero exit code alone is not the pass condition; the script must report a per-artifact status table and fail if any row is not `PASS`.

### Failure Output

On failure the script prints, for each failing artifact:

- The artifact name and source crate.
- The phase that failed (`build`, `deploy`, or `invoke`).
- The captured stderr from the failing command.
- The exit code of the failing command.

This makes a failure actionable from the output alone, without needing to re-run the script locally.

### Validation

To verify the smoke deploy covers every deployable artifact, break one artifact (for example, introduce a compile error in `contracts/grainlify-core`) and confirm the smoke deploy reports the failing artifact name, the failing phase, and the underlying error before exiting non-zero.
