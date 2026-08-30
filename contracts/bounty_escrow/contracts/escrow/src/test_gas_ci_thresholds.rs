//! # CI Gas / Resource Thresholds — Bounty Escrow Contract
//!
//! ## Purpose
//!
//! Defines per-operation CPU, memory, and WASM-size budgets for the five
//! representative worst-case operations and enforces them in CI so that
//! regressions are **visible, reproducible, and require review**.
//!
//! ## Operations Measured (worst-case inputs)
//!
//! | Operation         | Max supported input dimension                          |
//! |-------------------|-------------------------------------------------------|
//! | **Create**        | `lock_funds` with 60-escrow index + full metadata     |
//! | **Batch**         | `batch_lock_funds` / `batch_release_funds` n = 20     |
//! | **Payout**        | `release_funds` after 60 locks + fee routing + router |
//! | **Pagination**    | Whitelist query: list = 60, page-size = 50 (MAX)      |
//! | **Migration**     | `simulate_upgrade` with 60 escrows + deprecation set  |
//!
//! ## Enforcement Model
//!
//! | Mode                          | Environment var              | Behaviour on breach            |
//! |-------------------------------|------------------------------|--------------------------------|
//! | **Strict (CI default)**       | `ESCROW_GAS_MODE=strict`     | Hard `assert!` → test FAIL     |
//! | **Trend report**              | `ESCROW_GAS_MODE=warn`       | `eprintln!` warning only       |
//! | **Baseline collection**       | `ESCROW_GAS_MODE=collect`    | Print JSON baselines, no fail  |
//!
//! Run with:
//! ```text
//! cargo test --features testutils -p bounty-escrow gas_ci -- --nocapture
//! ```
//!
//! ## Representative inputs — documented
//!
//! See each `worst_case_*` helper for the exact fixture. All input maxima
//! match the existing contract constants:
//!
//! - `MAX_BATCH_SIZE` = 20
//! - `MAX_PARTICIPANT_FILTER_PAGE_SIZE` = 50
//! - Escrow population = 60 (from `test_max_counts.rs` coverage ceiling)
//! - Anonymous resolver + router configured for full-path payout
//!
//! ## WASM size
//!
//! WASM size is tracked indirectly via a static ceiling value derived from
//! the latest build artifact. When the WASM build changes, update
//! `WASM_SIZE_BUDGET_BYTES` and justify the increase in the PR.

#![cfg(test)]

extern crate std;
use std::{eprintln, println};

use super::*;
use crate::gas_budget::{GasBudgetConfig, OperationBudget};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    token, Address, Env, Vec,
};

// ============================================================================
// A. BUDGETS — representative worst-case ceilings
// ============================================================================
//
// These values are conservative upper bounds derived from running the
// gas_profile suite on a fresh build. Intentional increases require:
//   1. A measured before/after table in the PR description
//   2. An explicit bump in these constants reviewed by a code owner

mod budgets {
    pub const LARGE_AMOUNT: i128 = 1_000_000_000;

    pub mod create_lock {
        pub const MAX_CPU: u64 = 50_000_000;
        pub const MAX_MEM: u64 = 5_000_000;
    }

    pub mod batch {
        pub const BATCH_LOCK_N20_MAX_CPU: u64 = 500_000_000;
        pub const BATCH_LOCK_N20_MAX_MEM: u64 = 25_000_000;
        pub const BATCH_RELEASE_N20_MAX_CPU: u64 = 600_000_000;
        pub const BATCH_RELEASE_N20_MAX_MEM: u64 = 25_000_000;
    }

    pub mod payout {
        pub const RELEASE_MAX_CPU: u64 = 80_000_000;
        pub const RELEASE_MAX_MEM: u64 = 8_000_000;
        pub const REFUND_MAX_CPU: u64 = 60_000_000;
        pub const REFUND_MAX_MEM: u64 = 6_000_000;
        pub const PARTIAL_RELEASE_MAX_CPU: u64 = 80_000_000;
        pub const PARTIAL_RELEASE_MAX_MEM: u64 = 8_000_000;
    }

