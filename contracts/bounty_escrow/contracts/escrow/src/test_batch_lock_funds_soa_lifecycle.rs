// ============================================================================
// Lifecycle coverage for `batch_lock_funds_soa` (issue #1981)
// ============================================================================
//
// Drives `batch_lock_funds_soa` from each `EscrowStatus` state defined by
// `EscrowStatus` (Draft, Locked, Released, Refunded, PartiallyRefunded).
// Each outcome is asserted as a typed error or value; records in terminal
// states are not mutated by refused calls.

#![cfg(test)]

use soroban_sdk::{
    testutils::Address as _,
    token, vec, Address, Env, Vec,
};

use crate::{BountyEscrowContract, BountyEscrowContractClient, Error, EscrowStatus};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const AMOUNT: i128 = 500;
const DEADLINE_OFFSET: u64 = 3_600;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

struct Ctx<'a> {
    env: Env,
    client: BountyEscrowContractClient<'a>,
    _token_id: Address,
    _token_admin: Address,
    _admin: Address,
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
        _admin: admin,
    }
}

fn mint(ctx: &Ctx, recipient: &Address, amount: i128) {
    token::StellarAssetClient::new(&ctx.env, &ctx._token_id).mint(recipient, &amount);
}

fn call_soa(
    ctx: &Ctx,
    bounty_id: u64,
    depositor: &Address,
) -> Result<u32, soroban_sdk::Error> {
    let deadline = ctx.env.ledger().timestamp() + DEADLINE_OFFSET;
    let bounty_ids: Vec<u64> = vec![&ctx.env, bounty_id];
    let depositors: Vec<Address> = vec![&ctx.env, depositor.clone()];
    let amounts: Vec<i128> = vec![&ctx.env, AMOUNT];
    let deadlines: Vec<u64> = vec![&ctx.env, deadline];
    ctx.client.try_batch_lock_funds_soa(&bounty_ids, &depositors, &amounts, &deadlines)
}

fn seed_escrow_in_state(ctx: &Ctx, bounty_id: u64, depositor: &Address, status: EscrowStatus) {
    let deadline = ctx.env.ledger().timestamp() + DEADLINE_OFFSET;
    let escrow = crate::Escrow {
        depositor: depositor.clone(),
        amount: AMOUNT,
        remaining_amount: AMOUNT,
        status,
        deadline,
        refund_history: soroban_sdk::vec![&ctx.env],
        archived: false,
        archived_at: None,
    };

    token::StellarAssetClient::new(&ctx.env, &ctx._token_id).mint(&ctx.client.address, &AMOUNT);

    ctx.env.as_contract(&ctx.client.address, || {
        ctx.env
            .storage()
            .persistent()
            .set(&crate::DataKey::Escrow(bounty_id), &escrow);
    });
}

// ============================================================================
// Tests
// ============================================================================

#[test]
fn batch_lock_funds_soa_from_draft_state_refuses_bounty_exists_and_no_mutation() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT);
    let bounty_id = 1;

    seed_escrow_in_state(&ctx, bounty_id, &depositor, EscrowStatus::Draft);

    let before = ctx.client.get_escrow(&bounty_id);
    assert_eq!(before.status, EscrowStatus::Draft);

    let err = call_soa(&ctx, bounty_id, &depositor)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::BountyExists);

    let after = ctx.client.get_escrow(&bounty_id);
    assert_eq!(after.status, EscrowStatus::Draft);
    assert_eq!(after.amount, before.amount);
    assert_eq!(after.remaining_amount, before.remaining_amount);
}

#[test]
fn batch_lock_funds_soa_from_locked_state_refuses_bounty_exists_and_no_mutation() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT);
    let bounty_id = 2;

    seed_escrow_in_state(&ctx, bounty_id, &depositor, EscrowStatus::Locked);

    let before = ctx.client.get_escrow(&bounty_id);
    assert_eq!(before.status, EscrowStatus::Locked);

    let err = call_soa(&ctx, bounty_id, &depositor)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::BountyExists);

    let after = ctx.client.get_escrow(&bounty_id);
    assert_eq!(after.status, EscrowStatus::Locked);
    assert_eq!(after.amount, before.amount);
    assert_eq!(after.remaining_amount, before.remaining_amount);
}

#[test]
fn batch_lock_funds_soa_from_released_terminal_state_refuses_bounty_exists_and_no_mutation() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT);
    let bounty_id = 3;

    seed_escrow_in_state(&ctx, bounty_id, &depositor, EscrowStatus::Released);

    let before = ctx.client.get_escrow(&bounty_id);
    assert_eq!(before.status, EscrowStatus::Released);

    let err = call_soa(&ctx, bounty_id, &depositor)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::BountyExists);

    let after = ctx.client.get_escrow(&bounty_id);
    assert_eq!(after.status, EscrowStatus::Released);
    assert_eq!(after.amount, before.amount);
    assert_eq!(after.remaining_amount, before.remaining_amount);
}

#[test]
fn batch_lock_funds_soa_from_refunded_terminal_state_refuses_bounty_exists_and_no_mutation() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT);
    let bounty_id = 4;

    seed_escrow_in_state(&ctx, bounty_id, &depositor, EscrowStatus::Refunded);

    let before = ctx.client.get_escrow(&bounty_id);
    assert_eq!(before.status, EscrowStatus::Refunded);

    let err = call_soa(&ctx, bounty_id, &depositor)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::BountyExists);

    let after = ctx.client.get_escrow(&bounty_id);
    assert_eq!(after.status, EscrowStatus::Refunded);
    assert_eq!(after.amount, before.amount);
    assert_eq!(after.remaining_amount, before.remaining_amount);
}

#[test]
fn batch_lock_funds_soa_from_partially_refunded_state_refuses_bounty_exists_and_no_mutation() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT);
    let bounty_id = 5;

    seed_escrow_in_state(&ctx, bounty_id, &depositor, EscrowStatus::PartiallyRefunded);

    let before = ctx.client.get_escrow(&bounty_id);
    assert_eq!(before.status, EscrowStatus::PartiallyRefunded);

    let err = call_soa(&ctx, bounty_id, &depositor)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::BountyExists);

    let after = ctx.client.get_escrow(&bounty_id);
    assert_eq!(after.status, EscrowStatus::PartiallyRefunded);
    assert_eq!(after.amount, before.amount);
    assert_eq!(after.remaining_amount, before.remaining_amount);
}

#[test]
fn batch_lock_funds_soa_from_empty_state_succeeds() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT);
    let bounty_id = 6;

    let settled = call_soa(&ctx, bounty_id, &depositor).unwrap();
    assert_eq!(settled, 1);
    let escrow = ctx.client.get_escrow(&bounty_id);
    assert_eq!(escrow.status, EscrowStatus::Locked);
    assert_eq!(escrow.amount, AMOUNT);
}
