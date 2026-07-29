# Gas Regression Guardrails Documentation

## Overview

The `grainlify-stream` crate provides gas regression testing infrastructure for the Grainlify smart contract ecosystem. This document explains the guardrails that prevent gas regressions, the gas-sensitive execution paths, and the regression surface.

## Purpose

Gas regression guardrails ensure that contract changes do not inadvertently increase gas costs. The infrastructure provides:

- **Deterministic measurements**: Same operation + same binary = identical gas cost
- **Isolated test environments**: Each test gets a fresh, untainted environment
- **Explicit edge case coverage**: All boundary conditions are documented and tested
- **Regression surface definition**: Clear documentation of what constitutes a regression

## Core Components

### 1. GasRegressionFixture

A test fixture that provides a hardened, reproducible test environment.

**Key Properties:**
- Fresh Soroban `Env` with budget meters reset to zero
- Mock auth enabled by default (for testing authorized operations)
- No shared mutable state between fixture instances
- O(1) creation cost, no ledger state interaction

**Gas-Sensitive Execution Paths:**

1. **Fixture Creation (`new()` / `default()`)**:
   - Cost: Minimal (O(1))
   - Sensitivity: None - deterministic per binary
   - Regression Surface: Changes to Soroban SDK's `Env::default()` or `mock_all_auths()`

2. **Fixture Creation Without Mock Auth (`new_without_mock_auths()`)**:
   - Cost: Minimal (O(1))
   - Sensitivity: None - deterministic per binary
   - Regression Surface: Changes to Soroban SDK's `Env::default()`

3. **Budget Reset (`reset_budget()`)**:
   - Cost: Minimal (O(1))
   - Sensitivity: None - always produces zero state
   - Regression Surface: Changes to Soroban SDK's `budget().reset_unlimited()`

### 2. BudgetDelta

Captured budget deltas for a single measured operation.

**Key Properties:**
- `cpu`: u64, always >= 0 (prevents underflow)
- `mem`: u64, always >= 0
- Deterministic for same operation and inputs on same binary

**Gas-Sensitive Execution Paths:**

1. **Measurement Capture (`measure()`)**:
   - Cost: Minimal (2 budget reads before + 2 after)
   - Sensitivity: None - measurement overhead is constant
   - Regression Surface: Changes to Soroban SDK's `cost_estimate().budget()` API

2. **Cost Check (`has_positive_cost()`)**:
   - Cost: Zero (pure comparison)
   - Sensitivity: None
   - Regression Surface: None (pure Rust code)

### 3. measure() Helper

Captures Soroban budget meters before and after an operation.

**Key Properties:**
- Returns identical values for same binary, same `Env` state, same operation
- Uses `saturating_sub` to prevent underflow
- Does not modify the operation being measured

**Gas-Sensitive Execution Paths:**

1. **Budget Read Before Operation**:
   - Cost: ~100 CPU instructions (SDK estimate)
   - Sensitivity: SDK version, host implementation
   - Regression Surface: Soroban SDK changes to budget accounting

2. **Budget Read After Operation**:
   - Cost: ~100 CPU instructions (SDK estimate)
   - Sensitivity: SDK version, host implementation
   - Regression Surface: Soroban SDK changes to budget accounting

## Gas-Sensitive Execution Paths

### Path 1: Contract Deployment Operations

**Example:** Token contract registration and minting

**Cost Factors:**
- Storage writes for contract initialization
- Auth validation
- Token logic execution

**Regression Surface:**
- Soroban SDK changes to `register_stellar_asset_contract_v2()`
- Token contract implementation changes
- Storage cost model changes in Soroban host

**Guardrails:**
- Test: `contract_gas_token_deploy_and_mint`
- Test: `contract_gas_determinism_across_fixtures`
- Ensures: Identical operations produce identical gas costs

### Path 2: Batch Operations

**Example:** Multiple consecutive mints

**Cost Factors:**
- Per-operation storage writes
- Auth validation per operation
- Loop overhead

**Regression Surface:**
- Compiler optimization changes
- SDK changes to token client
- Storage cost model changes

**Guardrails:**
- Test: `contract_gas_mint_scaling`
- Ensures: Batch costs scale predictably (n operations ≈ n × single operation)

### Path 3: Address Generation

**Example:** `Address::generate()`

**Cost Factors:**
- Random number generation
- Address derivation

**Regression Surface:**
- SDK changes to `Address::generate()` implementation
- Host function cost model changes

