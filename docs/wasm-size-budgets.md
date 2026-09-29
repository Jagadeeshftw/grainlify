# Wasm Size Budgets

This repository enforces a wasm size budget for **every deployable contract** via
CI. Each artifact listed in [DEPLOYABLE_ARTIFACTS.md](../DEPLOYABLE_ARTIFACTS.md)
has a recorded size baseline in `.github/wasm-budgets.json`. The gate fails if an
artifact grows past its baseline plus the stated tolerance, or if it crosses the
absolute on-chain deployment ceiling.

## How It Works

1. **Budget file** (`.github/wasm-budgets.json`) records, for every deployable
   artifact:
   - `baseline_bytes` — the accepted size of the artifact,
   - `package` / `target_dir` / optional `target` and `artifact` — how to locate
     the built `.wasm`,
   - a human-readable `description`.

   It also sets two global gates:
   - `tolerance_percent` — how much growth over `baseline_bytes` is allowed,
   - `absolute_ceiling_bytes` — the on-chain limit, which must never be crossed.

2. **CI workflow** (`.github/workflows/wasm-size-budget.yml`) builds every
   workspace (`contracts/**` with `wasm32-unknown-unknown`, and the `soroban/`
   workspace with `wasm32v1-none`) in release mode, then runs the check script.

3. **Check script** (`scripts/check-wasm-budgets.sh`) compares each built wasm
   against its budget. It fails an artifact for either reason:
   - **growth** — `actual > baseline_bytes + tolerance_percent`,
   - **ceiling** — `actual > absolute_ceiling_bytes`.

   The effective limit is the tighter of the two.

4. **PR comment** posts the consolidated report, covering all artifacts in one
   place, directly on pull requests for immediate visibility.

## The absolute deployment ceiling

Stellar stores the maximum contract code size in the network config setting
`CONFIG_SETTING_CONTRACT_MAX_SIZE_BYTES` (`contractMaxSizeBytes`). On both
testnet and mainnet the current value is **131072 bytes (128 KiB)**:

```bash
# Read it straight from the network config via Soroban RPC
# (setting id: CONFIG_SETTING_CONTRACT_MAX_SIZE_BYTES)
```

A wasm larger than the ceiling cannot be uploaded at all, so the gate treats it
as an unconditional failure. Unlike `tolerance_percent`, it is not a policy you
can relax — the fix is to shrink the contract.

## Updating a Budget Intentionally

If your change legitimately increases a contract's wasm size:

```bash
# 1. Build every deployable (see DEPLOYABLE_ARTIFACTS.md), e.g.
cargo build --manifest-path contracts/bounty_escrow/Cargo.toml \
  --workspace --target wasm32-unknown-unknown --release
( cd soroban && cargo build -p escrow -p soroban-program-escrow \
  --target wasm32v1-none --release )

# 2. Refresh the recorded baselines from the built artifacts
bash scripts/update-wasm-budgets.sh

# 3. Review and commit
git diff .github/wasm-budgets.json
git add .github/wasm-budgets.json
git commit -m "chore(ci): update wasm size baselines

Reason: <explain why the size change is necessary>"
```

Include the rationale in your PR description so reviewers understand why the
baseline change is needed. The tolerance and ceiling are not changed by the
update script.

## Running Locally

```bash
# Build every deployable, then:
bash scripts/check-wasm-budgets.sh

# Fast regression tests for the check script (no Rust toolchain required):
bash scripts/tests/test_check_wasm_budgets.sh
```

## Artifact → Budget Mapping

Every artifact in `DEPLOYABLE_ARTIFACTS.md` has an entry:

| Artifact | Source Crate | Workspace | Target |
|----------|--------------|-----------|--------|
| `bounty_escrow.wasm` | `contracts/bounty_escrow/contracts/escrow` | `contracts/bounty_escrow` | `wasm32-unknown-unknown` |
| `grainlify_core.wasm` | `contracts/grainlify-core` | `contracts` | `wasm32-unknown-unknown` |
| `program_escrow.wasm` | `contracts/program-escrow` | `contracts` | `wasm32-unknown-unknown` |
| `escrow_view_facade.wasm` | `contracts/escrow-view-facade` | `contracts` | `wasm32-unknown-unknown` |
| `view_facade.wasm` | `contracts/view-facade` | `contracts` | `wasm32-unknown-unknown` |
| `escrow.wasm` | `soroban/contracts/escrow` | `soroban` | `wasm32v1-none` |
| `soroban_program_escrow.wasm` | `soroban/contracts/program-escrow` | `soroban` | `wasm32v1-none` |

## Why Budgets?

Soroban contracts are deployed as wasm blobs. Unchecked growth increases:

- **Deployment costs** — Stellar charges rent proportional to contract size.
- **Memory footprints** — Larger wasm means more memory during execution.
- **Review burden** — A size budget forces developers to justify growth.

The per-artifact baseline tracks each contract against its own history; the
shared absolute ceiling tracks every contract against what the network will
accept at all.
