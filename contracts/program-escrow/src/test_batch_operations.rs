#![cfg(test)]

use soroban_sdk::testutils::Ledger as _;
use soroban_sdk::testutils::LedgerInfo as _;
use soroban_sdk::{testutils::Address as _, token, vec, Address, Env, String, TryIntoVal, Vec};

use crate::{
    BatchError, LockItem, ProgramData, ProgramEscrowContract, ProgramEscrowContractClient,
    ReleaseItem, IDEMPOTENCY_KEY_TTL_LEDGERS,
};

pub struct Ctx<'a> {
    pub env: Env,
    pub client: ProgramEscrowContractClient<'a>,
    pub token_id: Address,
    pub token_admin: Address,
    pub admin: Address,
}

pub fn setup() -> Ctx<'static> {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract(token_admin.clone());

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    client.initialize_contract(&admin);

    Ctx {
        env,
        client,
        token_id,
        token_admin,
        admin,
    }
}

fn mint(ctx: &Ctx, recipient: &Address, amount: i128) {
    token::StellarAssetClient::new(&ctx.env, &ctx.token_id).mint(recipient, &amount);
}

pub fn init_program(ctx: &Ctx, program_id: &str, amount: i128) {
    let creator = Address::generate(&ctx.env);
    mint(ctx, &creator, amount);
    ctx.client.init_program(
        &String::from_str(&ctx.env, program_id),
        &ctx.admin.clone(), // authorized_payout_key
        &ctx.token_id,
        &creator,
        &Some(amount),
        &None,
    );
    ctx.client.publish_program();
}

#[test]
fn test_batch_lock_success() {
    let ctx = setup();
    init_program(&ctx, "PROG1", 1000);
    init_program(&ctx, "PROG2", 2000);

    let items = vec![
        &ctx.env,
        LockItem {
            program_id: String::from_str(&ctx.env, "PROG1"),
            amount: 500,
        },
        LockItem {
            program_id: String::from_str(&ctx.env, "PROG2"),
            amount: 1500,
        },
    ];

    let result = ctx.client.batch_lock(&items);
    assert_eq!(result, 2);

    let prog1 = ctx
        .client
        .get_program_info_v2(&String::from_str(&ctx.env, "PROG1"));
    assert_eq!(prog1.total_funds, 1500);
    assert_eq!(prog1.remaining_balance, 1500);

    let prog2 = ctx
        .client
        .get_program_info_v2(&String::from_str(&ctx.env, "PROG2"));
    assert_eq!(prog2.total_funds, 3500);
}

#[test]
fn test_batch_lock_atomicity() {
    let ctx = setup();
    init_program(&ctx, "PROG1", 1000);

    let items = vec![
        &ctx.env,
        LockItem {
            program_id: String::from_str(&ctx.env, "PROG1"),
            amount: 500,
        },
        LockItem {
            program_id: String::from_str(&ctx.env, "NONEXISTENT"),
            amount: 100,
        },
    ];

    let result = ctx.client.try_batch_lock(&items);
    assert!(result.is_err());

    // PROG1 should not be updated
    let prog1 = ctx
        .client
        .get_program_info_v2(&String::from_str(&ctx.env, "PROG1"));
    assert_eq!(prog1.total_funds, 1000);
}

#[test]
fn test_batch_release_success() {
    let ctx = setup();
    init_program(&ctx, "PROG1", 5000);

    // Create schedules
    let recipient1 = Address::generate(&ctx.env);
    let recipient2 = Address::generate(&ctx.env);

    ctx.client.create_program_release_schedule(
        &recipient1,
        &1000,
        &0, // immediate
    );
    ctx.client.create_program_release_schedule(
        &recipient2,
        &2000,
        &0, // immediate
    );

    let items = vec![
        &ctx.env,
        ReleaseItem {
            program_id: String::from_str(&ctx.env, "PROG1"),
            schedule_id: 1,
        },
        ReleaseItem {
            program_id: String::from_str(&ctx.env, "PROG1"),
            schedule_id: 2,
        },
    ];

    let result = ctx.client.batch_release(&items);
    assert_eq!(result, 2);

    // Verify balances
    let prog1 = ctx
        .client
        .get_program_info_v2(&String::from_str(&ctx.env, "PROG1"));
    assert_eq!(prog1.remaining_balance, 2000);

    // Verify tokens were transferred
    let token_client = token::Client::new(&ctx.env, &ctx.token_id);
    assert_eq!(token_client.balance(&recipient1), 1000);
    assert_eq!(token_client.balance(&recipient2), 2000);
}

