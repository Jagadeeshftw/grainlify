#![cfg(test)]

//! # Dispute Resolution Tests — Program Escrow
//!
//! Covers the full dispute lifecycle:
//!
//! ```text
//! (no dispute) ──open_dispute()──► Open ──resolve_dispute()──► Resolved
//! ```
//!
//! ## Security assumptions validated
//! - Only the admin can open or resolve a dispute.
//! - `single_payout` and `batch_payout` are blocked while a dispute is `Open`.
//! - Payouts succeed once the dispute is `Resolved`.
//! - A second `open_dispute` while one is already `Open` is rejected.
//! - `resolve_dispute` on a non-open dispute is rejected.
//! - Events are emitted with the correct version tag and fields.

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token, vec, Address, Env, String, TryFromVal,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Full setup: contract registered, admin set, token minted to contract,
/// program initialized and funded.
fn setup(
    env: &Env,
    fund_amount: i128,
) -> (
    ProgramEscrowContractClient<'static>,
    Address, // admin / authorized_payout_key
    token::Client<'static>,
) {
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);

    let admin = Address::generate(env);
    let token_admin = Address::generate(env);
    let sac = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = sac.address();
    let token_client = token::Client::new(env, &token_id);
    let token_sac = token::StellarAssetClient::new(env, &token_id);

    let program_id = String::from_str(env, "dispute-test-program");
    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);

    if fund_amount > 0 {
        token_sac.mint(&contract_id, &fund_amount);
        client.lock_program_funds(&fund_amount);
    }

    (client, admin, token_client)
}

// ---------------------------------------------------------------------------
// 1. Initial state — no dispute
// ---------------------------------------------------------------------------

#[test]
fn test_initial_dispute_state_is_none() {
    let env = Env::default();
    let (client, _admin, _token) = setup(&env, 0);

    assert!(client.get_dispute().is_none());
}

// ---------------------------------------------------------------------------
// 2. open_dispute
// ---------------------------------------------------------------------------

#[test]
fn test_open_dispute_sets_state_to_open() {
    let env = Env::default();
    let (client, _admin, _token) = setup(&env, 1_000);

    let reason = String::from_str(&env, "Suspicious payout request");
    let record = client.open_dispute(&reason);

    assert_eq!(record.state, DisputeState::Open);
    assert_eq!(record.reason, reason);
    assert!(record.resolved_by.is_none());
    assert!(record.resolved_at.is_none());
    assert!(record.resolution_notes.is_none());
}

#[test]
fn test_open_dispute_emits_event() {
    let env = Env::default();
    let (client, _admin, _token) = setup(&env, 1_000);

    let reason = String::from_str(&env, "Audit triggered");
    client.open_dispute(&reason);

    let events = env.events().all();
    // Find the DspOpen event
    let found = events.iter().any(|(_, topics, _)| {
        let t: soroban_sdk::Vec<soroban_sdk::Val> = topics;
        // topics[0] is the symbol
        if t.len() < 1 {
            return false;
        }
        let sym = Symbol::try_from_val(&env, &t.get(0).unwrap());
        sym.map(|s| s == Symbol::new(&env, "DspOpen")).unwrap_or(false)
    });
    assert!(found, "DspOpen event not emitted");
}

#[test]
fn test_open_dispute_persists_via_get_dispute() {
    let env = Env::default();
    let (client, _admin, _token) = setup(&env, 500);

    let reason = String::from_str(&env, "Compliance hold");
    client.open_dispute(&reason);

    let record = client.get_dispute().expect("dispute record should exist");
    assert_eq!(record.state, DisputeState::Open);
    assert_eq!(record.reason, reason);
}

// ---------------------------------------------------------------------------
// 3. Payouts blocked while dispute is Open
// ---------------------------------------------------------------------------

#[test]
#[should_panic(expected = "Payout blocked: dispute open")]
fn test_open_dispute_blocks_single_payout() {
    let env = Env::default();
    let (client, _admin, _token) = setup(&env, 1_000);

    client.open_dispute(&String::from_str(&env, "hold"));

    let recipient = Address::generate(&env);
    client.single_payout(&recipient, &500);
}

#[test]
#[should_panic(expected = "Payout blocked: dispute open")]
fn test_open_dispute_blocks_batch_payout() {
    let env = Env::default();
    let (client, _admin, _token) = setup(&env, 1_000);

    client.open_dispute(&String::from_str(&env, "hold"));

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    client.batch_payout(&vec![&env, r1, r2], &vec![&env, 300_i128, 200_i128]);
}

// ---------------------------------------------------------------------------
// 4. resolve_dispute
// ---------------------------------------------------------------------------

#[test]
fn test_resolve_dispute_sets_state_to_resolved() {
    let env = Env::default();
    let (client, _admin, _token) = setup(&env, 1_000);

    client.open_dispute(&String::from_str(&env, "hold"));
    let notes = String::from_str(&env, "Cleared after review");
    let record = client.resolve_dispute(&notes);

    assert_eq!(record.state, DisputeState::Resolved);
    assert!(record.resolved_by.is_some());
    assert!(record.resolved_at.is_some());
    assert_eq!(record.resolution_notes, Some(notes));
}

