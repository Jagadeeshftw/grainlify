#![cfg(test)]

use soroban_sdk::testutils::Address as _;
use soroban_sdk::{token, vec, Address, Env, String, Vec};

use crate::{
    PayoutRecord, ProgramData, ProgramEscrowContract, ProgramEscrowContractClient,
};

struct Ctx {
    env: Env,
    client: ProgramEscrowContractClient<'static>,
    token_id: Address,
    admin: Address,
}

fn setup() -> Ctx {
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
        admin,
    }
}

fn mint(ctx: &Ctx, recipient: &Address, amount: i128) {
    token::StellarAssetClient::new(&ctx.env, &ctx.token_id).mint(recipient, &amount);
}

fn init_program_with_funds(ctx: &Ctx, program_id: &str, amount: i128) {
    let creator = Address::generate(&ctx.env);
    mint(ctx, &creator, amount);
    ctx.client.init_program(
        &String::from_str(&ctx.env, program_id),
        &ctx.admin.clone(),
        &ctx.token_id,
        &creator,
        &Some(amount),
        &None,
    );
    ctx.client.publish_program();
}

#[test]
fn test_interleaved_payouts_same_recipient() {
    let ctx = setup();
    init_program_with_funds(&ctx, "PROG1", 10_000);

    let alice = Address::generate(&ctx.env);
    let bob = Address::generate(&ctx.env);
    let charlie = Address::generate(&ctx.env);

    // Step 1: single_payout
    ctx.client.single_payout(&alice, &100, &None);
    let records = ctx.client.query_recipient_history(&alice, &0, &100);
    assert_eq!(records.len(), 1, "step 1: alice should have 1 record");
    assert_eq!(records.get(0).unwrap().recipient, alice);

    // Step 2: batch_payout with alice + bob
    ctx.client.batch_payout(
        &vec![&ctx.env, alice.clone(), bob.clone()],
        &vec![&ctx.env, 200_i128, 300_i128],
        &None,
    );
    let records = ctx.client.query_recipient_history(&alice, &0, &100);
    assert_eq!(records.len(), 2, "step 2: alice should have 2 records");
    for record in records.iter() {
        assert_eq!(record.recipient, alice);
    }

    // Step 3: single_payout_idempotent
    let key1 = String::from_str(&ctx.env, "interleave-key-1");
    ctx.client
        .single_payout_idempotent(&alice, &400, &Some(key1.clone()));
    let records = ctx.client.query_recipient_history(&alice, &0, &100);
    assert_eq!(records.len(), 3, "step 3: alice should have 3 records");

    // Step 4: batch_payout_idempotent with alice + charlie
    let key2 = String::from_str(&ctx.env, "interleave-key-2");
    ctx.client.batch_payout_idempotent(
        &vec![&ctx.env, alice.clone(), charlie.clone()],
        &vec![&ctx.env, 500_i128, 600_i128],
        &Some(key2.clone()),
    );
    let records = ctx.client.query_recipient_history(&alice, &0, &100);
    assert_eq!(records.len(), 4, "step 4: alice should have 4 records");

    // Step 5: another single_payout
    ctx.client.single_payout(&alice, &700, &None);
    let records = ctx.client.query_recipient_history(&alice, &0, &100);
    assert_eq!(records.len(), 5, "step 5: alice should have 5 records");
    for record in records.iter() {
        assert_eq!(record.recipient, alice);
    }

    // Verify bob and charlie have correct counts
    let bob_records = ctx.client.query_recipient_history(&bob, &0, &100);
    assert_eq!(bob_records.len(), 1, "bob should have 1 record");
    let charlie_records = ctx.client.query_recipient_history(&charlie, &0, &100);
    assert_eq!(charlie_records.len(), 1, "charlie should have 1 record");

    // Verify idempotent key replay does not add duplicate
    ctx.client
        .single_payout_idempotent(&alice, &9999, &Some(key1.clone()));
    let records_after_replay = ctx.client.query_recipient_history(&alice, &0, &100);
    assert_eq!(
        records_after_replay.len(),
        5,
        "replaying key1 must not add records"
    );

    ctx.client.batch_payout_idempotent(
        &vec![&ctx.env, alice.clone()],
        &vec![&ctx.env, 9999_i128],
        &Some(key2.clone()),
    );
    let records_after_batch_replay = ctx.client.query_recipient_history(&alice, &0, &100);
    assert_eq!(
        records_after_batch_replay.len(),
        5,
        "replaying key2 must not add records"
    );
}

