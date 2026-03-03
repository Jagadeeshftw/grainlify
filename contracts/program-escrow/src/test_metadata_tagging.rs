#![cfg(test)]

use crate::*;
use soroban_sdk::{
    testutils::{Address as _, Events},
    token, Address, Env, String, Vec as SdkVec,
};

fn create_token(
    env: &Env,
    admin: &Address,
) -> (token::Client<'static>, token::StellarAssetClient<'static>) {
    let addr = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    (
        token::Client::new(env, &addr),
        token::StellarAssetClient::new(env, &addr),
    )
}

fn create_program_escrow(env: &Env) -> ProgramEscrowContractClient<'static> {
    let id = env.register_contract(None, ProgramEscrowContract);
    ProgramEscrowContractClient::new(env, &id)
}

struct Setup {
    env: Env,
    admin: Address,
    organizer: Address,
    backend: Address,
    escrow: ProgramEscrowContractClient<'static>,
    token: token::Client<'static>,
}

impl Setup {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let organizer = Address::generate(&env);
        let backend = Address::generate(&env);
        let (token, token_admin) = create_token(&env, &admin);
        let escrow = create_program_escrow(&env);
        token_admin.mint(&organizer, &100_000_000);
        Setup {
            env,
            admin,
            organizer,
            backend,
            escrow,
            token,
        }
    }
}

// ============================================================================
// Test 1: Program Metadata Storage and Retrieval
// ============================================================================

#[test]
#[ignore = "Program metadata functionality to be implemented - Issue #63"]
fn test_program_metadata_set_on_creation() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "Hackathon2024");

    // Create program metadata
    let mut tags = SdkVec::new(&s.env);
    tags.push_back(String::from_str(&s.env, "hackathon"));
    tags.push_back(String::from_str(&s.env, "defi"));
    tags.push_back(String::from_str(&s.env, "stellar"));

    let metadata = ProgramMetadata {
        program_name: Some(String::from_str(&s.env, "Stellar DeFi Hackathon 2024")),
        program_type: Some(String::from_str(&s.env, "hackathon")),
        ecosystem: Some(String::from_str(&s.env, "stellar")),
        tags: tags.clone(),
        start_date: Some(s.env.ledger().timestamp()),
        end_date: Some(s.env.ledger().timestamp() + 2_592_000), // 30 days
        custom_fields: SdkVec::new(&s.env),
    };

    // Initialize program with metadata
    s.escrow.init_program_with_metadata(
        &program_id,
        &s.backend,
        &s.token.address,
        &s.organizer,
        &None,
        &metadata,
    );

    // Retrieve and verify metadata
    let retrieved = s.escrow.get_program_metadata(&program_id);
    assert_eq!(
        retrieved.program_name,
        Some(String::from_str(&s.env, "Stellar DeFi Hackathon 2024"))
    );
    assert_eq!(
        retrieved.program_type,
        Some(String::from_str(&s.env, "hackathon"))
    );
    assert_eq!(retrieved.tags.len(), 3);
}

#[test]
#[ignore = "Program metadata functionality to be implemented - Issue #63"]
fn test_program_metadata_update() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "Program2024");

    // Initial metadata
    let initial_metadata = ProgramMetadata {
        program_name: Some(String::from_str(&s.env, "Initial Program")),
        program_type: Some(String::from_str(&s.env, "grant")),
        ecosystem: Some(String::from_str(&s.env, "stellar")),
        tags: SdkVec::new(&s.env),
        start_date: None,
        end_date: None,
        custom_fields: SdkVec::new(&s.env),
    };

    s.escrow.init_program_with_metadata(
        &program_id,
        &s.backend,
        &s.token.address,
        &s.organizer,
        &None,
        &initial_metadata,
    );

    // Update metadata
    let mut updated_tags = SdkVec::new(&s.env);
    updated_tags.push_back(String::from_str(&s.env, "updated"));

    let updated_metadata = ProgramMetadata {
        program_name: Some(String::from_str(&s.env, "Updated Program Name")),
        program_type: Some(String::from_str(&s.env, "bounty_program")),
        ecosystem: Some(String::from_str(&s.env, "stellar")),
        tags: updated_tags,
        start_date: Some(s.env.ledger().timestamp()),
        end_date: Some(s.env.ledger().timestamp() + 1_000_000),
        custom_fields: SdkVec::new(&s.env),
    };

    s.escrow
        .update_program_metadata(&program_id, &updated_metadata);

    // Verify update
    let retrieved = s.escrow.get_program_metadata(&program_id);
    assert_eq!(
        retrieved.program_name,
        Some(String::from_str(&s.env, "Updated Program Name"))
    );
    assert_eq!(
        retrieved.program_type,
        Some(String::from_str(&s.env, "bounty_program"))
    );
}

