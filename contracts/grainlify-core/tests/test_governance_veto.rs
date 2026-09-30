#![cfg(test)]

use grainlify_core::governance::{
    Error, GovernanceConfig, GovernanceContract, ProposalStatus, VotingScheme,
};
use soroban_sdk::{symbol_short, Address, BytesN, Env, Symbol};
use grainlify_core::governance::{Error, GovernanceConfig, ProposalStatus, VoteType, VotingScheme};
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::{symbol_short, Address, BytesN, Env};

mod gov_support;

fn zero_address(env: &Env) -> Address {
    // StrKey for the all-zero contract id — the canonical "zero address".
    use soroban_sdk::String;
    Address::from_string(&String::from_str(
        env,
        "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM",
    ))
}

/// Generates a fresh address that is guaranteed not to be the all-zero contract
/// address.
///
/// `Address::generate` yields the all-zero address as its first value on a fresh
/// `Env`, and the governance contract rejects that address as a role holder, so
/// the first (zero) value is skipped.
fn actor(env: &Env) -> Address {
    let candidate = Address::generate(env);
    if candidate == zero_address(env) {
        Address::generate(env)
    } else {
        candidate
    }
}

#[test]
fn test_veto_proposal_success() {
    let env = Env::default();
    gov_support::reset();
    env.mock_all_auths();

    let admin = actor(&env);
    let security_council = actor(&env);
    let proposer = actor(&env);
    let dummy_hash = BytesN::from_array(&env, &[0u8; 32]);

    // Initialize governance
    let config = GovernanceConfig {
        voting_period: 100,
        execution_delay: 50,
        quorum_percentage: 100,
        approval_threshold: 6000,
        min_proposal_stake: 0,
        voting_scheme: VotingScheme::OnePersonOneVote,
        governance_token: actor(&env),
    };

    GovernanceContract::init_governance_state(env.clone(), admin.clone(), config).unwrap();
    GovernanceContract::set_security_council(env.clone(), admin.clone(), security_council.clone())
    gov_support::init_governance_state(env.clone(), admin.clone(), config).unwrap();
    gov_support::set_security_council(env.clone(), admin.clone(), security_council.clone())
        .unwrap();

    // Create proposal
    let proposal_id = gov_support::create_proposal(
        env.clone(),
        proposer.clone(),
        dummy_hash.clone(),
        symbol_short!("test"),
    )
    .unwrap();

    // One "For" vote meets the 1% quorum.
    gov_support::cast_vote(env.clone(), proposer.clone(), proposal_id, VoteType::For).unwrap();

    // Advance past the voting period but stay inside the veto window.
    env.ledger().set_timestamp(120);

    // Finalize proposal (approved: quorum met, all decisive votes in favour)
    let status = gov_support::finalize_proposal(env.clone(), proposal_id).unwrap();
    assert_eq!(status, ProposalStatus::Approved);

    // Veto the proposal during timelock period
    gov_support::veto_proposal(env.clone(), security_council.clone(), proposal_id).unwrap();

    // Verify proposal is vetoed by checking it cannot be executed
    let upgrade_executor = actor(&env);
    gov_support::set_upgrade_role(env.clone(), admin.clone(), upgrade_executor.clone()).unwrap();
    let result = gov_support::execute_proposal(env.clone(), upgrade_executor, proposal_id);
    assert_eq!(result, Err(Error::ProposalNotApproved));
}

