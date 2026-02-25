// ============================================================
// FILE: contracts/program-escrow/src/test_claim_period_expiry_cancellation.rs
//
// Issue #480 — Tests for claim period expiry and cancellation
// Closes #480
//
// Timing assumptions:
//
// - Ledger timestamps are `u64` seconds since Unix epoch
// - `env.ledger().set()` is used to simulate time progression
// - Default claim window: 86,400 seconds (24 hours)
// - A claim is considered expired when:
//     env.ledger().timestamp() > claim.claim_deadline
//
// ============================================================

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token, Address, Env, String,
};

use crate::{
    ClaimRecord, ClaimStatus, DataKey, ProgramEscrowContract, ProgramEscrowContractClient,
};

fn create_token_contract<'a>(
    env: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    (
        token::Client::new(env, &sac.address()),
        token::StellarAssetClient::new(env, &sac.address()),
    )
}

struct TestSetup<'a> {
    env: Env,
    client: ProgramEscrowContractClient<'a>,
    token: token::Client<'a>,
    token_admin: token::StellarAssetClient<'a>,
    admin: Address,
    payout_key: Address,
    contributor: Address,
    program_id: String,
}

fn setup<'a>() -> TestSetup<'a> {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let payout_key = Address::generate(&env);
    let contributor = Address::generate(&env);

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let (token, token_admin) = create_token_contract(&env, &admin);

    token_admin.mint(&contract_id, &1_000_000_i128);

    let program_id = String::from_str(&env, "TestProgram2024");

    // initialize program
    client.init_program(&program_id, &payout_key, &token.address, &payout_key, &None);

    // lock funds
    client.lock_program_funds(&500_000_i128);

    client.set_admin(&admin);

    //  ledger timestamp
    env.ledger().set(LedgerInfo {
        timestamp: 1_000_000,
        protocol_version: 22,
        sequence_number: 10,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 1000,
        min_persistent_entry_ttl: 1000,
        max_entry_ttl: 3110400,
    });

    TestSetup {
        env,
        client,
        token,
        token_admin,
        admin,
        payout_key,
        contributor,
        program_id,
    }
}