// ============================================================================
// Test 2: Query Programs by Metadata
// ============================================================================

#[test]
#[ignore = "Program metadata query functionality to be implemented - Issue #63"]
fn test_query_programs_by_type() {
    let s = Setup::new();

    // Create programs with different types
    let program_types = vec!["hackathon", "grant", "hackathon", "bounty_program"];

    for (i, prog_type) in program_types.iter().enumerate() {
        let program_id = String::from_str(&s.env, &format!("Program{}", i + 1));

        let metadata = ProgramMetadata {
            program_name: Some(String::from_str(&s.env, &format!("Program {}", i + 1))),
            program_type: Some(String::from_str(&s.env, prog_type)),
            ecosystem: Some(String::from_str(&s.env, "stellar")),
            tags: SdkVec::new(&s.env),
            start_date: None,
            end_date: None,
            custom_fields: SdkVec::new(&s.env),
        };

        s.escrow.init_program_with_metadata(
            &program_id,
            &s.backend,
            &s.token.address,
            &s.organizer,
            &None,
            &metadata,
        );
    }

    // Query hackathon programs
    let hackathons = s.escrow.query_programs_by_type(
        &String::from_str(&s.env, "hackathon"),
        &0,
        &20,
    );
    assert_eq!(hackathons.len(), 2);

    // Query grant programs
    let grants = s
        .escrow
        .query_programs_by_type(&String::from_str(&s.env, "grant"), &0, &20);
    assert_eq!(grants.len(), 1);
}

#[test]
#[ignore = "Program metadata query functionality to be implemented - Issue #63"]
fn test_query_programs_by_ecosystem() {
    let s = Setup::new();

    // Create programs in different ecosystems
    let ecosystems = vec!["stellar", "ethereum", "stellar", "polkadot"];

    for (i, ecosystem) in ecosystems.iter().enumerate() {
        let program_id = String::from_str(&s.env, &format!("Program{}", i + 1));

        let metadata = ProgramMetadata {
            program_name: Some(String::from_str(&s.env, &format!("Program {}", i + 1))),
            program_type: Some(String::from_str(&s.env, "hackathon")),
            ecosystem: Some(String::from_str(&s.env, ecosystem)),
            tags: SdkVec::new(&s.env),
            start_date: None,
            end_date: None,
            custom_fields: SdkVec::new(&s.env),
        };

        s.escrow.init_program_with_metadata(
            &program_id,
            &s.backend,
            &s.token.address,
            &s.organizer,
            &None,
            &metadata,
        );
    }

    // Query stellar programs
    let stellar_programs = s.escrow.query_programs_by_ecosystem(
        &String::from_str(&s.env, "stellar"),
        &0,
        &20,
    );
    assert_eq!(stellar_programs.len(), 2);
}

