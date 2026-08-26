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
