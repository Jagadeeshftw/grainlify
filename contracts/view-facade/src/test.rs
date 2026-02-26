#![cfg(test)]

use crate::{ContractKind, ViewFacade, ViewFacadeClient};
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn test_register_and_lookup_contract() {
    let env = Env::default();
    env.mock_all_auths();

    let facade_id = env.register_contract(None, ViewFacade);
    let facade = ViewFacadeClient::new(&env, &facade_id);

    let admin = Address::generate(&env);
    let bounty_contract = Address::generate(&env);

    facade.init(&admin);
    facade.register(&bounty_contract, &ContractKind::BountyEscrow, &1u32);

    let entry = facade.get_contract(&bounty_contract).unwrap();
    assert_eq!(entry.address, bounty_contract);
    assert_eq!(entry.kind, ContractKind::BountyEscrow);
    assert_eq!(entry.version, 1);
}

#[test]
fn test_list_and_count_contracts() {
    let env = Env::default();
    env.mock_all_auths();

    let facade_id = env.register_contract(None, ViewFacade);
    let facade = ViewFacadeClient::new(&env, &facade_id);

    let admin = Address::generate(&env);
    facade.init(&admin);

    let c1 = Address::generate(&env);
    let c2 = Address::generate(&env);

    facade.register(&c1, &ContractKind::BountyEscrow, &1u32);
    facade.register(&c2, &ContractKind::ProgramEscrow, &2u32);

    assert_eq!(facade.contract_count(), 2);
    let all = facade.list_contracts();
    assert_eq!(all.len(), 2);
}

#[test]
fn test_deregister_contract() {
    let env = Env::default();
    env.mock_all_auths();

    let facade_id = env.register_contract(None, ViewFacade);
    let facade = ViewFacadeClient::new(&env, &facade_id);

    let admin = Address::generate(&env);
    let contract = Address::generate(&env);

    facade.init(&admin);
    facade.register(&contract, &ContractKind::GrainlifyCore, &3u32);
    assert_eq!(facade.contract_count(), 1);

    facade.deregister(&contract);

    assert_eq!(facade.contract_count(), 0);
    assert_eq!(facade.get_contract(&contract), None);
}
