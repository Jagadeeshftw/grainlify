//! # Chaos harness for `batch_payout` failure injection
//!
//! Deterministic, seedable harness that randomizes batch composition and
//! injects simulated cross-contract call failures into
//! [`crate::ProgramEscrowContract::batch_payout`] /
//! [`crate::ProgramEscrowContract::batch_payout_idempotent`].
//!
//! ## Invoking this suite alone
//!
//! ```bash
//! cargo test -p program-escrow chaos_batch_payout -- --nocapture
//! ```
//!
//! ## Invariants asserted after every run
//!
//! 1. **No double-payment** — recipient token balances never exceed the
//!    amounts credited by successful payouts; failed runs leave balances
//!    unchanged.
//! 2. **`remaining_balance` never negative** — always `>= 0`.
//! 3. **Circuit breaker consistency** — on injected panics the breaker
//!    state matches the pre-call snapshot (Soroban rolls back the
//!    invocation); on success, state is Closed or HalfOpen→Closed.
//! 4. **Idempotency bookkeeping** — a key is consumed only after a
//!    successful payout; failed runs leave the key reusable.
//!
//! See `docs/program-escrow-chaos-testing.md` for the full design.

#![cfg(test)]

extern crate std;

use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, EnvTestConfig, Ledger},
    token, vec, Address, Env, String, Vec,
};

use crate::{
    chaos, error_recovery,
    error_recovery::CircuitState, DELEGATE_PERMISSION_RELEASE, ProgramData,
    ProgramEscrowContract, ProgramEscrowContractClient, MAX_BATCH_SIZE,
};

// =============================================================================
// Deterministic PRNG (LCG) — same seed ⇒ same scenario
// =============================================================================

/// Lightweight LCG so failing runs are reproducible from the printed seed.
#[derive(Clone, Copy, Debug)]
struct ChaosRng {
    state: u64,
}

impl ChaosRng {
    fn new(seed: u64) -> Self {
        // Avoid the degenerate zero state.
        Self {
            state: seed | 1,
        }
    }

    fn next_u64(&mut self) -> u64 {
        // Numerical Recipes LCG
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        self.state
    }

    fn next_u32(&mut self, max_exclusive: u32) -> u32 {
        assert!(max_exclusive > 0);
        (self.next_u64() % (max_exclusive as u64)) as u32
    }

    fn next_bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    fn next_amount(&mut self) -> i128 {
        // Keep amounts small enough to fund many chaos iterations from one pool.
        1 + (self.next_u64() % 500) as i128
    }
}

// =============================================================================
// Failure modes
// =============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InjectedFailure {
    /// Happy path — no injection.
    None,
    /// Panic inside the transfer loop at recipient index.
    TransferFailAt(u32),
    /// Flip release-pause mid-batch at recipient index.
    PauseMidBatch(u32),
    /// Call `batch_payout_by` with an address that has no release permission.
    UnauthorizedDelegate,
    /// Open the circuit breaker before the call.
    CircuitOpen,
    /// Pause release before the call (entry-guard path).
    PausedEntry,
}

impl InjectedFailure {
    fn from_rng(rng: &mut ChaosRng, batch_size: u32) -> Self {
        match rng.next_u32(6) {
            0 => InjectedFailure::None,
            1 => InjectedFailure::TransferFailAt(rng.next_u32(batch_size)),
            2 => InjectedFailure::PauseMidBatch(rng.next_u32(batch_size)),
            3 => InjectedFailure::UnauthorizedDelegate,
            4 => InjectedFailure::CircuitOpen,
            _ => InjectedFailure::PausedEntry,
        }
    }

    fn expects_success(self) -> bool {
        matches!(self, InjectedFailure::None)
    }
}

// =============================================================================
// Scenario + harness context
// =============================================================================

#[derive(Clone, Debug)]
struct ChaosScenario {
    seed: u64,
    batch_size: u32,
    amounts: std::vec::Vec<i128>,
    failure: InjectedFailure,
    use_idempotent: bool,
}