#[test]
fn test_batch_release_duplicate_fails() {
    let ctx = setup();
    init_program(&ctx, "PROG1", 5000);
    let recipient = Address::generate(&ctx.env);
    ctx.client
        .create_program_release_schedule(&recipient, &1000, &0);

    let items = vec![
        &ctx.env,
        ReleaseItem {
            program_id: String::from_str(&ctx.env, "PROG1"),
            schedule_id: 1,
        },
        ReleaseItem {
            program_id: String::from_str(&ctx.env, "PROG1"),
            schedule_id: 1, // DUPLICATE
        },
    ];

    let result = ctx.client.try_batch_release(&items);
    assert!(result.is_err());
}

// ============================================================
// Idempotency Key Tests
// ============================================================

/// Helper: advance ledger sequence by `n` ledgers.
fn advance_ledger(ctx: &Ctx, n: u32) {
    ctx.env.ledger().with_mut(|li| {
        li.sequence_number += n;
    });
}

#[test]
fn test_batch_payout_with_key_succeeds_first_time() {
    let ctx = setup();
    init_program(&ctx, "PROG1", 10_000);

    let recipient = Address::generate(&ctx.env);
    let key = String::from_str(&ctx.env, "payout-batch-001");

    let result = ctx.client.try_batch_payout_with_key(
        &ctx.admin,
        &key,
        &vec![&ctx.env, recipient.clone()],
        &vec![&ctx.env, 1000i128],
    );
    assert!(result.is_ok());

    let token_client = token::Client::new(&ctx.env, &ctx.token_id);
    assert_eq!(token_client.balance(&recipient), 1000);
}

#[test]
fn test_batch_payout_with_key_duplicate_rejected() {
    let ctx = setup();
    init_program(&ctx, "PROG1", 10_000);

    let recipient = Address::generate(&ctx.env);
    let key = String::from_str(&ctx.env, "payout-batch-dup");

    // First call succeeds
    ctx.client.batch_payout_with_key(
        &ctx.admin,
        &key,
        &vec![&ctx.env, recipient.clone()],
        &vec![&ctx.env, 500i128],
    );

    // Second call with same key must fail with DuplicateIdempotencyKey
    let result = ctx.client.try_batch_payout_with_key(
        &ctx.admin,
        &key,
        &vec![&ctx.env, recipient.clone()],
        &vec![&ctx.env, 500i128],
    );
    assert!(result.is_err());
    // Verify no extra tokens were transferred
    let token_client = token::Client::new(&ctx.env, &ctx.token_id);
    assert_eq!(token_client.balance(&recipient), 500);
}

#[test]
fn test_batch_payout_with_key_expired_rejected_distinctly() {
    let ctx = setup();
    init_program(&ctx, "PROG1", 10_000);

    let recipient = Address::generate(&ctx.env);
    let key = String::from_str(&ctx.env, "payout-batch-exp");

    // First call succeeds
    ctx.client.batch_payout_with_key(
        &ctx.admin,
        &key,
        &vec![&ctx.env, recipient.clone()],
        &vec![&ctx.env, 500i128],
    );

    // Advance past TTL (100_001 ledgers)
    advance_ledger(&ctx, 100_001);

    // Second call with same key after expiry must also fail (expired, not duplicate)
    let result = ctx.client.try_batch_payout_with_key(
        &ctx.admin,
        &key,
        &vec![&ctx.env, recipient.clone()],
        &vec![&ctx.env, 500i128],
    );
    assert!(result.is_err());
}

