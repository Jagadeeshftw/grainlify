// ============================================================
// FILE: contracts/bounty_escrow/contracts/escrow/src/test_deterministic_event_ordering.rs
//
// Golden-fixture tests for issue #1748: "Add deterministic event
// ordering tests for multi-operation transactions".
//
// ## Documented event ordering (this file is the source of truth)
//
// | Operation                          | Order                                                          |
// |-------------------------------------|-----------------------------------------------------------------|
// | `batch_lock_funds`                  | per-item `FundsLocked` (ascending `bounty_id`), then one `BatchFundsLocked` last |
// | `batch_release_funds`               | per-item `FundsReleased` (ascending `bounty_id`), then one `BatchFundsReleased` last |
// | `lock_funds` (fee enabled)          | `FeeCollected`, `FeeRoutingInvariantChecked`, then `FundsLocked` |
// | `release_funds` (fee enabled)       | `FeeCollected`, `FeeRoutingInvariantChecked`, then `FundsReleased` |
// | `refund` (admin-approved)           | `RefundApprovalConsumed`, then `FundsRefunded`                 |
//
// Batch items are sorted by ascending `bounty_id` before processing
// regardless of input order (see `order_batch_lock_items` /
// `order_batch_release_items` in `lib.rs`), so the tests below
// deliberately submit batches out of order to prove the contract
// — not the caller — owns the final ordering.
//
// ## Failure-path guarantee
//
// Every validation failure in these flows returns before any event
// is published (see the early `reentrancy_guard::release` + `return
// Err(..)` branches in `lib.rs`). This file asserts that directly:
// a batch/refund call that fails leaves the event log exactly as it
// was before the call — no partial or misleading "success" events.
// ============================================================

#![cfg(test)]

use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events, Ledger, LedgerInfo},
    token, vec, Address, Env, IntoVal, Symbol, Vec,
};

use crate::{
    BountyEscrowContract, BountyEscrowContractClient, Error, LockFundsItem, RefundMode,
    ReleaseFundsItem,
};

const AMOUNT: i128 = 1_000;
const DEADLINE_OFFSET: u64 = 3_600;

struct Ctx<'a> {
    env: Env,
    client: BountyEscrowContractClient<'a>,
    contract_id: Address,
    token_id: Address,
}

fn setup() -> Ctx<'static> {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000_000);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);
    client.init(&admin, &token_id);

    Ctx {
        env,
        client,
        contract_id,
        token_id,
    }
}

fn mint(ctx: &Ctx, recipient: &Address, amount: i128) {
    token::StellarAssetClient::new(&ctx.env, &ctx.token_id).mint(recipient, &amount);
}

fn lock_item(ctx: &Ctx, bounty_id: u64, depositor: Address, amount: i128) -> LockFundsItem {
    LockFundsItem {
        bounty_id,
        depositor,
        amount,
        deadline: ctx.env.ledger().timestamp() + DEADLINE_OFFSET,
    }
}

/// Enable a flat percentage fee (10%) on both lock and release, routed to a
/// single recipient (no treasury split) — the simplest path that still
/// exercises `FeeCollected` + `FeeRoutingInvariantChecked`.
fn enable_flat_fee(ctx: &Ctx, fee_recipient: &Address) {
    ctx.client.update_fee_config(
        &Some(1_000i128), // lock_fee_rate: 10% (bps out of 10_000)
        &Some(1_000i128), // release_fee_rate: 10%
        &Some(0i128),     // lock_fixed_fee
        &Some(0i128),     // release_fixed_fee
        &Some(fee_recipient.clone()),
        &Some(true), // fee_enabled
    );
}

/// The first-topic `Symbol` of every event published by our contract, in
/// emission order, restricted to symbols in `wanted`. Events published by
/// other contracts (e.g. token transfer events from the SAC) are filtered
/// out by contract address; events whose first topic isn't in `wanted` are
/// dropped so each test only has to reason about the handful of event
/// types it cares about.
fn matching_event_symbols(ctx: &Ctx, wanted: &[Symbol]) -> Vec<Symbol> {
    let env = &ctx.env;
    let mut out: Vec<Symbol> = Vec::new(env);
    for event in env.events().all().iter() {
        if event.0 != ctx.contract_id {
            continue;
        }
        let topics = event.1;
        if topics.is_empty() {
            continue;
        }
        let first: Symbol = topics.get(0).unwrap().into_val(env);
        if wanted.contains(&first) {
            out.push_back(first);
        }
    }
    out
}