**Guardrails:**
- Test: `regression_baseline_address_generation_cost`
- Ensures: Address generation cost is stable per binary

### Path 4: No-Op Operations

**Example:** Empty closure or trivial let binding

**Cost Factors:**
- Compiler optimization level
- Whether operation is optimized away

**Regression Surface:**
- Compiler optimization changes
- SDK changes that add overhead to closures

**Guardrails:**
- Test: `regression_baseline_noop_measurement`
- Test: `edge_case_zero_cost_noop`
- Ensures: True no-ops consume zero or trivial CPU

## Edge Cases and Guardrails

### Edge Case 1: Zero-Cost Operations

**Behavior:** Empty closures consume exactly zero CPU and memory.

**Guardrails:**
- Test: `edge_case_zero_cost_noop`
- Assertion: `d.cpu == 0` and `d.mem == 0`
- Regression Surface: SDK changes that add phantom overhead

### Edge Case 2: Large Operations

**Behavior:** Operations can consume arbitrary amounts after `reset_unlimited()`.

**Guardrails:**
- Test: `edge_case_large_but_not_unlimited_operation`
- Assertion: Operation completes without `ExceededLimit`
- Regression Surface: Budget limit enforcement changes

### Edge Case 3: Boundary Values

**Behavior:** BudgetDelta handles u64::MAX correctly without overflow.

**Guardrails:**
- Test: `edge_case_budget_delta_extremes`
- Assertion: Type correctly handles zero and MAX values
- Regression Surface: None (pure Rust code)

### Edge Case 4: Sequential Measurements

**Behavior:** Sequential measurements with proper resets produce identical results.

**Guardrails:**
- Test: `edge_case_sequential_measurements_independent`
- Assertion: Sequential measurements match fresh fixture measurements
- Regression Surface: Budget reset correctness

### Edge Case 5: Fixture Reuse

**Behavior:** Fixtures can be reused indefinitely without degradation.

**Guardrails:**
- Test: `fixture_can_be_reused_multiple_cycles`
- Test: `fixture_mass_create_and_drop`
- Assertion: No resource leaks or state corruption
- Regression Surface: Fixture lifecycle management

## Regression Surface

### What Constitutes a Regression

A gas regression is detected when:

1. **Determinism Violation**: Same operation produces different gas costs on same binary
   - Detected by: `measurement_determinism_same_noop_twice`
   - Indicates: SDK bug, fixture bug, or environment contamination

2. **Baseline Shift**: Canonical no-op cost changes significantly
   - Detected by: `regression_baseline_noop_measurement`
   - Indicates: SDK upgrade, compiler optimization change

3. **Scaling Violation**: Batch operations don't scale linearly
   - Detected by: `contract_gas_mint_scaling`
   - Indicates: Storage caching regression, algorithm change

4. **Isolation Failure**: One fixture affects another
   - Detected by: `cross_fixture_no_contamination_shared_env_default`
   - Indicates: Shared mutable state bug

### Protected Against

The guardrails protect against:

1. **SDK Upgrades**: Any change to Soroban SDK that affects budget accounting
2. **Compiler Changes**: Optimization level or behavior changes
3. **Host Changes**: Soroban host implementation changes
4. **Fixture Bugs**: Any regression in the test infrastructure itself
5. **Environment Contamination**: Tests affecting each other

### Not Protected Against

The guardrails do NOT protect against:

1. **Intentional Algorithm Changes**: If you change the contract algorithm, gas costs will change (this is expected)
2. **Feature Additions**: Adding new features will increase gas (this is expected)
3. **Storage Layout Changes**: Changing storage structure will change costs (this is expected)
4. **Cross-Binary Variations**: Different builds may have different costs (this is expected)

## Determinism Guarantees

### What is Deterministic

For the same binary build, the following are guaranteed deterministic:

1. **Fixture Creation**: `GasRegressionFixture::new()` always produces identical initial state
2. **Budget Reset**: `reset_budget()` always produces zero state
3. **Measurement**: Same operation + same inputs = identical `BudgetDelta`
4. **Contract Operations**: Same contract call + same inputs = identical gas cost

### What is NOT Deterministic

The following are NOT guaranteed deterministic:

1. **Cross-Binary**: Different builds may have different costs
2. **Cross-SDK-Version**: Different SDK versions may have different costs
3. **Cross-Host-Version**: Different Soroban host versions may have different costs
4. **Different Inputs**: Different operation inputs will have different costs (by design)

