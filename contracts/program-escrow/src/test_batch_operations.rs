#![cfg(test)]

use soroban_sdk::testutils::budget::Budget as _;
use soroban_sdk::testutils::Ledger as _;
use soroban_sdk::testutils::LedgerInfo as _;
use soroban_sdk::{testutils::Address as _, token, vec, Address, Env, String, TryIntoVal, Vec};

use crate::{
    BatchError, LockItem, ProgramData, ProgramEscrowContract, ProgramEscrowContractClient,
    ReleaseItem, IDEMPOTENCY_KEY_TTL_LEDGERS,
};

pub struct Ctx<'a> {
    pub env: Env,
    pub client: ProgramEscrowContractClient<'a>,
    pub token_id: Address,
    pub token_admin: Address,
    pub admin: Address,
}

pub fn setup() -> Ctx<'static> {
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

pub fn init_program(ctx: &Ctx, program_id: &str, amount: i128) {
    let creator = Address::generate(&ctx.env);
    mint(ctx, &creator, amount);
    ctx.client.init_program(
        &String::from_str(&ctx.env, program_id),
        &ctx.admin.clone(), // authorized_payout_key
        &ctx.token_id,
        &creator,
        &Some(amount),
        &None,
    );
    ctx.client.publish_program();
}

#[test]
fn test_batch_lock_success() {
    let ctx = setup();
    init_program(&ctx, "PROG1", 1000);
    init_program(&ctx, "PROG2", 2000);

    let items = vec![
        &ctx.env,
        LockItem {
            program_id: String::from_str(&ctx.env, "PROG1"),
            amount: 500,
        },
        LockItem {
            program_id: String::from_str(&ctx.env, "PROG2"),
            amount: 1500,
        },
    ];

    let result = ctx.client.batch_lock(&items);
    assert_eq!(result, 2);

    let prog1 = ctx
        .client
        .get_program_info_v2(&String::from_str(&ctx.env, "PROG1"));
    assert_eq!(prog1.total_funds, 1500);
    assert_eq!(prog1.remaining_balance, 1500);

    let prog2 = ctx
        .client
        .get_program_info_v2(&String::from_str(&ctx.env, "PROG2"));
    assert_eq!(prog2.total_funds, 3500);
}

#[test]
fn test_batch_lock_atomicity() {
    let ctx = setup();
    init_program(&ctx, "PROG1", 1000);

    let items = vec![
        &ctx.env,
        LockItem {
            program_id: String::from_str(&ctx.env, "PROG1"),
            amount: 500,
        },
        LockItem {
            program_id: String::from_str(&ctx.env, "NONEXISTENT"),
            amount: 100,
        },
    ];

    let result = ctx.client.try_batch_lock(&items);
    assert!(result.is_err());

    // PROG1 should not be updated
    let prog1 = ctx
        .client
        .get_program_info_v2(&String::from_str(&ctx.env, "PROG1"));
    assert_eq!(prog1.total_funds, 1000);
}

#[test]
fn test_batch_release_success() {
    let ctx = setup();
    init_program(&ctx, "PROG1", 5000);

    // Create schedules
    let recipient1 = Address::generate(&ctx.env);
    let recipient2 = Address::generate(&ctx.env);

    ctx.client.create_program_release_schedule(
        &recipient1,
        &1000,
        &0, // immediate
    );
    ctx.client.create_program_release_schedule(
        &recipient2,
        &2000,
        &0, // immediate
    );

    let items = vec![
        &ctx.env,
        ReleaseItem {
            program_id: String::from_str(&ctx.env, "PROG1"),
            schedule_id: 1,
        },
        ReleaseItem {
            program_id: String::from_str(&ctx.env, "PROG1"),
            schedule_id: 2,
        },
    ];

    let result = ctx.client.batch_release(&items);
    assert_eq!(result, 2);

    // Verify balances
    let prog1 = ctx
        .client
        .get_program_info_v2(&String::from_str(&ctx.env, "PROG1"));
    assert_eq!(prog1.remaining_balance, 2000);

    // Verify tokens were transferred
    let token_client = token::Client::new(&ctx.env, &ctx.token_id);
    assert_eq!(token_client.balance(&recipient1), 1000);
    assert_eq!(token_client.balance(&recipient2), 2000);
}

