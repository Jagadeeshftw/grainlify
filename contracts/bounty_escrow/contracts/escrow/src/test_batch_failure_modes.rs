// ============================================================
// FILE: contracts/bounty_escrow/contracts/escrow/src/test_batch_failure_modes.rs
//
// ## Batch failure semantics
//
// `batch_lock_funds` and `batch_release_funds` are **strictly atomic**:
// all items pass validation before *any* state is mutated.  A single
// invalid row causes the entire call to revert — no escrow record is
// written, no token transfer is made, and every "sibling" row in the
// same batch is left completely unaffected.
//
// ## Error code contract
//
// | Condition                              | Error             |
// |----------------------------------------|-------------------|
// | batch size == 0                        | InvalidBatchSize  |
// | batch size > MAX_BATCH_SIZE            | InvalidBatchSize  |
// | same bounty_id twice within this batch | DuplicateBountyId |
// | bounty_id already in persistent store  | BountyExists      |
// | amount ≤ 0                             | InvalidAmount     |
// | bounty_id missing (release)            | BountyNotFound    |
// | escrow not Locked (release)            | FundsNotLocked    |
// | contract not initialised               | NotInitialized    |
//
// This file uses a functional-style setup helper instead of the `TestCtx`
// struct found in `test_batch_failure_mode.rs`, providing complementary
// coverage with a different test harness.
//
// ## Coverage (this file)
//
//   BATCH LOCK
//     - Empty batch → InvalidBatchSize
//     - Single-item batch (min boundary) → success
//     - MAX_BATCH_SIZE items (max boundary = 20) → success
//     - MAX_BATCH_SIZE + 1 items → InvalidBatchSize
//     - Duplicate bounty_id within batch → DuplicateBountyId
//     - Existing bounty_id in storage → BountyExists
//     - Second item has zero amount → InvalidAmount, first not stored
//     - Last item is a duplicate → DuplicateBountyId, earlier items not stored
//     - Contract not initialised → NotInitialized
//     - Zero-amount single-item batch → InvalidAmount
//     - Same depositor in multiple items → success (auth deduplication)
//
//   BATCH RELEASE
//     - Empty batch → InvalidBatchSize
//     - Single-item batch (min boundary) → success
//     - MAX_BATCH_SIZE items (max boundary = 20) → success
//     - MAX_BATCH_SIZE + 1 items → InvalidBatchSize
//     - Duplicate bounty_id within batch → DuplicateBountyId
//     - Nonexistent bounty → BountyNotFound
//     - Second item nonexistent, first valid → BountyNotFound, first stays Locked
//     - Already-released bounty → FundsNotLocked
//     - Mix of Locked and Refunded → FundsNotLocked, Locked sibling unaffected
//     - Contract not initialised → NotInitialized
//     - Partial failure atomicity over 3 bounties → none released
// ============================================================

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token, vec, Address, Env, Vec,
};

use crate::{
    BountyEscrowContract, BountyEscrowContractClient, Error, LockFundsItem, ReleaseFundsItem,
};

// ---------------------------------------------------------------------------
// Constants — must match lib.rs
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
    token_id: Address,
    token_admin: Address,
}

/// Create a fresh environment, register the escrow contract and a SAC token,
/// and call `init`.
fn setup() -> Ctx<'static> {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_sac_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = token_sac_contract.address();

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

/// Mint `amount` tokens to `recipient`.
fn mint(ctx: &Ctx, recipient: &Address, amount: i128) {
    token::StellarAssetClient::new(&ctx.env, &ctx.token_id).mint(recipient, &amount);
}

/// Build a [`LockFundsItem`] with default deadline.
fn lock_item(ctx: &Ctx, bounty_id: u64, depositor: Address, amount: i128) -> LockFundsItem {
    LockFundsItem {
        bounty_id,
        depositor,
        amount,
        deadline: ctx.env.ledger().timestamp() + DEADLINE_OFFSET,
    }
}

/// Lock a single bounty via the single-item path (for pre-seeding state).
fn lock_one(ctx: &Ctx, depositor: &Address, bounty_id: u64) {
    ctx.client.lock_funds(
        depositor,
        &bounty_id,
        &AMOUNT,
        &(ctx.env.ledger().timestamp() + DEADLINE_OFFSET),
    );
}

