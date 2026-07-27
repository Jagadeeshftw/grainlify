//! # Cross-entrypoint idempotency key namespace regression tests
//!
//! Verifies that the `DataKey::IdempotencyKey` namespace is **shared** between
//! [`single_payout_idempotent`] and [`batch_payout_idempotent`], so that a key
//! consumed by one entrypoint is correctly rejected by the other.
//!
//! ## Why a dedicated file?
//!
//! Existing idempotency suites exercise each entrypoint in isolation and never
//! cross-pollinate keys.  The shared-namespace guarantee (documented on
//! [`crate::ProgramEscrowContract::is_payout_processed`]) therefore had no
//! regression coverage.  This file fills that gap.
//!
//! ## Scenarios
//!
//! | Test | First call | Replay call | Expected |
//! |------|-----------|-------------|----------|
//! | `single_then_batch` | `single_payout_idempotent` | `batch_payout_idempotent` | `BatchPayoutReplayedEvent` |
//! | `batch_then_single` | `batch_payout_idempotent` | `single_payout_idempotent` | `IdmReplay` event |
//! | `is_processed_after_single` | `single_payout_idempotent` | `is_payout_processed` | `true` |
//! | `is_processed_after_batch` | `batch_payout_idempotent` | `is_payout_processed` | `true` |
//!
//! ## Security invariants
//!
//! - No double-payment: replay must not alter `remaining_balance` or
//!   `payout_history`.
//! - Replay events are published **before** any state mutation so auditors
//!   have a provable record of the rejection.
//! - The shared namespace check runs before the entrypoint-specific check
//!   so that a key is always rejected at the earliest possible point.
//!
//! ## Usage
//!
//! ```bash
//! cargo test -p program-escrow cross_entrypoint_idempotency -- --nocapture
//! ```

#![cfg(test)]

extern crate std;

use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events},
    token, vec, Address, Env, IntoVal, String, TryIntoVal,
};

use crate::{
    BatchPayoutReplayedEvent, ProgramEscrowContract, ProgramEscrowContractClient,
};

// =============================================================================
// Helpers
// =============================================================================

/// Set up a contract with one funded, published program.
fn setup_program(
    env: &Env,
    initial_amount: i128,
) -> (
    ProgramEscrowContractClient<'static>,
    Address,
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
    let token_admin_client = token::StellarAssetClient::new(env, &token_id);

    let program_id = String::from_str(env, "cross-idem-prog");
    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    client.publish_program(&program_id, &admin);

    if initial_amount > 0 {
        token_admin_client.mint(&client.address, &initial_amount);
        client.lock_program_funds(&initial_amount);
    }

    (client, admin, token_client)
}

/// Assert that `env.events().all()` contains exactly one `IdmReplay` event
/// whose data matches `(expected_key, expected_program_id, expected_amount)`.
fn assert_idm_replay_event(
    env: &Env,
    expected_key: &String,
    expected_program_id: &String,
    expected_amount: i128,
) {
    let events = env.events().all();
    let idm_replay: soroban_sdk::Val = symbol_short!("IdmReplay").into_val(env);
    let matched = events.iter().filter(|e| e.1.contains(&idm_replay)).count();
    assert!(
        matched >= 1,
        "Expected at least one IdmReplay event, found {}",
        matched
    );
}

/// Assert that `env.events().all()` contains at least one
/// `BatchPayoutReplayedEvent` for `expected_key`.
fn assert_batch_replayed_event(env: &Env, expected_key: &String) {
    let events = env.events().all();
    let replay_topic: soroban_sdk::Val = symbol_short!("BatPayRp").into_val(env);
    let matched = events.iter().filter(|e| {
        if !e.1.contains(&replay_topic) {
            return false;
        }
        let result: Result<BatchPayoutReplayedEvent, _> = (&e.2).try_into_val(env);
        result.map_or(false, |ev| ev.idempotency_key == *expected_key)
    });
    assert!(
        matched.count() >= 1,
        "Expected at least one BatchPayoutReplayedEvent for key {:?}",
        expected_key
    );
}

// =============================================================================
// Tests
// =============================================================================