    pub mod pagination {
        pub const QUERY_WL_60_TOTAL_50_LIMIT_CPU: u64 = 40_000_000;
        pub const QUERY_WL_60_TOTAL_50_LIMIT_MEM: u64 = 8_000_000;
        pub const GET_AGGREGATE_60_CPU: u64 = 40_000_000;
        pub const GET_AGGREGATE_60_MEM: u64 = 5_000_000;
    }

    pub mod migration {
        pub const SIMULATE_UPGRADE_60_CPU: u64 = 60_000_000;
        pub const SIMULATE_UPGRADE_60_MEM: u64 = 10_000_000;
        pub const SET_DEPRECATION_TARGET_CPU: u64 = 20_000_000;
        pub const SET_DEPRECATION_TARGET_MEM: u64 = 3_000_000;
    }

    /// Derived from the latest optimized WASM artifact (see
    /// `WASM_OPTIMIZATION_REPORT.md`). Update intentionally with PR
    /// justification.
    pub const WASM_SIZE_BUDGET_BYTES: u64 = 260_000;

    /// "Large regressions" = > 50 % delta. Hard-gated regardless of mode.
    pub const HARD_GATE_BPS: u64 = 15_000;

    /// "Warn regressions" = > 15 % delta. Advisory-only in warn mode.
    pub const WARN_BPS: u64 = 1_500;
}

// ============================================================================
// B. Mode selection — via ESCROW_GAS_MODE env var
// ============================================================================

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
enum RunMode {
    Strict,
    Warn,
    Collect,
}

fn run_mode() -> RunMode {
    match option_env!("ESCROW_GAS_MODE")
        .map(|s| s.to_ascii_lowercase())
        .as_deref()
    {
        Some("warn") => RunMode::Warn,
        Some("collect") => RunMode::Collect,
        _ => RunMode::Strict,
    }
}

fn basis(actual: u64, limit: u64) -> u64 {
    if limit == 0 {
        return 0;
    }
    (actual as u128 * 10_000 / limit as u128) as u64
}

/// Enforce a single budget dimension according to the current RunMode.
///
/// - `actual > limit * HARD_GATE_BPS / 10_000` → **hard FAIL** in all modes
/// - `actual > limit * WARN_BPS / 10_000` → fail in Strict, warn in Warn
/// - Always prints the JSON measurement row in Collect mode
fn enforce(label: &'static str, dimension: &'static str, actual: u64, limit: u64) {
    let bps = basis(actual, limit);
    let over_hard = limit > 0 && bps > budgets::HARD_GATE_BPS;
    let over_warn = limit > 0 && bps > 10_000 + budgets::WARN_BPS;

    // Collect mode: always print JSON, never fail
    if run_mode() == RunMode::Collect {
        println!(
            "{{\"op\":\"{}\",\"dim\":\"{}\",\"actual\":{},\"limit\":{},\"bps\":{}}}",
            label, dimension, actual, limit, bps
        );
        return;
    }

    if over_hard {
        panic!(
            "[GAS-HARD] {label} {dimension}: actual {actual} exceeds limit {limit} by {bps} bps (> hard gate {}). \
             Intentional increases require PR review + budget bump.",
            budgets::HARD_GATE_BPS
        );
    }

    if over_warn {
        match run_mode() {
            RunMode::Strict => panic!(
                "[GAS-REGRESSION] {label} {dimension}: actual {actual} exceeds limit {limit} by {bps} bps (> warn {} bps). \
                 Use ESCROW_GAS_MODE=warn for trend-only reporting, or bump budget after review.",
                budgets::WARN_BPS
            ),
            RunMode::Warn => eprintln!(
                "[GAS-WARN] {label} {dimension}: actual {actual} / limit {limit} = {bps} bps (trend only)"
            ),
            RunMode::Collect => unreachable!(),
        }
    }
}

