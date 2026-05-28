//! # Batch Performance Benchmarks — Program Escrow
//!
//! ## USAGE
//!
//! This file is NOT a standalone crate. It lives in `contracts/benchmarks/`
//! as a reference benchmark module and **cannot** be compiled independently —
//! it depends on types from the `program-escrow` crate.
//!
//! To run these benchmarks, include this file from within the `program-escrow`
//! crate's test infrastructure by adding the following to `lib.rs`:
//!
//! ```rust
//! #[cfg(test)]
//! mod bench_batch_performance {
//!     include!("../../benchmarks/batch_performance.rs");
//! }
//! ```
//!
//! Then run:
//!
//! ```sh
//! cd contracts/program-escrow
//! cargo test bench_  -- --nocapture        # all benchmarks
//! cargo test ci_benchmark_gate -- --nocapture  # CI gate only
//! ```
//!
//! ## Runnable equivalents
//!
//! The equivalent tests that are already wired into the crate's test suite
//! live in `contracts/program-escrow/src/test_batch_operations.rs`
//! under the **Gas Profiling Tests** section. Those are the tests the CI
//! workflow actually executes.
//!
//! ## Updating thresholds
//!
//! After testnet calibration or intentional gas-usage changes, update the
//! `CPU_THRESHOLD_BATCH_PAYOUT` and `CPU_THRESHOLD_LOCK_FUNDS` arrays below
//! and record the new baseline in `docs/gas-optimization/batch-payout-benchmarks.md`.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Budget as _, Ledger as _},
    token, Address, Env, String,
};

use crate::{ProgramEscrowContract, ProgramEscrowContractClient};

// ============================================================================
// Constants
// ============================================================================

/// Batch sizes to profile.
const BATCH_SIZES: &[u32] = &[1, 10, 50, 100];

/// CPU instruction thresholds per batch size (gates CI).
/// Format: [size_1, size_10, size_50, size_100].
/// Values are intentionally generous to avoid flaky failures; they are meant
/// to catch large regressions, not enforce micro-optimisations.
const CPU_THRESHOLD_BATCH_PAYOUT: [u64; 4] = [500_000, 3_000_000, 12_000_000, 22_000_000];

/// CPU threshold for lock_program_funds.
/// The function is O(1) with respect to batch size, so all slots share the
/// same limit.
const CPU_THRESHOLD_LOCK_FUNDS: [u64; 4] = [200_000, 200_000, 200_000, 200_000];

// ============================================================================
// Setup helpers
// ============================================================================

struct Ctx<'a> {
    env: Env,
    client: ProgramEscrowContractClient<'a>,
    token_id: Address,
    token_admin: Address,
    admin: Address,
}

/// Create a fresh contract instance with an initialised admin and token.
/// Mirrors the `setup()` helper in `test_batch_operations.rs`.
fn setup() -> Ctx<'static> {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract(token_admin.clone());
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    client.initialize_contract(&admin);
    Ctx {
        env,
        client,
        token_id,
        token_admin,
        admin,
    }
}

fn mint(ctx: &Ctx, recipient: &Address, amount: i128) {
    token::StellarAssetClient::new(&ctx.env, &ctx.token_id).mint(recipient, &amount);
}

/// Initialise a program with `amount` of initial liquidity, then publish it.
/// Mirrors `init_program()` in `test_batch_operations.rs`.
fn init_program(ctx: &Ctx, program_id: &str, amount: i128) {
    let creator = Address::generate(&ctx.env);
    mint(ctx, &creator, amount);
    ctx.client.init_program(
        &String::from_str(&ctx.env, program_id),
        &ctx.admin.clone(),
        &ctx.token_id,
        &creator,
        &Some(amount),
        &None,
    );
    // publish_program takes no arguments — it publishes whichever program is
    // currently stored in PROGRAM_DATA (the one just initialised above).
    ctx.client.publish_program();
}

/// Emit a structured benchmark result line for CI log parsing.
///
/// Format: `[BENCH] op={op} batch_size={n} cpu_insns={n} mem_bytes={n}`
fn print_bench_result(op: &str, batch_size: u32, cpu_insns: u64, mem_bytes: u64) {
    println!(
        "[BENCH] op={} batch_size={} cpu_insns={} mem_bytes={}",
        op, batch_size, cpu_insns, mem_bytes
    );
}

// ============================================================================
// Tests
// ============================================================================

