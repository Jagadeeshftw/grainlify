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

// ---------------------------------------------------------------------------
// § 14  authorize_claim and claim honour release_paused
// ---------------------------------------------------------------------------

#[test]
fn test_authorize_claim_blocked_when_release_paused() {
    let env = Env::default();
    let (client, _, depositor, _) = setup(&env, 1_000);

    lock_bounty(&client, &env, &depositor, 1, 500);
    client.set_claim_window(&3600);
    client.set_paused(&None, &Some(true), &None, &None);

    let contributor = Address::generate(&env);
    let result = client.try_authorize_claim(&1, &contributor, &DisputeReason::Other);
    assert!(result.is_err());
}

#[test]
fn test_authorize_claim_allowed_when_lock_and_refund_paused() {
    let env = Env::default();
    let (client, _, depositor, _) = setup(&env, 1_000);

    lock_bounty(&client, &env, &depositor, 1, 500);
    client.set_claim_window(&3600);
    client.set_paused(&Some(true), &None, &Some(true), &None);

    let contributor = Address::generate(&env);
    client.authorize_claim(&1, &contributor, &DisputeReason::Other);

    let claim = client.get_pending_claim(&1);
    assert_eq!(claim.amount, 500);
}

#[test]
fn test_claim_blocked_when_release_paused() {
    let env = Env::default();
    let (client, _, depositor, _) = setup(&env, 1_000);

    lock_bounty(&client, &env, &depositor, 1, 500);
    client.set_claim_window(&3600);

    let contributor = Address::generate(&env);
    client.authorize_claim(&1, &contributor, &DisputeReason::Other);

    // Now pause release — claim should be blocked
    client.set_paused(&None, &Some(true), &None, &None);
    let result = client.try_claim(&1);
    assert!(result.is_err());
}

#[test]
fn test_claim_allowed_when_only_lock_paused() {
    let env = Env::default();
    let (client, _, depositor, token) = setup(&env, 1_000);

    lock_bounty(&client, &env, &depositor, 1, 500);
    client.set_claim_window(&3600);

    let contributor = Address::generate(&env);
    client.authorize_claim(&1, &contributor, &DisputeReason::Other);

    client.set_paused(&Some(true), &None, &None, &None);
    client.claim(&1);
    assert_eq!(token.balance(&contributor), 500);
}

// ---------------------------------------------------------------------------
// § 15  Pause metadata — reason and paused_at
// ---------------------------------------------------------------------------

#[test]
fn test_pause_reason_stored_and_cleared() {
    let env = Env::default();
    let (client, _, _, _) = setup(&env, 0);

    let reason = soroban_sdk::String::from_str(&env, "security incident");
    client.set_paused(&Some(true), &None, &None, &Some(reason));

    let flags = client.get_pause_flags();
    assert!(flags.lock_paused);
    assert!(flags.pause_reason.is_some());

    // Unpause all — reason should be cleared
    client.set_paused(&Some(false), &None, &None, &None);
    let flags = client.get_pause_flags();
    assert!(flags.pause_reason.is_none());
    assert_eq!(flags.paused_at, 0);
}

#[test]
fn test_paused_at_set_on_first_pause() {
    let env = Env::default();
    let (client, _, _, _) = setup(&env, 0);

    env.ledger().set_timestamp(42_000);
    client.set_paused(&Some(true), &None, &None, &None);

    let flags = client.get_pause_flags();
    assert_eq!(flags.paused_at, 42_000);
}

#[test]
fn test_paused_at_not_overwritten_by_second_flag() {
    let env = Env::default();
    let (client, _, _, _) = setup(&env, 0);

    env.ledger().set_timestamp(10_000);
    client.set_paused(&Some(true), &None, &None, &None);

    env.ledger().set_timestamp(20_000);
    client.set_paused(&None, &Some(true), &None, &None);

    // paused_at should still reflect the first pause
    let flags = client.get_pause_flags();
    assert_eq!(flags.paused_at, 10_000);
}