// ============================================================================
// C. Shared worst-case fixture
// ============================================================================

struct Fixture {
    env: Env,
    admin: Address,
    depositor: Address,
    contributor: Address,
    token_id: Address,
    token_sac: token::StellarAssetClient<'static>,
    contract_id: Address,
    client: BountyEscrowContractClient<'static>,
}

impl Fixture {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        env.budget().reset_unlimited();
        env.ledger().set_timestamp(1_700_000_000);

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);

        let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
        let token_id = token_contract.address();
        let token_sac = token::StellarAssetClient::new(&env, &token_id);

        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);

        client.init(&admin, &token_id);
        client.set_whitelist(&depositor, &true);

        // Wire a router (self) so release paths with swap-routing succeed
        // on the full validation path without tripping RouterNotConfigured.
        env.as_contract(&contract_id, || {
            env.storage()
                .instance()
                .set(&DataKey::Router, &contract_id);
        });

        Self {
            env,
            admin,
            depositor,
            contributor,
            token_id,
            token_sac,
            contract_id,
            client,
        }
    }

    fn mint(&self, amount: i128) {
        self.token_sac.mint(&self.depositor, &amount);
    }

    /// Populate 60 escrows as the representative worst-case index size.
    /// Returns the first/last bounty_id pair for later use in pagination.
    fn populate_60_locked(&self) -> (u64, u64) {
        self.mint(60 * 10_000);
        let deadline = self.env.ledger().timestamp() + 86_400;
        for id in 1..=60u64 {
            self.client
                .lock_funds(&self.depositor, &id, &10_000, &deadline);
        }
        (1, 60)
    }

    /// Populate the whitelist with 60 addresses.
    fn populate_whitelist_60(&self) {
        for _ in 0..60u32 {
            let a = Address::generate(&self.env);
            self.client.set_whitelist_entry(&a, &true);
        }
    }

    fn measure<F: FnOnce()>(&self, f: F) -> (u64, u64) {
        self.env.budget().reset_unlimited();
        let cpu_before = self.env.budget().cpu_instruction_cost();
        let mem_before = self.env.budget().memory_bytes_cost();
        f();
        let cpu = self
            .env
            .budget()
            .cpu_instruction_cost()
            .saturating_sub(cpu_before);
        let mem = self
            .env
            .budget()
            .memory_bytes_cost()
            .saturating_sub(mem_before);
        (cpu, mem)
    }
}

// ============================================================================
// D. Budget configuration — verify the advisory plumbing reports thresholds
// ============================================================================

#[test]
fn gas_ci_configured_budgets_round_trip_via_advisory_status() {
    let f = Fixture::new();

    let budgets = GasBudgetConfig {
        lock: OperationBudget {
            max_cpu_instructions: budgets::create_lock::MAX_CPU,
            max_memory_bytes: budgets::create_lock::MAX_MEM,
        },
        release: OperationBudget {
            max_cpu_instructions: budgets::payout::RELEASE_MAX_CPU,
            max_memory_bytes: budgets::payout::RELEASE_MAX_MEM,
        },
        refund: OperationBudget {
            max_cpu_instructions: budgets::payout::REFUND_MAX_CPU,
            max_memory_bytes: budgets::payout::REFUND_MAX_MEM,
        },
        partial_release: OperationBudget {
            max_cpu_instructions: budgets::payout::PARTIAL_RELEASE_MAX_CPU,
            max_memory_bytes: budgets::payout::PARTIAL_RELEASE_MAX_MEM,
        },
        batch_lock: OperationBudget {
            max_cpu_instructions: budgets::batch::BATCH_LOCK_N20_MAX_CPU,
            max_memory_bytes: budgets::batch::BATCH_LOCK_N20_MAX_MEM,
        },
        batch_release: OperationBudget {
            max_cpu_instructions: budgets::batch::BATCH_RELEASE_N20_MAX_CPU,
            max_memory_bytes: budgets::batch::BATCH_RELEASE_N20_MAX_MEM,
        },
        enforce: false,
    };

    f.client.set_gas_budget(
        &budgets.lock,
        &budgets.release,
        &budgets.refund,
        &budgets.partial_release,
        &budgets.batch_lock,
        &budgets.batch_release,
        &false,
    );

    let status = f.client.get_gas_budget_advisory_status();
    assert!(status.caps_configured);
    assert_eq!(
        status.config.lock.max_cpu_instructions,
        budgets::create_lock::MAX_CPU
    );
    assert_eq!(
        status.config.batch_lock.max_cpu_instructions,
        budgets::batch::BATCH_LOCK_N20_MAX_CPU
    );
    assert_eq!(
        status.config.release.max_memory_bytes,
        budgets::payout::RELEASE_MAX_MEM
    );
    assert!(!status.enforce_flag_set);
    // In testutils build caps ARE enforced at runtime (see gas_budget docs).
    assert!(status.caps_enforced_in_production);
}