/// For every event published by our contract whose first topic equals
/// `target` and whose second topic is present, return that second topic
/// decoded as a `u64` (the `bounty_id`), in emission order. Used to assert
/// that batch operations emit per-item events in ascending `bounty_id`
/// order regardless of the order items were submitted in.
fn bounty_ids_for_topic(ctx: &Ctx, target: Symbol) -> Vec<u64> {
    let env = &ctx.env;
    let mut out: Vec<u64> = Vec::new(env);
    for event in env.events().all().iter() {
        if event.0 != ctx.contract_id {
            continue;
        }
        let topics = event.1;
        if topics.len() < 2 {
            continue;
        }
        let first: Symbol = topics.get(0).unwrap().into_val(env);
        if first != target {
            continue;
        }
        let bounty_id: u64 = topics.get(1).unwrap().into_val(env);
        out.push_back(bounty_id);
    }
    out
}

// ===========================================================================
// batch_lock_funds — event ordering
// ===========================================================================

/// Items submitted out of order (3, 1, 2) must still emit per-item
/// `FundsLocked` events in ascending `bounty_id` order, followed by exactly
/// one `BatchFundsLocked` aggregate event last.
#[test]
fn test_batch_lock_events_ordered_by_bounty_id_then_aggregate_last() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT * 3);

    let items = vec![
        &ctx.env,
        lock_item(&ctx, 3, depositor.clone(), AMOUNT),
        lock_item(&ctx, 1, depositor.clone(), AMOUNT),
        lock_item(&ctx, 2, depositor.clone(), AMOUNT),
    ];

    assert_eq!(ctx.client.batch_lock_funds(&items), 3);

    // Aggregate ordering: 3 FundsLocked events then 1 BatchFundsLocked, last.
    let wanted = [symbol_short!("f_lock"), symbol_short!("b_lock")];
    let seq = matching_event_symbols(&ctx, &wanted);
    assert_eq!(seq.len(), 4, "expected 3 FundsLocked + 1 BatchFundsLocked");
    assert_eq!(seq.get(0).unwrap(), symbol_short!("f_lock"));
    assert_eq!(seq.get(1).unwrap(), symbol_short!("f_lock"));
    assert_eq!(seq.get(2).unwrap(), symbol_short!("f_lock"));
    assert_eq!(seq.get(3).unwrap(), symbol_short!("b_lock"));

    // Per-item ordering: bounty_id 1, 2, 3 — ascending, not submission order (3, 1, 2).
    let ids = bounty_ids_for_topic(&ctx, symbol_short!("f_lock"));
    assert_eq!(ids.len(), 3);
    assert_eq!(ids.get(0).unwrap(), 1);
    assert_eq!(ids.get(1).unwrap(), 2);
    assert_eq!(ids.get(2).unwrap(), 3);
}

/// A batch that fails validation (duplicate `bounty_id`) must not emit a
/// single `FundsLocked` or `BatchFundsLocked` event — no misleading
/// partial-success events on a reverted transaction.
#[test]
fn test_batch_lock_failure_emits_no_funds_locked_events() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT * 3);

    let items = vec![
        &ctx.env,
        lock_item(&ctx, 1, depositor.clone(), AMOUNT),
        lock_item(&ctx, 2, depositor.clone(), AMOUNT),
        lock_item(&ctx, 1, depositor.clone(), AMOUNT), // duplicate bounty_id
    ];

    assert_eq!(
        ctx.client
            .try_batch_lock_funds(&items)
            .unwrap_err()
            .unwrap(),
        Error::DuplicateBountyId
    );

    let wanted = [symbol_short!("f_lock"), symbol_short!("b_lock")];
    let seq = matching_event_symbols(&ctx, &wanted);
    assert_eq!(
        seq.len(),
        0,
        "a reverted batch_lock_funds call must not emit any FundsLocked/BatchFundsLocked event"
    );
}

