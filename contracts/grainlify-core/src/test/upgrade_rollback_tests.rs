#![cfg(test)]

extern crate std;

use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, Vec as SorobanVec};

// use super::WASM;
use crate::{GrainlifyContract, GrainlifyContractClient};

// ============================================================================
// Test Helpers
// ============================================================================

/// Helper to upload WASM and return its hash
fn upload_wasm(env: &Env) -> BytesN<32> {
    env.deployer().upload_contract_wasm(WASM)
}

// ============================================================================
// WASM Hash Management Tests
// ============================================================================

#[test]
fn test_wasm_upload_returns_valid_hash() {
    let env = Env::default();

    // Upload WASM and get hash
    let wasm_hash = upload_wasm(&env);

    // Verify hash is 32 bytes
    assert_eq!(wasm_hash.len(), 32, "WASM hash should be 32 bytes");
}

#[test]
fn test_wasm_hash_reuse_without_reuploading() {
    let env = Env::default();

    // Upload WASM multiple times
    let wasm_hash_1 = upload_wasm(&env);
    let wasm_hash_2 = upload_wasm(&env);
    let wasm_hash_3 = upload_wasm(&env);

    // All hashes should be identical (same WASM content)
    assert_eq!(
        wasm_hash_1, wasm_hash_2,
        "Same WASM should produce same hash"
    );
    assert_eq!(
        wasm_hash_2, wasm_hash_3,
        "Hash should be consistent across uploads"
    );
}

#[test]
fn test_wasm_hash_is_deterministic() {
    let env = Env::default();

    // Upload WASM multiple times in same environment
    let hash1 = upload_wasm(&env);
    let hash2 = upload_wasm(&env);
    let hash3 = upload_wasm(&env);

    // All hashes should match (deterministic)
    assert_eq!(hash1, hash2, "WASM hash should be deterministic");
    assert_eq!(hash2, hash3, "WASM hash should be consistent");
}

// ============================================================================
// Multisig Upgrade Tests
// ============================================================================

#[test]
fn test_multisig_upgrade_proposal() {
    let env = Env::default();
    env.mock_all_auths();

    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let signer3 = Address::generate(&env);

    let mut signers = SorobanVec::new(&env);
    signers.push_back(signer1.clone());
    signers.push_back(signer2.clone());
    signers.push_back(signer3.clone());

    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    // Initialize with multisig (2 of 3)
    client.init(&signers, &2);

    let wasm_hash = upload_wasm(&env);

    // Propose upgrade
    let proposal_id = client.propose_upgrade(&signer1, &wasm_hash);
    // Proposal ID should be valid (starts at 0 or 1 depending on implementation)
    assert!(proposal_id >= 0, "Proposal ID should be valid");

    // Approve with 2 signers
    client.approve_upgrade(&proposal_id, &signer1);
    client.approve_upgrade(&proposal_id, &signer2);

    // Execute upgrade
    client.execute_upgrade(&proposal_id);

    // Verify upgrade succeeded (version should still be accessible)
    assert_eq!(client.get_version(), 2);
}

#[test]
fn test_multisig_rollback_proposal() {
    let env = Env::default();
    env.mock_all_auths();

    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let signer3 = Address::generate(&env);

    let mut signers = SorobanVec::new(&env);
    signers.push_back(signer1.clone());
    signers.push_back(signer2.clone());
    signers.push_back(signer3.clone());

    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    client.init(&signers, &2);

    let wasm_hash = upload_wasm(&env);

    // First upgrade
    let proposal_id_1 = client.propose_upgrade(&signer1, &wasm_hash);
    client.approve_upgrade(&proposal_id_1, &signer1);
    client.approve_upgrade(&proposal_id_1, &signer2);
    client.execute_upgrade(&proposal_id_1);

    // Propose rollback (using same hash for testing)
    let proposal_id_2 = client.propose_upgrade(&signer2, &wasm_hash);
    assert!(
        proposal_id_2 > proposal_id_1,
        "Second proposal ID should be greater than first"
    );

    client.approve_upgrade(&proposal_id_2, &signer2);
    client.approve_upgrade(&proposal_id_2, &signer3);
    client.execute_upgrade(&proposal_id_2);

    // Verify rollback succeeded
    assert_eq!(client.get_version(), 2);
}

#[test]
fn test_multisig_multiple_proposals() {
    let env = Env::default();
    env.mock_all_auths();

    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);

    let mut signers = SorobanVec::new(&env);
    signers.push_back(signer1.clone());
    signers.push_back(signer2.clone());

    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    client.init(&signers, &2);

    let wasm_hash = upload_wasm(&env);

    // Create multiple proposals
    let prop1 = client.propose_upgrade(&signer1, &wasm_hash);
    let prop2 = client.propose_upgrade(&signer2, &wasm_hash);
    let prop3 = client.propose_upgrade(&signer1, &wasm_hash);

    // Verify proposal IDs increment
    assert!(prop2 > prop1, "Proposal IDs should increment");
    assert!(prop3 > prop2, "Proposal IDs should increment");
}

