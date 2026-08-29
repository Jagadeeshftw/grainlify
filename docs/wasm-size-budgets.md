# Wasm Size Budgets

This repository enforces per-contract wasm size budgets via CI. Each deployable
Soroban contract has a maximum allowed `.wasm` size recorded in
`.github/wasm-budgets.json`. The CI gate fails if any contract exceeds its
budget, preventing unreviewed bloat from landing on `main`.

## How It Works

1. **Budget file** (`.github/wasm-budgets.json`) maps each contract to a
   maximum size in bytes.
2. **CI workflow** (`.github/workflows/wasm-size-budget.yml`) builds all
   contracts targeting `wasm32-unknown-unknown` in release mode, then runs the
   check script.
3. **Check script** (`scripts/check-wasm-budgets.sh`) compares the actual wasm
   sizes against the budgets, prints a table with exact byte deltas, and exits
   non-zero if any contract exceeds its limit.
4. **PR comment** posts the budget report directly on pull requests for
   immediate visibility.

## Updating a Budget Intentionally

If your change legitimately increases a contract's wasm size:

```bash
# 1. Build all contracts
cargo build --target wasm32-unknown-unknown --release

# 2. Run the update script (adds 5 % headroom automatically)
bash scripts/update-wasm-budgets.sh

# 3. Review and commit
git diff .github/wasm-budgets.json
git add .github/wasm-budgets.json
git commit -m "chore(ci): increase <contract> wasm budget to <N> bytes

Reason: <explain why the size increase is necessary>"
```

Include the rationale in your PR description so reviewers understand why the
budget change is needed.

## Running Locally

```bash
# Build all contracts
cargo build --target wasm32-unknown-unknown --release

# Check sizes against budgets
bash scripts/check-wasm-budgets.sh
```

## Contract → Budget Mapping

| Contract | Manifest | Wasm Artifact |
|----------|----------|---------------|
| `bounty_escrow` | `contracts/bounty_escrow/Cargo.toml` | `bounty_escrow.wasm` |
| `grainlify_core` | `contracts/grainlify-core/Cargo.toml` | `grainlify_core.wasm` |
| `program_escrow` | `contracts/program-escrow/Cargo.toml` | `program_escrow.wasm` |
| `escrow_view_facade` | `contracts/escrow-view-facade/Cargo.toml` | `escrow_view_facade.wasm` |
| `view_facade` | `contracts/view-facade/Cargo.toml` | `view_facade.wasm` |

## Why Budgets?

Soroban contracts are deployed as wasm blobs. Unchecked growth increases:

- **Deployment costs** — Stellar charges rent proportional to contract size.
- **Memory footprints** — Larger wasm means more memory during execution.
- **Review burden** — A size budget forces developers to justify growth.

Budgets are intentionally generous (300 KB default) to avoid blocking日常
development while still catching accidental regressions or runaway code
generation. Adjust as the codebase matures.

## Measurement Evidence

Measured locally on **Sat Aug 29 2026** with **Rust 1.98.0**
(`cargo 1.98.0`, `wasm32-unknown-unknown` target) and **Soroban SDK 21.7.7**
(`soroban-sdk = "=21.7.7"`), using:

```bash
cargo build --target wasm32-unknown-unknown --release
bash scripts/check-wasm-budgets.sh
```

### Branch `feat/issue-1703-wasm-size-budgets`

| Contract | Actual (bytes) | Budget (bytes) | Delta | Status |
|----------|----------------|----------------|-------|--------|
| `bounty_escrow` | 302,875 | 319,488 | −16,613 | ✅ PASS |
| `grainlify_core` | 178,032 | 188,416 | −10,384 | ✅ PASS |
| `program_escrow` | 362,254 | 381,952 | −19,698 | ✅ PASS |
| `escrow_view_facade` | — | 400,000 | — | ❌ NOT BUILT |
| `view_facade` | — | 400,000 | — | ❌ NOT BUILT |

### Baseline (branch vs. `master`)

| Contract | `master` (bytes) | Branch (bytes) | Delta | % Change |
|----------|------------------|----------------|-------|----------|
| `bounty_escrow` | 293,218 | 302,875 | +9,657 | +3.29% |
| `grainlify_core` | 178,295 | 178,032 | −263 | −0.15% |
| `program_escrow` | n/a (does not compile on `master`) | 362,254 | — | — |
| `escrow_view_facade` | n/a (does not compile) | n/a (does not compile) | — | — |
| `view_facade` | n/a (does not compile) | n/a (does not compile) | — | — |

### ⚠️ Build status (critical finding)

Only **3 of 5** contracts compile on this branch. The remaining two fail to
build with genuine code errors (independent of the size-budget feature), so no
wasm can be produced for them and they cannot be measured:

- **`program_escrow`** (branch): failed to compile until a minimal fix was
  applied locally — its `DataKey` enum was missing the `TokenDecimals(Address)`
  variant that `add_allowed_token_with_decimals` references. The fix is a
  one-line enum variant addition. (Note: `master`'s `program_escrow` also does
  not compile — "custom attribute panicked" / "duplicate definitions with name
  `add_allowed_token_with_decimals`" — so this branch is itself an in-progress
  compilation fix.)
- **`view_facade`**: fails at the **linker** step with
  `duplicate symbol: get_admin` — the crate exports program-escrow's entire
  contract ABI (`get_admin`, `set_admin`, `add_allowed_token`, …) alongside its
  own entrypoints, producing conflicting wasm exports. This is an architectural
  issue in the `#[contractimpl]` wiring, not a budget issue.
- **`escrow_view_facade`**: fails to compile with an unclosed delimiter /
  missing `mod query;` + `mod types;` declarations and duplicate `use` imports —
  the `impl QueryCache` block was never closed and several module declarations
  are missing.

**Conclusion:** the size-budget gate and both scripts
(`check-wasm-budgets.sh`, `update-wasm-budgets.sh`) work correctly for the
contracts that build. However, the PR is **not merge-ready** until the
`program_escrow` / `view_facade` / `escrow_view_facade` compilation errors are
resolved — otherwise CI cannot produce wasm for 2 of the 5 budgets, and
`master`'s `program_escrow` does not build either.

### Verification method

- Build command: `cargo build --target wasm32-unknown-unknown --release`
- Check script: `bash scripts/check-wasm-budgets.sh` — prints per-contract table,
  exits `0` when all built contracts are within budget.
- Update script: `bash scripts/update-wasm-budgets.sh` — sets each budget to
  `actual + 5% + 1 KB`, rounded up to 1 KB (only for contracts with a built
  wasm; unbuilt contracts keep their placeholder budget).
- The budgets in `.github/wasm-budgets.json` were regenerated from these
  measurements for the three contracts that build; the two unbuilt facades keep
  their 400,000-byte placeholder until they compile.