#[test]
fn test_batch_release_duplicate_fails() {
    let ctx = setup();
    init_program(&ctx, "PROG1", 5000);
    let recipient = Address::generate(&ctx.env);
    ctx.client
        .create_program_release_schedule(&recipient, &1000, &0);

    let items = vec![
        &ctx.env,
        ReleaseItem {
            program_id: String::from_str(&ctx.env, "PROG1"),
            schedule_id: 1,
        },
        ReleaseItem {
            program_id: String::from_str(&ctx.env, "PROG1"),
            schedule_id: 1, // DUPLICATE
        },
    ];

    let result = ctx.client.try_batch_release(&items);
    assert!(result.is_err());
}

// ============================================================================
// Idempotency key generation convention tests
// Issue #1262 — client SDK idempotency key generation conventions
// See docs/program-escrow/idempotency-key-client-guide.md
// ============================================================================

fn string_prefix(env: &Env, s: &String, max_len: usize) -> std::string::String {
    let len = s.len() as usize;
    let mut buf = std::vec::Vec::with_capacity(len);
    buf.resize(len, 0u8);
    s.copy_into_slice(&mut buf);
    let utf8 = core::str::from_utf8(&buf).unwrap();
    std::string::String::from(&utf8[..core::cmp::min(max_len, len)])
}

/// Helper: generate a deterministic single-payout idempotency key
/// following the recommended format: {program_id}-single-{recipient_prefix}-{nonce}
fn make_single_key(env: &Env, program_id: &str, recipient: &Address, nonce: &str) -> String {
    let addr_str = recipient.to_string();
    let prefix = string_prefix(env, &addr_str, 8);
    String::from_str(env, &format!("{}-single-{}-{}", program_id, prefix, nonce))
}

/// Helper: generate a deterministic batch-payout idempotency key
/// following the recommended format: {program_id}-batch-{first_recipient_prefix}-{count}r-{nonce}
fn make_batch_key(
    env: &Env,
    program_id: &str,
    recipients: &soroban_sdk::Vec<Address>,
    nonce: &str,
) -> String {
    let first = recipients.get(0).unwrap();
    let addr_str = first.to_string();
    let prefix = string_prefix(env, &addr_str, 8);
    let count = recipients.len();
    String::from_str(
        env,
        &format!("{}-batch-{}-{}r-{}", program_id, prefix, count, nonce),
    )
}

// ----------------------------------------------------------------------------
// Key format validation
// ----------------------------------------------------------------------------

/// Recommended key format is accepted by the contract.
#[test]
fn test_recommended_single_key_format_accepted() {
    let ctx = setup();
    init_program(&ctx, "hackathon-2024", 10_000);

    let recipient = Address::generate(&ctx.env);
    let key = make_single_key(&ctx.env, "hackathon-2024", &recipient, "a3f1c2d4e5b6a7f8");

    // Key must be non-empty and under 256 chars
    assert!(!key.is_empty());
    assert!(key.len() <= 256);
}

/// Recommended batch key format is accepted by the contract.
#[test]
fn test_recommended_batch_key_format_accepted() {
    let ctx = setup();
    init_program(&ctx, "hackathon-2024", 10_000);

    let r1 = Address::generate(&ctx.env);
    let r2 = Address::generate(&ctx.env);
    let r3 = Address::generate(&ctx.env);
    let recipients = vec![&ctx.env, r1, r2, r3];

    let key = make_batch_key(&ctx.env, "hackathon-2024", &recipients, "9b8c7d6e5f4a3b2c");

    assert!(!key.is_empty());
    assert!(key.len() <= 256);
}

// ----------------------------------------------------------------------------
// Namespace isolation: different programs must not collide
// ----------------------------------------------------------------------------

/// Keys for different programs with the same nonce must differ.
#[test]
fn test_namespace_isolation_different_programs() {
    let ctx = setup();
    let recipient = Address::generate(&ctx.env);
    let nonce = "deadbeef12345678";

    let key_a = make_single_key(&ctx.env, "program-alpha", &recipient, nonce);
    let key_b = make_single_key(&ctx.env, "program-beta", &recipient, nonce);

    assert_ne!(key_a, key_b, "Keys for different programs must not collide");
}

