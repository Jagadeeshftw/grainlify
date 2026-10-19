//! Lifecycle-state test coverage for [`ViewFacade::is_circuit_open`] (Issue #2009).
//!
//! Verifies the behavior of `is_circuit_open` across every defined lifecycle state
//! of both the circuit breaker and program records, including terminal states.
//! Ensures all outcomes are explicitly asserted without relying on panics and that
//! calls from terminal states leave records immutable and unmutated.

use crate::{FeeConfig, ProgramData, Storage, ViewFacade};

// ─── Fixtures ─────────────────────────────────────────────────────────────────

fn active_program() -> ProgramData {
    ProgramData {
        program_id: "prog-active".into(),
        total_locked: 100_000,
        remaining_balance: 100_000,
        is_active: true,
        metadata: "Active Program".into(),
    }
}

fn inactive_program() -> ProgramData {
    ProgramData {
        program_id: "prog-inactive".into(),
        total_locked: 100_000,
        remaining_balance: 50_000,
        is_active: false,
        metadata: "Inactive Program".into(),
    }
}

/// Terminal state: all funds have been fully disbursed; remaining balance is zero.
fn depleted_terminal_program() -> ProgramData {
    ProgramData {
        program_id: "prog-depleted".into(),
        total_locked: 100_000,
        remaining_balance: 0,
        is_active: true,
        metadata: "Depleted Terminal Program".into(),
    }
}

