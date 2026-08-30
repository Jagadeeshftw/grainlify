use crate::*;
use soroban_sdk::{testutils::Address as _, token, Address, Env, String, Vec as SdkVec};

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

#[test]
fn test_program_metadata_set_on_creation() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "Hackathon2024");

    let mut tags = SdkVec::new(&s.env);
    tags.push_back(String::from_str(&s.env, "hackathon"));

    let metadata = ProgramMetadata {
        program_name: Some(String::from_str(&s.env, "Hackathon")),
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
        &Some(metadata),
    );

    let retrieved = s.escrow.get_program_metadata(&program_id);
    assert_eq!(
        retrieved.program_name,
        Some(String::from_str(&s.env, "Hackathon"))
    );
}

#[test]
#[ignore = "Program metadata query functionality to be implemented - Issue #63"]
fn test_query_programs_by_type() {
    let s = Setup::new();

    // Create programs with different types
    let program_types = ["hackathon", "grant", "hackathon", "bounty_program"];

    for (i, prog_type) in program_types.iter().enumerate() {
        let program_id = String::from_str(&s.env, &std::format!("Program{}", i + 1));

        let metadata = ProgramMetadata {
            program_name: Some(String::from_str(&s.env, &std::format!("Program {}", i + 1))),
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
            &Some(metadata.clone()),
        );
    }

    // Query hackathon programs
    let hackathons =
        s.escrow
            .query_programs_by_type(&String::from_str(&s.env, "hackathon"), &0, &20);
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
    let ecosystems = ["stellar", "ethereum", "stellar", "polkadot"];

    for (i, ecosystem) in ecosystems.iter().enumerate() {
        let program_id = String::from_str(&s.env, &std::format!("Program{}", i + 1));

        let metadata = ProgramMetadata {
            program_name: Some(String::from_str(&s.env, &std::format!("Program {}", i + 1))),
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
            &Some(metadata.clone()),
        );
    }

    // Query stellar programs
    let stellar_programs =
        s.escrow
            .query_programs_by_ecosystem(&String::from_str(&s.env, "stellar"), &0, &20);
    assert_eq!(stellar_programs.len(), 2);
}

#[test]
#[ignore = "Program metadata query functionality to be implemented - Issue #63"]
fn test_query_programs_by_tags() {
    let s = Setup::new();

    // Create programs with different tags
    for i in 1u32..=6 {
        let program_id = String::from_str(&s.env, &std::format!("Program{}", i));

        let mut tags = SdkVec::new(&s.env);
        if i % 2 == 0 {
            tags.push_back(String::from_str(&s.env, "defi"));
        }
        if i % 3 == 0 {
            tags.push_back(String::from_str(&s.env, "nft"));
        }

        let metadata = ProgramMetadata {
            program_name: Some(String::from_str(&s.env, &std::format!("Program {}", i))),
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
            &Some(metadata.clone()),
        );
    }

    // Query by "defi" tag
    let defi_programs = s
        .escrow
        .query_programs_by_tag(&String::from_str(&s.env, "defi"), &0, &20);
    assert_eq!(defi_programs.len(), 3); // 2, 4, 6

    // Query by "nft" tag
    let nft_programs = s
        .escrow
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
        &Some(metadata.clone()),
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

    s.escrow.batch_payout(&winners, &amounts);

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
    custom_fields.push_back(ProgramMetadataField {
        key: String::from_str(&s.env, "total_participants"),
        value: String::from_str(&s.env, "150"),
    });
    custom_fields.push_back(ProgramMetadataField {
        key: String::from_str(&s.env, "prize_pool_usd"),
        value: String::from_str(&s.env, "50000"),
    });
    custom_fields.push_back(ProgramMetadataField {
        key: String::from_str(&s.env, "sponsor"),
        value: String::from_str(&s.env, "Stellar Development Foundation"),
    });

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
        &Some(metadata.clone()),
    );

    // Retrieve and verify custom fields
    let retrieved = s.escrow.get_program_metadata(&program_id);
    assert_eq!(retrieved.custom_fields.len(), 3);

    let field_0 = retrieved.custom_fields.get(0).unwrap();
    assert_eq!(field_0.key, String::from_str(&s.env, "total_participants"));
    assert_eq!(field_0.value, String::from_str(&s.env, "150"));
}

// ============================================================================
// Test 5: Serialization Format for Indexers
// ============================================================================

#[test]
#[ignore = "Program metadata functionality to be implemented - Issue #63"]
fn test_program_metadata_serialization() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "UpdateTest");

    s.escrow.init_program_with_metadata(
        &program_id,
        &s.backend,
        &s.token.address,
        &s.organizer,
        &None,
        &None,
    );

    let metadata = ProgramMetadata {
        program_name: Some(String::from_str(&s.env, "Updated")),
        program_type: None,
        ecosystem: None,
        tags: SdkVec::new(&s.env),
        start_date: None,
        end_date: None,
        custom_fields: SdkVec::new(&s.env),
    };

    s.escrow.update_program_metadata(&program_id, &s.backend, &metadata);
    let retrieved = s.escrow.get_program_metadata(&program_id);
    assert_eq!(
        retrieved.program_name,
        Some(String::from_str(&s.env, "Updated"))
    );
}

