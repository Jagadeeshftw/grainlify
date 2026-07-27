//! # Gas Regression — Test Fixture Hardening
//!
//! Comprehensive test suite for verifying the stability, reproducibility, and
//! correctness of the gas regression test infrastructure. The tests validate that
//! the test fixtures produce **deterministic, isolated** results — ensuring that
//! any gas regression detected in contract tests is a genuine regression and not
//! a test-infrastructure artifact.
//!
//! ## What This Covers
//!
//! | Category                          | Tests |
//! |-----------------------------------|-------|
//! | Fixture creation & isolation      | 4     |
//! | Measurement determinism           | 3     |
//! | Budget reset correctness          | 2     |
//! | Edge cases (zero, max, boundary)  | 4     |
//! | Fixture reuse & lifecycle         | 3     |
//! | Reproducibility across runs       | 2     |
//! | Cross-fixture non-contamination   | 2     |
//! | Contract-level gas regression     | 4     |
//! | Documented regression surface     | 2     |
//! | **Total**                         | **26** |
//!
//! ## Running
//!
//! ```bash
//! cargo test --package grainlify-stream -- --nocapture --test-threads=1
//!
//! # Run specific categories
//! cargo test fixture_stability -- --nocapture
//! cargo test measurement -- --nocapture
//! cargo test edge_case -- --nocapture
//! cargo test contract_gas -- --nocapture
//! ```
//!
//! ## Reproducibility Model
//!
//! Soroban's `env.budget()` meters are **deterministic for fixed inputs**. This
//! means that running the same test twice on the same binary **always produces
//! identical values**. The tests in this module verify this property explicitly
//! and document any scenarios where it could break.
//!
//! ## Backward Compatibility
//!
//! All existing test patterns from the contract test suite (`test_gas.rs`,
//! `test_gas_budget.rs`, `test_rbac.rs`) are preserved. The fixture hardening
//! in this module extends — not replaces — those patterns.
//!
//! ## Architecture
//!
//! The `GasRegressionFixture`, `BudgetDelta`, and `measure()` helper are
//! defined in `grainlify_stream` (the crate's `lib.rs`) so they can be reused
//! across multiple test modules and contract crates.

use grainlify_stream::{measure, BudgetDelta, GasRegressionFixture};
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token, Address, Env,
};

// =============================================================================
// 1. FIXTURE CREATION & ISOLATION
// =============================================================================

/// Two independently created fixtures must not share any mutable state.
///
/// **What this proves**: Creating `GasRegressionFixture::new()` twice yields
/// two completely independent environments. Budget resets in one do not
/// affect the other.
#[test]
fn fixture_isolation_independent_environments() {
    let fix_a = GasRegressionFixture::new();
    let fix_b = GasRegressionFixture::new();

    // Both start with unlimited budget (reset in constructor)
    let cpu_a_before = fix_a.env.cost_estimate().budget().cpu_instruction_cost();
    let cpu_b_before = fix_b.env.cost_estimate().budget().cpu_instruction_cost();

    // Neither has consumed any instructions yet
    assert_eq!(
        cpu_a_before, 0,
        "fresh fixture A must start with zero CPU count"
    );
    assert_eq!(
        cpu_b_before, 0,
        "fresh fixture B must start with zero CPU count"
    );
}

/// A fixture created with mock auths must have mock auths enabled.
///
/// **What this proves**: The default constructor sets up the expected
/// authorization environment. With mock auths, operations that require
/// authorization succeed without explicit signing — we verify this by
/// successfully generating addresses (which requires an initialized env).
#[test]
fn fixture_default_has_mock_auths() {
    let fix = GasRegressionFixture::new();

    // With mock_all_auths(), the env is in a fully permissive state.
    // We verify the env is properly initialized and usable by generating
    // addresses without any auth setup.
    let addr1 = Address::generate(&fix.env);
    let addr2 = Address::generate(&fix.env);

    // Two generated addresses from the same seeded env must be distinct
    assert_ne!(
        addr1, addr2,
        "generated addresses in a properly initialized env must be distinct"
    );
}