// ===========================================================================
// batch_release_funds — event ordering
// ===========================================================================

/// Items submitted out of order (3, 1, 2) must still emit per-item
/// `FundsReleased` events in ascending `bounty_id` order, followed by
/// exactly one `BatchFundsReleased` aggregate event last.
#[test]
fn test_batch_release_events_ordered_by_bounty_id_then_aggregate_last() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    let contributor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT * 3);

    let lock_items = vec![
        &ctx.env,
        lock_item(&ctx, 1, depositor.clone(), AMOUNT),
        lock_item(&ctx, 2, depositor.clone(), AMOUNT),
        lock_item(&ctx, 3, depositor.clone(), AMOUNT),
    ];
    ctx.client.batch_lock_funds(&lock_items);

    let release_items = vec![
        &ctx.env,
        ReleaseFundsItem {
            bounty_id: 3,
            contributor: contributor.clone(),
        },
        ReleaseFundsItem {
            bounty_id: 1,
            contributor: contributor.clone(),
        },
        ReleaseFundsItem {
            bounty_id: 2,
            contributor: contributor.clone(),
        },
    ];

    assert_eq!(ctx.client.batch_release_funds(&release_items), 3);

    let wanted = [symbol_short!("f_rel"), symbol_short!("b_rel")];
    let seq = matching_event_symbols(&ctx, &wanted);
    assert_eq!(
        seq.len(),
        4,
        "expected 3 FundsReleased + 1 BatchFundsReleased"
    );
    assert_eq!(seq.get(0).unwrap(), symbol_short!("f_rel"));
    assert_eq!(seq.get(1).unwrap(), symbol_short!("f_rel"));
    assert_eq!(seq.get(2).unwrap(), symbol_short!("f_rel"));
    assert_eq!(seq.get(3).unwrap(), symbol_short!("b_rel"));

    let ids = bounty_ids_for_topic(&ctx, symbol_short!("f_rel"));
    assert_eq!(ids.len(), 3);
    assert_eq!(ids.get(0).unwrap(), 1);
    assert_eq!(ids.get(1).unwrap(), 2);
    assert_eq!(ids.get(2).unwrap(), 3);
}

/// A batch release that fails validation (one bounty not found) must not
/// emit any `FundsReleased` or `BatchFundsReleased` event, even though an
/// earlier item in the batch would have succeeded on its own.
#[test]
fn test_batch_release_failure_emits_no_funds_released_events() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    let contributor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT);

    ctx.client.batch_lock_funds(&vec![
        &ctx.env,
        lock_item(&ctx, 1, depositor.clone(), AMOUNT),
    ]);

    let release_items = vec![
        &ctx.env,
        ReleaseFundsItem {
            bounty_id: 1,
            contributor: contributor.clone(),
        },
        ReleaseFundsItem {
            bounty_id: 999, // does not exist
            contributor: contributor.clone(),
        },
    ];

    assert_eq!(
        ctx.client
            .try_batch_release_funds(&release_items)
            .unwrap_err()
            .unwrap(),
        Error::BountyNotFound
    );

    let wanted = [symbol_short!("f_rel"), symbol_short!("b_rel")];
    let seq = matching_event_symbols(&ctx, &wanted);
    assert_eq!(
        seq.len(),
        0,
        "a reverted batch_release_funds call must not emit any FundsReleased/BatchFundsReleased event"
    );
}

// ===========================================================================
// Fee routing — event ordering relative to the funds event
// ===========================================================================

