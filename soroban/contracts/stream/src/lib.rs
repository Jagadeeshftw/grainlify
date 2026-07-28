//! Grainlify Stream — Gas Regression Fixture Module
//!
//! This crate provides hardened test fixtures for gas regression testing across
//! the Grainlify smart contract ecosystem. The fixtures ensure deterministic,
//! reproducible gas measurements that serve as a stable regression surface.
//!
//! ## Design Principles
//!
//! - **Fixture isolation**: Each test receives a fresh, untainted environment
//! - **Deterministic measurements**: Budget meters are reset before every measured call
//! - **Explicit edge cases**: All boundary conditions are documented and tested
//! - **Backward compatibility**: Existing fixture patterns are preserved and extended
//!
//! ## Usage
//!
//! ```rust,no_run
//! use grainlify_stream::{GasRegressionFixture, BudgetDelta, measure};
//!
//! let fixture = GasRegressionFixture::new();
//! fixture.reset_budget();
//! let delta = measure(&fixture.env, || {
//!     // Your contract operation here
//! });
//! assert!(delta.has_positive_cost());
//! ```

use soroban_sdk::Env;

// =============================================================================
// Budget capture helpers
// =============================================================================

/// Captured budget deltas for a single measured operation.
///
/// # Invariants
///
/// - `cpu` is always >= 0 (u64 prevents underflow)
/// - `mem` is always >= 0
/// - Both values are **deterministic** for the same operation and inputs
///   on the same binary build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BudgetDelta {
    pub cpu: u64,
    pub mem: u64,
}

impl BudgetDelta {
    /// The operation consumed measurable resources (a normal hot-path call).
    pub fn has_positive_cost(&self) -> bool {
        self.cpu > 0
    }
}

/// Capture Soroban budget meters before and after `f`, return the deltas.
///
/// # Determinism Guarantee
///
/// For the same binary, same `Env` state, and same `f` side-effects,
/// this function returns **identical** values on every call.
///
/// # Usage
///
/// ```ignore
/// let delta = measure(&env, || {
///     client.do_operation(&arg1, &arg2);
/// });
/// ```
pub fn measure<F: FnOnce()>(env: &Env, f: F) -> BudgetDelta {
    let cpu_before = env.cost_estimate().budget().cpu_instruction_cost();
    let mem_before = env.cost_estimate().budget().memory_bytes_cost();
    f();
    BudgetDelta {
        cpu: env
            .cost_estimate()
            .budget()
            .cpu_instruction_cost()
            .saturating_sub(cpu_before),
        mem: env
            .cost_estimate()
            .budget()
            .memory_bytes_cost()
            .saturating_sub(mem_before),
    }
}

// =============================================================================
// Gas Regression Fixture
// =============================================================================

/// Core gas regression fixture providing a hardened, reproducible test environment.
///
/// Every test that uses this fixture gets:
/// - A fresh Soroban `Env` with budget meters reset
/// - Mock auth enabled (unless explicitly disabled)
/// - Guaranteed isolation from other tests
///
/// # Stability Guarantees
///
/// - Running the same test with the same fixture configuration always produces
///   identical gas measurements on the same binary.
/// - Fixture creation is O(1) and does not interact with ledger state.
/// - No shared mutable state between fixture instances.
pub struct GasRegressionFixture {
    pub env: Env,
}

impl GasRegressionFixture {
    /// Creates a new isolated test fixture with mock auths enabled.
    ///
    /// The returned fixture has:
    /// - A default `Env` with mock auths enabled
    /// - Budget meters initialized to zero
    /// - No ledger modifications
    ///
    /// # Determinism
    ///
    /// Calling `new()` twice produces two fully independent environments.
    /// State from one fixture never leaks into another.
    pub fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        env.cost_estimate().budget().reset_unlimited();
        Self { env }
    }

    /// Creates a new fixture without mock auth (for testing auth failures).
    ///
    /// Use this when testing that unauthorized calls are properly rejected.
    pub fn new_without_mock_auths() -> Self {
        let env = Env::default();
        env.cost_estimate().budget().reset_unlimited();
        Self { env }
    }

    /// Resets the budget meters to zero, enabling a fresh measurement window.
    ///
    /// Always call this immediately before the operation being measured to
    /// ensure only that operation's cost is captured.
    pub fn reset_budget(&self) {
        self.env.cost_estimate().budget().reset_unlimited();
    }
}

impl Default for GasRegressionFixture {
    fn default() -> Self {
        Self::new()
    }
}