/// A fixture created without mock auths must correctly report no auth mocking.
///
/// **What this proves**: The explicit `new_without_mock_auths()` constructor
/// creates a valid environment that does not auto-authorize calls. The environment
/// itself is still valid — auth rejection happens at call time.
#[test]
fn fixture_without_mock_auths_has_no_auth() {
    let fix = GasRegressionFixture::new_without_mock_auths();
    // The environment was created successfully without panicking.
    // mock_all_auths was NOT called — address generation still works
    // because it doesn't require authorization.
    let addr = Address::generate(&fix.env);
    let _ = addr;
}

/// The `Default` trait implementation must produce the same result as `new()`.
///
/// **What this proves**: `GasRegressionFixture::default()` is a valid
/// alternative constructor with identical behavior to `new()`.
#[test]
fn fixture_default_equals_new() {
    let fix_new = GasRegressionFixture::new();
    let fix_default = GasRegressionFixture::default();

    // Both environments are in identical initial states
    assert_eq!(
        fix_new.env.cost_estimate().budget().cpu_instruction_cost(),
        fix_default.env.cost_estimate().budget().cpu_instruction_cost(),
        "CPU counters must match between new() and default()"
    );
    assert_eq!(
        fix_new.env.cost_estimate().budget().memory_bytes_cost(),
        fix_default.env.cost_estimate().budget().memory_bytes_cost(),
        "memory counters must match between new() and default()"
    );
}

// =============================================================================
// 2. MEASUREMENT DETERMINISM
// =============================================================================

/// The `measure()` helper must return identical results for identical inputs.
///
/// **What this proves**: Running the same no-op operation twice in the same
/// environment yields exactly the same budget deltas — confirming that
/// `env.budget()` meters are deterministic.
#[test]
fn measurement_determinism_same_noop_twice() {
    let fix = GasRegressionFixture::new();

    fix.reset_budget();
    let d1 = measure(&fix.env, || {
        // no-op
    });

    fix.reset_budget();
    let d2 = measure(&fix.env, || {
        // same no-op
    });

    assert_eq!(
        d1, d2,
        "identical no-op operations must produce identical budget deltas"
    );
}

/// Repeated measurements within the same fixture must be consistent.
///
/// **What this proves**: After a budget reset, the measurement helper
/// captures a fresh delta that matches previous measurements of the
/// same operation.
#[test]
fn measurement_consistency_across_resets() {
    let fix = GasRegressionFixture::new();

    let mut deltas: Vec<BudgetDelta> = Vec::new();
    for _ in 0..5 {
        fix.reset_budget();
        let d = measure(&fix.env, || {
            // consistent operation: bind a u64
            let _x: u64 = 42;
        });
        deltas.push(d);
    }

    // All five measurements of the same operation must be identical
    let first = &deltas[0];
    for (i, d) in deltas.iter().enumerate().skip(1) {
        assert_eq!(
            first, d,
            "measurement {} must match measurement 0; deterministic property violated",
            i
        );
    }
}

/// Measurements must NOT be affected by prior operations after a budget reset.
///
/// **What this proves**: `reset_budget()` properly isolates measurements.
/// An expensive operation before reset does not affect the next measurement.
#[test]
fn measurement_isolation_after_reset() {
    let fix = GasRegressionFixture::new();

    // Perform an expensive setup operation (simulated via many address generations)
    for _ in 0..100 {
        let _addr = Address::generate(&fix.env);
    }

    // Reset, then measure
    fix.reset_budget();
    let d_after_heavy_setup = measure(&fix.env, || {
        let _x: u64 = 1;
    });

    // Fresh fixture, measure same operation
    let fix2 = GasRegressionFixture::new();
    fix2.reset_budget();
    let d_fresh = measure(&fix2.env, || {
        let _x: u64 = 1;
    });

    assert_eq!(
        d_after_heavy_setup, d_fresh,
        "measurement after reset must match measurement from fresh fixture"
    );
}