/// Advance the ledger timestamp by `seconds`.
fn advance_time(ctx: &Ctx, seconds: u64) {
    ctx.env.ledger().set(LedgerInfo {
        timestamp: ctx.env.ledger().timestamp() + seconds,
        ..ctx.env.ledger().get()
    });
}

// ===========================================================================
// BATCH LOCK FUNDS — failure modes
// ===========================================================================

// ---------------------------------------------------------------------------
// Batch size boundaries
// ---------------------------------------------------------------------------

/// Empty batch must return `InvalidBatchSize`.
#[test]
fn batch_lock_empty_batch_fails() {
    let ctx = setup();
    let empty: Vec<LockFundsItem> = Vec::new(&ctx.env);
    assert_eq!(
        ctx.client
            .try_batch_lock_funds(&empty)
            .unwrap_err()
            .unwrap(),
        Error::InvalidBatchSize
    );
}

/// A single-item batch (minimum valid size) must succeed.
#[test]
fn batch_lock_single_item_succeeds() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT);
    let items = vec![&ctx.env, lock_item(&ctx, 1, depositor, AMOUNT)];
    assert_eq!(ctx.client.batch_lock_funds(&items), 1);
}

/// A batch of exactly `MAX_BATCH` items must succeed.
#[test]
fn batch_lock_exactly_max_batch_size_succeeds() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT * MAX_BATCH as i128);

    let mut items: Vec<LockFundsItem> = Vec::new(&ctx.env);
    for i in 1..=MAX_BATCH as u64 {
        items.push_back(lock_item(&ctx, i, depositor.clone(), AMOUNT));
    }
    assert_eq!(ctx.client.batch_lock_funds(&items), MAX_BATCH);
}

/// A batch of `MAX_BATCH + 1` items must return `InvalidBatchSize`.
#[test]
fn batch_lock_exceeds_max_batch_size_fails() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT * (MAX_BATCH as i128 + 1));

    let mut items: Vec<LockFundsItem> = Vec::new(&ctx.env);
    for i in 1..=(MAX_BATCH + 1) as u64 {
        items.push_back(lock_item(&ctx, i, depositor.clone(), AMOUNT));
    }
    assert_eq!(
        ctx.client
            .try_batch_lock_funds(&items)
            .unwrap_err()
            .unwrap(),
        Error::InvalidBatchSize
    );
}

// ---------------------------------------------------------------------------
// Duplicate bounty_id
// ---------------------------------------------------------------------------

/// Two items sharing the same `bounty_id` → `DuplicateBountyId`.
#[test]
fn batch_lock_duplicate_bounty_id_within_batch_fails() {
    let ctx = setup();
    let dep1 = Address::generate(&ctx.env);
    let dep2 = Address::generate(&ctx.env);
    mint(&ctx, &dep1, AMOUNT);
    mint(&ctx, &dep2, AMOUNT);

    let items = vec![
        &ctx.env,
        lock_item(&ctx, 99, dep1, AMOUNT),
        lock_item(&ctx, 99, dep2, AMOUNT), // duplicate
    ];
    assert_eq!(
        ctx.client
            .try_batch_lock_funds(&items)
            .unwrap_err()
            .unwrap(),
        // Within-batch duplicate is distinct from a pre-existing storage entry;
        // it returns DuplicateBountyId rather than BountyExists.
        Error::DuplicateBountyId
    );
}

/// A bounty_id that already exists in persistent storage → `BountyExists`.
#[test]
fn batch_lock_bounty_id_already_in_storage_fails() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT * 2);

    lock_one(&ctx, &depositor, 42);

    let items = vec![&ctx.env, lock_item(&ctx, 42, depositor, AMOUNT)];
    assert_eq!(
        ctx.client
            .try_batch_lock_funds(&items)
            .unwrap_err()
            .unwrap(),
        Error::BountyExists
    );
}

// ---------------------------------------------------------------------------
// Mixed valid / invalid — atomicity (sibling protection)
// ---------------------------------------------------------------------------