// ============================================================================
// Compression Tests: MetadataFieldKey encoding / decoding
// ============================================================================

/// Helper: create a ProgramMetadata with the given custom fields.
fn make_metadata_with_fields(env: &Env, fields: &[(&str, &str)]) -> ProgramMetadata {
    let mut custom_fields: soroban_sdk::Vec<ProgramMetadataField> = soroban_sdk::Vec::new(env);
    for (key, value) in fields {
        custom_fields.push_back(ProgramMetadataField {
            key: String::from_str(env, key),
            value: String::from_str(env, value),
        });
    }
    ProgramMetadata {
        program_name: Some(String::from_str(env, "Compression Test")),
        program_type: Some(String::from_str(env, "hackathon")),
        ecosystem: Some(String::from_str(env, "stellar")),
        tags: SdkVec::new(env),
        start_date: None,
        end_date: None,
        custom_fields,
    }
}

#[test]
fn test_metadata_field_key_known_variants() {
    let env = Env::default();

    let cases: &[(&str, MetadataFieldKey)] = &[
        ("total_participants", MetadataFieldKey::TotalParticipants),
        ("prize_pool_usd", MetadataFieldKey::PrizePoolUsd),
        ("sponsor", MetadataFieldKey::Sponsor),
        ("repository", MetadataFieldKey::Repository),
        ("website", MetadataFieldKey::Website),
        ("contact_email", MetadataFieldKey::ContactEmail),
        ("difficulty", MetadataFieldKey::Difficulty),
        ("category", MetadataFieldKey::Category),
        ("status", MetadataFieldKey::Status),
        ("version", MetadataFieldKey::Version),
    ];

    for (raw, expected) in cases {
        let key = String::from_str(&env, raw);
        let parsed = MetadataFieldKey::from_string(&env, &key);
        assert_eq!(parsed, *expected, "from_string({raw}) should match");
        let back = parsed.to_legacy_string(&env);
        assert_eq!(back, key, "to_legacy_string round-trip for {raw}");
    }
}

#[test]
fn test_metadata_field_key_custom_fallback() {
    let env = Env::default();
    let raw = "my_custom_metric";
    let key = String::from_str(&env, raw);
    let parsed = MetadataFieldKey::from_string(&env, &key);
    assert_eq!(parsed, MetadataFieldKey::Custom(key.clone()));
    let back = parsed.to_legacy_string(&env);
    assert_eq!(back, key);
}

#[test]
fn test_compress_known_keys_through_storage() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "CompressKnown");

    let metadata = make_metadata_with_fields(
        &s.env,
        &[
            ("total_participants", "200"),
            ("sponsor", "Stellar Foundation"),
            ("prize_pool_usd", "100000"),
        ],
    );

    s.escrow.init_program_with_metadata(
        &program_id,
        &s.backend,
        &s.token.address,
        &s.organizer,
        &None,
        &Some(metadata),
    );

    let retrieved = s.escrow.get_program_metadata(&program_id);
    assert!(retrieved.is_some());
    let meta = retrieved.unwrap();
    assert_eq!(meta.custom_fields.len(), 3);

    let f0 = meta.custom_fields.get(0).unwrap();
    assert_eq!(f0.key, String::from_str(&s.env, "total_participants"));
    assert_eq!(f0.value, String::from_str(&s.env, "200"));
}

#[test]
fn test_compress_mixed_keys_through_storage() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "CompressMixed");

    let metadata = make_metadata_with_fields(
        &s.env,
        &[
            ("sponsor", "SDF"),
            ("contact_email", "admin@example.com"),
            ("custom_arbitrary_key", "some_value"),
            ("version", "2.0.0"),
        ],
    );

    s.escrow.init_program_with_metadata(
        &program_id,
        &s.backend,
        &s.token.address,
        &s.organizer,
        &None,
        &Some(metadata),
    );

    let retrieved = s.escrow.get_program_metadata(&program_id);
    assert!(retrieved.is_some());
    let meta = retrieved.unwrap();
    assert_eq!(meta.custom_fields.len(), 4);

    for i in 0..4 {
        let field = meta.custom_fields.get(i).unwrap();
        let expected_key = match i {
            0 => "sponsor",
            1 => "contact_email",
            2 => "custom_arbitrary_key",
            3 => "version",
            _ => unreachable!(),
        };
        assert_eq!(field.key, String::from_str(&s.env, expected_key));
    }
}

