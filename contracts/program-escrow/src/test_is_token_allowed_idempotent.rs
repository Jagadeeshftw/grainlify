//! # `is_token_allowed` repeat-call tests (issue #1990)
//!
//! `is_token_allowed` is a read-only view, so the behaviour pinned here is:
//! calling it twice in a row leaves the same state as calling it once, the
//! second call returns the same value and is never refused, and neither call
//! moves tokens or emits events.
//!
//! Each test snapshots the observable state (allowlist, token balances,
//! program balance) before the first call and asserts it again after the
//! first and after the second call. Events are asserted per call: the host
//! accumulates events across invocations, so the event log is compared before
//! the first call, after the first call and after the second call.

#![cfg(test)]

use crate::{ProgramEscrowContract, ProgramEscrowContractClient};
use soroban_sdk::{
    testutils::{Address as _, Events},
    token, Address, Env, String,
};

struct Setup<'a> {
    env: Env,
    client: ProgramEscrowContractClient<'a>,
    token: Address,
    token_client: token::Client<'a>,
    payout_key: Address,
}

impl<'a> Setup<'a> {
    /// Contract with an allowlist containing the program's own token, a
    /// program initialised with it, and 1_000 tokens locked.
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize_contract(&admin);

        let sac = env.register_stellar_asset_contract_v2(Address::generate(&env));
        let token = sac.address();
        let token_client = token::Client::new(&env, &token);
        let token_admin = token::StellarAssetClient::new(&env, &token);

        let payout_key = Address::generate(&env);
        client.initialize_program(
            &String::from_str(&env, "idem-prog"),
            &payout_key,
            &token,
            &admin,
            &None,
            &None,
        );
        token_admin.mint(&contract_id, &1_000);
        client.lock_program_funds(&1_000);

        Self {
            env,
            client,
            token,
            token_client,
            payout_key,
        }
    }

    /// Everything a repeat call could plausibly disturb.
    fn snapshot(&self) -> (i128, i128, i128, soroban_sdk::Vec<Address>, u32) {
        (
            self.token_client.balance(&self.client.address),
            self.token_client.balance(&self.payout_key),
            self.client.get_remaining_balance(),
            self.client.get_allowed_tokens(),
            self.client.get_allowed_tokens_with_decimals().len(),
        )
    }

    fn event_count(&self) -> u32 {
        self.env.events().all().len()
    }

    fn assert_no_new_events(&self, baseline: u32, call: &str) {
        assert_eq!(
            self.event_count(),
            baseline,
            "{call} call to is_token_allowed must emit no events"
        );
    }
}

#[test]
fn empty_allowlist_twice_returns_true_and_changes_nothing() {
    let s = Setup::new();
    let other = Address::generate(&s.env);
    let before = s.snapshot();
    let events = s.event_count();

    assert!(s.client.is_token_allowed(&other));
    s.assert_no_new_events(events, "first");
    assert_eq!(s.snapshot(), before, "state after first call");

    assert!(s.client.is_token_allowed(&other));
    s.assert_no_new_events(events, "second");
    assert_eq!(s.snapshot(), before, "state after second call");

    // Token balances are untouched by both calls.
    assert_eq!(s.token_client.balance(&s.client.address), 1_000);
    assert_eq!(s.token_client.balance(&s.payout_key), 0);
}

#[test]
fn listed_token_twice_returns_true_and_changes_nothing() {
    let s = Setup::new();
    s.client.add_allowed_token(&s.token);
    let before = s.snapshot();
    let events = s.event_count();

    assert!(s.client.is_token_allowed(&s.token));
    s.assert_no_new_events(events, "first");
    assert_eq!(s.snapshot(), before, "state after first call");

    assert!(s.client.is_token_allowed(&s.token));
    s.assert_no_new_events(events, "second");
    assert_eq!(s.snapshot(), before, "state after second call");
}

#[test]
fn unlisted_token_twice_returns_false_without_error_or_state_change() {
    let s = Setup::new();
    s.client.add_allowed_token(&s.token);
    let unlisted = Address::generate(&s.env);
    let before = s.snapshot();
    let events = s.event_count();

    // `try_` proves the second call is not refused with a typed error.
    assert_eq!(s.client.try_is_token_allowed(&unlisted), Ok(Ok(false)));
    s.assert_no_new_events(events, "first");
    assert_eq!(s.snapshot(), before, "state after first call");

    assert_eq!(s.client.try_is_token_allowed(&unlisted), Ok(Ok(false)));
    s.assert_no_new_events(events, "second");
    assert_eq!(s.snapshot(), before, "state after second call");
}

#[test]
fn repeat_calls_do_not_modify_the_allowlist() {
    let s = Setup::new();
    let a = Address::generate(&s.env);
    let b = Address::generate(&s.env);
    s.client.add_allowed_token(&a);
    s.client.add_allowed_token(&b);
    let before = s.client.get_allowed_tokens_with_decimals();

    for _ in 0..2 {
        assert!(s.client.is_token_allowed(&a));
        assert!(s.client.is_token_allowed(&b));
        assert!(!s.client.is_token_allowed(&s.token));
    }

    assert_eq!(s.client.get_allowed_tokens_with_decimals(), before);
}

#[test]
fn result_changes_only_when_the_allowlist_changes() {
    let s = Setup::new();
    let t = Address::generate(&s.env);

    // Empty list: allowed, twice.
    assert!(s.client.is_token_allowed(&t));
    assert!(s.client.is_token_allowed(&t));

    // Another token listed: `t` is now refused, twice.
    s.client.add_allowed_token(&s.token);
    assert!(!s.client.is_token_allowed(&t));
    assert!(!s.client.is_token_allowed(&t));

    // `t` listed: allowed, twice.
    s.client.add_allowed_token(&t);
    assert!(s.client.is_token_allowed(&t));
    assert!(s.client.is_token_allowed(&t));

    // `t` removed again: refused, twice (the other token is still listed).
    s.client.remove_allowed_token(&t);
    assert!(!s.client.is_token_allowed(&t));
    assert!(!s.client.is_token_allowed(&t));
}

#[test]
fn calling_the_view_between_mutations_does_not_affect_their_events() {
    let s = Setup::new();
    let t = Address::generate(&s.env);

    let events = s.event_count();
    s.client.is_token_allowed(&t);
    s.client.is_token_allowed(&t);
    s.assert_no_new_events(events, "repeat");

    // The mutation still emits its own events.
    s.client.add_allowed_token(&t);
    let after_add = s.event_count();
    assert!(after_add > events, "add_allowed_token must emit events");

    // And the view calls after it emit none.
    s.client.is_token_allowed(&t);
    s.client.is_token_allowed(&t);
    s.assert_no_new_events(after_add, "post-mutation");
}