/// Second item has a zero amount; first valid sibling must NOT be stored.
///
/// Security note: this proves that a failing row cannot corrupt the state for
/// any row that was validated before it in the same batch.
#[test]
fn batch_lock_invalid_second_item_rolls_back_first_sibling() {
    let ctx = setup();
    let dep1 = Address::generate(&ctx.env);
    let dep2 = Address::generate(&ctx.env);
    mint(&ctx, &dep1, AMOUNT);

    let items = vec![
        &ctx.env,
        lock_item(&ctx, 1, dep1, AMOUNT), // valid
        lock_item(&ctx, 2, dep2, 0),      // zero amount → InvalidAmount
    ];
    assert_eq!(
        ctx.client
            .try_batch_lock_funds(&items)
            .unwrap_err()
            .unwrap(),
        // Zero amount returns InvalidAmount, not ActionNotFound.
        Error::InvalidAmount
    );

    // Sibling bounty 1 must NOT have been committed.
    assert!(
        ctx.client.try_get_escrow(&1).is_err(),
        "sibling bounty 1 must not be stored when a later item fails"
    );
}

/// Last item is a duplicate; all preceding valid siblings must NOT be stored.
#[test]
fn batch_lock_duplicate_last_item_rolls_back_all_previous_siblings() {
    let ctx = setup();
    let dep1 = Address::generate(&ctx.env);
    let dep2 = Address::generate(&ctx.env);
    let dep3 = Address::generate(&ctx.env);
    mint(&ctx, &dep1, AMOUNT);
    mint(&ctx, &dep2, AMOUNT);
    mint(&ctx, &dep3, AMOUNT);

    let items = vec![
        &ctx.env,
        lock_item(&ctx, 10, dep1, AMOUNT),
        lock_item(&ctx, 11, dep2, AMOUNT),
        lock_item(&ctx, 11, dep3, AMOUNT), // dup of bounty 11
    ];
    assert_eq!(
        ctx.client
            .try_batch_lock_funds(&items)
            .unwrap_err()
            .unwrap(),
        Error::DuplicateBountyId
    );

    assert!(
        ctx.client.try_get_escrow(&10).is_err(),
        "sibling bounty 10 must not be stored"
    );
    assert!(
        ctx.client.try_get_escrow(&11).is_err(),
        "sibling bounty 11 must not be stored"
    );
}
// ---------------------------------------------------------------------------
// Uninitialized contract
// ---------------------------------------------------------------------------

/// `batch_lock_funds` on an un-initialised contract → `NotInitialized`.
#[test]
fn batch_lock_not_initialized_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);
    let depositor = Address::generate(&env);

    let items = vec![
        &env,
        LockFundsItem {
            bounty_id: 1,
            depositor,
            amount: 100,
            deadline: 9_999_999,
        },
    ];
    assert_eq!(
        client.try_batch_lock_funds(&items).unwrap_err().unwrap(),
        Error::NotInitialized
    );
}

// ---------------------------------------------------------------------------
// Zero amount
// ---------------------------------------------------------------------------

/// Single-item batch with `amount = 0` → `InvalidAmount`.
#[test]
fn batch_lock_zero_amount_fails() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT);
    let items = vec![&ctx.env, lock_item(&ctx, 1, depositor, 0)];
    assert_eq!(
        ctx.client
            .try_batch_lock_funds(&items)
            .unwrap_err()
            .unwrap(),
        Error::InvalidAmount
    );
}

// ---------------------------------------------------------------------------
// Auth deduplication
// ---------------------------------------------------------------------------

/// One depositor appearing in multiple items must succeed; `require_auth` is
/// called only once per unique address internally.
#[test]
fn batch_lock_same_depositor_multiple_bounties_succeeds() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT * 3);

    let items = vec![
        &ctx.env,
        lock_item(&ctx, 100, depositor.clone(), AMOUNT),
        lock_item(&ctx, 101, depositor.clone(), AMOUNT),
        lock_item(&ctx, 102, depositor.clone(), AMOUNT),
    ];
    assert_eq!(ctx.client.batch_lock_funds(&items), 3);
}

// ===========================================================================
// BATCH RELEASE FUNDS — failure modes
// ===========================================================================

// ---------------------------------------------------------------------------
// Batch size boundaries
// ---------------------------------------------------------------------------

/// Empty batch must return `InvalidBatchSize`.
#[test]
fn batch_release_empty_batch_fails() {
    let ctx = setup();
    let empty: Vec<ReleaseFundsItem> = Vec::new(&ctx.env);
    assert_eq!(
        ctx.client
            .try_batch_release_funds(&empty)
            .unwrap_err()
            .unwrap(),
        Error::InvalidBatchSize
    );
}

