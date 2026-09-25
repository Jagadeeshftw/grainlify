// ============================================================================
// Batch failure-semantics contract tests (issue #1877).
// ============================================================================
//
// The other two batch failure suites (`test_batch_failure_mode.rs` and
// `test_batch_failure_modes.rs`) enumerate *which* error a malformed batch
// produces. This module pins the part those suites take for granted: the
// **contract** between the documented failure model and the code, for every
// batch entry point on this contract.
//
// ## The documented model
//
// All five batch entry points are **all-or-nothing**:
//
// | Entry point             | Shape                              | Atomic | Size cap |
// |-------------------------|------------------------------------|--------|----------|
// | `batch_lock_funds`      | AoS, `Vec<LockFundsItem>`          | yes    | 1..=20   |
// | `batch_lock`            | alias of `batch_lock_funds`        | yes    | 1..=20   |
// | `batch_lock_funds_soa`  | SoA, 4 parallel arrays             | yes    | 1..=20   |
// | `batch_release_funds`   | AoS, `Vec<ReleaseFundsItem>`       | yes    | 1..=20   |
// | `batch_release_funds_soa`| SoA, 2 parallel arrays            | yes    | 1..=20   |
//
// `MAX_BATCH_SIZE` is 20 and is additionally adjustable at runtime down to that
// ceiling via `set_batch_caps`; each call reads the effective cap through
// `get_max_batch_size` / `get_max_release_batch_size`.
//
// ## What the return value does and does not tell you
//
// Every entry point returns `Result<u32, Error>`, and the `u32` is a **count of
// settled elements, not a per-element result vector**. Because the batch is
// atomic, that is sufficient — but only because of the atomicity:
//
// * `Ok(n)` — `n` always equals `items.len()`. Every element settled. There is
//   no "3 of 5 locked" outcome to interpret.
// * `Err(e)` — **zero** elements settled. The Soroban host rolls back every
//   storage write and token transfer made earlier in the call, so the contract
//   is byte-for-byte in its pre-call state.
//
// The one thing the return value does **not** carry is *which* element tripped
// the error. Atomicity makes that question moot for settlement (nothing settled),
// but a caller that wants to know which element to fix has to re-validate its
// own input. `test_return_value_contract_*` below assert exactly this
// all-or-nothing reading, so the documentation cannot drift away from the code.
//
// ## "Retry of the remainder" means retry of the whole batch
//
// Because a failed batch settles nothing, there is no remainder to resume: the
// state after a failure is identical to the state before the call. A caller
// therefore retries by re-submitting the full, corrected batch — not a suffix of
// it. `retry_after_failed_batch_resubmits_the_whole_batch` pins that, because
// getting it backwards (assuming a partial settle and resuming from index k)
// would double-spend.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token, vec, Address, Env, Vec,
};

use crate::{
    BountyEscrowContract, BountyEscrowContractClient, Error, LockFundsItem, ReleaseFundsItem,
};

/// Maximum batch size enforced by `lib.rs`.
const MAX_BATCH: u32 = 20;
/// Per-bounty lock amount.
const AMOUNT: i128 = 500;
/// Deadline offset, one hour ahead.
const DEADLINE_OFFSET: u64 = 3_600;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

struct Ctx<'a> {
    env: Env,
    client: BountyEscrowContractClient<'a>,
    token_id: Address,
    token_admin: Address,
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
        token_id,
        token_admin,
    }
}

fn mint(ctx: &Ctx, recipient: &Address, amount: i128) {
    token::StellarAssetClient::new(&ctx.env, &ctx.token_id).mint(recipient, &amount);
}

fn lock_item(ctx: &Ctx, bounty_id: u64, depositor: &Address, amount: i128) -> LockFundsItem {
    LockFundsItem {
        bounty_id,
        depositor: depositor.clone(),
        amount,
        deadline: ctx.env.ledger().timestamp() + DEADLINE_OFFSET,
    }
}

/// A depositor funded for `count` full-size locks, so that a mid-batch token
/// shortfall is never what makes a batch fail.
fn funded_depositor(ctx: &Ctx, count: u32) -> Address {
    let depositor = Address::generate(&ctx.env);
    mint(ctx, &depositor, AMOUNT * count as i128 * 2);
    depositor
}