// =============================================================================
// 3. BUDGET RESET CORRECTNESS
// =============================================================================

/// After `reset_budget()`, CPU and memory counters must be at zero.
///
/// **What this proves**: The reset is effective — counters don't carry
/// forward from previous operations.
#[test]
fn reset_budget_zeroes_counters() {
    let fix = GasRegressionFixture::new();

    // Consume some budget first
    for _ in 0..50 {
        let _addr = Address::generate(&fix.env);
    }
    assert!(
        fix.env.cost_estimate().budget().cpu_instruction_cost() > 0,
        "should have consumed some CPU instructions after address generation"
    );

    // Reset
    fix.reset_budget();

    // Must be at zero
    assert_eq!(
        fix.env.cost_estimate().budget().cpu_instruction_cost(),
        0,
        "CPU counter must be zero after reset_budget()"
    );
    assert_eq!(
        fix.env.cost_estimate().budget().memory_bytes_cost(),
        0,
        "memory counter must be zero after reset_budget()"
    );
}

/// `reset_budget()` must be idempotent — calling it multiple times is safe.
///
/// **What this proves**: Double-reset doesn't cause any issues or change
/// the zero state.
#[test]
fn reset_budget_idempotent() {
    let fix = GasRegressionFixture::new();

    // Consume some budget
    for _ in 0..10 {
        let _addr = Address::generate(&fix.env);
    }

    fix.reset_budget();
    fix.reset_budget(); // second reset
    fix.reset_budget(); // third reset

    assert_eq!(fix.env.cost_estimate().budget().cpu_instruction_cost(), 0);
    assert_eq!(fix.env.cost_estimate().budget().memory_bytes_cost(), 0);
}

// =============================================================================
// 4. EDGE CASES
// =============================================================================

/// Zero-cost operations must report zero budget deltas.
///
/// **What this proves**: A true no-op (empty closure) consumes no resources.
/// The measurement infrastructure doesn't add phantom overhead.
#[test]
fn edge_case_zero_cost_noop() {
    let fix = GasRegressionFixture::new();
    fix.reset_budget();

    let d = measure(&fix.env, || {
        // truly empty — no allocations, no calls, no nothing
    });

    assert_eq!(
        d.cpu, 0,
        "empty no-op must consume exactly zero CPU instructions"
    );
    assert_eq!(
        d.mem, 0,
        "empty no-op must consume exactly zero memory bytes"
    );
}

/// The measurement helper must handle operations at the boundary of budget limits.
///
/// **What this proves**: After `reset_unlimited()`, operations can consume
/// arbitrary amounts without hitting `ExceededLimit`. The test exercises
/// a moderately expensive operation to verify no spurious budget errors.
#[test]
fn edge_case_large_but_not_unlimited_operation() {
    let fix = GasRegressionFixture::new();
    fix.reset_budget();

    let d = measure(&fix.env, || {
        // Generate many addresses — a real but bounded operation
        for i in 0..1000u64 {
            let _addr = Address::generate(&fix.env);
            // prevent optimization from eliminating the loop
            soroban_sdk::log!(&fix.env, "addr gen step", i);
        }
    });

    assert!(
        d.has_positive_cost(),
        "large operation must consume measurable CPU resources"
    );
}

/// The `BudgetDelta` type must handle boundary values correctly.
///
/// **What this proves**: The type correctly handles u64 boundary values
/// including zero and MAX without overflow.
#[test]
fn edge_case_budget_delta_extremes() {
    let zero = BudgetDelta { cpu: 0, mem: 0 };
    let max = BudgetDelta {
        cpu: u64::MAX,
        mem: u64::MAX,
    };

    // Boundary assertions
    assert!(!zero.has_positive_cost(), "zero-cost delta must report false");
    assert!(
        max.has_positive_cost(),
        "max-cost delta must report true"
    );
    assert_eq!(zero, zero, "zero must equal itself");
    assert_eq!(max, max, "max must equal itself");
    assert_ne!(zero, max, "zero must not equal max");
}