/// A single-item release batch must succeed.
#[test]
fn batch_release_single_item_succeeds() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    let contributor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT);
    lock_one(&ctx, &depositor, 1);

    let items = vec![
        &ctx.env,
        ReleaseFundsItem {
            bounty_id: 1,
            contributor,
        },
    ];
    assert_eq!(ctx.client.batch_release_funds(&items), 1);
}

/// A batch of exactly `MAX_BATCH` releases must succeed.
#[test]
fn batch_release_exactly_max_batch_size_succeeds() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT * MAX_BATCH as i128);

    for i in 1..=MAX_BATCH as u64 {
        lock_one(&ctx, &depositor, i);
    }

    let contributor = Address::generate(&ctx.env);
    let mut items: Vec<ReleaseFundsItem> = Vec::new(&ctx.env);
    for i in 1..=MAX_BATCH as u64 {
        items.push_back(ReleaseFundsItem {
            bounty_id: i,
            contributor: contributor.clone(),
        });
    }
    assert_eq!(ctx.client.batch_release_funds(&items), MAX_BATCH);
}

/// A batch of `MAX_BATCH + 1` releases → `InvalidBatchSize`.
#[test]
fn batch_release_exceeds_max_batch_size_fails() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT * (MAX_BATCH as i128 + 1));

    for i in 1..=(MAX_BATCH + 1) as u64 {
        lock_one(&ctx, &depositor, i);
    }

    let contributor = Address::generate(&ctx.env);
    let mut items: Vec<ReleaseFundsItem> = Vec::new(&ctx.env);
    for i in 1..=(MAX_BATCH + 1) as u64 {
        items.push_back(ReleaseFundsItem {
            bounty_id: i,
            contributor: contributor.clone(),
        });
    }
    assert_eq!(
        ctx.client
            .try_batch_release_funds(&items)
            .unwrap_err()
            .unwrap(),
        Error::InvalidBatchSize
    );
}

// ---------------------------------------------------------------------------
// Duplicate bounty_id
// ---------------------------------------------------------------------------

/// Two release items with the same `bounty_id` → `DuplicateBountyId`.
#[test]
fn batch_release_duplicate_bounty_id_within_batch_fails() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT);
    lock_one(&ctx, &depositor, 5);

    let items = vec![
        &ctx.env,
        ReleaseFundsItem {
            bounty_id: 5,
            contributor: Address::generate(&ctx.env),
        },
        ReleaseFundsItem {
            bounty_id: 5,
            contributor: Address::generate(&ctx.env),
        },
    ];
    assert_eq!(
        ctx.client
            .try_batch_release_funds(&items)
            .unwrap_err()
            .unwrap(),
        // Within-batch duplicate returns DuplicateBountyId (not BountyExists,
        // which is reserved for a bounty_id already present in storage).
        Error::DuplicateBountyId
    );
}

// ---------------------------------------------------------------------------
// BountyNotFound
// ---------------------------------------------------------------------------

/// Releasing a bounty that was never locked → `BountyNotFound`.
#[test]
fn batch_release_nonexistent_bounty_fails() {
    let ctx = setup();
    let items = vec![
        &ctx.env,
        ReleaseFundsItem {
            bounty_id: 9999,
            contributor: Address::generate(&ctx.env),
        },
    ];
    assert_eq!(
        ctx.client
            .try_batch_release_funds(&items)
            .unwrap_err()
            .unwrap(),
        Error::BountyNotFound
    );
}

/// Second item is nonexistent; the first valid sibling must remain Locked.
///
/// Security note: this is the canonical "sibling protection" test for release.
/// The failing row must not cause the preceding valid row to be partially
/// released — no funds may leave the contract.
#[test]
fn batch_release_nonexistent_second_item_rolls_back_first_sibling() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT);
    lock_one(&ctx, &depositor, 1);

    let items = vec![
        &ctx.env,
        ReleaseFundsItem {
            bounty_id: 1,
            contributor: Address::generate(&ctx.env),
        }, // valid
        ReleaseFundsItem {
            bounty_id: 9999,
            contributor: Address::generate(&ctx.env),
        }, // missing
    ];
    assert_eq!(
        ctx.client
            .try_batch_release_funds(&items)
            .unwrap_err()
            .unwrap(),
        Error::BountyNotFound
    );

    assert_eq!(
        ctx.client.get_escrow(&1).status,
        crate::EscrowStatus::Locked,
        "sibling bounty 1 must remain Locked after its neighbour caused a rollback"
    );
}