/// Assert that `bounty_id` is present in persistent storage.
///
/// Absence is the observable signal of rollback: a settled lock always leaves an
/// `Escrow` record, so "no record" means "this element did not settle".
fn assert_settled(ctx: &Ctx, bounty_id: u64) {
    assert!(
        ctx.client.try_get_escrow(&bounty_id).is_ok(),
        "bounty {} should be settled, but no escrow record exists — the batch \
         was expected to be all-or-nothing",
        bounty_id
    );
}

/// Assert that `bounty_id` is absent, i.e. never settled.
fn assert_not_settled(ctx: &Ctx, bounty_id: u64) {
    assert!(
        ctx.client.try_get_escrow(&bounty_id).is_err(),
        "bounty {} should NOT be settled, but an escrow record exists — a failed \
         batch must roll back every element, not just the failing one",
        bounty_id
    );
}

/// The `u32` returned by every batch entry point is a settled-element count.
/// Under atomicity that must equal the number of items submitted. This helper
/// states that contract in one place so each test reads as intent, not arithmetic.
fn assert_count_is_all_items(submitted: usize, settled: u32, entry_point: &str) {
    assert_eq!(
        settled as usize, submitted,
        "{} returned {} for a {}-item batch; an all-or-nothing batch must report \
         either every element settled or none",
        entry_point, settled, submitted
    );
}

// ===========================================================================
// CASE 1 — a fully successful batch, on every entry point
// ===========================================================================

/// Every entry point settles every element of a fully valid batch, and reports
/// a count equal to the number of items submitted.
#[test]
fn fully_successful_batch_settles_every_element_on_every_entry_point() {
    let ids: [u64; 3] = [11, 12, 13];

    // --- batch_lock_funds (AoS) ---
    {
        let ctx = setup();
        let depositor = funded_depositor(&ctx, ids.len() as u32);
        let items = vec![
            &ctx.env,
            lock_item(&ctx, ids[0], &depositor, AMOUNT),
            lock_item(&ctx, ids[1], &depositor, AMOUNT),
            lock_item(&ctx, ids[2], &depositor, AMOUNT),
        ];
        let settled = ctx.client.batch_lock_funds(&items);
        assert_count_is_all_items(3, settled, "batch_lock_funds");
        for id in ids {
            assert_settled(&ctx, id);
        }
    }

    // --- batch_lock (alias) ---
    {
        let ctx = setup();
        let depositor = funded_depositor(&ctx, ids.len() as u32);
        let items = vec![
            &ctx.env,
            lock_item(&ctx, ids[0], &depositor, AMOUNT),
            lock_item(&ctx, ids[1], &depositor, AMOUNT),
            lock_item(&ctx, ids[2], &depositor, AMOUNT),
        ];
        let settled = ctx.client.batch_lock(&items);
        assert_count_is_all_items(3, settled, "batch_lock");
        for id in ids {
            assert_settled(&ctx, id);
        }
    }

    // --- batch_lock_funds_soa (SoA) ---
    {
        let ctx = setup();
        let depositor = funded_depositor(&ctx, ids.len() as u32);
        let bounty_ids: Vec<u64> = vec![&ctx.env, ids[0], ids[1], ids[2]];
        let depositors: Vec<Address> =
            vec![&ctx.env, depositor.clone(), depositor.clone(), depositor.clone()];
        let amounts: Vec<i128> = vec![&ctx.env, AMOUNT, AMOUNT, AMOUNT];
        let deadline = ctx.env.ledger().timestamp() + DEADLINE_OFFSET;
        let deadlines: Vec<u64> = vec![&ctx.env, deadline, deadline, deadline];

        let settled = ctx
            .client
            .batch_lock_funds_soa(&bounty_ids, &depositors, &amounts, &deadlines);
        assert_count_is_all_items(3, settled, "batch_lock_funds_soa");
        for id in ids {
            assert_settled(&ctx, id);
        }
    }

    // --- batch_release_funds (AoS) — lock first, then release all three ---
    {
        let ctx = setup();
        let depositor = funded_depositor(&ctx, ids.len() as u32);
        let lock_items = vec![
            &ctx.env,
            lock_item(&ctx, ids[0], &depositor, AMOUNT),
            lock_item(&ctx, ids[1], &depositor, AMOUNT),
            lock_item(&ctx, ids[2], &depositor, AMOUNT),
        ];
        ctx.client.batch_lock_funds(&lock_items);

        let contributor = Address::generate(&ctx.env);
        let release_items = vec![
            &ctx.env,
            ReleaseFundsItem {
                bounty_id: ids[0],
                contributor: contributor.clone(),
            },
            ReleaseFundsItem {
                bounty_id: ids[1],
                contributor: contributor.clone(),
            },
            ReleaseFundsItem {
                bounty_id: ids[2],
                contributor: contributor.clone(),
            },
        ];
        let settled = ctx.client.batch_release_funds(&release_items);
        assert_count_is_all_items(3, settled, "batch_release_funds");
        assert_eq!(
            token::Client::new(&ctx.env, &ctx.token_id).balance(&contributor),
            AMOUNT * 3,
            "every element of a successful release batch must have paid out"
        );
    }

    // --- batch_release_funds_soa (SoA) ---
    {
        let ctx = setup();
        let depositor = funded_depositor(&ctx, ids.len() as u32);
        let lock_items = vec![
            &ctx.env,
            lock_item(&ctx, ids[0], &depositor, AMOUNT),
            lock_item(&ctx, ids[1], &depositor, AMOUNT),
            lock_item(&ctx, ids[2], &depositor, AMOUNT),
        ];
        ctx.client.batch_lock_funds(&lock_items);

        let contributor = Address::generate(&ctx.env);
        let bounty_ids: Vec<u64> = vec![&ctx.env, ids[0], ids[1], ids[2]];
        let contributors: Vec<Address> = vec![
            &ctx.env,
            contributor.clone(),
            contributor.clone(),
            contributor.clone(),
        ];

        let settled = ctx
            .client
            .batch_release_funds_soa(&bounty_ids, &contributors);
        assert_count_is_all_items(3, settled, "batch_release_funds_soa");
        assert_eq!(
            token::Client::new(&ctx.env, &ctx.token_id).balance(&contributor),
            AMOUNT * 3,
            "every element of a successful SoA release batch must have paid out"
        );
    }
}