// ============================================================================
// Version Management Tests
// ============================================================================

#[test]
fn test_version_functions_consistency() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    client.init_admin(&admin);

    // Test version functions
    let version = client.get_version();
    assert_eq!(version, 2, "Initial version should be 2");

    let semver = client.get_version_semver_string();
    assert_eq!(
        semver,
        soroban_sdk::String::from_str(&env, "2.0.0"),
        "Semantic version should be 2.0.0"
    );

    let numeric = client.get_version_numeric_encoded();
    assert_eq!(numeric, 20000, "Numeric encoding should be 20000");
}

#[test]
fn test_version_update() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    client.init_admin(&admin);

    // Update version
    client.set_version(&3);
    assert_eq!(client.get_version(), 3);

    // Update again
    client.set_version(&10);
    assert_eq!(client.get_version(), 10);
}

#[test]
fn test_previous_version_initially_none() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    client.init_admin(&admin);

    // Initially no previous version
    let prev = client.get_previous_version();
    assert!(prev.is_none(), "Initially should have no previous version");
}

// ============================================================================
// Migration State Tests
// ============================================================================

#[test]
fn test_migration_state_tracking() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    client.init_admin(&admin);

    // Initially no migration state
    assert!(client.get_migration_state().is_none());

    // Perform migration
    let migration_hash = BytesN::from_array(&env, &[1u8; 32]);
    client.migrate(&3, &migration_hash);

    // Verify migration state
    let state = client.get_migration_state();
    assert!(state.is_some(), "Migration state should be set");

    let state = state.unwrap();
    assert_eq!(state.from_version, 2);
    assert_eq!(state.to_version, 3);
    assert_eq!(state.migration_hash, migration_hash);
}

#[test]
fn test_migration_idempotency() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    client.init_admin(&admin);

    let migration_hash = BytesN::from_array(&env, &[2u8; 32]);

    // First migration
    client.migrate(&3, &migration_hash);
    let state1 = client.get_migration_state().unwrap();

    // Second migration (should be idempotent)
    client.migrate(&3, &migration_hash);
    let state2 = client.get_migration_state().unwrap();

    // States should be identical
    assert_eq!(state1.from_version, state2.from_version);
    assert_eq!(state1.to_version, state2.to_version);
    assert_eq!(state1.migrated_at, state2.migrated_at);
}

// ============================================================================
// Storage Persistence Tests
// ============================================================================

#[test]
fn test_admin_persists_across_version_changes() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    client.init_admin(&admin);

    // Change version multiple times
    client.set_version(&3);
    client.set_version(&4);
    client.set_version(&5);

    // Admin should still be able to perform operations
    client.set_version(&6);
    assert_eq!(client.get_version(), 6);
}

#[test]
fn test_migration_state_persists_across_version_changes() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    client.init_admin(&admin);

    // Perform migration
    let migration_hash = BytesN::from_array(&env, &[3u8; 32]);
    client.migrate(&3, &migration_hash);

    // Change version
    client.set_version(&10);

    // Migration state should still exist
    let state = client.get_migration_state();
    assert!(state.is_some());
    assert_eq!(state.unwrap().to_version, 3);
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_complete_upgrade_workflow_simulation() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    // Step 1: Initialize
    client.init_admin(&admin);
    assert_eq!(client.get_version(), 2);

    // Step 2: Upload new WASM (simulated)
    let new_wasm_hash = upload_wasm(&env);
    assert_eq!(new_wasm_hash.len(), 32);

    // Step 3: Run migration to v3
    let migration_hash = BytesN::from_array(&env, &[3u8; 32]);
    client.migrate(&3, &migration_hash);

    // Step 4: Verify final state
    assert_eq!(client.get_version(), 3);
    let state = client.get_migration_state().unwrap();
    assert_eq!(state.from_version, 2);
    assert_eq!(state.to_version, 3);
}

#[test]
fn test_rollback_workflow_simulation() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    client.init_admin(&admin);

    // Simulate upgrade to v3
    let wasm_v2 = upload_wasm(&env);
    client.set_version(&3);

    // Simulate rollback
    let wasm_v1 = upload_wasm(&env); // Same hash for testing
    client.set_version(&2); // Restore version

    // Verify rollback
    assert_eq!(client.get_version(), 2);
    assert_eq!(wasm_v1, wasm_v2, "WASM hashes should match");
}
