#![cfg(test)]

use super::*;
use crate::invariants;
use soroban_sdk::{testutils::Address as _, token, Address, Env};

fn setup_bounty(env: &Env) -> (BountyEscrowContractClient<'static>, Address, Address) {
    env.mock_all_auths();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(env, &contract_id);

    let admin = Address::generate(env);
    let depositor = Address::generate(env);
    let token_admin = Address::generate(env);
    let token_id = env.register_stellar_asset_contract(token_admin.clone());
    let token_admin_client = token::StellarAssetClient::new(env, &token_id);

    client.init(&admin, &token_id);
    token_admin_client.mint(&depositor, &50_000);

    (client, admin, depositor)
}

#[test]
fn test_invariant_checker_ci_called_in_major_bounty_flows() {
    let env = Env::default();
    let (client, _admin, depositor) = setup_bounty(&env);
    env.as_contract(&client.address, || invariants::reset_test_state(&env));

    let bounty_id = 42_u64;
    let contributor = Address::generate(&env);
    let amount = 10_000_i128;
    let deadline = env.ledger().timestamp() + 1000;

    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);
    client.release_funds(&bounty_id, &contributor);

    let calls = env.as_contract(&client.address, || invariants::call_count_for_test(&env));
    assert!(calls >= 2);
}

#[test]
#[should_panic(expected = "Invariant checks disabled")]
fn test_invariant_checker_ci_panics_when_disabled() {
    let env = Env::default();
    let (client, _admin, depositor) = setup_bounty(&env);
    env.as_contract(&client.address, || {
        invariants::reset_test_state(&env);
        invariants::set_disabled_for_test(&env, true);
    });

    client.lock_funds(&depositor, &7_u64, &5_000_i128, &(0_u64 + 500));
}