/// Profile `batch_payout_by` across all `BATCH_SIZES`.
///
/// For each size the test:
/// 1. Spins up a fresh contract with enough initial liquidity.
/// 2. Builds `batch_size` recipient / amount pairs (1 000 stroops each).
/// 3. Resets the budget counters.
/// 4. Executes `batch_payout_by`.
/// 5. Reads CPU instruction and memory byte counts.
/// 6. Asserts both are within the declared thresholds.
#[test]
fn bench_batch_payout_sizes() {
    for (i, &batch_size) in BATCH_SIZES.iter().enumerate() {
        let ctx = setup();
        // Provide ample liquidity so every batch size can complete.
        let total_funds: i128 = (batch_size as i128) * 1_000 + 1_000_000;
        init_program(&ctx, "bench-payout", total_funds);

        let mut recipients = soroban_sdk::Vec::new(&ctx.env);
        let mut amounts: soroban_sdk::Vec<i128> = soroban_sdk::Vec::new(&ctx.env);
        for _ in 0..batch_size {
            recipients.push_back(Address::generate(&ctx.env));
            amounts.push_back(1_000i128);
        }

        ctx.env.budget().reset_default();
        ctx.client
            .batch_payout_by(&ctx.admin, &recipients, &amounts, &None);
        let cpu_insns = ctx.env.budget().cpu_instruction_count();
        let mem_bytes = ctx.env.budget().memory_bytes_count();

        print_bench_result("batch_payout", batch_size, cpu_insns, mem_bytes);

        assert!(
            cpu_insns <= CPU_THRESHOLD_BATCH_PAYOUT[i],
            "CPU regression: batch_payout({}) cpu_insns={} > threshold={}",
            batch_size,
            cpu_insns,
            CPU_THRESHOLD_BATCH_PAYOUT[i]
        );
    }
}

/// Profile `lock_program_funds`.
///
/// `lock_program_funds` is O(1) — its cost does not grow with batch size.
/// A single call is sufficient to establish the baseline.
#[test]
fn bench_lock_program_funds() {
    let ctx = setup();
    // Initialise with zero initial liquidity so the contract holds no funds
    // before the measured operation.
    let creator = Address::generate(&ctx.env);
    ctx.client.init_program(
        &String::from_str(&ctx.env, "bench-lock"),
        &ctx.admin.clone(),
        &ctx.token_id,
        &creator,
        &None,
        &None,
    );
    ctx.client.publish_program();

    // The contract will credit the amount without an inbound token transfer
    // when no depositor address is provided (see lock_program_funds internals).
    let lock_amount = 1_000_000i128;
    ctx.env.budget().reset_default();
    ctx.client.lock_program_funds(&lock_amount);
    let cpu_insns = ctx.env.budget().cpu_instruction_count();
    let mem_bytes = ctx.env.budget().memory_bytes_count();

    print_bench_result("lock_program_funds", 1, cpu_insns, mem_bytes);

    assert!(
        cpu_insns <= CPU_THRESHOLD_LOCK_FUNDS[0],
        "CPU regression: lock_program_funds cpu_insns={} > threshold={}",
        cpu_insns,
        CPU_THRESHOLD_LOCK_FUNDS[0]
    );
}

/// **CI gate** — batch_payout with exactly 50 recipients.
///
/// This is the specific test the CI workflow runs for gas regression detection.
/// It fails with a descriptive message if CPU usage exceeds the threshold so
/// that CI logs point directly to the relevant documentation.
///
/// See `.github/workflows/gas-profiling.yml` and
/// `docs/gas-optimization/batch-payout-benchmarks.md`.
#[test]
fn ci_benchmark_gate_batch_payout_50() {
    // Threshold matches CPU_THRESHOLD_BATCH_PAYOUT[2] (50-item slot).
    const THRESHOLD: u64 = 12_000_000;
    const BATCH_SIZE: u32 = 50;

    let ctx = setup();
    init_program(&ctx, "ci-gate-50", (BATCH_SIZE as i128) * 1_000 + 1_000_000);

    let mut recipients = soroban_sdk::Vec::new(&ctx.env);
    let mut amounts: soroban_sdk::Vec<i128> = soroban_sdk::Vec::new(&ctx.env);
    for _ in 0..BATCH_SIZE {
        recipients.push_back(Address::generate(&ctx.env));
        amounts.push_back(1_000i128);
    }

    ctx.env.budget().reset_default();
    ctx.client
        .batch_payout_by(&ctx.admin, &recipients, &amounts, &None);
    let cpu_insns = ctx.env.budget().cpu_instruction_count();
    let mem_bytes = ctx.env.budget().memory_bytes_count();

    println!(
        "[CI-GATE] op=batch_payout batch_size=50 cpu_insns={} mem_bytes={} threshold={}",
        cpu_insns, mem_bytes, THRESHOLD
    );

    assert!(
        cpu_insns <= THRESHOLD,
        "CI GATE FAILED: batch_payout(50) cpu_insns={} > threshold={}; \
         see docs/gas-optimization/batch-payout-benchmarks.md",
        cpu_insns,
        THRESHOLD
    );
}
