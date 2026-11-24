#![cfg(test)]

use crate::{
    governance, monitoring, DataKey, GovernanceConfig, GrainlifyContract, GrainlifyContractClient,
    VotingScheme,
};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, String, String as SdkString, Symbol, Vec,
};

fn setup_contract(env: &Env) -> (GrainlifyContractClient<'_>, Address) {
    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.init_admin(&admin);
    (client, admin)
}

fn default_governance_config(env: &Env) -> GovernanceConfig {
    GovernanceConfig {
        voting_period: 86400,
        execution_delay: 3600,
        quorum_percentage: 4000,
        approval_threshold: 6000,
        min_proposal_stake: 1000,
        voting_scheme: VotingScheme::OnePersonOneVote,
        governance_token: Address::generate(env),
    }
}

// ============================================================================
// Initialization Path Tests: init()
// ============================================================================

/// Tests successful initialization via the multisig path (init)
#[test]
fn test_init_multisig_success() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let signers = Vec::from_array(&env, [signer1, signer2]);

    // Initialize with multisig
    client.init(&signers, &2u32);

    // Verify version is set
    let version = client.get_version();
    assert_eq!(version, 2); // VERSION constant = 2
}

/// Tests that multisig init prevents re-initialization
#[test]
#[should_panic(expected = "Already initialized")]
fn test_init_multisig_prevents_reinit() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let signers = Vec::from_array(&env, [signer1, signer2]);

    // First initialization
    client.init(&signers, &2u32);

    // Second initialization should panic
    client.init(&signers, &2u32);
}

/// Tests that init_admin prevents multisig init (via Version storage)
#[test]
#[should_panic(expected = "Already initialized")]
fn test_init_multisig_blocked_after_init_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let signers = Vec::from_array(&env, [signer1.clone(), signer2]);

    // Initialize with admin
    client.init_admin(&admin);

    // Attempt multisig init should panic
    client.init(&signers, &2u32);
}

// ============================================================================
// Initialization Path Tests: init_admin()
// ============================================================================

/// Tests successful initialization via the single-admin path
#[test]
fn test_init_admin_success() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);

    // Verify admin is set
    let report = client.check_invariants();
    assert!(report.admin_set);
    assert!(report.version_set);
    assert_eq!(report.version, 2); // VERSION constant = 2
}

/// Tests that init_admin prevents re-initialization
#[test]
#[should_panic(expected = "Already initialized")]
fn test_init_admin_prevents_reinit() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);

    // First initialization
    client.init_admin(&admin);

    // Second initialization should panic
    client.init_admin(&admin);
}

/// Tests that multisig init prevents init_admin (via Version storage)
#[test]
#[should_panic(expected = "Already initialized")]
fn test_init_admin_blocked_after_init_multisig() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let signers = Vec::from_array(&env, [signer1, signer2]);

    // Initialize with multisig
    client.init(&signers, &2u32);

    // Attempt admin init should panic
    client.init_admin(&admin);
}

// ============================================================================
// Initialization Path Tests: init_governance()
// ============================================================================

/// Tests successful initialization via the governance path
#[test]
fn test_init_governance_success() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let gov_config = default_governance_config(&env);

    // Initialize with governance
    client.init_governance(&admin, &gov_config);

    // Verify version is set
    let version = client.get_version();
    assert_eq!(version, 2); // VERSION constant = 2

    env.as_contract(&client.address, || {
        let stored_config: GovernanceConfig = env
            .storage()
            .instance()
            .get(&governance::GOVERNANCE_CONFIG)
            .unwrap();
        let proposal_count: u32 = env
            .storage()
            .instance()
            .get(&governance::PROPOSAL_COUNT)
            .unwrap();

        assert_eq!(stored_config, gov_config);
        assert_eq!(proposal_count, 0);
    });
}

/// Tests that init_governance prevents re-initialization
#[test]
#[should_panic(expected = "Already initialized")]
fn test_init_governance_prevents_reinit() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let gov_config = GovernanceConfig {
        voting_period: 86400,
        execution_delay: 3600,
        quorum_percentage: 4000,
        approval_threshold: 6000,
        min_proposal_stake: 1000,
        voting_scheme: VotingScheme::OnePersonOneVote,
        governance_token: Address::generate(&env),
    };

    // First initialization
    client.init_governance(&admin, &gov_config);

    // Second initialization should panic
    client.init_governance(&admin, &gov_config);
}