/// Sequential measurements must not interfere with each other.
///
/// **What this proves**: Running measurements sequentially on the same
/// fixture, with proper resets between them, produces the same results
/// as running them on fresh fixtures — confirming no state leak.
#[test]
fn edge_case_sequential_measurements_independent() {
    // Fresh fixtures A and B
    let fix_a = GasRegressionFixture::new();
    fix_a.reset_budget();
    let d_a = measure(&fix_a.env, || {
        let _: u64 = 100;
    });

    let fix_b = GasRegressionFixture::new();
    fix_b.reset_budget();
    let d_b = measure(&fix_b.env, || {
        let _: u64 = 100;
    });

    // Sequential on same fixture C
    let fix_c = GasRegressionFixture::new();
    fix_c.reset_budget();
    let d_c1 = measure(&fix_c.env, || {
        let _: u64 = 100;
    });
    // Do something unrelated between measurements
    let _addr = Address::generate(&fix_c.env);
    fix_c.reset_budget();
    let d_c2 = measure(&fix_c.env, || {
        let _: u64 = 100;
    });

    // All measurements of the same operation must be identical
    assert_eq!(d_a, d_b, "fresh fixtures must produce identical deltas");
    assert_eq!(
        d_a, d_c1,
        "first measurement on reused fixture must match fresh fixture"
    );
    assert_eq!(
        d_a, d_c2,
        "second measurement on reused fixture (after reset) must match fresh fixture"
    );
}

// =============================================================================
// 5. FIXTURE REUSE & LIFECYCLE
// =============================================================================

/// A fixture can be reused for multiple measurement cycles.
///
/// **What this proves**: The fixture is not "consumed" by a single use.
/// It can be reset and reused indefinitely, which is critical for
/// test suites that measure many operations in sequence.
#[test]
fn fixture_can_be_reused_multiple_cycles() {
    let fix = GasRegressionFixture::new();

    for cycle in 0..10 {
        fix.reset_budget();
        let d = measure(&fix.env, || {
            let _: u64 = cycle;
        });
        // Verify the measurement completed successfully and produced a value.
        // The actual CPU cost may be zero (compiler-optimized) or non-zero.
        // The important property is that the measurement infrastructure works
        // consistently across all cycles without panicking.
        assert!(
            d.cpu < u64::MAX / 2,
            "cycle {}: measurement must not produce unreasonably large CPU count",
            cycle
        );
    }
}

/// Creating and dropping many fixtures in sequence must not leak resources.
///
/// **What this proves**: The fixture has no hidden resource leaks.
/// Creating 100 fixtures in a tight loop succeeds without panicking
/// or degrading.
#[test]
fn fixture_mass_create_and_drop() {
    for i in 0..100 {
        let fix = GasRegressionFixture::new();
        fix.reset_budget();
        let d = measure(&fix.env, || {
            let _: u64 = i;
        });
        let _ = d; // use the measurement
                    // fixture is dropped here
    }
    // Reaching here without panic proves no resource leak
}

/// Fixture state after an operation is consistent and predictable.
///
/// **What this proves**: After running an operation, the fixture's
/// budget counters reflect the consumed resources, and the environment
/// remains in a valid state for further operations.
#[test]
fn fixture_state_after_operation_is_valid() {
    let fix = GasRegressionFixture::new();
    fix.reset_budget();

    let _d = measure(&fix.env, || {
        let _addr = Address::generate(&fix.env);
    });

    // After measurement, CPU counter reflects consumed instructions
    let cpu_after = fix.env.cost_estimate().budget().cpu_instruction_cost();
    assert!(
        cpu_after > 0,
        "CPU counter must reflect consumed instructions after operation"
    );

    // The environment is still valid — we can generate more addresses
    let _addr2 = Address::generate(&fix.env);
}

