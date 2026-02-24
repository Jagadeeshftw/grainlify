#![cfg(test)]

//! # Granular Pause Per-Operation Tests — Bounty Escrow
//!
//! Tests every combination of pause flags (lock, release, refund) for the
//! BountyEscrowContract to confirm that each flag blocks only its intended
//! operation and leaves all other operations unaffected.
//!
//! ## Pause Flag Matrix
//!
//! | lock_paused | release_paused | refund_paused | lock_funds | release_funds | refund |
//! |-------------|----------------|---------------|------------|---------------|--------|
//! | false       | false          | false         | ✓          | ✓             | ✓      |
//! | true        | false          | false         | ✗          | ✓             | ✓      |
//! | false       | true           | false         | ✓          | ✗             | ✓      |
//! | false       | false          | true          | ✓          | ✓             | ✗      |
//! | true        | true           | false         | ✗          | ✗             | ✓      |
//! | true        | false          | true          | ✗          | ✓             | ✗      |
//! | false       | true           | true          | ✓          | ✗             | ✗      |
//! | true        | true           | true          | ✗          | ✗             | ✗      |

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn create_token(
    env: &Env,
    admin: &Address,
) -> (token::Client<'static>, token::StellarAssetClient<'static>) {
    let addr = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    (
        token::Client::new(env, &addr),
        token::StellarAssetClient::new(env, &addr),
    )
}

fn create_escrow(env: &Env) -> (BountyEscrowContractClient<'static>, Address) {
    let id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(env, &id);
    (client, id)
}

/// Full setup: init contract + token, mint `amount` to depositor.
/// Returns `(client, admin, depositor, token_client)`.
fn setup(
    env: &Env,
    depositor_balance: i128,
) -> (
    BountyEscrowContractClient<'static>,
    Address,
    Address,
    token::Client<'static>,
) {
    env.mock_all_auths();

    let admin = Address::generate(env);
    let token_admin = Address::generate(env);
    let depositor = Address::generate(env);

    let (token_client, token_sac) = create_token(env, &token_admin);
    let (escrow_client, _) = create_escrow(env);

    escrow_client.init(&admin, &token_client.address);
    token_sac.mint(&depositor, &depositor_balance);

    (escrow_client, admin, depositor, token_client)
}

/// Lock a bounty and return its `deadline`.
fn lock_bounty(
    client: &BountyEscrowContractClient<'static>,
    env: &Env,
    depositor: &Address,
    bounty_id: u64,
    amount: i128,
) -> u64 {
    let deadline = env.ledger().timestamp() + 10_000;
    client.lock_funds(depositor, &bounty_id, &amount, &deadline);
    deadline
}

// ---------------------------------------------------------------------------
// § 1  Default state — all flags false
// ---------------------------------------------------------------------------

#[test]
fn test_default_all_flags_false() {
    let env = Env::default();
    let (client, _, _, _) = setup(&env, 0);

    let flags = client.get_pause_flags();
    assert!(!flags.lock_paused);
    assert!(!flags.release_paused);
    assert!(!flags.refund_paused);
}

// ---------------------------------------------------------------------------
// § 2  Individual flag set / unset
// ---------------------------------------------------------------------------

#[test]
fn test_set_lock_paused_only() {
    let env = Env::default();
    let (client, _, _, _) = setup(&env, 0);

    client.set_paused(&Some(true), &None, &None, &None);
    let flags = client.get_pause_flags();
    assert!(flags.lock_paused);
    assert!(!flags.release_paused);
    assert!(!flags.refund_paused);
}

#[test]
fn test_set_release_paused_only() {
    let env = Env::default();
    let (client, _, _, _) = setup(&env, 0);

    client.set_paused(&None, &Some(true), &None, &None);
    let flags = client.get_pause_flags();
    assert!(!flags.lock_paused);
    assert!(flags.release_paused);
    assert!(!flags.refund_paused);
}

#[test]
fn test_set_refund_paused_only() {
    let env = Env::default();
    let (client, _, _, _) = setup(&env, 0);

    client.set_paused(&None, &None, &Some(true), &None);
    let flags = client.get_pause_flags();
    assert!(!flags.lock_paused);
    assert!(!flags.release_paused);
    assert!(flags.refund_paused);
}