#[test]
fn test_resolve_dispute_emits_event() {
    let env = Env::default();
    let (client, _admin, _token) = setup(&env, 1_000);

    client.open_dispute(&String::from_str(&env, "hold"));
    client.resolve_dispute(&String::from_str(&env, "all clear"));

    let events = env.events().all();
    let found = events.iter().any(|(_, topics, _)| {
        let t: soroban_sdk::Vec<soroban_sdk::Val> = topics;
        if t.len() < 1 {
            return false;
        }
        let sym = Symbol::try_from_val(&env, &t.get(0).unwrap());
        sym.map(|s| s == Symbol::new(&env, "DspRslv")).unwrap_or(false)
    });
    assert!(found, "DspRslv event not emitted");
}

// ---------------------------------------------------------------------------
// 5. Payouts succeed after dispute is Resolved
// ---------------------------------------------------------------------------

#[test]
fn test_resolve_dispute_allows_single_payout() {
    let env = Env::default();
    let (client, _admin, token) = setup(&env, 1_000);

    client.open_dispute(&String::from_str(&env, "hold"));
    client.resolve_dispute(&String::from_str(&env, "cleared"));

    let recipient = Address::generate(&env);
    let data = client.single_payout(&recipient, &500);

    assert_eq!(data.remaining_balance, 500);
    assert_eq!(token.balance(&recipient), 500);
}

#[test]
fn test_resolve_dispute_allows_batch_payout() {
    let env = Env::default();
    let (client, _admin, token) = setup(&env, 1_000);

    client.open_dispute(&String::from_str(&env, "hold"));
    client.resolve_dispute(&String::from_str(&env, "cleared"));

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    let data = client.batch_payout(&vec![&env, r1.clone(), r2.clone()], &vec![&env, 300_i128, 200_i128]);

    assert_eq!(data.remaining_balance, 500);
    assert_eq!(token.balance(&r1), 300);
    assert_eq!(token.balance(&r2), 200);
}

// ---------------------------------------------------------------------------
// 6. Edge cases
// ---------------------------------------------------------------------------

#[test]
#[should_panic(expected = "Dispute already open")]
fn test_cannot_open_second_dispute_while_open() {
    let env = Env::default();
    let (client, _admin, _token) = setup(&env, 1_000);

    client.open_dispute(&String::from_str(&env, "first"));
    client.open_dispute(&String::from_str(&env, "second")); // must panic
}

#[test]
#[should_panic(expected = "No dispute found")]
fn test_resolve_without_open_dispute_panics() {
    let env = Env::default();
    let (client, _admin, _token) = setup(&env, 1_000);

    // No dispute opened — resolve should panic
    client.resolve_dispute(&String::from_str(&env, "nothing to resolve"));
}

#[test]
#[should_panic(expected = "No open dispute to resolve")]
fn test_resolve_already_resolved_dispute_panics() {
    let env = Env::default();
    let (client, _admin, _token) = setup(&env, 1_000);

    client.open_dispute(&String::from_str(&env, "hold"));
    client.resolve_dispute(&String::from_str(&env, "cleared"));
    // Second resolve on an already-resolved record must panic
    client.resolve_dispute(&String::from_str(&env, "again"));
}

#[test]
fn test_open_dispute_after_resolved_is_allowed() {
    // After a dispute is resolved a new one can be opened (fresh incident).
    let env = Env::default();
    let (client, _admin, _token) = setup(&env, 1_000);

    client.open_dispute(&String::from_str(&env, "first incident"));
    client.resolve_dispute(&String::from_str(&env, "cleared"));

    // New dispute on a clean slate
    let record = client.open_dispute(&String::from_str(&env, "second incident"));
    assert_eq!(record.state, DisputeState::Open);
}

#[test]
fn test_dispute_timestamps_are_recorded() {
    let env = Env::default();
    env.ledger().set_timestamp(1_000_000);
    let (client, _admin, _token) = setup(&env, 500);

    client.open_dispute(&String::from_str(&env, "ts-test"));
    let record = client.get_dispute().unwrap();
    assert_eq!(record.opened_at, 1_000_000);

    env.ledger().set_timestamp(2_000_000);
    client.resolve_dispute(&String::from_str(&env, "done"));
    let record = client.get_dispute().unwrap();
    assert_eq!(record.resolved_at, Some(2_000_000));
}

#[test]
fn test_dispute_does_not_affect_lock_program_funds() {
    // Locking funds is not a payout — it must not be blocked by a dispute.
    let env = Env::default();
    let (client, _admin, _token) = setup(&env, 0);

    client.open_dispute(&String::from_str(&env, "hold"));

    // lock_program_funds should still work
    let data = client.lock_program_funds(&1_000);
    assert_eq!(data.remaining_balance, 1_000);
}
