#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token, Address, Env, String,
};

use crate::{ClaimStatus, ProgramEscrowContract, ProgramEscrowContractClient};

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
    admin: Address,
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
    client.init_program(
        &program_id,
        &payout_key,
        &token.address,
        &payout_key,
        &None,
        &None,
    );
    client.publish_program(&program_id);

    // lock funds
    client.lock_program_funds(&500_000_i128);
    client.set_admin(&admin);

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

    TestSetup { env, client, token, admin, contributor, program_id }
}

#[test]
fn test_claim_within_window_succeeds() {
    let t = setup();
    let now = t.env.ledger().timestamp();
    let amount: i128 = 10_000;
    let deadline = now + 86_400;

    let claim_id = t.client.create_pending_claim(&t.program_id, &t.contributor, &amount, &deadline);

    let claim = t.client.get_claim(&t.program_id, &claim_id);
    assert_eq!(claim.status, ClaimStatus::Pending);
    assert_eq!(claim.amount, amount);
    assert_eq!(claim.recipient, t.contributor);

    let balance_before = t.token.balance(&t.contributor);

    t.env.ledger().set(LedgerInfo { timestamp: now + 21_600, ..t.env.ledger().get() });
    t.client.execute_claim(&t.program_id, &claim_id, &t.contributor);

    assert_eq!(t.token.balance(&t.contributor) - balance_before, amount);
    assert_eq!(t.client.get_claim(&t.program_id, &claim_id).status, ClaimStatus::Completed);
    assert_eq!(t.client.get_program_info().remaining_balance, 500_000 - amount);
}

#[test]
#[should_panic(expected = "ClaimExpired")]
fn test_claim_after_expiry_fails() {
    let t = setup();
    let now = t.env.ledger().timestamp();
    let claim_id =
        t.client.create_pending_claim(&t.program_id, &t.contributor, &5_000_i128, &(now + 3_600));

    t.env.ledger().set(LedgerInfo { timestamp: now + 7_200, ..t.env.ledger().get() });

    assert_eq!(t.client.get_claim(&t.program_id, &claim_id).status, ClaimStatus::Pending);
    t.client.execute_claim(&t.program_id, &claim_id, &t.contributor);
}

#[test]
fn test_admin_cancel_pending_claim_restores_escrow() {
    let t = setup();
    let now = t.env.ledger().timestamp();
    let amount: i128 = 8_000;
    let claim_id =
        t.client.create_pending_claim(&t.program_id, &t.contributor, &amount, &(now + 86_400));

    let balance_after_create = t.client.get_remaining_balance();

    t.env.ledger().set(LedgerInfo { timestamp: now + 1_800, ..t.env.ledger().get() });
    t.client.cancel_claim(&t.program_id, &claim_id, &t.admin);

    assert_eq!(t.client.get_remaining_balance(), balance_after_create + amount);
    assert_eq!(t.client.get_claim(&t.program_id, &claim_id).status, ClaimStatus::Cancelled);
    assert_eq!(t.token.balance(&t.contributor), 0);
}

#[test]
fn test_admin_cancel_expired_claim_succeeds() {
    let t = setup();
    let now = t.env.ledger().timestamp();
    let amount: i128 = 3_000;
    let claim_id =
        t.client.create_pending_claim(&t.program_id, &t.contributor, &amount, &(now + 3_600));

    t.env.ledger().set(LedgerInfo { timestamp: now + 7_200, ..t.env.ledger().get() });

    let balance_before = t.client.get_remaining_balance();
    t.client.cancel_claim(&t.program_id, &claim_id, &t.admin);

    assert_eq!(t.client.get_remaining_balance(), balance_before + amount);
    assert_eq!(t.client.get_claim(&t.program_id, &claim_id).status, ClaimStatus::Cancelled);
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_non_admin_cannot_cancel_claim() {
    let t = setup();
    let now = t.env.ledger().timestamp();
    let claim_id =
        t.client.create_pending_claim(&t.program_id, &t.contributor, &5_000_i128, &(now + 86_400));

    t.client.cancel_claim(&t.program_id, &claim_id, &Address::generate(&t.env));
}

#[test]
#[should_panic(expected = "ClaimAlreadyProcessed")]
fn test_cannot_double_claim() {
    let t = setup();
    let now = t.env.ledger().timestamp();
    let claim_id =
        t.client.create_pending_claim(&t.program_id, &t.contributor, &10_000_i128, &(now + 86_400));

    t.client.execute_claim(&t.program_id, &claim_id, &t.contributor);
    t.client.execute_claim(&t.program_id, &claim_id, &t.contributor);
}

#[test]
#[should_panic(expected = "ClaimAlreadyProcessed")]
fn test_cannot_execute_cancelled_claim() {
    let t = setup();
    let now = t.env.ledger().timestamp();
    let claim_id =
        t.client.create_pending_claim(&t.program_id, &t.contributor, &5_000_i128, &(now + 86_400));

    t.client.cancel_claim(&t.program_id, &claim_id, &t.admin);
    t.client.execute_claim(&t.program_id, &claim_id, &t.contributor);
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_wrong_recipient_cannot_execute_claim() {
    let t = setup();
    let now = t.env.ledger().timestamp();
    let claim_id =
        t.client.create_pending_claim(&t.program_id, &t.contributor, &5_000_i128, &(now + 86_400));

    t.client.execute_claim(&t.program_id, &claim_id, &Address::generate(&t.env));
}
