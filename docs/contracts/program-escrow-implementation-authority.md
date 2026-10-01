# Program-escrow implementation authority

Grainlify has two Rust/Soroban crates whose source directory is named
`program-escrow`. They sit in different workspaces on different SDK majors, both
have substantial test suites, and neither name is sufficient to tell them apart.
This document records which implementation is authoritative, which is
superseded, and the evidence for that decision, so that no build, deploy, or
integration can pick up the wrong contract and so that issues and pull requests
can be routed to the right tree.

This is the program-escrow counterpart to
[`escrow-implementation-authority.md`](escrow-implementation-authority.md).

## Summary

| | Authoritative | Superseded |
|---|---|---|
| Source | `contracts/program-escrow/src/lib.rs` | `soroban/contracts/program-escrow/src/lib.rs` |
| Cargo package | `program-escrow` | `soroban-program-escrow` |
| Contract type | `ProgramEscrowContract` | `SorobanProgramEscrow` |
| Deployable artifact | `program_escrow.wasm` | `soroban_program_escrow.wasm` (not deployable) |
| Workspace | `contracts/Cargo.toml` | `soroban/Cargo.toml` |
| SDK | `=21.7.7` (protocol 21) | `=23.4.1` (protocol 23) |
| Size / surface | 7,951-line `lib.rs`, full entrypoint catalog | 1,676-line `lib.rs`, registry and search surface |
| Manifest | `contracts/program-escrow-manifest.json` (1.1.0) | `soroban/contracts/program-escrow/program-escrow-manifest.json` (1.0.0, no entrypoints) |
| Tests | 1,190 `#[test]` functions across 75 files | 139 `#[test]` functions across 6 files |
| Status | **Authoritative — the only deployable program escrow** | **Superseded — reference only** |

### Which implementation is deployable

`contracts/program-escrow` (package `program-escrow`, artifact
`program_escrow.wasm`) is the authoritative program escrow contract. It is the
only program escrow that may be deployed to testnet or mainnet. It locks program
prize pools and executes single, batch, and scheduled payouts.

No network deployment for this crate is recorded in this repository:
`contracts/program-escrow-manifest.json` carries no contract ID, and
`contracts/deployments/` holds no program-escrow record. It is therefore tracked
as **Staged** in [`DEPLOYABLE_ARTIFACTS.md`](../../DEPLOYABLE_ARTIFACTS.md).
Per the convention stated in the root `README.md`, a missing network ID means a
live deployment is unverified here — not that one has never existed.

### Which implementation is not deployed

`soroban/contracts/program-escrow` (package `soroban-program-escrow`, artifact
`soroban_program_escrow.wasm`) is superseded. It is **not deployed** and must
not be deployed.

The crate labels itself as superseded in its own README
([`soroban/contracts/program-escrow/README.md`](../../soroban/contracts/program-escrow/README.md)),
and the workspace overview in `soroban/README.md` repeats the notice.

### The two crates are not interchangeable

Unlike the two `escrow` crates, these are **not** competing implementations of
one feature set, and neither is a copy of the other:

- `contracts/program-escrow` holds program funds and pays recipients: prize
  pools, single/batch/scheduled payouts, disputes, granular pause flags,
  circuit breakers, fee-on-transfer routing, delegation, and reputation.
- `soroban/contracts/program-escrow` registers programs and serves indexed,
  cursor-paginated search over them, with labels, jurisdiction rules, and
  ownership transfer.

"Superseded" here means *this is not the deployable program escrow and it is
retained for reference only*. It does **not** mean the registry/search surface
has been reimplemented in the authoritative crate — it has not. Do not assume an
entrypoint exists in one crate because it exists in the other, and never join
them with a Cargo path dependency across the SDK-21/SDK-23 boundary.

## Evidence

- **Size budget** — `.github/wasm-budgets.json` has a `program_escrow` entry
  (package `program-escrow`, `contracts/program-escrow`, 381,952 B). There is no
  entry for `soroban-program-escrow`, matching the treatment of the superseded
  `soroban/contracts/escrow` crate.
- **Coverage gate** — `.github/coverage-thresholds.json` maps `program-escrow`
  to `contracts/program-escrow/Cargo.toml` only.
