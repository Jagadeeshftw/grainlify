#![cfg(test)]

//! Repeat-submission safety for `set_paused` (issue #2006).
//!
//! A client that retries after a timeout submits the same `set_paused` call
//! twice. These tests pin the documented behaviour:
//! - state after the second call equals state after the first (idempotent);
//! - the second call is not refused;
//! - both calls emit `PauseStateChanged` and `PauseStateChangedV2`, each with a
//!   fresh `receipt_id`, and the second V2 event reports `previous_paused == paused`;
//! - token balances never change.

extern crate std;

use super::*;
use crate::test_support::setup_program;
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events, Ledger},
    token, Address, Env, String, TryIntoVal,
};

const FUNDS: i128 = 10_000;

fn balances(
    token: &token::Client<'_>,
    contract: &Address,
    admin: &Address,
    bystander: &Address,
) -> (i128, i128, i128) {
    (
        token.balance(contract),
        token.balance(admin),
        token.balance(bystander),
    )
}

fn flags(env: &Env, contract: &Address) -> PauseFlags {
    env.as_contract(contract, || ProgramEscrowContract::get_pause_flags(env))
}

/// Pause events (V1, V2) published by `contract` since event index `before`.
fn pause_events_since(
    env: &Env,
    before: u32,
    contract: &Address,
) -> (
    std::vec::Vec<PauseStateChanged>,
    std::vec::Vec<PauseStateChangedV2>,
) {
    let all = env.events().all();
    let mut v1 = std::vec::Vec::new();
    let mut v2 = std::vec::Vec::new();
    for i in before..all.len() {
        let (addr, topics, data) = all.get(i).unwrap();
        if addr != *contract {
            continue;
        }
        match topics.len() {
            1 => v1.push(data.try_into_val(env).unwrap()),
            2 => v2.push(data.try_into_val(env).unwrap()),
            n => panic!("unexpected pause event with {} topics", n),
        }
    }
    (v1, v2)
}

#[test]
fn test_set_paused_twice_pause_is_idempotent_and_reemits_events() {
    let env = Env::default();
    let (client, admin, token, _sac) = setup_program(&env, FUNDS);
    let bystander = Address::generate(&env);
    let reason = Some(String::from_str(&env, "incident-42"));
    let unpause_at = Some(10_000u64);

    let bal_start = balances(&token, &client.address, &admin, &bystander);
    assert_eq!(bal_start.0, FUNDS);

    // First submission at t=1000.
    env.ledger().with_mut(|l| l.timestamp = 1_000);
    let before1 = env.events().all().len();
    client.set_paused(&Some(true), &Some(true), &None, &reason, &unpause_at);
    let flags1 = flags(&env, &client.address);
    let bal1 = balances(&token, &client.address, &admin, &bystander);
    let (v1_a, v2_a) = pause_events_since(&env, before1, &client.address);

    // Retry at t=2000: must not panic.
    env.ledger().with_mut(|l| l.timestamp = 2_000);
    let before2 = env.events().all().len();
    client.set_paused(&Some(true), &Some(true), &None, &reason, &unpause_at);
    let flags2 = flags(&env, &client.address);
    let bal2 = balances(&token, &client.address, &admin, &bystander);
    let (v1_b, v2_b) = pause_events_since(&env, before2, &client.address);

    // State: identical after both calls; paused_at keeps the first timestamp.
    assert_eq!(flags1, flags2);
    assert!(flags2.lock_paused && flags2.release_paused && !flags2.refund_paused);
    assert_eq!(flags2.paused_at, 1_000);
    assert_eq!(flags2.pause_reason, reason);
    assert_eq!(flags2.lock_unpause_at, unpause_at);
    assert_eq!(flags2.release_unpause_at, unpause_at);

    // Token balances: unchanged after both calls.
    assert_eq!(bal1, bal_start);
    assert_eq!(bal2, bal_start);

    // Events: both calls emit one V1 + one V2 per mode (lock, release).
    for (v1, v2) in [(&v1_a, &v2_a), (&v1_b, &v2_b)] {
        assert_eq!(v1.len(), 2);
        assert_eq!(v2.len(), 2);
        assert_eq!(v2[0].operation, symbol_short!("lock"));
        assert_eq!(v2[1].operation, symbol_short!("release"));
        for i in 0..2 {
            assert!(v1[i].paused && v2[i].paused);
            assert_eq!(v1[i].receipt_id, v2[i].receipt_id);
            assert_eq!(v2[i].actor, admin);
            assert_eq!(v2[i].reason, reason);
        }
    }
    // First call transitions false -> true; the retry reports true -> true.
    assert!(!v2_a[0].previous_paused && !v2_a[1].previous_paused);
    assert!(v2_b[0].previous_paused && v2_b[1].previous_paused);
    // Each call carries its own ledger timestamp.
    assert_eq!(v2_a[0].timestamp, 1_000);
    assert_eq!(v2_b[0].timestamp, 2_000);
    // Receipt ids advance on the retry.
    assert!(v2_b[0].receipt_id > v2_a[1].receipt_id);
    assert!(v2_b[1].receipt_id > v2_b[0].receipt_id);
}