impl ChaosScenario {
    fn generate(seed: u64) -> Self {
        let mut rng = ChaosRng::new(seed);
        // Keep batches modest for host-resource limits; still exercise 1..=16.
        let batch_size = 1 + rng.next_u32(16.min(MAX_BATCH_SIZE));
        let mut amounts = std::vec::Vec::with_capacity(batch_size as usize);
        for _ in 0..batch_size {
            amounts.push(rng.next_amount());
        }
        let failure = InjectedFailure::from_rng(&mut rng, batch_size);
        let use_idempotent = rng.next_bool();
        Self {
            seed,
            batch_size,
            amounts,
            failure,
            use_idempotent,
        }
    }

    fn total(&self) -> i128 {
        self.amounts.iter().sum()
    }
}

struct Harness<'a> {
    env: Env,
    client: ProgramEscrowContractClient<'a>,
    contract_id: Address,
    token: token::Client<'a>,
    admin: Address,
    payout_key: Address,
    program_id: String,
    initial_balance: i128,
}

fn setup_harness(initial_balance: i128) -> Harness<'static> {
    // Disable ledger snapshot capture — the seeded sweep creates dozens of
    // Env instances and would otherwise flood test_snapshots/ with noise.
    let env = Env::new_with_config(EnvTestConfig {
        capture_snapshot_at_drop: false,
    });
    env.mock_all_auths();
    env.ledger().set_timestamp(1_700_000_000);

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize_contract(&admin);

    let payout_key = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let token = token::Client::new(&env, &token_address);

    let program_id = String::from_str(&env, "chaos-batch");
    client.init_program(
        &program_id,
        &payout_key,
        &token_address,
        &admin,
        &None,
        &None,
    );
    client.publish_program(&program_id, &admin);

    if initial_balance > 0 {
        token::StellarAssetClient::new(&env, &token_address).mint(&client.address, &initial_balance);
        client.lock_program_funds(&initial_balance);
    }

    // Quiet circuit breaker defaults for chaos runs.
    env.as_contract(&contract_id, || {
        error_recovery::set_config(
            &env,
            error_recovery::CircuitBreakerConfig {
                failure_threshold: 5,
                success_threshold: 2,
                max_error_log: 32,
                recovery_window: 60,
            },
        );
    });

    Harness {
        env,
        client,
        contract_id,
        token,
        admin,
        payout_key,
        program_id,
        initial_balance,
    }
}

fn snapshot_balances(h: &Harness, recipients: &[Address]) -> (i128, std::vec::Vec<i128>, CircuitState) {
    let data = h.client.get_program_info();
    let mut bals = std::vec::Vec::with_capacity(recipients.len());
    for r in recipients {
        bals.push(h.token.balance(r));
    }
    let cb = h.env.as_contract(&h.contract_id, || error_recovery::get_state(&h.env));
    (data.remaining_balance, bals, cb)
}