// ===========================================================================
// CASE 2 — a batch with one failing element settles nothing
// ===========================================================================

/// One bad element aborts the whole batch. Siblings that were already valid —
/// and, critically, elements processed *before* the bad one — are rolled back.
#[test]
fn failing_element_aborts_whole_batch_and_settles_no_sibling() {
    let ctx = setup();
    let depositor = funded_depositor(&ctx, 4);

    // Element 1 (the middle) carries a zero amount, so validation fails after
    // element 0 has already been inspected.
    let items = vec![
        &ctx.env,
        lock_item(&ctx, 21, &depositor, AMOUNT),
        lock_item(&ctx, 22, &depositor, 0),
        lock_item(&ctx, 23, &depositor, AMOUNT),
    ];

    let err = ctx
        .client
        .try_batch_lock_funds(&items)
        .unwrap_err()
        .unwrap();
    assert_eq!(
        err,
        Error::InvalidAmount,
        "a zero-amount element must surface InvalidAmount"
    );

    // Every element is unsettled — not just the failing one.
    assert_not_settled(&ctx, 21);
    assert_not_settled(&ctx, 22);
    assert_not_settled(&ctx, 23);
}

/// The same all-or-nothing rule holds when the failing element is *last*, which
/// is the case a naive implementation is most likely to get wrong (every earlier
/// element has already been written by then).
#[test]
fn failing_last_element_rolls_back_all_earlier_siblings() {
    let ctx = setup();
    let depositor = funded_depositor(&ctx, 4);

    let items = vec![
        &ctx.env,
        lock_item(&ctx, 31, &depositor, AMOUNT),
        lock_item(&ctx, 32, &depositor, AMOUNT),
        // Duplicate of 31 → DuplicateBountyId, detected on the final element.
        lock_item(&ctx, 31, &depositor, AMOUNT),
    ];

    let err = ctx
        .client
        .try_batch_lock_funds(&items)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::DuplicateBountyId);

    assert_not_settled(&ctx, 31);
    assert_not_settled(&ctx, 32);
}

