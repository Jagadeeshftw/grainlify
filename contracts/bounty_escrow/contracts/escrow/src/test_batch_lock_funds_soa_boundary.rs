// ============================================================================
// Boundary tests for `batch_lock_funds_soa` (issue #1980)
// ============================================================================
//
// This module pins the boundary behaviour of `batch_lock_funds_soa`, testing
// the extremes of its numeric and time-valued arguments:
//
// - Batch size: 0, 1, MAX_BATCH_SIZE (20), MAX_BATCH_SIZE + 1 (21), and the
//   configurable lock cap (N succeeds, N + 1 is refused).
// - Amount: 0, 1, i128::MAX, and negatives (-1, i128::MIN).
// - Deadline: 0, 1, u64::MAX.
// - Bounty ID: 0, 1, u64::MAX.
//
// "Maximum + 1" is not constructible for i128 / u64 arguments (the type is
// the bound), so the only enforceable upper bound is the batch size.
//
// Every refusal must return a typed error, never panic, and must leave no
// state behind.

#![cfg(test)]

use soroban_sdk::{
    testutils::Address as _,
    token, vec, Address, Env, Vec,
};

use crate::{BountyEscrowContract, BountyEscrowContractClient, Error};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum batch size enforced by the contract.
const MAX_BATCH: u32 = 20;
/// Default per-bounty lock amount.
const AMOUNT: i128 = 500;
/// Default deadline offset in seconds (1 hour ahead).
const DEADLINE_OFFSET: u64 = 3_600;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

struct Ctx<'a> {
    env: Env,
    client: BountyEscrowContractClient<'a>,
    _token_id: Address,
    _token_admin: Address,
}

fn setup() -> Ctx<'static> {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = sac.address();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);
    client.init(&admin, &token_id);

    Ctx {
        env,
        client,
        _token_id: token_id,
        _token_admin: token_admin,
    }
}

fn mint(ctx: &Ctx, recipient: &Address, amount: i128) {
    token::StellarAssetClient::new(&ctx.env, &ctx._token_id).mint(recipient, &amount);
}

/// Builds a valid SoA batch of `n` distinct bounties for one funded depositor.
fn soa_batch(ctx: &Ctx, n: u32) -> (Vec<u64>, Vec<Address>, Vec<i128>, Vec<u64>) {
    let depositor = Address::generate(&ctx.env);
    mint(ctx, &depositor, AMOUNT * (n as i128 + 1));
    let deadline = ctx.env.ledger().timestamp() + DEADLINE_OFFSET;

    let mut ids: Vec<u64> = Vec::new(&ctx.env);
    let mut deps: Vec<Address> = Vec::new(&ctx.env);
    let mut amts: Vec<i128> = Vec::new(&ctx.env);
    let mut dls: Vec<u64> = Vec::new(&ctx.env);
    for i in 1..=n as u64 {
        ids.push_back(i);
        deps.push_back(depositor.clone());
        amts.push_back(AMOUNT);
        dls.push_back(deadline);
    }
    (ids, deps, amts, dls)
}

// ===========================================================================
// Batch size boundaries
// ===========================================================================

/// Empty batch (size 0) must return `InvalidBatchSize`.
#[test]
fn batch_lock_funds_soa_empty_batch_fails() {
    let ctx = setup();
    let _depositor = Address::generate(&ctx.env);

    let bounty_ids: Vec<u64> = vec![&ctx.env];
    let depositors: Vec<Address> = vec![&ctx.env];
    let amounts: Vec<i128> = vec![&ctx.env];
    let deadlines: Vec<u64> = vec![&ctx.env];

    let err = ctx
        .client
        .try_batch_lock_funds_soa(&bounty_ids, &depositors, &amounts, &deadlines)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::InvalidBatchSize);
}

/// Single-item batch (size 1, minimum valid) must succeed.
#[test]
fn batch_lock_funds_soa_single_item_succeeds() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT);

    let bounty_ids: Vec<u64> = vec![&ctx.env, 1];
    let depositors: Vec<Address> = vec![&ctx.env, depositor.clone()];
    let amounts: Vec<i128> = vec![&ctx.env, AMOUNT];
    let deadline = ctx.env.ledger().timestamp() + DEADLINE_OFFSET;
    let deadlines: Vec<u64> = vec![&ctx.env, deadline];

    let settled = ctx
        .client
        .batch_lock_funds_soa(&bounty_ids, &depositors, &amounts, &deadlines);
    assert_eq!(settled, 1);

    // Verify the bounty was actually locked
    let escrow = ctx.client.get_escrow(&1);
    assert_eq!(escrow.amount, AMOUNT);
}