fn assert_invariants(
    h: &Harness,
    scenario: &ChaosScenario,
    recipients: &[Address],
    before_balance: i128,
    before_recipient: &[i128],
    before_cb: CircuitState,
    succeeded: bool,
    idem_key: Option<&String>,
) {
    let data: ProgramData = h.client.get_program_info();
    assert!(
        data.remaining_balance >= 0,
        "seed={} remaining_balance went negative: {}",
        scenario.seed,
        data.remaining_balance
    );

    let after_cb = h
        .env
        .as_contract(&h.contract_id, || error_recovery::get_state(&h.env));

    if succeeded {
        let expected = before_balance
            .checked_sub(scenario.total())
            .expect("balance underflow on success path");
        assert_eq!(
            data.remaining_balance, expected,
            "seed={} remaining_balance mismatch after success",
            scenario.seed
        );
        for (i, r) in recipients.iter().enumerate() {
            let bal = h.token.balance(r);
            assert_eq!(
                bal,
                before_recipient[i] + scenario.amounts[i],
                "seed={} recipient[{i}] over/under-paid",
                scenario.seed
            );
        }
        // Success should not leave the breaker Open.
        assert_ne!(
            after_cb,
            CircuitState::Open,
            "seed={} circuit unexpectedly Open after success",
            scenario.seed
        );
        if let Some(key) = idem_key {
            // Replay must be a no-op (no second debit).
            let mid = data.remaining_balance;
            let _ = h.client.batch_payout_idempotent(
                key,
                &recipients_vec(&h.env, recipients),
                &amounts_vec(&h.env, &scenario.amounts),
            );
            let again = h.client.get_program_info();
            assert_eq!(
                again.remaining_balance, mid,
                "seed={} idempotent replay double-paid",
                scenario.seed
            );
        }
    } else {
        // Full rollback: balances and CB unchanged.
        assert_eq!(
            data.remaining_balance, before_balance,
            "seed={} remaining_balance mutated on failure (expected rollback)",
            scenario.seed
        );
        for (i, r) in recipients.iter().enumerate() {
            assert_eq!(
                h.token.balance(r),
                before_recipient[i],
                "seed={} recipient[{i}] balance changed on failure",
                scenario.seed
            );
        }
        assert_eq!(
            after_cb, before_cb,
            "seed={} circuit breaker drifted on rolled-back failure",
            scenario.seed
        );
        if let Some(key) = idem_key {
            // Key must remain reusable after failure — a clean retry with no
            // injection should succeed (when funds allow).
            h.env.as_contract(&h.contract_id, || chaos::reset(&h.env));
            // Clear pause / circuit side-effects that are outside the failed
            // invocation (entry-guard failures never entered the transfer loop).
            match scenario.failure {
                InjectedFailure::PausedEntry => {
                    h.client.set_paused(
                        &Some(false),
                        &Some(false),
                        &Some(false),
                        &None::<String>,
                        &None::<u64>,
                    );
                }
                InjectedFailure::CircuitOpen => {
                    h.env.as_contract(&h.contract_id, || {
                        error_recovery::close_circuit(&h.env);
                    });
                }
                _ => {}
            }
            if before_balance >= scenario.total()
                && !matches!(
                    scenario.failure,
                    InjectedFailure::UnauthorizedDelegate
                )
            {
                let retry = h.client.try_batch_payout_idempotent(
                    key,
                    &recipients_vec(&h.env, recipients),
                    &amounts_vec(&h.env, &scenario.amounts),
                );
                assert!(
                    retry.is_ok(),
                    "seed={} idempotency key should be reusable after failure: {:?}",
                    scenario.seed,
                    retry
                );
                let after = h.client.get_program_info();
                assert_eq!(
                    after.remaining_balance,
                    before_balance - scenario.total(),
                    "seed={} retry after failure did not debit correctly",
                    scenario.seed
                );
            }
        }
    }
}

fn recipients_vec(env: &Env, recipients: &[Address]) -> Vec<Address> {
    let mut out = Vec::new(env);
    for r in recipients {
        out.push_back(r.clone());
    }
    out
}

fn amounts_vec(env: &Env, amounts: &[i128]) -> Vec<i128> {
    let mut out = Vec::new(env);
    for a in amounts {
        out.push_back(*a);
    }
    out
}

fn apply_failure_preconditions(h: &Harness, scenario: &ChaosScenario) {
    h.env.as_contract(&h.contract_id, || chaos::reset(&h.env));

    match scenario.failure {
        InjectedFailure::None => {}
        InjectedFailure::TransferFailAt(at) => {
            h.env.as_contract(&h.contract_id, || {
                chaos::configure_transfer_fail(&h.env, at);
            });
        }
        InjectedFailure::PauseMidBatch(at) => {
            h.env.as_contract(&h.contract_id, || {
                chaos::configure_pause_mid_batch(&h.env, at);
            });
        }
        InjectedFailure::UnauthorizedDelegate => {
            // Leave no delegate configured — batch_payout_by with a random
            // caller must be rejected.
        }
        InjectedFailure::CircuitOpen => {
            h.env.as_contract(&h.contract_id, || {
                error_recovery::set_config(
                    &h.env,
                    error_recovery::CircuitBreakerConfig {
                        failure_threshold: 1,
                        success_threshold: 1,
                        max_error_log: 16,
                        // Non-zero so Open does not instantly age into HalfOpen.
                        recovery_window: 3_600,
                    },
                );
                error_recovery::record_failure(
                    &h.env,
                    h.program_id.clone(),
                    symbol_short!("chaos"),
                    1001,
                    None,
                );
                assert_eq!(error_recovery::get_state(&h.env), CircuitState::Open);
            });
        }
        InjectedFailure::PausedEntry => {
            h.client.set_paused(
                &Some(false),
                &Some(true),
                &Some(false),
                &Some(String::from_str(&h.env, "chaos-entry-pause")),
                &None::<u64>,
            );
        }
    }
}

