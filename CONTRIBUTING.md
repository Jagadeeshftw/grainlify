# Contributing to Grainlify

This guide is the starting point for choosing the right code tree and reproducing CI locally. Run commands from the repository root unless a command explicitly changes directory. CI runs on Linux; on Windows, use Git Bash or WSL for the Bash scripts and shell commands below.

## Choose the right tree

There are **three Cargo workspaces** in two Soroban contract trees. Do not choose a crate by its name alone: there are separate crates named `escrow` and similar program-escrow packages.

| Change                                                                  | Make it here                               | Notes                                                                                                                                                                   |
| ----------------------------------------------------------------------- | ------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Production bounty escrow behavior, tests, or deployable escrow WASM     | `contracts/bounty_escrow/contracts/escrow` | Authoritative escrow; package `bounty-escrow`, SDK 21.7.7, workspace root `contracts/bounty_escrow`. This alone produces the deployable `bounty_escrow.wasm`.           |
| Production program funds, payouts, recovery, or payout API              | `contracts/program-escrow`                 | Authoritative program escrow; package `program-escrow`, SDK 21.7.7. This alone produces the deployable `program_escrow.wasm`.                                           |
| Shared contract types, governance, upgrade, storage, or feature support | `contracts/grainlify-core`                 | Shared by production contracts and facades.                                                                                                                             |
| Read-only program registry/query API                                    | `contracts/view-facade`                    | Program-focused registry facade. Keep mirrored ABI types synchronized.                                                                                                  |
| Read-only bounty summaries and portfolio API                            | `contracts/escrow-view-facade`             | Bounty/program query facade; update its local bindings when a canonical contract type changes.                                                                          |
| Supporting Rust utility library/CLI or root contract tooling            | `contracts/`                               | This is also its own workspace and root package, not just a directory containing the other crates.                                                                      |
| Protocol-23 experiment, migration work, or stream fixture               | `soroban/contracts/*`                      | Separate SDK 23.4.1 workspace. Both its `escrow` and `program-escrow` crates are superseded/reference-only and must not be deployed.                                    |
| TypeScript client SDK                                                   | `contracts/sdk`                            | Separate npm package; use its own lockfile and CI commands below.                                                                                                       |
| Repository-wide architecture, security, deployment, or contributor docs | `docs/` or the matching root guide         | Keep operational facts aligned with the owning code/workspace.                                                                                                          |

### Two trees and SDK versions

`contracts/` is the production contract tree and currently uses the exact Soroban SDK pin `=21.7.7` (protocol 21). It contains both the `contracts` Cargo workspace and the nested `contracts/bounty_escrow` workspace; the nested workspace is excluded from the outer workspace. `soroban/` is an independent protocol-23 tree pinned to `=23.4.1`. These two SDK majors are an intentional, owned migration exception—not permission to add another version. **Never add a Cargo path dependency across these trees.** Read [contracts/SDK_COMPATIBILITY.md](contracts/SDK_COMPATIBILITY.md) before changing SDK pins.

The names `escrow` and `program-escrow` are not sufficient to establish authority. In particular, only `contracts/bounty_escrow/contracts/escrow` is the deployable bounty escrow; `soroban/contracts/escrow` is not. Likewise, only `contracts/program-escrow` is the deployable program escrow; `soroban/contracts/program-escrow` is not, and the two are not interchangeable — the soroban crate is a program registry/search contract with no payout logic, while the authoritative crate has no registry/search surface. See [the escrow authority note](docs/contracts/escrow-implementation-authority.md), [the program-escrow authority note](docs/contracts/program-escrow-implementation-authority.md), and [DEPLOYABLE_ARTIFACTS.md](DEPLOYABLE_ARTIFACTS.md).

## Local setup

Install Rust stable with `rustfmt` and `clippy`, plus both WebAssembly targets used in CI. The formatting workflow pins Rust 1.96.1; some reproducibility and size jobs pin 1.88.0. Install Python 3, Node.js 20/npm, and `jq`; Rust wasm checks require `wasm32-unknown-unknown` (and CI also installs `wasm32v1-none`). For coverage, install `llvm-tools-preview` and `cargo-llvm-cov`. The commands assume Bash, `git`, `rustup`, `cargo`, `python3`, `npm`, `jq`, and (for the separate legacy manifest-schema workflow) `ajv` are available.