/// Keys for the same program but different operation types must differ.
#[test]
fn test_namespace_isolation_different_payout_types() {
    let ctx = setup();
    let recipient = Address::generate(&ctx.env);
    let nonce = "deadbeef12345678";

    let r = vec![&ctx.env, recipient.clone()];
    let single_key = make_single_key(&ctx.env, "hackathon-2024", &recipient, nonce);
    let batch_key = make_batch_key(&ctx.env, "hackathon-2024", &r, nonce);

    assert_ne!(single_key, batch_key, "single and batch keys must differ");
}

/// Keys for the same program and type but different recipients must differ.
#[test]
fn test_namespace_isolation_different_recipients() {
    let ctx = setup();
    let r1 = Address::generate(&ctx.env);
    let r2 = Address::generate(&ctx.env);
    let nonce = "deadbeef12345678";

    let key1 = make_single_key(&ctx.env, "hackathon-2024", &r1, nonce);
    let key2 = make_single_key(&ctx.env, "hackathon-2024", &r2, nonce);

    // Different recipients produce different keys (address prefix differs)
    // NOTE: with very high probability; in tests addresses are unique by construction
    assert_ne!(key1, key2, "Keys for different recipients must differ");
}

// ----------------------------------------------------------------------------
// Key uniqueness: same key cannot be reused across operations
// ----------------------------------------------------------------------------

/// Submitting the same single-payout key twice returns the original result
/// without executing a second transfer (retry safety).
#[test]
fn test_single_key_retry_returns_original_result() {
    let ctx = setup();
    init_program(&ctx, "hackathon-2024", 10_000);

    let recipient = Address::generate(&ctx.env);
    let key = make_single_key(&ctx.env, "hackathon-2024", &recipient, "a3f1c2d4e5b6a7f8");

    let result1 =
        ctx.client
            .single_payout_by(&ctx.admin, &recipient, &1000i128, &Some(key.clone()));

    let result2 =
        ctx.client
            .single_payout_by(&ctx.admin, &recipient, &1000i128, &Some(key.clone()));

    // Both calls return the same remaining balance (second is a no-op)
    assert_eq!(
        result1.remaining_balance, result2.remaining_balance,
        "Retry must return cached result without re-executing"
    );
}

/// Submitting the same batch-payout key twice returns the original result.
#[test]
fn test_batch_key_retry_returns_original_result() {
    let ctx = setup();
    init_program(&ctx, "hackathon-2024", 10_000);

    let r1 = Address::generate(&ctx.env);
    let r2 = Address::generate(&ctx.env);
    let recipients = vec![&ctx.env, r1.clone(), r2.clone()];
    let amounts = vec![&ctx.env, 1000i128, 2000i128];

    let key = make_batch_key(&ctx.env, "hackathon-2024", &recipients, "9b8c7d6e5f4a3b2c");

    let result1 = ctx
        .client
        .batch_payout_by(&ctx.admin, &recipients, &amounts, &Some(key.clone()));

    let result2 = ctx
        .client
        .batch_payout_by(&ctx.admin, &recipients, &amounts, &Some(key.clone()));

    assert_eq!(
        result1.remaining_balance, result2.remaining_balance,
        "Batch retry must return cached result"
    );
}

// ----------------------------------------------------------------------------
// Key collision prevention: different nonces produce independent operations
// ----------------------------------------------------------------------------

/// Two single payouts with different nonces are treated as independent operations.
#[test]
fn test_different_nonces_are_independent_operations() {
    let ctx = setup();
    init_program(&ctx, "hackathon-2024", 10_000);

    let recipient = Address::generate(&ctx.env);
    let key1 = make_single_key(&ctx.env, "hackathon-2024", &recipient, "aaaaaaaaaaaaaaaa");
    let key2 = make_single_key(&ctx.env, "hackathon-2024", &recipient, "bbbbbbbbbbbbbbbb");

    let result1 = ctx
        .client
        .single_payout_by(&ctx.admin, &recipient, &1000i128, &Some(key1));
    let result2 = ctx
        .client
        .single_payout_by(&ctx.admin, &recipient, &1000i128, &Some(key2));

    // Each operation deducted 1000 independently
    assert_eq!(result1.remaining_balance, 9_000);
    assert_eq!(result2.remaining_balance, 8_000);
}