#[test]
fn test_set_paused_twice_unpause_is_idempotent_and_reemits_events() {
    let env = Env::default();
    let (client, admin, token, _sac) = setup_program(&env, FUNDS);
    let bystander = Address::generate(&env);
    let reason = Some(String::from_str(&env, "incident-42"));

    env.ledger().with_mut(|l| l.timestamp = 1_000);
    client.set_paused(&Some(true), &None, &Some(true), &reason, &None);
    let bal_start = balances(&token, &client.address, &admin, &bystander);

    // First unpause at t=2000.
    env.ledger().with_mut(|l| l.timestamp = 2_000);
    let before1 = env.events().all().len();
    client.set_paused(&Some(false), &None, &Some(false), &None, &None);
    let flags1 = flags(&env, &client.address);
    let bal1 = balances(&token, &client.address, &admin, &bystander);
    let (v1_a, v2_a) = pause_events_since(&env, before1, &client.address);

    // Retry at t=3000: must not panic.
    env.ledger().with_mut(|l| l.timestamp = 3_000);
    let before2 = env.events().all().len();
    client.set_paused(&Some(false), &None, &Some(false), &None, &None);
    let flags2 = flags(&env, &client.address);
    let bal2 = balances(&token, &client.address, &admin, &bystander);
    let (v1_b, v2_b) = pause_events_since(&env, before2, &client.address);

    // State: fully unpaused, shared metadata cleared, identical after both calls.
    assert_eq!(flags1, flags2);
    assert!(!flags2.lock_paused && !flags2.release_paused && !flags2.refund_paused);
    assert_eq!(flags2.pause_reason, None);
    assert_eq!(flags2.paused_at, 0);
    assert_eq!(flags2.lock_unpause_at, None);
    assert_eq!(flags2.refund_unpause_at, None);

    // Token balances: unchanged after both calls.
    assert_eq!(bal1, bal_start);
    assert_eq!(bal2, bal_start);

    // Events: both calls emit one V1 + one V2 per mode (lock, refund).
    for (v1, v2) in [(&v1_a, &v2_a), (&v1_b, &v2_b)] {
        assert_eq!(v1.len(), 2);
        assert_eq!(v2.len(), 2);
        assert_eq!(v2[0].operation, symbol_short!("lock"));
        assert_eq!(v2[1].operation, symbol_short!("refund"));
        for i in 0..2 {
            assert!(!v1[i].paused && !v2[i].paused);
            assert_eq!(v1[i].receipt_id, v2[i].receipt_id);
        }
    }
    // First call transitions true -> false; the retry reports false -> false.
    assert!(v2_a[0].previous_paused && v2_a[1].previous_paused);
    assert!(!v2_b[0].previous_paused && !v2_b[1].previous_paused);
    assert!(v2_b[0].receipt_id > v2_a[1].receipt_id);
}
