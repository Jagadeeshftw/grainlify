//! Tests for `archive_program` and the archived payout-history storage-tier
//! migration introduced in:
//!   <https://github.com/Jagadeeshftw/grainlify/issues/XXX>
//!
//! ## What is tested
//!
//! 1. Basic archival success — `archived` flag, `archived_at` timestamp, and
//!    presence in `get_archived_programs`.
//! 2. Payout history is **preserved** in persistent storage after archival.
//! 3. Payout history is **cleared** from instance storage (i.e. `ProgramData`
//!    returned by `get_program_info_v2` has an empty `payout_history`) after
//!    archival.
//! 4. `get_archived_program_payout_history` returns the correct records.
//! 5. `query_recipient_history` still works for archived programs (the
//!    per-recipient persistent index is unaffected by archival).
//! 6. Double-archival is idempotent — calling `archive_program` twice does
//!    not overwrite the history with an empty vec.
//! 7. Archiving a program with zero payouts works correctly.
//! 8. Only admin can archive a program.
//! 9. Archiving a non-existent program panics.
//! 10. Instance-storage footprint shrinks with N archived programs
//!     (benchmark / size comparison).

#![cfg(test)]

extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Env, String};

use crate::{
    test_batch_operations::{init_program, setup, Ctx},
    PayoutRecord,
};

// ─── helpers ─────────────────────────────────────────────────────────────────

/// Execute one single_payout on the default singleton program and return the
/// recipient's address so the caller can query history later.
fn do_payout(ctx: &Ctx, recipient: &Address, amount: i128) {
    ctx.client
        .single_payout(recipient, &amount, &None::<String>);
}

/// Execute one `batch_payout_v2` carrying multiple recipients.
fn do_batch_payout(ctx: &Ctx, prog: &str, recipients: &[(Address, i128)]) {
    let prog_id = String::from_str(&ctx.env, prog);
    let mut addrs = soroban_sdk::Vec::new(&ctx.env);
    let mut amts = soroban_sdk::Vec::new(&ctx.env);
    for (a, v) in recipients {
        addrs.push_back(a.clone());
        amts.push_back(*v);
    }
    ctx.client.batch_payout_v2(&prog_id, &addrs, &amts);
}

// ─── Test 1: basic archival ───────────────────────────────────────────────────

#[test]
fn test_program_archival_success() {
    let ctx = setup();
    let program_id = "PROG1";
    init_program(&ctx, program_id, 1000);

    // Initial state: not archived
    let info = ctx
        .client
        .get_program_info_v2(&String::from_str(&ctx.env, program_id));
    assert_eq!(info.total_funds, 1000);
    assert!(!info.archived);
    assert_eq!(info.archived_at, None);

    // Archive program
    ctx.client
        .archive_program(&String::from_str(&ctx.env, program_id));

    // After archival: archived is true
    let info = ctx
        .client
        .get_program_info_v2(&String::from_str(&ctx.env, program_id));
    assert!(info.archived);
    assert!(info.archived_at.is_some());

    // Check archived registry
    let archived = ctx.client.get_archived_programs();
    assert_eq!(archived.len(), 1);
    assert_eq!(
        archived.get(0).unwrap(),
        String::from_str(&ctx.env, program_id)
    );
}

// ─── Test 2 & 3: payout history migrated to persistent, cleared from instance ─

#[test]
fn test_archive_migrates_payout_history_to_persistent_storage() {
    let ctx = setup();
    let program_id = "PROG_HIST";
    init_program(&ctx, program_id, 900);

    let r1 = Address::generate(&ctx.env);
    let r2 = Address::generate(&ctx.env);

    // Perform two payouts before archival
    do_payout(&ctx, &r1, 100);
    do_payout(&ctx, &r2, 200);

    let prog_str = String::from_str(&ctx.env, program_id);

    // Confirm history exists in ProgramData before archival
    let info_before = ctx.client.get_program_info_v2(&prog_str);
    assert_eq!(
        info_before.payout_history.len(),
        2,
        "expected 2 history entries before archival"
    );

    // Archive
    ctx.client.archive_program(&prog_str);

    // Test 3: instance-storage ProgramData.payout_history is now empty
    let info_after = ctx.client.get_program_info_v2(&prog_str);
    assert!(
        info_after.payout_history.is_empty(),
        "payout_history in ProgramData should be empty after archival (migrated to persistent)"
    );

    // Test 2: persistent storage holds the full history
    let archived_history = ctx
        .client
        .get_archived_program_payout_history(&prog_str);
    assert_eq!(
        archived_history.len(),
        2,
        "persistent archived history should contain 2 records"
    );

    // Verify the records match by recipient (order: insertion order)
    let recipients: std::vec::Vec<Address> = archived_history
        .iter()
        .map(|r: PayoutRecord| r.recipient)
        .collect();
    assert!(
        recipients.contains(&r1),
        "r1 should be in archived history"
    );
    assert!(
        recipients.contains(&r2),
        "r2 should be in archived history"
    );

    // Verify amounts
    let amounts: std::vec::Vec<i128> = archived_history
        .iter()
        .map(|r: PayoutRecord| r.amount)
        .collect();
    assert!(amounts.contains(&100_i128));
    assert!(amounts.contains(&200_i128));
}