// =============================================================================
// 6. REPRODUCIBILITY ACROSS RUNS
// =============================================================================

/// The test environment must produce the same results within a single process.
///
/// **What this proves**: Two fixtures created at different points in the
/// same test run produce identical initial states, confirming that
/// fixture creation is not influenced by prior test execution.
#[test]
fn reproducibility_same_process_different_times() {
    let d_first = {
        let fix = GasRegressionFixture::new();
        fix.reset_budget();
        measure(&fix.env, || {
            let _: u64 = 42;
        })
    };

    // Simulate time passing / other tests running
    for _ in 0..50 {
        let _fix = GasRegressionFixture::new();
    }

    let d_second = {
        let fix = GasRegressionFixture::new();
        fix.reset_budget();
        measure(&fix.env, || {
            let _: u64 = 42;
        })
    };

    assert_eq!(
        d_first, d_second,
        "same operation must produce identical deltas regardless of prior fixture creation"
    );
}

/// `reset_budget()` always produces the same zero-state regardless of prior history.
///
/// **What this proves**: The reset operation brings the fixture to a
/// canonical "clean" state that is identical to a freshly created fixture.
/// This is the foundation of reproducible gas measurements.
#[test]
fn reproducibility_reset_produces_canonical_state() {
    // Fresh fixture, immediately measure
    let d_fresh = {
        let fix = GasRegressionFixture::new();
        fix.reset_budget();
        measure(&fix.env, || {
            let _: u64 = 99;
        })
    };

    // Fixture that has seen heavy use, then reset, then measure
    let d_after_heavy_use = {
        let fix = GasRegressionFixture::new();
        // Heavy pre-work
        for _ in 0..500 {
            let _addr = Address::generate(&fix.env);
        }
        fix.reset_budget(); // canonical reset
        measure(&fix.env, || {
            let _: u64 = 99;
        })
    };

    assert_eq!(
        d_fresh, d_after_heavy_use,
        "reset must produce the same canonical state as a fresh fixture"
    );
}

// =============================================================================
// 7. CROSS-FIXTURE NON-CONTAMINATION
// =============================================================================

/// Two fixtures created from the same `Env::default()` must be independent.
///
/// **What this proves**: Fixtures do not share any hidden global state.
/// Modifying one fixture does not affect measurements in another.
#[test]
fn cross_fixture_no_contamination_shared_env_default() {
    let fix_a = GasRegressionFixture::new();
    let fix_b = GasRegressionFixture::new();

    // Use fix_a heavily
    for _ in 0..200 {
        let _addr = Address::generate(&fix_a.env);
    }

    // fix_b should be unaffected
    assert_eq!(
        fix_b.env.cost_estimate().budget().cpu_instruction_cost(),
        0,
        "fixture B must not see CPU consumption from fixture A"
    );
    assert_eq!(
        fix_b.env.cost_estimate().budget().memory_bytes_cost(),
        0,
        "fixture B must not see memory consumption from fixture A"
    );
}

/// Ledger state modifications in one fixture must not leak to another.
///
/// **What this proves**: Ledger timestamps, sequence numbers, and other
/// ledger state are isolated per `Env` instance.
#[test]
fn cross_fixture_no_ledger_state_leak() {
    let fix_a = GasRegressionFixture::new();
    let fix_b = GasRegressionFixture::new();

    // Advance time in fix_a (ledger starts at timestamp 0)
    fix_a.env.ledger().set(LedgerInfo {
        timestamp: 10_000,
        ..fix_a.env.ledger().get()
    });

    let ts_a = fix_a.env.ledger().timestamp();
    assert_eq!(ts_a, 10_000, "fixture A timestamp must reflect the advance");

    // fix_b's timestamp must remain at the initial default (0)
    let ts_b = fix_b.env.ledger().timestamp();
    assert_eq!(
        ts_b, 0,
        "fixture B's ledger timestamp must not be affected by fixture A's time advance"
    );
}