#[test]
#[ignore = "Program metadata query functionality to be implemented - Issue #63"]
fn test_query_programs_by_tags() {
    let s = Setup::new();

    // Create programs with different tags
    for i in 1u32..=6 {
        let program_id = String::from_str(&s.env, &format!("Program{}", i));

        let mut tags = SdkVec::new(&s.env);
        if i % 2 == 0 {
            tags.push_back(String::from_str(&s.env, "defi"));
        }
        if i % 3 == 0 {
            tags.push_back(String::from_str(&s.env, "nft"));
        }

        let metadata = ProgramMetadata {
            program_name: Some(String::from_str(&s.env, &format!("Program {}", i))),
            program_type: Some(String::from_str(&s.env, "hackathon")),
            ecosystem: Some(String::from_str(&s.env, "stellar")),
            tags,
            start_date: None,
            end_date: None,
            custom_fields: SdkVec::new(&s.env),
        };

        s.escrow.init_program_with_metadata(
            &program_id,
            &s.backend,
            &s.token.address,
            &s.organizer,
            &None,
            &metadata,
        );
    }

    // Query by "defi" tag
    let defi_programs =
        s.escrow
            .query_programs_by_tag(&String::from_str(&s.env, "defi"), &0, &20);
    assert_eq!(defi_programs.len(), 3); // 2, 4, 6

    // Query by "nft" tag
    let nft_programs =
        s.escrow
            .query_programs_by_tag(&String::from_str(&s.env, "nft"), &0, &20);
    assert_eq!(nft_programs.len(), 2); // 3, 6
}

// ============================================================================
// Test 3: Metadata Persistence Through Program Lifecycle
// ============================================================================

#[test]
#[ignore = "Program metadata functionality to be implemented - Issue #63"]
fn test_metadata_persists_through_lifecycle() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "LifecycleTest");
    let prize_pool = 10_000_0000000i128;

    // Create program with metadata
    let metadata = ProgramMetadata {
        program_name: Some(String::from_str(&s.env, "Lifecycle Test Program")),
        program_type: Some(String::from_str(&s.env, "hackathon")),
        ecosystem: Some(String::from_str(&s.env, "stellar")),
        tags: SdkVec::new(&s.env),
        start_date: Some(s.env.ledger().timestamp()),
        end_date: Some(s.env.ledger().timestamp() + 1_000_000),
        custom_fields: SdkVec::new(&s.env),
    };

    s.escrow.init_program_with_metadata(
        &program_id,
        &s.backend,
        &s.token.address,
        &s.organizer,
        &Some(prize_pool),
        &metadata,
    );

    // Verify metadata after initialization
    let after_init = s.escrow.get_program_metadata(&program_id);
    assert_eq!(
        after_init.program_name,
        Some(String::from_str(&s.env, "Lifecycle Test Program"))
    );

    // Perform payout
    let winner = Address::generate(&s.env);
    let mut winners = SdkVec::new(&s.env);
    winners.push_back(winner.clone());
    let mut amounts = SdkVec::new(&s.env);
    amounts.push_back(5_000_0000000i128);

    s.escrow.batch_payout(&program_id, &winners, &amounts);

    // Verify metadata persists after payout
    let after_payout = s.escrow.get_program_metadata(&program_id);
    assert_eq!(
        after_payout.program_name,
        Some(String::from_str(&s.env, "Lifecycle Test Program"))
    );
    assert_eq!(
        after_payout.program_type,
        Some(String::from_str(&s.env, "hackathon"))
    );
}

// ============================================================================
// Test 4: Custom Fields and Extensibility
// ============================================================================

