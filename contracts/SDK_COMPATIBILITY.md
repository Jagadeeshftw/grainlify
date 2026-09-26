# Soroban SDK Compatibility Policy

Status: authoritative for the repository SDK **target** and per-tree pin state (issues #1743, #1848).
Related: [VERSIONS.md](VERSIONS.md) for contract code versioning,
[docs/abi-stability-matrix.md](../docs/abi-stability-matrix.md) for ABI stability,
[SERIALIZATION_COMPATIBILITY_TESTS.md](SERIALIZATION_COMPATIBILITY_TESTS.md) for the
XDR golden-test policy.

## Repository target

| Field | Value |
|---|---|
| **Target `soroban-sdk`** | `=23.4.1` (protocol **23**) |
| **Why this target** | The `soroban/` workspace already builds and locks against 23.4.1; protocol 23 is the forward host/ABI the repo is moving toward. |
| **Authoritative document** | This file. The CI gate (`scripts/check_sdk_versions.sh`) and `test_sdk_pin_consistency` must stay in sync with the tables below. |

Every tree is either **on target** or an **owned exception**. A third SDK major is never allowed.

## Tree status (migration board)

| Tree / manifest root | Current pin | Status | Owner | Notes |
|---|---|---|---|---|
| `soroban/` (workspace: escrow, soroban-program-escrow, stream) | `=23.4.1` | **On target** | `@Jagadeeshftw` | Protocol-23 sandbox / next-line contracts. |
| `contracts/` (grainlify-contracts) | `=21.7.7` | **Exception — migrating** | `@Jagadeeshftw` | Deployable production tree; stays on protocol 21 until the pin-move checklist below is green. |
| `contracts/bounty_escrow` (workspace) | `=21.7.7` | **Exception — migrating** | `@Jagadeeshftw` | Same production pin as `contracts/`. |
| `contracts/grainlify-core` | `=21.7.7` | **Exception — migrating** | `@Jagadeeshftw` | Anchor of the production graph. |
| `contracts/program-escrow` | `=21.7.7` | **Exception — migrating** | `@Jagadeeshftw` | |
| `contracts/view-facade` | `=21.7.7` | **Exception — migrating** | `@Jagadeeshftw` | |
| `contracts/escrow-view-facade` | `=21.7.7` | **Exception — migrating** | `@Jagadeeshftw` | |

### Owned exception (why two majors exist)

**Reason:** The deployable `contracts/` tree is frozen on `=21.7.7` (protocol 21) because gas measurements, serialization goldens, storage-layout guardrails, and E2E upgrade suites are proven against that pin. The `soroban/` workspace intentionally tracks the repository **target** (`=23.4.1`) so protocol-23 work can proceed without dragging production into an unproven host.

**Owner:** `@Jagadeeshftw` (repository maintainer) — responsible for either completing the `contracts/` → 23.4.1 migration or explicitly renewing this exception when the target moves.

**Hard rule:** The two worlds must never meet in one dependency graph. Cross-tree `path` dependencies between `soroban/` and `contracts/` are forbidden (the historical violation was `soroban/contracts/escrow` path-depending on `grainlify-core`, which mixed SDK 21 and 23 in `soroban/Cargo.lock`).
| Manifest root | soroban-sdk pin | Target protocol |
|---|---|---|
| `contracts/` (grainlify-contracts) | `=21.7.7` | 21 |
| `contracts/bounty_escrow` (workspace) | `=21.7.7` | 21 |
| `contracts/grainlify-core` | `=21.7.7` | 21 |
| `contracts/program-escrow` | `=21.7.7` | 21 |
| `contracts/view-facade` | `=21.7.7` | 21 |
| `contracts/escrow-view-facade` | `=21.7.7` | 21 |
| `soroban/` (workspace: escrow, soroban-program-escrow, stream) | `=23.4.1` | 23 |

Notes:

- Pins are exact (`=x.y.z`), never caret ranges, so a lockfile regeneration can
  never float the SDK. Crates inside a workspace inherit the pin via
  `workspace = true`.
- Every manifest root has a committed `Cargo.lock`, and all checks run with
  `--locked` so a manifest/lock mismatch fails instead of silently re-resolving.
- The wasm build target is currently inconsistent across the repo's own
  tooling: `deploy.sh`, `deploy-sandbox.sh`, `demo_upgrade.sh`, the
  `config/*.env` files, and `Makefile.e2e` build `wasm32-unknown-unknown`,
  while the escrow Makefile (via `stellar contract build`) and
  `run_testnet_benchmarks.sh` expect `wasm32v1-none`. CI installs both. The
  recorded toolchain artifact captures which targets are installed; picking
  one target per tree is part of the next pin move, not this gate.

## Current pins (exact table for the gate)

| Manifest root | soroban-sdk pin | Target protocol | vs repository target |
|---|---|---|---|
| `contracts/` (grainlify-contracts) | `=21.7.7` | 21 | behind (exception) |
| `contracts/bounty_escrow` (workspace) | `=21.7.7` | 21 | behind (exception) |
| `contracts/grainlify-core` | `=21.7.7` | 21 | behind (exception) |
| `contracts/program-escrow` | `=21.7.7` | 21 | behind (exception) |
| `contracts/view-facade` | `=21.7.7` | 21 | behind (exception) |
| `contracts/escrow-view-facade` | `=21.7.7` | 21 | behind (exception) |
| `soroban/` (workspace: escrow, soroban-program-escrow, stream) | `=23.4.1` | 23 | on target |

## The gate

`scripts/check_sdk_versions.sh` enforces this table **and reports the cross-tree divergence** on every run (it must not pass silently as if the repo were on a single major). For every root above it
runs `cargo tree --locked` and fails when:

- a graph resolves anything other than exactly one `soroban-sdk` version,
- the resolved version differs from the table,
- a manifest carries a non-exact or unknown `soroban-sdk` requirement (a **third** version fails here),
- a root is missing its committed `Cargo.lock`.

Allowed exact pins today are only `=21.7.7` and `=23.4.1`. Introducing `=22.x`, a caret range, or any other major fails CI.

CI runs the gate in `.github/workflows/sdk-version-check.yml` on every push and
pull request that touches a manifest, lockfile, or the script itself, and
uploads a `toolchain-versions` artifact recording `rustc -Vv`, `cargo -V`, the
installed wasm targets, and the resolved
soroban-sdk/soroban-env-host/stellar-xdr versions per lockfile.

`contracts/grainlify-core/tests/test_sdk_pin_consistency.rs` is the fast local
guardrail: `cargo test -p grainlify-core` asserts the manifests, this document,
and the gate script all state the same pins, the repository **target**, and the
owned exception.

To inspect a graph by hand (always with `--locked`, otherwise cargo may
rewrite or create lockfiles):

```bash
./scripts/check_sdk_versions.sh
cargo tree --manifest-path contracts/bounty_escrow/Cargo.toml --locked -i soroban-sdk
cargo tree --manifest-path soroban/Cargo.toml --locked -i soroban-sdk
```

Expect `-d` on a single tree to list darling 0.20/0.21 twice (soroban-sdk-macros vs
serde_with_macros pull different majors); that duplication is outside the SDK
family and not gated.

## When the pin may move

The pin is a design decision, not a chore update. A PR that moves it must state:

1. Why: the upstream release notes reviewed, and the concrete feature or fix
   needed from the new SDK.
2. Protocol fit: the new SDK's target protocol confirmed against the protocol
   currently active on testnet AND mainnet (an SDK targeting a protocol the
   network has not activated yet must wait).
3. Blast radius: all crates in a tree move in lockstep. `grainlify-core` is the
   anchor of the `contracts/` tree; every path-dependent crate moves in the
   same PR, and every affected `Cargo.lock` is regenerated with the new exact
   pin.
4. Target sync: if the repository target changes, update the **Repository target**
   section, the migration board owner row, the gate allow-list, and the
   guardrail test in the same PR.

## Required evidence before merging a pin move

- `scripts/check_sdk_versions.sh` green with the updated table (divergence report still accurate).
- `cargo test -p grainlify-core` green, including the updated
  `test_sdk_pin_consistency` pins.
- Serialization goldens green
  (`contracts/grainlify-core/src/test_serialization_compatibility.rs`); a
  breaking XDR change requires a migration plan first, per
  [SERIALIZATION_COMPATIBILITY_TESTS.md](SERIALIZATION_COMPATIBILITY_TESTS.md).
- Storage layout guardrail green (`cargo test --test test_storage_layout` in
  `contracts/grainlify-core`).
- The E2E upgrade suite green (`make -f Makefile.e2e test-e2e-all` in
  `contracts/`).
- A fresh testnet benchmark run via `contracts/scripts/run_testnet_benchmarks.sh`,
  with the gas-measurement docs updated to cite the new SDK version (the
  existing gas measurements record soroban-sdk 21.7.7 as their provenance;
  keep that accurate).
- The table in this document, the gate script, and the guardrail test updated
  together.

## Known caveats

- Test suites currently hardcode differing `LedgerInfo.protocol_version`
  values (20 in bounty-escrow tests, 22 in one program-escrow test, 21 in
  env-default snapshots). They pass because the pinned SDK does not enforce
  the ledger protocol in unit tests; a pin move should reconcile them to the
  documented target protocol rather than inherit the drift.
- `contracts/escrow` is a committed symlink into `contracts/bounty_escrow`;
  tooling that globs manifests must not double-count it (the gate script's
  `grep -r` does not follow directory symlinks).