/// Terminal state: program deactivated and balance depleted.
fn terminated_terminal_program() -> ProgramData {
    ProgramData {
        program_id: "prog-terminated".into(),
        total_locked: 100_000,
        remaining_balance: 0,
        is_active: false,
        metadata: "Terminated Terminal Program".into(),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 1 — Circuit Breaker Lifecycle States
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn is_circuit_open_from_closed() {
    let storage = Storage::new();
    let facade = ViewFacade::new(&storage);
    // In the initial closed state, payouts are permitted; is_circuit_open must return false.
    assert!(!facade.is_circuit_open());
}

#[test]
fn is_circuit_open_from_open() {
    let mut storage = Storage::new();
    storage.set_circuit_open(true);
    let facade = ViewFacade::new(&storage);
    // In the open state (tripped), payouts are suspended; is_circuit_open must return true.
    assert!(facade.is_circuit_open());
}

#[test]
fn is_circuit_open_from_half_open() {
    let mut storage = Storage::new();
    // Transition: Closed -> Open -> HalfOpen / Reset
    storage.set_circuit_open(true);
    assert!(storage.is_circuit_open());
    storage.set_circuit_open(false);
    let facade = ViewFacade::new(&storage);
    // Resetting circuit breaker brings it back to closed/half-open operating state.
    assert!(!facade.is_circuit_open());
}

#[test]
fn is_circuit_open_from_reset() {
    let mut storage = Storage::new();
    storage.set_circuit_open(true);
    let facade_open = ViewFacade::new(&storage);
    assert!(facade_open.is_circuit_open());

    // Admin reset
    storage.set_circuit_open(false);
    let facade_reset = ViewFacade::new(&storage);
    assert!(!facade_reset.is_circuit_open());
}

#[test]
fn is_circuit_open_from_reopened() {
    let mut storage = Storage::new();
    // Transition: Closed -> Open -> Reset -> Re-opened
    storage.set_circuit_open(true);
    storage.set_circuit_open(false);
    storage.set_circuit_open(true);
    let facade = ViewFacade::new(&storage);
    assert!(facade.is_circuit_open());
}

#[test]
fn is_circuit_open_from_idempotent_open() {
    let mut storage = Storage::new();
    storage.set_circuit_open(true);
    storage.set_circuit_open(true);
    let facade = ViewFacade::new(&storage);
    assert!(facade.is_circuit_open());
}

#[test]
fn is_circuit_open_from_idempotent_closed() {
    let mut storage = Storage::new();
    storage.set_circuit_open(false);
    storage.set_circuit_open(false);
    let facade = ViewFacade::new(&storage);
    assert!(!facade.is_circuit_open());
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 2 — Program Lifecycle States in Storage
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn is_circuit_open_from_unregistered_program_state() {
    let mut storage = Storage::new();
    let facade1 = ViewFacade::new(&storage);
    // Unregistered program with closed circuit
    assert!(!facade1.is_circuit_open());

    storage.set_circuit_open(true);
    let facade2 = ViewFacade::new(&storage);
    // Unregistered program with open circuit
    assert!(facade2.is_circuit_open());
}

#[test]
fn is_circuit_open_from_active_state() {
    let mut storage = Storage::new();
    storage.set_program(active_program());

    let facade_closed = ViewFacade::new(&storage);
    assert!(!facade_closed.is_circuit_open());

    storage.set_circuit_open(true);
    let facade_open = ViewFacade::new(&storage);
    assert!(facade_open.is_circuit_open());
}

#[test]
fn is_circuit_open_from_inactive_state() {
    let mut storage = Storage::new();
    storage.set_program(inactive_program());

    let facade_closed = ViewFacade::new(&storage);
    assert!(!facade_closed.is_circuit_open());

    storage.set_circuit_open(true);
    let facade_open = ViewFacade::new(&storage);
    assert!(facade_open.is_circuit_open());
}

#[test]
fn is_circuit_open_from_depleted_terminal_state() {
    let mut storage = Storage::new();
    storage.set_program(depleted_terminal_program());

    let facade_closed = ViewFacade::new(&storage);
    assert!(!facade_closed.is_circuit_open());

    storage.set_circuit_open(true);
    let facade_open = ViewFacade::new(&storage);
    assert!(facade_open.is_circuit_open());
}

#[test]
fn is_circuit_open_from_terminated_terminal_state() {
    let mut storage = Storage::new();
    storage.set_program(terminated_terminal_program());

    let facade_closed = ViewFacade::new(&storage);
    assert!(!facade_closed.is_circuit_open());

    storage.set_circuit_open(true);
    let facade_open = ViewFacade::new(&storage);
    assert!(facade_open.is_circuit_open());
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 3 — Terminal-State Immutability
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn is_circuit_open_terminal_depleted_state_immutability() {
    let mut storage = Storage::new();
    let prog = depleted_terminal_program();
    storage.set_program(prog.clone());
    storage.set_fee_config(FeeConfig {
        default_rate_bp: 250,
        brackets: vec![],
    });

    // Capture record state before calling is_circuit_open
    let prog_before = storage.get_program("prog-depleted").cloned().unwrap();
    let fee_cfg_before = storage.get_fee_config().cloned().unwrap();
    let circuit_before = storage.is_circuit_open();

    let facade = ViewFacade::new(&storage);
    let res = facade.is_circuit_open();
    assert_eq!(res, circuit_before);

    // Verify record immutability: neither the program nor the storage mutated
    assert_eq!(
        storage.get_program("prog-depleted").unwrap(),
        &prog_before,
        "program record was mutated by is_circuit_open in depleted terminal state"
    );
    assert_eq!(
        storage.get_fee_config().unwrap(),
        &fee_cfg_before,
        "fee config was mutated by is_circuit_open in depleted terminal state"
    );
    assert_eq!(
        storage.is_circuit_open(),
        circuit_before,
        "circuit open state was mutated by is_circuit_open in depleted terminal state"
    );
}

#[test]
fn is_circuit_open_terminal_terminated_state_immutability() {
    let mut storage = Storage::new();
    let prog = terminated_terminal_program();
    storage.set_program(prog.clone());
    storage.set_circuit_open(true);

    // Capture record state before calling is_circuit_open
    let prog_before = storage.get_program("prog-terminated").cloned().unwrap();
    let circuit_before = storage.is_circuit_open();

    let facade = ViewFacade::new(&storage);
    let res = facade.is_circuit_open();
    assert!(res);

    // Verify record immutability: neither the program nor the storage mutated
    assert_eq!(
        storage.get_program("prog-terminated").unwrap(),
        &prog_before,
        "program record was mutated by is_circuit_open in terminated terminal state"
    );
    assert_eq!(
        storage.is_circuit_open(),
        circuit_before,
        "circuit open state was mutated by is_circuit_open in terminated terminal state"
    );
}

#[test]
fn is_circuit_open_terminal_open_circuit_immutability() {
    let mut storage = Storage::new();
    storage.set_circuit_open(true);
    storage.set_program(active_program());

    let prog_before = storage.get_program("prog-active").cloned().unwrap();
    let circuit_before = storage.is_circuit_open();

    let facade = ViewFacade::new(&storage);
    // Invoke is_circuit_open repeatedly
    for _ in 0..5 {
        assert!(facade.is_circuit_open());
    }

    // Verify record unchanged
    assert_eq!(storage.get_program("prog-active").unwrap(), &prog_before);
    assert_eq!(storage.is_circuit_open(), circuit_before);
}