#[test]
fn test_veto_proposal_not_security_council() {
    let env = Env::default();
    gov_support::reset();
    env.mock_all_auths();

    let admin = actor(&env);
    let security_council = actor(&env);
    let unauthorized = actor(&env);
    let proposer = actor(&env);
    let dummy_hash = BytesN::from_array(&env, &[0u8; 32]);

    let config = GovernanceConfig {
        voting_period: 100,
        execution_delay: 50,
        quorum_percentage: 5000,
        approval_threshold: 6000,
        min_proposal_stake: 0,
        voting_scheme: VotingScheme::OnePersonOneVote,
        governance_token: actor(&env),
    };

    GovernanceContract::init_governance_state(env.clone(), admin.clone(), config).unwrap();
    GovernanceContract::set_security_council(env.clone(), admin.clone(), security_council.clone())
    gov_support::init_governance_state(env.clone(), admin.clone(), config).unwrap();
    gov_support::set_security_council(env.clone(), admin.clone(), security_council.clone())
        .unwrap();

    let proposal_id = gov_support::create_proposal(
        env.clone(),
        proposer.clone(),
        dummy_hash.clone(),
        symbol_short!("test"),
    )
    .unwrap();

    env.ledger().set_timestamp(150);
    gov_support::finalize_proposal(env.clone(), proposal_id).unwrap();

    // Try to veto with unauthorized address
    let result = gov_support::veto_proposal(env.clone(), unauthorized.clone(), proposal_id);
    assert_eq!(result, Err(Error::NotSecurityCouncil));
}

#[test]
fn test_veto_proposal_security_council_not_set() {
    let env = Env::default();
    gov_support::reset();
    env.mock_all_auths();

    let admin = actor(&env);
    let security_council = actor(&env);
    let proposer = actor(&env);
    let dummy_hash = BytesN::from_array(&env, &[0u8; 32]);

    let config = GovernanceConfig {
        voting_period: 100,
        execution_delay: 50,
        quorum_percentage: 5000,
        approval_threshold: 6000,
        min_proposal_stake: 0,
        voting_scheme: VotingScheme::OnePersonOneVote,
        governance_token: actor(&env),
    };

    gov_support::init_governance_state(env.clone(), admin.clone(), config).unwrap();
    // Note: Security Council NOT set

    let proposal_id = gov_support::create_proposal(
        env.clone(),
        proposer.clone(),
        dummy_hash.clone(),
        symbol_short!("test"),
    )
    .unwrap();

    env.ledger().set_timestamp(150);
    gov_support::finalize_proposal(env.clone(), proposal_id).unwrap();

    // Try to veto without Security Council set
    let result =
        GovernanceContract::veto_proposal(env.clone(), security_council.clone(), proposal_id);
    let result = gov_support::veto_proposal(env.clone(), security_council.clone(), proposal_id);
    assert_eq!(result, Err(Error::SecurityCouncilNotSet));
}

#[test]
fn test_veto_proposal_not_approved() {
    let env = Env::default();
    gov_support::reset();
    env.mock_all_auths();

    let admin = actor(&env);
    let security_council = actor(&env);
    let proposer = actor(&env);
    let dummy_hash = BytesN::from_array(&env, &[0u8; 32]);

    let config = GovernanceConfig {
        voting_period: 100,
        execution_delay: 50,
        quorum_percentage: 5000,
        approval_threshold: 6000,
        min_proposal_stake: 0,
        voting_scheme: VotingScheme::OnePersonOneVote,
        governance_token: actor(&env),
    };

    GovernanceContract::init_governance_state(env.clone(), admin.clone(), config).unwrap();
    GovernanceContract::set_security_council(env.clone(), admin.clone(), security_council.clone())
    gov_support::init_governance_state(env.clone(), admin.clone(), config).unwrap();
    gov_support::set_security_council(env.clone(), admin.clone(), security_council.clone())
        .unwrap();

    let proposal_id = gov_support::create_proposal(
        env.clone(),
        proposer.clone(),
        dummy_hash.clone(),
        symbol_short!("test"),
    )
    .unwrap();

    // Try to veto while proposal is still Active
    let result =
        GovernanceContract::veto_proposal(env.clone(), security_council.clone(), proposal_id);
    let result = gov_support::veto_proposal(env.clone(), security_council.clone(), proposal_id);
    assert_eq!(result, Err(Error::CannotVeto));
}