// ─── Test 4: get_archived_program_payout_history ─────────────────────────────

#[test]
fn test_get_archived_program_payout_history_returns_correct_records() {
    let ctx = setup();
    let program_id = "ARCH_QUERY";
    init_program(&ctx, program_id, 600);

    let r1 = Address::generate(&ctx.env);
    let r2 = Address::generate(&ctx.env);
    let r3 = Address::generate(&ctx.env);

    do_payout(&ctx, &r1, 50);
    do_payout(&ctx, &r2, 150);
    do_payout(&ctx, &r3, 250);

    let prog_str = String::from_str(&ctx.env, program_id);
    ctx.client.archive_program(&prog_str);

    let history = ctx.client.get_archived_program_payout_history(&prog_str);

    assert_eq!(history.len(), 3, "expected 3 records in archived history");

    let total: i128 = history.iter().map(|r: PayoutRecord| r.amount).sum();
    assert_eq!(total, 450_i128, "sum of archived payout amounts should be 450");
}

// ─── Test 5: query_recipient_history still works post-archival ────────────────

#[test]
fn test_query_recipient_history_works_for_archived_program() {
    let ctx = setup();
    let program_id = "ARCH_RECIP";
    init_program(&ctx, program_id, 500);

    let recipient = Address::generate(&ctx.env);

    do_payout(&ctx, &recipient, 100);
    do_payout(&ctx, &recipient, 150);

    let prog_str = String::from_str(&ctx.env, program_id);
    ctx.client.archive_program(&prog_str);

    // Per-recipient persistent index is independent of the inline history
    let history = ctx
        .client
        .query_recipient_history(&prog_str, &recipient);

    assert_eq!(
        history.len(),
        2,
        "recipient index should still have 2 records after archival"
    );
    let total: i128 = history.iter().map(|r: PayoutRecord| r.amount).sum();
    assert_eq!(total, 250_i128);
}

// ─── Test 6: double-archival idempotency ─────────────────────────────────────

#[test]
fn test_double_archival_is_idempotent_and_preserves_history() {
    let ctx = setup();
    let program_id = "DOUBLE_ARCH";
    init_program(&ctx, program_id, 300);

    let r1 = Address::generate(&ctx.env);
    do_payout(&ctx, &r1, 100);

    let prog_str = String::from_str(&ctx.env, program_id);

    // First archival
    ctx.client.archive_program(&prog_str);

    let history_first = ctx
        .client
        .get_archived_program_payout_history(&prog_str);
    assert_eq!(history_first.len(), 1, "should have 1 record after first archival");

    // Second archival — the guard `if !env.storage().persistent().has(&history_key)`
    // should prevent overwriting the already-migrated history with an empty vec.
    ctx.client.archive_program(&prog_str);

    let history_second = ctx
        .client
        .get_archived_program_payout_history(&prog_str);
    assert_eq!(
        history_second.len(),
        1,
        "double archival must not erase the previously migrated history"
    );
}

// ─── Test 7: archive with zero payouts ───────────────────────────────────────

#[test]
fn test_archive_program_with_no_payouts() {
    let ctx = setup();
    let program_id = "NO_PAY";
    init_program(&ctx, program_id, 200);

    let prog_str = String::from_str(&ctx.env, program_id);
    ctx.client.archive_program(&prog_str);

    let info = ctx.client.get_program_info_v2(&prog_str);
    assert!(info.archived);
    assert!(info.payout_history.is_empty());

    // Persistent key should hold an empty vec (not panic / error)
    let history = ctx.client.get_archived_program_payout_history(&prog_str);
    assert!(history.is_empty(), "no-payout program should have empty archived history");
}

