//! # `is_circuit_open` — repeat-call (twice-in-a-row) safety tests
//!
//! [`ViewFacade::is_circuit_open`] (`contracts/src/lib.rs:356`) is the
//! circuit-breaker query. Repeat submission is routine on chain: a client that
//! times out retries the same call. These tests pin down what a second call in
//! a row does, so an accidental change that lets a repeat call double-apply an
//! effect fails here instead of on chain.
//!
//! # Behaviour under test
//!
//! Calling `is_circuit_open` twice leaves the same state as calling it once.
//! The behaviour chosen is **idempotence, not refusal**: the entrypoint is a
//! pure read returning `bool`, so there is no typed error to return and no
//! state for a repeat call to double-apply. This is stated in the entrypoint's
//! doc comment in `contracts/src/lib.rs` and asserted by
//! [`test_doc_comment_states_the_repeat_call_behaviour`].
//!
//! # Acceptance criteria
//!
//! | Criterion | Test |
//! |-----------|------|
//! | Twice == once state (second call not refused) | [`test_twice_leaves_same_state_as_once`], [`test_second_call_is_not_refused_and_reports_current_state`] |
//! | Token balances asserted after both calls | [`test_twice_leaves_token_balances_untouched`] |
//! | Emitted events asserted for both calls | [`test_twice_emits_no_events`] |
//! | Behaviour stated in the doc comment | [`test_doc_comment_states_the_repeat_call_behaviour`] |

use crate::{FeeBracket, FeeConfig, ProgramData, Storage, ViewFacade};
use soroban_sdk::{
    testutils::{Address as _, Events as _},
    token, Address, Env,
};

// ─── Fixtures ─────────────────────────────────────────────────────────────────

/// Program identifier used by every fixture in this module.
const PROGRAM_ID: &str = "prog-1";

/// Escrow balance shared by the simulated program and the real SAC token, so
/// the token ledger and the facade's view of the same funds agree.
const ESCROW_BALANCE: u128 = 100_000;

/// Builds the in-process ledger the facade reads through.
///
/// `remaining_balance` mirrors the token balance the SAC holds for the escrow
/// (see [`token_ledger`]), and the fee config is deliberately non-trivial so a
/// state snapshot is a meaningful comparison.
fn storage_mirroring(remaining_balance: u128, circuit_open: bool) -> Storage {
    let mut storage = Storage::new();
    storage.set_program(ProgramData {
        program_id: PROGRAM_ID.into(),
        total_locked: remaining_balance,
        remaining_balance,
        is_active: true,
        metadata: "is_circuit_open repeat safety".into(),
    });
    storage.set_fee_config(FeeConfig {
        default_rate_bp: 250,
        brackets: vec![FeeBracket {
            ceiling: Some(10_000),
            rate_bp: 100,
        }],
    });
    storage.set_circuit_open(circuit_open);
    storage
}

/// Registers a real Stellar Asset Contract token and funds the escrow with
/// `escrow_balance` plus a third-party address with a non-zero balance.
///
/// Returns `(env, token_id, escrow, other)`. Balances at `escrow` and `other`
/// must be untouched by a repeat circuit-breaker query.
fn token_ledger(escrow_balance: u128) -> (Env, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let token_id = env
        .register_stellar_asset_contract_v2(Address::generate(&env))
        .address();
    let sac = token::StellarAssetClient::new(&env, &token_id);

    let escrow = Address::generate(&env);
    let other = Address::generate(&env);
    sac.mint(&escrow, &(escrow_balance as i128));
    sac.mint(&other, &7);

    (env, token_id, escrow, other)
}

/// Every piece of state the circuit-breaker query can read through `Storage`,
/// captured as one comparable value.
#[derive(Debug, Clone, PartialEq)]
struct StateSnapshot {
    program: Option<ProgramData>,
    fee_config: Option<FeeConfig>,
    circuit_open: bool,
}