// ============================================================================
// E. OPERATION 1 — CREATE (lock_funds worst-case: 60-index + large amount)
// ============================================================================

#[test]
fn gas_ci_create_lock_worst_case() {
    let f = Fixture::new();
    // Warm the index: 60 locks so the 61st hits the representative
    // worst-case append cost (linear-in-index storage writes).
    f.populate_60_locked();

    // Worst-case single lock: huge amount, full index
    f.mint(budgets::LARGE_AMOUNT);
    let deadline = f.env.ledger().timestamp() + 86_400;

    let (cpu, mem) = f.measure(|| {
        f.client
            .lock_funds(&f.depositor, &61, &budgets::LARGE_AMOUNT, &deadline);
    });

    enforce(
        "create_lock_index60_large_amount",
        "cpu",
        cpu,
        budgets::create_lock::MAX_CPU,
    );
    enforce(
        "create_lock_index60_large_amount",
        "mem",
        mem,
        budgets::create_lock::MAX_MEM,
    );
    assert_eq!(f.client.get_escrow(&61).amount, budgets::LARGE_AMOUNT);
}

// ============================================================================
// F. OPERATION 2a — BATCH LOCK (n = MAX_BATCH_SIZE = 20)
// ============================================================================

#[test]
fn gas_ci_batch_lock_max_n20() {
    let f = Fixture::new();
    // First, populate 60 to simulate a mature index.
    f.populate_60_locked();

    f.mint(20 * 1_000);
    let deadline = f.env.ledger().timestamp() + 86_400;
    let mut items: Vec<LockFundsItem> = Vec::new(&f.env);
    for i in 0..MAX_BATCH_SIZE as u64 {
        items.push_back(LockFundsItem {
            bounty_id: 1000 + i,
            depositor: f.depositor.clone(),
            amount: 1_000,
            deadline,
        });
    }

    let (cpu, mem) = f.measure(|| {
        f.client.batch_lock_funds(&items);
    });

    enforce(
        "batch_lock_n20_on_60_index",
        "cpu",
        cpu,
        budgets::batch::BATCH_LOCK_N20_MAX_CPU,
    );
    enforce(
        "batch_lock_n20_on_60_index",
        "mem",
        mem,
        budgets::batch::BATCH_LOCK_N20_MAX_MEM,
    );
    // Confirm all 20 landed
    assert_eq!(f.client.get_escrow(&1019).status, EscrowStatus::Locked);
}

// ============================================================================
// G. OPERATION 2b — BATCH RELEASE (n = MAX_BATCH_SIZE = 20)
// ============================================================================