/// Tests that init_admin prevents init_governance
#[test]
#[should_panic(expected = "Already initialized")]
fn test_init_governance_blocked_after_init_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let gov_config = GovernanceConfig {
        voting_period: 86400,
        execution_delay: 3600,
        quorum_percentage: 4000,
        approval_threshold: 6000,
        min_proposal_stake: 1000,
        voting_scheme: VotingScheme::OnePersonOneVote,
        governance_token: Address::generate(&env),
    };

    // First initialize with admin
    client.init_admin(&admin);

    // Second initialization via governance should panic
    client.init_governance(&admin, &gov_config);
}

// ============================================================================
// Governance Configuration Validation Tests
// ============================================================================

/// Tests that invalid quorum percentage is rejected
#[test]
#[should_panic(expected = "Invalid governance threshold")]
fn test_init_governance_rejects_invalid_quorum() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let gov_config = GovernanceConfig {
        voting_period: 86400,
        execution_delay: 3600,
        quorum_percentage: 10001, // Invalid: > 10000
        approval_threshold: 6000,
        min_proposal_stake: 1000,
        voting_scheme: VotingScheme::OnePersonOneVote,
        governance_token: Address::generate(&env),
    };

    client.init_governance(&admin, &gov_config);
}

/// Tests that invalid approval threshold is rejected
#[test]
#[should_panic(expected = "Invalid governance threshold")]
fn test_init_governance_rejects_invalid_approval_threshold() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let gov_config = GovernanceConfig {
        voting_period: 86400,
        execution_delay: 3600,
        quorum_percentage: 4000,
        approval_threshold: 10001, // Invalid: > 10000
        min_proposal_stake: 1000,
        voting_scheme: VotingScheme::OnePersonOneVote,
        governance_token: Address::generate(&env),
    };

    client.init_governance(&admin, &gov_config);
}

/// Tests that approval threshold below 50% (5000 bps) is rejected
#[test]
#[should_panic(expected = "Approval threshold too low")]
fn test_init_governance_rejects_low_approval_threshold() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let gov_config = GovernanceConfig {
        voting_period: 86400,
        execution_delay: 3600,
        quorum_percentage: 4000,
        approval_threshold: 4999, // Invalid: < 5000
        min_proposal_stake: 1000,
        voting_scheme: VotingScheme::OnePersonOneVote,
        governance_token: Address::generate(&env),
    };

    client.init_governance(&admin, &gov_config);
}

/// Tests that minimum approval threshold of exactly 50% is accepted
#[test]
fn test_init_governance_accepts_exact_threshold() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let gov_config = GovernanceConfig {
        voting_period: 86400,
        execution_delay: 3600,
        quorum_percentage: 4000,
        approval_threshold: 5000, // Exactly 50%
        min_proposal_stake: 1000,
        voting_scheme: VotingScheme::OnePersonOneVote,
        governance_token: Address::generate(&env),
    };

    client.init_governance(&admin, &gov_config);

    // Verify successful initialization
    let version = client.get_version();
    assert_eq!(version, 2);
}

/// Tests maximum valid thresholds (100%)
#[test]
fn test_init_governance_accepts_max_thresholds() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let gov_config = GovernanceConfig {
        voting_period: 86400,
        execution_delay: 3600,
        quorum_percentage: 10000,  // 100%
        approval_threshold: 10000, // 100%
        min_proposal_stake: 1000,
        voting_scheme: VotingScheme::OnePersonOneVote,
        governance_token: Address::generate(&env),
    };

    client.init_governance(&admin, &gov_config);

    // Verify successful initialization
    let version = client.get_version();
    assert_eq!(version, 2);
}

// ============================================================================
// Invariants After Initialization
// ============================================================================

/// Tests that contract is healthy after init_admin
#[test]
fn test_monitoring_views_are_safe_on_empty_state() {
    let env = Env::default();
    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    let health = client.health_check();
    assert!(!health.is_healthy);
    assert_eq!(health.last_operation, 0);
    assert_eq!(health.total_operations, 0);
    assert_eq!(health.contract_version, SdkString::from_str(&env, "0.0.0"));

    let analytics = client.get_analytics();
    assert_eq!(analytics.operation_count, 0);
    assert_eq!(analytics.unique_users, 0);
    assert_eq!(analytics.error_count, 0);
    assert_eq!(analytics.error_rate, 0);
}

