#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, vec, Address, Env, String,
};

fn make_client(env: &Env) -> (ProgramEscrowContractClient<'static>, Address) {
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);
    (client, contract_id)
}

fn fund_contract(
    env: &Env,
    contract_id: &Address,
    amount: i128,
) -> (token::Client<'static>, Address, token::StellarAssetClient<'static>) {
    let token_admin = Address::generate(env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = token_contract.address();
    let token_client = token::Client::new(env, &token_id);
    let token_sac = token::StellarAssetClient::new(env, &token_id);
    if amount > 0 {
        token_sac.mint(contract_id, &amount);
    }
    (token_client, token_id, token_sac)
}

fn setup_active_program(
    env: &Env,
    amount: i128,
) -> (
    ProgramEscrowContractClient<'static>,
    Address,
    Address,
    token::Client<'static>,
    token::StellarAssetClient<'static>,
) {
    env.mock_all_auths();
    let (client, contract_id) = make_client(env);
    let (token_client, token_id, token_sac) = fund_contract(env, &contract_id, amount);
    let admin = Address::generate(env);
    let program_id = String::from_str(env, "rep-test");
    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    client.publish_program(&program_id);
    if amount > 0 {
        client.lock_program_funds(&amount);
    }
    (client, admin, contract_id, token_client, token_sac)
}

#[test]
fn test_reputation_fresh_program() {
    let env = Env::default();
    let (client, _, _, _, _) = setup_active_program(&env, 0);

    let rep = client.get_program_reputation();
    assert_eq!(rep.total_payouts, 0);
    assert_eq!(rep.total_scheduled, 0);
    assert_eq!(rep.completed_releases, 0);
    assert_eq!(rep.pending_releases, 0);
    assert_eq!(rep.overdue_releases, 0);
    assert_eq!(rep.dispute_count, 0);
    assert_eq!(rep.refund_count, 0);
    assert_eq!(rep.total_funds_locked, 0);
    assert_eq!(rep.total_funds_distributed, 0);
    assert_eq!(rep.completion_rate_bps, 10_000);
    assert_eq!(rep.payout_fulfillment_rate_bps, 10_000);
    assert_eq!(rep.overall_score_bps, 10_000);
}

#[test]
fn test_reputation_funded_no_payouts() {
    let env = Env::default();
    let (client, _, _, _, _) = setup_active_program(&env, 500_000);

    let rep = client.get_program_reputation();
    assert_eq!(rep.total_funds_locked, 500_000);
    assert_eq!(rep.total_funds_distributed, 0);
    assert_eq!(rep.payout_fulfillment_rate_bps, 0);
    assert_eq!(rep.completion_rate_bps, 10_000);
}

#[test]
fn test_reputation_after_payouts() {
    let env = Env::default();
    let (client, _, _, _token_client, _) = setup_active_program(&env, 100_000);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    client.batch_payout(&vec![&env, r1.clone(), r2.clone()], &vec![&env, 30_000, 20_000]);

    let rep = client.get_program_reputation();
    assert_eq!(rep.total_payouts, 2);
    assert_eq!(rep.total_funds_locked, 100_000);
    assert_eq!(rep.total_funds_distributed, 50_000);
    assert_eq!(rep.payout_fulfillment_rate_bps, 5_000);
    assert_eq!(rep.completion_rate_bps, 10_000);
    assert_eq!(rep.overall_score_bps, 8_000);
}

#[test]
fn test_reputation_full_distribution() {
    let env = Env::default();
    let (client, _, _, _, _) = setup_active_program(&env, 100_000);

    let r1 = Address::generate(&env);
    client.single_payout(&r1, &100_000);

    let rep = client.get_program_reputation();
    assert_eq!(rep.total_payouts, 1);
    assert_eq!(rep.total_funds_distributed, 100_000);
    assert_eq!(rep.payout_fulfillment_rate_bps, 10_000);
    assert_eq!(rep.overall_score_bps, 10_000);
}

#[test]
fn test_reputation_with_schedules() {
    let env = Env::default();
    let (client, _, _, _, _) = setup_active_program(&env, 300_000);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    let r3 = Address::generate(&env);

    // Create 3 schedules: two due now, one in the future
    env.ledger().set_timestamp(1000);
    client.create_program_release_schedule(&r1, &100_000, &500);
    client.create_program_release_schedule(&r2, &100_000, &800);
    client.create_program_release_schedule(&r3, &100_000, &2000);

    let rep = client.get_program_reputation();
    assert_eq!(rep.total_scheduled, 3);
    assert_eq!(rep.completed_releases, 0);
    assert_eq!(rep.pending_releases, 3);
    assert_eq!(rep.overdue_releases, 2); // two are past due (500, 800 < 1000)
    assert_eq!(rep.completion_rate_bps, 0);

    // Trigger releases for the two due schedules
    client.trigger_program_releases();

    let rep = client.get_program_reputation();
    assert_eq!(rep.completed_releases, 2);
    assert_eq!(rep.pending_releases, 1);
    assert_eq!(rep.overdue_releases, 0); // the remaining one is future
    assert_eq!(rep.completion_rate_bps, 6_666); // 2/3

    // Advance time and trigger the last one
    env.ledger().set_timestamp(2500);
    client.trigger_program_releases();

    let rep = client.get_program_reputation();
    assert_eq!(rep.completed_releases, 3);
    assert_eq!(rep.pending_releases, 0);
    assert_eq!(rep.overdue_releases, 0);
    assert_eq!(rep.completion_rate_bps, 10_000);
    assert_eq!(rep.payout_fulfillment_rate_bps, 10_000);
    assert_eq!(rep.overall_score_bps, 10_000);
}

#[test]
fn test_reputation_mixed_payouts_and_schedules() {
    let env = Env::default();
    let (client, _, _, _, _) = setup_active_program(&env, 500_000);

    // Direct payout
    let r1 = Address::generate(&env);
    client.single_payout(&r1, &200_000);

    // Schedule a release
    let r2 = Address::generate(&env);
    env.ledger().set_timestamp(100);
    client.create_program_release_schedule(&r2, &100_000, &50);

    // Trigger the due schedule
    client.trigger_program_releases();

    let rep = client.get_program_reputation();
    assert_eq!(rep.total_payouts, 2); // 1 direct + 1 from schedule trigger
    assert_eq!(rep.total_scheduled, 1);
    assert_eq!(rep.completed_releases, 1);
    assert_eq!(rep.total_funds_distributed, 300_000);
    assert_eq!(rep.completion_rate_bps, 10_000);
    assert_eq!(rep.payout_fulfillment_rate_bps, 6_000); // 300k/500k
}

#[test]
fn test_reputation_overdue_schedules() {
    let env = Env::default();
    let (client, _, _, _, _) = setup_active_program(&env, 200_000);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    env.ledger().set_timestamp(1000);
    client.create_program_release_schedule(&r1, &100_000, &500);
    client.create_program_release_schedule(&r2, &100_000, &800);

    // Both are overdue (timestamps 500, 800 are before now=1000)
    let rep = client.get_program_reputation();
    assert_eq!(rep.overdue_releases, 2);
    assert_eq!(rep.completion_rate_bps, 0);
    // overall = (0 * 60 + 0 * 40) / 100 = 0
    assert_eq!(rep.overall_score_bps, 0);
}