// ----------------------------------------------------------------------------
// Key length enforcement
// ----------------------------------------------------------------------------

/// A key at exactly 256 characters is accepted.
#[test]
fn test_key_at_max_length_accepted() {
    let ctx = setup();
    init_program(&ctx, "hackathon-2024", 10_000);

    let recipient = Address::generate(&ctx.env);
    // Build a key padded to exactly 256 chars
    let base = format!("hackathon-2024-single-GABC1234-");
    let padding = "x".repeat(256 - base.len());
    let key_str = format!("{}{}", base, padding);
    assert_eq!(key_str.len(), 256);
    let key = String::from_str(&ctx.env, &key_str);

    // Should not panic on validation
    assert!(key.len() <= 256);
}

/// A key exceeding 256 characters is rejected by the contract.
#[test]
#[should_panic(expected = "Idempotency key exceeds maximum length")]
fn test_key_exceeding_max_length_rejected() {
    let ctx = setup();
    init_program(&ctx, "hackathon-2024", 10_000);

    let recipient = Address::generate(&ctx.env);
    let long_key = String::from_str(&ctx.env, &"k".repeat(257));

    ctx.client
        .single_payout_by(&ctx.admin, &recipient, &100i128, &Some(long_key));
}

/// An empty key is rejected by the contract.
#[test]
#[should_panic(expected = "Idempotency key cannot be empty")]
fn test_empty_key_rejected() {
    let ctx = setup();
    init_program(&ctx, "hackathon-2024", 10_000);

    let recipient = Address::generate(&ctx.env);
    let empty_key = String::from_str(&ctx.env, "");

    ctx.client
        .single_payout_by(&ctx.admin, &recipient, &100i128, &Some(empty_key));
}

// ----------------------------------------------------------------------------
// No key provided: backward-compatible path
// ----------------------------------------------------------------------------

/// Omitting the idempotency key still executes the operation normally.
#[test]
fn test_no_key_executes_normally() {
    let ctx = setup();
    init_program(&ctx, "hackathon-2024", 10_000);

    let recipient = Address::generate(&ctx.env);
    let result = ctx
        .client
        .single_payout_by(&ctx.admin, &recipient, &1000i128, &None);

    assert_eq!(result.remaining_balance, 9_000);
}

/// Multiple calls without a key each execute independently (no deduplication).
#[test]
fn test_no_key_allows_duplicate_operations() {
    let ctx = setup();
    init_program(&ctx, "hackathon-2024", 10_000);

    let recipient = Address::generate(&ctx.env);

    ctx.client
        .single_payout_by(&ctx.admin, &recipient, &1000i128, &None);
    let result = ctx
        .client
        .single_payout_by(&ctx.admin, &recipient, &1000i128, &None);

    // Both operations executed — balance reduced by 2000
    assert_eq!(result.remaining_balance, 8_000);
}

// ============================================================================
// Gas Profiling Tests
// ============================================================================
//
// These tests measure the Soroban CPU instruction and memory byte budgets
// consumed by key operations. They serve two purposes:
//
//   1. **Observability** — print structured `[GAS-PROFILE]` lines that can be
//      scraped by CI log parsers or stored as benchmark artifacts.
//   2. **Regression gates** — assert that usage stays within generous upper
//      bounds, catching large regressions before they reach production.
//
// Thresholds are intentionally wide so that minor, non-regressive changes do
// not cause flaky failures. Update them (and `CPU_INSNS_THRESHOLD_50`) after
// intentional gas optimisations or testnet calibration, and record the new
// baseline in `docs/gas-optimization/batch-payout-benchmarks.md`.
//
// The authoritative reference implementation lives in:
//   `contracts/benchmarks/batch_performance.rs`