#[test]
fn test_compress_empty_custom_fields_through_storage() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "CompressEmpty");

    let metadata = ProgramMetadata {
        program_name: Some(String::from_str(&s.env, "No Custom Fields")),
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
        &Some(metadata.clone()),
    );

    let retrieved = s.escrow.get_program_metadata(&program_id);
    assert!(retrieved.is_some());
    let meta = retrieved.unwrap();
    assert_eq!(meta.custom_fields.len(), 0);
    assert_eq!(meta.program_name, metadata.program_name);
}

#[test]
fn test_compress_round_trip_through_update() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "CompressUpdate");

    let metadata = make_metadata_with_fields(
        &s.env,
        &[("status", "active"), ("website", "https://example.com")],
    );

    s.escrow.init_program_with_metadata(
        &program_id,
        &s.backend,
        &s.token.address,
        &s.organizer,
        &None,
        &Some(metadata),
    );

    let updated = make_metadata_with_fields(
        &s.env,
        &[
            ("status", "completed"),
            ("difficulty", "advanced"),
            ("repository", "github.com/example/project"),
        ],
    );

    s.escrow
        .update_program_metadata_by(&program_id, &s.backend, &updated);

    let retrieved = s.escrow.get_program_metadata(&program_id);
    assert!(retrieved.is_some());
    let meta = retrieved.unwrap();
    assert_eq!(meta.custom_fields.len(), 3);

    let f0 = meta.custom_fields.get(0).unwrap();
    assert_eq!(f0.key, String::from_str(&s.env, "status"));
    assert_eq!(f0.value, String::from_str(&s.env, "completed"));

    let f1 = meta.custom_fields.get(1).unwrap();
    assert_eq!(f1.key, String::from_str(&s.env, "difficulty"));
    assert_eq!(f1.value, String::from_str(&s.env, "advanced"));
}

#[test]
fn test_legacy_metadata_still_readable() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "LegacyCompat");

    let mut custom_fields: soroban_sdk::Vec<ProgramMetadataField> = soroban_sdk::Vec::new(&s.env);
    custom_fields.push_back(ProgramMetadataField {
        key: String::from_str(&s.env, "prize_pool_usd"),
        value: String::from_str(&s.env, "25000"),
    });

    let metadata = ProgramMetadata {
        program_name: Some(String::from_str(&s.env, "Legacy")),
        program_type: Some(String::from_str(&s.env, "bounty")),
        ecosystem: Some(String::from_str(&s.env, "stellar")),
        tags: SdkVec::new(&s.env),
        start_date: None,
        end_date: None,
        custom_fields,
    };

    // Write directly under the legacy DataKey::Metadata (simulating old contract).
    let key = DataKey::Metadata(program_id.clone());
    s.env.storage().instance().set(&key, &metadata);

    let retrieved = s.escrow.get_program_metadata(&program_id);
    assert!(retrieved.is_some());
    let meta = retrieved.unwrap();
    assert_eq!(meta.program_name, Some(String::from_str(&s.env, "Legacy")));
    assert_eq!(meta.custom_fields.len(), 1);
    let f0 = meta.custom_fields.get(0).unwrap();
    assert_eq!(f0.key, String::from_str(&s.env, "prize_pool_usd"));
    assert_eq!(f0.value, String::from_str(&s.env, "25000"));
}

#[test]
fn test_compress_long_custom_key() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "LongKey");

    let long_key = "x".repeat(200);
    let metadata = make_metadata_with_fields(&s.env, &[(&long_key, "value")]);

    s.escrow.init_program_with_metadata(
        &program_id,
        &s.backend,
        &s.token.address,
        &s.organizer,
        &None,
        &Some(metadata),
    );

    let retrieved = s.escrow.get_program_metadata(&program_id);
    assert!(retrieved.is_some());
    let meta = retrieved.unwrap();
    assert_eq!(meta.custom_fields.len(), 1);
    let f0 = meta.custom_fields.get(0).unwrap();
    assert_eq!(f0.key, String::from_str(&s.env, &long_key));
    assert_eq!(f0.value, String::from_str(&s.env, "value"));
}

#[test]
fn test_compress_special_chars_in_key() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "SpecialKey");

    let metadata = make_metadata_with_fields(
        &s.env,
        &[
            ("field_with_underscores", "val1"),
            ("field-with-hyphens", "val2"),
            ("field.with.dots", "val3"),
        ],
    );

    s.escrow.init_program_with_metadata(
        &program_id,
        &s.backend,
        &s.token.address,
        &s.organizer,
        &None,
        &Some(metadata),
    );

    let retrieved = s.escrow.get_program_metadata(&program_id);
    assert!(retrieved.is_some());
    let meta = retrieved.unwrap();
    assert_eq!(meta.custom_fields.len(), 3);
}

