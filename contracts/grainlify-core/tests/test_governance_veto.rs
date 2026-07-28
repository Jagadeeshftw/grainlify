#![cfg(test)]

use soroban_sdk::{symbol_short, Address, BytesN, Env, Symbol};
use grainlify_core::governance::{
    GovernanceContract, GovernanceConfig, ProposalStatus, VotingScheme, Error,
};

#[test]
fn test_veto_proposal_success() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let security_council = Address::generate(&env);
    let proposer = Address::generate(&env);
    let dummy_hash = BytesN::from_array(&env, &[0u8; 32]);

    // Initialize governance
    let config = GovernanceConfig {
        voting_period: 100,
        execution_delay: 50,
        quorum_percentage: 5000,
        approval_threshold: 6000,
        min_proposal_stake: 0,
        voting_scheme: VotingScheme::OnePersonOneVote,
        governance_token: Address::generate(&env),
    };

    GovernanceContract::init_governance_state(env.clone(), admin.clone(), config).unwrap();
    GovernanceContract::set_security_council(env.clone(), admin.clone(), security_council.clone()).unwrap();

    // Create proposal
    let proposal_id = GovernanceContract::create_proposal(
        env.clone(),
        proposer.clone(),
        dummy_hash.clone(),
        symbol_short!("test"),
    ).unwrap();

    // Advance time past voting period
    env.ledger().set_timestamp(150);

    // Finalize proposal (should be approved with mock quorum)
    let status = GovernanceContract::finalize_proposal(env.clone(), proposal_id).unwrap();
    assert_eq!(status, ProposalStatus::Approved);

    // Veto the proposal during timelock period
    GovernanceContract::veto_proposal(env.clone(), security_council.clone(), proposal_id).unwrap();

    // Verify proposal is vetoed by checking it cannot be executed
    let result = GovernanceContract::execute_proposal(env.clone(), proposal_id);
    assert_eq!(result, Err(Error::ProposalNotApproved));
}

#[test]
fn test_veto_proposal_not_security_council() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let security_council = Address::generate(&env);
    let unauthorized = Address::generate(&env);
    let proposer = Address::generate(&env);
    let dummy_hash = BytesN::from_array(&env, &[0u8; 32]);

    let config = GovernanceConfig {
        voting_period: 100,
        execution_delay: 50,
        quorum_percentage: 5000,
        approval_threshold: 6000,
        min_proposal_stake: 0,
        voting_scheme: VotingScheme::OnePersonOneVote,
        governance_token: Address::generate(&env),
    };

    GovernanceContract::init_governance_state(env.clone(), admin.clone(), config).unwrap();
    GovernanceContract::set_security_council(env.clone(), admin.clone(), security_council.clone()).unwrap();

    let proposal_id = GovernanceContract::create_proposal(
        env.clone(),
        proposer.clone(),
        dummy_hash.clone(),
        symbol_short!("test"),
    ).unwrap();

    env.ledger().set_timestamp(150);
    GovernanceContract::finalize_proposal(env.clone(), proposal_id).unwrap();

    // Try to veto with unauthorized address
    let result = GovernanceContract::veto_proposal(env.clone(), unauthorized.clone(), proposal_id);
    assert_eq!(result, Err(Error::NotSecurityCouncil));
}

#[test]
fn test_veto_proposal_security_council_not_set() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let security_council = Address::generate(&env);
    let proposer = Address::generate(&env);
    let dummy_hash = BytesN::from_array(&env, &[0u8; 32]);

    let config = GovernanceConfig {
        voting_period: 100,
        execution_delay: 50,
        quorum_percentage: 5000,
        approval_threshold: 6000,
        min_proposal_stake: 0,
        voting_scheme: VotingScheme::OnePersonOneVote,
        governance_token: Address::generate(&env),
    };

    GovernanceContract::init_governance_state(env.clone(), admin.clone(), config).unwrap();
    // Note: Security Council NOT set

    let proposal_id = GovernanceContract::create_proposal(
        env.clone(),
        proposer.clone(),
        dummy_hash.clone(),
        symbol_short!("test"),
    ).unwrap();

    env.ledger().set_timestamp(150);
    GovernanceContract::finalize_proposal(env.clone(), proposal_id).unwrap();

    // Try to veto without Security Council set
    let result = GovernanceContract::veto_proposal(env.clone(), security_council.clone(), proposal_id);
    assert_eq!(result, Err(Error::SecurityCouncilNotSet));
}

