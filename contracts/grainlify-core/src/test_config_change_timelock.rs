#![cfg(test)]

extern crate std;

use soroban_sdk::{testutils::{Address as _, Ledger as _}, Address, Env};

use crate::{GrainlifyContract, GrainlifyContractClient};

fn setup(env: &Env) -> (GrainlifyContractClient, Address) {
    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.init_admin(&admin);
    (client, admin)
}

#[test]
fn test_config_change_delay_default() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    assert_eq!(client.get_config_change_delay(), 21_600);
}

#[test]
fn test_propose_and_execute_restore_after_timelock() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    let snapshot_id = client.create_config_snapshot();
    client.set_version(&5);

    client.set_config_change_delay(&3_600);
    let proposal_id = client.propose_config_snapshot_restore(&snapshot_id);

    assert!(client.get_config_change_status(&proposal_id).unwrap() > 0);

    let early = client.try_execute_config_snapshot_restore(&proposal_id);
    assert!(early.is_err(), "must not execute before timelock expires");

    env.ledger().set_timestamp(env.ledger().timestamp() + 3_700);
    client.execute_config_snapshot_restore(&proposal_id);

    assert_eq!(client.get_version(), 2, "version should be restored from snapshot");
    let proposal = client.get_config_change_proposal(&proposal_id).unwrap();
    assert!(proposal.executed);
    assert!(!proposal.cancelled);
}

#[test]
fn test_cancelled_config_change_cannot_execute() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    let snapshot_id = client.create_config_snapshot();
    client.set_config_change_delay(&3_600);
    let proposal_id = client.propose_config_snapshot_restore(&snapshot_id);

    client.cancel_config_change(&proposal_id);

    env.ledger().set_timestamp(env.ledger().timestamp() + 3_700);
    let result = client.try_execute_config_snapshot_restore(&proposal_id);
    assert!(result.is_err(), "cancelled proposal must never execute");
}

#[test]
#[should_panic(expected = "Config change delay must be at least 1 hour")]
fn test_set_config_change_delay_rejects_too_small() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    client.set_config_change_delay(&1_800);
}

#[test]
#[should_panic(expected = "Snapshot not found or has been pruned")]
fn test_propose_config_restore_requires_existing_snapshot() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    client.propose_config_snapshot_restore(&999);
}

#[test]
fn test_config_change_status_counts_down() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    let snapshot_id = client.create_config_snapshot();
    client.set_config_change_delay(&3_600);
    let proposal_id = client.propose_config_snapshot_restore(&snapshot_id);

    let initial_remaining = client.get_config_change_status(&proposal_id).unwrap();
    assert!(initial_remaining > 0);

    env.ledger().set_timestamp(env.ledger().timestamp() + 3_600);
    assert_eq!(client.get_config_change_status(&proposal_id), Some(0));
}
