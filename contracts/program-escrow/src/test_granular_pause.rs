#![cfg(test)]

//! Exhaustive matrix tests for the three granular pause flags.
//!
//! These tests cover all eight combinations of:
//! - `lock_paused`
//! - `release_paused`
//! - `refund_paused`
//!
//! The matrix is intentionally table-driven so reviewers can verify, at a
//! glance, that each flag gates only its intended operation family:
//!
//! | lock | release | refund | lock_program_funds | single_payout | create_pending_claim | execute_claim | trigger_program_releases | cancel_claim |
//! |------|---------|--------|--------------------|---------------|----------------------|---------------|--------------------------|--------------|
//! | false | false | false | allow | allow | allow | allow | allow | allow |
//! | true  | false | false | block | allow | allow | allow | allow | allow |
//! | false | true  | false | allow | block | block | block | block | allow |
//! | false | false | true  | allow | allow | allow | allow | allow | block |
//! | true  | true  | false | block | block | block | block | block | allow |
//! | true  | false | true  | block | allow | allow | allow | allow | block |
//! | false | true  | true  | allow | block | block | block | block | block |
//! | true  | true  | true  | block | block | block | block | block | block |

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, String,
};

/// Fixed amount used by the lock-path checks.
const LOCK_AMOUNT: i128 = 100;
/// Fixed amount used by payout / claim / scheduled release checks.
const RELEASE_AMOUNT: i128 = 150;
/// Initial program liquidity for release/refund-path tests.
const INITIAL_LOCKED_BALANCE: i128 = 1_000;
/// Deterministic ledger timestamp used for claim/schedule deadlines.
const TEST_TIMESTAMP: u64 = 500;
/// Deterministic future deadline used for claim creation.
const CLAIM_DEADLINE: u64 = 5_000;
/// Deterministic past timestamp used to make a schedule due immediately.
const DUE_RELEASE_TIMESTAMP: u64 = 100;

#[derive(Copy, Clone, Debug)]
struct Flags {
    lock_paused: bool,
    release_paused: bool,
    refund_paused: bool,
}

#[derive(Copy, Clone, Debug)]
struct Expectations {
    lock_program_funds: bool,
    single_payout: bool,
    create_pending_claim: bool,
    execute_claim: bool,
    trigger_program_releases: bool,
    cancel_claim: bool,
}

#[derive(Copy, Clone, Debug)]
struct Case {
    name: &'static str,
    flags: Flags,
    expected: Expectations,
}

struct TestContext<'a> {
    client: ProgramEscrowContractClient<'a>,
    token: token::Client<'a>,
    admin: Address,
    program_id: String,
}

/// Build a published program escrow instance and optionally pre-lock funds.
///
/// # Security
/// The helper mints directly into the contract address and then locks only the
/// requested amount. This keeps each operation test isolated from unrelated
/// transfer behavior while exercising the same pause checks as production calls.
fn setup_program(env: &Env, initial_locked_balance: i128) -> TestContext<'static> {
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);

    let admin = Address::generate(env);
    client.initialize_contract(&admin);

    let payout_key = Address::generate(env);
    let token_admin = Address::generate(env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let token = token::Client::new(env, &token_address);

    let program_id = String::from_str(env, "granular-pause-matrix");
    client.init_program(
        &program_id,
        &payout_key,
        &token_address,
        &admin,
        &None,
        &None,
    );
    client.publish_program();

    if initial_locked_balance > 0 {
        mint_to_contract(env, &client, &token, initial_locked_balance);
        client.lock_program_funds(&initial_locked_balance);
    }

    TestContext {
        client,
        token,
        admin,
        program_id,
    }
}

/// Mint test tokens into the escrow contract address.
fn mint_to_contract(
    env: &Env,
    client: &ProgramEscrowContractClient<'_>,
    token_client: &token::Client<'_>,
    amount: i128,
) {
    let sac = token::StellarAssetClient::new(env, &token_client.address);
    sac.mint(&client.address, &amount);
}