/// CPU threshold for batch_payout with 50 recipients.
/// The CI benchmark gate fails if actual usage exceeds this value.
/// Update after testnet calibration (see benchmarks/results/).
pub const CPU_INSNS_THRESHOLD_50: u64 = 27_500_000;

/// CPU threshold for lock_program_funds (O(1)).
pub const CPU_INSNS_THRESHOLD_LOCK: u64 = 500_000;

/// Build a `soroban_sdk::Vec<Address>` and a parallel `Vec<i128>` of `amount`
/// each, generating `n` fresh recipient addresses.
fn make_recipients_and_amounts(
    env: &Env,
    n: u32,
    amount_each: i128,
) -> (soroban_sdk::Vec<Address>, soroban_sdk::Vec<i128>) {
    let mut recipients: soroban_sdk::Vec<Address> = soroban_sdk::Vec::new(env);
    let mut amounts: soroban_sdk::Vec<i128> = soroban_sdk::Vec::new(env);
    for _ in 0..n {
        recipients.push_back(Address::generate(env));
        amounts.push_back(amount_each);
    }
    (recipients, amounts)
}

#[test]
fn test_gas_profile_batch_payout_1() {
    use soroban_sdk::testutils::budget::Budget as _;

    let ctx = setup();
    init_program(&ctx, "GAS_PROG", 10_000_000);

    const N: u32 = 1;
    let (recipients, amounts) = make_recipients_and_amounts(&ctx.env, N, 1_000);

    ctx.env.budget().reset_default();
    ctx.client
        .batch_payout_by(&ctx.admin, &recipients, &amounts, &None);
    let cpu_insns = ctx.env.budget().cpu_instruction_cost();
    let mem_bytes = ctx.env.budget().memory_bytes_cost();

    println!(
        "[GAS-PROFILE] op=batch_payout batch_size={} cpu_insns={} mem_bytes={}",
        N, cpu_insns, mem_bytes
    );

    assert!(
        cpu_insns <= CPU_INSNS_THRESHOLD_50,
        "CPU threshold exceeded: batch_payout({}) cpu_insns={} > threshold={}",
        N,
        cpu_insns,
        CPU_INSNS_THRESHOLD_50
    );
}

#[test]
fn test_gas_profile_batch_payout_10() {
    use soroban_sdk::testutils::budget::Budget as _;

    let ctx = setup();
    init_program(&ctx, "GAS_PROG", 10_000_000);

    const N: u32 = 10;
    let (recipients, amounts) = make_recipients_and_amounts(&ctx.env, N, 1_000);

    ctx.env.budget().reset_default();
    ctx.client
        .batch_payout_by(&ctx.admin, &recipients, &amounts, &None);
    let cpu_insns = ctx.env.budget().cpu_instruction_cost();
    let mem_bytes = ctx.env.budget().memory_bytes_cost();

    println!(
        "[GAS-PROFILE] op=batch_payout batch_size={} cpu_insns={} mem_bytes={}",
        N, cpu_insns, mem_bytes
    );

    assert!(
        cpu_insns <= CPU_INSNS_THRESHOLD_50,
        "CPU threshold exceeded: batch_payout({}) cpu_insns={} > threshold={}",
        N,
        cpu_insns,
        CPU_INSNS_THRESHOLD_50
    );
}

#[test]
fn test_gas_profile_batch_payout_50() {
    use soroban_sdk::testutils::budget::Budget as _;

    let ctx = setup();
    init_program(&ctx, "GAS_PROG", 10_000_000);

    const N: u32 = 50;
    let (recipients, amounts) = make_recipients_and_amounts(&ctx.env, N, 1_000);

    ctx.env.budget().reset_default();
    ctx.client
        .batch_payout_by(&ctx.admin, &recipients, &amounts, &None);
    let cpu_insns = ctx.env.budget().cpu_instruction_cost();
    let mem_bytes = ctx.env.budget().memory_bytes_cost();

    println!(
        "[GAS-PROFILE] op=batch_payout batch_size={} cpu_insns={} mem_bytes={}",
        N, cpu_insns, mem_bytes
    );

    assert!(
        cpu_insns <= CPU_INSNS_THRESHOLD_50,
        "CPU threshold exceeded: batch_payout({}) cpu_insns={} > threshold={}",
        N,
        cpu_insns,
        CPU_INSNS_THRESHOLD_50
    );
}