#[test]
fn test_monitoring_views_report_initialized_state() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|ledger| ledger.timestamp = 7);

    let (client, _admin) = setup_contract(&env);

    let health = client.health_check();
    assert!(health.is_healthy);
    assert_eq!(health.last_operation, 7);
    assert_eq!(health.total_operations, 1);
    assert_eq!(health.contract_version, SdkString::from_str(&env, "2.0.0"));

    let analytics = client.get_analytics();
    assert_eq!(analytics.operation_count, 1);
    assert_eq!(analytics.unique_users, 1);
    assert_eq!(analytics.error_count, 0);
    assert_eq!(analytics.error_rate, 0);
}

#[test]
fn test_monitoring_unique_user_count_is_bounded() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    env.ledger().with_mut(|ledger| ledger.timestamp = 99);
    env.as_contract(&client.address, || {
        for index in 0..(monitoring::MAX_TRACKED_USERS + 5) {
            let caller = Address::generate(&env);
            let operation = Symbol::new(&env, if index % 2 == 0 { "ping" } else { "pong" });
            monitoring::track_operation(&env, operation, caller, true);
        }
    });

    let health = client.health_check();
    assert_eq!(health.last_operation, 99);
    assert_eq!(
        health.total_operations,
        monitoring::MAX_TRACKED_USERS as u64 + 6
    );

    let analytics = client.get_analytics();
    assert_eq!(
        analytics.operation_count,
        monitoring::MAX_TRACKED_USERS as u64 + 6
    );
    assert_eq!(analytics.unique_users, monitoring::MAX_TRACKED_USERS as u64);
    assert_eq!(analytics.error_count, 0);
    assert_eq!(analytics.error_rate, 0);
}

#[test]
fn test_check_invariants_healthy_after_init() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let report = client.check_invariants();
    assert!(report.healthy);
    assert!(report.config_sane);
    assert!(report.metrics_sane);
    assert!(report.admin_set);
    assert!(report.version_set);
    assert_eq!(report.violation_count, 0);
    assert!(client.verify_invariants());
}

/// Tests that contract is healthy after init_governance
#[test]
fn test_check_invariants_healthy_after_init_governance() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let gov_config = GovernanceConfig {
        voting_period: 86400,
        execution_delay: 3600,
        quorum_percentage: 4000,
        approval_threshold: 6000,
        min_proposal_stake: 1000,
        voting_scheme: VotingScheme::OnePersonOneVote,
        governance_token: Address::generate(&env),
    };

    client.init_governance(&admin, &gov_config);

    let report = client.check_invariants();
    assert!(report.healthy);
    assert!(report.config_sane);
    assert!(report.metrics_sane);
    assert!(report.admin_set);
    assert!(report.version_set);
    assert_eq!(report.violation_count, 0);
}

#[test]
fn test_check_invariants_detects_metric_drift() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    env.as_contract(&client.address, || {
        let op_key = Symbol::new(&env, "op_count");
        let err_key = Symbol::new(&env, "err_count");
        env.storage().persistent().set(&op_key, &2_u64);
        env.storage().persistent().set(&err_key, &5_u64);
    });

    let report = client.check_invariants();
    assert!(report.config_sane);
    assert!(!report.metrics_sane);
    assert!(!report.healthy);
    assert!(report.violation_count > 0);
    assert!(!client.verify_invariants());
}

#[test]
fn test_check_invariants_detects_config_drift() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    env.as_contract(&client.address, || {
        env.storage().instance().remove(&DataKey::Version);
    });

    let report = client.check_invariants();
    assert!(!report.config_sane);
    assert!(!report.healthy);
    assert!(report.violation_count > 0);
    assert!(!client.verify_invariants());
}

// ============================================================================
// Cross-Path Initialization Interaction Tests
// ============================================================================

