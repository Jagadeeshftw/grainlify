#![cfg(test)]
use super::*;
use soroban_sdk::{map, testutils::Address as _, Address, Env, String, Vec};

#[test]
fn test_escrow_metadata_basic_operations() {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    // Initialize contract
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    client.init(&admin, &token);

    // Lock funds for bounty
    let depositor = Address::generate(&env);
    let bounty_id = 42u64;
    let amount = 1000_0000000i128;
    let deadline = env.ledger().timestamp() + 2592000; // 30 days

    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

    // Set metadata
    let metadata = EscrowMetadata {
        repo_id: Some(String::from_str(&env, "owner/repo")),
        issue_id: Some(String::from_str(&env, "123")),
        bounty_type: Some(String::from_str(&env, "bug")),
        tags: vec![
            &env,
            String::from_str(&env, "priority-high"),
            String::from_str(&env, "security"),
        ],
        custom_fields: map![
            &env,
            (
                String::from_str(&env, "difficulty"),
                String::from_str(&env, "medium")
            ),
            (
                String::from_str(&env, "estimated_hours"),
                String::from_str(&env, "20")
            )
        ],
    };

    client.set_escrow_metadata(&bounty_id, &metadata);

    // Retrieve metadata
    let retrieved_metadata = client.get_escrow_metadata(&bounty_id).unwrap();
    assert_eq!(retrieved_metadata, Some(metadata));

    // Retrieve combined view
    let escrow_with_meta = client.get_escrow_with_metadata(&bounty_id).unwrap();
    assert_eq!(escrow_with_meta.escrow.amount, amount);
    assert_eq!(escrow_with_meta.escrow.status, EscrowStatus::Locked);
    assert_eq!(escrow_with_meta.metadata, Some(metadata));
}

#[test]
fn test_escrow_metadata_authorization() {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    // Initialize contract
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    client.init(&admin, &token);

    // Lock funds
    let depositor = Address::generate(&env);
    let other_user = Address::generate(&env);
    let bounty_id = 42u64;
    let amount = 1000_0000000i128;
    let deadline = env.ledger().timestamp() + 2592000;

    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

    // Set metadata with wrong depositor should fail
    let metadata = EscrowMetadata {
        repo_id: Some(String::from_str(&env, "owner/repo")),
        issue_id: Some(String::from_str(&env, "123")),
        bounty_type: Some(String::from_str(&env, "bug")),
        tags: vec![&env],
        custom_fields: map![&env],
    };

    // This should panic due to authorization failure
    let result = client.try_set_escrow_metadata(&other_user, &bounty_id, &metadata);
    assert!(result.is_err());
}

#[test]
fn test_escrow_metadata_size_limits() {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    // Initialize contract
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    client.init(&admin, &token);

    // Lock funds
    let depositor = Address::generate(&env);
    let bounty_id = 42u64;
    let amount = 1000_0000000i128;
    let deadline = env.ledger().timestamp() + 2592000;

    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

    // Test tags limit (should be <= 20)
    let mut tags = Vec::new(&env);
    for i in 0..25 {
        tags.push_back(String::from_str(&env, &format!("tag{}", i)));
    }

    let oversized_metadata = EscrowMetadata {
        repo_id: Some(String::from_str(&env, "owner/repo")),
        issue_id: Some(String::from_str(&env, "123")),
        bounty_type: Some(String::from_str(&env, "bug")),
        tags,
        custom_fields: map![&env],
    };

    // This should fail due to size limits
    let result = client.try_set_escrow_metadata(&depositor, &bounty_id, &oversized_metadata);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), Error::MetadataTooLarge);
}

#[test]
fn test_escrow_metadata_optional_fields() {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    // Initialize contract
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    client.init(&admin, &token);

    // Lock funds
    let depositor = Address::generate(&env);
    let bounty_id = 42u64;
    let amount = 1000_0000000i128;
    let deadline = env.ledger().timestamp() + 2592000;

    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

    // Metadata with only some fields set
    let partial_metadata = EscrowMetadata {
        repo_id: Some(String::from_str(&env, "owner/repo")),
        issue_id: None,
        bounty_type: None,
        tags: vec![&env],
        custom_fields: map![&env],
    };

    client.set_escrow_metadata(&bounty_id, &partial_metadata);

    let retrieved = client.get_escrow_metadata(&bounty_id).unwrap();
    assert_eq!(retrieved, Some(partial_metadata));
}

#[test]
fn test_escrow_nonexistent_bounty() {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    // Initialize contract
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    client.init(&admin, &token);

    // Try to get metadata for non-existent bounty
    let result = client.try_get_escrow_metadata(&999u64);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), Error::BountyNotFound);

    // Try to set metadata for non-existent bounty
    let metadata = EscrowMetadata {
        repo_id: Some(String::from_str(&env, "owner/repo")),
        issue_id: Some(String::from_str(&env, "123")),
        bounty_type: Some(String::from_str(&env, "bug")),
        tags: vec![&env],
        custom_fields: map![&env],
    };

    let result = client.try_set_escrow_metadata(&Address::generate(&env), &999u64, &metadata);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), Error::BountyNotFound);
}