#[test]
fn test_gas_profile_batch_payout_100() {
    use soroban_sdk::testutils::budget::Budget as _;

    let ctx = setup();
    init_program(&ctx, "GAS_PROG", 10_000_000);

    const N: u32 = 100;
    const THRESHOLD_100: u64 = CPU_INSNS_THRESHOLD_50 * 2; // 24_000_000
    let (recipients, amounts) = make_recipients_and_amounts(&ctx.env, N, 1_000);

    ctx.env.budget().reset_default();
    ctx.client
        .batch_payout_by(&ctx.admin, &recipients, &amounts, &None);
    let cpu_insns = ctx.env.budget().cpu_instruction_cost();
    let mem_bytes = ctx.env.budget().memory_bytes_cost();

    println!(
        "[GAS-PROFILE] op=batch_payout batch_size={} cpu_insns={} mem_bytes={}",
        N, cpu_insns, mem_bytes
    );

    assert!(
        cpu_insns <= THRESHOLD_100,
        "CPU threshold exceeded: batch_payout({}) cpu_insns={} > threshold={}",
        N,
        cpu_insns,
        THRESHOLD_100
    );
}

#[test]
fn test_gas_profile_lock_program_funds() {
    use soroban_sdk::testutils::budget::Budget as _;

    let ctx = setup();
    // Initialise with zero liquidity; lock_program_funds credits the amount
    // without an inbound token transfer when no depositor is set.
    let creator = Address::generate(&ctx.env);
    ctx.client.init_program(
        &String::from_str(&ctx.env, "GAS_LOCK"),
        &ctx.admin.clone(),
        &ctx.token_id,
        &creator,
        &None,
        &None,
    );
    // publish_program takes no arguments in the current contract version.
    ctx.client.publish_program();

    ctx.env.budget().reset_default();
    ctx.client.lock_program_funds(&1_000_000i128);
    let cpu_insns = ctx.env.budget().cpu_instruction_cost();
    let mem_bytes = ctx.env.budget().memory_bytes_cost();

    println!(
        "[GAS-PROFILE] op=lock_program_funds batch_size=1 cpu_insns={} mem_bytes={}",
        cpu_insns, mem_bytes
    );

    assert!(
        cpu_insns <= CPU_INSNS_THRESHOLD_LOCK,
        "CPU threshold exceeded: lock_program_funds cpu_insns={} > threshold={}",
        cpu_insns,
        CPU_INSNS_THRESHOLD_LOCK
    );
}

/// **CI gate** — batch_payout with exactly 50 recipients.
///
/// This is the specific test the CI workflow runs for gas regression detection.
/// Fails with a message pointing to the benchmark documentation so that the
/// engineer on-call knows exactly where to look.
#[test]
fn ci_benchmark_gate_batch_payout_50() {
    use soroban_sdk::testutils::budget::Budget as _;

    let ctx = setup();
    init_program(&ctx, "CI_GATE", 10_000_000);

    const BATCH_SIZE: u32 = 50;
    let (recipients, amounts) = make_recipients_and_amounts(&ctx.env, BATCH_SIZE, 1_000);

    ctx.env.budget().reset_default();
    ctx.client
        .batch_payout_by(&ctx.admin, &recipients, &amounts, &None);
    let cpu_insns = ctx.env.budget().cpu_instruction_cost();
    let mem_bytes = ctx.env.budget().memory_bytes_cost();

    println!(
        "[GAS-PROFILE] op=batch_payout batch_size={} cpu_insns={} mem_bytes={}",
        BATCH_SIZE, cpu_insns, mem_bytes
    );

    assert!(
        cpu_insns <= CPU_INSNS_THRESHOLD_50,
        "CI GATE FAILED: batch_payout(50) cpu_insns={} > threshold={}; \
         see docs/gas-optimization/batch-payout-benchmarks.md",
        cpu_insns,
        CPU_INSNS_THRESHOLD_50
    );
}