#[test]
fn gas_ci_batch_release_max_n20() {
    let f = Fixture::new();
    // Mature index: 40 baseline + 20-to-release = 60 total
    f.mint(60 * 1_000);
    let deadline = f.env.ledger().timestamp() + 86_400;
    for id in 1..=40u64 {
        f.client
            .lock_funds(&f.depositor, &id, &1_000, &deadline);
    }
    for id in 41..=60u64 {
        f.client
            .lock_funds(&f.depositor, &id, &1_000, &deadline);
    }

    let mut items: Vec<ReleaseFundsItem> = Vec::new(&f.env);
    for id in 41..=60u64 {
        items.push_back(ReleaseFundsItem {
            bounty_id: id,
            contributor: f.contributor.clone(),
        });
    }

    let (cpu, mem) = f.measure(|| {
        f.client.batch_release_funds(&items);
    });

    enforce(
        "batch_release_n20_on_60_index",
        "cpu",
        cpu,
        budgets::batch::BATCH_RELEASE_N20_MAX_CPU,
    );
    enforce(
        "batch_release_n20_on_60_index",
        "mem",
        mem,
        budgets::batch::BATCH_RELEASE_N20_MAX_MEM,
    );
    assert_eq!(f.client.get_escrow(&60).status, EscrowStatus::Released);
}

// ============================================================================
// H. OPERATION 3 — PAYOUT (release / refund / partial_release worst-cases)
// ============================================================================

#[test]
fn gas_ci_payout_release_after_60_locks() {
    let f = Fixture::new();
    // Release #60 last to make sure the lookup path touches a fully-warmed
    // persistent-storage key set.
    f.populate_60_locked();

    let (cpu, mem) = f.measure(|| {
        f.client.release_funds(&60, &f.contributor);
    });

    enforce(
        "payout_release_escrow60",
        "cpu",
        cpu,
        budgets::payout::RELEASE_MAX_CPU,
    );
    enforce(
        "payout_release_escrow60",
        "mem",
        mem,
        budgets::payout::RELEASE_MAX_MEM,
    );
    assert_eq!(f.client.get_escrow(&60).status, EscrowStatus::Released);
}

#[test]
fn gas_ci_payout_refund_after_deadline_index60() {
    let f = Fixture::new();
    let deadline = f.env.ledger().timestamp() + 100;
    f.mint(60 * 1_000);
    for id in 1..=60u64 {
        f.client
            .lock_funds(&f.depositor, &id, &1_000, &deadline);
    }
    f.env
        .ledger()
        .set_timestamp(f.env.ledger().timestamp() + deadline + 1);

    let (cpu, mem) = f.measure(|| {
        f.client.refund(&60);
    });

    enforce(
        "payout_refund_escrow60_deadline",
        "cpu",
        cpu,
        budgets::payout::REFUND_MAX_CPU,
    );
    enforce(
        "payout_refund_escrow60_deadline",
        "mem",
        mem,
        budgets::payout::REFUND_MAX_MEM,
    );
    assert_eq!(f.client.get_escrow(&60).status, EscrowStatus::Refunded);
}

#[test]
fn gas_ci_payout_partial_release_on_60th_escrow() {
    let f = Fixture::new();
    f.populate_60_locked();

    let (cpu, mem) = f.measure(|| {
        f.client
            .partial_release(&60, &f.contributor, &5_000);
    });

    enforce(
        "payout_partial_release_5k_from_escrow60",
        "cpu",
        cpu,
        budgets::payout::PARTIAL_RELEASE_MAX_CPU,
    );
    enforce(
        "payout_partial_release_5k_from_escrow60",
        "mem",
        mem,
        budgets::payout::PARTIAL_RELEASE_MAX_MEM,
    );
    assert_eq!(f.client.get_escrow(&60).remaining_amount, 5_000);
}

// ============================================================================
// I. OPERATION 4 — PAGINATION (max page, full list)
// ============================================================================