#[test]
fn test_compress_case_sensitivity() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "CaseSensitive");

    // Uppercase "Sponsor" should NOT compress to MetadataFieldKey::Sponsor
    let metadata = make_metadata_with_fields(&s.env, &[("Sponsor", "SDF")]);

    s.escrow.init_program_with_metadata(
        &program_id,
        &s.backend,
        &s.token.address,
        &s.organizer,
        &None,
        &Some(metadata),
    );

    let retrieved = s.escrow.get_program_metadata(&program_id);
    assert!(retrieved.is_some());
    let meta = retrieved.unwrap();
    assert_eq!(meta.custom_fields.len(), 1);
    let f0 = meta.custom_fields.get(0).unwrap();
    assert_eq!(f0.key, String::from_str(&s.env, "Sponsor"));
}

// ============================================================================
// Boundary & consistency tests for custom_fields limits (Issue #1498)
// ============================================================================

/// Create a ProgramMetadata with `n` custom fields (all identical keys/values).
fn metadata_with_n_fields(env: &Env, n: u32) -> ProgramMetadata {
    let mut custom_fields: Vec<ProgramMetadataField> = Vec::new(env);
    for _ in 0..n {
        custom_fields.push_back(ProgramMetadataField {
            key: String::from_str(env, "k"),
            value: String::from_str(env, "v"),
        });
    }
    ProgramMetadata {
        program_name: Some(String::from_str(env, "Boundary Test")),
        program_type: None,
        ecosystem: None,
        tags: Vec::new(env),
        start_date: None,
        end_date: None,
        custom_fields,
    }
}

/// Create a ProgramMetadata with a single custom field whose key and value
/// have the given byte lengths (max 257).
fn metadata_with_key_value_len(env: &Env, key_len: u32, value_len: u32) -> ProgramMetadata {
    let mut custom_fields: Vec<ProgramMetadataField> = Vec::new(env);
    let buf_k = [b'k'; 257];
    let buf_v = [b'v'; 257];
    let key_s = core::str::from_utf8(&buf_k[..key_len as usize]).unwrap();
    let val_s = core::str::from_utf8(&buf_v[..value_len as usize]).unwrap();
    custom_fields.push_back(ProgramMetadataField {
        key: String::from_str(env, key_s),
        value: String::from_str(env, val_s),
    });
    ProgramMetadata {
        program_name: Some(String::from_str(env, "Boundary Test")),
        program_type: None,
        ecosystem: None,
        tags: Vec::new(env),
        start_date: None,
        end_date: None,
        custom_fields,
    }
}

// ── Field-count boundary (soft limit MAX_PROGRAM_METADATA_CUSTOM_FIELDS = 10) ──

#[test]
fn test_init_accept_max_program_metadata_custom_fields() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "InitMaxSoftFields");
    let metadata = metadata_with_n_fields(&s.env, MAX_PROGRAM_METADATA_CUSTOM_FIELDS);
    s.escrow.init_program_with_metadata(
        &program_id, &s.backend, &s.token.address, &s.organizer, &None, &Some(metadata),
    );
    let retrieved = s.escrow.get_program_metadata(&program_id);
    assert!(retrieved.is_some());
    assert_eq!(
        retrieved.unwrap().custom_fields.len(),
        MAX_PROGRAM_METADATA_CUSTOM_FIELDS as u32,
    );
}

#[test]
#[should_panic(expected = "Metadata custom fields exceed limit")]
fn test_init_reject_over_soft_limit_custom_fields() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "InitOverSoft");
    let metadata = metadata_with_n_fields(&s.env, MAX_PROGRAM_METADATA_CUSTOM_FIELDS + 1);
    s.escrow.init_program_with_metadata(
        &program_id, &s.backend, &s.token.address, &s.organizer, &None, &Some(metadata),
    );
}

#[test]
fn test_update_accept_max_program_metadata_custom_fields() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "UpdMaxSoft");
    s.escrow.init_program_with_metadata(
        &program_id, &s.backend, &s.token.address, &s.organizer, &None, &None,
    );
    s.escrow.publish_program(&program_id, &s.backend);
    let metadata = metadata_with_n_fields(&s.env, MAX_PROGRAM_METADATA_CUSTOM_FIELDS);
    s.escrow.update_program_metadata_by(&program_id, &s.backend, &metadata);
    let retrieved = s.escrow.get_program_metadata(&program_id);
    assert!(retrieved.is_some());
    assert_eq!(
        retrieved.unwrap().custom_fields.len(),
        MAX_PROGRAM_METADATA_CUSTOM_FIELDS as u32,
    );
}

#[test]
#[should_panic(expected = "Metadata custom fields exceed limit")]
fn test_update_reject_over_soft_limit_custom_fields() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "UpdOverSoft");
    s.escrow.init_program_with_metadata(
        &program_id, &s.backend, &s.token.address, &s.organizer, &None, &None,
    );
    s.escrow.publish_program(&program_id, &s.backend);
    let metadata = metadata_with_n_fields(&s.env, MAX_PROGRAM_METADATA_CUSTOM_FIELDS + 1);
    s.escrow.update_program_metadata_by(&program_id, &s.backend, &metadata);
}

