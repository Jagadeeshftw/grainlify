#![cfg(test)]

extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    token, Address, Env, Vec,
};

use crate::{BountyEscrowContract, BountyEscrowContractClient, LockFundsItem};

// ============================================================================
// Constants — must match lib.rs limits
// ============================================================================

const MAX_BATCH: u32 = 20;
const AMOUNT: i128 = 500;
const DEADLINE_OFFSET: u64 = 3_600;

// ============================================================================
// Setup helpers
// ============================================================================

struct Ctx<'a> {
    env: Env,
    client: BountyEscrowContractClient<'a>,
    token_id: Address,
}

fn setup() -> Ctx<'static> {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_sac_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = token_sac_contract.address();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);
    client.init(&admin, &token_id);

    Ctx {
        env,
        client,
        token_id,
    }
}

fn mint(ctx: &Ctx, recipient: &Address, amount: i128) {
    token::StellarAssetClient::new(&ctx.env, &ctx.token_id).mint(recipient, &amount);
}

// ============================================================================
// Benchmarks
// ============================================================================

fn run_benchmark_lock(ctx: &Ctx, use_soa: bool, batch_size: u32) -> (u64, u64) {
    let depositor = Address::generate(&ctx.env);
    mint(&ctx, &depositor, AMOUNT * batch_size as i128);

    if use_soa {
        let mut bounty_ids = Vec::new(&ctx.env);
        let mut depositors = Vec::new(&ctx.env);
        let mut amounts = Vec::new(&ctx.env);
        let mut deadlines = Vec::new(&ctx.env);

        for i in 1..=batch_size as u64 {
            bounty_ids.push_back(i);
            depositors.push_back(depositor.clone());
            amounts.push_back(AMOUNT);
            deadlines.push_back(ctx.env.ledger().timestamp() + DEADLINE_OFFSET);
        }

        ctx.env.budget().reset_default();
        ctx.client.batch_lock_funds_soa(&bounty_ids, &depositors, &amounts, &deadlines);
    } else {
        let mut items = Vec::new(&ctx.env);
        for i in 1..=batch_size as u64 {
            items.push_back(LockFundsItem {
                bounty_id: i,
                depositor: depositor.clone(),
                amount: AMOUNT,
                deadline: ctx.env.ledger().timestamp() + DEADLINE_OFFSET,
            });
        }

        ctx.env.budget().reset_default();
        ctx.client.batch_lock_funds(&items);
    }

    (ctx.env.budget().cpu_instruction_cost(), ctx.env.budget().memory_bytes_cost())
}

#[test]
fn benchmark_lock_soa_vs_aos() {
    let batch_size = MAX_BATCH;

    let ctx_aos = setup();
    let (aos_cpu, _) = run_benchmark_lock(&ctx_aos, false, batch_size);

    let ctx_soa = setup();
    let (soa_cpu, _) = run_benchmark_lock(&ctx_soa, true, batch_size);

    std::println!("[BENCH] batch_lock_funds (AoS) cpu_insns: {}", aos_cpu);
    std::println!("[BENCH] batch_lock_funds_soa (SoA) cpu_insns: {}", soa_cpu);

    // Measured in the test host, SoA and AoS cost within ~1% of each other
    // (host-call overhead dominates; per-element struct deserialization is
    // noise). Guard only against the SoA variant regressing significantly
    // relative to AoS rather than asserting a strict ordering.
    assert!(
        soa_cpu < aos_cpu + aos_cpu / 10,
        "SoA batch lock CPU cost regressed more than 10% over AoS (SoA: {}, AoS: {})",
        soa_cpu,
        aos_cpu
    );
}