## Reproduce CI locally

Run the gates relevant to the files you changed. For a full repository pass, run every applicable section below; CI workflows are path-filtered, so not every workflow runs for every PR. The workflows in `.github/workflows/` are authoritative if commands change.

### Formatting (all three workspaces)

CI runs Rustfmt 1.96.1 against each workspace:

```bash
(cd contracts && cargo +1.96.1 fmt --all -- --check)
(cd contracts/bounty_escrow && cargo +1.96.1 fmt --all -- --check)
(cd soroban && cargo +1.96.1 fmt --all -- --check)
```

### Smart-contract CI (`contracts-ci.yml`)

CI uses stable Rust, installs `wasm32v1-none` and `wasm32-unknown-unknown` plus Clippy, and runs these checks:

```bash
python3 scripts/check_inventory.py
bash contracts/scripts/ci-contracts.sh
cargo test --manifest-path contracts/program-escrow/Cargo.toml -- test_token_allowlist test_fot_routing
bash contracts/scripts/ci-batch-tests.sh
python3 scripts/check-program-escrow-no-traps.py
python3 scripts/check-program-escrow-no-traps.py --fixture scripts/tests/fixtures/program_escrow_unwrap_trap.rs --expect-fail
cargo test --manifest-path contracts/view-facade/Cargo.toml --test conformance --test dependency_gating
cargo test --manifest-path contracts/escrow-view-facade/Cargo.toml --test conformance --test dependency_gating
bash contracts/scripts/check-feature-matrix.sh
bash contracts/scripts/ci-grainlify-core.sh
```

`ci-contracts.sh` is the shared CI/local runner for its Rust contract build/test, Clippy, wasm-release, and related checks; `ci-batch-tests.sh` is intentionally a separate gate as well. The scripts require their documented Rust targets. The workflow also uploads the resulting bounty-escrow wasm.

### Soroban workspace CI (`soroban-ci.yml`)

Run these from `soroban/` (CI installs stable Rust, rustfmt, Clippy, and `wasm32-unknown-unknown`):

```bash
cd soroban
cargo fmt -p grainlify-stream -- --check
cargo clippy -p grainlify-stream --all-targets --locked -- -D warnings
cargo test -p grainlify-stream --locked
cargo build -p soroban-program-escrow --target wasm32-unknown-unknown --release --locked
```

The resulting `soroban_program_escrow.wasm` must also be at most 131072 bytes; the workflow checks this after the build. Note that `soroban-program-escrow` is superseded and **not deployable** — this gate only catches size regressions in a crate that is still tested. The deployable program escrow is `contracts/program-escrow`.

### Other CI gates

Run the matching commands when your changes touch the relevant code or artifacts:

```bash
# TypeScript SDK (.github/workflows/sdk-ci.yml); from contracts/sdk
cd contracts/sdk
npm ci
npm run build
npm test -- --runInBand
cd ../..

# Soroban SDK version and lockfile policy
./scripts/check_sdk_versions.sh
(cd contracts/grainlify-core && cargo test --locked --test test_sdk_pin_consistency)

# Storage-layout validation
(cd contracts/grainlify-core && cargo test --test test_storage_layout)
python3 scripts/verify-storage-layout-manifest.py

test -f contracts/program-escrow/STORAGE_LAYOUT.md

# Contract manifests and their Node tests
npm test --prefix contracts

# Wasm budgets: use the same pinned compiler as this workflow (Rust 1.88.0)
cargo +1.88.0 build --manifest-path contracts/bounty_escrow/Cargo.toml --workspace --target wasm32-unknown-unknown --release --locked
cargo +1.88.0 build --manifest-path contracts/grainlify-core/Cargo.toml --target wasm32-unknown-unknown --release --locked
cargo +1.88.0 build --manifest-path contracts/program-escrow/Cargo.toml --target wasm32-unknown-unknown --release --locked
cargo +1.88.0 build --manifest-path contracts/escrow-view-facade/Cargo.toml --target wasm32-unknown-unknown --release --locked
cargo +1.88.0 build --manifest-path contracts/view-facade/Cargo.toml --target wasm32-unknown-unknown --release --locked
bash scripts/check-wasm-budgets.sh

# Program-escrow reentrancy smoke gate
cargo test --locked --manifest-path contracts/program-escrow/Cargo.toml reentrancy_tests::

# Entry-point coverage map check
python3 scripts/check_coverage.py

# Contract coverage report and floor enforcement
./scripts/run_contract_coverage.sh
./scripts/check-coverage-thresholds.sh

# Bounty-escrow gas/resource threshold gate
(cd contracts/bounty_escrow/contracts/escrow && ESCROW_GAS_MODE=strict cargo test --features testutils gas_ci -- --nocapture --test-threads=1)

# Verify that build outputs are not tracked
tracked=$(git ls-files | grep -E '(^|/)target(-[^/]*|_([^/]*))?/' || true)
test -z "$tracked"
```

