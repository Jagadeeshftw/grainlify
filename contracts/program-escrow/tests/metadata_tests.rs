#![cfg(test)]
use super::*;
use soroban_sdk::{map, testutils::Address as _, Address, Env, String, Vec};

#[test]
fn test_program_metadata_basic_operations() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    // Initialize program
    let program_id = String::from_str(&env, "Hackathon2024");
    let authorized_key = Address::generate(&env);
    let token = Address::generate(&env);

    client
        .init_program(&program_id, &authorized_key, &token)
        .unwrap();

    // Set metadata
    let metadata = ProgramMetadata {
        event_name: Some(String::from_str(&env, "Stellar Hackathon 2024")),
        event_type: Some(String::from_str(&env, "hackathon")),
        start_date: Some(String::from_str(&env, "2024-06-01")),
        end_date: Some(String::from_str(&env, "2024-06-30")),
        website: Some(String::from_str(&env, "https://hackathon.stellar.org")),
        tags: vec![
            &env,
            String::from_str(&env, "blockchain"),
            String::from_str(&env, "defi"),
            String::from_str(&env, "web3"),
        ],
        custom_fields: map![
            &env,
            (
                String::from_str(&env, "track_count"),
                String::from_str(&env, "5")
            ),
            (
                String::from_str(&env, "expected_participants"),
                String::from_str(&env, "500")
            ),
        ],
    };

    client.set_program_metadata(&metadata).unwrap();

    // Retrieve metadata
    let retrieved_metadata = client.get_program_metadata().unwrap();
    assert_eq!(retrieved_metadata, Some(metadata));

    // Retrieve combined view
    let program_with_meta = client.get_program_with_metadata().unwrap();
    assert_eq!(program_with_meta.program.program_id, program_id);
    assert_eq!(program_with_meta.program.total_funds, 0);
    assert_eq!(program_with_meta.metadata, Some(metadata));
}

#[test]
fn test_program_metadata_authorization() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    // Initialize program
    let program_id = String::from_str(&env, "Hackathon2024");
    let authorized_key = Address::generate(&env);
    let unauthorized_key = Address::generate(&env);
    let token = Address::generate(&env);

    client
        .init_program(&program_id, &authorized_key, &token)
        .unwrap();

    // Set metadata with unauthorized key should fail
    let metadata = ProgramMetadata {
        event_name: Some(String::from_str(&env, "Test Event")),
        event_type: Some(String::from_str(&env, "hackathon")),
        start_date: Some(String::from_str(&env, "2024-01-01")),
        end_date: Some(String::from_str(&env, "2024-01-31")),
        website: None,
        tags: vec![&env],
        custom_fields: map![&env],
    };

    // Switch to unauthorized caller
    env.mock_all_auths_allowing_non_root_auth();

    // This should return an error due to authorization failure
    let result = client.try_set_program_metadata(&metadata);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), Error::Unauthorized);
}

#[test]
fn test_program_metadata_size_limits() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    // Initialize program
    let program_id = String::from_str(&env, "Hackathon2024");
    let authorized_key = Address::generate(&env);
    let token = Address::generate(&env);

    client
        .init_program(&program_id, &authorized_key, &token)
        .unwrap();

    // Test tags limit (should be <= 30)
    let mut tags = Vec::new(&env);
    for i in 0..35 {
        tags.push_back(String::from_str(&env, &format!("tag{}", i)));
    }

    let oversized_metadata = ProgramMetadata {
        event_name: Some(String::from_str(&env, "Test Event")),
        event_type: Some(String::from_str(&env, "hackathon")),
        start_date: Some(String::from_str(&env, "2024-01-01")),
        end_date: Some(String::from_str(&env, "2024-01-31")),
        website: Some(String::from_str(&env, "https://example.com")),
        tags,
        custom_fields: map![&env],
    };

    // This should return an error due to size limits
    let result = client.try_set_program_metadata(&oversized_metadata);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), Error::MetadataTooLarge);
}

#[test]
fn test_program_metadata_optional_fields() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    // Initialize program
    let program_id = String::from_str(&env, "Hackathon2024");
    let authorized_key = Address::generate(&env);
    let token = Address::generate(&env);

    client
        .init_program(&program_id, &authorized_key, &token)
        .unwrap();

    // Metadata with only some fields set
    let partial_metadata = ProgramMetadata {
        event_name: Some(String::from_str(&env, "Simple Event")),
        event_type: None,
        start_date: None,
        end_date: None,
        website: None,
        tags: vec![&env, String::from_str(&env, "simple")],
        custom_fields: map![&env],
    };

    client.set_program_metadata(&partial_metadata).unwrap();

    let retrieved = client.get_program_metadata().unwrap();
    assert_eq!(retrieved, Some(partial_metadata));
}

#[test]
fn test_program_nonexistent_program() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    // Try to get metadata before initialization
    let result = client.try_get_program_metadata();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), Error::NotInitialized);

    // Try to get combined view before initialization
    let result = client.try_get_program_with_metadata();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), Error::NotInitialized);
}
