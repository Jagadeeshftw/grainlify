#![cfg(test)]

use super::reputation::{
    benchmark_overall_scores_dust_vs_typical, REPUTATION_DUST_PAYOUT_AMOUNT,
    REPUTATION_MIN_QUALIFYING_PAYOUT_AMOUNT, REPUTATION_TYPICAL_PAYOUT_AMOUNT,
};
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
    client.publish_program(program_id.clone(), admin.clone());
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
    assert_eq!(rep.qualified_payout_count, 0);
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
    assert_eq!(rep.qualified_payout_count, 0);
    assert_eq!(rep.payout_fulfillment_rate_bps, 0);
    assert_eq!(rep.completion_rate_bps, 10_000);
}

#[test]
fn test_reputation_after_payouts() {
    let env = Env::default();
    let (client, _, _, _token_client, _) = setup_active_program(&env, 100_000);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    client.batch_payout(
        &vec![&env, r1.clone(), r2.clone()],
        &vec![&env, 30_000, 20_000],
    );

    let rep = client.get_program_reputation();
    assert_eq!(rep.total_payouts, 2);
    assert_eq!(rep.qualified_payout_count, 2);
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
    client.single_payout(&r1, &100_000, &None);

    let rep = client.get_program_reputation();
    assert_eq!(rep.total_payouts, 1);
    assert_eq!(rep.qualified_payout_count, 1);
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

    env.ledger().set_timestamp(1000);
    client.create_program_release_schedule(&r1, &100_000, &500);
    client.create_program_release_schedule(&r2, &100_000, &800);
    client.create_program_release_schedule(&r3, &100_000, &2000);

    let rep = client.get_program_reputation();
    assert_eq!(rep.total_scheduled, 3);
    assert_eq!(rep.completed_releases, 0);
    assert_eq!(rep.pending_releases, 3);
    assert_eq!(rep.overdue_releases, 2);
    assert_eq!(rep.completion_rate_bps, 0);

    client.trigger_program_releases(&None);

    let rep = client.get_program_reputation();
    assert_eq!(rep.completed_releases, 2);
    assert_eq!(rep.pending_releases, 1);
    assert_eq!(rep.overdue_releases, 0);
    assert_eq!(rep.completion_rate_bps, 6_666);

    env.ledger().set_timestamp(2500);
    client.trigger_program_releases(&None);

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

    let r1 = Address::generate(&env);
    client.single_payout(&r1, &200_000, &None);

    let r2 = Address::generate(&env);
    env.ledger().set_timestamp(100);
    client.create_program_release_schedule(&r2, &100_000, &50);

    client.trigger_program_releases(&None);

    let rep = client.get_program_reputation();
    assert_eq!(rep.total_payouts, 2);
    assert_eq!(rep.qualified_payout_count, 2);
    assert_eq!(rep.total_scheduled, 1);
    assert_eq!(rep.completed_releases, 1);
    assert_eq!(rep.total_funds_distributed, 300_000);
    assert_eq!(rep.completion_rate_bps, 10_000);
    assert_eq!(rep.payout_fulfillment_rate_bps, 6_000);
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

    let rep = client.get_program_reputation();
    assert_eq!(rep.overdue_releases, 2);
    assert_eq!(rep.completion_rate_bps, 0);
    assert_eq!(rep.overall_score_bps, 0);
}

/// Many minimum-size payouts increase raw history length but not qualifying activity.
#[test]
fn test_reputation_dust_payouts_gaming_resistance() {
    let env = Env::default();
    const DUST_COUNT: u32 = 40;
    let locked = (DUST_COUNT as i128).saturating_mul(REPUTATION_TYPICAL_PAYOUT_AMOUNT);
    let (client, _, _, _, _) = setup_active_program(&env, locked);

    let recipient = Address::generate(&env);
    for _ in 0..DUST_COUNT {
        client.single_payout(&recipient, &REPUTATION_DUST_PAYOUT_AMOUNT, &None);
    }

    let rep = client.get_program_reputation();
    assert_eq!(rep.total_payouts, DUST_COUNT);
    assert_eq!(rep.qualified_payout_count, 0);
    assert_eq!(
        rep.total_funds_distributed,
        (DUST_COUNT as i128).saturating_mul(REPUTATION_DUST_PAYOUT_AMOUNT)
    );
    assert!(
        rep.payout_fulfillment_rate_bps < 100,
        "dust fulfillment should stay far below 1% of locked pool"
    );
    assert!(
        rep.overall_score_bps < 7_000,
        "overall score must remain low despite high payout count"
    );
}

/// Same locked pool: N typical payouts should reach a much higher score than N dust payouts.
#[test]
fn test_reputation_benchmark_dust_vs_typical_on_chain() {
    let env = Env::default();
    const N: u32 = 25;
    let locked = (N as i128).saturating_mul(REPUTATION_TYPICAL_PAYOUT_AMOUNT);

    let (dust_client, _, _, _, _) = setup_active_program(&env, locked);
    let dust_recipient = Address::generate(&env);
    for _ in 0..N {
        dust_client.single_payout(&dust_recipient, &REPUTATION_DUST_PAYOUT_AMOUNT, &None);
    }
    let dust_rep = dust_client.get_program_reputation();

    let env_typical = Env::default();
    let (typical_client, _, _, _, _) = setup_active_program(&env_typical, locked);
    let typical_recipient = Address::generate(&env_typical);
    for _ in 0..N {
        typical_client.single_payout(
            &typical_recipient,
            &REPUTATION_TYPICAL_PAYOUT_AMOUNT,
            &None,
        );
    }
    let typical_rep = typical_client.get_program_reputation();

    let (pure_dust, pure_typical) = benchmark_overall_scores_dust_vs_typical(N);
    assert_eq!(dust_rep.overall_score_bps, pure_dust);
    assert_eq!(typical_rep.overall_score_bps, pure_typical);
    assert!(typical_rep.overall_score_bps > dust_rep.overall_score_bps);
    assert_eq!(typical_rep.payout_fulfillment_rate_bps, 10_000);
    assert_eq!(typical_rep.qualified_payout_count, N);
    assert_eq!(dust_rep.qualified_payout_count, 0);
}

#[test]
fn test_reputation_qualifying_threshold_boundary() {
    let env = Env::default();
    let (client, _, _, _, _) = setup_active_program(&env, 50_000);

    let below = Address::generate(&env);
    let at_floor = Address::generate(&env);
    client.single_payout(
        &below,
        &(REPUTATION_MIN_QUALIFYING_PAYOUT_AMOUNT - 1),
        &None,
    );
    client.single_payout(
        &at_floor,
        &REPUTATION_MIN_QUALIFYING_PAYOUT_AMOUNT,
        &None,
    );

    let rep = client.get_program_reputation();
    assert_eq!(rep.total_payouts, 2);
    assert_eq!(rep.qualified_payout_count, 1);
}

