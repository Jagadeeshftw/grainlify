# Test Fixture Hardening — Bounty Escrow Contract

> **Issue:** #1643 — Document test fixture hardening  
> **Contract:** BountyEscrow v1  
> **Last updated:** 2026-07-27

---

## 0. Path Note

The issue references `contracts/stream/tests/gas_regression.rs`, but the actual
fixture hardening infrastructure lives in the escrow contract test suite:

- **Gas budget enforcement tests:** `contracts/escrow/src/test_gas_budget.rs`
- **Gas profiling tests:** `contracts/escrow/src/test_gas.rs`
- **Gas budget module:** `contracts/escrow/src/gas_budget.rs`

(`contracts/escrow` is a symlink to `contracts/bounty_escrow/contracts/escrow`.)

---

## 1. Purpose

This document defines the **test fixture hardening** strategy for the BountyEscrow
smart contract test suite. It spells out:

- **What fixtures exist** and how they are constructed
- **What stability guarantees** the test suite provides
- **What reproducibility properties** tests can rely on
- **What the regression surface is** — i.e. what can break and how we detect it
- **What edge cases** around fixture determinism exist and are covered

---

## 2. Fixture Architecture

### 2.1 Setup Patterns

The test suite uses two shared fixture patterns:

#### Pattern A: `Setup` struct (gas budget tests)

File: `test_gas_budget.rs`

```
Setup::new()
  ├─ Env::default()                     // fresh Soroban test environment
  ├─ env.mock_all_auths()               // bypass auth for measurement-only tests
  ├─ env.budget().reset_unlimited()     // no host-level budget limits
  ├─ Address::generate(&env) × 3        // admin, depositor, contributor
  ├─ register_stellar_asset_contract_v2  // token via Stellar Asset Contract
  ├─ register_contract + client.init()   // deploy + initialize contract
  └─ client.set_whitelist(&depositor)    // bypass anti-abuse rate limiting
```

#### Pattern B: `Setup` struct (gas profiling tests)

File: `test_gas.rs` → `gas_profile::Setup`

```
Setup::new()
  ├─ Env::default()
  ├─ env.mock_all_auths()
  ├─ env.budget().reset_unlimited()
  ├─ Address::generate(&env) × 3
  ├─ register_stellar_asset_contract_v2
  ├─ register_contract + client.init()
  └─ client.set_whitelist(&depositor)
```

### 2.2 Why These Patterns Produce Deterministic Fixtures

Every `Setup::new()` call starts from an **identical clean slate**:

| Component | Determinism Guarantee |
|-----------|----------------------|
| `Env::default()` | Always starts with ledger sequence = 0, timestamp = 0, empty storage |
| `Address::generate(&env)` | Produces deterministic addresses from a fixed seed for a given call sequence |
| `register_stellar_asset_contract_v2` | Deterministic contract ID derivation from admin address |
| `register_contract(None, ...)` | Deterministic contract ID from a sequential counter |
| `env.budget().reset_unlimited()` | Resets CPU and memory meters to zero |

**Key property:** Within a single binary build, running `Setup::new()` twice
produces **identical** contract IDs, addresses, and storage layouts. This is
what makes gas measurements reproducible across repeated test runs.

---

## 3. Stability Guarantees

### 3.1 Sort-Order Stability

All batch operations sort items by ascending `bounty_id` before processing:

- `batch_lock_funds`: items sorted via `order_batch_lock_items()` (insertion sort by `bounty_id`)
- `batch_release_funds`: items sorted via `order_batch_release_items()` (insertion sort by `bounty_id`)

This ensures **deterministic execution order regardless of input ordering**,
which in turn guarantees deterministic storage writes and gas consumption.

### 3.2 Gas Measurement Stability

When running `cargo test gas_profile -- --nocapture`, the printed CPU and memory
numbers are:
- **Deterministic per binary build** — same binary, same numbers, every time
- **Isolated** — `env.budget().reset_unlimited()` is called before each measurement
- **Excluding setup cost** — only the operation under test is measured
- **Excluding anti-abuse overhead** — depositor is whitelisted in setup

### 3.3 What CAN Change the Numbers

Gas numbers are NOT stable across:
- **Soroban SDK version bumps** — new SDK versions may change internal cost accounting
- **Compiler version changes** — different Rust versions may produce different WASM
- **Contract code changes** — adding storage writes, events, or computation changes the cost
- **`MAX_BATCH_SIZE` changes** — affects batch scaling curves

### 3.4 What Does NOT Change the Numbers

- **Re-running the same test binary** — 100% deterministic
- **Different OS / architecture** — Soroban's test host is platform-independent
- **Parallel test execution** — each `Env` is isolated
- **Time of day, machine load, etc.** — no wall-clock dependency

---

## 4. Regression Surface

### 4.1 Gas Budget Cap Enforcement

Tests verify that setting a `max_cpu_instructions = 1` cap with `enforce = true`
causes every operation to return `Error::GasBudgetExceeded`. This is the
**canonical regression guard** — any change that accidentally skips the budget
check will be caught.

| Test | Guards |
|------|--------|
| `test_gas_budget_lock_cap_enforced` | `lock_funds` → `GasBudgetExceeded` |
| `test_gas_budget_release_cap_enforced` | `release_funds` → `GasBudgetExceeded` |
| `test_gas_budget_refund_cap_enforced` | `refund` → `GasBudgetExceeded` |
| `test_gas_budget_partial_release_cap_enforced` | `partial_release` → `GasBudgetExceeded` |
| `test_gas_budget_batch_lock_cap_enforced` | `batch_lock_funds` → `GasBudgetExceeded` |
| `test_gas_budget_batch_release_cap_enforced` | `batch_release_funds` → `GasBudgetExceeded` |

