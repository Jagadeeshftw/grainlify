# Contract Benchmarks

This folder contains gas and CPU instruction benchmarks for the `program-escrow` Soroban contract.

---

## Purpose

Soroban contracts are billed for CPU instructions, memory, and ledger I/O on every transaction.
Unnoticed regressions can silently double fee costs for end-users. This folder keeps:

- **Deterministic unit-test benchmarks** — run locally or in CI, no network required.
- **Testnet benchmark results** — recorded against a real deployed contract so real fee data is preserved across releases.

---

## Types of Benchmarks

### 1. Unit Test Benchmarks (CI gate)

These use the `soroban-sdk` simulation environment and the `Budget` testutil to count CPU instructions and memory bytes consumed by a single transaction. Results are **deterministic** and do not vary by network conditions or ledger state.

| Test name | What it measures |
|-----------|-----------------|
| `ci_benchmark_gate_batch_payout_50` | Hard gate: 50-recipient `batch_payout` must stay under `CPU_INSNS_THRESHOLD_50` |
| `test_gas_profile_batch_payout_1` | Informational: 1-recipient baseline |
| `test_gas_profile_batch_payout_10` | Informational: 10-recipient scaling |
| `test_gas_profile_batch_payout_50` | Informational: 50-recipient scaling |
| `test_gas_profile_batch_payout_100` | Informational: 100-recipient scaling (threshold × 2) |
| `test_gas_profile_lock_program_funds` | Hard gate: single `lock_program_funds` call |

**Run the CI gate test:**

```bash
cargo test -p program-escrow ci_benchmark_gate_batch_payout_50 -- --nocapture
```

**Run all gas-profile tests (informational):**

```bash
cargo test -p program-escrow test_gas_profile_ -- --nocapture
```

Look for `[GAS-PROFILE]` lines in the output, e.g.:

```
[GAS-PROFILE] op=batch_payout batch_size=50 cpu_insns=9812345 mem_bytes=887200
```

### 2. Testnet Benchmarks

These deploy the contract to Stellar testnet and execute real transactions, capturing ledger sequence, transaction hash, and actual fee in stroops as reported by the RPC node.

**Run the full testnet benchmark suite:**

```bash
./contracts/scripts/run_testnet_benchmarks.sh
```

See `contracts/scripts/run_testnet_benchmarks.sh --help` for prerequisites (funded identity, Stellar CLI).

Results are written to `contracts/benchmarks/results/`.

---

## Interpreting Results in `results/`

Result files are JSON with the following top-level fields:

| Field | Description |
|-------|-------------|
| `schema_version` | Schema revision; increment when adding new fields |
| `generated_at` | ISO-8601 timestamp when the file was written |
| `network` | `testnet` or `mainnet` |
| `contract_name` | Cargo package name of the measured contract |
| `function` | Contract function that was benchmarked |
| `note` | Free-text context (e.g. "ESTIMATED — replace with real data") |
| `ci_threshold_cpu_instructions_50` | The value of `CPU_INSNS_THRESHOLD_50` at time of measurement |
| `measurements` | Array of per-batch-size measurement objects |

Each measurement object:

| Field | Description |
|-------|-------------|
| `batch_size` | Number of recipients / operations in the call |
| `ledger_sequence` | Ledger number at submission; `0` = not yet recorded |
| `transaction_hash` | Transaction hash on-chain; `"PENDING"` = not yet recorded |
| `fee_stroops` | Total fee paid (inclusion + resource), in stroops (1 XLM = 10,000,000 stroops) |
| `fee_xlm` | Human-readable XLM string (`fee_stroops / 10_000_000`) |
| `cpu_instructions` | CPU instructions consumed, as reported by the network |
| `memory_bytes` | Memory bytes consumed |
| `ledger_reads` | Number of ledger entries read |
| `ledger_writes` | Number of ledger entries written |
| `status` | `"estimated"`, `"measured"`, or `"verified"` |
| `measured_at` | ISO-8601 timestamp of the on-chain submission |

---

## CI Gate Behaviour

The GitHub Actions workflow `.github/workflows/benchmark-gate.yml` runs on every PR that touches `contracts/program-escrow/**` or `contracts/benchmarks/**`.

The gate test `ci_benchmark_gate_batch_payout_50` fails the build if:

```
cpu_instructions > CPU_INSNS_THRESHOLD_50   (currently 27_500_000)
```

The threshold constant is defined in:

```
contracts/program-escrow/src/test_batch_operations.rs
```

```rust
pub const CPU_INSNS_THRESHOLD_50: u64 = 27_500_000;
```

### Updating the Threshold

If you deliberately make the contract heavier (e.g. add a new feature) and the new CPU usage is acceptable:

1. Measure the new baseline with the gas-profile tests.
2. Update `CPU_INSNS_THRESHOLD_50` in `test_batch_operations.rs` to the new value (leave headroom of ~20%).
3. Re-run the testnet benchmarks and commit the updated JSON under `contracts/benchmarks/results/`.
4. Add a comment to the constant explaining why it changed.

---

## CPU Instructions → Testnet Fee (Approximate)

Stellar charges a **resource fee** proportional to the resources consumed plus a small **inclusion fee**:

```
fee_stroops ≈ inclusion_fee
            + (cpu_insns / 500_000) × 10_000
            + ledger_write_fee × ledger_writes
```

Where typical values are:

- `inclusion_fee` ≈ 100 stroops
- `cpu_insns / 500_000 × 10_000` ≈ the dominant cost driver
- `ledger_write_fee` ≈ 100–500 stroops per write entry

**Example — 50-recipient `batch_payout`:**

```
cpu_insns   = 9,800,000
fee         ≈ 100 + (9,800,000 / 500,000) × 10,000 + 53 × 300
            ≈ 100 + 196,000 + 15,900
            ≈ ~28,500 stroops  (≈ 0.00285 XLM)
```

See `docs/gas-optimization/batch-payout-benchmarks.md` for the full cost model and scaling table.
