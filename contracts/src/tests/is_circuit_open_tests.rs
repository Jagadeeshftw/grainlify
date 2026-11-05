//! Coverage for `ViewFacade::is_circuit_open`.
//!
//! `is_circuit_open` is a permissionless, read-only view (see the module
//! docs in `lib.rs`). It takes no caller and performs no `require_auth`, so
//! there is no authorisation path to refuse. These tests lock in that
//! documented contract: any holder of a facade can read the flag, the
//! answer follows storage, and a call never mutates state.

use crate::{FeeConfig, ProgramData, Storage, ViewFacade};

fn program(id: &str) -> ProgramData {
    ProgramData {
        program_id: id.to_string(),
        total_locked: 1_000,
        remaining_balance: 1_000,
        is_active: true,
        metadata: String::new(),
    }
}

#[test]
fn circuit_is_closed_by_default() {
    let s = Storage::new();
    assert!(!ViewFacade::new(&s).is_circuit_open());
}

#[test]
fn reports_open_when_breaker_is_open() {
    let mut s = Storage::new();
    s.set_circuit_open(true);
    assert!(ViewFacade::new(&s).is_circuit_open());
}

#[test]
fn follows_storage_when_breaker_is_toggled() {
    let mut s = Storage::new();

    s.set_circuit_open(true);
    {
        let f = ViewFacade::new(&s);
        assert!(f.is_circuit_open());
    }

    s.set_circuit_open(false);
    {
        let f = ViewFacade::new(&s);
        assert!(!f.is_circuit_open());
    }
}

#[test]
fn is_permissionless_every_facade_sees_the_same_answer() {
    let mut s = Storage::new();
    s.set_circuit_open(true);

    // No caller identity or credentials are involved: independent facades
    // over the same storage all read the flag successfully.
    let a = ViewFacade::new(&s);
    let b = ViewFacade::new(&s);
    assert!(a.is_circuit_open());
    assert!(b.is_circuit_open());
}

#[test]
fn call_does_not_mutate_any_state() {
    let mut s = Storage::new();
    s.set_circuit_open(true);
    s.set_fee_config(FeeConfig {
        default_rate_bp: 500,
        brackets: vec![],
    });
    s.set_program(program("p1"));

    let fee_before = s.get_fee_config().cloned();
    let prog_before = s.get_program("p1").cloned();

    {
        let f = ViewFacade::new(&s);
        for _ in 0..3 {
            assert!(f.is_circuit_open());
        }
    }

    assert!(s.is_circuit_open());
    assert_eq!(s.get_fee_config().cloned(), fee_before);
    assert_eq!(s.get_program("p1").cloned(), prog_before);
}

#[test]
fn call_does_not_create_missing_state() {
    let s = Storage::new();
    assert!(!ViewFacade::new(&s).is_circuit_open());

    assert!(s.get_fee_config().is_none());
    assert!(s.get_program("p1").is_none());
}