### 4.2 Advisory Mode Safety

Tests verify that with `enforce = false`, operations **still succeed** even when
the cap is breached, and a `GasBudgetCapExceeded` event is emitted. This guards
against the enforcement flag leaking into advisory mode.

### 4.3 Production Gap Detection

The `caps_enforced_in_production` field is a compile-time constant. Tests verify:
- In `testutils` builds: `caps_enforced_in_production == true`
- In production WASM: `caps_enforced_in_production == false` (verified by `cfg!`)

The `GasBudgetAdvisoryStatus` struct and `get_gas_budget_advisory_status` endpoint
make this gap **observable on-chain** without requiring code inspection.

### 4.4 Config Round-Trip

Tests verify that:
- Default config is fully uncapped (`max_cpu_instructions == 0`, `enforce == false`)
- Admin can set and read config (full round-trip)
- Non-admin authorization is rejected
- Config can be updated multiple times
- Advisory status matches `get_gas_budget` output exactly
- `caps_configured` returns `false` after resetting all caps to zero

### 4.5 Batch Size Hardening

The `MAX_BATCH_SIZE` constant (20) is tested at boundaries:
- n=1 (minimum batch)
- n=5 (mid-range)
- n=10 (half max)
- n=20 (at the cap)

The runtime-configurable `BatchSizeCaps` provides an additional hardening layer
without contract redeployment.

### 4.6 Warning Threshold

The 80% warning threshold (`WARNING_THRESHOLD_BPS = 8000`) is tested by setting
a cap just above the actual cost and verifying that a `GasBudgetCapApproached`
event is emitted when usage reaches the threshold.

---

## 5. Edge Cases Covered

### 5.1 Cap = 1 Always Breached

A cap of `max_cpu_instructions = 1` is always exceeded by any real Soroban
operation. This property is used as the **universal regression trigger**:
any code path that fails to call the budget check will pass silently when
the cap is 1.

### 5.2 Uncapped = No False Positives

When all caps are zero (uncapped), no operation should ever return
`GasBudgetExceeded`. This is the default state for all deployments and
is verified by `test_gas_budget_lock_uncapped_succeeds`.

### 5.3 Advisory Mode = No Reverts

When `enforce = false` and a cap is breached, the operation succeeds and
the event is emitted. This guards against the enforcement flag accidentally
causing reverts in advisory mode.

### 5.4 Config Reset Cleans Up

After setting caps and then resetting to uncapped, `caps_configured` returns
`false`. This verifies that the advisory status correctly reflects the
post-reset state.

### 5.5 Event Isolation

Each test verifies that events are emitted correctly without interfering
with each other. `setup()` is called fresh for every test, so event logs
are never contaminated across test cases.

### 5.6 Multi-Operation Budget Isolation

Setting a cap on one operation (e.g., `lock`) does NOT affect other
operations (e.g., `release`). Each operation's budget check only reads
its own cap from the config.

---

## 6. Running Fixture Hardening Tests

```bash
# Run all gas budget enforcement tests
cd contracts/bounty_escrow/contracts/escrow
cargo test test_gas_budget -- --nocapture

# Run all gas profiling tests (prints Markdown tables)
cargo test gas_profile -- --nocapture --test-threads=1

# Run the consolidated scaling summary
cargo test gas_profile_scaling_summary -- --nocapture

# Run all escrow tests (full regression suite)
cargo test --lib
```

---

## 7. Adding New Fixture-Hardened Tests

When adding a new operation that should have budget enforcement:

1. Add an `OperationBudget` field to `GasBudgetConfig` in `gas_budget.rs`
2. Add a setter helper in `test_gas_budget.rs`'s `Setup` struct
3. Add a cap-enforced test: `test_gas_budget_<op>_cap_enforced`
4. Add an advisory-mode test: `test_gas_budget_<op>_cap_advisory_succeeds`
5. Add a gas profiling test in `test_gas.rs`'s `gas_profile` module
6. Add a row in `gas_profile_scaling_summary`
7. Update this document

---

## 8. Expected Regression Triggers

The following changes should cause at least one fixture-hardened test to fail:

| Change | Breaking Test |
|--------|--------------|
| Removing `gas_budget::check()` call from any operation | `test_gas_budget_<op>_cap_enforced` |
| Changing `enforce` flag semantics | `test_gas_budget_lock_cap_advisory_succeeds` |
| Breaking batch sort order | `test_gas_budget_batch_lock_cap_enforced` |
| SDK `env.budget()` API removal/changes | `test_gas_budget_lock_cap_enforced` (compile error) |
| `GasBudgetConfig` struct layout change | `test_gas_budget_admin_can_set_and_read_config` |
| `is_any_cap_configured` logic error | `test_is_any_cap_configured_*` |
| Advisory status mismatch | `test_advisory_status_config_matches_get_gas_budget` |

---

## 9. References

- `gas_budget.rs` — core budget module (both `contracts/escrow/src/` and `contracts/bounty_escrow/contracts/escrow/src/`)
- `test_gas_budget.rs` — enforcement tests
- `test_gas.rs` — profiling tests
- `GAS_TESTS.md` — profiling report and methodology
- `CI_LOCAL_CHECK.md` — local CI verification steps
- `.github/workflows/contracts-ci.yml` — CI pipeline definition
- `scripts/run_contract_test_matrix.sh` — multi-contract test runner