#[test]
fn test_unset_lock_paused() {
    let env = Env::default();
    let (client, _, _, _) = setup(&env, 0);

    client.set_paused(&Some(true), &None, &None, &None);
    client.set_paused(&Some(false), &None, &None, &None);
    assert!(!client.get_pause_flags().lock_paused);
}

#[test]
fn test_unset_release_paused() {
    let env = Env::default();
    let (client, _, _, _) = setup(&env, 0);

    client.set_paused(&None, &Some(true), &None, &None);
    client.set_paused(&None, &Some(false), &None, &None);
    assert!(!client.get_pause_flags().release_paused);
}

#[test]
fn test_unset_refund_paused() {
    let env = Env::default();
    let (client, _, _, _) = setup(&env, 0);

    client.set_paused(&None, &None, &Some(true), &None);
    client.set_paused(&None, &None, &Some(false), &None);
    assert!(!client.get_pause_flags().refund_paused);
}

// ---------------------------------------------------------------------------
// § 3  None arguments preserve other flags
// ---------------------------------------------------------------------------

#[test]
fn test_partial_update_preserves_other_flags() {
    let env = Env::default();
    let (client, _, _, _) = setup(&env, 0);

    client.set_paused(&Some(true), &Some(true), &Some(true), &None);

    // Only unpause release; others stay paused
    client.set_paused(&None, &Some(false), &None, &None);
    let flags = client.get_pause_flags();
    assert!(flags.lock_paused);
    assert!(!flags.release_paused);
    assert!(flags.refund_paused);
}

// ---------------------------------------------------------------------------
// § 4  lock_paused = true  ─►  lock_funds and batch_lock_funds blocked
// ---------------------------------------------------------------------------

#[test]
fn test_lock_funds_blocked_when_lock_paused() {
    let env = Env::default();
    let (client, _, depositor, _) = setup(&env, 1_000);

    client.set_paused(&Some(true), &None, &None, &None);
    let deadline = env.ledger().timestamp() + 1_000;
    let result = client.try_lock_funds(&depositor, &1, &100, &deadline);
    assert!(result.is_err());
}

#[test]
fn test_batch_lock_blocked_when_lock_paused() {
    let env = Env::default();
    let (client, _, depositor, _) = setup(&env, 1_000);

    client.set_paused(&Some(true), &None, &None, &None);
    let deadline = env.ledger().timestamp() + 1_000;
    let items = soroban_sdk::vec![
        &env,
        LockFundsItem {
            bounty_id: 1,
            depositor: depositor.clone(),
            amount: 100,
            deadline,
        }
    ];
    let result = client.try_batch_lock_funds(&items);
    assert!(result.is_err());
}

/// lock_paused does NOT block release_funds
#[test]
fn test_release_allowed_when_only_lock_paused() {
    let env = Env::default();
    let (client, _, depositor, token) = setup(&env, 1_000);

    let _deadline = lock_bounty(&client, &env, &depositor, 1, 500);
    client.set_paused(&Some(true), &None, &None, &None);

    let contributor = Address::generate(&env);
    client.release_funds(&1, &contributor);
    assert_eq!(token.balance(&contributor), 500);
}

/// lock_paused does NOT block refund (after deadline)
#[test]
fn test_refund_allowed_when_only_lock_paused() {
    let env = Env::default();
    let (client, _, depositor, token) = setup(&env, 1_000);

    let deadline = lock_bounty(&client, &env, &depositor, 1, 300);
    client.set_paused(&Some(true), &None, &None, &None);
    env.ledger().set_timestamp(deadline + 1);

    let balance_before = token.balance(&depositor);
    client.refund(&1);
    assert_eq!(token.balance(&depositor), balance_before + 300);
}

// ---------------------------------------------------------------------------
// § 5  release_paused = true  ─►  release_funds and batch_release_funds blocked
// ---------------------------------------------------------------------------

#[test]
fn test_release_funds_blocked_when_release_paused() {
    let env = Env::default();
    let (client, _, depositor, _) = setup(&env, 1_000);

    lock_bounty(&client, &env, &depositor, 1, 200);
    client.set_paused(&None, &Some(true), &None, &None);

    let contributor = Address::generate(&env);
    let result = client.try_release_funds(&1, &contributor);
    assert!(result.is_err());
}