- **Manifest validation** — `.github/workflows/validate-manifests.yml` requires
  `contracts/program-escrow-manifest.json`. The soroban crate's manifest is a
  stub: `entrypoints.public`, `.admin`, and `.view` are all empty and
  `behaviors.authorization.model` is `"None"`, so it describes no shippable ABI.
- **Downstream dependents** — `contracts/view-facade/Cargo.toml:12` and
  `contracts/escrow-view-facade/Cargo.toml:11` declare Cargo path dependencies
  on `program-escrow` and query its ABI. No `Cargo.toml` anywhere in the
  repository depends on `soroban-program-escrow`.
- **Deployment tooling** — `contracts/scripts/deploy-sandbox.sh` builds and
  deploys `contracts/program-escrow`'s `program_escrow.wasm` (as
  `sandbox-program-escrow`), and `contracts/scripts/run_testnet_benchmarks.sh`
  deploys the same artifact to testnet. No deploy script targets
  `soroban_program_escrow.wasm`.
- **Packaging intent** — `soroban/contracts/program-escrow/Cargo.toml` declares
  `version = "0.0.0"` and `publish = false`; `contracts/program-escrow/Cargo.toml`
  does not.
- **CI** — `.github/workflows/build-reproducibility.yml` lists
  `contracts/program-escrow`'s `program_escrow.wasm` in the authoritative
  deployable artifact inventory and uploads it as `wasm-program-escrow`.
- **Test investment** — the authoritative crate carries roughly 8.5x the test
  functions of the superseded one (1,190 vs 139).

## Deployable artifact inventory

There is exactly **one** deployable program-escrow wasm in the repository:

| Artifact | Produced by | Deployment |
|---|---|---|
| `program_escrow.wasm` | `contracts/program-escrow/Cargo.toml` (package `program-escrow`) | Authoritative — the only program escrow that may be deployed. Staged; no network ID recorded here. |

The superseded `soroban/contracts/program-escrow` crate is listed in
[`DEPLOYABLE_ARTIFACTS.md`](../../DEPLOYABLE_ARTIFACTS.md) under *Not
deployable*, alongside the superseded `soroban/contracts/escrow` crate. Its wasm
name must remain mentioned in that file because `scripts/check_inventory.py`
fails CI when any `cdylib` crate's wasm name is absent — that check verifies
name presence only, so the non-deployable section satisfies it without implying
the artifact is shippable.

## Known inconsistencies

These do not change the authority decision above, but they are recorded so the
next contributor is not misled by them:

- `.github/workflows/soroban-ci.yml` builds and size-checks
  `soroban-program-escrow` and uploads it as the `soroban-wasm` artifact. The
  build is intentional — the crate is still tested — but the artifact is **not**
  deployable, and the workflow's wording has been corrected to say so.
- `.github/workflows/build-reproducibility.yml` still requires
  `all-wasm/wasm-soroban/program_escrow.wasm` in its final inventory step, so
  the superseded crate is *not* given the full exclusion that
  `soroban/contracts/escrow` receives in that file's comments. Reproducibility
  hashing a non-deployable artifact is harmless, but the header comment listing
  it under "Authoritative deployable artifact inventory" is wrong.
- `scripts/smoke-deploy.sh` enumerates both program-escrow trees. It is a local
  smoke harness, not a deployment path.
- `contracts/scripts/error-codes.json` pins the superseded crate's `Error` enum.
  Keeping the pin is correct while the crate remains in the tree; it does not
  imply deployability.

Removing the superseded crate from CI builds, or deleting it outright, is
deliberately **out of scope** for this record: it would drop 139 passing tests
and its registry/search surface has no replacement in the authoritative crate.

## Migration guidance

If any tooling, script, or documentation still references
`soroban/contracts/program-escrow` or `soroban_program_escrow.wasm` as a
deployable contract, repoint it at `contracts/program-escrow` /
`program_escrow.wasm`.

Route work using the table in [`CONTRIBUTING.md`](../../CONTRIBUTING.md#choose-the-right-tree):

| Change | Make it here |
|---|---|
| Program funds, payouts, releases, disputes, pause, fees, payout API | `contracts/program-escrow` |
| SDK-23 program registry/search experiment, migration research | `soroban/contracts/program-escrow` (reference only — never deployed) |

If the registry/search capability is ever needed in production, it must be
implemented or ported into `contracts/program-escrow` under the SDK-21 pin and
added to that crate's manifest — not deployed from the soroban tree.