#[test]
fn test_different_keys_are_independent() {
    let ctx = setup();
    init_program(&ctx, "PROG1", 10_000);

    let recipient = Address::generate(&ctx.env);
    let key1 = String::from_str(&ctx.env, "key-001");
    let key2 = String::from_str(&ctx.env, "key-002");

    ctx.client.batch_payout_with_key(
        &ctx.admin,
        &key1,
        &vec![&ctx.env, recipient.clone()],
        &vec![&ctx.env, 500i128],
    );

    // key2 is independent — must succeed
    let result = ctx.client.try_batch_payout_with_key(
        &ctx.admin,
        &key2,
        &vec![&ctx.env, recipient.clone()],
        &vec![&ctx.env, 500i128],
    );
    assert!(result.is_ok());

    let token_client = token::Client::new(&ctx.env, &ctx.token_id);
    assert_eq!(token_client.balance(&recipient), 1000);
}

#[test]
fn test_prune_idempotency_keys_removes_expired() {
    let ctx = setup();
    init_program(&ctx, "PROG1", 10_000);

    let recipient = Address::generate(&ctx.env);

    // Register 3 keys
    for i in 0u32..3 {
        let key = String::from_str(&ctx.env, &soroban_sdk::format!(&ctx.env, "key-{}", i));
        ctx.client.batch_payout_with_key(
            &ctx.admin,
            &key,
            &vec![&ctx.env, recipient.clone()],
            &vec![&ctx.env, 100i128],
        );
    }

    // Advance past TTL
    advance_ledger(&ctx, 100_001);

    // Prune up to 10 keys
    let pruned = ctx
        .client
        .prune_idempotency_keys(&String::from_str(&ctx.env, "PROG1"), &10);
    assert_eq!(pruned, 3);
}

#[test]
fn test_prune_respects_max_prune_limit() {
    let ctx = setup();
    init_program(&ctx, "PROG1", 10_000);

    let recipient = Address::generate(&ctx.env);

    // Register 5 keys
    for i in 0u32..5 {
        let key = String::from_str(&ctx.env, &soroban_sdk::format!(&ctx.env, "key-{}", i));
        ctx.client.batch_payout_with_key(
            &ctx.admin,
            &key,
            &vec![&ctx.env, recipient.clone()],
            &vec![&ctx.env, 100i128],
        );
    }

    advance_ledger(&ctx, 100_001);

    // Only prune 2 at a time
    let pruned = ctx
        .client
        .prune_idempotency_keys(&String::from_str(&ctx.env, "PROG1"), &2);
    assert_eq!(pruned, 2);

    // Prune the rest
    let pruned2 = ctx
        .client
        .prune_idempotency_keys(&String::from_str(&ctx.env, "PROG1"), &10);
    assert_eq!(pruned2, 3);
}

#[test]
fn test_prune_does_not_remove_valid_keys() {
    let ctx = setup();
    init_program(&ctx, "PROG1", 10_000);

    let recipient = Address::generate(&ctx.env);
    let key = String::from_str(&ctx.env, "still-valid");

    ctx.client.batch_payout_with_key(
        &ctx.admin,
        &key,
        &vec![&ctx.env, recipient.clone()],
        &vec![&ctx.env, 100i128],
    );

    // Do NOT advance ledger — key is still valid
    let pruned = ctx
        .client
        .prune_idempotency_keys(&String::from_str(&ctx.env, "PROG1"), &10);
    assert_eq!(pruned, 0);

    // Key must still be rejected as duplicate
    let result = ctx.client.try_batch_payout_with_key(
        &ctx.admin,
        &key,
        &vec![&ctx.env, recipient.clone()],
        &vec![&ctx.env, 100i128],
    );
    assert!(result.is_err());
}

#[test]
fn test_prune_empty_index_returns_zero() {
    let ctx = setup();
    init_program(&ctx, "PROG1", 1000);

    let pruned = ctx
        .client
        .prune_idempotency_keys(&String::from_str(&ctx.env, "PROG1"), &10);
    assert_eq!(pruned, 0);
}

#[test]
fn test_batch_payout_without_key_unaffected() {
    // Ensure existing batch_payout (no key) still works normally
    let ctx = setup();
    init_program(&ctx, "PROG1", 5000);

    let recipient = Address::generate(&ctx.env);
    let result = ctx.client.try_batch_payout(
        &vec![&ctx.env, recipient.clone()],
        &vec![&ctx.env, 1000i128],
    );
    assert!(result.is_ok());
}