#[test]
fn test_idempotent_replay_no_duplicate() {
    let ctx = setup();
    init_program_with_funds(&ctx, "PROG1", 10_000);

    let alice = Address::generate(&ctx.env);

    let key_a = String::from_str(&ctx.env, "idem-key-a");
    let key_b = String::from_str(&ctx.env, "idem-key-b");

    // First execution
    ctx.client
        .single_payout_idempotent(&alice, &100, &Some(key_a.clone()));
    let records = ctx.client.query_recipient_history(&alice, &0, &100);
    assert_eq!(records.len(), 1);

    // Replay — should not add
    ctx.client
        .single_payout_idempotent(&alice, &100, &Some(key_a.clone()));
    let records = ctx.client.query_recipient_history(&alice, &0, &100);
    assert_eq!(records.len(), 1, "replay of key_a must not add records");

    // Batch idempotent
    ctx.client.batch_payout_idempotent(
        &vec![&ctx.env, alice.clone()],
        &vec![&ctx.env, 200_i128],
        &Some(key_b.clone()),
    );
    let records = ctx.client.query_recipient_history(&alice, &0, &100);
    assert_eq!(records.len(), 2);

    // Replay batch key
    ctx.client.batch_payout_idempotent(
        &vec![&ctx.env, alice.clone()],
        &vec![&ctx.env, 200_i128],
        &Some(key_b.clone()),
    );
    let records = ctx.client.query_recipient_history(&alice, &0, &100);
    assert_eq!(records.len(), 2, "replay of key_b must not add records");

    // Verify the payout_history on ProgramData is also consistent
    let data: ProgramData = ctx.client.get_program_info();
    assert_eq!(data.payout_history.len(), 2);
}

#[test]
fn test_payout_history_chronological_order() {
    let ctx = setup();
    init_program_with_funds(&ctx, "PROG1", 10_000);

    let alice = Address::generate(&ctx.env);

    let amounts = [100, 200, 300, 400, 500];
    for &amt in &amounts {
        ctx.client.single_payout(&alice, &amt, &None);
    }

    let records = ctx.client.query_recipient_history(&alice, &0, &100);
    assert_eq!(records.len() as usize, amounts.len());

    for (i, record) in records.iter().enumerate() {
        assert_eq!(record.recipient, alice);
        assert_eq!(
            record.amount, amounts[i],
            "record {} should have amount {}",
            i, amounts[i]
        );
    }
}

#[test]
fn test_pagination_consistency_after_interleaved() {
    let ctx = setup();
    init_program_with_funds(&ctx, "PROG1", 10_000);

    let alice = Address::generate(&ctx.env);
    let bob = Address::generate(&ctx.env);

    // Interleave payouts to alice and bob
    for i in 0..5 {
        ctx.client.single_payout(&alice, &(100 * (i + 1)), &None);
        ctx.client.single_payout(
            &bob,
            &(50 * (i + 1)),
            &None,
        );
    }
    // Also a batch payout containing alice
    ctx.client.batch_payout(
        &vec![&ctx.env, alice.clone()],
        &vec![&ctx.env, 999_i128],
        &None,
    );

    // Full result
    let full = ctx.client.query_recipient_history(&alice, &0, &200);
    assert_eq!(full.len(), 6, "alice should have 6 records total");

    // Paginate through
    let mut collected: Vec<PayoutRecord> = Vec::new(&ctx.env);
    let page_size = 2;
    let mut offset = 0;
    loop {
        let page = ctx
            .client
            .query_recipient_history(&alice, &offset, &page_size);
        if page.len() == 0 {
            break;
        }
        for record in page.iter() {
            collected.push_back(record);
        }
        offset += page.len();
    }

    assert_eq!(collected.len(), full.len());
    for (a, b) in collected.iter().zip(full.iter()) {
        assert_eq!(a.recipient, b.recipient);
        assert_eq!(a.amount, b.amount);
    }
}