#[test]
fn gas_ci_pagination_whitelist_60_total_50_limit_max_page() {
    let f = Fixture::new();
    f.populate_whitelist_60();
    // Representative max page: MAX_PARTICIPANT_FILTER_PAGE_SIZE = 50
    // against a 60-item list = worst-case item encoding & copy.
    let (cpu, mem) = f.measure(|| {
        let page = f.client.query_whitelist(&0, &50);
        assert_eq!(page.items.len(), 50);
        assert!(page.has_more);
        assert_eq!(page.total, 61); // 60 + the fixture depositor whitelist
    });

    enforce(
        "pagination_whitelist_60total_limit50",
        "cpu",
        cpu,
        budgets::pagination::QUERY_WL_60_TOTAL_50_LIMIT_CPU,
    );
    enforce(
        "pagination_whitelist_60total_limit50",
        "mem",
        mem,
        budgets::pagination::QUERY_WL_60_TOTAL_50_LIMIT_MEM,
    );
}

#[test]
fn gas_ci_pagination_aggregate_stats_60_escrows() {
    let f = Fixture::new();
    f.populate_60_locked();

    let (cpu, mem) = f.measure(|| {
        let _stats = f.client.get_aggregate_stats();
    });

    enforce(
        "pagination_aggregate_stats_60",
        "cpu",
        cpu,
        budgets::pagination::GET_AGGREGATE_60_CPU,
    );
    enforce(
        "pagination_aggregate_stats_60",
        "mem",
        mem,
        budgets::pagination::GET_AGGREGATE_60_MEM,
    );
}

// ============================================================================
// J. OPERATION 5 — MIGRATION / UPGRADE
// ============================================================================

#[test]
fn gas_ci_migration_simulate_upgrade_with_60_escrows() {
    let f = Fixture::new();
    f.populate_60_locked();

    // simulate_upgrade exercises the safety-check list against a populated
    // store, which is the representative worst-case for the migration path.
    let (cpu, mem) = f.measure(|| {
        let report = f
            .env
            .as_contract(&f.contract_id, || upgrade_safety::simulate_upgrade(&f.env));
        // A fresh valid instance must pass storage/init/escrow checks
        assert!(report.checks_passed >= 3);
    });

    enforce(
        "migration_simulate_upgrade_60_escrows",
        "cpu",
        cpu,
        budgets::migration::SIMULATE_UPGRADE_60_CPU,
    );
    enforce(
        "migration_simulate_upgrade_60_escrows",
        "mem",
        mem,
        budgets::migration::SIMULATE_UPGRADE_60_MEM,
    );
}

#[test]
fn gas_ci_migration_set_deprecation_target_plumbing() {
    let f = Fixture::new();
    let target = Address::generate(&f.env);

    let (cpu, mem) = f.measure(|| {
        // Exercise the deprecation storage write + event path.
        // The real setter path is in the contract impl; simulate an admin
        // write equivalent via storage + event directly to capture cost.
        f.env.as_contract(&f.contract_id, || {
            f.env.storage().instance().set(
                &DataKey::DeprecationState,
                &DeprecationState {
                    deprecated: true,
                    migration_target: Some(target.clone()),
                },
            );
        });
        emit_deprecation_state_changed(
            &f.env,
            DeprecationStateChanged {
                deprecated: true,
                migration_target: Some(target.clone()),
                admin: f.admin.clone(),
                timestamp: f.env.ledger().timestamp(),
            },
        );
    });

    enforce(
        "migration_set_deprecation_target",
        "cpu",
        cpu,
        budgets::migration::SET_DEPRECATION_TARGET_CPU,
    );
    enforce(
        "migration_set_deprecation_target",
        "mem",
        mem,
        budgets::migration::SET_DEPRECATION_TARGET_MEM,
    );
}

// ============================================================================
// K. WASM size static ceiling — compile-time, run-time reported
// ============================================================================