/// Apply the full pause-flag tuple for a matrix case.
fn set_pause_flags(client: &ProgramEscrowContractClient<'_>, flags: Flags) {
    client.set_paused(
        &Some(flags.lock_paused),
        &Some(flags.release_paused),
        &Some(flags.refund_paused),
        &None::<String>,
    );
}

/// Assert that `get_pause_flags()` reflects the exact flag tuple under test.
fn assert_flag_round_trip(flags: Flags) {
    let env = Env::default();
    let ctx = setup_program(&env, INITIAL_LOCKED_BALANCE);

    set_pause_flags(&ctx.client, flags);

    let stored = ctx.client.get_pause_flags();
    assert_eq!(stored.lock_paused, flags.lock_paused, "lock flag mismatch");
    assert_eq!(
        stored.release_paused, flags.release_paused,
        "release flag mismatch"
    );
    assert_eq!(
        stored.refund_paused, flags.refund_paused,
        "refund flag mismatch"
    );
}

/// Return whether `lock_program_funds` succeeds under the supplied flags.
fn observe_lock_program_funds(flags: Flags) -> bool {
    let env = Env::default();
    let ctx = setup_program(&env, 0);
    mint_to_contract(&env, &ctx.client, &ctx.token, LOCK_AMOUNT);
    set_pause_flags(&ctx.client, flags);
    ctx.client.try_lock_program_funds(&LOCK_AMOUNT).is_ok()
}

/// Return whether `single_payout` succeeds under the supplied flags.
fn observe_single_payout(flags: Flags) -> bool {
    let env = Env::default();
    let ctx = setup_program(&env, INITIAL_LOCKED_BALANCE);
    let recipient = Address::generate(&env);
    set_pause_flags(&ctx.client, flags);
    ctx.client
        .try_single_payout(&recipient, &RELEASE_AMOUNT, &None)
        .is_ok()
}

/// Return whether `create_pending_claim` succeeds under the supplied flags.
fn observe_create_pending_claim(flags: Flags) -> bool {
    let env = Env::default();
    let ctx = setup_program(&env, INITIAL_LOCKED_BALANCE);
    let recipient = Address::generate(&env);
    env.ledger().set_timestamp(TEST_TIMESTAMP);
    set_pause_flags(&ctx.client, flags);
    ctx.client
        .try_create_pending_claim(
            &ctx.program_id,
            &recipient,
            &RELEASE_AMOUNT,
            &CLAIM_DEADLINE,
        )
        .is_ok()
}

/// Return whether `execute_claim` succeeds under the supplied flags.
fn observe_execute_claim(flags: Flags) -> bool {
    let env = Env::default();
    let ctx = setup_program(&env, INITIAL_LOCKED_BALANCE);
    let recipient = Address::generate(&env);
    env.ledger().set_timestamp(TEST_TIMESTAMP);
    let claim_id = ctx.client.create_pending_claim(
        &ctx.program_id,
        &recipient,
        &RELEASE_AMOUNT,
        &CLAIM_DEADLINE,
    );
    set_pause_flags(&ctx.client, flags);
    ctx.client
        .try_execute_claim(&ctx.program_id, &claim_id, &recipient)
        .is_ok()
}

/// Return whether `trigger_program_releases` succeeds under the supplied flags.
fn observe_trigger_program_releases(flags: Flags) -> bool {
    let env = Env::default();
    let ctx = setup_program(&env, INITIAL_LOCKED_BALANCE);
    let recipient = Address::generate(&env);
    env.ledger().set_timestamp(TEST_TIMESTAMP);
    ctx.client
        .create_program_release_schedule(&recipient, &RELEASE_AMOUNT, &DUE_RELEASE_TIMESTAMP);
    set_pause_flags(&ctx.client, flags);
    ctx.client.try_trigger_program_releases().is_ok()
}

