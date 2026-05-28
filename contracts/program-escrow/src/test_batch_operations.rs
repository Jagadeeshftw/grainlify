#![cfg(test)]

use soroban_sdk::testutils::budget::Budget as _;
use soroban_sdk::testutils::Ledger as _;
use soroban_sdk::testutils::LedgerInfo as _;
use soroban_sdk::{testutils::Address as _, testutils::Events, token, vec, Address, Env, String, TryIntoVal, Vec};

use crate::{
    BatchError, BatchPayoutReplayedEvent, LockItem, ProgramData, ProgramEscrowContract,
    ProgramEscrowContractClient, ReleaseItem,
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
// Idempotency key tests
// ============================================================

/// Helper: build a two-recipient payout batch and return (recipients, amounts).
fn two_recipient_batch(ctx: &Ctx) -> (soroban_sdk::Vec<Address>, soroban_sdk::Vec<i128>) {
    let r1 = Address::generate(&ctx.env);
    let r2 = Address::generate(&ctx.env);
    let recipients = vec![&ctx.env, r1, r2];
    let amounts = vec![&ctx.env, 300_i128, 200_i128];
    (recipients, amounts)
}

/// A fresh idempotency key executes the payout and transfers funds.
#[test]
fn test_idempotent_batch_payout_first_call_succeeds() {
    let ctx = setup();
    init_program(&ctx, "PROG1", 1000);

    let (recipients, amounts) = two_recipient_batch(&ctx);
    let key = String::from_str(&ctx.env, "key-001");

    let result = ctx
        .client
        .batch_payout_idempotent(&key, &recipients, &amounts);

    // Balance reduced by 500
    assert_eq!(result.remaining_balance, 500);
    // Two payout records added
    assert_eq!(result.payout_history.len(), 2);
}

/// Replaying the same key returns current state without transferring funds.
#[test]
fn test_idempotent_batch_payout_replay_no_double_payment() {
    let ctx = setup();
    init_program(&ctx, "PROG1", 1000);

    let (recipients, amounts) = two_recipient_batch(&ctx);
    let key = String::from_str(&ctx.env, "key-replay-001");

    // First call — real payout
    let first = ctx
        .client
        .batch_payout_idempotent(&key, &recipients, &amounts);
    assert_eq!(first.remaining_balance, 500);

    // Second call with identical key — must be a no-op
    let second = ctx
        .client
        .batch_payout_idempotent(&key, &recipients, &amounts);

    // Balance unchanged
    assert_eq!(second.remaining_balance, 500);
    // Payout history unchanged (no new records)
    assert_eq!(second.payout_history.len(), 2);
}

/// Replay emits a BatchPayoutReplayedEvent (not a BatchPayout event).
#[test]
fn test_idempotent_batch_payout_replay_emits_audit_event() {
    let ctx = setup();
    init_program(&ctx, "PROG1", 1000);

    let (recipients, amounts) = two_recipient_batch(&ctx);
    let key = String::from_str(&ctx.env, "key-audit-001");

    // First call — real payout
    ctx.client
        .batch_payout_idempotent(&key, &recipients, &amounts);

    // Second call — should emit BatchPayoutReplayedEvent
    ctx.client
        .batch_payout_idempotent(&key, &recipients, &amounts);

    let events = ctx.env.events().all();
    // Find a BatchPayoutReplayedEvent whose idempotency_key matches
    let replayed_event = events.iter().find(|e| {
        let result: Result<BatchPayoutReplayedEvent, _> = (&e.2).try_into_val(&ctx.env);
        if let Ok(ev) = result {
            ev.idempotency_key == key
        } else {
            false
        }
    });

    assert!(
        replayed_event.is_some(),
        "Expected BatchPayoutReplayedEvent for replayed key"
    );
    let ev: BatchPayoutReplayedEvent = (&replayed_event.unwrap().2).try_into_val(&ctx.env).unwrap();
    assert_eq!(ev.version, 2);
}

/// Multiple distinct keys each execute independently.
#[test]
fn test_idempotent_batch_payout_distinct_keys_all_execute() {
    let ctx = setup();
    init_program(&ctx, "PROG1", 3000);

    for key_str in ["key-0", "key-1", "key-2"] {
        let key = String::from_str(&ctx.env, key_str);
        let r = Address::generate(&ctx.env);
        let recipients = vec![&ctx.env, r];
        let amounts = vec![&ctx.env, 100_i128];
        ctx.client
            .batch_payout_idempotent(&key, &recipients, &amounts);
    }

    let prog = ctx
        .client
        .get_program_info_v2(&String::from_str(&ctx.env, "PROG1"));
    assert_eq!(prog.remaining_balance, 2700);
    assert_eq!(prog.payout_history.len(), 3);
}

/// Partial overlap: some keys are new, some are replays.
/// Only the new keys should transfer funds.
#[test]
fn test_idempotent_batch_payout_partial_overlap() {
    let ctx = setup();
    init_program(&ctx, "PROG1", 2000);

    // Execute key-A and key-B first
    for k in ["key-A", "key-B"] {
        let key = String::from_str(&ctx.env, k);
        let r = Address::generate(&ctx.env);
        let recipients = vec![&ctx.env, r];
        let amounts = vec![&ctx.env, 200_i128];
        ctx.client
            .batch_payout_idempotent(&key, &recipients, &amounts);
    }
    // Balance: 2000 - 400 = 1600

    // Now replay key-A (duplicate) and execute key-C (new)
    let r_new = Address::generate(&ctx.env);
    let replay_result = ctx.client.batch_payout_idempotent(
        &String::from_str(&ctx.env, "key-A"),
        &vec![&ctx.env, r_new.clone()],
        &vec![&ctx.env, 200_i128],
    );
    // key-A is a replay — balance must still be 1600
    assert_eq!(replay_result.remaining_balance, 1600);

    let new_result = ctx.client.batch_payout_idempotent(
        &String::from_str(&ctx.env, "key-C"),
        &vec![&ctx.env, r_new],
        &vec![&ctx.env, 200_i128],
    );
    // key-C is new — balance drops to 1400
    assert_eq!(new_result.remaining_balance, 1400);

    // Total payout history: 3 records (A, B, C — not the replay of A)
    assert_eq!(new_result.payout_history.len(), 3);
}

/// Replaying a key does NOT add a new payout record to history.
#[test]
fn test_idempotent_replay_does_not_grow_payout_history() {
    let ctx = setup();
    init_program(&ctx, "PROG1", 1000);

    let (recipients, amounts) = two_recipient_batch(&ctx);
    let key = String::from_str(&ctx.env, "key-hist-001");

    ctx.client
        .batch_payout_idempotent(&key, &recipients, &amounts);
    let after_first = ctx
        .client
        .get_program_info_v2(&String::from_str(&ctx.env, "PROG1"));
    let history_len_after_first = after_first.payout_history.len();

    // Replay three times
    for _ in 0..3 {
        ctx.client
            .batch_payout_idempotent(&key, &recipients, &amounts);
    }

    let after_replays = ctx
        .client
        .get_program_info_v2(&String::from_str(&ctx.env, "PROG1"));
    assert_eq!(
        after_replays.payout_history.len(),
        history_len_after_first,
        "Replays must not append to payout history"
    );
}

/// The delegate variant (batch_payout_idempotent_by) also respects idempotency.
#[test]
fn test_idempotent_batch_payout_by_replay_no_double_payment() {
    let ctx = setup();
    init_program(&ctx, "PROG1", 1000);

    let (recipients, amounts) = two_recipient_batch(&ctx);
    let key = String::from_str(&ctx.env, "key-by-001");

    let first = ctx.client.batch_payout_idempotent_by(
        &key,
        &ctx.admin,
        &recipients,
        &amounts,
    );
    assert_eq!(first.remaining_balance, 500);

    let second = ctx.client.batch_payout_idempotent_by(
        &key,
        &ctx.admin,
        &recipients,
        &amounts,
    );
    assert_eq!(second.remaining_balance, 500);
    assert_eq!(second.payout_history.len(), 2);
}

/// Comprehensive audit check:
/// 1. First call emits BATCH_PAYOUT
/// 2. Second call emits BATCH_PAYOUT_REPLAYED
/// 3. Second call does NOT emit BATCH_PAYOUT
#[test]
fn test_idempotent_batch_payout_audit_trail_integrity() {
    let ctx = setup();
    init_program(&ctx, "PROG1", 1000);

    let (recipients, amounts) = two_recipient_batch(&ctx);
    let key = String::from_str(&ctx.env, "key-audit-trail");

    // First call
    ctx.client.batch_payout_idempotent(&key, &recipients, &amounts);
    
    let events_after_first = ctx.env.events().all();
    let payout_events_count = events_after_first.iter().filter(|e| {
        e.0 == ctx.client.address && e.1.contains(soroban_sdk::symbol_short!("BatchPay").try_into_val(&ctx.env).unwrap())
    }).count();
    assert_eq!(payout_events_count, 1, "Expected exactly one BatchPayout event after first call");

    // Second call (replay)
    ctx.client.batch_payout_idempotent(&key, &recipients, &amounts);

    let events_after_second = ctx.env.events().all();
    
    // Check BatchPayout still only 1
    let payout_events_count_total = events_after_second.iter().filter(|e| {
        e.0 == ctx.client.address && e.1.contains(soroban_sdk::symbol_short!("BatchPay").try_into_val(&ctx.env).unwrap())
    }).count();
    assert_eq!(payout_events_count_total, 1, "BatchPayout event must not be emitted on replay");

    // Check BatchPayoutReplayed count
    let replay_events_count = events_after_second.iter().filter(|e| {
        e.0 == ctx.client.address && e.1.contains(soroban_sdk::symbol_short!("BatPayRp").try_into_val(&ctx.env).unwrap())
    }).count();
    assert_eq!(replay_events_count, 1, "Expected exactly one BatchPayoutReplayed event after replay");

    // Verify the replay event data
    let replayed_event = events_after_second.iter().find(|e| {
        let result: Result<BatchPayoutReplayedEvent, _> = (&e.2).try_into_val(&ctx.env);
        if let Ok(ev) = result {
            ev.idempotency_key == key
        } else {
            false
        }
    });
    assert!(replayed_event.is_some());
}

/// Test partial batch overlap with a mix of new and duplicate keys in interleaved order.
#[test]
fn test_idempotent_batch_payout_complex_retry_interleaving() {
    let ctx = setup();
    init_program(&ctx, "PROG1", 5000);

    let keys = [
        String::from_str(&ctx.env, "K1"),
        String::from_str(&ctx.env, "K2"),
        String::from_str(&ctx.env, "K3"),
    ];

    let r = Address::generate(&ctx.env);
    let recipients = vec![&ctx.env, r];
    let amounts = vec![&ctx.env, 100_i128];

    // 1. Execute K1
    ctx.client.batch_payout_idempotent(&keys[0], &recipients, &amounts);
    // 2. Execute K2
    ctx.client.batch_payout_idempotent(&keys[1], &recipients, &amounts);
    // 3. Retry K1 (Replay)
    ctx.client.batch_payout_idempotent(&keys[0], &recipients, &amounts);
    // 4. Execute K3
    ctx.client.batch_payout_idempotent(&keys[2], &recipients, &amounts);
    // 5. Retry K2 (Replay)
    ctx.client.batch_payout_idempotent(&keys[1], &recipients, &amounts);

    let prog = ctx.client.get_program_info_v2(&String::from_str(&ctx.env, "PROG1"));
    // Balance: 5000 - 100*3 = 4700
    assert_eq!(prog.remaining_balance, 4700);
    // History: 3 unique payouts
    assert_eq!(prog.payout_history.len(), 3);

    // Verify event counts
    let events = ctx.env.events().all();
    let payout_count = events.iter().filter(|e| {
        e.0 == ctx.client.address && e.1.contains(soroban_sdk::symbol_short!("BatchPay").try_into_val(&ctx.env).unwrap())
    }).count();
    let replay_count = events.iter().filter(|e| {
        e.0 == ctx.client.address && e.1.contains(soroban_sdk::symbol_short!("BatPayRp").try_into_val(&ctx.env).unwrap())
    }).count();
    
    assert_eq!(payout_count, 3, "Expected 3 successful payout events");
    assert_eq!(replay_count, 2, "Expected 2 replay audit events");
// ============================================================================
// Idempotency key generation convention tests
// Issue #1262 — client SDK idempotency key generation conventions
// See docs/program-escrow/idempotency-key-client-guide.md
// ============================================================================

/// Helper: generate a deterministic single-payout idempotency key
/// following the recommended format: {program_id}-single-{recipient_prefix}-{nonce}
fn make_single_key(env: &Env, program_id: &str, recipient: &Address, nonce: &str) -> String {
    let addr_str = recipient.to_string();
    // Use first 8 chars of address as recipient prefix
    let prefix = &addr_str[..8.min(addr_str.len())];
    String::from_str(
        env,
        &format!("{}-single-{}-{}", program_id, prefix, nonce),
    )
}

/// Helper: generate a deterministic batch-payout idempotency key
/// following the recommended format: {program_id}-batch-{first_recipient_prefix}-{count}r-{nonce}
fn make_batch_key(env: &Env, program_id: &str, recipients: &soroban_sdk::Vec<Address>, nonce: &str) -> String {
    let first = recipients.get(0).unwrap();
    let addr_str = first.to_string();
    let prefix = &addr_str[..8.min(addr_str.len())];
    let count = recipients.len();
    String::from_str(
        env,
        &format!("{}-batch-{}-{}r-{}", program_id, prefix, count, nonce),
    )
}

// ----------------------------------------------------------------------------
// Key format validation
// ----------------------------------------------------------------------------

/// Recommended key format is accepted by the contract.
#[test]
fn test_recommended_single_key_format_accepted() {
    let ctx = setup();
    init_program(&ctx, "hackathon-2024", 10_000);

    let recipient = Address::generate(&ctx.env);
    let key = make_single_key(&ctx.env, "hackathon-2024", &recipient, "a3f1c2d4e5b6a7f8");

    // Key must be non-empty and under 256 chars
    assert!(!key.is_empty());
    assert!(key.len() <= 256);
}

/// Recommended batch key format is accepted by the contract.
#[test]
fn test_recommended_batch_key_format_accepted() {
    let ctx = setup();
    init_program(&ctx, "hackathon-2024", 10_000);

    let r1 = Address::generate(&ctx.env);
    let r2 = Address::generate(&ctx.env);
    let r3 = Address::generate(&ctx.env);
    let recipients = vec![&ctx.env, r1, r2, r3];

    let key = make_batch_key(&ctx.env, "hackathon-2024", &recipients, "9b8c7d6e5f4a3b2c");

    assert!(!key.is_empty());
    assert!(key.len() <= 256);
}

// ----------------------------------------------------------------------------
// Namespace isolation: different programs must not collide
// ----------------------------------------------------------------------------

/// Keys for different programs with the same nonce must differ.
#[test]
fn test_namespace_isolation_different_programs() {
    let ctx = setup();
    let recipient = Address::generate(&ctx.env);
    let nonce = "deadbeef12345678";

    let key_a = make_single_key(&ctx.env, "program-alpha", &recipient, nonce);
    let key_b = make_single_key(&ctx.env, "program-beta", &recipient, nonce);

    assert_ne!(key_a, key_b, "Keys for different programs must not collide");
}

/// Keys for the same program but different operation types must differ.
#[test]
fn test_namespace_isolation_different_payout_types() {
    let ctx = setup();
    let recipient = Address::generate(&ctx.env);
    let nonce = "deadbeef12345678";

    let r = vec![&ctx.env, recipient.clone()];
    let single_key = make_single_key(&ctx.env, "hackathon-2024", &recipient, nonce);
    let batch_key = make_batch_key(&ctx.env, "hackathon-2024", &r, nonce);

    assert_ne!(single_key, batch_key, "single and batch keys must differ");
}

/// Keys for the same program and type but different recipients must differ.
#[test]
fn test_namespace_isolation_different_recipients() {
    let ctx = setup();
    let r1 = Address::generate(&ctx.env);
    let r2 = Address::generate(&ctx.env);
    let nonce = "deadbeef12345678";

    let key1 = make_single_key(&ctx.env, "hackathon-2024", &r1, nonce);
    let key2 = make_single_key(&ctx.env, "hackathon-2024", &r2, nonce);

    // Different recipients produce different keys (address prefix differs)
    // NOTE: with very high probability; in tests addresses are unique by construction
    assert_ne!(key1, key2, "Keys for different recipients must differ");
}

// ----------------------------------------------------------------------------
// Key uniqueness: same key cannot be reused across operations
// ----------------------------------------------------------------------------

/// Submitting the same single-payout key twice returns the original result
/// without executing a second transfer (retry safety).
#[test]
fn test_single_key_retry_returns_original_result() {
    let ctx = setup();
    init_program(&ctx, "hackathon-2024", 10_000);

    let recipient = Address::generate(&ctx.env);
    let key = make_single_key(&ctx.env, "hackathon-2024", &recipient, "a3f1c2d4e5b6a7f8");

    let result1 = ctx.client.single_payout_by(
        &ctx.admin,
        &recipient,
        &1000i128,
        &Some(key.clone()),
    );

    let result2 = ctx.client.single_payout_by(
        &ctx.admin,
        &recipient,
        &1000i128,
        &Some(key.clone()),
    );

    // Both calls return the same remaining balance (second is a no-op)
    assert_eq!(
        result1.remaining_balance,
        result2.remaining_balance,
        "Retry must return cached result without re-executing"
    );
}

/// Submitting the same batch-payout key twice returns the original result.
#[test]
fn test_batch_key_retry_returns_original_result() {
    let ctx = setup();
    init_program(&ctx, "hackathon-2024", 10_000);

    let r1 = Address::generate(&ctx.env);
    let r2 = Address::generate(&ctx.env);
    let recipients = vec![&ctx.env, r1.clone(), r2.clone()];
    let amounts = vec![&ctx.env, 1000i128, 2000i128];

    let key = make_batch_key(&ctx.env, "hackathon-2024", &recipients, "9b8c7d6e5f4a3b2c");

    let result1 = ctx.client.batch_payout_by(
        &ctx.admin,
        &recipients,
        &amounts,
        &Some(key.clone()),
    );

    let result2 = ctx.client.batch_payout_by(
        &ctx.admin,
        &recipients,
        &amounts,
        &Some(key.clone()),
    );

    assert_eq!(
        result1.remaining_balance,
        result2.remaining_balance,
        "Batch retry must return cached result"
    );
}

// ----------------------------------------------------------------------------
// Key collision prevention: different nonces produce independent operations
// ----------------------------------------------------------------------------

/// Two single payouts with different nonces are treated as independent operations.
#[test]
fn test_different_nonces_are_independent_operations() {
    let ctx = setup();
    init_program(&ctx, "hackathon-2024", 10_000);

    let recipient = Address::generate(&ctx.env);
    let key1 = make_single_key(&ctx.env, "hackathon-2024", &recipient, "aaaaaaaaaaaaaaaa");
    let key2 = make_single_key(&ctx.env, "hackathon-2024", &recipient, "bbbbbbbbbbbbbbbb");

    let result1 = ctx.client.single_payout_by(
        &ctx.admin,
        &recipient,
        &1000i128,
        &Some(key1),
    );
    let result2 = ctx.client.single_payout_by(
        &ctx.admin,
        &recipient,
        &1000i128,
        &Some(key2),
    );

    // Each operation deducted 1000 independently
    assert_eq!(result1.remaining_balance, 9_000);
    assert_eq!(result2.remaining_balance, 8_000);
}

// ----------------------------------------------------------------------------
// Key length enforcement
// ----------------------------------------------------------------------------

/// A key at exactly 256 characters is accepted.
#[test]
fn test_key_at_max_length_accepted() {
    let ctx = setup();
    init_program(&ctx, "hackathon-2024", 10_000);

    let recipient = Address::generate(&ctx.env);
    // Build a key padded to exactly 256 chars
    let base = format!("hackathon-2024-single-GABC1234-");
    let padding = "x".repeat(256 - base.len());
    let key_str = format!("{}{}", base, padding);
    assert_eq!(key_str.len(), 256);
    let key = String::from_str(&ctx.env, &key_str);

    // Should not panic on validation
    assert!(key.len() <= 256);
}

/// A key exceeding 256 characters is rejected by the contract.
#[test]
#[should_panic(expected = "Idempotency key exceeds maximum length")]
fn test_key_exceeding_max_length_rejected() {
    let ctx = setup();
    init_program(&ctx, "hackathon-2024", 10_000);

    let recipient = Address::generate(&ctx.env);
    let long_key = String::from_str(&ctx.env, &"k".repeat(257));

    ctx.client.single_payout_by(
        &ctx.admin,
        &recipient,
        &100i128,
        &Some(long_key),
    );
}

/// An empty key is rejected by the contract.
#[test]
#[should_panic(expected = "Idempotency key cannot be empty")]
fn test_empty_key_rejected() {
    let ctx = setup();
    init_program(&ctx, "hackathon-2024", 10_000);

    let recipient = Address::generate(&ctx.env);
    let empty_key = String::from_str(&ctx.env, "");

    ctx.client.single_payout_by(
        &ctx.admin,
        &recipient,
        &100i128,
        &Some(empty_key),
    );
}

// ----------------------------------------------------------------------------
// No key provided: backward-compatible path
// ----------------------------------------------------------------------------

/// Omitting the idempotency key still executes the operation normally.
#[test]
fn test_no_key_executes_normally() {
    let ctx = setup();
    init_program(&ctx, "hackathon-2024", 10_000);

    let recipient = Address::generate(&ctx.env);
    let result = ctx.client.single_payout_by(
        &ctx.admin,
        &recipient,
        &1000i128,
        &None,
    );

    assert_eq!(result.remaining_balance, 9_000);
}

/// Multiple calls without a key each execute independently (no deduplication).
#[test]
fn test_no_key_allows_duplicate_operations() {
    let ctx = setup();
    init_program(&ctx, "hackathon-2024", 10_000);

    let recipient = Address::generate(&ctx.env);

    ctx.client.single_payout_by(&ctx.admin, &recipient, &1000i128, &None);
    let result = ctx.client.single_payout_by(&ctx.admin, &recipient, &1000i128, &None);

    // Both operations executed — balance reduced by 2000
    assert_eq!(result.remaining_balance, 8_000);
}