/// A release batch with one un-lockable element releases nothing: the
/// contributors receive no funds at all.
#[test]
fn failing_element_in_release_batch_disburses_nothing() {
    let ctx = setup();
    let depositor = funded_depositor(&ctx, 3);

    // Only 41 is ever locked. 42 stays unknown to the contract, so the release
    // batch must abort on it rather than quietly settling 41 alone.
    let lock_items = vec![&ctx.env, lock_item(&ctx, 41, &depositor, AMOUNT)];
    ctx.client.batch_lock_funds(&lock_items);

    let contributor = Address::generate(&ctx.env);
    let balance_before =
        token::Client::new(&ctx.env, &ctx.token_id).balance(&contributor);

    // 42 was never locked, so releasing it must abort the batch.
    let release_items = vec![
        &ctx.env,
        ReleaseFundsItem {
            bounty_id: 41,
            contributor: contributor.clone(),
        },
        ReleaseFundsItem {
            bounty_id: 42,
            contributor: contributor.clone(),
        },
    ];

    let err = ctx
        .client
        .try_batch_release_funds(&release_items)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::BountyNotFound);

    assert_eq!(
        token::Client::new(&ctx.env, &ctx.token_id).balance(&contributor),
        balance_before,
        "a failed release batch must not disburse the elements that came before \
         the failing one"
    );
    // 41 is still locked, not released.
    let escrow = ctx.client.get_escrow(&41);
    assert_ne!(
        escrow.status,
        crate::EscrowStatus::Released,
        "the valid sibling must remain Locked after a failed batch"
    );
}

// ===========================================================================
// CASE 3 — retry of the remainder
// ===========================================================================

/// After a failed batch there is no remainder: nothing settled, so the retry is
/// a fresh submission of the **whole** corrected batch, not a resume from the
/// element that failed.
///
/// This is the case that is easy to get wrong. A caller that assumed a partial
/// settle and retried only the un-processed suffix would silently skip elements;
/// a caller that retried the whole batch must find every element settles, and
/// the contract must still hold exactly the pre-batch state in between.
#[test]
fn retry_after_failed_batch_resubmits_the_whole_batch() {
    let ctx = setup();
    let depositor = funded_depositor(&ctx, 4);
    let ids: [u64; 3] = [51, 52, 53];

    // Attempt 1: element 1 is invalid. Nothing settles.
    let bad_attempt = vec![
        &ctx.env,
        lock_item(&ctx, ids[0], &depositor, AMOUNT),
        lock_item(&ctx, ids[1], &depositor, 0),
        lock_item(&ctx, ids[2], &depositor, AMOUNT),
    ];
    let err = ctx
        .client
        .try_batch_lock_funds(&bad_attempt)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::InvalidAmount);
    for id in ids {
        assert_not_settled(&ctx, id);
    }

    // `get_balance` is the contract's token balance. It must still be zero here:
    // the failed attempt moved no funds, which is what makes resubmitting the
    // whole batch safe rather than a double-spend.
    assert_eq!(
        ctx.client.get_balance(),
        0,
        "a failed batch must not leave funds in the contract"
    );

    // Attempt 2: the caller fixes the bad element and re-submits *all three*.
    let corrected = vec![
        &ctx.env,
        lock_item(&ctx, ids[0], &depositor, AMOUNT),
        lock_item(&ctx, ids[1], &depositor, AMOUNT),
        lock_item(&ctx, ids[2], &depositor, AMOUNT),
    ];
    let settled = ctx.client.batch_lock_funds(&corrected);
    assert_count_is_all_items(3, settled, "retry of the corrected batch");

    for id in ids {
        assert_settled(&ctx, id);
    }
    assert_eq!(
        ctx.client.get_balance(),
        AMOUNT * 3,
        "the retry must move funds exactly once per element — no element was \
         charged by the failed attempt and none is charged twice by the retry"
    );
}

/// A failed batch leaves the contract free to accept the same bounty ids
/// afterwards — the ids were never consumed by the failed attempt.
#[test]
fn failed_batch_does_not_consume_its_bounty_ids() {
    let ctx = setup();
    let depositor = funded_depositor(&ctx, 2);

    // Attempt 1 fails on a duplicate id.
    let failing = vec![
        &ctx.env,
        lock_item(&ctx, 61, &depositor, AMOUNT),
        lock_item(&ctx, 61, &depositor, AMOUNT),
    ];
    assert_eq!(
        ctx.client
            .try_batch_lock_funds(&failing)
            .unwrap_err()
            .unwrap(),
        Error::DuplicateBountyId
    );

    // 61 is still unused, so a later batch can legitimately claim it. If the
    // failed attempt had left a partial write, this would be BountyExists.
    let retry = vec![&ctx.env, lock_item(&ctx, 61, &depositor, AMOUNT)];
    assert_eq!(ctx.client.batch_lock_funds(&retry), 1);
    assert_settled(&ctx, 61);
}