/// Return whether `cancel_claim` succeeds under the supplied flags.
fn observe_cancel_claim(flags: Flags) -> bool {
    let env = Env::default();
    let ctx = setup_program(&env, INITIAL_LOCKED_BALANCE);
    let recipient = Address::generate(&env);
    env.ledger().set_timestamp(TEST_TIMESTAMP);
    let claim_id = ctx.client.create_pending_claim(
        &ctx.program_id,
        &recipient,
        &RELEASE_AMOUNT,
        &CLAIM_DEADLINE,
    );
    set_pause_flags(&ctx.client, flags);
    ctx.client
        .try_cancel_claim(&ctx.program_id, &claim_id, &ctx.admin)
        .is_ok()
}

/// Assert one full matrix row: stored flags plus each guarded operation family.
fn assert_matrix_case(case: Case) {
    assert_flag_round_trip(case.flags);

    assert_eq!(
        observe_lock_program_funds(case.flags),
        case.expected.lock_program_funds,
        "{}: lock_program_funds permission mismatch",
        case.name,
    );
    assert_eq!(
        observe_single_payout(case.flags),
        case.expected.single_payout,
        "{}: single_payout permission mismatch",
        case.name,
    );
    assert_eq!(
        observe_create_pending_claim(case.flags),
        case.expected.create_pending_claim,
        "{}: create_pending_claim permission mismatch",
        case.name,
    );
    assert_eq!(
        observe_execute_claim(case.flags),
        case.expected.execute_claim,
        "{}: execute_claim permission mismatch",
        case.name,
    );
    assert_eq!(
        observe_trigger_program_releases(case.flags),
        case.expected.trigger_program_releases,
        "{}: trigger_program_releases permission mismatch",
        case.name,
    );
    assert_eq!(
        observe_cancel_claim(case.flags),
        case.expected.cancel_claim,
        "{}: cancel_claim permission mismatch",
        case.name,
    );
}

#[test]
fn test_pause_matrix_000_none_paused() {
    assert_matrix_case(Case {
        name: "000 none paused",
        flags: Flags {
            lock_paused: false,
            release_paused: false,
            refund_paused: false,
        },
        expected: Expectations {
            lock_program_funds: true,
            single_payout: true,
            create_pending_claim: true,
            execute_claim: true,
            trigger_program_releases: true,
            cancel_claim: true,
        },
    });
}

#[test]
fn test_pause_matrix_100_lock_only_paused() {
    assert_matrix_case(Case {
        name: "100 lock only paused",
        flags: Flags {
            lock_paused: true,
            release_paused: false,
            refund_paused: false,
        },
        expected: Expectations {
            lock_program_funds: false,
            single_payout: true,
            create_pending_claim: true,
            execute_claim: true,
            trigger_program_releases: true,
            cancel_claim: true,
        },
    });
}

#[test]
fn test_pause_matrix_010_release_only_paused() {
    assert_matrix_case(Case {
        name: "010 release only paused",
        flags: Flags {
            lock_paused: false,
            release_paused: true,
            refund_paused: false,
        },
        expected: Expectations {
            lock_program_funds: true,
            single_payout: false,
            create_pending_claim: false,
            execute_claim: false,
            trigger_program_releases: false,
            cancel_claim: true,
        },
    });
}

#[test]
fn test_pause_matrix_001_refund_only_paused() {
    assert_matrix_case(Case {
        name: "001 refund only paused",
        flags: Flags {
            lock_paused: false,
            release_paused: false,
            refund_paused: true,
        },
        expected: Expectations {
            lock_program_funds: true,
            single_payout: true,
            create_pending_claim: true,
            execute_claim: true,
            trigger_program_releases: true,
            cancel_claim: false,
        },
    });
}

#[test]
fn test_pause_matrix_110_lock_and_release_paused() {
    assert_matrix_case(Case {
        name: "110 lock and release paused",
        flags: Flags {
            lock_paused: true,
            release_paused: true,
            refund_paused: false,
        },
        expected: Expectations {
            lock_program_funds: false,
            single_payout: false,
            create_pending_claim: false,
            execute_claim: false,
            trigger_program_releases: false,
            cancel_claim: true,
        },
    });
}