#[test]
fn test_query_equals_get() {
    let ctx = setup();
    init_program_with_funds(&ctx, "PROG1", 10_000);

    let alice = Address::generate(&ctx.env);
    let bob = Address::generate(&ctx.env);

    ctx.client.single_payout(&alice, &100, &None);
    ctx.client
        .single_payout(&alice, &200, &None);
    ctx.client.single_payout(&bob, &300, &None);
    ctx.client.batch_payout(
        &vec![&ctx.env, alice.clone(), bob.clone()],
        &vec![&ctx.env, 400_i128, 500_i128],
        &None,
    );

    let query_result = ctx.client.query_payouts_by_recipient(&alice, &0, &100);
    let get_result = ctx.client.get_payouts_by_recipient(&alice, &0, &100);

    assert_eq!(query_result.len(), get_result.len());
    for (qr, gr) in query_result.iter().zip(get_result.iter()) {
        assert_eq!(qr.recipient, gr.recipient);
        assert_eq!(qr.amount, gr.amount);
        assert_eq!(qr.timestamp, gr.timestamp);
    }

    // Also verify the new wrapper matches
    let wrapper_result = ctx.client.query_recipient_history(&alice, &0, &100);
    assert_eq!(wrapper_result.len(), query_result.len());
    for (wr, qr) in wrapper_result.iter().zip(query_result.iter()) {
        assert_eq!(wr.recipient, qr.recipient);
        assert_eq!(wr.amount, qr.amount);
    }
}

#[test]
fn test_multi_program_isolation() {
    let ctx = setup();

    // Initialize two separate programs in the same contract
    // (Note: single_payout operates on the "active" program, so we init
    //  PROG1, do its payouts, then init PROG2 as the new active program.)
    init_program_with_funds(&ctx, "PROG1", 10_000);

    let alice = Address::generate(&ctx.env);

    ctx.client.single_payout(&alice, &500, &None);
    ctx.client
        .single_payout(&alice, &500, &None);

    let prog1_records = ctx.client.query_recipient_history(&alice, &0, &100);
    assert_eq!(prog1_records.len(), 2, "PROG1 should have 2 records for alice");

    // Now initialize PROG2 — this replaces the active program data
    init_program_with_funds(&ctx, "PROG2", 10_000);

    ctx.client
        .single_payout(&alice, &1000, &None);

    let prog2_records = ctx.client.query_recipient_history(&alice, &0, &100);
    assert_eq!(prog2_records.len(), 1, "PROG2 should have 1 record for alice");

    // Verify each record has correct amounts
    assert_eq!(
        prog1_records.get(0).unwrap().amount, 500,
        "first PROG1 payout should be 500"
    );
    assert_eq!(
        prog1_records.get(1).unwrap().amount, 500,
        "second PROG1 payout should be 500"
    );
    assert_eq!(
        prog2_records.get(0).unwrap().amount, 1000,
        "PROG2 payout should be 1000"
    );
}

#[test]
fn test_unknown_recipient_returns_empty() {
    let ctx = setup();
    init_program_with_funds(&ctx, "PROG1", 1_000);

    let alice = Address::generate(&ctx.env);
    let unknown = Address::generate(&ctx.env);

    ctx.client
        .single_payout(&alice, &100, &None);

    let records = ctx.client.query_recipient_history(&unknown, &0, &100);
    assert_eq!(records.len(), 0, "unknown recipient should get empty result");
}

#[test]
fn test_batch_payout_multiple_recipients_query_isolation() {
    let ctx = setup();
    init_program_with_funds(&ctx, "PROG1", 10_000);

    let alice = Address::generate(&ctx.env);
    let bob = Address::generate(&ctx.env);

    ctx.client.batch_payout(
        &vec![&ctx.env, alice.clone(), bob.clone()],
        &vec![&ctx.env, 1000_i128, 2000_i128],
        &None,
    );

    let alice_records = ctx.client.query_recipient_history(&alice, &0, &100);
    assert_eq!(alice_records.len(), 1);
    assert_eq!(alice_records.get(0).unwrap().amount, 1000);

    let bob_records = ctx.client.query_recipient_history(&bob, &0, &100);
    assert_eq!(bob_records.len(), 1);
    assert_eq!(bob_records.get(0).unwrap().amount, 2000);
}

#[test]
fn test_query_pagination_edge_cases_with_recipient_history() {
    let ctx = setup();
    init_program_with_funds(&ctx, "PROG1", 10_000);

    let alice = Address::generate(&ctx.env);
    for i in 0..3 {
        ctx.client
            .single_payout(&alice, &(100 * (i + 1)), &None);
    }

    // offset beyond available
    let empty = ctx.client.query_recipient_history(&alice, &10, &10);
    assert_eq!(empty.len(), 0, "offset beyond count should be empty");

    // partial last page
    let last = ctx.client.query_recipient_history(&alice, &2, &10);
    assert_eq!(
        last.len(),
        1,
        "offset 2 with 3 total should return 1 record"
    );
}