#[test]
fn test_veto_proposal_after_timelock() {
    let env = Env::default();
    gov_support::reset();
    env.mock_all_auths();

    let admin = actor(&env);
    let security_council = actor(&env);
    let proposer = actor(&env);
    let dummy_hash = BytesN::from_array(&env, &[0u8; 32]);

    let config = GovernanceConfig {
        voting_period: 100,
        execution_delay: 50,
        quorum_percentage: 5000,
        approval_threshold: 6000,
        min_proposal_stake: 0,
        voting_scheme: VotingScheme::OnePersonOneVote,
        governance_token: actor(&env),
    };

    GovernanceContract::init_governance_state(env.clone(), admin.clone(), config).unwrap();
    GovernanceContract::set_security_council(env.clone(), admin.clone(), security_council.clone())
    gov_support::init_governance_state(env.clone(), admin.clone(), config).unwrap();
    gov_support::set_security_council(env.clone(), admin.clone(), security_council.clone())
        .unwrap();

    let proposal_id = gov_support::create_proposal(
        env.clone(),
        proposer.clone(),
        dummy_hash.clone(),
        symbol_short!("test"),
    )
    .unwrap();

    env.ledger().set_timestamp(150);
    gov_support::finalize_proposal(env.clone(), proposal_id).unwrap();

    // Advance time past timelock period
    env.ledger().set_timestamp(201);

    // Try to veto after timelock has passed
    let result =
        GovernanceContract::veto_proposal(env.clone(), security_council.clone(), proposal_id);
    let result = gov_support::veto_proposal(env.clone(), security_council.clone(), proposal_id);
    assert_eq!(result, Err(Error::CannotVeto));
}

#[test]
fn test_set_and_get_security_council() {
    let env = Env::default();
    gov_support::reset();
    env.mock_all_auths();

    let admin = actor(&env);
    let security_council = actor(&env);

    let config = GovernanceConfig {
        voting_period: 100,
        execution_delay: 50,
        quorum_percentage: 5000,
        approval_threshold: 6000,
        min_proposal_stake: 0,
        voting_scheme: VotingScheme::OnePersonOneVote,
        governance_token: actor(&env),
    };

    gov_support::init_governance_state(env.clone(), admin.clone(), config).unwrap();

    // Set Security Council
    GovernanceContract::set_security_council(env.clone(), admin.clone(), security_council.clone())
    gov_support::set_security_council(env.clone(), admin.clone(), security_council.clone())
        .unwrap();

    // Get Security Council
    let retrieved = gov_support::get_security_council(env.clone()).unwrap();
    assert_eq!(retrieved, security_council);
}

#[test]
fn test_vetoed_proposal_cannot_be_executed() {
    let env = Env::default();
    gov_support::reset();
    env.mock_all_auths();

    let admin = actor(&env);
    let security_council = actor(&env);
    let proposer = actor(&env);
    let dummy_hash = BytesN::from_array(&env, &[0u8; 32]);

    let config = GovernanceConfig {
        voting_period: 100,
        execution_delay: 50,
        quorum_percentage: 100,
        approval_threshold: 6000,
        min_proposal_stake: 0,
        voting_scheme: VotingScheme::OnePersonOneVote,
        governance_token: actor(&env),
    };

    GovernanceContract::init_governance_state(env.clone(), admin.clone(), config).unwrap();
    GovernanceContract::set_security_council(env.clone(), admin.clone(), security_council.clone())
    gov_support::init_governance_state(env.clone(), admin.clone(), config).unwrap();
    gov_support::set_security_council(env.clone(), admin.clone(), security_council.clone())
        .unwrap();

    let proposal_id = gov_support::create_proposal(
        env.clone(),
        proposer.clone(),
        dummy_hash.clone(),
        symbol_short!("test"),
    )
    .unwrap();

    gov_support::cast_vote(env.clone(), proposer.clone(), proposal_id, VoteType::For).unwrap();
    env.ledger().set_timestamp(120);
    gov_support::finalize_proposal(env.clone(), proposal_id).unwrap();

    // Veto the proposal
    gov_support::veto_proposal(env.clone(), security_council.clone(), proposal_id).unwrap();

    // Try to execute vetoed proposal
    let upgrade_executor = actor(&env);
    gov_support::set_upgrade_role(env.clone(), admin.clone(), upgrade_executor.clone()).unwrap();
    let result = gov_support::execute_proposal(env.clone(), upgrade_executor, proposal_id);
    assert_eq!(result, Err(Error::ProposalNotApproved));
}
