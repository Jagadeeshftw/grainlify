#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};
use crate::{ProgramEscrowClient, ProgramMetadata, ProgramMetadataField, DELEGATE_PERMISSION_UPDATE_META};

fn setup_env<'a>() -> (Env, ProgramEscrowClient<'a>, Address, Address, String) {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, crate::ProgramEscrow);
    let client = ProgramEscrowClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    
    client.initialize(&admin);
    
    let program_id = String::from_str(&env, "TEST_PROG");
    let payout_key = Address::generate(&env);
    
    // Admin initializes the program
    client.init_program(&program_id, &payout_key, &token);
    
    // Need to publish the program so it moves from Draft to Active
    client.publish_program(&program_id);
    
    (env, client, admin, token, program_id)
}

#[test]
fn test_delegate_rate_limit_engages() {
    let (env, client, admin, _, program_id) = setup_env();
    
    // Set up a delegate with metadata update permission
    let delegate = Address::generate(&env);
    client.set_program_delegate(
        &program_id,
        &delegate,
        &DELEGATE_PERMISSION_UPDATE_META,
    );

    let metadata = ProgramMetadata {
        program_name: Some(String::from_str(&env, "Test Name")),
        program_type: None,
        ecosystem: None,
        tags: Vec::new(&env),
        start_date: None,
        end_date: None,
        custom_fields: Vec::new(&env),
    };

    // First update should succeed
    client.update_program_metadata_by(&program_id, &delegate, &metadata);

    // Second update immediately after should fail due to rate limit
    let result = client.try_update_program_metadata_by(&program_id, &delegate, &metadata);
    assert!(result.is_err());
    
    // Advance time by 61 seconds (DELEGATE_METADATA_UPDATE_INTERVAL is 60)
    env.ledger().set_timestamp(env.ledger().timestamp() + 61);
    
    // Update should now succeed
    client.update_program_metadata_by(&program_id, &delegate, &metadata);
}

#[test]
fn test_admin_bypasses_rate_limit() {
    let (env, client, admin, _, program_id) = setup_env();
    
    let metadata = ProgramMetadata {
        program_name: Some(String::from_str(&env, "Test Name Admin")),
        program_type: None,
        ecosystem: None,
        tags: Vec::new(&env),
        start_date: None,
        end_date: None,
        custom_fields: Vec::new(&env),
    };

    // Admin first update
    client.update_program_metadata_by(&program_id, &admin, &metadata);

    // Admin second update immediately after (no time advance) should succeed
    client.update_program_metadata_by(&program_id, &admin, &metadata);
}

#[test]
#[should_panic(expected = "Metadata custom fields exceed limit")]
fn test_custom_fields_size_cap() {
    let (env, client, admin, _, program_id) = setup_env();
    
    let delegate = Address::generate(&env);
    client.set_program_delegate(
        &program_id,
        &delegate,
        &DELEGATE_PERMISSION_UPDATE_META,
    );

    let mut custom_fields = Vec::new(&env);
    // Add 11 fields (MAX_PROGRAM_METADATA_CUSTOM_FIELDS is 10)
    for i in 0..11 {
        custom_fields.push_back(ProgramMetadataField {
            key: String::from_str(&env, "key"),
            value: String::from_str(&env, "val"),
        });
    }

    let metadata = ProgramMetadata {
        program_name: Some(String::from_str(&env, "Test Name")),
        program_type: None,
        ecosystem: None,
        tags: Vec::new(&env),
        start_date: None,
        end_date: None,
        custom_fields,
    };

    // Should panic because > 10 fields
    client.update_program_metadata_by(&program_id, &delegate, &metadata);
}
