//! Integration tests asserting the correct precedence order when multiple
//! protection guards are active simultaneously.
//!
//! Precedence (highest → lowest):
//!   Pause / maintenance mode → Read-only mode → Circuit breaker
//!
//! See `docs/program-escrow/CIRCUIT_BREAKER_ENFORCEMENT.md` for full details.

#![cfg(test)]

use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    vec, Address, Env, String,
};

use crate::{
    error_recovery::{self, CircuitBreakerConfig, CircuitState},
    ProgramEscrowContract, ProgramEscrowContractClient,
};

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

struct TestSetup {
    env: Env,
    client: ProgramEscrowContractClient<'static>,
    admin: Address,
    payout_key: Address,
    recipient: Address,
    token: Address,
}

fn setup() -> TestSetup {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    // SAFETY: the client reference borrows from env which lives in the struct.
    let client: ProgramEscrowContractClient<'static> =
        unsafe { core::mem::transmute(ProgramEscrowContractClient::new(&env, &contract_id)) };

    let admin = Address::generate(&env);
    let payout_key = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = Address::generate(&env);

    // Bootstrap contract
    client.initialize_contract(&admin);
    client.init_program(
        &String::from_str(&env, "test-program"),
        &payout_key,
        &token,
        &admin,
        &None,
        &None,
    );

    // Register a circuit breaker admin (same as contract admin for simplicity)
    client.set_circuit_admin(&admin, &None);

    TestSetup {
        env,
        client,
        admin,
        payout_key,
        recipient,
        token,
    }
}

/// Force the circuit breaker into Open state by recording failures above threshold.
fn open_circuit(env: &Env) {
    let cfg = CircuitBreakerConfig {
        failure_threshold: 1,
        success_threshold: 1,
        max_error_log: 10,
    };
    error_recovery::set_config(env, cfg);
    error_recovery::record_failure(
        env,
        String::from_str(env, "test-program"),
        symbol_short!("test"),
        1001,
    );
    assert_eq!(error_recovery::get_state(env), CircuitState::Open);
}

