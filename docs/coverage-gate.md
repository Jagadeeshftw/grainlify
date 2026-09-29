# Contract Coverage Gate

This repository measures line coverage for every contract crate in CI and
enforces a per-crate floor that cannot be dropped silently.

It exists because the test suite is large — roughly 50,000 lines of tests
against roughly 20,000 lines of contract source — and nothing was measuring
which of those contract lines the tests actually reach. Whether any of the 164
entry points in `bounty-escrow` or the 167 in `program-escrow` were exercised
at all was simply not answerable from CI.

## How It Works

Four pieces, split so that a measurement run and a pass/fail decision are
independent steps:

1. **Crate registry** (`.github/coverage-thresholds.json`) lists every contract
   crate, its manifest, its line-coverage floor, and — for crates that cannot be
   measured yet — the reason why. It is the single source of truth for both
   scripts, so adding a contract crate means adding one entry here.
2. **Measurement** (`scripts/run_contract_coverage.sh`) runs each registered
   crate's own test suite under LLVM source-based coverage via `cargo-llvm-cov`
   and writes per-crate LCOV, JSON, HTML and text reports into `coverage/`.
3. **Gate** (`scripts/check-coverage-thresholds.sh`) compares the measurement
   against the recorded floors and exits non-zero if any crate fell below its
   floor, lost its measurement, or was never given a floor.
4. **Workflow** (`.github/workflows/coverage.yml`) runs both and publishes the
   reports as the `contract-coverage` build artifact, plus a table in the job
   summary.

The floors are set to the level that was actually measured when the gate was
introduced, so the gate is green on day one and turns red the moment coverage
is lost.

## Reports

`scripts/run_contract_coverage.sh` writes, per crate:

| Path | Format |
|------|--------|
| `coverage/<crate>/lcov.info` | LCOV, for external tooling |
| `coverage/<crate>/coverage.json` | machine-readable totals (codecov format) |
| `coverage/<crate>/html/index.html` | browsable, line-by-line |
| `coverage/<crate>/summary.txt` | per-file text table |
| `coverage/<crate>/run.log` | the crate's full test log |
| `coverage/coverage-summary.json` | aggregate, consumed by the gate |

All of these are published as the `contract-coverage` artifact on every run,
including failing ones — a red build with no way to see what moved is the worst
possible outcome. `coverage/` is gitignored; reports are never committed.

## Running Locally

```bash
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov

# Measure every registered contract crate, then apply the gate
./scripts/run_contract_coverage.sh
./scripts/check-coverage-thresholds.sh

# Or just one crate (other crates' results are preserved)
./scripts/run_contract_coverage.sh bounty-escrow
./scripts/check-coverage-thresholds.sh
```

Useful environment variables:

| Variable | Default | Purpose |
|----------|---------|---------|
| `COVERAGE_OUT_DIR` | `<repo>/coverage` | where reports land |
| `COVERAGE_CLEAN` | `1` | discard stale `.profdata` before measuring |
| `COVERAGE_MERGE` | `1` | keep other crates' results on a partial run |

`COVERAGE_CLEAN` defaults to `1` for correctness, not hygiene. LLVM coverage
profiles accumulate, so a stale `.profdata` left over from a previous run would
still count lines hit by a test that has since been deleted — and the gate would
happily pass a suite that no longer exists. Only the profile data is discarded,
not the compiled instrumented objects, so the run still benefits from the Cargo
cache. Set `COVERAGE_CLEAN=0` for a fast, dirty local iteration.

## Changing a Floor

Coverage going **down** is the thing the gate exists to catch, so lowering a
floor is a deliberate act that has to be visible:

```bash
./scripts/run_contract_coverage.sh
jq '.crates' coverage/coverage-summary.json      # read what was actually measured
```

Then edit `min_lines_pct` for that crate in `.github/coverage-thresholds.json`,
round the new floor **down** to the measured value (never above it), and explain
the change in the PR description.

Coverage going **up** is the good case: raise the floor to the new measured
level in the same PR that added the tests. Leaving a floor behind an
achievable number is how gates stop meaning anything.

## Crate → Measurement Mapping

| Crate | Manifest | State |
|-------|----------|-------|
| `grainify-contracts` | `contracts/Cargo.toml` | measured |
| `bounty-escrow` | `contracts/bounty_escrow/Cargo.toml` | measured |
| `program-escrow` | `contracts/program-escrow/Cargo.toml` | measured |
| `grainlify-core` | `contracts/grainlify-core/Cargo.toml` | not measurable |
| `view-facade` | `contracts/view-facade/Cargo.toml` | not measurable |
| `escrow-view-facade` | `contracts/escrow-view-facade/Cargo.toml` | not measurable |

### Crates that cannot be measured yet

A crate is listed with `"status": "blocked"` and a written reason when its test
suite does not currently build. That is deliberate, and it is a temporary state:

- It makes the unmeasured set visible in the registry and in the job summary
  instead of something a contributor can widen by accident.
- It means a crate that *was* measurable and stops being measurable **fails the
  gate** rather than quietly dropping out of it.
- Crates are only ever expected to move `blocked` → `measured`, never back.

The three blocked crates have pre-existing build failures at `main` that are
independent of coverage work. `view-facade` and `escrow-view-facade` fail inside
`soroban-env-host 21.2.1` with an `ed25519-dalek` / `rand_core` version conflict
(`docs/wasm-size-budgets.md` records the same two facades as unbuildable on
`master`). `grainlify-core`'s test targets do not compile. All three are tracked
for separate fixes; this gate does not attempt them.

## A note on failing tests

Measurement uses `cargo-llvm-cov --ignore-run-fail`, so a pre-existing red test
cannot stop the report from being produced. Coverage measurement and test
correctness are separate concerns: the `build-test` job in `contracts-ci.yml` is
the authority on whether tests pass.

This is not a way to hide red tests. The passing and failing counts are recorded
in `coverage/coverage-summary.json` and shown in the job summary, and the full
test log is in the artifact.

The trade-off is worth stating: if a test panics hard enough to take its whole
test binary down before the others run, that binary's coverage is partial. The
gate still catches the regressions it is for — a deleted suite always shows up
as a drop — but an individual number should be read alongside the test counts.

## Why line coverage?

The question this gate answers is "does any test reach this contract line at
all", and line coverage answers it most directly. Region and function coverage
are also emitted by the tooling and are useful when reading a report; line
coverage is what the floor is expressed in because it is the least ambiguous of
the three and the easiest to compare against a diff.