/// Batch of exactly `MAX_BATCH` items (20) must succeed.
#[test]
fn batch_lock_funds_soa_exactly_max_batch_size_succeeds() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT * MAX_BATCH as i128);

    let mut bounty_ids: Vec<u64> = Vec::new(&ctx.env);
    let mut depositors: Vec<Address> = Vec::new(&ctx.env);
    let mut amounts: Vec<i128> = Vec::new(&ctx.env);
    let deadline = ctx.env.ledger().timestamp() + DEADLINE_OFFSET;
    let mut deadlines: Vec<u64> = Vec::new(&ctx.env);

    for i in 1..=MAX_BATCH as u64 {
        bounty_ids.push_back(i);
        depositors.push_back(depositor.clone());
        amounts.push_back(AMOUNT);
        deadlines.push_back(deadline);
    }

    let settled = ctx
        .client
        .batch_lock_funds_soa(&bounty_ids, &depositors, &amounts, &deadlines);
    assert_eq!(settled, MAX_BATCH);

    // Verify all bounties were locked
    for i in 1..=MAX_BATCH as u64 {
        let escrow = ctx.client.get_escrow(&i);
        assert_eq!(escrow.amount, AMOUNT);
    }
}

/// Batch of `MAX_BATCH + 1` items (21) must return `InvalidBatchSize`.
#[test]
fn batch_lock_funds_soa_exceeds_max_batch_size_fails() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT * (MAX_BATCH as i128 + 1));

    let mut bounty_ids: Vec<u64> = Vec::new(&ctx.env);
    let mut depositors: Vec<Address> = Vec::new(&ctx.env);
    let mut amounts: Vec<i128> = Vec::new(&ctx.env);
    let deadline = ctx.env.ledger().timestamp() + DEADLINE_OFFSET;
    let mut deadlines: Vec<u64> = Vec::new(&ctx.env);

    for i in 1..=(MAX_BATCH + 1) as u64 {
        bounty_ids.push_back(i);
        depositors.push_back(depositor.clone());
        amounts.push_back(AMOUNT);
        deadlines.push_back(deadline);
    }

    let err = ctx
        .client
        .try_batch_lock_funds_soa(&bounty_ids, &depositors, &amounts, &deadlines)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::InvalidBatchSize);
}

/// A batch of exactly the configured lock cap succeeds.
#[test]
fn batch_lock_funds_soa_at_configured_cap_succeeds() {
    let ctx = setup();
    ctx.client.set_batch_size_caps(&5, &MAX_BATCH);

    let (ids, deps, amts, dls) = soa_batch(&ctx, 5);
    let settled = ctx.client.batch_lock_funds_soa(&ids, &deps, &amts, &dls);
    assert_eq!(settled, 5);
}

/// One past the configured lock cap is refused with a typed error, and
/// nothing is locked.
#[test]
fn batch_lock_funds_soa_one_past_configured_cap_fails() {
    let ctx = setup();
    ctx.client.set_batch_size_caps(&5, &MAX_BATCH);

    let (ids, deps, amts, dls) = soa_batch(&ctx, 6);
    let balance_before = ctx.client.get_balance();

    let err = ctx
        .client
        .try_batch_lock_funds_soa(&ids, &deps, &amts, &dls)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::InvalidBatchSize);
    assert_eq!(ctx.client.get_balance(), balance_before);
}

// ===========================================================================
// Amount boundaries
// ===========================================================================

/// Zero amount must return `InvalidAmount`.
#[test]
fn batch_lock_funds_soa_zero_amount_fails() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT);

    let bounty_ids: Vec<u64> = vec![&ctx.env, 1];
    let depositors: Vec<Address> = vec![&ctx.env, depositor.clone()];
    let amounts: Vec<i128> = vec![&ctx.env, 0];
    let deadline = ctx.env.ledger().timestamp() + DEADLINE_OFFSET;
    let deadlines: Vec<u64> = vec![&ctx.env, deadline];

    let err = ctx
        .client
        .try_batch_lock_funds_soa(&bounty_ids, &depositors, &amounts, &deadlines)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::InvalidAmount);
}

/// Negative amounts (one below zero, and the type minimum) must return
/// `InvalidAmount`.
#[test]
fn batch_lock_funds_soa_negative_amount_fails() {
    for bad in [-1_i128, i128::MIN] {
        let ctx = setup();
        let depositor = Address::generate(&ctx.env);
        mint(&ctx, &depositor, AMOUNT);

        let bounty_ids: Vec<u64> = vec![&ctx.env, 1];
        let depositors: Vec<Address> = vec![&ctx.env, depositor.clone()];
        let amounts: Vec<i128> = vec![&ctx.env, bad];
        let deadline = ctx.env.ledger().timestamp() + DEADLINE_OFFSET;
        let deadlines: Vec<u64> = vec![&ctx.env, deadline];

        let err = ctx
            .client
            .try_batch_lock_funds_soa(&bounty_ids, &depositors, &amounts, &deadlines)
            .unwrap_err()
            .unwrap();
        assert_eq!(err, Error::InvalidAmount);
    }
}

