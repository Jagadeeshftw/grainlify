# Grainlify Core Feature-Flag Model & Supported Combinations

This document defines the feature-flag model for `grainlify-core`, specifies the intended purpose and target for each feature flag, enumerates all supported and unsupported combinations, and explains the dependency architecture required by downstream contracts and facades.

---

## 1. Feature Flag Declarations

`grainlify-core` declares seven cargo features in its `Cargo.toml`:

| Feature Flag | Default | Description & Scope |
|---|---|---|
| `default` | Yes (`["contract"]`) | Active by default. Enables `contract` for standalone builds. |
| `contract` | Included in `default` | Gates the on-chain contract implementation (`GrainlifyContract`, `#[contractimpl]` entrypoints, and `UpgradeInterface` trait implementation). |
| `strict-mode` | No | Enables runtime invariant enforcement and publishes `(symbol_short!("strict"), symbol_short!("inv_fail"))` events on invariant violation. |
| `testutils` | No | Exposes host-side mock utilities and test helpers for downstream crates. Host-only (requires `std`). |
| `upgrade_rollback_tests` | No | Test harness feature gating upgrade authorization, snapshot, and rollback test suites. |
| `governance_contract_tests` | No | Test harness feature gating governance state, proposal lifecycle, voting, and RBAC tests. |
| `wasm_tests` | No | Host-side test harness feature gating core monitoring, pseudo-randomness, and serialization compatibility tests. |

---

## 2. Downstream Contracts & Facade Architecture

Downstream contracts and facades (`program-escrow`, `view-facade`, `escrow-view-facade`) must depend on `grainlify-core` with:

```toml
grainlify-core = { path = "../grainlify-core", default-features = false }
```

### Why `default-features = false` is Mandatory

When compiling to a WebAssembly `cdylib` artifact (`--target wasm32-unknown-unknown`), Soroban exports the contract's entrypoints as WebAssembly functions (including `_` dispatch symbols).

1. **Library Mode (`default-features = false`)**:
   `grainlify-core` acts as a pure Rust library (`rlib`). It exports types (`DeployedContract`, `ContractKind`, `GovernanceConfig`), data storage keys (`DataKey`), error enums (`ContractError`), traits (`UpgradeInterface`), constants, events, and utility functions without defining `GrainlifyContract`'s `#[contractimpl]` entrypoints. Downstream contracts and facades link against this without symbol conflicts.

2. **Contract Mode (`contract` feature enabled)**:
   `GrainlifyContract`'s entrypoints are compiled and exported. If a downstream contract or facade includes `grainlify-core` with `default-features = true` (or `--features contract`), both the downstream contract and `GrainlifyContract` export duplicate contract ABI entrypoints into the same `cdylib` binary. At link time, LLVM's `wasm-ld` fails with duplicate symbol / link errors.

---

## 3. Supported Feature Combinations

The following combinations are officially supported and built/tested:

| # | Combination Flags | Target | Purpose / Intended Use | Primary Consumer |
|---|---|---|---|---|
| 1 | `default` (`contract`) | Host / wasm32 | Deployable standalone GrainlifyCore contract. | Standalone deployment (`grainlify_core.wasm`), e2e tests |
| 2 | `--no-default-features` (none) | Host / wasm32 | Pure library mode providing types, storage keys, errors, and traits without contract entrypoints. | `program-escrow`, `view-facade`, `escrow-view-facade` |
| 3 | `--no-default-features --features strict-mode` | Host / wasm32 | Pure library mode with strict runtime invariant checks active. | Downstream crates needing strict validation |
| 4 | `--features strict-mode` (`contract,strict-mode`) | Host / wasm32 | Deployable contract with strict runtime invariant validation enabled. | Deployable contract under strict-mode testing/operation |
| 5 | `--features testutils` (`contract,testutils`) | Host | Host-side test harness providing mock authorization and test utilities. | Unit/integration testing harnesses |
| 6 | `--features upgrade_rollback_tests` | Host | Test harness for upgrade authorization, snapshot, and rollback verification. | CI test runners |
| 7 | `--features governance_contract_tests` | Host | Test harness for governance state, voting, execution, and RBAC tests. | CI test runners |
| 8 | `--features wasm_tests` | Host | Host-side test harness for monitoring, pseudo-randomness, and serialization compatibility. | CI test runners |
| 9 | `--all-features` | Host | Complete verification with all features enabled simultaneously. | CI gate verification |

---

## 4. Unsupported Combinations & Fail-Fast Behavior

To prevent confusing link-time errors or unresolved symbols during builds, the following combinations are explicitly disallowed and fail fast with compile-time errors:

1. **`wasm_tests` without `contract` (`--no-default-features --features wasm_tests`)**:
   Wasm tests verify entrypoints and client interactions on `GrainlifyContractClient`, which requires `GrainlifyContract`. Fails with:
   `Feature combination error: 'wasm_tests' requires the 'contract' feature.`

2. **`upgrade_rollback_tests` without `contract` (`--no-default-features --features upgrade_rollback_tests`)**:
   Upgrade and rollback tests exercise contract upgrade entrypoints on `GrainlifyContractClient`. Fails with:
   `Feature combination error: 'upgrade_rollback_tests' requires the 'contract' feature.`

3. **`governance_contract_tests` without `contract` (`--no-default-features --features governance_contract_tests`)**:
   Governance tests exercise contract entrypoints. Fails with:
   `Feature combination error: 'governance_contract_tests' requires the 'contract' feature.`

4. **`testutils` on `wasm32-unknown-unknown`**:
   `testutils` relies on host facilities (`soroban-sdk/testutils` and `std`) which cannot be compiled into WebAssembly contract binaries. Fails with:
   `Feature combination error: 'testutils' cannot be compiled for target wasm32-unknown-unknown.`

5. **Downstream contracts omitting `default-features = false`**:
   Asserted by build test `test_feature_matrix.rs` to prevent wasm link-time duplicate symbol errors before packaging.