#[test]
fn test_veto_proposal_not_approved() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let security_council = Address::generate(&env);
    let proposer = Address::generate(&env);
    let dummy_hash = BytesN::from_array(&env, &[0u8; 32]);

    let config = GovernanceConfig {
        voting_period: 100,
        execution_delay: 50,
        quorum_percentage: 5000,
        approval_threshold: 6000,
        min_proposal_stake: 0,
        voting_scheme: VotingScheme::OnePersonOneVote,
        governance_token: Address::generate(&env),
    };

    GovernanceContract::init_governance_state(env.clone(), admin.clone(), config).unwrap();
    GovernanceContract::set_security_council(env.clone(), admin.clone(), security_council.clone()).unwrap();

    let proposal_id = GovernanceContract::create_proposal(
        env.clone(),
        proposer.clone(),
        dummy_hash.clone(),
        symbol_short!("test"),
    ).unwrap();

    // Try to veto while proposal is still Active
    let result = GovernanceContract::veto_proposal(env.clone(), security_council.clone(), proposal_id);
    assert_eq!(result, Err(Error::CannotVeto));
}

#[test]
fn test_veto_proposal_after_timelock() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let security_council = Address::generate(&env);
    let proposer = Address::generate(&env);
    let dummy_hash = BytesN::from_array(&env, &[0u8; 32]);

    let config = GovernanceConfig {
        voting_period: 100,
        execution_delay: 50,
        quorum_percentage: 5000,
        approval_threshold: 6000,
        min_proposal_stake: 0,
        voting_scheme: VotingScheme::OnePersonOneVote,
        governance_token: Address::generate(&env),
    };

    GovernanceContract::init_governance_state(env.clone(), admin.clone(), config).unwrap();
    GovernanceContract::set_security_council(env.clone(), admin.clone(), security_council.clone()).unwrap();

    let proposal_id = GovernanceContract::create_proposal(
        env.clone(),
        proposer.clone(),
        dummy_hash.clone(),
        symbol_short!("test"),
    ).unwrap();

    env.ledger().set_timestamp(150);
    GovernanceContract::finalize_proposal(env.clone(), proposal_id).unwrap();

    // Advance time past timelock period
    env.ledger().set_timestamp(201);

    // Try to veto after timelock has passed
    let result = GovernanceContract::veto_proposal(env.clone(), security_council.clone(), proposal_id);
    assert_eq!(result, Err(Error::CannotVeto));
}

#[test]
fn test_set_and_get_security_council() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let security_council = Address::generate(&env);

    let config = GovernanceConfig {
        voting_period: 100,
        execution_delay: 50,
        quorum_percentage: 5000,
        approval_threshold: 6000,
        min_proposal_stake: 0,
        voting_scheme: VotingScheme::OnePersonOneVote,
        governance_token: Address::generate(&env),
    };

    GovernanceContract::init_governance_state(env.clone(), admin.clone(), config).unwrap();

    // Set Security Council
    GovernanceContract::set_security_council(env.clone(), admin.clone(), security_council.clone()).unwrap();

    // Get Security Council
    let retrieved = GovernanceContract::get_security_council(env.clone()).unwrap();
    assert_eq!(retrieved, security_council);
}

#[test]
fn test_vetoed_proposal_cannot_be_executed() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let security_council = Address::generate(&env);
    let proposer = Address::generate(&env);
    let dummy_hash = BytesN::from_array(&env, &[0u8; 32]);

    let config = GovernanceConfig {
        voting_period: 100,
        execution_delay: 50,
        quorum_percentage: 5000,
        approval_threshold: 6000,
        min_proposal_stake: 0,
        voting_scheme: VotingScheme::OnePersonOneVote,
        governance_token: Address::generate(&env),
    };

    GovernanceContract::init_governance_state(env.clone(), admin.clone(), config).unwrap();
    GovernanceContract::set_security_council(env.clone(), admin.clone(), security_council.clone()).unwrap();

    let proposal_id = GovernanceContract::create_proposal(
        env.clone(),
        proposer.clone(),
        dummy_hash.clone(),
        symbol_short!("test"),
    ).unwrap();

    env.ledger().set_timestamp(150);
    GovernanceContract::finalize_proposal(env.clone(), proposal_id).unwrap();

    // Veto the proposal
    GovernanceContract::veto_proposal(env.clone(), security_council.clone(), proposal_id).unwrap();

    // Try to execute vetoed proposal
    let result = GovernanceContract::execute_proposal(env.clone(), proposal_id);
    assert_eq!(result, Err(Error::ProposalNotApproved));
}