/// A refused batch must leave no state behind: a valid first item followed by
/// a zero-amount item is rejected as a whole, so no funds move.
#[test]
fn batch_lock_funds_soa_refusal_leaves_balances_unchanged() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT * 2);

    let token = token::Client::new(&ctx.env, &ctx._token_id);
    let contract_before = ctx.client.get_balance();
    let depositor_before = token.balance(&depositor);

    let bounty_ids: Vec<u64> = vec![&ctx.env, 1, 2];
    let depositors: Vec<Address> = vec![&ctx.env, depositor.clone(), depositor.clone()];
    let amounts: Vec<i128> = vec![&ctx.env, AMOUNT, 0];
    let deadline = ctx.env.ledger().timestamp() + DEADLINE_OFFSET;
    let deadlines: Vec<u64> = vec![&ctx.env, deadline, deadline];

    let err = ctx
        .client
        .try_batch_lock_funds_soa(&bounty_ids, &depositors, &amounts, &deadlines)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::InvalidAmount);

    assert_eq!(ctx.client.get_balance(), contract_before);
    assert_eq!(token.balance(&depositor), depositor_before);
}

/// Amount of 1 (minimum positive) must succeed.
#[test]
fn batch_lock_funds_soa_amount_one_succeeds() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, 10);

    let bounty_ids: Vec<u64> = vec![&ctx.env, 1];
    let depositors: Vec<Address> = vec![&ctx.env, depositor.clone()];
    let amounts: Vec<i128> = vec![&ctx.env, 1];
    let deadline = ctx.env.ledger().timestamp() + DEADLINE_OFFSET;
    let deadlines: Vec<u64> = vec![&ctx.env, deadline];

    let settled = ctx
        .client
        .batch_lock_funds_soa(&bounty_ids, &depositors, &amounts, &deadlines);
    assert_eq!(settled, 1);

    let escrow = ctx.client.get_escrow(&1);
    assert_eq!(escrow.amount, 1);
}

/// Maximum valid i128 amount must succeed.
#[test]
fn batch_lock_funds_soa_max_i128_amount_succeeds() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    let max_amount = i128::MAX;
    mint(&ctx, &depositor, max_amount);

    let bounty_ids: Vec<u64> = vec![&ctx.env, 1];
    let depositors: Vec<Address> = vec![&ctx.env, depositor.clone()];
    let amounts: Vec<i128> = vec![&ctx.env, max_amount];
    let deadline = ctx.env.ledger().timestamp() + DEADLINE_OFFSET;
    let deadlines: Vec<u64> = vec![&ctx.env, deadline];

    let settled = ctx
        .client
        .batch_lock_funds_soa(&bounty_ids, &depositors, &amounts, &deadlines);
    assert_eq!(settled, 1);

    let escrow = ctx.client.get_escrow(&1);
    assert_eq!(escrow.amount, max_amount);
}

// ===========================================================================
// Deadline boundaries
// ===========================================================================

/// Deadline of 0 (current ledger time) must succeed.
#[test]
fn batch_lock_funds_soa_deadline_zero_succeeds() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT);

    let bounty_ids: Vec<u64> = vec![&ctx.env, 1];
    let depositors: Vec<Address> = vec![&ctx.env, depositor.clone()];
    let amounts: Vec<i128> = vec![&ctx.env, AMOUNT];
    let deadlines: Vec<u64> = vec![&ctx.env, 0];

    let settled = ctx
        .client
        .batch_lock_funds_soa(&bounty_ids, &depositors, &amounts, &deadlines);
    assert_eq!(settled, 1);

    let escrow = ctx.client.get_escrow(&1);
    assert_eq!(escrow.amount, AMOUNT);
    assert_eq!(escrow.deadline, 0);
}

/// Deadline of 1 (minimum positive) must succeed.
#[test]
fn batch_lock_funds_soa_deadline_one_succeeds() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT);

    let bounty_ids: Vec<u64> = vec![&ctx.env, 1];
    let depositors: Vec<Address> = vec![&ctx.env, depositor.clone()];
    let amounts: Vec<i128> = vec![&ctx.env, AMOUNT];
    let deadlines: Vec<u64> = vec![&ctx.env, 1];

    let settled = ctx
        .client
        .batch_lock_funds_soa(&bounty_ids, &depositors, &amounts, &deadlines);
    assert_eq!(settled, 1);

    let escrow = ctx.client.get_escrow(&1);
    assert_eq!(escrow.amount, AMOUNT);
    assert_eq!(escrow.deadline, 1);
}