## Usage Patterns

### Pattern 1: Single Operation Measurement

```rust
let fix = GasRegressionFixture::new();
fix.reset_budget();
let delta = measure(&fix.env, || {
    client.do_operation(&arg1, &arg2);
});
assert!(delta.has_positive_cost());
```

**Guardrails:** Ensures operation consumes measurable resources.

### Pattern 2: Determinism Verification

```rust
let d1 = {
    let fix = GasRegressionFixture::new();
    fix.reset_budget();
    measure(&fix.env, || { operation() })
};

let d2 = {
    let fix = GasRegressionFixture::new();
    fix.reset_budget();
    measure(&fix.env, || { operation() })
};

assert_eq!(d1, d2);
```

**Guardrails:** Ensures operation is deterministic.

### Pattern 3: Batch Scaling Verification

```rust
let fix = GasRegressionFixture::new();

fix.reset_budget();
let d_single = measure(&fix.env, || { single_operation() });

fix.reset_budget();
let d_batch = measure(&fix.env, || {
    for _ in 0..10 { single_operation() }
});

assert!(d_batch.cpu > d_single.cpu);
```

**Guardrails:** Ensures batch operations scale predictably.

## Test Coverage Summary

| Category | Tests | Purpose |
|----------|-------|---------|
| Fixture Creation & Isolation | 4 | Verify fixtures are independent and properly initialized |
| Measurement Determinism | 3 | Verify measurements are reproducible |
| Budget Reset Correctness | 2 | Verify reset produces canonical zero state |
| Edge Cases | 9 | Verify boundary conditions are handled |
| Fixture Reuse & Lifecycle | 3 | Verify fixtures can be reused safely |
| Reproducibility Across Runs | 2 | Verify same-process reproducibility |
| Cross-Fixture Non-Contamination | 2 | Verify no state leak between fixtures |
| Contract-Level Gas Regression | 4 | Verify fixture works with real contracts |
| Documented Regression Surface | 3 | Pin down baseline costs |
| **Total** | **32** | **Comprehensive coverage** |

## Running the Guardrails

```bash
# Run all gas regression tests
cargo test --package grainlify-stream -- --nocapture --test-threads=1

# Run specific categories
cargo test fixture_stability -- --nocapture
cargo test measurement -- --nocapture
cargo test edge_case -- --nocapture
cargo test contract_gas -- --nocapture
cargo test regression_baseline -- --nocapture
```

## Integration with Other Contracts

The `grainlify-stream` crate is designed to be used by other contract test suites:

```rust
// In escrow/tests/test_gas.rs
use grainlify_stream::{GasRegressionFixture, measure};

#[test]
fn escrow_lock_gas_cost() {
    let fix = GasRegressionFixture::new();
    // ... setup escrow contract ...
    fix.reset_budget();
    let delta = measure(&fix.env, || {
        escrow_client.lock(&depositor, &amount);
    });
    assert!(delta.has_positive_cost());
}
```

## Maintenance Guidelines

### When to Update Guardrails

1. **SDK Upgrade**: After Soroban SDK upgrade, re-run tests to update baseline expectations
2. **Contract Algorithm Change**: Update tests to reflect new expected costs
3. **New Feature**: Add new tests for the feature's gas profile

### When NOT to Update Guardrails

1. **Test Flakiness**: If tests fail intermittently, investigate environment contamination
2. **Minor Cost Variations**: Small variations (< 5%) are normal across builds
3. **Compiler Optimizations**: If compiler optimizes away no-ops, this is expected

### Adding New Guardrails

When adding new gas-sensitive paths:

1. Add a test to `tests/gas_regression.rs`
2. Document the regression surface in this file
3. Add to the test coverage summary table
4. Run tests to establish baseline

## Backward Compatibility

The guardrails maintain backward compatibility:

- Existing test patterns are preserved
- Fixture API is stable (no breaking changes)
- Measurement semantics are unchanged
- All existing tests continue to pass

## Conclusion

The gas regression guardrails provide a robust foundation for detecting gas regressions in the Grainlify smart contract ecosystem. By ensuring deterministic measurements, isolated test environments, and comprehensive edge case coverage, the infrastructure prevents unintended gas cost increases while allowing intentional optimizations.

The regression surface is well-defined and documented, making it clear what constitutes a regression versus an expected change. This clarity is essential for maintaining gas efficiency over time as the codebase evolves.
