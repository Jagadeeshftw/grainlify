#![cfg(test)]
extern crate std;

use crate::test_support::setup_program;
use crate::PAUSE_REASON_MAX_LEN;
use soroban_sdk::{Env, String};

fn reason_of_len(env: &Env, len: u32) -> String {
    let mut raw = std::string::String::new();
    for _ in 0..len {
        raw.push('a');
    }
    String::from_str(env, &raw)
}

#[test]
fn test_set_paused_accepts_empty_reason() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 0);

    let result = client.try_set_paused(
        &Some(true),
        &None,
        &None,
        &Some(String::from_str(&env, "")),
        &None,
    );

    assert!(result.is_ok(), "empty reason must be accepted");
    assert_eq!(
        client.get_pause_flags().pause_reason,
        Some(String::from_str(&env, ""))
    );
}

#[test]
fn test_set_paused_accepts_single_character_reason() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 0);

    let reason = reason_of_len(&env, 1);
    let result = client.try_set_paused(&Some(true), &None, &None, &Some(reason.clone()), &None);

    assert!(result.is_ok(), "one-character reason must be accepted");
    assert_eq!(client.get_pause_flags().pause_reason, Some(reason));
}

#[test]
fn test_set_paused_accepts_reason_at_max_length() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 0);

    let reason = reason_of_len(&env, PAUSE_REASON_MAX_LEN);
    let result = client.try_set_paused(&Some(true), &None, &None, &Some(reason.clone()), &None);

    assert!(
        result.is_ok(),
        "reason at PAUSE_REASON_MAX_LEN must be accepted"
    );
    assert_eq!(client.get_pause_flags().pause_reason, Some(reason));
}

#[test]
fn test_set_paused_rejects_reason_one_past_max_with_invalid_config() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 0);

    let reason = reason_of_len(&env, PAUSE_REASON_MAX_LEN + 1);
    let result = client.try_set_paused(&Some(true), &None, &None, &Some(reason), &None);

    match result {
        Err(Ok(error)) => assert_eq!(
            error.get_code(),
            crate::errors::ContractError::InvalidConfig as u32,
            "oversized reason must return ContractError::InvalidConfig"
        ),
        other => panic!("expected ContractError::InvalidConfig, got {other:?}"),
    }
}

#[test]
fn test_set_paused_oversized_reason_leaves_state_unchanged() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 0);

    let reason = reason_of_len(&env, PAUSE_REASON_MAX_LEN + 1);
    let result = client.try_set_paused(&Some(true), &None, &None, &Some(reason), &None);

    assert!(result.is_err(), "oversized reason must be rejected");
    let flags = client.get_pause_flags();
    assert!(!flags.lock_paused);
    assert_eq!(flags.pause_reason, None);
}

#[test]
fn test_set_paused_accepts_unpause_at_zero() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 0);

    let result = client.try_set_paused(&Some(true), &None, &None, &None, &Some(0));

    assert!(result.is_ok(), "unpause_at = 0 must be accepted");
    assert_eq!(client.get_pause_flags().lock_unpause_at, Some(0));
}

#[test]
fn test_set_paused_accepts_unpause_at_one() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 0);

    let result = client.try_set_paused(&Some(true), &None, &None, &None, &Some(1));

    assert!(result.is_ok(), "unpause_at = 1 must be accepted");
    assert_eq!(client.get_pause_flags().lock_unpause_at, Some(1));
}

#[test]
fn test_set_paused_accepts_unpause_at_u64_max() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 0);

    let result = client.try_set_paused(&Some(true), &None, &None, &None, &Some(u64::MAX));

    assert!(result.is_ok(), "unpause_at = u64::MAX must be accepted");
    assert_eq!(
        client.get_pause_flags().lock_unpause_at,
        Some(u64::MAX),
        "u64::MAX must round-trip without truncation or overflow"
    );
}

#[test]
fn test_set_paused_accepts_max_unpause_at_for_all_modes() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 0);

    let result = client.try_set_paused(
        &Some(true),
        &Some(true),
        &Some(true),
        &None,
        &Some(u64::MAX),
    );

    assert!(result.is_ok(), "u64::MAX must be accepted for every mode");
    let flags = client.get_pause_flags();
    assert_eq!(flags.lock_unpause_at, Some(u64::MAX));
    assert_eq!(flags.release_unpause_at, Some(u64::MAX));
    assert_eq!(flags.refund_unpause_at, Some(u64::MAX));
}

#[test]
fn test_set_paused_zero_unpause_at_succeeds_without_panic_for_every_mode() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 0);

    let result = client.try_set_paused(
        &Some(true),
        &Some(true),
        &Some(true),
        &Some(String::from_str(&env, "boundary")),
        &Some(0),
    );

    assert!(result.is_ok(), "unpause_at = 0 must not panic");
    let flags = client.get_pause_flags();
    assert_eq!(flags.lock_unpause_at, Some(0));
    assert_eq!(flags.release_unpause_at, Some(0));
    assert_eq!(flags.refund_unpause_at, Some(0));
    assert_eq!(flags.pause_reason, Some(String::from_str(&env, "boundary")));
}