#[test]
fn test_batch_release_blocked_when_release_paused() {
    let env = Env::default();
    let (client, _, depositor, _) = setup(&env, 1_000);

    lock_bounty(&client, &env, &depositor, 1, 200);
    client.set_paused(&None, &Some(true), &None, &None);

    let contributor = Address::generate(&env);
    let items = soroban_sdk::vec![
        &env,
        ReleaseFundsItem {
            bounty_id: 1,
            contributor,
        }
    ];
    let result = client.try_batch_release_funds(&items);
    assert!(result.is_err());
}

/// release_paused does NOT block lock_funds
#[test]
fn test_lock_allowed_when_only_release_paused() {
    let env = Env::default();
    let (client, _, depositor, _) = setup(&env, 1_000);

    client.set_paused(&None, &Some(true), &None, &None);
    let deadline = env.ledger().timestamp() + 1_000;
    client.lock_funds(&depositor, &1, &100, &deadline);

    let escrow = client.get_escrow_info(&1);
    assert_eq!(escrow.amount, 100);
}

/// release_paused does NOT block refund (after deadline)
#[test]
fn test_refund_allowed_when_only_release_paused() {
    let env = Env::default();
    let (client, _, depositor, token) = setup(&env, 1_000);

    let deadline = lock_bounty(&client, &env, &depositor, 1, 400);
    client.set_paused(&None, &Some(true), &None, &None);
    env.ledger().set_timestamp(deadline + 1);

    let before = token.balance(&depositor);
    client.refund(&1);
    assert_eq!(token.balance(&depositor), before + 400);
}

// ---------------------------------------------------------------------------
// § 6  refund_paused = true  ─►  refund blocked
// ---------------------------------------------------------------------------

#[test]
fn test_refund_blocked_when_refund_paused() {
    let env = Env::default();
    let (client, _, depositor, _) = setup(&env, 1_000);

    let deadline = lock_bounty(&client, &env, &depositor, 1, 200);
    client.set_paused(&None, &None, &Some(true), &None);
    env.ledger().set_timestamp(deadline + 1);

    let result = client.try_refund(&1);
    assert!(result.is_err());
}

/// refund_paused does NOT block lock_funds
#[test]
fn test_lock_allowed_when_only_refund_paused() {
    let env = Env::default();
    let (client, _, depositor, _) = setup(&env, 1_000);

    client.set_paused(&None, &None, &Some(true), &None);
    let deadline = env.ledger().timestamp() + 1_000;
    client.lock_funds(&depositor, &1, &100, &deadline);

    let escrow = client.get_escrow_info(&1);
    assert_eq!(escrow.amount, 100);
}

/// refund_paused does NOT block release_funds
#[test]
fn test_release_allowed_when_only_refund_paused() {
    let env = Env::default();
    let (client, _, depositor, token) = setup(&env, 1_000);

    lock_bounty(&client, &env, &depositor, 1, 300);
    client.set_paused(&None, &None, &Some(true), &None);

    let contributor = Address::generate(&env);
    client.release_funds(&1, &contributor);
    assert_eq!(token.balance(&contributor), 300);
}

// ---------------------------------------------------------------------------
// § 7  Combination: lock + release paused
// ---------------------------------------------------------------------------

#[test]
fn test_lock_blocked_when_lock_and_release_paused() {
    let env = Env::default();
    let (client, _, depositor, _) = setup(&env, 1_000);

    client.set_paused(&Some(true), &Some(true), &None, &None);
    let deadline = env.ledger().timestamp() + 1_000;
    assert!(client
        .try_lock_funds(&depositor, &1, &100, &deadline)
        .is_err());
}

#[test]
fn test_release_blocked_when_lock_and_release_paused() {
    let env = Env::default();
    let (client, _, depositor, _) = setup(&env, 1_000);

    lock_bounty(&client, &env, &depositor, 1, 200);
    client.set_paused(&Some(true), &Some(true), &None, &None);

    let contributor = Address::generate(&env);
    assert!(client.try_release_funds(&1, &contributor).is_err());
}

/// When lock + release paused, refund still works (deadline must pass)
#[test]
fn test_refund_allowed_when_lock_and_release_paused() {
    let env = Env::default();
    let (client, _, depositor, token) = setup(&env, 1_000);

    let deadline = lock_bounty(&client, &env, &depositor, 1, 200);
    client.set_paused(&Some(true), &Some(true), &None, &None);
    env.ledger().set_timestamp(deadline + 1);

    let before = token.balance(&depositor);
    client.refund(&1);
    assert_eq!(token.balance(&depositor), before + 200);
}

