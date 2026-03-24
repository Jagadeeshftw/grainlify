#![cfg(test)]

use soroban_sdk::{symbol_short, testutils::Address as _, Address, Env};

use crate::nonce::{
    get_nonce, get_nonce_with_domain, validate_and_increment_nonce,
    validate_and_increment_nonce_with_domain, NonceError,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn env() -> Env {
    Env::default()
}

fn addr(e: &Env) -> Address {
    Address::generate(e)
}

// ---------------------------------------------------------------------------
// get_nonce
// ---------------------------------------------------------------------------

#[test]
fn nonce_starts_at_zero() {
    let e = env();
    let signer = addr(&e);
    assert_eq!(get_nonce(&e, &signer), 0);
}

#[test]
fn nonce_with_domain_starts_at_zero() {
    let e = env();
    let signer = addr(&e);
    assert_eq!(get_nonce_with_domain(&e, &signer, symbol_short!("pay")), 0);
}

// ---------------------------------------------------------------------------
// validate_and_increment_nonce – happy path
// ---------------------------------------------------------------------------

#[test]
fn nonce_increments_sequentially() {
    let e = env();
    let signer = addr(&e);

    for expected in 0u64..5 {
        assert_eq!(get_nonce(&e, &signer), expected);
        validate_and_increment_nonce(&e, &signer, expected).unwrap();
    }
    assert_eq!(get_nonce(&e, &signer), 5);
}

#[test]
fn nonce_with_domain_increments_sequentially() {
    let e = env();
    let signer = addr(&e);
    let domain = symbol_short!("batch");

    for expected in 0u64..3 {
        assert_eq!(get_nonce_with_domain(&e, &signer, domain.clone()), expected);
        validate_and_increment_nonce_with_domain(&e, &signer, domain.clone(), expected).unwrap();
    }
    assert_eq!(get_nonce_with_domain(&e, &signer, domain), 3);
}

// ---------------------------------------------------------------------------
// Replay protection
// ---------------------------------------------------------------------------

#[test]
fn replay_same_nonce_rejected() {
    let e = env();
    let signer = addr(&e);

    validate_and_increment_nonce(&e, &signer, 0).unwrap();

    // Replaying nonce=0 must fail
    let err = validate_and_increment_nonce(&e, &signer, 0).unwrap_err();
    assert_eq!(err, NonceError::InvalidNonce);
}

#[test]
fn replay_old_nonce_rejected_after_multiple_increments() {
    let e = env();
    let signer = addr(&e);

    validate_and_increment_nonce(&e, &signer, 0).unwrap();
    validate_and_increment_nonce(&e, &signer, 1).unwrap();
    validate_and_increment_nonce(&e, &signer, 2).unwrap();

    // All previous nonces must be rejected
    for stale in 0u64..3 {
        let err = validate_and_increment_nonce(&e, &signer, stale).unwrap_err();
        assert_eq!(err, NonceError::InvalidNonce, "stale nonce {stale} should be rejected");
    }
}

#[test]
fn skipped_nonce_rejected() {
    let e = env();
    let signer = addr(&e);

    // Current nonce is 0; skip to 5
    let err = validate_and_increment_nonce(&e, &signer, 5).unwrap_err();
    assert_eq!(err, NonceError::InvalidNonce);
    // Nonce must not have advanced
    assert_eq!(get_nonce(&e, &signer), 0);
}

#[test]
fn failed_validation_does_not_advance_nonce() {
    let e = env();
    let signer = addr(&e);

    let _ = validate_and_increment_nonce(&e, &signer, 99);
    assert_eq!(get_nonce(&e, &signer), 0, "nonce must stay at 0 after rejection");
}

// ---------------------------------------------------------------------------
// Per-signer isolation
// ---------------------------------------------------------------------------

#[test]
fn nonces_are_independent_per_signer() {
    let e = env();
    let alice = addr(&e);
    let bob = addr(&e);

    validate_and_increment_nonce(&e, &alice, 0).unwrap();
    validate_and_increment_nonce(&e, &alice, 1).unwrap();

    // Bob's nonce is still 0
    assert_eq!(get_nonce(&e, &bob), 0);
    validate_and_increment_nonce(&e, &bob, 0).unwrap();

    assert_eq!(get_nonce(&e, &alice), 2);
    assert_eq!(get_nonce(&e, &bob), 1);
}

// ---------------------------------------------------------------------------
// Domain isolation
// ---------------------------------------------------------------------------

#[test]
fn domain_nonces_are_independent_from_plain_nonces() {
    let e = env();
    let signer = addr(&e);
    let domain = symbol_short!("pay");

    validate_and_increment_nonce(&e, &signer, 0).unwrap();
    validate_and_increment_nonce(&e, &signer, 1).unwrap();

    // Domain nonce is still 0
    assert_eq!(get_nonce_with_domain(&e, &signer, domain.clone()), 0);
    validate_and_increment_nonce_with_domain(&e, &signer, domain.clone(), 0).unwrap();

    assert_eq!(get_nonce(&e, &signer), 2);
    assert_eq!(get_nonce_with_domain(&e, &signer, domain), 1);
}

#[test]
fn different_domains_have_independent_nonces() {
    let e = env();
    let signer = addr(&e);
    let d1 = symbol_short!("pay");
    let d2 = symbol_short!("batch");

    validate_and_increment_nonce_with_domain(&e, &signer, d1.clone(), 0).unwrap();
    validate_and_increment_nonce_with_domain(&e, &signer, d1.clone(), 1).unwrap();

    // d2 nonce untouched
    assert_eq!(get_nonce_with_domain(&e, &signer, d2.clone()), 0);
    validate_and_increment_nonce_with_domain(&e, &signer, d2.clone(), 0).unwrap();

    assert_eq!(get_nonce_with_domain(&e, &signer, d1), 2);
    assert_eq!(get_nonce_with_domain(&e, &signer, d2), 1);
}

#[test]
fn domain_replay_rejected() {
    let e = env();
    let signer = addr(&e);
    let domain = symbol_short!("pay");

    validate_and_increment_nonce_with_domain(&e, &signer, domain.clone(), 0).unwrap();

    let err =
        validate_and_increment_nonce_with_domain(&e, &signer, domain.clone(), 0).unwrap_err();
    assert_eq!(err, NonceError::InvalidNonce);
}

// ---------------------------------------------------------------------------
// Cross-entrypoint: plain and domain nonces share no state
// ---------------------------------------------------------------------------

#[test]
fn cross_entrypoint_nonce_isolation() {
    let e = env();
    let signer = addr(&e);
    let domain = symbol_short!("rel");

    // Use plain nonce twice
    validate_and_increment_nonce(&e, &signer, 0).unwrap();
    validate_and_increment_nonce(&e, &signer, 1).unwrap();

    // Domain nonce is still 0 – cannot reuse plain nonce on domain path
    let err = validate_and_increment_nonce_with_domain(&e, &signer, domain.clone(), 1).unwrap_err();
    assert_eq!(err, NonceError::InvalidNonce);

    validate_and_increment_nonce_with_domain(&e, &signer, domain, 0).unwrap();
}