#[test]
fn test_pause_matrix_101_lock_and_refund_paused() {
    assert_matrix_case(Case {
        name: "101 lock and refund paused",
        flags: Flags {
            lock_paused: true,
            release_paused: false,
            refund_paused: true,
        },
        expected: Expectations {
            lock_program_funds: false,
            single_payout: true,
            create_pending_claim: true,
            execute_claim: true,
            trigger_program_releases: true,
            cancel_claim: false,
        },
    });
}

#[test]
fn test_pause_matrix_011_release_and_refund_paused() {
    assert_matrix_case(Case {
        name: "011 release and refund paused",
        flags: Flags {
            lock_paused: false,
            release_paused: true,
            refund_paused: true,
        },
        expected: Expectations {
            lock_program_funds: true,
            single_payout: false,
            create_pending_claim: false,
            execute_claim: false,
            trigger_program_releases: false,
            cancel_claim: false,
        },
    });
}

#[test]
fn test_pause_matrix_111_all_paused() {
    assert_matrix_case(Case {
        name: "111 all paused",
        flags: Flags {
            lock_paused: true,
            release_paused: true,
            refund_paused: true,
        },
        expected: Expectations {
            lock_program_funds: false,
            single_payout: false,
            create_pending_claim: false,
            execute_claim: false,
            trigger_program_releases: false,
            cancel_claim: false,
        },
    });
}

/// Verify that clearing one flag does not accidentally clear other paused flags.
#[test]
fn test_unpausing_release_does_not_unpause_lock_or_refund() {
    let env = Env::default();
    let ctx = setup_program(&env, INITIAL_LOCKED_BALANCE);
    env.ledger().set_timestamp(TEST_TIMESTAMP);

    ctx.client
        .set_paused(&Some(true), &Some(true), &Some(true), &None::<String>);
    ctx.client
        .set_paused(&None, &Some(false), &None, &None::<String>);

    let stored = ctx.client.get_pause_flags();
    assert!(stored.lock_paused, "lock pause should remain enabled");
    assert!(
        !stored.release_paused,
        "release pause should be the only flag cleared"
    );
    assert!(stored.refund_paused, "refund pause should remain enabled");

    mint_to_contract(&env, &ctx.client, &ctx.token, LOCK_AMOUNT);
    assert!(
        ctx.client.try_lock_program_funds(&LOCK_AMOUNT).is_err(),
        "lock should remain blocked while only release is unpaused"
    );

    let payout_recipient = Address::generate(&env);
    assert!(
        ctx.client
            .try_single_payout(&payout_recipient, &RELEASE_AMOUNT, &None)
            .is_ok(),
        "single payout should be restored when release is unpaused"
    );

    let claim_recipient = Address::generate(&env);
    assert!(
        ctx.client
            .try_create_pending_claim(
                &ctx.program_id,
                &claim_recipient,
                &RELEASE_AMOUNT,
                &CLAIM_DEADLINE,
            )
            .is_ok(),
        "claim creation should be restored when release is unpaused"
    );

    let execute_recipient = Address::generate(&env);
    let execute_claim_id = ctx.client.create_pending_claim(
        &ctx.program_id,
        &execute_recipient,
        &RELEASE_AMOUNT,
        &CLAIM_DEADLINE,
    );
    assert!(
        ctx.client
            .try_execute_claim(&ctx.program_id, &execute_claim_id, &execute_recipient)
            .is_ok(),
        "claim execution should be restored when release is unpaused"
    );

    let refund_recipient = Address::generate(&env);
    let refund_claim_id = ctx.client.create_pending_claim(
        &ctx.program_id,
        &refund_recipient,
        &RELEASE_AMOUNT,
        &CLAIM_DEADLINE,
    );
    assert!(
        ctx.client
            .try_cancel_claim(&ctx.program_id, &refund_claim_id, &ctx.admin)
            .is_err(),
        "refund should remain blocked while refund_paused stays enabled"
    );
}
