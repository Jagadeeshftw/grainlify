#![cfg(test)]
use soroban_sdk::{testutils::{Address as _}, Address, Env, String};
use crate::{ProgramEscrowContract, ProgramEscrowContractClient};

#[test]
fn test_lock_program_funds_from_allowance() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);

    #[allow(deprecated)]
    let token_address = env.register_stellar_asset_contract(token_admin.clone());

    // Admin client to mint
    let token_admin_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_address);
    // Standard client to approve
    let token_client = soroban_sdk::token::Client::new(&env, &token_address);

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let program_id = String::from_str(&env, "prog_allowance");
    let creator = Address::generate(&env);
    let depositor = Address::generate(&env);

    client.initialize_program(
        &program_id,
        &admin,
        &token_address,
        &creator,
        &None,
        &None,
    );
    client.publish_program(&program_id);

    token_admin_client.mint(&depositor, &100_000);

    // Approve the contract to spend the depositor's tokens.
    // 200_u32 is the expiration ledger.
    token_client.approve(&depositor, &contract_id, &50_000, &200_u32);

    let data = client.lock_program_funds_from(&50_000, &depositor);

    assert_eq!(data.total_funds, 50_000);
    assert_eq!(data.remaining_balance, 50_000);
}