#[test]
fn test_paused_at_resets_after_full_unpause_and_repause() {
    let env = Env::default();
    let (client, _, _, _) = setup(&env, 0);

    env.ledger().set_timestamp(5_000);
    client.set_paused(&Some(true), &None, &None, &None);
    assert_eq!(client.get_pause_flags().paused_at, 5_000);

    // Fully unpause
    client.set_paused(&Some(false), &None, &None, &None);
    assert_eq!(client.get_pause_flags().paused_at, 0);

    // Re-pause at a later time
    env.ledger().set_timestamp(50_000);
    client.set_paused(&None, &Some(true), &None, &None);
    assert_eq!(client.get_pause_flags().paused_at, 50_000);
}

// ---------------------------------------------------------------------------
// § 16  Rapid toggle — pause/unpause/pause stability
// ---------------------------------------------------------------------------

#[test]
fn test_rapid_toggle_lock_flag() {
    let env = Env::default();
    let (client, _, depositor, _) = setup(&env, 5_000);

    for round in 0u64..5 {
        client.set_paused(&Some(true), &None, &None, &None);
        let deadline = env.ledger().timestamp() + 1_000;
        assert!(client
            .try_lock_funds(&depositor, &(round * 2), &100, &deadline)
            .is_err());

        client.set_paused(&Some(false), &None, &None, &None);
        client.lock_funds(&depositor, &(round * 2 + 1), &100, &deadline);
    }
}

// ---------------------------------------------------------------------------
// § 17  Multiple bounties under mixed pause states
// ---------------------------------------------------------------------------

#[test]
fn test_multiple_bounties_lock_then_selective_release_and_refund() {
    let env = Env::default();
    let (client, _, depositor, token) = setup(&env, 5_000);

    // Lock three bounties while everything is unpaused
    let deadline = lock_bounty(&client, &env, &depositor, 10, 500);
    lock_bounty(&client, &env, &depositor, 11, 600);
    lock_bounty(&client, &env, &depositor, 12, 700);

    // Pause release, leave refund open
    client.set_paused(&None, &Some(true), &None, &None);

    let contributor = Address::generate(&env);
    assert!(client.try_release_funds(&10, &contributor).is_err());

    // Refund bounty 12 after deadline
    env.ledger().set_timestamp(deadline + 1);
    let before = token.balance(&depositor);
    client.refund(&12);
    assert_eq!(token.balance(&depositor), before + 700);

    // Unpause release, pause refund
    client.set_paused(&None, &Some(false), &Some(true), &None);

    // Release bounty 10 now succeeds
    client.release_funds(&10, &contributor);
    assert_eq!(token.balance(&contributor), 500);

    // Refund bounty 11 should fail (refund paused)
    assert!(client.try_refund(&11).is_err());
}

// ---------------------------------------------------------------------------
// § 18  Idempotent flag updates
// ---------------------------------------------------------------------------

#[test]
fn test_setting_already_paused_flag_is_idempotent() {
    let env = Env::default();
    let (client, _, _, _) = setup(&env, 0);

    client.set_paused(&Some(true), &None, &None, &None);
    client.set_paused(&Some(true), &None, &None, &None);

    let flags = client.get_pause_flags();
    assert!(flags.lock_paused);
    assert!(!flags.release_paused);
    assert!(!flags.refund_paused);
}

#[test]
fn test_setting_already_unpaused_flag_is_idempotent() {
    let env = Env::default();
    let (client, _, _, _) = setup(&env, 0);

    client.set_paused(&Some(false), &None, &None, &None);
    let flags = client.get_pause_flags();
    assert!(!flags.lock_paused);
    assert_eq!(flags.paused_at, 0);
}

// ---------------------------------------------------------------------------
// § 19  All-None call preserves state
// ---------------------------------------------------------------------------

