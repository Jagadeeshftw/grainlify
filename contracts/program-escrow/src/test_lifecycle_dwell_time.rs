//! # Lifecycle Dwell-Time Tests
//!
//! Verifies that the contract correctly records per-status transition
//! timestamps via [`record_status_transition`] and exposes them through
//! [`get_program_lifecycle_timeline`].
//!
//! ## Coverage goals
//! - A program that cycles Draft → Active records both transitions.
//! - Dwell times (time spent in each status) can be inferred from
//!   consecutive transition timestamps.
//! - Programs created via `init_program` and `batch_initialize_programs`
//!   both start with the initial Draft→Draft entry.
//! - Calling `get_program_lifecycle_timeline` for a non-existent or
//!   legacy program returns an empty vec (no panic).
//! - Publishing an already‑published program panics and does **not** write
//!   a spurious transition.

use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token, vec, Address, Env, String, Vec,
};

use crate::{
    ProgramEscrowContract, ProgramEscrowContractClient, ProgramStatus, StatusTransition,
};

// -----------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------

fn setup() -> (Env, Address, ProgramEscrowContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, admin, client)
}

fn make_token(env: &Env, admin: &Address) -> Address {
    let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
    token_contract.address()
}

fn pid(env: &Env, s: &str) -> String {
    String::from_str(env, s)
}

/// Advance the ledger timestamp by `delta` seconds so we can observe
/// non‑zero dwell times between transitions.
fn advance_time(env: &Env, delta: u64) {
    let old = env.ledger().timestamp();
    env.ledger().set_timestamp(old + delta);
}

/// Assert that the timeline contains exactly one transition whose
/// `to_status` is `expected`, with the given `expected_timestamp`.
/// The initial transition is always Draft → Draft, recording the moment
/// the program was created (there is no "Created" / "Null" variant in
/// [`ProgramStatus`]).
fn assert_single_transition(
    timeline: &Vec<StatusTransition>,
    expected: ProgramStatus,
    expected_timestamp: u64,
) {
    assert_eq!(
        timeline.len(),
        1,
        "expected exactly 1 transition, got {}",
        timeline.len()
    );
    let t = timeline.get(0).unwrap();
    assert_eq!(t.to_status, expected, "to_status mismatch");
    assert_eq!(t.timestamp, expected_timestamp, "timestamp mismatch");
    // from_status should be ProgramStatus::Draft for the initial entry
    assert_eq!(
        t.from_status,
        ProgramStatus::Draft,
        "initial from_status should be Draft"
    );
}

/// Return the number of seconds between two consecutive transitions
/// (index `i` and `i+1`).  Panics if `i+1` is out of bounds.
fn dwell_seconds(timeline: &Vec<StatusTransition>, i: u32) -> u64 {
    let from = timeline.get(i).unwrap();
    let to = timeline.get(i + 1).unwrap();
    to.timestamp - from.timestamp
}

// -----------------------------------------------------------------------
// Tests: init_program → nothing recorded yet?  Actually we record an
// initial Draft→Draft entry so the start time is always visible.
// -----------------------------------------------------------------------

#[test]
fn test_initial_transition_recorded_on_init() {
    let (env, admin, client) = setup();
    let token = make_token(&env, &admin);

    let t0 = env.ledger().timestamp();
    client.init_program(
        &pid(&env, "p-001"),
        &admin,
        &token,
        &admin,
        &None,
        &None,
    );

    let timeline = client.get_program_lifecycle_timeline(&pid(&env, "p-001"));
    assert_single_transition(&timeline, ProgramStatus::Draft, t0);
}