fn execute_scenario(scenario: &ChaosScenario) {
    // Fund generously so InsufficientBalance is not an accidental confounder.
    let fund = scenario.total().saturating_mul(4).max(10_000);
    let h = setup_harness(fund);

    let mut recipients = std::vec::Vec::with_capacity(scenario.batch_size as usize);
    for _ in 0..scenario.batch_size {
        recipients.push(Address::generate(&h.env));
    }

    apply_failure_preconditions(&h, scenario);

    let (before_bal, before_recv, before_cb) = snapshot_balances(&h, &recipients);
    let recip_v = recipients_vec(&h.env, &recipients);
    let amt_v = amounts_vec(&h.env, &scenario.amounts);

    let idem_key = if scenario.use_idempotent {
        Some(String::from_str(
            &h.env,
            &std::format!("chaos-idem-{}", scenario.seed),
        ))
    } else {
        None
    };

    let result = match (scenario.failure, scenario.use_idempotent) {
        (InjectedFailure::UnauthorizedDelegate, false) => {
            let stranger = Address::generate(&h.env);
            h.client.try_batch_payout_by(&stranger, &recip_v, &amt_v)
        }
        (InjectedFailure::UnauthorizedDelegate, true) => {
            let stranger = Address::generate(&h.env);
            let key = idem_key.as_ref().unwrap();
            h.client
                .try_batch_payout_idempotent_by(key, &stranger, &recip_v, &amt_v)
        }
        (_, true) => {
            let key = idem_key.as_ref().unwrap();
            h.client.try_batch_payout_idempotent(key, &recip_v, &amt_v)
        }
        (_, false) => h.client.try_batch_payout(&recip_v, &amt_v),
    };

    let succeeded = result.is_ok();
    assert_eq!(
        succeeded,
        scenario.failure.expects_success(),
        "seed={} failure={:?} expected success={} got ok={} err={:?}",
        scenario.seed,
        scenario.failure,
        scenario.failure.expects_success(),
        succeeded,
        result
    );

    assert_invariants(
        &h,
        scenario,
        &recipients,
        before_bal,
        &before_recv,
        before_cb,
        succeeded,
        idem_key.as_ref(),
    );
}

// =============================================================================
// Public test targets (separately invocable via filter `chaos_batch_payout`)
// =============================================================================

/// Smoke: a fixed seed covering transfer-fail injection + rollback invariants.
#[test]
fn chaos_batch_payout_transfer_fail_rolls_back() {
    let mut scenario = ChaosScenario::generate(0xC0FFEE_u64);
    scenario.failure = InjectedFailure::TransferFailAt(0);
    scenario.use_idempotent = true;
    scenario.batch_size = 3;
    scenario.amounts = std::vec![10, 20, 30];
    execute_scenario(&scenario);
}

/// Smoke: mid-batch pause injection rolls back with stable pause message path.
#[test]
fn chaos_batch_payout_pause_mid_batch_rolls_back() {
    let mut scenario = ChaosScenario::generate(0xBEEF_u64);
    scenario.failure = InjectedFailure::PauseMidBatch(1);
    scenario.use_idempotent = false;
    scenario.batch_size = 3;
    scenario.amounts = std::vec![11, 22, 33];
    execute_scenario(&scenario);
}

/// Smoke: unauthorized delegate is rejected without mutating balances.
#[test]
fn chaos_batch_payout_unauthorized_delegate_rejected() {
    let mut scenario = ChaosScenario::generate(0xDEAD_u64);
    scenario.failure = InjectedFailure::UnauthorizedDelegate;
    scenario.use_idempotent = false;
    scenario.batch_size = 2;
    scenario.amounts = std::vec![50, 75];
    execute_scenario(&scenario);
}