#[test]
fn test_set_paused_all_none_preserves_flags() {
    let env = Env::default();
    let (client, _, _, _) = setup(&env, 0);

    client.set_paused(&Some(true), &Some(false), &Some(true), &None);
    client.set_paused(&None, &None, &None, &None);

    let flags = client.get_pause_flags();
    assert!(flags.lock_paused);
    assert!(!flags.release_paused);
    assert!(flags.refund_paused);
}

// ---------------------------------------------------------------------------
// § 20  Batch operations with multiple items under mixed flags
// ---------------------------------------------------------------------------

#[test]
fn test_batch_lock_multiple_items_succeeds_when_unpaused() {
    let env = Env::default();
    let (client, _, depositor, _) = setup(&env, 5_000);

    let deadline = env.ledger().timestamp() + 1_000;
    let items = soroban_sdk::vec![
        &env,
        LockFundsItem {
            bounty_id: 20,
            depositor: depositor.clone(),
            amount: 100,
            deadline,
        },
        LockFundsItem {
            bounty_id: 21,
            depositor: depositor.clone(),
            amount: 200,
            deadline,
        },
        LockFundsItem {
            bounty_id: 22,
            depositor: depositor.clone(),
            amount: 300,
            deadline,
        }
    ];
    let count = client.batch_lock_funds(&items);
    assert_eq!(count, 3);
}

#[test]
fn test_batch_lock_blocked_even_with_only_lock_paused() {
    let env = Env::default();
    let (client, _, depositor, _) = setup(&env, 5_000);

    // Only lock is paused; release and refund are open
    client.set_paused(&Some(true), &Some(false), &Some(false), &None);

    let deadline = env.ledger().timestamp() + 1_000;
    let items = soroban_sdk::vec![
        &env,
        LockFundsItem {
            bounty_id: 30,
            depositor: depositor.clone(),
            amount: 100,
            deadline,
        }
    ];
    assert!(client.try_batch_lock_funds(&items).is_err());
}

#[test]
fn test_batch_release_multiple_items_succeeds_when_unpaused() {
    let env = Env::default();
    let (client, _, depositor, token) = setup(&env, 5_000);

    lock_bounty(&client, &env, &depositor, 40, 100);
    lock_bounty(&client, &env, &depositor, 41, 200);

    let c1 = Address::generate(&env);
    let c2 = Address::generate(&env);
    let items = soroban_sdk::vec![
        &env,
        ReleaseFundsItem {
            bounty_id: 40,
            contributor: c1.clone(),
        },
        ReleaseFundsItem {
            bounty_id: 41,
            contributor: c2.clone(),
        }
    ];
    let count = client.batch_release_funds(&items);
    assert_eq!(count, 2);
    assert_eq!(token.balance(&c1), 100);
    assert_eq!(token.balance(&c2), 200);
}

// ---------------------------------------------------------------------------
// § 21  Pause does not affect cancel_pending_claim (admin-only, no pause gate)
// ---------------------------------------------------------------------------

#[test]
fn test_cancel_pending_claim_unaffected_by_all_paused() {
    let env = Env::default();
    let (client, _, depositor, _) = setup(&env, 1_000);

    lock_bounty(&client, &env, &depositor, 1, 500);
    client.set_claim_window(&3600);

    let contributor = Address::generate(&env);
    client.authorize_claim(&1, &contributor, &DisputeReason::Other);

    // Pause everything
    client.set_paused(&Some(true), &Some(true), &Some(true), &None);

    // cancel_pending_claim is admin-only and not gated by pause flags
    client.cancel_pending_claim(&1, &DisputeOutcome::CancelledByAdmin);

    // Claim record should be gone
    assert!(client.try_get_pending_claim(&1).is_err());
}

// ---------------------------------------------------------------------------
// § 22  Pause reason preserved across partial flag changes
// ---------------------------------------------------------------------------