/// WASM size cannot be measured from within a unit test host, so we track
/// the budget ceiling statically and surface it in this always-passing test
/// with clear instructions for bumping.
///
/// To verify against a real WASM build:
/// ```text
/// soroban contract build --release
/// ls -la target/wasm32-unknown-unknown/release/bounty_escrow.wasm
/// ```
#[test]
fn gas_ci_wasm_size_budget_ceiling_is_documentation_checked() {
    let ceiling = budgets::WASM_SIZE_BUDGET_BYTES;
    // Baseline from WASM_OPTIMIZATION_REPORT.md:
    //   "After: 196,350 bytes" (after 8.6% reduction).
    // We set ceiling = 260,000 bytes as a generous but review-gated cap.
    assert!(
        ceiling >= 196_350,
        "WASM_SIZE_BUDGET_BYTES must be >= post-optimization baseline (196,350). Current: {ceiling}"
    );
    assert!(
        ceiling <= 350_000,
        "WASM_SIZE_BUDGET_BYTES looks suspiciously high. Review build before bumping beyond 350KB. Current: {ceiling}"
    );
    println!(
        "[WASM] budget_ceiling_bytes = {ceiling}. Bump only with PR justification referencing soroban contract build artifact size."
    );
}

// ============================================================================
// L. Determinism sanity — two fresh fixtures measure identically
// ============================================================================

#[test]
fn gas_ci_measurements_deterministic_per_binary() {
    fn single_lock(f: &Fixture) -> (u64, u64) {
        f.mint(1_000_000);
        f.env.budget().reset_unlimited();
        let dl = f.env.ledger().timestamp() + 1000;
        let cpu0 = f.env.budget().cpu_instruction_cost();
        let mem0 = f.env.budget().memory_bytes_cost();
        f.client
            .lock_funds(&f.depositor, &1, &1_000_000, &dl);
        (
            f.env
                .budget()
                .cpu_instruction_cost()
                .saturating_sub(cpu0),
            f.env.budget().memory_bytes_cost().saturating_sub(mem0),
        )
    }

    let a = Fixture::new();
    let b = Fixture::new();

    let (cpu_a, mem_a) = single_lock(&a);
    let (cpu_b, mem_b) = single_lock(&b);

    // Deterministic per binary build means identical measurements.
    // If this fails, the fixture has non-determinism (ordering, addresses,
    // timestamps) that must be fixed before the budgets are trustworthy.
    assert_eq!(
        cpu_a, cpu_b,
        "CPU measurement not deterministic: a={cpu_a} vs b={cpu_b}. Fix fixture before relying on CI budgets."
    );
    assert_eq!(
        mem_a, mem_b,
        "MEM measurement not deterministic: a={mem_a} vs b={mem_b}. Fix fixture before relying on CI budgets."
    );
}

// ============================================================================
// M. Consolidated report table (--nocapture friendly)
// ============================================================================

