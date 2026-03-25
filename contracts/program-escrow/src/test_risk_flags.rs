#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn setup_program(env: &Env) -> (ProgramEscrowContractClient<'static>, Address, String) {
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);

    let admin = Address::generate(env);
    client.initialize_contract(&admin);

    let token_admin = Address::generate(env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();

    let program_id = String::from_str(env, "risk-prog");
    client.init_program(
        &program_id,
        &admin,
        &token_id,
        &admin,
        &None,
        &None,
    );

    (client, admin, program_id)
}

#[test]
fn test_program_risk_flags_set_and_clear() {
    let env = Env::default();
    let (client, _admin, program_id) = setup_program(&env);

    let flags = RISK_FLAG_HIGH_RISK | RISK_FLAG_UNDER_REVIEW;
    let updated = client.set_program_risk_flags(&program_id, &flags);
    assert_eq!(updated.risk_flags, flags);

    let cleared = client.clear_program_risk_flags(&program_id, &RISK_FLAG_UNDER_REVIEW);
    assert_eq!(cleared.risk_flags, RISK_FLAG_HIGH_RISK);

    let fetched = client.get_program_info_v2(&program_id);
    assert_eq!(fetched.risk_flags, RISK_FLAG_HIGH_RISK);
}
