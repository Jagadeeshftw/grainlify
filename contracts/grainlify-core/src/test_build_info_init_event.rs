#![cfg(test)]

extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Events},
    vec, Address, Env, String, Symbol, TryFromVal,
};

use crate::{GovernanceConfig, GrainlifyContract, GrainlifyContractClient, VotingScheme};

fn count_build_events(env: &Env) -> usize {
    let events = env.events().all();
    events
        .iter()
        .filter(|e| {
            if e.1.len() < 2 {
                return false;
            }
            let t0 = match Symbol::try_from_val(env, &e.1.get(0).unwrap()) {
                Ok(v) => v,
                Err(_) => return false,
            };
            let t1 = match Symbol::try_from_val(env, &e.1.get(1).unwrap()) {
                Ok(v) => v,
                Err(_) => return false,
            };
            t0 == Symbol::new(env, "init") && t1 == Symbol::new(env, "build")
        })
        .count()
}

fn governance_config(env: &Env) -> GovernanceConfig {
    GovernanceConfig {
        voting_period: 86_400,
        execution_delay: 3_600,
        quorum_percentage: 5_000,
        approval_threshold: 6_000,
        min_proposal_stake: 1,
        voting_scheme: VotingScheme::OnePersonOneVote,
        governance_token: Address::generate(env),
    }
}

#[test]
fn init_admin_emits_build_info_event() {
    let env = Env::default();
    env.mock_all_auths();

    let id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &id);
    let admin = Address::generate(&env);

    let before = count_build_events(&env);
    client.init_admin(&admin);
    let after = count_build_events(&env);

    assert_eq!(after, before + 1);
}

#[test]
fn init_multisig_emits_build_info_event() {
    let env = Env::default();
    env.mock_all_auths();

    let id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &id);
    let signers = vec![&env, Address::generate(&env), Address::generate(&env)];

    let before = count_build_events(&env);
    client.init(&signers, &2);
    let after = count_build_events(&env);

    assert_eq!(after, before + 1);
}

#[test]
fn init_with_network_emits_build_info_event() {
    let env = Env::default();
    env.mock_all_auths();

    let id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &id);
    let admin = Address::generate(&env);

    let before = count_build_events(&env);
    client.init_with_network(
        &admin,
        &String::from_str(&env, "stellar-main"),
        &String::from_str(&env, "mainnet"),
    );
    let after = count_build_events(&env);

    assert_eq!(after, before + 1);
}

#[test]
fn init_governance_emits_build_info_event() {
    let env = Env::default();
    env.mock_all_auths();

    let id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &id);
    let admin = Address::generate(&env);

    let before = count_build_events(&env);
    client.init_governance(&admin, &governance_config(&env));
    let after = count_build_events(&env);

    assert_eq!(after, before + 1);
}

#[test]
fn failed_reinit_does_not_emit_extra_build_event() {
    let env = Env::default();
    env.mock_all_auths();

    let id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &id);
    let admin = Address::generate(&env);

    client.init_admin(&admin);
    let after_first = count_build_events(&env);
    let result = client.try_init_admin(&admin);

    assert!(result.is_err());
    assert_eq!(count_build_events(&env), after_first);
}

#[test]
fn failed_unauthorized_init_emits_no_build_event() {
    let env = Env::default();

    let id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &id);
    let admin = Address::generate(&env);

    let before = count_build_events(&env);
    let result = client.try_init_admin(&admin);

    assert!(result.is_err());
    assert_eq!(count_build_events(&env), before);
}