#[test]
fn test_publish_records_draft_to_active_transition() {
    let (env, admin, client) = setup();
    let token = make_token(&env, &admin);

    let t0 = env.ledger().timestamp();
    client.init_program(
        &pid(&env, "p-001"),
        &admin,
        &token,
        &admin,
        &None,
        &None,
    );

    // let dwell time pass
    advance_time(&env, 3600); // 1 hour
    let t1 = env.ledger().timestamp();
    client.publish_program(&pid(&env, "p-001"), &admin);

    let timeline = client.get_program_lifecycle_timeline(&pid(&env, "p-001"));
    assert_eq!(timeline.len(), 2, "expected 2 transitions after publish");

    // Check Draft transition
    let draft_t = timeline.get(0).unwrap();
    assert_eq!(draft_t.from_status, ProgramStatus::Draft);
    assert_eq!(draft_t.to_status, ProgramStatus::Draft);
    assert_eq!(draft_t.timestamp, t0);

    // Check Active transition
    let active_t = timeline.get(1).unwrap();
    assert_eq!(active_t.from_status, ProgramStatus::Draft);
    assert_eq!(active_t.to_status, ProgramStatus::Active);
    assert_eq!(active_t.timestamp, t1);

    // dwell time in Draft
    assert_eq!(dwell_seconds(&timeline, 0), 3600);
}

#[test]
fn test_full_lifecycle_draft_to_active_dwell_times() {
    let (env, admin, client) = setup();
    let token = make_token(&env, &admin);

    // Start at t=0
    env.ledger().set_timestamp(1000);
    client.init_program(
        &pid(&env, "lifecycle-prog"),
        &admin,
        &token,
        &admin,
        &None,
        &None,
    );

    // Spend 2 days in Draft
    advance_time(&env, 172_800); // 2 days
    client.publish_program(&pid(&env, "lifecycle-prog"), &admin);

    let timeline = client.get_program_lifecycle_timeline(&pid(&env, "lifecycle-prog"));
    assert_eq!(timeline.len(), 2);

    // Dwell in Draft
    assert_eq!(
        dwell_seconds(&timeline, 0),
        172_800,
        "should have spent 2 days in Draft"
    );

    // Active dwell: not applicable yet since no further transition,
    // but the fields are there for querying.
    let active_entry = timeline.get(1).unwrap();
    assert_eq!(active_entry.to_status, ProgramStatus::Active);
    assert_eq!(active_entry.timestamp, 1000 + 172_800);
}

// -----------------------------------------------------------------------
// Tests: batch_initialize_programs
// -----------------------------------------------------------------------

#[test]
fn test_batch_init_records_transitions() {
    let (env, admin, client) = setup();
    let token = make_token(&env, &admin);

    let pid1 = pid(&env, "batch-1");
    let pid2 = pid(&env, "batch-2");

    let t0 = env.ledger().timestamp();

    // Use batch_initialize_programs to create two programs at once
    let items = soroban_sdk::vec![
        &env,
        crate::ProgramInitItem {
            program_id: pid1.clone(),
            authorized_payout_key: admin.clone(),
            token_address: token.clone(),
            reference_hash: None,
        },
        crate::ProgramInitItem {
            program_id: pid2.clone(),
            authorized_payout_key: admin.clone(),
            token_address: token.clone(),
            reference_hash: None,
        },
    ];
    let count = client.batch_initialize_programs(&items).unwrap();
    assert_eq!(count, 2, "expected 2 programs to be initialized");

    let tl1 = client.get_program_lifecycle_timeline(&pid1);
    let tl2 = client.get_program_lifecycle_timeline(&pid2);

    assert_single_transition(&tl1, ProgramStatus::Draft, t0);
    assert_single_transition(&tl2, ProgramStatus::Draft, t0);
}

// -----------------------------------------------------------------------
// Tests: non-existent / legacy program (no timeline stored)
// -----------------------------------------------------------------------

#[test]
fn test_non_existent_program_returns_empty() {
    let (env, _admin, client) = setup();
    let timeline = client.get_program_lifecycle_timeline(&pid(&env, "ghost"));
    assert_eq!(
        timeline.len(),
        0,
        "non‑existent program should return empty timeline"
    );
}



// -----------------------------------------------------------------------
// Tests: a program that is never published still has its initial
// Draft→Draft entry.
// -----------------------------------------------------------------------