// ---------------------------------------------------------------------------
// § 8  Combination: lock + refund paused  (release still allowed)
// ---------------------------------------------------------------------------

#[test]
fn test_lock_blocked_when_lock_and_refund_paused() {
    let env = Env::default();
    let (client, _, depositor, _) = setup(&env, 1_000);

    client.set_paused(&Some(true), &None, &Some(true), &None);
    let deadline = env.ledger().timestamp() + 1_000;
    assert!(client
        .try_lock_funds(&depositor, &1, &100, &deadline)
        .is_err());
}

#[test]
fn test_release_allowed_when_lock_and_refund_paused() {
    let env = Env::default();
    let (client, _, depositor, token) = setup(&env, 1_000);

    lock_bounty(&client, &env, &depositor, 1, 350);
    client.set_paused(&Some(true), &None, &Some(true), &None);

    let contributor = Address::generate(&env);
    client.release_funds(&1, &contributor);
    assert_eq!(token.balance(&contributor), 350);
}

#[test]
fn test_refund_blocked_when_lock_and_refund_paused() {
    let env = Env::default();
    let (client, _, depositor, _) = setup(&env, 1_000);

    let deadline = lock_bounty(&client, &env, &depositor, 1, 200);
    client.set_paused(&Some(true), &None, &Some(true), &None);
    env.ledger().set_timestamp(deadline + 1);

    assert!(client.try_refund(&1).is_err());
}

// ---------------------------------------------------------------------------
// § 9  Combination: release + refund paused  (lock still allowed)
// ---------------------------------------------------------------------------

#[test]
fn test_lock_allowed_when_release_and_refund_paused() {
    let env = Env::default();
    let (client, _, depositor, _) = setup(&env, 1_000);

    client.set_paused(&None, &Some(true), &Some(true), &None);
    let deadline = env.ledger().timestamp() + 1_000;
    client.lock_funds(&depositor, &1, &250, &deadline);

    let escrow = client.get_escrow_info(&1);
    assert_eq!(escrow.amount, 250);
}

#[test]
fn test_release_blocked_when_release_and_refund_paused() {
    let env = Env::default();
    let (client, _, depositor, _) = setup(&env, 1_000);

    lock_bounty(&client, &env, &depositor, 1, 200);
    client.set_paused(&None, &Some(true), &Some(true), &None);

    let contributor = Address::generate(&env);
    assert!(client.try_release_funds(&1, &contributor).is_err());
}

#[test]
fn test_refund_blocked_when_release_and_refund_paused() {
    let env = Env::default();
    let (client, _, depositor, _) = setup(&env, 1_000);

    let deadline = lock_bounty(&client, &env, &depositor, 1, 200);
    client.set_paused(&None, &Some(true), &Some(true), &None);
    env.ledger().set_timestamp(deadline + 1);

    assert!(client.try_refund(&1).is_err());
}

// ---------------------------------------------------------------------------
// § 10  All flags paused
// ---------------------------------------------------------------------------

#[test]
fn test_lock_blocked_when_all_paused() {
    let env = Env::default();
    let (client, _, depositor, _) = setup(&env, 1_000);

    client.set_paused(&Some(true), &Some(true), &Some(true), &None);
    let deadline = env.ledger().timestamp() + 1_000;
    assert!(client
        .try_lock_funds(&depositor, &1, &100, &deadline)
        .is_err());
}

#[test]
fn test_release_blocked_when_all_paused() {
    let env = Env::default();
    let (client, _, depositor, _) = setup(&env, 1_000);

    lock_bounty(&client, &env, &depositor, 1, 200);
    client.set_paused(&Some(true), &Some(true), &Some(true), &None);

    let contributor = Address::generate(&env);
    assert!(client.try_release_funds(&1, &contributor).is_err());
}

#[test]
fn test_refund_blocked_when_all_paused() {
    let env = Env::default();
    let (client, _, depositor, _) = setup(&env, 1_000);

    let deadline = lock_bounty(&client, &env, &depositor, 1, 200);
    client.set_paused(&Some(true), &Some(true), &Some(true), &None);
    env.ledger().set_timestamp(deadline + 1);

    assert!(client.try_refund(&1).is_err());
}