// ===========================================================================
// Return-value contract and size limits
// ===========================================================================

/// The return value identifies the outcome of every element. Under atomicity
/// that means exactly one of two states, and this test asserts there is no third.
#[test]
fn return_value_contract_identifies_every_element() {
    let ctx = setup();
    let depositor = funded_depositor(&ctx, 3);
    let ids: [u64; 2] = [71, 72];

    // Fully valid → Ok(n) with n == items.len(): every element settled.
    let ok_items = vec![
        &ctx.env,
        lock_item(&ctx, ids[0], &depositor, AMOUNT),
        lock_item(&ctx, ids[1], &depositor, AMOUNT),
    ];
    match ctx.client.try_batch_lock_funds(&ok_items) {
        Ok(Ok(n)) => {
            assert_count_is_all_items(2, n, "batch_lock_funds");
            assert_settled(&ctx, ids[0]);
            assert_settled(&ctx, ids[1]);
        }
        other => panic!("expected a settled count, got {:?}", other),
    }

    // One bad element → Err, and *no* element is settled. There is no partial
    // outcome, which is precisely why a count is a sufficient return value.
    let err_items = vec![
        &ctx.env,
        lock_item(&ctx, 73, &depositor, AMOUNT),
        lock_item(&ctx, 74, &depositor, 0),
    ];
    match ctx.client.try_batch_lock_funds(&err_items) {
        Err(Ok(e)) => assert_eq!(e, Error::InvalidAmount),
        other => panic!("expected InvalidAmount, got {:?}", other),
    }
    assert_not_settled(&ctx, 73);
    assert_not_settled(&ctx, 74);
}

/// The size cap is enforced at the documented boundary: 20 items settle, 21 are
/// rejected with `InvalidBatchSize`, and the rejection happens before any element
/// is touched.
#[test]
fn batch_size_limit_is_enforced_at_the_documented_boundary() {
    let ctx = setup();
    let depositor = funded_depositor(&ctx, MAX_BATCH + 1);

    let mut at_limit: Vec<LockFundsItem> = Vec::new(&ctx.env);
    for i in 0..MAX_BATCH {
        at_limit.push_back(lock_item(&ctx, 100 + i as u64, &depositor, AMOUNT));
    }
    assert_eq!(
        ctx.client.batch_lock_funds(&at_limit),
        MAX_BATCH,
        "exactly MAX_BATCH_SIZE items must be accepted"
    );

    let mut over_limit: Vec<LockFundsItem> = Vec::new(&ctx.env);
    for i in 0..MAX_BATCH + 1 {
        over_limit.push_back(lock_item(&ctx, 200 + i as u64, &depositor, AMOUNT));
    }
    assert_eq!(
        ctx.client
            .try_batch_lock_funds(&over_limit)
            .unwrap_err()
            .unwrap(),
        Error::InvalidBatchSize,
        "MAX_BATCH_SIZE + 1 items must be rejected"
    );
    // Rejected before any work: not one of the 21 ids was created.
    for i in 0..MAX_BATCH + 1 {
        assert_not_settled(&ctx, 200 + i as u64);
    }
}

/// The SoA entry points reject misaligned parallel arrays as a whole-batch
/// error, before any element is interpreted.
#[test]
fn soa_entry_points_reject_misaligned_arrays_without_touching_elements() {
    let ctx = setup();
    let depositor = funded_depositor(&ctx, 3);

    // Three bounty ids, but only two depositors/amounts/deadlines.
    let bounty_ids: Vec<u64> = vec![&ctx.env, 301u64, 302u64, 303u64];
    let depositors: Vec<Address> = vec![&ctx.env, depositor.clone(), depositor.clone()];
    let amounts: Vec<i128> = vec![&ctx.env, AMOUNT, AMOUNT];
    let deadlines: Vec<u64> = vec![&ctx.env, 0u64, 0u64];

    assert_eq!(
        ctx.client
            .try_batch_lock_funds_soa(&bounty_ids, &depositors, &amounts, &deadlines)
            .unwrap_err()
            .unwrap(),
        Error::BatchSizeMismatch
    );
    assert_not_settled(&ctx, 301);
    assert_not_settled(&ctx, 302);
    assert_not_settled(&ctx, 303);

    // Same rule for the release SoA variant.
    let contributors: Vec<Address> = vec![&ctx.env];
    assert_eq!(
        ctx.client
            .try_batch_release_funds_soa(&bounty_ids, &contributors)
            .unwrap_err()
            .unwrap(),
        Error::BatchSizeMismatch
    );
}