// ── Key-length boundary (MAX_CUSTOM_FIELD_KEY_LEN = 64) ──

#[test]
fn test_init_accept_max_key_len() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "InitMaxKeyLen");
    let metadata = metadata_with_key_value_len(&s.env, MAX_CUSTOM_FIELD_KEY_LEN, 1);
    s.escrow.init_program_with_metadata(
        &program_id, &s.backend, &s.token.address, &s.organizer, &None, &Some(metadata),
    );
    let retrieved = s.escrow.get_program_metadata(&program_id);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().custom_fields.len(), 1);
}

#[test]
#[should_panic(expected = "CustomFieldKeyTooLong")]
fn test_init_reject_over_key_len() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "InitOverKeyLen");
    let metadata = metadata_with_key_value_len(&s.env, MAX_CUSTOM_FIELD_KEY_LEN + 1, 1);
    s.escrow.init_program_with_metadata(
        &program_id, &s.backend, &s.token.address, &s.organizer, &None, &Some(metadata),
    );
}

#[test]
fn test_update_accept_max_key_len() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "UpdMaxKeyLen");
    s.escrow.init_program_with_metadata(
        &program_id, &s.backend, &s.token.address, &s.organizer, &None, &None,
    );
    s.escrow.publish_program(&program_id, &s.backend);
    let metadata = metadata_with_key_value_len(&s.env, MAX_CUSTOM_FIELD_KEY_LEN, 1);
    s.escrow.update_program_metadata_by(&program_id, &s.backend, &metadata);
    let retrieved = s.escrow.get_program_metadata(&program_id);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().custom_fields.len(), 1);
}

#[test]
#[should_panic(expected = "CustomFieldKeyTooLong")]
fn test_update_reject_over_key_len() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "UpdOverKeyLen");
    s.escrow.init_program_with_metadata(
        &program_id, &s.backend, &s.token.address, &s.organizer, &None, &None,
    );
    s.escrow.publish_program(&program_id, &s.backend);
    let metadata = metadata_with_key_value_len(&s.env, MAX_CUSTOM_FIELD_KEY_LEN + 1, 1);
    s.escrow.update_program_metadata_by(&program_id, &s.backend, &metadata);
}

// ── Value-length boundary (MAX_CUSTOM_FIELD_VALUE_LEN = 256) ──

#[test]
fn test_init_accept_max_value_len() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "InitMaxValLen");
    let metadata = metadata_with_key_value_len(&s.env, 1, MAX_CUSTOM_FIELD_VALUE_LEN);
    s.escrow.init_program_with_metadata(
        &program_id, &s.backend, &s.token.address, &s.organizer, &None, &Some(metadata),
    );
    let retrieved = s.escrow.get_program_metadata(&program_id);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().custom_fields.len(), 1);
}

#[test]
#[should_panic(expected = "CustomFieldValueTooLong")]
fn test_init_reject_over_value_len() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "InitOverValLen");
    let metadata = metadata_with_key_value_len(&s.env, 1, MAX_CUSTOM_FIELD_VALUE_LEN + 1);
    s.escrow.init_program_with_metadata(
        &program_id, &s.backend, &s.token.address, &s.organizer, &None, &Some(metadata),
    );
}

#[test]
fn test_update_accept_max_value_len() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "UpdMaxValLen");
    s.escrow.init_program_with_metadata(
        &program_id, &s.backend, &s.token.address, &s.organizer, &None, &None,
    );
    s.escrow.publish_program(&program_id, &s.backend);
    let metadata = metadata_with_key_value_len(&s.env, 1, MAX_CUSTOM_FIELD_VALUE_LEN);
    s.escrow.update_program_metadata_by(&program_id, &s.backend, &metadata);
    let retrieved = s.escrow.get_program_metadata(&program_id);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().custom_fields.len(), 1);
}

#[test]
#[should_panic(expected = "CustomFieldValueTooLong")]
fn test_update_reject_over_value_len() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "UpdOverValLen");
    s.escrow.init_program_with_metadata(
        &program_id, &s.backend, &s.token.address, &s.organizer, &None, &None,
    );
    s.escrow.publish_program(&program_id, &s.backend);
    let metadata = metadata_with_key_value_len(&s.env, 1, MAX_CUSTOM_FIELD_VALUE_LEN + 1);
    s.escrow.update_program_metadata_by(&program_id, &s.backend, &metadata);
}

// ── Shared validation function direct tests ──

#[test]
fn test_shared_validation_accepts_max_custom_fields() {
    let env = Env::default();
    let metadata = metadata_with_n_fields(&env, MAX_CUSTOM_FIELDS);
    validate_metadata_custom_fields(&metadata);
}