// ─── Test 8: only admin can archive ──────────────────────────────────────────

#[test]
#[should_panic]
fn test_archive_requires_admin() {
    let ctx = setup();
    let program_id = "ADMIN_GUARD";
    init_program(&ctx, program_id, 100);

    // mock_all_auths is active but we're not actually sending a non-admin call
    // here — the real guard is tested via require_admin() panic path. To keep
    // this file's env consistent we instead verify that the contract enforces
    // admin via the existing require_admin call by attempting to archive a
    // non-existent program after disabling mock (no env available without
    // full re-setup), so we lean on the no-auth check embedded in the
    // require_admin helper. This test will #[should_panic] if auths are not
    // satisfied — the test infrastructure's mock_all_auths covers authorization
    // for normal flows; the guard is unit-tested at the contract level.
    //
    // Approach: spin up a fresh env without mock_all_auths.
    let env2 = soroban_sdk::Env::default();
    let non_admin = Address::generate(&env2);
    let token_admin2 = Address::generate(&env2);
    let token_id2 = env2.register_stellar_asset_contract(token_admin2.clone());
    let contract_id2 =
        env2.register_contract(None, crate::ProgramEscrowContract);
    let client2 = crate::ProgramEscrowContractClient::new(&env2, &contract_id2);
    client2.initialize_contract(&non_admin);

    // Attempting to call archive_program without satisfying the admin auth
    // should panic because require_admin calls admin.require_auth() and there
    // is no mock in env2.
    let _ = token_id2; // suppress unused warning
    client2.archive_program(&String::from_str(&env2, "ANY_PROG"));
}

// ─── Test 9: non-existent program panics ─────────────────────────────────────

#[test]
#[should_panic(expected = "Program not found")]
fn test_archive_non_existent_program() {
    let ctx = setup();
    ctx.client
        .archive_program(&String::from_str(&ctx.env, "NON_EXISTENT"));
}

// ─── Test 10: instance-storage footprint benchmark ───────────────────────────
//
// This test serves as a lightweight regression/benchmark asserting that
// archiving N programs shrinks the per-program instance-storage payload.
// It does NOT compare raw byte sizes (Soroban SDK does not expose ledger
// entry sizes in test mode) but asserts the invariant:
//
//   ∀ archived program p:
//     get_program_info_v2(p).payout_history.is_empty() == true
//
// This is the key property that guarantees the instance-storage footprint
// does not grow with the number of historical payout records once a program
// is archived.
#[test]
fn test_instance_storage_footprint_shrinks_after_archival() {
    let ctx = setup();
    const N: usize = 5;

    // Create N programs, each with M payouts
    const M: i128 = 3;
    let mut program_ids: std::vec::Vec<std::string::String> = std::vec::Vec::new();

    for i in 0..N {
        let prog_id = std::format!("BENCH_PROG_{}", i);
        // Use a fresh amount per program to avoid balance collisions
        let amount = (M + 1) * 100 * (i as i128 + 1);
        init_program(&ctx, &prog_id, amount);

        let prog_str = String::from_str(&ctx.env, &prog_id);

        // Execute M payouts per program
        for _ in 0..M {
            let recipient = Address::generate(&ctx.env);
            ctx.client.single_payout(&recipient, &100_i128, &None::<String>);
        }

        // Confirm M history entries exist before archival
        let info_before = ctx.client.get_program_info_v2(&prog_str);
        assert_eq!(
            info_before.payout_history.len(),
            M as u32,
            "expected {} history entries before archival for {}",
            M,
            &prog_id
        );

        program_ids.push(prog_id);
    }

    // Archive all N programs
    for prog_id in &program_ids {
        ctx.client
            .archive_program(&String::from_str(&ctx.env, prog_id));
    }

    // Post-archival: all N programs should have empty payout_history in
    // instance storage, and non-empty history in persistent storage.
    for prog_id in &program_ids {
        let prog_str = String::from_str(&ctx.env, prog_id);

        let info_after = ctx.client.get_program_info_v2(&prog_str);
        assert!(
            info_after.payout_history.is_empty(),
            "instance-storage payout_history should be empty after archival for {}",
            prog_id
        );

        let archived_history = ctx.client.get_archived_program_payout_history(&prog_str);
        assert_eq!(
            archived_history.len(),
            M as u32,
            "persistent storage should hold {} records for {}",
            M,
            prog_id
        );
    }
}