/// Smoke: open circuit breaker rejects the batch; state stays Open.
#[test]
fn chaos_batch_payout_circuit_open_rejected() {
    let mut scenario = ChaosScenario::generate(0xCAFE_u64);
    scenario.failure = InjectedFailure::CircuitOpen;
    scenario.use_idempotent = true;
    scenario.batch_size = 2;
    scenario.amounts = std::vec![5, 5];
    execute_scenario(&scenario);
}

/// Smoke: entry-guard pause rejects before any transfer.
#[test]
fn chaos_batch_payout_entry_pause_rejected() {
    let mut scenario = ChaosScenario::generate(0xA11CE_u64);
    scenario.failure = InjectedFailure::PausedEntry;
    scenario.use_idempotent = false;
    scenario.batch_size = 2;
    scenario.amounts = std::vec![7, 8];
    execute_scenario(&scenario);
}

/// Happy-path control: randomized successful batch preserves invariants.
#[test]
fn chaos_batch_payout_happy_path_invariants() {
    let mut scenario = ChaosScenario::generate(0x1234_u64);
    scenario.failure = InjectedFailure::None;
    scenario.use_idempotent = true;
    scenario.batch_size = 4;
    scenario.amounts = std::vec![1, 2, 3, 4];
    execute_scenario(&scenario);
}

/// Multi-seed sweep — primary chaos target.
///
/// Each seed is fully deterministic.  On failure the assertion messages
/// include the seed so the exact scenario can be re-run in isolation.
#[test]
fn chaos_batch_payout_seeded_sweep() {
    // Keep the sweep modest for CI wall-clock; expand locally as needed.
    const SEEDS: u64 = 64;
    const BASE: u64 = 0xCA05_u64; // memorable "chaos" base

    for i in 0..SEEDS {
        let seed = BASE.wrapping_add(i.wrapping_mul(0x9E37_79B9));
        let scenario = ChaosScenario::generate(seed);
        // Skip unauthorized+idempotent retry edge that needs a funded delegate
        // path when remaining balance would be exhausted by a prior success in
        // the same scenario — each scenario gets a fresh harness anyway.
        execute_scenario(&scenario);
    }
}

/// Explicit reproducibility check: identical seeds produce identical scenarios.
#[test]
fn chaos_batch_payout_seed_is_deterministic() {
    let a = ChaosScenario::generate(42);
    let b = ChaosScenario::generate(42);
    assert_eq!(a.batch_size, b.batch_size);
    assert_eq!(a.amounts, b.amounts);
    assert_eq!(a.failure, b.failure);
    assert_eq!(a.use_idempotent, b.use_idempotent);

    // Salted seeds must eventually diverge (LCG period >> 64).
    let mut found_divergent = false;
    for salt in 1u64..64 {
        let other = ChaosScenario::generate(42 ^ (salt.wrapping_mul(0x9E37_79B9)));
        if other.batch_size != a.batch_size
            || other.amounts != a.amounts
            || other.failure != a.failure
            || other.use_idempotent != a.use_idempotent
        {
            found_divergent = true;
            break;
        }
    }
    assert!(found_divergent, "LCG should produce divergent scenarios");
}

/// Authorized delegate happy path (proves the unauthorized case is meaningful).
#[test]
fn chaos_batch_payout_authorized_delegate_succeeds() {
    let h = setup_harness(1_000);
    let delegate = Address::generate(&h.env);
    h.client.set_program_delegate(
        &h.program_id,
        &h.admin,
        &delegate,
        &DELEGATE_PERMISSION_RELEASE,
    );

    let r1 = Address::generate(&h.env);
    let r2 = Address::generate(&h.env);
    let recipients = vec![&h.env, r1.clone(), r2.clone()];
    let amounts = vec![&h.env, 10_i128, 15_i128];

    let before = h.client.get_program_info().remaining_balance;
    let data = h.client.batch_payout_by(&delegate, &recipients, &amounts);
    assert_eq!(data.remaining_balance, before - 25);
    assert_eq!(h.token.balance(&r1), 10);
    assert_eq!(h.token.balance(&r2), 15);
    assert!(data.remaining_balance >= 0);
}