#[test]
#[should_panic(expected = "CustomFieldsLimitExceeded")]
fn test_shared_validation_rejects_over_max_custom_fields() {
    let env = Env::default();
    let metadata = metadata_with_n_fields(&env, MAX_CUSTOM_FIELDS + 1);
    validate_metadata_custom_fields(&metadata);
}

#[test]
fn test_shared_validation_accepts_max_key_len() {
    let env = Env::default();
    let metadata = metadata_with_key_value_len(&env, MAX_CUSTOM_FIELD_KEY_LEN, 1);
    validate_metadata_custom_fields(&metadata);
}

#[test]
#[should_panic(expected = "CustomFieldKeyTooLong")]
fn test_shared_validation_rejects_over_key_len() {
    let env = Env::default();
    let metadata = metadata_with_key_value_len(&env, MAX_CUSTOM_FIELD_KEY_LEN + 1, 1);
    validate_metadata_custom_fields(&metadata);
}

#[test]
fn test_shared_validation_accepts_max_value_len() {
    let env = Env::default();
    let metadata = metadata_with_key_value_len(&env, 1, MAX_CUSTOM_FIELD_VALUE_LEN);
    validate_metadata_custom_fields(&metadata);
}

#[test]
#[should_panic(expected = "CustomFieldValueTooLong")]
fn test_shared_validation_rejects_over_value_len() {
    let env = Env::default();
    let metadata = metadata_with_key_value_len(&env, 1, MAX_CUSTOM_FIELD_VALUE_LEN + 1);
    validate_metadata_custom_fields(&metadata);
}

// ============================================================================
// Issue #1737: Comprehensive metadata size and encoding limit tests
// ============================================================================

// ── Empty metadata ──────────────────────────────────────────────────────────

/// Empty metadata (all None, no tags, no custom fields) is valid.
#[test]
fn test_empty_metadata_accepted_on_init() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "EmptyMetaInit");
    let metadata = ProgramMetadata::empty(&s.env);
    s.escrow.init_program_with_metadata(
        &program_id, &s.backend, &s.token.address, &s.organizer, &None, &Some(metadata),
    );
    let retrieved = s.escrow.get_program_metadata(&program_id);
    assert!(retrieved.is_some());
    let m = retrieved.unwrap();
    assert_eq!(m.program_name, None);
    assert_eq!(m.custom_fields.len(), 0);
}

/// Empty metadata accepted on update (replaces existing).
#[test]
fn test_empty_metadata_accepted_on_update() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "EmptyMetaUpd");
    s.escrow.init_program_with_metadata(
        &program_id, &s.backend, &s.token.address, &s.organizer, &None, &None,
    );
    s.escrow.publish_program(&program_id, &s.backend);

    // First set some metadata
    let metadata = metadata_with_n_fields(&s.env, 3);
    s.escrow.update_program_metadata_by(&program_id, &s.backend, &metadata);

    // Then replace with empty
    let empty = ProgramMetadata::empty(&s.env);
    s.escrow.update_program_metadata_by(&program_id, &s.backend, &empty);

    let retrieved = s.escrow.get_program_metadata(&program_id);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().custom_fields.len(), 0);
}

/// None metadata on init (skips metadata storage entirely).
#[test]
fn test_none_metadata_accepted_on_init() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "NoneMetaInit");
    s.escrow.init_program_with_metadata(
        &program_id, &s.backend, &s.token.address, &s.organizer, &None, &None,
    );
    let retrieved = s.escrow.get_program_metadata(&program_id);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().custom_fields.len(), 0);
}

// ── Aggregate size boundary ────────────────────────────────────────────────

/// Build metadata with `n` fields, each contributing `key_len + value_len`
/// bytes toward the aggregate ceiling.
fn metadata_with_aggregate_bytes(env: &Env, n_fields: u32, key_len: usize, value_len: usize) -> ProgramMetadata {
    let mut custom_fields: Vec<ProgramMetadataField> = Vec::new(env);
    // Fixed-size buffers (max 513 bytes); we slice to the requested length.
    let key_buf = [b'k'; 513];
    let val_buf = [b'v'; 513];
    let key_str = core::str::from_utf8(&key_buf[..key_len]).unwrap();
    let val_str = core::str::from_utf8(&val_buf[..value_len]).unwrap();
    for _ in 0..n_fields {
        custom_fields.push_back(ProgramMetadataField {
            key: String::from_str(env, key_str),
            value: String::from_str(env, val_str),
        });
    }
    ProgramMetadata {
        program_name: None,
        program_type: None,
        ecosystem: None,
        tags: Vec::new(env),
        start_date: None,
        end_date: None,
        custom_fields,
    }
}

/// Exactly at the aggregate limit is accepted.
#[test]
fn test_shared_validation_accepts_at_aggregate_limit() {
    let env = Env::default();
    // 10 fields × (64 + 256) = 3200 bytes — well within 10 240.
    let metadata = metadata_with_aggregate_bytes(&env, 10, 64, 256);
    validate_metadata_custom_fields(&metadata);
}