#[test]
fn test_claim_within_window_succeeds() {
    let t = setup();
    let env = &t.env;

    let now: u64 = env.ledger().timestamp();
    let claim_amount: i128 = 10_000;
    let claim_deadline: u64 = now + 86_400; // 24 hours

    let claim_id = t.client.create_pending_claim(
        &t.program_id,
        &t.contributor,
        &claim_amount,
        &claim_deadline,
    );

    // verify if  claim is in it pending state
    let claim = t.client.get_claim(&t.program_id, &claim_id);
    assert_eq!(
        claim.status,
        ClaimStatus::Pending,
        "Claim should be Pending"
    );
    assert_eq!(claim.amount, claim_amount);
    assert_eq!(claim.recipient, t.contributor);

    let balance_before = t.token.balance(&t.contributor);

    // Contributor claims well within the time frame of 6 hours later
    env.ledger().set(LedgerInfo {
        timestamp: now + 21_600,
        ..env.ledger().get()
    });

    t.client
        .execute_claim(&t.program_id, &claim_id, &t.contributor);

    let balance_after = t.token.balance(&t.contributor);
    assert_eq!(
        balance_after - balance_before,
        claim_amount,
        "Contributor should have received exactly the claim amount"
    );

    // assert claim Completed
    let claim = t.client.get_claim(&t.program_id, &claim_id);
    assert_eq!(
        claim.status,
        ClaimStatus::Completed,
        "Claim should be Completed"
    );

    // assert escrow balance decreased
    let program = t.client.get_program_info();
    assert_eq!(program.remaining_balance, 500_000 - claim_amount);
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 2: Claim attempt after expiry should fail
// ═══════════════════════════════════════════════════════════════════════════

#[test]
#[should_panic(expected = "ClaimExpired")]
fn test_claim_after_expiry_fails() {
    let t = setup();
    let env = &t.env;

    let now: u64 = env.ledger().timestamp();
    let claim_amount: i128 = 5_000;
    let claim_deadline: u64 = now + 3_600; // 1 hour timeframe

    let claim_id = t.client.create_pending_claim(
        &t.program_id,
        &t.contributor,
        &claim_amount,
        &claim_deadline,
    );

    // advance time PAST the deadline (2 hours later)
    env.ledger().set(LedgerInfo {
        timestamp: now + 7_200,
        ..env.ledger().get()
    });

    // verifies claim is still Pending — nothing auto-cancels it
    let claim = t.client.get_claim(&t.program_id, &claim_id);
    assert_eq!(claim.status, ClaimStatus::Pending);

    // panics with "ClaimExpired"
    t.client
        .execute_claim(&t.program_id, &claim_id, &t.contributor);
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 3: Admin cancels a pending (active) claim — funds return to escrow
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_admin_cancel_pending_claim_restores_escrow() {
    let t = setup();
    let env = &t.env;

    let now: u64 = env.ledger().timestamp();
    let claim_amount: i128 = 8_000;
    let claim_deadline: u64 = now + 86_400;

    let claim_id = t.client.create_pending_claim(
        &t.program_id,
        &t.contributor,
        &claim_amount,
        &claim_deadline,
    );

    // Escrow balance should have decreased when claim was created (reserved)
    let balance_after_create = t.client.get_remaining_balance();

    // Admin cancels the still-active pending claim (well within deadline)
    env.ledger().set(LedgerInfo {
        timestamp: now + 1_800, // 30 minutes in — still active
        ..env.ledger().get()
    });

    t.client.cancel_claim(&t.program_id, &claim_id, &t.admin);

    // Assert funds returned to escrow
    let balance_after_cancel = t.client.get_remaining_balance();
    assert_eq!(
        balance_after_cancel,
        balance_after_create + claim_amount,
        "Funds should be returned to escrow after cancellation"
    );

    // Assert claim status is Cancelled
    let claim = t.client.get_claim(&t.program_id, &claim_id);
    assert_eq!(
        claim.status,
        ClaimStatus::Cancelled,
        "Claim should be Cancelled"
    );

    // Assert contributor received nothing
    assert_eq!(
        t.token.balance(&t.contributor),
        0,
        "Contributor should have received nothing after cancel"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 4: Admin cancels an already-expired claim
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_admin_cancel_expired_claim_succeeds() {
    let t = setup();
    let env = &t.env;

    let now: u64 = env.ledger().timestamp();
    let claim_amount: i128 = 3_000;
    let claim_deadline: u64 = now + 3_600;

    let claim_id = t.client.create_pending_claim(
        &t.program_id,
        &t.contributor,
        &claim_amount,
        &claim_deadline,
    );

    // Time passes — claim window expires without contributor acting
    env.ledger().set(LedgerInfo {
        timestamp: now + 7_200, // 2 hours later
        ..env.ledger().get()
    });

    let balance_before_cancel = t.client.get_remaining_balance();

    // Admin cleans up the expired claim
    t.client.cancel_claim(&t.program_id, &claim_id, &t.admin);

    // Funds should return to escrow
    let balance_after_cancel = t.client.get_remaining_balance();
    assert_eq!(
        balance_after_cancel,
        balance_before_cancel + claim_amount,
        "Expired claim cancellation should restore funds to escrow"
    );

    let claim = t.client.get_claim(&t.program_id, &claim_id);
    assert_eq!(
        claim.status,
        ClaimStatus::Cancelled,
        "Expired claim should be Cancelled"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 5: Non-admin cannot cancel a claim
// ═══════════════════════════════════════════════════════════════════════════

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_non_admin_cannot_cancel_claim() {
    let t = setup();
    let env = &t.env;

    let now: u64 = env.ledger().timestamp();
    let claim_id =
        t.client
            .create_pending_claim(&t.program_id, &t.contributor, &5_000_i128, &(now + 86_400));

    let random_user = Address::generate(env);

    // A non-admin user attempts to cancel the claim — should panic
    t.client
        .cancel_claim(&t.program_id, &claim_id, &random_user);
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 6: Prevent double-claim (cannot execute an already completed claim)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
#[should_panic(expected = "ClaimAlreadyProcessed")]
fn test_cannot_double_claim() {
    let t = setup();
    let env = &t.env;

    let now: u64 = env.ledger().timestamp();
    let claim_id =
        t.client
            .create_pending_claim(&t.program_id, &t.contributor, &10_000_i128, &(now + 86_400));

    // First execution succeeds
    t.client
        .execute_claim(&t.program_id, &claim_id, &t.contributor);

    // Second execution on the same claim_id must fail
    t.client
        .execute_claim(&t.program_id, &claim_id, &t.contributor);
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 7: Cannot execute a cancelled claim
// ═══════════════════════════════════════════════════════════════════════════

#[test]
#[should_panic(expected = "ClaimAlreadyProcessed")]
fn test_cannot_execute_cancelled_claim() {
    let t = setup();
    let env = &t.env;

    let now: u64 = env.ledger().timestamp();
    let claim_id =
        t.client
            .create_pending_claim(&t.program_id, &t.contributor, &5_000_i128, &(now + 86_400));

    // Admin cancels the claim first
    t.client.cancel_claim(&t.program_id, &claim_id, &t.admin);

    // Contributor then attempts to execute the cancelled claim — should fail
    t.client
        .execute_claim(&t.program_id, &claim_id, &t.contributor);
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 8: Only the designated recipient can execute a claim
// ═══════════════════════════════════════════════════════════════════════════

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_wrong_recipient_cannot_execute_claim() {
    let t = setup();
    let env = &t.env;

    let now: u64 = env.ledger().timestamp();
    let claim_id =
        t.client
            .create_pending_claim(&t.program_id, &t.contributor, &5_000_i128, &(now + 86_400));

    let impostor = Address::generate(env);

    // An unrelated address tries to execute the claim — should panic
    t.client.execute_claim(&t.program_id, &claim_id, &impostor);
}