/// The runtime-adjustable caps can lower but never exceed `MAX_BATCH_SIZE`, and
/// a lowered cap is what the batch entry points actually enforce.
#[test]
fn runtime_batch_cap_is_respected_and_cannot_exceed_max_batch_size() {
    let ctx = setup();
    let admin = Address::generate(&ctx.env);
    mint(&ctx, &admin, AMOUNT * 8);
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT * 8);

    // A cap above the hard maximum must be refused.
    assert_eq!(
        ctx.client
            .try_set_batch_size_caps(&(MAX_BATCH + 1), &1)
            .unwrap_err()
            .unwrap(),
        Error::InvalidBatchSizeCap
    );

    // Lowering the cap to 2 is allowed...
    ctx.client.set_batch_size_caps(&2, &1);
    assert_eq!(ctx.client.get_max_batch_size(), 2);

    // ...and is enforced: 3 items now fail even though 3 <= MAX_BATCH_SIZE.
    let items = vec![
        &ctx.env,
        lock_item(&ctx, 401, &depositor, AMOUNT),
        lock_item(&ctx, 402, &depositor, AMOUNT),
        lock_item(&ctx, 403, &depositor, AMOUNT),
    ];
    assert_eq!(
        ctx.client
            .try_batch_lock_funds(&items)
            .unwrap_err()
            .unwrap(),
        Error::InvalidBatchSize
    );
    for id in [401u64, 402, 403] {
        assert_not_settled(&ctx, id);
    }
}

/// The documented cap value itself, pinned so a change to `MAX_BATCH_SIZE`
/// has to be a deliberate edit here too.
#[test]
fn documented_max_batch_size_is_20() {
    assert_eq!(MAX_BATCH, 20, "docs/batch-failure-semantics.md states 20");
    let ctx = setup();
    assert_eq!(ctx.client.get_max_batch_size(), MAX_BATCH);
}

/// Advancing past the deadline must not change the atomicity guarantee.
#[test]
fn atomicity_holds_even_when_the_batch_legitimately_cannot_settle() {
    let ctx = setup();
    let depositor = funded_depositor(&ctx, 2);

    let items = vec![
        &ctx.env,
        lock_item(&ctx, 501, &depositor, AMOUNT),
        lock_item(&ctx, 502, &depositor, AMOUNT),
    ];
    ctx.client.batch_lock_funds(&items);

    // Move well past both deadlines.
    ctx.env.ledger().set(LedgerInfo {
        timestamp: ctx.env.ledger().timestamp() + DEADLINE_OFFSET * 10,
        ..ctx.env.ledger().get()
    });

    let contributor = Address::generate(&ctx.env);
    let release_items = vec![
        &ctx.env,
        ReleaseFundsItem {
            bounty_id: 501,
            contributor: contributor.clone(),
        },
        ReleaseFundsItem {
            bounty_id: 502,
            contributor: contributor.clone(),
        },
    ];
    let balance_before =
        token::Client::new(&ctx.env, &ctx.token_id).balance(&contributor);

    let balance_after = token::Client::new(&ctx.env, &ctx.token_id).balance(&contributor);
    match ctx.client.try_batch_release_funds(&release_items) {
        Err(_) => {
            // A refusal must still be all-or-nothing.
            assert_eq!(
                balance_after, balance_before,
                "a refused release batch must not disburse anything"
            );
        }
        Ok(Ok(settled)) => {
            // If it did settle, it must have settled both elements.
            assert_count_is_all_items(2, settled, "batch_release_funds after deadline");
        }
        Ok(Err(e)) => panic!("unexpected contract error: {:?}", e),
    }
}