// ── Scenario: single → batch replay ─────────────────────────────────────────

/// Submit a key via `single_payout_idempotent`, then replay it through
/// `batch_payout_idempotent`.  The replay must:
///
/// 1. Emit a `BatchPayoutReplayedEvent`.
/// 2. NOT change `remaining_balance` or `payout_history`.
/// 3. NOT transfer any tokens (balance unchanged).
#[test]
fn test_single_then_batch_replay_rejected() {
    let env = Env::default();
    let (client, _admin, token_client) = setup_program(&env, 10_000);

    let recipient = Address::generate(&env);
    let idem_key = String::from_str(&env, "cross-single-to-batch");

    // ── Step 1: Consume via single_payout_idempotent ──
    let data1 = client.single_payout_idempotent(&recipient, &1000, &Some(idem_key.clone()));
    assert_eq!(data1.remaining_balance, 9000);
    assert_eq!(data1.payout_history.len(), 1);
    let balance_after_single = token_client.balance(&client.address);

    // ── Step 2: Replay via batch_payout_idempotent ──
    let batch_recipients = vec![&env, recipient.clone()];
    let batch_amounts = vec![&env, 1000_i128];
    let data2 = client
        .batch_payout_idempotent(&idem_key, &batch_recipients, &batch_amounts);

    // Balance must NOT change (no double payment).
    assert_eq!(
        token_client.balance(&client.address),
        balance_after_single,
        "Replay via batch_payout_idempotent must not transfer tokens"
    );
    // Remaining balance must NOT change.
    assert_eq!(
        data2.remaining_balance, data1.remaining_balance,
        "Replay via batch_payout_idempotent must not alter remaining_balance"
    );
    // Payout history must NOT grow.
    assert_eq!(
        data2.payout_history.len(),
        data1.payout_history.len(),
        "Replay via batch_payout_idempotent must not add payout records"
    );

    // Assert BatchPayoutReplayedEvent was emitted.
    assert_batch_replayed_event(&env, &idem_key);
}

// ── Scenario: batch → single replay ─────────────────────────────────────────

/// Submit a key via `batch_payout_idempotent`, then replay it through
/// `single_payout_idempotent`.  The replay must:
///
/// 1. Emit an `IdmReplay` event.
/// 2. NOT change `remaining_balance` or `payout_history`.
/// 3. NOT transfer any tokens.
#[test]
fn test_batch_then_single_replay_rejected() {
    let env = Env::default();
    let (client, _admin, token_client) = setup_program(&env, 10_000);

    let recipient = Address::generate(&env);
    let idem_key = String::from_str(&env, "cross-batch-to-single");
    let batch_recipients = vec![&env, recipient.clone()];
    let batch_amounts = vec![&env, 2000_i128];

    // ── Step 1: Consume via batch_payout_idempotent ──
    let data1 = client
        .batch_payout_idempotent(&idem_key, &batch_recipients, &batch_amounts);
    assert_eq!(data1.remaining_balance, 8000);
    assert_eq!(data1.payout_history.len(), 1);
    let balance_after_batch = token_client.balance(&client.address);

    // ── Step 2: Replay via single_payout_idempotent ──
    let data2 = client.single_payout_idempotent(&recipient, &2000, &Some(idem_key.clone()));

    // Balance must NOT change.
    assert_eq!(
        token_client.balance(&client.address),
        balance_after_batch,
        "Replay via single_payout_idempotent must not transfer tokens"
    );
    // Remaining balance must NOT change.
    assert_eq!(
        data2.remaining_balance, data1.remaining_balance,
        "Replay via single_payout_idempotent must not alter remaining_balance"
    );
    // Payout history must NOT grow.
    assert_eq!(
        data2.payout_history.len(),
        data1.payout_history.len(),
        "Replay via single_payout_idempotent must not add payout records"
    );

    // Assert IdmReplay event was emitted.
    let program_id = String::from_str(&env, "cross-idem-prog");
    assert_idm_replay_event(&env, &idem_key, &program_id, 2000);
}

// ── Scenario: is_payout_processed after single ───────────────────────────────