/// Tests that all three init paths are mutually exclusive
#[test]
fn test_all_init_paths_mutually_exclusive() {
    let env = Env::default();
    env.mock_all_auths();

    // Test 1: init -> init_admin fails
    {
        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let signer1 = Address::generate(&env);
        let signer2 = Address::generate(&env);
        let signers = Vec::from_array(&env, [signer1, signer2.clone()]);

        client.init(&signers, &2u32);

        // Try to initialize with admin - should fail
        let result = client.try_init_admin(&signer2);
        assert!(result.is_err());
    }

    // Test 2: init -> init_governance fails
    {
        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let signer1 = Address::generate(&env);
        let signer2 = Address::generate(&env);
        let signers = Vec::from_array(&env, [signer1, signer2.clone()]);

        let admin = Address::generate(&env);
        let gov_config = GovernanceConfig {
            voting_period: 86400,
            execution_delay: 3600,
            quorum_percentage: 4000,
            approval_threshold: 6000,
            min_proposal_stake: 1000,
            voting_scheme: VotingScheme::OnePersonOneVote,
            governance_token: Address::generate(&env),
        };

        client.init(&signers, &2u32);

        // Try to initialize with governance - should fail
        let result = client.try_init_governance(&admin, &gov_config);
        assert!(result.is_err());
    }

    // Test 3: init_admin -> init fails
    {
        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.init_admin(&admin);

        let signer1 = Address::generate(&env);
        let signer2 = Address::generate(&env);
        let signers = Vec::from_array(&env, [signer1, signer2]);

        // Try to initialize with multisig - should fail
        let result = client.try_init(&signers, &2u32);
        assert!(result.is_err());
    }

    // Test 4: init_admin -> init_governance fails
    {
        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.init_admin(&admin);

        let gov_config = GovernanceConfig {
            voting_period: 86400,
            execution_delay: 3600,
            quorum_percentage: 4000,
            approval_threshold: 6000,
            min_proposal_stake: 1000,
            voting_scheme: VotingScheme::OnePersonOneVote,
            governance_token: Address::generate(&env),
        };

        // Try to initialize with governance - should fail
        let result = client.try_init_governance(&admin, &gov_config);
        assert!(result.is_err());
    }
}

/// Tests that multisig initialization also blocks the legacy network init path.
#[test]
fn test_init_with_network_blocked_after_multisig_init() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let signers = Vec::from_array(&env, [signer1, signer2]);
    client.init(&signers, &2u32);

    let admin = Address::generate(&env);
    let chain_id = String::from_str(&env, "stellar");
    let network_id = String::from_str(&env, "testnet");
    let result = client.try_init_with_network(&admin, &chain_id, &network_id);

    assert!(result.is_err());
}

/// Tests that governance initialization also blocks the legacy network init path.
#[test]
fn test_init_with_network_blocked_after_governance_init() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let gov_config = default_governance_config(&env);
    client.init_governance(&admin, &gov_config);

    let chain_id = String::from_str(&env, "stellar");
    let network_id = String::from_str(&env, "testnet");
    let result = client.try_init_with_network(&admin, &chain_id, &network_id);

    assert!(result.is_err());
}

// ============================================================================
// Edge Cases and Corner Cases
// ============================================================================

/// Tests init_governance with all voting schemes
#[test]
fn test_init_governance_with_token_weighted_voting() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let gov_config = GovernanceConfig {
        voting_period: 86400,
        execution_delay: 3600,
        quorum_percentage: 4000,
        approval_threshold: 6000,
        min_proposal_stake: 1000,
        voting_scheme: VotingScheme::TokenWeighted,
        governance_token: Address::generate(&env),
    };

    client.init_governance(&admin, &gov_config);

    // Verify successful initialization
    let version = client.get_version();
    assert_eq!(version, 2);
}

/// Tests init_governance with zero voting period
#[test]
fn test_init_governance_with_zero_voting_period() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let gov_config = GovernanceConfig {
        voting_period: 0, // Edge case: zero voting period
        execution_delay: 3600,
        quorum_percentage: 4000,
        approval_threshold: 6000,
        min_proposal_stake: 1000,
        voting_scheme: VotingScheme::OnePersonOneVote,
        governance_token: Address::generate(&env),
    };

    // Should still succeed (governance doesn't validate voting period)
    client.init_governance(&admin, &gov_config);

    let version = client.get_version();
    assert_eq!(version, 2);
}

/// Tests init_governance with max valid configuration
#[test]
fn test_init_governance_max_config() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let gov_config = GovernanceConfig {
        voting_period: u64::MAX,
        execution_delay: u64::MAX,
        quorum_percentage: 10000,
        approval_threshold: 10000,
        min_proposal_stake: i128::MAX,
        voting_scheme: VotingScheme::OnePersonOneVote,
        governance_token: Address::generate(&env),
    };

    client.init_governance(&admin, &gov_config);

    let version = client.get_version();
    assert_eq!(version, 2);
}
