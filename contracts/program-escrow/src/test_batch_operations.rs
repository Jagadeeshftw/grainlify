#![cfg(test)]

use soroban_sdk::testutils::Ledger as _;
use soroban_sdk::testutils::LedgerInfo as _;
use soroban_sdk::{testutils::Address as _, token, vec, Address, Env, String, TryIntoVal, Vec};

use crate::{
    BatchError, LockItem, ProgramData, ProgramEscrowContract, ProgramEscrowContractClient,
    ReleaseItem,
};

pub struct Ctx<'a> {
    pub env: Env,
    pub client: ProgramEscrowContractClient<'a>,
    pub token_id: Address,
    pub token_admin: Address,
    pub admin: Address,
}

pub fn setup() -> Ctx<'static> {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract(token_admin.clone());

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    client.initialize_contract(&admin);

    Ctx {
        env,
        client,
        token_id,
        token_admin,
        admin,
    }
}

fn mint(ctx: &Ctx, recipient: &Address, amount: i128) {
    token::StellarAssetClient::new(&ctx.env, &ctx.token_id).mint(recipient, &amount);
}

pub fn init_program(ctx: &Ctx, program_id: &str, amount: i128) {
    let creator = Address::generate(&ctx.env);
    mint(ctx, &creator, amount);
    ctx.client.init_program(
        &String::from_str(&ctx.env, program_id),
        &ctx.admin.clone(), // authorized_payout_key
        &ctx.token_id,
        &creator,
        &Some(amount),
        &None,
    );
    ctx.client.publish_program(&String::from_str(&ctx.env, program_id));
}

#[test]
fn test_batch_lock_success() {
    let ctx = setup();
    init_program(&ctx, "PROG1", 1000);
    init_program(&ctx, "PROG2", 2000);

    let items = vec![
        &ctx.env,
        LockItem {
            program_id: String::from_str(&ctx.env, "PROG1"),
            amount: 500,
        },
        LockItem {
            program_id: String::from_str(&ctx.env, "PROG2"),
            amount: 1500,
        },
    ];

    let result = ctx.client.batch_lock(&items);
    assert_eq!(result, 2);

    let prog1 = ctx
        .client
        .get_program_info_v2(&String::from_str(&ctx.env, "PROG1"));
    assert_eq!(prog1.total_funds, 1500);
    assert_eq!(prog1.remaining_balance, 1500);

    let prog2 = ctx
        .client
        .get_program_info_v2(&String::from_str(&ctx.env, "PROG2"));
    assert_eq!(prog2.total_funds, 3500);
}

#[test]
fn test_batch_lock_atomicity() {
    let ctx = setup();
    init_program(&ctx, "PROG1", 1000);

    let items = vec![
        &ctx.env,
        LockItem {
            program_id: String::from_str(&ctx.env, "PROG1"),
            amount: 500,
        },
        LockItem {
            program_id: String::from_str(&ctx.env, "NONEXISTENT"),
            amount: 100,
        },
    ];

    let result = ctx.client.try_batch_lock(&items);
    assert!(result.is_err());

    // PROG1 should not be updated
    let prog1 = ctx
        .client
        .get_program_info_v2(&String::from_str(&ctx.env, "PROG1"));
    assert_eq!(prog1.total_funds, 1000);
}

#[test]
fn test_batch_release_success() {
    let ctx = setup();
    init_program(&ctx, "PROG1", 5000);

    // Create schedules
    let recipient1 = Address::generate(&ctx.env);
    let recipient2 = Address::generate(&ctx.env);

    ctx.client.create_program_release_schedule(
        &recipient1,
        &1000,
        &0, // immediate
    );
    ctx.client.create_program_release_schedule(
        &recipient2,
        &2000,
        &0, // immediate
    );

    let items = vec![
        &ctx.env,
        ReleaseItem {
            program_id: String::from_str(&ctx.env, "PROG1"),
            schedule_id: 1,
        },
        ReleaseItem {
            program_id: String::from_str(&ctx.env, "PROG1"),
            schedule_id: 2,
        },
    ];

    let result = ctx.client.batch_release(&items);
    assert_eq!(result, 2);

    // Verify balances
    let prog1 = ctx
        .client
        .get_program_info_v2(&String::from_str(&ctx.env, "PROG1"));
    assert_eq!(prog1.remaining_balance, 2000);

    // Verify tokens were transferred
    let token_client = token::Client::new(&ctx.env, &ctx.token_id);
    assert_eq!(token_client.balance(&recipient1), 1000);
    assert_eq!(token_client.balance(&recipient2), 2000);
}

#[test]
fn test_batch_release_duplicate_fails() {
    let ctx = setup();
    init_program(&ctx, "PROG1", 5000);
    let recipient = Address::generate(&ctx.env);
    ctx.client
        .create_program_release_schedule(&recipient, &1000, &0);

    let items = vec![
        &ctx.env,
        ReleaseItem {
            program_id: String::from_str(&ctx.env, "PROG1"),
            schedule_id: 1,
        },
        ReleaseItem {
            program_id: String::from_str(&ctx.env, "PROG1"),
            schedule_id: 1, // DUPLICATE
        },
    ];

    let result = ctx.client.try_batch_release(&items);
    assert!(result.is_err());
}