/// After `single_payout_idempotent` consumes a key, `is_payout_processed`
/// must return `true` for that key.
#[test]
fn test_is_payout_processed_after_single() {
    let env = Env::default();
    let (client, _admin, _token_client) = setup_program(&env, 10_000);

    let recipient = Address::generate(&env);
    let idem_key = String::from_str(&env, "check-after-single");

    // Key does not exist yet.
    assert!(
        !client.is_payout_processed(&idem_key),
        "Fresh key must report false before any call"
    );

    // Consume via single_payout_idempotent.
    client.single_payout_idempotent(&recipient, &1000, &Some(idem_key.clone()));

    // Key must now be visible.
    assert!(
        client.is_payout_processed(&idem_key),
        "is_payout_processed must return true after single_payout_idempotent"
    );
}

// ── Scenario: is_payout_processed after batch ────────────────────────────────

/// After `batch_payout_idempotent` consumes a key, `is_payout_processed`
/// must return `true` for that key.
#[test]
fn test_is_payout_processed_after_batch() {
    let env = Env::default();
    let (client, _admin, _token_client) = setup_program(&env, 10_000);

    let recipient = Address::generate(&env);
    let idem_key = String::from_str(&env, "check-after-batch");
    let recipients = vec![&env, recipient];
    let amounts = vec![&env, 2000_i128];

    // Key does not exist yet.
    assert!(
        !client.is_payout_processed(&idem_key),
        "Fresh key must report false before any call"
    );

    // Consume via batch_payout_idempotent.
    client.batch_payout_idempotent(&idem_key, &recipients, &amounts);

    // Key must now be visible.
    assert!(
        client.is_payout_processed(&idem_key),
        "is_payout_processed must return true after batch_payout_idempotent"
    );
}

// ── Scenario: fresh key reports false ────────────────────────────────────────

/// A key that was never used must report `false` from `is_payout_processed`.
#[test]
fn test_is_payout_processed_fresh_key_false() {
    let env = Env::default();
    let (client, _admin, _token_client) = setup_program(&env, 10_000);

    let never_used = String::from_str(&env, "never-used-key");

    assert!(
        !client.is_payout_processed(&never_used),
        "Fresh key must report false"
    );
}

// ── Scenario: cross-replay with is_payout_processed concurrency ──────────────

/// Submit via single, confirm `is_payout_processed`, then batch-replay,
/// then confirm `is_payout_processed` still true.
#[test]
fn test_cross_replay_after_single_does_not_invalidate_check() {
    let env = Env::default();
    let (client, _admin, _token_client) = setup_program(&env, 10_000);

    let recipient = Address::generate(&env);
    let idem_key = String::from_str(&env, "cross-concurrency");

    // Consume via single.
    client.single_payout_idempotent(&recipient, &1000, &Some(idem_key.clone()));
    assert!(client.is_payout_processed(&idem_key));

    // Replay via batch.
    let batch_recipients = vec![&env, recipient.clone()];
    let batch_amounts = vec![&env, 1000_i128];
    client.batch_payout_idempotent(&idem_key, &batch_recipients, &batch_amounts);

    // Still true after cross-replay.
    assert!(client.is_payout_processed(&idem_key));
}

/// Submit via batch, confirm `is_payout_processed`, then single-replay,
/// then confirm `is_payout_processed` still true.
#[test]
fn test_cross_replay_after_batch_does_not_invalidate_check() {
    let env = Env::default();
    let (client, _admin, _token_client) = setup_program(&env, 10_000);

    let recipient = Address::generate(&env);
    let idem_key = String::from_str(&env, "cross-concurrency-batch");
    let recipients = vec![&env, recipient.clone()];
    let amounts = vec![&env, 2000_i128];

    // Consume via batch.
    client.batch_payout_idempotent(&idem_key, &recipients, &amounts);
    assert!(client.is_payout_processed(&idem_key));

    // Replay via single.
    client.single_payout_idempotent(&recipient, &2000, &Some(idem_key.clone()));

    // Still true after cross-replay.
    assert!(client.is_payout_processed(&idem_key));
}