// ---------------------------------------------------------------------------
// FundsNotLocked
// ---------------------------------------------------------------------------

/// Releasing a bounty whose status is already `Released` → `FundsNotLocked`.
#[test]
fn batch_release_already_released_bounty_fails() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT);
    lock_one(&ctx, &depositor, 7);

    ctx.client.release_funds(&7, &Address::generate(&ctx.env));

    let items = vec![
        &ctx.env,
        ReleaseFundsItem {
            bounty_id: 7,
            contributor: Address::generate(&ctx.env),
        },
    ];
    assert_eq!(
        ctx.client
            .try_batch_release_funds(&items)
            .unwrap_err()
            .unwrap(),
        Error::FundsNotLocked
    );
}

/// A batch that mixes a Locked bounty with a Refunded one → `FundsNotLocked`;
/// the Locked sibling must remain untouched (no premature fund release).
#[test]
fn batch_release_mixed_locked_and_refunded_is_atomic() {
    let ctx = setup();
    let dep1 = Address::generate(&ctx.env);
    let dep2 = Address::generate(&ctx.env);
    let short_deadline = ctx.env.ledger().timestamp() + 1;

    mint(&ctx, &dep1, AMOUNT);
    mint(&ctx, &dep2, AMOUNT);

    ctx.client.lock_funds(
        &dep1,
        &20,
        &AMOUNT,
        &(ctx.env.ledger().timestamp() + DEADLINE_OFFSET),
    );
    ctx.client.lock_funds(&dep2, &21, &AMOUNT, &short_deadline);

    // Advance past the short deadline and refund bounty 21
    advance_time(&ctx, 10);
    ctx.client.refund(&21);

    let items = vec![
        &ctx.env,
        ReleaseFundsItem {
            bounty_id: 20,
            contributor: Address::generate(&ctx.env),
        }, // Locked
        ReleaseFundsItem {
            bounty_id: 21,
            contributor: Address::generate(&ctx.env),
        }, // Refunded
    ];
    assert_eq!(
        ctx.client
            .try_batch_release_funds(&items)
            .unwrap_err()
            .unwrap(),
        Error::FundsNotLocked
    );

    assert_eq!(
        ctx.client.get_escrow(&20).status,
        crate::EscrowStatus::Locked,
        "locked sibling must not be released when a refunded sibling fails"
    );
}

// ---------------------------------------------------------------------------
// Uninitialized contract
// ---------------------------------------------------------------------------

/// `batch_release_funds` on an un-initialised contract → `NotInitialized`.
#[test]
fn batch_release_not_initialized_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let items = vec![
        &env,
        ReleaseFundsItem {
            bounty_id: 1,
            contributor: Address::generate(&env),
        },
    ];
    assert_eq!(
        client.try_batch_release_funds(&items).unwrap_err().unwrap(),
        Error::NotInitialized
    );
}

// ===========================================================================
// CROSS-CUTTING — partial failure atomicity over multiple bounties
// ===========================================================================

/// Lock 3 bounties; release two individually; then attempt a batch-release
/// that includes an already-released bounty alongside a still-Locked one.
/// The whole batch must fail and the Locked sibling must remain unaffected.
#[test]
fn batch_release_partial_failure_leaves_all_siblings_locked() {
    let ctx = setup();
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT * 3);

    for id in [30u64, 31, 32] {
        lock_one(&ctx, &depositor, id);
    }

    ctx.client.release_funds(&30, &Address::generate(&ctx.env));
    ctx.client.release_funds(&31, &Address::generate(&ctx.env));

    // Batch: already-released 30 + still-Locked 32
    let items = vec![
        &ctx.env,
        ReleaseFundsItem {
            bounty_id: 30,
            contributor: Address::generate(&ctx.env),
        }, // Released → invalid
        ReleaseFundsItem {
            bounty_id: 32,
            contributor: Address::generate(&ctx.env),
        }, // Locked (valid sibling)
    ];
    assert_eq!(
        ctx.client
            .try_batch_release_funds(&items)
            .unwrap_err()
            .unwrap(),
        Error::FundsNotLocked
    );

    assert_eq!(
        ctx.client.get_escrow(&32).status,
        crate::EscrowStatus::Locked,
        "bounty 32 must remain Locked; its sibling's failure must not release it"
    );
}