// ---------------------------------------------------------------------------
// § 11  Resume after pause — operations restored
// ---------------------------------------------------------------------------

#[test]
fn test_lock_restored_after_unpause() {
    let env = Env::default();
    let (client, _, depositor, _) = setup(&env, 1_000);

    client.set_paused(&Some(true), &None, &None, &None);
    let deadline = env.ledger().timestamp() + 1_000;
    assert!(client
        .try_lock_funds(&depositor, &1, &100, &deadline)
        .is_err());

    client.set_paused(&Some(false), &None, &None, &None);
    client.lock_funds(&depositor, &1, &100, &deadline);
    let escrow = client.get_escrow_info(&1);
    assert_eq!(escrow.amount, 100);
}

#[test]
fn test_release_restored_after_unpause() {
    let env = Env::default();
    let (client, _, depositor, token) = setup(&env, 1_000);

    lock_bounty(&client, &env, &depositor, 1, 300);
    client.set_paused(&None, &Some(true), &None, &None);

    let contributor = Address::generate(&env);
    assert!(client.try_release_funds(&1, &contributor).is_err());

    client.set_paused(&None, &Some(false), &None, &None);
    client.release_funds(&1, &contributor);
    assert_eq!(token.balance(&contributor), 300);
}

#[test]
fn test_refund_restored_after_unpause() {
    let env = Env::default();
    let (client, _, depositor, token) = setup(&env, 1_000);

    let deadline = lock_bounty(&client, &env, &depositor, 1, 400);
    client.set_paused(&None, &None, &Some(true), &None);
    env.ledger().set_timestamp(deadline + 1);

    assert!(client.try_refund(&1).is_err());

    client.set_paused(&None, &None, &Some(false), &None);
    let before = token.balance(&depositor);
    client.refund(&1);
    assert_eq!(token.balance(&depositor), before + 400);
}

// ---------------------------------------------------------------------------
// § 12  Read-only queries unaffected by any flag
// ---------------------------------------------------------------------------

#[test]
fn test_get_escrow_info_unaffected_when_all_paused() {
    let env = Env::default();
    let (client, _, depositor, _) = setup(&env, 1_000);

    lock_bounty(&client, &env, &depositor, 1, 500);
    client.set_paused(&Some(true), &Some(true), &Some(true), &None);

    let escrow = client.get_escrow_info(&1);
    assert_eq!(escrow.amount, 500);
}

#[test]
fn test_get_balance_unaffected_when_all_paused() {
    let env = Env::default();
    let (client, _, depositor, _) = setup(&env, 1_000);

    lock_bounty(&client, &env, &depositor, 1, 500);
    client.set_paused(&Some(true), &Some(true), &Some(true), &None);

    let balance = client.get_balance();
    assert_eq!(balance, 500);
}

// ---------------------------------------------------------------------------
// § 13  batch_lock_funds and batch_release_funds honour their respective flags
// ---------------------------------------------------------------------------

#[test]
fn test_batch_lock_allowed_when_release_and_refund_paused() {
    let env = Env::default();
    let (client, _, depositor, _) = setup(&env, 1_000);

    client.set_paused(&None, &Some(true), &Some(true), &None);
    let deadline = env.ledger().timestamp() + 1_000;
    let items = soroban_sdk::vec![
        &env,
        LockFundsItem {
            bounty_id: 1,
            depositor: depositor.clone(),
            amount: 200,
            deadline,
        }
    ];
    let count = client.batch_lock_funds(&items);
    assert_eq!(count, 1);
}

#[test]
fn test_batch_release_allowed_when_lock_and_refund_paused() {
    let env = Env::default();
    let (client, _, depositor, token) = setup(&env, 1_000);

    lock_bounty(&client, &env, &depositor, 1, 250);
    client.set_paused(&Some(true), &None, &Some(true), &None);

    let contributor = Address::generate(&env);
    let items = soroban_sdk::vec![
        &env,
        ReleaseFundsItem {
            bounty_id: 1,
            contributor: contributor.clone(),
        }
    ];
    let count = client.batch_release_funds(&items);
    assert_eq!(count, 1);
    assert_eq!(token.balance(&contributor), 250);
}