#[test]
fn test_reason_preserved_when_adding_second_flag_without_reason() {
    let env = Env::default();
    let (client, _, _, _) = setup(&env, 0);

    let reason = soroban_sdk::String::from_str(&env, "audit in progress");
    client.set_paused(&Some(true), &None, &None, &Some(reason));

    // Add release pause without providing a new reason
    client.set_paused(&None, &Some(true), &None, &None);

    let flags = client.get_pause_flags();
    assert!(flags.lock_paused);
    assert!(flags.release_paused);
    // Original reason should still be present
    assert!(flags.pause_reason.is_some());
}

#[test]
fn test_reason_overwritten_when_new_reason_provided() {
    let env = Env::default();
    let (client, _, _, _) = setup(&env, 0);

    let reason1 = soroban_sdk::String::from_str(&env, "first reason");
    client.set_paused(&Some(true), &None, &None, &Some(reason1));

    let reason2 = soroban_sdk::String::from_str(&env, "updated reason");
    client.set_paused(&None, &Some(true), &None, &Some(reason2));

    let flags = client.get_pause_flags();
    assert!(flags.pause_reason.is_some());
}

// ---------------------------------------------------------------------------
// § 23  get_pause_flags returns defaults before any set_paused call
// ---------------------------------------------------------------------------

#[test]
fn test_get_pause_flags_returns_defaults_on_fresh_contract() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token_client, _) = create_token(&env, &token_admin);
    let (client, _) = create_escrow(&env);
    client.init(&admin, &token_client.address);

    let flags = client.get_pause_flags();
    assert!(!flags.lock_paused);
    assert!(!flags.release_paused);
    assert!(!flags.refund_paused);
    assert!(flags.pause_reason.is_none());
    assert_eq!(flags.paused_at, 0);
}

// ---------------------------------------------------------------------------
// § 24  Simultaneous set of all three flags in one call
// ---------------------------------------------------------------------------

#[test]
fn test_set_all_three_flags_at_once() {
    let env = Env::default();
    let (client, _, _, _) = setup(&env, 0);

    client.set_paused(&Some(true), &Some(true), &Some(true), &None);
    let flags = client.get_pause_flags();
    assert!(flags.lock_paused);
    assert!(flags.release_paused);
    assert!(flags.refund_paused);
}

#[test]
fn test_unset_all_three_flags_at_once() {
    let env = Env::default();
    let (client, _, _, _) = setup(&env, 0);

    client.set_paused(&Some(true), &Some(true), &Some(true), &None);
    client.set_paused(&Some(false), &Some(false), &Some(false), &None);

    let flags = client.get_pause_flags();
    assert!(!flags.lock_paused);
    assert!(!flags.release_paused);
    assert!(!flags.refund_paused);
    assert_eq!(flags.paused_at, 0);
}

// ---------------------------------------------------------------------------
// § 25  Refund with approval also respects refund_paused
// ---------------------------------------------------------------------------

#[test]
fn test_approved_refund_blocked_when_refund_paused() {
    let env = Env::default();
    let (client, _, depositor, _) = setup(&env, 1_000);

    lock_bounty(&client, &env, &depositor, 1, 500);

    // Admin approves an early refund
    client.approve_refund(&1, &250, &depositor, &RefundMode::Partial);

    // Pause refund — even approved refunds should be blocked
    client.set_paused(&None, &None, &Some(true), &None);
    let result = client.try_refund(&1);
    assert!(result.is_err());
}

#[test]
fn test_approved_refund_succeeds_when_only_lock_paused() {
    let env = Env::default();
    let (client, _, depositor, token) = setup(&env, 1_000);

    lock_bounty(&client, &env, &depositor, 1, 500);
    client.approve_refund(&1, &200, &depositor, &RefundMode::Partial);

    // Only lock is paused — refund should still work
    client.set_paused(&Some(true), &None, &None, &None);

    let before = token.balance(&depositor);
    client.refund(&1);
    assert_eq!(token.balance(&depositor), before + 200);
}