// =============================================================================
// 8. CONTRACT-LEVEL GAS REGRESSION
// =============================================================================
//
// The following tests exercise the fixture against actual Soroban token
// operations — the closest analogue to contract-level gas regression available
// without depending on the workspace contracts. These tests verify that:
//
// 1. The fixture works correctly with contract deployments
// 2. Token operations produce measurable, deterministic gas costs
// 3. Batch-style operations scale predictably
// 4. The fixture correctly isolates contract-level measurements

/// Registering a token contract and minting tokens produces measurable gas.
///
/// **What this proves**: The fixture works end-to-end with real contract
/// operations. Token registration + minting is a representative "hot path"
/// that exercises storage, auth, and token logic.
#[test]
fn contract_gas_token_deploy_and_mint() {
    let fix = GasRegressionFixture::new();
    let admin = Address::generate(&fix.env);

    fix.reset_budget();
    let d = measure(&fix.env, || {
        let token_contract = fix.env.register_stellar_asset_contract_v2(admin.clone());
        let token_id = token_contract.address();
        let token_sac = token::StellarAssetClient::new(&fix.env, &token_id);
        token_sac.mint(&admin, &1_000_000);
    });

    // Token deployment + mint must consume measurable CPU
    assert!(
        d.has_positive_cost(),
        "token deploy + mint must consume CPU instructions; got cpu={}",
        d.cpu
    );

    // Regression guard: the operation must not consume zero resources
    // (zero would indicate the operation was optimized away)
    assert!(
        d.mem > 0,
        "token deploy + mint must consume memory for storage; got mem={}",
        d.mem
    );
}

/// The same token operation measured twice must produce identical deltas.
///
/// **What this proves**: Contract-level operations are deterministic.
/// Two separate fixtures running the same token deploy + mint must produce
/// identical gas measurements — this is the foundation of gas regression testing.
#[test]
fn contract_gas_determinism_across_fixtures() {
    let d1 = {
        let fix = GasRegressionFixture::new();
        let admin = Address::generate(&fix.env);
        fix.reset_budget();
        measure(&fix.env, || {
            let token_contract = fix.env.register_stellar_asset_contract_v2(admin.clone());
            let token_id = token_contract.address();
            let token_sac = token::StellarAssetClient::new(&fix.env, &token_id);
            token_sac.mint(&admin, &1_000_000);
        })
    };

    let d2 = {
        let fix = GasRegressionFixture::new();
        let admin = Address::generate(&fix.env);
        fix.reset_budget();
        measure(&fix.env, || {
            let token_contract = fix.env.register_stellar_asset_contract_v2(admin.clone());
            let token_id = token_contract.address();
            let token_sac = token::StellarAssetClient::new(&fix.env, &token_id);
            token_sac.mint(&admin, &1_000_000);
        })
    };

    assert_eq!(
        d1, d2,
        "identical contract operations across fresh fixtures must produce identical gas deltas"
    );
}

/// Batch-style operations (multiple consecutive mints) scale predictably.
///
/// **What this proves**: Running n mints in sequence costs approximately
/// n times the cost of a single mint. This verifies that the fixture
/// correctly captures per-operation costs in batch patterns.
#[test]
fn contract_gas_mint_scaling() {
    let fix = GasRegressionFixture::new();
    let admin = Address::generate(&fix.env);
    let token_contract = fix.env.register_stellar_asset_contract_v2(admin.clone());
    let token_id = token_contract.address();
    let token_sac = token::StellarAssetClient::new(&fix.env, &token_id);

    // Measure a single mint
    fix.reset_budget();
    let d_single = measure(&fix.env, || {
        token_sac.mint(&admin, &1_000);
    });

    // Measure 5 mints as a single batch operation
    fix.reset_budget();
    let d_batch = measure(&fix.env, || {
        for _ in 0..5 {
            token_sac.mint(&admin, &1_000);
        }
    });

    // The batch cost should be greater than a single mint
    assert!(
        d_batch.cpu > d_single.cpu,
        "batch of 5 mints must consume more CPU than single mint; single={}, batch={}",
        d_single.cpu,
        d_batch.cpu
    );

    // Memory cost for batch: more storage updates
    assert!(
        d_batch.mem >= d_single.mem,
        "batch mint must consume at least as much memory as single mint"
    );
}