fn snapshot(storage: &Storage) -> StateSnapshot {
    StateSnapshot {
        program: storage.get_program(PROGRAM_ID).cloned(),
        fee_config: storage.get_fee_config().cloned(),
        circuit_open: storage.is_circuit_open(),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Criterion 1 — calling twice leaves the same state as calling once
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_twice_leaves_same_state_as_once() {
    // Both breaker states: the repeat path must be identical whether the
    // breaker is open (payouts suspended) or closed.
    for circuit_open in [false, true] {
        let storage = storage_mirroring(ESCROW_BALANCE, circuit_open);
        let facade = ViewFacade::new(&storage);

        let before = snapshot(&storage);
        let first = facade.is_circuit_open();
        let after_once = snapshot(&storage);
        let second = facade.is_circuit_open();
        let after_twice = snapshot(&storage);

        assert_eq!(
            first, circuit_open,
            "first call must report the stored state"
        );
        assert_eq!(
            second, first,
            "second call must return the same value as the first"
        );
        assert_eq!(after_once, before, "one call must not mutate state");
        assert_eq!(
            after_twice, after_once,
            "the second call must leave the same state as the first"
        );
        assert_eq!(after_twice, before, "twice must equal the pre-call state");
        assert_eq!(
            after_twice.program.as_ref().unwrap().remaining_balance,
            ESCROW_BALANCE,
            "program balance must be unchanged after both calls"
        );
    }
}

#[test]
fn test_second_call_is_not_refused_and_reports_current_state() {
    // The chosen behaviour is idempotence, not refusal: the signature returns
    // `bool`, so there is no typed error a second call could return. It must
    // succeed and report the live breaker value.
    let mut storage = storage_mirroring(ESCROW_BALANCE, false);
    {
        let facade = ViewFacade::new(&storage);
        assert!(!facade.is_circuit_open(), "closed breaker: first call");
        assert!(!facade.is_circuit_open(), "closed breaker: second call");
    }

    storage.set_circuit_open(true);
    {
        let facade = ViewFacade::new(&storage);
        assert!(facade.is_circuit_open(), "open breaker: first call");
        assert!(facade.is_circuit_open(), "open breaker: second call");
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Criterion 2 — token balances asserted after both calls
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_twice_leaves_token_balances_untouched() {
    let (env, token_id, escrow, other) = token_ledger(ESCROW_BALANCE);
    // Breaker open — the state in which a real payout would be blocked — so
    // the repeat query is exercised with the escrow fully funded.
    let storage = storage_mirroring(ESCROW_BALANCE, true);
    let facade = ViewFacade::new(&storage);
    let token = token::Client::new(&env, &token_id);

    assert_eq!(
        token.balance(&escrow),
        ESCROW_BALANCE as i128,
        "fixture must fund the escrow before the calls"
    );
    let other_before = token.balance(&other);

    // First call.
    let _ = facade.is_circuit_open();
    assert_eq!(
        token.balance(&escrow),
        ESCROW_BALANCE as i128,
        "token balance after the first call"
    );
    assert_eq!(
        token.balance(&other),
        other_before,
        "third-party token balance after the first call"
    );

    // Second call (the retry).
    let _ = facade.is_circuit_open();
    assert_eq!(
        token.balance(&escrow),
        ESCROW_BALANCE as i128,
        "token balance after the second call"
    );
    assert_eq!(
        token.balance(&other),
        other_before,
        "third-party token balance after the second call"
    );

    // The program balance the facade reports still matches the token ledger.
    assert_eq!(
        storage.get_program(PROGRAM_ID).unwrap().remaining_balance,
        token.balance(&escrow) as u128,
        "program balance and token balance must stay in sync"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Criterion 3 — emitted events asserted for both calls
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_twice_emits_no_events() {
    let (env, _token_id, _escrow, _other) = token_ledger(ESCROW_BALANCE);
    let storage = storage_mirroring(ESCROW_BALANCE, true);
    let facade = ViewFacade::new(&storage);

    // Baseline after fixture setup: the query starts from here.
    let baseline = env.events().all().len();

    let _ = facade.is_circuit_open();
    let after_first = env.events().all().len();
    assert_eq!(after_first, baseline, "the first call must emit no events");

    let _ = facade.is_circuit_open();
    let after_second = env.events().all().len();
    assert_eq!(
        after_second, baseline,
        "the second call must emit no events"
    );
    assert_eq!(
        after_second, after_first,
        "both calls must emit the same number of events"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Criterion 4 — behaviour stated in the entrypoint's doc comment
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_doc_comment_states_the_repeat_call_behaviour() {
    let source = include_str!("../lib.rs");
    let marker = "/// # Repeat calls";
    let start = source
        .find(marker)
        .expect("is_circuit_open must document its repeat-call behaviour");
    let doc_block = &source[start..];
    let end = doc_block
        .find("pub fn is_circuit_open")
        .expect("the repeat-call section must document is_circuit_open");
    // Strip the `///` markers and collapse line breaks so an assertion is not
    // defeated by re-wrapping.
    let doc_block = doc_block[..end]
        .lines()
        .map(|line| line.trim().trim_start_matches("///").trim())
        .collect::<Vec<_>>()
        .join(" ");

    assert!(
        doc_block.contains("twice"),
        "the doc comment must state what happens on a second call"
    );
    assert!(
        doc_block.contains("twice leaves the same state as calling it once"),
        "the doc comment must state the chosen behaviour (twice == once)"
    );
    assert!(
        doc_block.contains("not") && doc_block.contains("typed error"),
        "the doc comment must state that a repeat call is not refused with a typed error"
    );
}
