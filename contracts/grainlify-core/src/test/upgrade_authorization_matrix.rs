#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, Vec as SorobanVec};

use crate::{GrainlifyContract, GrainlifyContractClient, UpgradeProposalRecord};

fn wasm_hash(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

fn setup(env: &Env) -> (GrainlifyContractClient, [Address; 3]) {
    let id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(env, &id);
    let signers = [
        Address::generate(env),
        Address::generate(env),
        Address::generate(env),
    ];
    let mut configured = SorobanVec::new(env);
    for signer in &signers {
        configured.push_back(signer.clone());
    }
    client.init(&configured, &2);
    (client, signers)
}

fn assert_unchanged(
    client: &GrainlifyContractClient,
    proposal_id: u64,
    proposal: &UpgradeProposalRecord,
    version: u32,
    previous_version: Option<u32>,
    timelock: Option<u64>,
) {
    assert_eq!(client.get_upgrade_proposal(&proposal_id), Some(proposal.clone()));
    assert_eq!(client.get_version(), version);
    assert_eq!(client.get_previous_version(), previous_version);
    assert_eq!(client.get_timelock_status(&proposal_id), timelock);
}

#[test]
fn direct_admin_upgrade_is_rejected_and_cannot_change_state() {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &id);
    let admin = Address::generate(&env);
    client.init_admin(&admin);
    let version = client.get_version();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.upgrade(&wasm_hash(&env, 1));
    }));

    assert!(result.is_err());
    assert_eq!(client.get_version(), version);
    assert!(client.get_previous_version().is_none());
    assert_eq!(client.get_admin(), Some(admin));
}

#[test]
fn pending_state_rejects_outsider_and_preserves_state() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, signers) = setup(&env);
    let outsider = Address::generate(&env);
    let proposal_id = client.propose_upgrade(&signers[0], &wasm_hash(&env, 1), &0);
    let proposal = client.get_upgrade_proposal(&proposal_id).unwrap();
    let version = client.get_version();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.approve_upgrade(&proposal_id, &outsider);
    }));

    assert!(result.is_err());
    assert_unchanged(&client, proposal_id, &proposal, version, None, None);
}

#[test]
fn executable_state_requires_threshold_and_timelock() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, signers) = setup(&env);
    let proposal_id = client.propose_upgrade(&signers[0], &wasm_hash(&env, 2), &0);
    client.approve_upgrade(&proposal_id, &signers[0]);
    client.approve_upgrade(&proposal_id, &signers[1]);
    assert_eq!(client.get_timelock_status(&proposal_id), Some(86400));

    env.ledger().set_timestamp(env.ledger().timestamp() + 86400);
    assert!(client.can_execute(&proposal_id));

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.execute_upgrade(&proposal_id);
    }));
    assert!(result.is_err());
    assert!(client.get_version() > 0);
    assert!(client.get_previous_version().is_none());
}

#[test]
fn expired_state_rejects_approval_and_execution_without_changes() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, signers) = setup(&env);
    let expiry = env.ledger().timestamp() + 100;
    let proposal_id = client.propose_upgrade(&signers[0], &wasm_hash(&env, 3), &expiry);
    client.approve_upgrade(&proposal_id, &signers[0]);
    client.approve_upgrade(&proposal_id, &signers[1]);
    let proposal = client.get_upgrade_proposal(&proposal_id).unwrap();
    let version = client.get_version();
    let timelock = client.get_timelock_status(&proposal_id);
    env.ledger().set_timestamp(expiry);

    for action in [0, 1] {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if action == 0 {
                client.approve_upgrade(&proposal_id, &signers[2]);
            } else {
                client.execute_upgrade(&proposal_id);
            }
        }));
        assert!(result.is_err());
        assert_unchanged(&client, proposal_id, &proposal, version, None, timelock);
    }
}

#[test]
fn cancelled_state_rejects_approval_and_execution_without_changes() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, signers) = setup(&env);
    let proposal_id = client.propose_upgrade(&signers[0], &wasm_hash(&env, 4), &0);
    client.cancel_upgrade(&proposal_id, &signers[0]);
    let proposal = client.get_upgrade_proposal(&proposal_id).unwrap();
    let version = client.get_version();

    for action in [0, 1] {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if action == 0 {
                client.approve_upgrade(&proposal_id, &signers[1]);
            } else {
                client.execute_upgrade(&proposal_id);
            }
        }));
        assert!(result.is_err());
        assert_unchanged(&client, proposal_id, &proposal, version, None, None);
    }
}