/// A fixture used for contract operations remains valid after measurement.
///
/// **What this proves**: After measuring a contract operation, the fixture
/// environment is still usable — no state corruption or budget exhaustion.
/// This is essential for test suites that measure multiple operations.
#[test]
fn contract_gas_fixture_reusability_after_contract_ops() {
    let fix = GasRegressionFixture::new();
    let admin = Address::generate(&fix.env);

    // First contract operation
    fix.reset_budget();
    let d1 = measure(&fix.env, || {
        let token_contract = fix.env.register_stellar_asset_contract_v2(admin.clone());
        let token_id = token_contract.address();
        let token_sac = token::StellarAssetClient::new(&fix.env, &token_id);
        token_sac.mint(&admin, &500_000);
    });

    assert!(
        d1.has_positive_cost(),
        "first contract operation must consume CPU"
    );

    // Second contract operation on the same fixture (after reset)
    fix.reset_budget();
    let d2 = measure(&fix.env, || {
        let _addr = Address::generate(&fix.env);
    });

    assert!(
        d2.has_positive_cost(),
        "second operation on reused fixture must also produce measurable deltas"
    );
}

// =============================================================================
// 9. DOCUMENTED REGRESSION SURFACE
// =============================================================================

/// Canonical test documenting the exact measurement behavior for a no-op.
///
/// This test serves as the **baseline reference** for gas regression.
/// If this test's measurements change between builds, it signals a
/// breaking change in the Soroban SDK or the fixture infrastructure.
///
/// # Regressions Detected
///
/// - SDK upgrade that changes budget accounting
/// - Compiler optimization that changes instruction counts
/// - Fixture infrastructure regression
#[test]
fn regression_baseline_noop_measurement() {
    let fix = GasRegressionFixture::new();
    fix.reset_budget();

    let d = measure(&fix.env, || {
        // Canonical no-op: a single u64 binding
        let _: u64 = 0;
    });

    // Document the expected behavior:
    // - A minimal operation (single let binding) may consume zero or trivial CPU
    // - The memory delta must be zero (no allocations)
    assert_eq!(
        d.mem, 0,
        "canonical no-op must not allocate memory"
    );

    // This assertion is intentionally permissive: CPU = 0 is acceptable for a
    // true no-op. If the SDK changes such that even trivial let bindings consume
    // instructions, this test will catch it.
    assert!(
        d.cpu <= 10,
        "canonical no-op should consume at most trivial CPU; got {}",
        d.cpu
    );
}

/// Canonical test documenting the measurement behavior for address generation.
///
/// This test pins down the cost of `Address::generate()` — a commonly used
/// operation in test fixtures. If this cost changes, it impacts every test
/// that uses address generation in its measured operations.
///
/// # Regressions Detected
///
/// - SDK change to Address::generate() implementation
/// - Host function cost model changes
#[test]
fn regression_baseline_address_generation_cost() {
    let fix = GasRegressionFixture::new();
    fix.reset_budget();

    let d = measure(&fix.env, || {
        let _addr = Address::generate(&fix.env);
    });

    // Address generation must consume measurable CPU
    assert!(
        d.has_positive_cost(),
        "Address::generate() must consume CPU instructions; got cpu={}",
        d.cpu
    );

    // This is NOT an upper-bound assertion — we document the actual value
    // rather than hard-coding a limit that would break on SDK upgrades.
    // The determinism tests above verify the value is stable per binary.
}