/// Maximum valid u64 deadline must succeed.
#[test]
fn batch_lock_funds_soa_max_u64_deadline_succeeds() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT);

    let bounty_ids: Vec<u64> = vec![&ctx.env, 1];
    let depositors: Vec<Address> = vec![&ctx.env, depositor.clone()];
    let amounts: Vec<i128> = vec![&ctx.env, AMOUNT];
    let deadlines: Vec<u64> = vec![&ctx.env, u64::MAX];

    let settled = ctx
        .client
        .batch_lock_funds_soa(&bounty_ids, &depositors, &amounts, &deadlines);
    assert_eq!(settled, 1);

    let escrow = ctx.client.get_escrow(&1);
    assert_eq!(escrow.amount, AMOUNT);
    assert_eq!(escrow.deadline, u64::MAX);
}

// ===========================================================================
// Bounty ID boundaries
// ===========================================================================

/// Bounty ID of 0 must succeed.
#[test]
fn batch_lock_funds_soa_bounty_id_zero_succeeds() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT);

    let bounty_ids: Vec<u64> = vec![&ctx.env, 0];
    let depositors: Vec<Address> = vec![&ctx.env, depositor.clone()];
    let amounts: Vec<i128> = vec![&ctx.env, AMOUNT];
    let deadline = ctx.env.ledger().timestamp() + DEADLINE_OFFSET;
    let deadlines: Vec<u64> = vec![&ctx.env, deadline];

    let settled = ctx
        .client
        .batch_lock_funds_soa(&bounty_ids, &depositors, &amounts, &deadlines);
    assert_eq!(settled, 1);

    let escrow = ctx.client.get_escrow(&0);
    assert_eq!(escrow.amount, AMOUNT);
}

/// Bounty ID of 1 (minimum positive) must succeed.
#[test]
fn batch_lock_funds_soa_bounty_id_one_succeeds() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT);

    let bounty_ids: Vec<u64> = vec![&ctx.env, 1];
    let depositors: Vec<Address> = vec![&ctx.env, depositor.clone()];
    let amounts: Vec<i128> = vec![&ctx.env, AMOUNT];
    let deadline = ctx.env.ledger().timestamp() + DEADLINE_OFFSET;
    let deadlines: Vec<u64> = vec![&ctx.env, deadline];

    let settled = ctx
        .client
        .batch_lock_funds_soa(&bounty_ids, &depositors, &amounts, &deadlines);
    assert_eq!(settled, 1);

    let escrow = ctx.client.get_escrow(&1);
    assert_eq!(escrow.amount, AMOUNT);
}

/// Maximum valid u64 bounty ID must succeed.
#[test]
fn batch_lock_funds_soa_max_u64_bounty_id_succeeds() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT);

    let bounty_ids: Vec<u64> = vec![&ctx.env, u64::MAX];
    let depositors: Vec<Address> = vec![&ctx.env, depositor.clone()];
    let amounts: Vec<i128> = vec![&ctx.env, AMOUNT];
    let deadline = ctx.env.ledger().timestamp() + DEADLINE_OFFSET;
    let deadlines: Vec<u64> = vec![&ctx.env, deadline];

    let settled = ctx
        .client
        .batch_lock_funds_soa(&bounty_ids, &depositors, &amounts, &deadlines);
    assert_eq!(settled, 1);

    let escrow = ctx.client.get_escrow(&u64::MAX);
    assert_eq!(escrow.amount, AMOUNT);
}

// ===========================================================================
// Array length mismatch
// ===========================================================================

/// Mismatched array lengths must return `BatchSizeMismatch`.
#[test]
fn batch_lock_funds_soa_array_length_mismatch_fails() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT * 2);

    let bounty_ids: Vec<u64> = vec![&ctx.env, 1, 2];
    let depositors: Vec<Address> = vec![&ctx.env, depositor.clone()];
    let amounts: Vec<i128> = vec![&ctx.env, AMOUNT, AMOUNT];
    let deadline = ctx.env.ledger().timestamp() + DEADLINE_OFFSET;
    let deadlines: Vec<u64> = vec![&ctx.env, deadline, deadline];

    let err = ctx
        .client
        .try_batch_lock_funds_soa(&bounty_ids, &depositors, &amounts, &deadlines)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::BatchSizeMismatch);
}