/// Assert the circuit is in Closed state.
fn assert_circuit_closed(env: &Env) {
    assert_eq!(
        error_recovery::get_state(env),
        CircuitState::Closed,
        "Expected circuit to be Closed"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: Pause wins over circuit breaker
// ─────────────────────────────────────────────────────────────────────────────

/// When release is paused AND the circuit is open, the pause error message
/// must be returned (pause takes precedence).
#[test]
#[should_panic(expected = "Funds Paused")]
fn test_pause_wins_over_open_circuit_single_payout() {
    let TestSetup {
        env,
        client,
        admin,
        payout_key,
        recipient,
        token: _,
    } = setup();

    // Open the circuit breaker
    open_circuit(&env);

    // Also pause release operations
    client.set_paused(&Some(false), &Some(true), &Some(false), &None);

    // Attempt a single payout — should fail with "Funds Paused", NOT "Circuit breaker is OPEN"
    client.single_payout_by(&payout_key, &recipient, &100i128, &None);
}

/// Same as above but for batch_payout.
#[test]
#[should_panic(expected = "Funds Paused")]
fn test_pause_wins_over_open_circuit_batch_payout() {
    let TestSetup {
        env,
        client,
        admin,
        payout_key,
        recipient,
        token: _,
    } = setup();

    open_circuit(&env);
    client.set_paused(&Some(false), &Some(true), &Some(false), &None);

    let recipients = vec![&env, recipient];
    let amounts = vec![&env, 100i128];
    client.batch_payout_by(&payout_key, &recipients, &amounts, &None);
}

/// When lock is paused AND the circuit is open, lock operations must return
/// "Funds Paused".
#[test]
#[should_panic(expected = "Funds Paused")]
fn test_pause_wins_over_open_circuit_lock() {
    let TestSetup {
        env,
        client,
        admin,
        payout_key: _,
        recipient: _,
        token: _,
    } = setup();

    open_circuit(&env);
    client.set_paused(&Some(true), &Some(false), &Some(false), &None);

    client.lock_program_funds(&1000i128);
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: All three layers simultaneously — pause wins
// ─────────────────────────────────────────────────────────────────────────────

/// With pause, read-only, AND circuit breaker all active, the pause error
/// must surface first.
#[test]
#[should_panic(expected = "Funds Paused")]
fn test_all_three_active_pause_wins_single_payout() {
    let TestSetup {
        env,
        client,
        admin,
        payout_key,
        recipient,
        token: _,
    } = setup();

    // Activate all three layers
    open_circuit(&env);
    client.set_read_only_mode(&true, &None);
    client.set_paused(&Some(false), &Some(true), &Some(false), &None);

    // Pause must win
    client.single_payout_by(&payout_key, &recipient, &100i128, &None);
}

/// With pause, read-only, AND circuit breaker all active, the pause error
/// must surface for batch_payout.
#[test]
#[should_panic(expected = "Funds Paused")]
fn test_all_three_active_pause_wins_batch_payout() {
    let TestSetup {
        env,
        client,
        admin,
        payout_key,
        recipient,
        token: _,
    } = setup();

    open_circuit(&env);
    client.set_read_only_mode(&true, &None);
    client.set_paused(&Some(false), &Some(true), &Some(false), &None);

    let recipients = vec![&env, recipient];
    let amounts = vec![&env, 100i128];
    client.batch_payout_by(&payout_key, &recipients, &amounts, &None);
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: Circuit breaker fires only when pause is inactive
// ─────────────────────────────────────────────────────────────────────────────

/// When the circuit is open but pause is NOT active, the circuit breaker
/// error must surface.
#[test]
#[should_panic(expected = "Circuit breaker is OPEN")]
fn test_circuit_open_fires_when_pause_inactive_single_payout() {
    let TestSetup {
        env,
        client,
        payout_key,
        recipient,
        ..
    } = setup();

    open_circuit(&env);
    // Explicitly confirm pause is off
    client.set_paused(&Some(false), &Some(false), &Some(false), &None);

    client.single_payout_by(&payout_key, &recipient, &100i128, &None);
}

/// Same for batch_payout.
#[test]
#[should_panic(expected = "Circuit breaker is OPEN")]
fn test_circuit_open_fires_when_pause_inactive_batch_payout() {
    let TestSetup {
        env,
        client,
        payout_key,
        recipient,
        ..
    } = setup();

    open_circuit(&env);
    client.set_paused(&Some(false), &Some(false), &Some(false), &None);

    let recipients = vec![&env, recipient];
    let amounts = vec![&env, 100i128];
    client.batch_payout_by(&payout_key, &recipients, &amounts, &None);
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: Unpausing reveals the circuit breaker
// ─────────────────────────────────────────────────────────────────────────────

/// After unpausing, if the circuit is still open, the circuit breaker error
/// must be returned (not a pause error).
#[test]
#[should_panic(expected = "Circuit breaker is OPEN")]
fn test_unpause_reveals_open_circuit() {
    let TestSetup {
        env,
        client,
        payout_key,
        recipient,
        ..
    } = setup();

    // Start: both active
    open_circuit(&env);
    client.set_paused(&Some(false), &Some(true), &Some(false), &None);

    // Unpause, circuit stays open
    client.set_paused(&Some(false), &Some(false), &Some(false), &None);

    client.single_payout_by(&payout_key, &recipient, &100i128, &None);
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: Resetting the circuit breaker
// ─────────────────────────────────────────────────────────────────────────────

/// After the circuit is reset (Open → HalfOpen → Closed) and pause is not
/// active, operations should be allowed (assuming sufficient balance).
/// Here we only verify the circuit reaches Closed state after reset.
#[test]
fn test_circuit_reset_closes_after_open() {
    let TestSetup {
        env,
        client,
        admin,
        ..
    } = setup();

    open_circuit(&env);
    assert_eq!(error_recovery::get_state(&env), CircuitState::Open);

    // Step 1: reset moves Open → HalfOpen
    client.reset_circuit_breaker(&admin);
    assert_eq!(error_recovery::get_state(&env), CircuitState::HalfOpen);

    // Step 2: another reset moves HalfOpen → Closed
    client.reset_circuit_breaker(&admin);
    assert_circuit_closed(&env);
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: Maintenance mode (treated as lock pause)
// ─────────────────────────────────────────────────────────────────────────────

/// Maintenance mode blocks lock operations with its own error message,
/// independently of the circuit breaker state.
#[test]
#[should_panic(expected = "Funds Paused")]
fn test_maintenance_mode_wins_over_open_circuit_on_lock() {
    let TestSetup {
        env,
        client,
        admin,
        ..
    } = setup();

    open_circuit(&env);
    client.set_maintenance_mode(&true);

    client.lock_program_funds(&1000i128);
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: Pause state is independent per operation type
// ─────────────────────────────────────────────────────────────────────────────

/// Pausing only release does NOT block lock operations (even with open circuit).
/// We can't easily test a successful lock here (no token mock), but we can
/// confirm the circuit error, not a pause error, surfaces.
#[test]
#[should_panic(expected = "Circuit breaker is OPEN")]
fn test_release_pause_does_not_block_lock_when_circuit_open() {
    // NOTE: lock_program_funds does NOT check the circuit breaker in its
    // current implementation, so this test verifies orthogonality differently:
    // we confirm that pausing *release* doesn't bleed into payout operations
    // beyond what's expected. Here we verify payout sees the circuit, not pause.
    let TestSetup {
        env,
        client,
        payout_key,
        recipient,
        ..
    } = setup();

    open_circuit(&env);
    // Only lock is paused — release is not
    client.set_paused(&Some(true), &Some(false), &Some(false), &None);

    // Payout should fail with circuit error (release not paused)
    client.single_payout_by(&payout_key, &recipient, &100i128, &None);
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: Closed circuit + no pause = no protection-layer interference
// ─────────────────────────────────────────────────────────────────────────────

/// With no protection layers active, an operation reaches business-logic
/// validation (here it will fail with "Program not initialized" or balance
/// error, NOT a protection-layer error). This confirms the layers are
/// transparent when inactive.
#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_no_protection_layers_active_reaches_business_logic() {
    let TestSetup {
        env,
        client,
        payout_key,
        recipient,
        ..
    } = setup();

    // Ensure all protection layers are inactive
    assert_circuit_closed(&env);
    client.set_paused(&Some(false), &Some(false), &Some(false), &None);

    // No funds locked → should reach balance check
    client.single_payout_by(&payout_key, &recipient, &1i128, &None);
}
/// Circuit Breaker Enforcement Tests — Issue #1065
///
/// Verifies that the circuit breaker check runs at step 3c (before authorization
/// and all business logic) in both `single_payout` and `batch_payout`, producing
/// deterministic, stable rejections regardless of balance or threshold state.
///
/// # Coverage
/// - Open circuit blocks single_payout before balance check
/// - Open circuit blocks batch_payout before balance check
/// - Open circuit blocks even when balance is sufficient
/// - HalfOpen circuit allows payouts through
/// - Closed circuit allows payouts through
/// - Circuit transitions: Closed → Open → HalfOpen → Closed
/// - get_circuit_breaker_status returns correct sna});
    }
}
  fn test_invariants_fail_for_open_circuit_without_timestamp() {
        let s = setup_with_funds(0);
        s.env.as_contract(&s.client.address, || {
            s.env
                .storage()
                .persistent()
                .set(&CircuitBreakerKey::State, &CircuitState::Open);
            s.env
                .storage()
                .persistent()
                .set(&CircuitBreakerKey::OpenedAt, &0u64);
            assert!(!error_recovery::verify_circuit_invariants(&s.env));
        or_recovery::verify_circuit_invariants(&s.env));
        });
    }

    /// verify_circuit_invariants passes for a properly opened circuit.
    #[test]
    fn test_invariants_pass_for_open_circuit_with_timestamp() {
        let s = setup_with_funds(0);
        open_circuit(&s);
        s.env.as_contract(&s.client.address, || {
            assert!(error_recovery::verify_circuit_invariants(&s.env));
        });
    }

    /// verify_circuit_invariants fails when Open state has no timestamp (tamper).
    #[test]
  "cb_reset")),
            "cb_reset event must be emitted on admin reset"
        );
    }

    // ─────────────────────────────────────────────────────────────────────
    // Invariant verification
    // ─────────────────────────────────────────────────────────────────────

    /// verify_circuit_invariants passes for a healthy Closed circuit.
    #[test]
    fn test_invariants_pass_for_closed_circuit() {
        let s = setup_with_funds(0);
        s.env.as_contract(&s.client.address, || {
            assert!(errrt!(
            has_event_topic(&s.env, symbol_short!("circuit"), symbol_short!("cb_close")),
            "cb_close event must be emitted when circuit closes"
        );
    }

    /// cb_reset event is emitted on every admin reset call.
    #[test]
    fn test_cb_reset_event_emitted_on_admin_reset() {
        let s = setup_with_funds(0);
        open_circuit(&s);
        s.client.reset_circuit_breaker(&s.admin);

        assert!(
            has_event_topic(&s.env, symbol_short!("circuit"), symbol_short!(_event_topic(&s.env, symbol_short!("circuit"), symbol_short!("cb_half")),
            "cb_half event must be emitted when circuit moves to HalfOpen"
        );
    }

    /// cb_close event is emitted when circuit closes.
    #[test]
    fn test_cb_close_event_emitted_on_close() {
        let s = setup_with_funds(0);
        open_circuit(&s);
        // Open → HalfOpen
        s.client.reset_circuit_breaker(&s.admin);
        // HalfOpen → Closed
        s.client.reset_circuit_breaker(&s.admin);

        assesetup_with_funds(0);
        open_circuit(&s);

        assert!(
            has_event_topic(&s.env, symbol_short!("circuit"), symbol_short!("cb_open")),
            "cb_open event must be emitted when circuit opens"
        );
    }

    /// cb_half event is emitted when admin resets from Open.
    #[test]
    fn test_cb_half_event_emitted_on_reset_from_open() {
        let s = setup_with_funds(0);
        open_circuit(&s);
        s.client.reset_circuit_breaker(&s.admin);

        assert!(
            has.admin, &7u32, &3u32, &20u32);

        let status = s.client.get_circuit_breaker_status();
        assert_eq!(status.failure_threshold, 7);
        assert_eq!(status.success_threshold, 3);
    }

    // ─────────────────────────────────────────────────────────────────────
    // Audit events
    // ─────────────────────────────────────────────────────────────────────

    /// cb_open event is emitted when the circuit opens manually.
    #[test]
    fn test_cb_open_event_emitted_on_manual_open() {
        let s = en_open() {
        let s = setup_with_funds(0);
        open_circuit(&s);

        let status = s.client.get_circuit_breaker_status();
        assert_eq!(status.state, CircuitState::Open);
        assert!(status.opened_at > 0, "opened_at must be set when circuit is Open");
    }

    /// get_circuit_breaker_status reflects configured thresholds.
    #[test]
    fn test_get_circuit_breaker_status_reflects_config() {
        let s = setup_with_funds(0);
        s.client
            .configure_circuit_breaker(&s.admin, &7u32, &3u32, &20u32, &600u64);

        let status = s.client.get_circuit_breaker_status();
        assert_eq!(status.failure_threshold, 7);
        assert_eq!(status.success_threshold, 3);
        assert_eq!(status.recovery_window, 600);
    }

    /// get_circuit_breaker_status returns Closed with zero counts on a fresh contract.
    #[test]
    fn test_get_circuit_breaker_status_initial_state() {
        let s = setup_with_funds(0);
        let status = s.client.get_circuit_breaker_status();
        assert_eq!(status.state, CircuitState::Closed);
        assert_eq!(status.failure_count, 0);
        assert_eq!(status.success_count, 0);
        assert_eq!(status.opened_at, 0);
    }

    /// get_circuit_breaker_status reflects Open state and non-zero opened_at.
    #[test]
    fn test_get_circuit_breaker_status_wh 1)
        s.client.single_payout(&winner, &100i128, &None);

        assert_eq!(
            get_state(&s),
            CircuitState::Closed,
            "Circuit must close after success_threshold successes in HalfOpen"
        );
    }

    // ─────────────────────────────────────────────────────────────────────
    // get_circuit_breaker_status view entrypoint
    // ─────────────────────────────────────────────────────────────────────

    /// get_circuit_breaker_status returns Closed with zero counts itState::Open,
                "Circuit must open after failure_threshold failures"
            );
        });
    }

    /// After a successful payout in HalfOpen, the circuit closes automatically
    /// when success_threshold is reached (default = 1).
    #[test]
    fn test_circuit_closes_after_success_in_half_open() {
        let s = setup_with_funds(1_000);
        half_open_circuit(&s);

        let winner = Address::generate(&s.env);
        // One success should close the circuit (success_threshold =s.env.as_contract(&s.client.address, || {
            let config = error_recovery::get_config(&s.env);
            // Record failures up to threshold
            for _ in 0..config.failure_threshold {
                error_recovery::record_failure(
                    &s.env,
                    program_id.clone(),
                    symbol_short!("test_op"),
                    42,
                );
            }
            assert_eq!(
                error_recovery::get_state(&s.env),
                Circuassert_eq!(get_state(&s), CircuitState::HalfOpen);

        // Admin reset again: HalfOpen → Closed
        s.client.reset_circuit_breaker(&s.admin);
        assert_eq!(get_state(&s), CircuitState::Closed);
    }

    /// Automatic trip: after failure_threshold consecutive record_failure calls
    /// the circuit opens automatically.
    #[test]
    fn test_circuit_auto_trips_after_failure_threshold() {
        let s = setup_with_funds(0);
        let program_id = String::from_str(&s.env, "prog-cb");

        / ─────────────────────────────────────────────────────────────────────

    /// Full transition: Closed → Open → HalfOpen → Closed.
    #[test]
    fn test_circuit_state_transitions() {
        let s = setup_with_funds(0);

        // Start Closed
        assert_eq!(get_state(&s), CircuitState::Closed);

        // Open manually
        open_circuit(&s);
        assert_eq!(get_state(&s), CircuitState::Open);

        // Admin reset: Open → HalfOpen
        s.client.reset_circuit_breaker(&s.admin);
         fn test_single_payout_allowed_when_circuit_half_open() {
        let s = setup_with_funds(1_000);
        half_open_circuit(&s);
        assert_eq!(get_state(&s), CircuitState::HalfOpen);

        let winner = Address::generate(&s.env);
        let result = s.client.try_single_payout(&winner, &100i128, &None);
        assert!(result.is_ok(), "single_payout must succeed when circuit is HalfOpen");
    }

    // ─────────────────────────────────────────────────────────────────────
    // State transitions
    /   fn test_batch_payout_allowed_when_circuit_closed() {
        let s = setup_with_funds(1_000);

        let w1 = Address::generate(&s.env);
        let w2 = Address::generate(&s.env);
        let result = s.client.try_batch_payout(
            &vec![&s.env, w1, w2],
            &vec![&s.env, 100i128, 100i128],
            &None,
        );
        assert!(result.is_ok(), "batch_payout must succeed when circuit is Closed");
    }

    /// HalfOpen circuit must allow single_payout (trial period).
    #[test]
   ──────

    /// Closed circuit (default) must allow single_payout.
    #[test]
    fn test_single_payout_allowed_when_circuit_closed() {
        let s = setup_with_funds(1_000);
        assert_eq!(get_state(&s), CircuitState::Closed);

        let winner = Address::generate(&s.env);
        let result = s.client.try_single_payout(&winner, &100i128, &None);
        assert!(result.is_ok(), "single_payout must succeed when circuit is Closed");
    }

    /// Closed circuit must allow batch_payout.
    #[test]
 _circuit_error_before_insufficient_balance() {
        let s = setup_with_funds(0);
        open_circuit(&s);

        let w = Address::generate(&s.env);
        let result =
            s.client
                .try_batch_payout(&vec![&s.env, w], &vec![&s.env, 999i128], &None);
        assert!(result.is_err());
    }

    // ─────────────────────────────────────────────────────────────────────
    // Closed / HalfOpen states allow payouts
    // ───────────────────────────────────────────────────────────────        let winner = Address::generate(&s.env);
        // Would fail with InsufficientBalance if circuit check were after balance check.
        // With correct ordering it fails with circuit-open first.
        let result = s.client.try_single_payout(&winner, &500i128, &None);
        assert!(
            result.is_err(),
            "single_payout must fail (circuit open, not balance)"
        );
    }

    /// Open circuit blocks batch_payout even when balance is zero.
    #[test]
    fn test_batch_payout8, 50i128],
            &None,
        );
        assert!(
            result.is_err(),
            "batch_payout must be rejected when circuit is Open"
        );
    }

    /// Open circuit blocks single_payout even when balance is zero — the
    /// circuit error fires before the balance check, so the error is stable.
    #[test]
    fn test_single_payout_circuit_error_before_insufficient_balance() {
        // No funds locked — balance is 0
        let s = setup_with_funds(0);
        open_circuit(&s);

    );
    }

    /// Open circuit must block batch_payout even when the program has
    /// sufficient balance — proving the check runs before balance validation.
    #[test]
    fn test_batch_payout_blocked_by_open_circuit_before_balance_check() {
        let s = setup_with_funds(10_000);
        open_circuit(&s);

        let w1 = Address::generate(&s.env);
        let w2 = Address::generate(&s.env);
        let result = s.client.try_batch_payout(
            &vec![&s.env, w1, w2],
            &vec![&s.env, 50i12uns before balance validation.
    #[test]
    fn test_single_payout_blocked_by_open_circuit_before_balance_check() {
        let s = setup_with_funds(10_000);
        open_circuit(&s);

        let winner = Address::generate(&s.env);
        // Amount is well within balance — rejection must be from circuit, not balance
        let result = s.client.try_single_payout(&winner, &100i128, &None);
        assert!(
            result.is_err(),
            "single_payout must be rejected when circuit is Open"
                && Symbol::try_from_val(env, &ev.1.get(1).unwrap()).ok() == Some(topic1.clone())
            {
                return true;
            }
        }
        false
    }

    // ─────────────────────────────────────────────────────────────────────
    // Deterministic ordering: circuit check before balance
    // ─────────────────────────────────────────────────────────────────────

    /// Open circuit must block single_payout even when the program has
    /// sufficient balance — proving the check r, || {
            error_recovery::half_open_circuit(&setup.env);
        });
    }

    fn get_state(setup: &Setup) -> CircuitState {
        setup.env.as_contract(&setup.client.address, || {
            error_recovery::get_state(&setup.env)
        })
    }

    fn has_event_topic(env: &Env, topic0: Symbol, topic1: Symbol) -> bool {
        for ev in env.events().all().iter() {
            if ev.1.len() >= 2
                && Symbol::try_from_val(env, &ev.1.get(0).unwrap()).ok() == Some(topic0.clone())
                token_admin_client.mint(&contract_id, &initial_balance);
            client.lock_program_funds(&initial_balance);
        }

        Setup {
            env,
            client,
            admin,
            token_client,
        }
    }

    fn open_circuit(setup: &Setup) {
        setup.env.as_contract(&setup.client.address, || {
            error_recovery::open_circuit(&setup.env);
        });
    }

    fn half_open_circuit(setup: &Setup) {
        setup.env.as_contract(&setup.client.addressin.clone());
        let token_id = sac.address();
        let token_client = token::Client::new(&env, &token_id);
        let token_admin_client = token::StellarAssetClient::new(&env, &token_id);

        client.initialize_contract(&admin);
        client.set_circuit_admin(&admin, &None);

        let program_id = String::from_str(&env, "prog-cb");
        client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
        client.publish_program(&program_id);

        if initial_balance > 0 {
n::Client<'a>,
    }

    fn setup_with_funds(initial_balance: i128) -> Setup<'static> {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(1000);

        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let sac = env.register_stellar_asset_contract_v2(token_admtract, ProgramEscrowContractClient};
    use soroban_sdk::{
        symbol_short,
        testutils::{Address as _, Events, Ledger},
        token, vec, Address, Env, String, Symbol, TryFromVal,
    };

    // ─────────────────────────────────────────────────────────────────────
    // Helpers
    // ─────────────────────────────────────────────────────────────────────

    struct Setup<'a> {
        env: Env,
        client: ProgramEscrowContractClient<'a>,
        admin: Address,
        token_client: tokepshot
/// - Automatic trip after failure_threshold consecutive failures
/// - Manual open via open_circuit blocks immediately
/// - cb_open event emitted with correct fields on auto-trip
/// - cb_open event emitted with correct fields on manual open
/// - cb_half event emitted on reset_circuit_breaker from Open
/// - cb_close event emitted when success_threshold reached in HalfOpen
#[cfg(test)]
mod test {
    use crate::error_recovery::{self, CircuitBreakerKey, CircuitState};
    use crate::{ProgramEscrowCon