#[test]
fn test_never_published_still_has_initial_entry() {
    let (env, admin, client) = setup();
    let token = make_token(&env, &admin);

    let t0 = env.ledger().timestamp();
    client.init_program(
        &pid(&env, "p-001"),
        &admin,
        &token,
        &admin,
        &None,
        &None,
    );

    // Advance time significantly to prove we don't get a "transition" from
    // just the passage of time — the timeline should still have 1 entry.
    advance_time(&env, 1_000_000);
    let timeline = client.get_program_lifecycle_timeline(&pid(&env, "p-001"));
    assert_eq!(timeline.len(), 1, "unpublished program should have 1 entry");
    assert_eq!(timeline.get(0).unwrap().timestamp, t0);
    assert_eq!(timeline.get(0).unwrap().to_status, ProgramStatus::Draft);
}

// -----------------------------------------------------------------------
// Tests: verify the timeline is *not* modified when publish fails.
// -----------------------------------------------------------------------

#[test]
fn test_failed_publish_does_not_record_transition() {
    let (env, admin, client) = setup();
    let token = make_token(&env, &admin);

    client.init_program(
        &pid(&env, "p-001"),
        &admin,
        &token,
        &admin,
        &None,
        &None,
    );

    // Capture timeline length before the failing double-publish
    let len_before = client
        .get_program_lifecycle_timeline(&pid(&env, "p-001"))
        .len();

    // Publish once, successfully
    client.publish_program(&pid(&env, "p-001"), &admin);
    assert_eq!(
        client
            .get_program_lifecycle_timeline(&pid(&env, "p-001"))
            .len(),
        len_before + 1,
        "successful publish should add exactly one transition"
    );

    let len_after_first_pub = client
        .get_program_lifecycle_timeline(&pid(&env, "p-001"))
        .len();

    // Try to publish again — should panic
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.publish_program(&pid(&env, "p-001"), &admin);
    }));
    assert!(result.is_err(), "double publish should panic");

    // Timeline should be unchanged
    assert_eq!(
        client
            .get_program_lifecycle_timeline(&pid(&env, "p-001"))
            .len(),
        len_after_first_pub,
        "failed publish must not add a transition"
    );
}

// -----------------------------------------------------------------------
// Tests: StatusTransition field correctness
// -----------------------------------------------------------------------

#[test]
fn test_status_transition_struct_fields() {
    let transition = StatusTransition {
        from_status: ProgramStatus::Draft,
        to_status: ProgramStatus::Active,
        timestamp: 12345,
    };
    assert_eq!(transition.from_status, ProgramStatus::Draft);
    assert_eq!(transition.to_status, ProgramStatus::Active);
    assert_eq!(transition.timestamp, 12345);
}

// -----------------------------------------------------------------------
// Tests: multiple programs have independent timelines
// -----------------------------------------------------------------------

#[test]
fn test_independent_timelines_per_program() {
    let (env, admin, client) = setup();
    let token = make_token(&env, &admin);

    let pid_a = pid(&env, "prog-a");
    let pid_b = pid(&env, "prog-b");

    env.ledger().set_timestamp(1000);
    client.init_program(&pid_a, &admin, &token, &admin, &None, &None);

    advance_time(&env, 100);
    client.init_program(&pid_b, &admin, &token, &admin, &None, &None);

    // Publish A after another delay
    advance_time(&env, 500);
    client.publish_program(&pid_a, &admin);

    let tl_a = client.get_program_lifecycle_timeline(&pid_a);
    let tl_b = client.get_program_lifecycle_timeline(&pid_b);

    // A has 2 transitions, B has 1
    assert_eq!(tl_a.len(), 2, "program A should have 2 transitions");
    assert_eq!(tl_b.len(), 1, "program B should have 1 transition");

    // A's first transition at t=1000
    assert_eq!(tl_a.get(0).unwrap().timestamp, 1000);
    // A's second (publish) at t=1000+100+500 = 1600
    assert_eq!(tl_a.get(1).unwrap().timestamp, 1600);

    // B's only transition at t=1100
    assert_eq!(tl_b.get(0).unwrap().timestamp, 1100);
}
