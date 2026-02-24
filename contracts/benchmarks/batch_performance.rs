//! Performance regression benchmarks for batch operations in Program Escrow
//!
//! Measures gas and execution time for batch lock, batch release, and batch payouts
//! across increasing batch sizes. Fails if regressions surpass defined thresholds.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, vec, Address, Env, String,
};

use program_escrow::{ProgramEscrowContract, ProgramEscrowContractClient};

const BATCH_SIZES: [usize; 5] = [1, 5, 10, 25, 50];
const GAS_THRESHOLDS: [u64; 5] = [80_000, 200_000, 350_000, 800_000, 1_600_000]; // Example values
const EXEC_TIME_THRESHOLDS: [u128; 5] = [10_000, 20_000, 40_000, 100_000, 200_000]; // microseconds

fn setup(env: &Env, initial_amount: i128) -> (ProgramEscrowContractClient, Address, token::Client) {
    env.mock_all_auths();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let token_admin = Address::generate(env);
    let token_id = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::Client::new(env, &token_id);
    let program_id = String::from_str(env, "bench-batch");
    client.init_program(&program_id, &admin, &token_id);
    if initial_amount > 0 {
        token::StellarAssetClient::new(env, &token_id).mint(&client.address, &initial_amount);
        client.lock_program_funds(&initial_amount);
    }
    (client, admin, token_client)
}

#[test]
fn benchmark_batch_payouts() {
    let env = Env::default();
    let total = 2_000_000;
    let (client, _admin, _token_client) = setup(&env, total);
    for (i, &batch_size) in BATCH_SIZES.iter().enumerate() {
        let recipients = (0..batch_size)
            .map(|_| Address::generate(&env))
            .collect::<Vec<_>>();
        let amounts = vec![&env];
        for _ in 0..batch_size {
            amounts.push_back(total / (batch_size as i128 * BATCH_SIZES.len() as i128));
        }
        let recipients_vec = vec![&env];
        for r in &recipients {
            recipients_vec.push_back(r.clone());
        }
        let start_gas = env.remaining_gas();
        let start_time = std::time::Instant::now();
        client.batch_payout(&recipients_vec, &amounts);
        let elapsed = start_time.elapsed().as_micros();
        let used_gas = start_gas - env.remaining_gas();
        println!("Batch size: {} | Gas: {} | Time: {}μs", batch_size, used_gas, elapsed);
        assert!(used_gas <= GAS_THRESHOLDS[i], "Gas regression: {} > {}", used_gas, GAS_THRESHOLDS[i]);
        assert!(elapsed <= EXEC_TIME_THRESHOLDS[i], "Exec time regression: {} > {}", elapsed, EXEC_TIME_THRESHOLDS[i]);
    }
}

// TODO: Add similar benchmarks for batch lock and batch release if implemented