/// When a lock fee is configured, `FeeCollected` and
/// `FeeRoutingInvariantChecked` must both be emitted before `FundsLocked`.
#[test]
fn test_lock_fee_events_precede_funds_locked_event() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    let fee_recipient = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT);
    enable_flat_fee(&ctx, &fee_recipient);

    let deadline = ctx.env.ledger().timestamp() + DEADLINE_OFFSET;
    ctx.client.lock_funds(&depositor, &1u64, &AMOUNT, &deadline);

    let wanted = [
        symbol_short!("fee"),
        symbol_short!("fee_inv"),
        symbol_short!("f_lock"),
    ];
    let seq = matching_event_symbols(&ctx, &wanted);

    assert_eq!(seq.len(), 3);
    assert_eq!(seq.get(0).unwrap(), symbol_short!("fee"));
    assert_eq!(seq.get(1).unwrap(), symbol_short!("fee_inv"));
    assert_eq!(seq.get(2).unwrap(), symbol_short!("f_lock"));
}

/// When a release fee is configured, `FeeCollected` and
/// `FeeRoutingInvariantChecked` must both be emitted before `FundsReleased`.
#[test]
fn test_release_fee_events_precede_funds_released_event() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    let contributor = Address::generate(&ctx.env);
    let fee_recipient = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT);

    // Lock first, with fees disabled, so the lock itself doesn't pollute
    // the event sequence we're about to inspect for the release call.
    let deadline = ctx.env.ledger().timestamp() + DEADLINE_OFFSET;
    ctx.client.lock_funds(&depositor, &1u64, &AMOUNT, &deadline);

    enable_flat_fee(&ctx, &fee_recipient);

    ctx.client.release_funds(&1u64, &contributor);

    let wanted = [
        symbol_short!("fee"),
        symbol_short!("fee_inv"),
        symbol_short!("f_rel"),
    ];
    let seq = matching_event_symbols(&ctx, &wanted);

    assert_eq!(seq.len(), 3);
    assert_eq!(seq.get(0).unwrap(), symbol_short!("fee"));
    assert_eq!(seq.get(1).unwrap(), symbol_short!("fee_inv"));
    assert_eq!(seq.get(2).unwrap(), symbol_short!("f_rel"));
}

// ===========================================================================
// Refund ("rollback") — event ordering and failure-path silence
// ===========================================================================

/// An admin-approved refund must emit `RefundApprovalConsumed` before
/// `FundsRefunded`.
#[test]
fn test_refund_approval_consumed_event_precedes_funds_refunded_event() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT);

    let deadline = ctx.env.ledger().timestamp() + DEADLINE_OFFSET;
    ctx.client.lock_funds(&depositor, &1u64, &AMOUNT, &deadline);

    // Early admin-approved full refund, before the deadline has passed.
    ctx.client
        .approve_refund(&1u64, &AMOUNT, &depositor, &RefundMode::Full);
    ctx.client.refund(&1u64);

    let wanted = [symbol_short!("r_apcns"), symbol_short!("f_ref")];
    let seq = matching_event_symbols(&ctx, &wanted);

    assert_eq!(seq.len(), 2);
    assert_eq!(seq.get(0).unwrap(), symbol_short!("r_apcns"));
    assert_eq!(seq.get(1).unwrap(), symbol_short!("f_ref"));
}

/// A refund attempted before the deadline and with no admin approval must
/// fail with `DeadlineNotPassed` and must not emit `FundsRefunded` (or any
/// other refund-related event) — the failure path must stay silent.
#[test]
fn test_refund_failure_before_deadline_emits_no_events() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT);

    let deadline = ctx.env.ledger().timestamp() + DEADLINE_OFFSET;
    ctx.client.lock_funds(&depositor, &1u64, &AMOUNT, &deadline);

    // Still well before the deadline, and no approval has been set.
    ctx.env.ledger().set(LedgerInfo {
        timestamp: ctx.env.ledger().timestamp() + 10,
        ..ctx.env.ledger().get()
    });

    assert_eq!(
        ctx.client.try_refund(&1u64).unwrap_err().unwrap(),
        Error::DeadlineNotPassed
    );

    let wanted = [
        symbol_short!("r_apcns"),
        symbol_short!("f_ref"),
        symbol_short!("r_appr"),
    ];
    let seq = matching_event_symbols(&ctx, &wanted);
    assert_eq!(
        seq.len(),
        0,
        "a reverted refund call must not emit any refund-related event"
    );
}