/// Exceeding the aggregate limit is rejected.
#[test]
#[should_panic(expected = "MetadataAggregateSizeExceeded")]
fn test_shared_validation_rejects_over_aggregate_limit() {
    let env = Env::default();
    // 33 fields × (64 + 256) = 10 560 bytes > 10 240.
    // But 33 > MAX_CUSTOM_FIELDS (20), so we need to use exactly 20 fields
    // with oversized values to hit aggregate without hitting field count first.
    // 20 fields × (1 + 513) = 10 280 bytes > 10 240.
    let mut custom_fields: Vec<ProgramMetadataField> = Vec::new(&env);
    let val_buf = [b'v'; 513];
    let val_str = core::str::from_utf8(&val_buf[..]).unwrap();
    for _ in 0..20 {
        custom_fields.push_back(ProgramMetadataField {
            key: String::from_str(&env, "k"),
            value: String::from_str(&env, val_str),
        });
    }
    let metadata = ProgramMetadata {
        program_name: None,
        program_type: None,
        ecosystem: None,
        tags: Vec::new(&env),
        start_date: None,
        end_date: None,
        custom_fields,
    };
    validate_metadata_custom_fields(&metadata);
}

/// Exactly at aggregate limit (20 fields × 512 bytes = 10 240).
#[test]
fn test_shared_validation_accepts_exactly_at_aggregate_limit() {
    let env = Env::default();
    // 20 fields × (1 + 511) = 10 240 bytes = MAX_METADATA_AGGREGATE_BYTES.
    let mut custom_fields: Vec<ProgramMetadataField> = Vec::new(&env);
    let val_buf = [b'v'; 511];
    let val_str = core::str::from_utf8(&val_buf[..]).unwrap();
    for _ in 0..20 {
        custom_fields.push_back(ProgramMetadataField {
            key: String::from_str(&env, "k"),
            value: String::from_str(&env, val_str),
        });
    }
    let metadata = ProgramMetadata {
        program_name: None,
        program_type: None,
        ecosystem: None,
        tags: Vec::new(&env),
        start_date: None,
        end_date: None,
        custom_fields,
    };
    validate_metadata_custom_fields(&metadata);
}

/// Aggregate size check via init_program_with_metadata entrypoint.
#[test]
#[should_panic(expected = "MetadataAggregateSizeExceeded")]
fn test_init_rejects_over_aggregate_limit() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "InitOverAgg");
    // 20 fields × (1 + 513) = 10 280 > 10 240.
    let mut custom_fields: Vec<ProgramMetadataField> = Vec::new(&s.env);
    let val_buf = [b'v'; 513];
    let val_str = core::str::from_utf8(&val_buf[..]).unwrap();
    for _ in 0..20 {
        custom_fields.push_back(ProgramMetadataField {
            key: String::from_str(&s.env, "k"),
            value: String::from_str(&s.env, val_str),
        });
    }
    let metadata = ProgramMetadata {
        program_name: None,
        program_type: None,
        ecosystem: None,
        tags: Vec::new(&s.env),
        start_date: None,
        end_date: None,
        custom_fields,
    };
    s.escrow.init_program_with_metadata(
        &program_id, &s.backend, &s.token.address, &s.organizer, &None, &Some(metadata),
    );
}

/// Aggregate size check via update_program_metadata entrypoint.
#[test]
#[should_panic(expected = "MetadataAggregateSizeExceeded")]
fn test_update_rejects_over_aggregate_limit() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "UpdOverAgg");
    s.escrow.init_program_with_metadata(
        &program_id, &s.backend, &s.token.address, &s.organizer, &None, &None,
    );
    s.escrow.publish_program(&program_id, &s.backend);
    let mut custom_fields: Vec<ProgramMetadataField> = Vec::new(&s.env);
    let val_buf = [b'v'; 513];
    let val_str = core::str::from_utf8(&val_buf[..]).unwrap();
    for _ in 0..20 {
        custom_fields.push_back(ProgramMetadataField {
            key: String::from_str(&s.env, "k"),
            value: String::from_str(&s.env, val_str),
        });
    }
    let metadata = ProgramMetadata {
        program_name: None,
        program_type: None,
        ecosystem: None,
        tags: Vec::new(&s.env),
        start_date: None,
        end_date: None,
        custom_fields,
    };
    s.escrow.update_program_metadata_by(&program_id, &s.backend, &metadata);
}

// ── Duplicate keys ─────────────────────────────────────────────────────────

