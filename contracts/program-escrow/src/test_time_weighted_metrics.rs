#![cfg(test)]

//! Tests for time-weighted average (TWA) metrics.
//!
//! Window: 24 buckets × 1 hour = 24h. Formulae:
//! - avg_lock_size = sum(lock_amounts) / lock_count in window
//! - avg_settlement_time_secs = sum(lock_to_payout_secs) / settlement_count in window

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, String,
};

fn setup(
    env: &Env,
    initial_lock: i128,
) -> (ProgramEscrowContractClient<'static>, Address, Address) {
    env.mock_all_auths();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let token_admin = Address::generate(env);
    let sac = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = sac.address();
    let token_client = token::Client::new(env, &token_id);
    let token_asset = token::StellarAssetClient::new(env, &token_id);
    let program_id = String::from_str(env, "hack-2026");
    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    client.publish_program(&program_id);
    if initial_lock > 0 {
        token_asset.mint(&client.address, &initial_lock);
        client.lock_program_funds(&initial_lock);
    }
    (client, admin, token_id)
}

#[test]
fn test_time_weighted_metrics_initial_zero() {
    let env = Env::default();
    let (client, _admin, _token_id) = setup(&env, 0);
    let m = client.get_time_weighted_metrics();
    assert_eq!(m.window_secs, 24 * 3600);
    assert_eq!(m.avg_lock_size, 0);
    assert_eq!(m.avg_settlement_time_secs, 0);
    assert_eq!(m.lock_count, 0);
    assert_eq!(m.settlement_count, 0);
}

#[test]
fn test_time_weighted_metrics_after_locks() {
    let env = Env::default();
    let (client, _admin, _token_id) = setup(&env, 0);
    client.lock_program_funds(&10_000);
    client.lock_program_funds(&20_000);
    client.lock_program_funds(&30_000);
    let m = client.get_time_weighted_metrics();
    assert_eq!(m.lock_count, 3);
    assert_eq!(m.avg_lock_size, 20_000); // (10k + 20k + 30k) / 3
    assert_eq!(m.settlement_count, 0);
    assert_eq!(m.avg_settlement_time_secs, 0);
}

#[test]
fn test_time_weighted_metrics_avg_settlement_time() {
    let env = Env::default();
    let (client, admin, _token_id) = setup(&env, 100_000);
    let recipient = Address::generate(&env);
    let t0 = env.ledger().timestamp();
    env.ledger().set_timestamp(t0 + 3600); // 1 hour later
    client.single_payout(&recipient, &10_000);
    let m = client.get_time_weighted_metrics();
    assert_eq!(m.settlement_count, 1);
    assert!(
        m.avg_settlement_time_secs >= 3600,
        "settlement time should be ~1h"
    );
    assert_eq!(m.lock_count, 1);
    assert_eq!(m.avg_lock_size, 100_000);
}

#[test]
fn test_time_weighted_metrics_evolution_over_activity() {
    let env = Env::default();
    let (client, _admin, _token_id) = setup(&env, 50_000);
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    env.ledger().set_timestamp(env.ledger().timestamp() + 3600);
    client.single_payout(&r1, &10_000);
    client.single_payout(&r2, &15_000);
    let m = client.get_time_weighted_metrics();
    assert_eq!(m.settlement_count, 2);
    assert!(m.avg_settlement_time_secs > 0);
    assert_eq!(m.lock_count, 1);
}