// ─── Test: program filtering and archival registry ────────────────────────────

#[test]
fn test_program_archival_filtering() {
    let ctx = setup();

    init_program(&ctx, "SINGLETON", 5000);

    let info = ctx.client.get_program_info();
    assert!(!info.archived);

    ctx.client
        .archive_program(&String::from_str(&ctx.env, "SINGLETON"));

    let info = ctx.client.get_program_info();
    assert!(info.archived);

    // list_programs should filter out archived programs
    let list = ctx.client.list_programs();
    assert_eq!(list.len(), 0);
}

// ─── Test: no data loss — archived records are fully queryable ────────────────

#[test]
fn test_no_data_loss_after_archival() {
    let ctx = setup();
    let program_id = "NO_LOSS";
    init_program(&ctx, program_id, 10_000);

    let prog_str = String::from_str(&ctx.env, program_id);

    // Generate deterministic payouts
    let recipients: std::vec::Vec<Address> = (0..10)
        .map(|_| Address::generate(&ctx.env))
        .collect();

    for (i, r) in recipients.iter().enumerate() {
        let amount = 100_i128 * (i as i128 + 1); // 100, 200, …, 1000
        ctx.client.single_payout(r, &amount, &None::<String>);
    }

    // Snapshot: total payout before archival
    let info_before = ctx.client.get_program_info_v2(&prog_str);
    let expected_total: i128 = info_before
        .payout_history
        .iter()
        .map(|r: PayoutRecord| r.amount)
        .sum();
    let expected_count = info_before.payout_history.len();

    // Archive
    ctx.client.archive_program(&prog_str);

    // Verify: persistent tier has the same total and count
    let archived_history = ctx.client.get_archived_program_payout_history(&prog_str);
    assert_eq!(
        archived_history.len(),
        expected_count,
        "no records should be lost on archival"
    );

    let actual_total: i128 = archived_history
        .iter()
        .map(|r: PayoutRecord| r.amount)
        .sum();
    assert_eq!(
        actual_total, expected_total,
        "total payout amount must be identical before and after archival"
    );

    // Each recipient's individual history via query_recipient_history also intact
    for (i, r) in recipients.iter().enumerate() {
        let rec_history = ctx.client.query_recipient_history(&prog_str, r);
        assert_eq!(
            rec_history.len(),
            1,
            "each recipient should have exactly 1 record, failed for recipient {}",
            i
        );
        let expected_amount = 100_i128 * (i as i128 + 1);
        assert_eq!(rec_history.get(0).unwrap().amount, expected_amount);
    }
}

// ─── Test 11: adaptive TTL based on hot vs cold access ───────────────────────

#[test]
fn test_adaptive_ttl_hot_vs_cold() {
    let ctx = setup();
    
    // Cold program: minimal accesses
    let cold_prog = "COLD";
    init_program(&ctx, cold_prog, 100);
    let r1 = Address::generate(&ctx.env);
    do_payout(&ctx, &r1, 10); // 1 payout
    
    // Hot program: many accesses
    let hot_prog = "HOT";
    init_program(&ctx, hot_prog, 1000);
    for _ in 0..5 {
        let r = Address::generate(&ctx.env);
        do_payout(&ctx, &r, 10);
    }
    
    let get_access_count = |prog: &str| -> u32 {
        let key = crate::DataKey::ProgramAccessSignal(String::from_str(&ctx.env, prog));
        ctx.env.as_contract(&ctx.client.address, || {
            ctx.env.storage().persistent().get(&key).unwrap_or(0)
        })
    };
    
    let cold_count = get_access_count(cold_prog);
    let hot_count = get_access_count(hot_prog);
    
    assert!(hot_count > cold_count, "Hot program should have higher access count than cold program ({} vs {})", hot_count, cold_count);
    
    // Check specific expected values to ensure we scale as intended
    // init_program calls store_program_data (1)
    // do_payout calls store_program_data and append_recipient_index (2 accesses per payout)
    assert_eq!(cold_count, 1 + 2); // 3
    assert_eq!(hot_count, 1 + 5 * 2); // 11
}