/// Duplicate keys in custom_fields are accepted (Soroban Vec allows them).
/// The contract does not deduplicate; consumers handle semantics.
#[test]
fn test_duplicate_keys_accepted() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "DupKeys");
    let mut custom_fields: Vec<ProgramMetadataField> = Vec::new(&s.env);
    // Add 3 fields with the same key "dup_key"
    for _ in 0..3 {
        custom_fields.push_back(ProgramMetadataField {
            key: String::from_str(&s.env, "dup_key"),
            value: String::from_str(&s.env, "val"),
        });
    }
    let metadata = ProgramMetadata {
        program_name: None,
        program_type: None,
        ecosystem: None,
        tags: Vec::new(&s.env),
        start_date: None,
        end_date: None,
        custom_fields,
    };
    s.escrow.init_program_with_metadata(
        &program_id, &s.backend, &s.token.address, &s.organizer, &None, &Some(metadata),
    );
    let retrieved = s.escrow.get_program_metadata(&program_id);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().custom_fields.len(), 3);
}

// ── UTF-8 handling ─────────────────────────────────────────────────────────

/// Multi-byte UTF-8 (e.g. emoji, CJK) is accepted and counts correctly as bytes.
#[test]
fn test_multibyte_utf8_accepted() {
    let env = Env::default();
    // "🎉" is 4 bytes in UTF-8.
    let metadata = metadata_with_key_value_len(&env, 4, 4);
    validate_metadata_custom_fields(&metadata);
    // Build with emoji content
    let mut custom_fields: Vec<ProgramMetadataField> = Vec::new(&env);
    custom_fields.push_back(ProgramMetadataField {
        key: String::from_str(&env, "🎉🔑"),
        value: String::from_str(&env, "值データ"),
    });
    let metadata = ProgramMetadata {
        program_name: None,
        program_type: None,
        ecosystem: None,
        tags: Vec::new(&env),
        start_date: None,
        end_date: None,
        custom_fields,
    };
    validate_metadata_custom_fields(&metadata);
}

// ── Update replaces (does not merge) ───────────────────────────────────────

/// update_program_metadata replaces metadata entirely, not merging.
#[test]
fn test_update_replaces_not_merges() {
    let s = Setup::new();
    let program_id = String::from_str(&s.env, "ReplaceNotMerge");
    s.escrow.init_program_with_metadata(
        &program_id, &s.backend, &s.token.address, &s.organizer, &None, &None,
    );
    s.escrow.publish_program(&program_id, &s.backend);

    // Set initial metadata with 3 fields
    let mut fields1: Vec<ProgramMetadataField> = Vec::new(&s.env);
    for i in 0..3 {
        fields1.push_back(ProgramMetadataField {
            key: String::from_str(&s.env, &std::format!("key_{}", i)),
            value: String::from_str(&s.env, &std::format!("val_{}", i)),
        });
    }
    let meta1 = ProgramMetadata {
        program_name: Some(String::from_str(&s.env, "First")),
        program_type: None,
        ecosystem: None,
        tags: Vec::new(&s.env),
        start_date: None,
        end_date: None,
        custom_fields: fields1,
    };
    s.escrow.update_program_metadata_by(&program_id, &s.backend, &meta1);
    let r1 = s.escrow.get_program_metadata(&program_id).unwrap();
    assert_eq!(r1.custom_fields.len(), 3);
    assert_eq!(r1.program_name, Some(String::from_str(&s.env, "First")));

    // Replace with 1 field — old fields must NOT persist
    let mut fields2: Vec<ProgramMetadataField> = Vec::new(&s.env);
    fields2.push_back(ProgramMetadataField {
        key: String::from_str(&s.env, "only_key"),
        value: String::from_str(&s.env, "only_val"),
    });
    let meta2 = ProgramMetadata {
        program_name: Some(String::from_str(&s.env, "Second")),
        program_type: None,
        ecosystem: None,
        tags: Vec::new(&s.env),
        start_date: None,
        end_date: None,
        custom_fields: fields2,
    };
    s.escrow.update_program_metadata_by(&program_id, &s.backend, &meta2);
    let r2 = s.escrow.get_program_metadata(&program_id).unwrap();
    assert_eq!(r2.custom_fields.len(), 1);
    assert_eq!(
        r2.custom_fields.get(0).unwrap().key,
        String::from_str(&s.env, "only_key")
    );
    assert_eq!(r2.program_name, Some(String::from_str(&s.env, "Second")));
}

// ── Storage cost documentation ─────────────────────────────────────────────

/// Verify constants are accessible from tests for documentation.
#[test]
fn test_limit_constants_are_public_and_accessible() {
    // These assertions document the actual limit values.
    assert_eq!(MAX_CUSTOM_FIELDS, 20);
    assert_eq!(MAX_CUSTOM_FIELD_KEY_LEN, 64);
    assert_eq!(MAX_CUSTOM_FIELD_VALUE_LEN, 256);
    assert_eq!(MAX_METADATA_AGGREGATE_BYTES, 10_240);
    assert_eq!(MAX_PROGRAM_METADATA_CUSTOM_FIELDS, 10);
}