The legacy `.github/workflows/validate-manifests.yml` also uses globally installed `ajv-cli`/`ajv-formats`, `jq`, and Bash to validate each tracked `contracts/*-manifest.json` (excluding `storage-layout-manifest.json`), required manifest presence, structure, SemVer fields, and allowed authorization strings. Its schema/unit-test counterpart is `npm test --prefix contracts` above.

### Build reproducibility and E2E upgrades

`.github/workflows/build-reproducibility.yml` cleans and builds each deployable artifact twice, then compares the outputs byte-for-byte. Use its workspace-specific `cargo clean` and `cargo build --manifest-path … --workspace --target wasm32-unknown-unknown --release` (and standalone crate equivalents) twice, with `cmp`/SHA-256 comparison as in that workflow; run `scripts/verify-wasm-artifacts.sh <artifact-dir> <artifact.wasm>...` to check artifact presence, nonzero size, and wasm magic bytes. For upgrade behavior run `(cd contracts && make -f Makefile.e2e test-e2e-all)`; it needs the Stellar CLI and its configured test network/tools. See [Build and Validation Guide](contracts/Build_and_Validation_Guide.md) for contract-specific setup and E2E details.

## Adding a deployable contract crate

A new crate is not covered merely because its source exists. In the same change:

1. Choose the correct SDK tree and workspace. For `contracts/Cargo.toml`, add the crate to `members` and `default-members` (unless it is intentionally excluded); for the nested bounty-escrow and `soroban` workspaces, follow their existing `contracts/*` member globs and exclusions. Keep dependencies within one SDK tree.
2. Commit a `Cargo.lock` at the workspace root and add an exact SDK pin via the workspace dependency. Run `./scripts/check_sdk_versions.sh`; update [contracts/SDK_COMPATIBILITY.md](contracts/SDK_COMPATIBILITY.md), its guardrail test, and pin table together only for an approved SDK migration.
3. For each deployable `cdylib`, add a valid `*-manifest.json` under `contracts/` that satisfies `contracts/contract-manifest-schema.json`; ensure `npm test --prefix contracts` discovers it.
4. Add its artifact filename to [DEPLOYABLE_ARTIFACTS.md](DEPLOYABLE_ARTIFACTS.md). `python3 scripts/check_inventory.py` checks deployable `cdylib` crates against that inventory.
5. Register it in `.github/coverage-thresholds.json` with a measured floor or a documented blocked reason, and in `.github/wasm-budgets.json` with its package, target directory, and budget. Update the build-reproducibility workflow's artifact list/build matrix and applicable storage-layout fixtures/gates.
6. Add a focused test/build gate in the workflow or shared CI script for the owning workspace. Run that gate and the full relevant sections above before opening a PR.

## Change and review notes

- Contract storage fields, error codes, event schemas, or public ABI changes can affect deployed state and downstream clients. Check storage-layout fixtures, serialization goldens, manifests, and facade bindings; update all mirrored copies in the same PR. See [docs/abi-stability-matrix.md](docs/abi-stability-matrix.md).
- Do not commit Cargo `target/`, generated build outputs, coverage output, or local secrets. Keep changes focused and include the test commands/results in your PR description.