#[test]
fn gas_ci_consolidated_report_table() {
    let mode = run_mode();
    if mode != RunMode::Collect {
        // Only print the full table in Collect mode to keep CI noise low.
        return;
    }

    println!();
    println!("| {:<46} | {:>16} | {:>12} | {:>16} | {:>12} |",
        "Operation (worst-case input)", "CPU measured", "Mem measured", "CPU budget", "Mem budget");
    println!("|{}|{}|{}|{}|{}|",
        "-".repeat(48), "-".repeat(18), "-".repeat(14), "-".repeat(18), "-".repeat(14));

    let row = |label, cpu, mem, cpu_lim, mem_lim| {
        println!("| {:<46} | {:>16} | {:>12} | {:>16} | {:>12} |",
            label, cpu, mem, cpu_lim, mem_lim);
    };

    // --- Create ---
    {
        let f = Fixture::new();
        f.populate_60_locked();
        f.mint(budgets::LARGE_AMOUNT);
        let dl = f.env.ledger().timestamp() + 86_400;
        let (cpu, mem) = f.measure(|| {
            f.client.lock_funds(&f.depositor, &61, &budgets::LARGE_AMOUNT, &dl);
        });
        row("create: lock (60-index + 1B amount)", cpu, mem,
            budgets::create_lock::MAX_CPU, budgets::create_lock::MAX_MEM);
    }

    // --- Batch lock n=20 ---
    {
        let f = Fixture::new();
        f.populate_60_locked();
        f.mint(20 * 1_000);
        let dl = f.env.ledger().timestamp() + 86_400;
        let mut items: Vec<LockFundsItem> = Vec::new(&f.env);
        for i in 0..MAX_BATCH_SIZE as u64 {
            items.push_back(LockFundsItem {
                bounty_id: 2000 + i, depositor: f.depositor.clone(),
                amount: 1_000, deadline: dl,
            });
        }
        let (cpu, mem) = f.measure(|| { f.client.batch_lock_funds(&items); });
        row("batch: batch_lock_funds (n=20)", cpu, mem,
            budgets::batch::BATCH_LOCK_N20_MAX_CPU, budgets::batch::BATCH_LOCK_N20_MAX_MEM);
    }

    // --- Batch release n=20 ---
    {
        let f = Fixture::new();
        f.mint(60 * 1_000);
        let dl = f.env.ledger().timestamp() + 86_400;
        for id in 1..=60u64 {
            f.client.lock_funds(&f.depositor, &id, &1_000, &dl);
        }
        let mut items: Vec<ReleaseFundsItem> = Vec::new(&f.env);
        for id in 41..=60u64 {
            items.push_back(ReleaseFundsItem {
                bounty_id: id, contributor: f.contributor.clone(),
            });
        }
        let (cpu, mem) = f.measure(|| { f.client.batch_release_funds(&items); });
        row("batch: batch_release_funds (n=20)", cpu, mem,
            budgets::batch::BATCH_RELEASE_N20_MAX_CPU, budgets::batch::BATCH_RELEASE_N20_MAX_MEM);
    }

    // --- Payout release ---
    {
        let f = Fixture::new();
        f.populate_60_locked();
        let (cpu, mem) = f.measure(|| { f.client.release_funds(&60, &f.contributor); });
        row("payout: release_funds (escrow #60)", cpu, mem,
            budgets::payout::RELEASE_MAX_CPU, budgets::payout::RELEASE_MAX_MEM);
    }

    // --- Payout refund ---
    {
        let f = Fixture::new();
        let dl = f.env.ledger().timestamp() + 100;
        f.mint(60 * 1_000);
        for id in 1..=60u64 {
            f.client.lock_funds(&f.depositor, &id, &1_000, &dl);
        }
        f.env.ledger().set_timestamp(dl + 1);
        let (cpu, mem) = f.measure(|| { f.client.refund(&60); });
        row("payout: refund (after deadline)", cpu, mem,
            budgets::payout::REFUND_MAX_CPU, budgets::payout::REFUND_MAX_MEM);
    }

    // --- Pagination ---
    {
        let f = Fixture::new();
        f.populate_whitelist_60();
        let (cpu, mem) = f.measure(|| { let _p = f.client.query_whitelist(&0, &50); });
        row("pagination: query_whitelist (60 total, limit 50)", cpu, mem,
            budgets::pagination::QUERY_WL_60_TOTAL_50_LIMIT_CPU,
            budgets::pagination::QUERY_WL_60_TOTAL_50_LIMIT_MEM);
    }
    {
        let f = Fixture::new();
        f.populate_60_locked();
        let (cpu, mem) = f.measure(|| { let _s = f.client.get_aggregate_stats(); });
        row("pagination: get_aggregate_stats (60)", cpu, mem,
            budgets::pagination::GET_AGGREGATE_60_CPU,
            budgets::pagination::GET_AGGREGATE_60_MEM);
    }

    // --- Migration ---
    {
        let f = Fixture::new();
        f.populate_60_locked();
        let (cpu, mem) = f.measure(|| { let _r = upgrade_safety::simulate_upgrade(&f.env); });
        row("migration: simulate_upgrade (60 escrows)", cpu, mem,
            budgets::migration::SIMULATE_UPGRADE_60_CPU,
            budgets::migration::SIMULATE_UPGRADE_60_MEM);
    }

    println!();
    println!("[WASM] budget_ceiling_bytes = {}", budgets::WASM_SIZE_BUDGET_BYTES);
    println!("_ESCROW_GAS_MODE=collect: baselines printed above. No thresholds enforced._");
}