#[test]
#[ignore = "Program metadata functionality to be implemented - Issue #63"]
fn test_program_custom_fields() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "CustomFieldsTest");

    // Create metadata with custom fields
    let mut custom_fields = SdkVec::new(&s.env);
    custom_fields.push_back((
        String::from_str(&s.env, "total_participants"),
        String::from_str(&s.env, "150"),
    ));
    custom_fields.push_back((
        String::from_str(&s.env, "prize_pool_usd"),
        String::from_str(&s.env, "50000"),
    ));
    custom_fields.push_back((
        String::from_str(&s.env, "sponsor"),
        String::from_str(&s.env, "Stellar Development Foundation"),
    ));

    let metadata = ProgramMetadata {
        program_name: Some(String::from_str(&s.env, "Custom Fields Program")),
        program_type: Some(String::from_str(&s.env, "hackathon")),
        ecosystem: Some(String::from_str(&s.env, "stellar")),
        tags: SdkVec::new(&s.env),
        start_date: None,
        end_date: None,
        custom_fields,
    };

    s.escrow.init_program_with_metadata(
        &program_id,
        &s.backend,
        &s.token.address,
        &s.organizer,
        &None,
        &metadata,
    );

    // Retrieve and verify custom fields
    let retrieved = s.escrow.get_program_metadata(&program_id);
    assert_eq!(retrieved.custom_fields.len(), 3);

    let field_0 = retrieved.custom_fields.get(0).unwrap();
    assert_eq!(
        field_0.0,
        String::from_str(&s.env, "total_participants")
    );
    assert_eq!(field_0.1, String::from_str(&s.env, "150"));
}

// ============================================================================
// Test 5: Serialization Format for Indexers
// ============================================================================

#[test]
#[ignore = "Program metadata functionality to be implemented - Issue #63"]
fn test_program_metadata_serialization() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "SerializationTest");

    // Create comprehensive metadata
    let mut tags = SdkVec::new(&s.env);
    tags.push_back(String::from_str(&s.env, "hackathon"));
    tags.push_back(String::from_str(&s.env, "defi"));

    let mut custom_fields = SdkVec::new(&s.env);
    custom_fields.push_back((
        String::from_str(&s.env, "region"),
        String::from_str(&s.env, "global"),
    ));

    let metadata = ProgramMetadata {
        program_name: Some(String::from_str(&s.env, "Serialization Test")),
        program_type: Some(String::from_str(&s.env, "hackathon")),
        ecosystem: Some(String::from_str(&s.env, "stellar")),
        tags,
        start_date: Some(s.env.ledger().timestamp()),
        end_date: Some(s.env.ledger().timestamp() + 1_000_000),
        custom_fields,
    };

    s.escrow.init_program_with_metadata(
        &program_id,
        &s.backend,
        &s.token.address,
        &s.organizer,
        &None,
        &metadata,
    );

    // Retrieve and verify structure
    let retrieved = s.escrow.get_program_metadata(&program_id);

    // Verify all fields are accessible for indexers
    assert!(retrieved.program_name.is_some());
    assert!(retrieved.program_type.is_some());
    assert!(retrieved.ecosystem.is_some());
    assert!(retrieved.start_date.is_some());
    assert!(retrieved.end_date.is_some());
    assert_eq!(retrieved.tags.len(), 2);
    assert_eq!(retrieved.custom_fields.len(), 1);
}

// ============================================================================
// Test 6: Edge Cases
// ============================================================================

#[test]
#[ignore = "Program metadata functionality to be implemented - Issue #63"]
fn test_empty_program_metadata() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "EmptyMetadata");

    // Create with minimal metadata
    let metadata = ProgramMetadata {
        program_name: None,
        program_type: None,
        ecosystem: None,
        tags: SdkVec::new(&s.env),
        start_date: None,
        end_date: None,
        custom_fields: SdkVec::new(&s.env),
    };

    s.escrow.init_program_with_metadata(
        &program_id,
        &s.backend,
        &s.token.address,
        &s.organizer,
        &None,
        &metadata,
    );

    // Verify empty metadata is handled correctly
    let retrieved = s.escrow.get_program_metadata(&program_id);
    assert!(retrieved.program_name.is_none());
    assert!(retrieved.program_type.is_none());
    assert!(retrieved.ecosystem.is_none());
    assert_eq!(retrieved.tags.len(), 0);
    assert_eq!(retrieved.custom_fields.len(), 0);
}
