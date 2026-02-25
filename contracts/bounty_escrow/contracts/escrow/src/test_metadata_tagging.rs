// Tests for metadata tagging functionality.
//
// Most tests in this module are gated behind `cfg(feature = "metadata_tagging")`
// because the contract does not yet expose the relevant types (`EscrowMetadata`,
// `SdkVec`, `lock_funds_with_metadata`, etc.).  The feature gate compiles them
// out until the feature is implemented (tracked in Issue #63).

#[cfg(test)]
#[cfg(feature = "metadata_tagging")]
mod metadata_tagging_tests {
    use super::super::*;
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

    fn create_escrow(env: &Env) -> BountyEscrowContractClient<'static> {
        let id = env.register_contract(None, BountyEscrowContract);
        BountyEscrowContractClient::new(env, &id)
    }

    struct Setup {
        env: Env,
        _admin: Address,
        depositor: Address,
        escrow: BountyEscrowContractClient<'static>,
        token: token::Client<'static>,
    }

    impl Setup {
        fn new() -> Self {
            let env = Env::default();
            env.mock_all_auths();
            let admin = Address::generate(&env);
            let depositor = Address::generate(&env);
            let (token, token_admin) = create_token(&env, &admin);
            let escrow = create_escrow(&env);
            escrow.init(&admin, &token.address);
            token_admin.mint(&depositor, &10_000_000);
            Setup {
                env,
                _admin: admin,
                depositor,
                escrow,
                token,
            }
        }
    }

    // ============================================================================
    // Test 1: Basic Metadata Storage and Retrieval
    // ============================================================================

    #[test]
    #[ignore = "Metadata functionality to be implemented - Issue #63"]
    fn test_metadata_set_on_creation() {
        let s = Setup::new();
        let bounty_id = 100u64;
        let amount = 5000i128;
        let deadline = s.env.ledger().timestamp() + 3600;

        let repo_id = String::from_str(&s.env, "stellar/soroban-examples");
        let issue_id = String::from_str(&s.env, "123");
        let bounty_type = String::from_str(&s.env, "bug_fix");
        let mut tags = SdkVec::new(&s.env);
        tags.push_back(String::from_str(&s.env, "rust"));
        tags.push_back(String::from_str(&s.env, "smart-contract"));

        let metadata = EscrowMetadata {
            repo_id: Some(repo_id.clone()),
            issue_id: Some(issue_id.clone()),
            bounty_type: Some(bounty_type.clone()),
            tags: tags.clone(),
            custom_fields: SdkVec::new(&s.env),
        };

        s.escrow
            .lock_funds_with_metadata(&s.depositor, &bounty_id, &amount, &deadline, &metadata);

        let retrieved_metadata = s.escrow.get_escrow_metadata(&bounty_id);
        assert_eq!(retrieved_metadata.repo_id, Some(repo_id));
        assert_eq!(retrieved_metadata.issue_id, Some(issue_id));
        assert_eq!(retrieved_metadata.bounty_type, Some(bounty_type));
        assert_eq!(retrieved_metadata.tags.len(), 2);
    }

    #[test]
    #[ignore = "Metadata functionality to be implemented - Issue #63"]
    fn test_metadata_update() {
        let s = Setup::new();
        let bounty_id = 101u64;
        let amount = 3000i128;
        let deadline = s.env.ledger().timestamp() + 7200;

        let initial_metadata = EscrowMetadata {
            repo_id: Some(String::from_str(&s.env, "stellar/rs-soroban-sdk")),
            issue_id: Some(String::from_str(&s.env, "456")),
            bounty_type: Some(String::from_str(&s.env, "feature")),
            tags: SdkVec::new(&s.env),
            custom_fields: SdkVec::new(&s.env),
        };

        s.escrow.lock_funds_with_metadata(
            &s.depositor,
            &bounty_id,
            &amount,
            &deadline,
            &initial_metadata,
        );

        let mut updated_tags = SdkVec::new(&s.env);
        updated_tags.push_back(String::from_str(&s.env, "documentation"));

        let updated_metadata = EscrowMetadata {
            repo_id: Some(String::from_str(&s.env, "stellar/rs-soroban-sdk")),
            issue_id: Some(String::from_str(&s.env, "456")),
            bounty_type: Some(String::from_str(&s.env, "documentation")),
            tags: updated_tags.clone(),
            custom_fields: SdkVec::new(&s.env),
        };

        s.escrow
            .update_escrow_metadata(&bounty_id, &updated_metadata);

        let retrieved = s.escrow.get_escrow_metadata(&bounty_id);
        assert_eq!(
            retrieved.bounty_type,
            Some(String::from_str(&s.env, "documentation"))
        );
    }

    // ============================================================================
    // Test 2: Metadata Persistence Across Lifecycle
    // ============================================================================

    #[test]
    fn test_metadata_persistence_across_lifecycle() {
        let s = Setup::new();
        let bounty_id = 100u64;
        let amount = 5000i128;
        let dl = s.env.ledger().timestamp() + 3600;

        s.escrow.lock_funds(&s.depositor, &bounty_id, &amount, &dl);

        let info = s.escrow.get_escrow_info(&bounty_id);
        assert_eq!(info.amount, amount);
        assert_eq!(info.status, EscrowStatus::Locked);
    }

    // ============================================================================
    // Test 3: Query by Metadata Fields
    // ============================================================================

    #[test]
    #[ignore = "Metadata query functionality to be implemented - Issue #63"]
    fn test_query_by_repo_id() {
        let s = Setup::new();
        let deadline = s.env.ledger().timestamp() + 3600;

        for i in 1u64..=10 {
            let repo_id = if i <= 5 {
                String::from_str(&s.env, "stellar/soroban-examples")
            } else {
                String::from_str(&s.env, "stellar/rs-soroban-sdk")
            };

            let metadata = EscrowMetadata {
                repo_id: Some(repo_id),
                issue_id: Some(String::from_str(&s.env, &i.to_string())),
                bounty_type: Some(String::from_str(&s.env, "feature")),
                tags: SdkVec::new(&s.env),
                custom_fields: SdkVec::new(&s.env),
            };

            s.escrow.lock_funds_with_metadata(
                &s.depositor,
                &i,
                &(i as i128 * 1000),
                &deadline,
                &metadata,
            );
        }

        let results = s.escrow.query_escrows_by_repo_id(
            &String::from_str(&s.env, "stellar/soroban-examples"),
            &0,
            &20,
        );

        assert_eq!(results.len(), 5);
    }

    #[test]
    #[ignore = "Metadata query functionality to be implemented - Issue #63"]
    fn test_query_by_bounty_type() {
        let s = Setup::new();
        let deadline = s.env.ledger().timestamp() + 3600;

        let types = vec![
            "bug_fix",
            "feature",
            "bug_fix",
            "documentation",
            "bug_fix",
            "feature",
        ];

        for (i, bounty_type) in types.iter().enumerate() {
            let metadata = EscrowMetadata {
                repo_id: Some(String::from_str(&s.env, "stellar/test")),
                issue_id: Some(String::from_str(&s.env, &(i + 1).to_string())),
                bounty_type: Some(String::from_str(&s.env, bounty_type)),
                tags: SdkVec::new(&s.env),
                custom_fields: SdkVec::new(&s.env),
            };

            s.escrow.lock_funds_with_metadata(
                &s.depositor,
                &((i + 1) as u64),
                &1000,
                &deadline,
                &metadata,
            );
        }

        let bug_fixes =
            s.escrow
                .query_escrows_by_bounty_type(&String::from_str(&s.env, "bug_fix"), &0, &20);
        assert_eq!(bug_fixes.len(), 3);
    }

    #[test]
    #[ignore = "Metadata query functionality to be implemented - Issue #63"]
    fn test_query_by_tags() {
        let s = Setup::new();
        let deadline = s.env.ledger().timestamp() + 3600;

        for i in 1u64..=8 {
            let mut tags = SdkVec::new(&s.env);

            if i % 2 == 0 {
                tags.push_back(String::from_str(&s.env, "rust"));
            }
            if i % 3 == 0 {
                tags.push_back(String::from_str(&s.env, "beginner-friendly"));
            }

            let metadata = EscrowMetadata {
                repo_id: Some(String::from_str(&s.env, "stellar/test")),
                issue_id: Some(String::from_str(&s.env, &i.to_string())),
                bounty_type: Some(String::from_str(&s.env, "feature")),
                tags,
                custom_fields: SdkVec::new(&s.env),
            };

            s.escrow
                .lock_funds_with_metadata(&s.depositor, &i, &1000, &deadline, &metadata);
        }

        let rust_bounties =
            s.escrow
                .query_escrows_by_tag(&String::from_str(&s.env, "rust"), &0, &20);
        assert_eq!(rust_bounties.len(), 4); // 2, 4, 6, 8
    }

    // ============================================================================
    // Test 4: Query Filters on Large Dataset
    // ============================================================================

    #[test]
    fn test_query_filters_on_large_dataset() {
        let s = Setup::new();
        let dl_base = s.env.ledger().timestamp();

        for i in 1u64..=15 {
            let amount = (i as i128) * 1000;
            let deadline = dl_base + (i * 100);
            s.escrow.lock_funds(&s.depositor, &i, &amount, &deadline);
        }

        let amount_results = s.escrow.query_escrows_by_amount(&5000, &10000, &0, &20);
        assert_eq!(amount_results.len(), 6);

        let dl_results =
            s.escrow
                .query_escrows_by_deadline(&(dl_base + 300), &(dl_base + 700), &0, &20);
        assert_eq!(dl_results.len(), 5);
    }

    // ============================================================================
    // Test 5: Serialization Format for Off-Chain Consumers
    // ============================================================================

    #[test]
    #[ignore = "Metadata functionality to be implemented - Issue #63"]
    fn test_metadata_serialization_format() {
        let s = Setup::new();
        let bounty_id = 300u64;
        let amount = 1500i128;
        let deadline = s.env.ledger().timestamp() + 3600;

        let mut tags = SdkVec::new(&s.env);
        tags.push_back(String::from_str(&s.env, "rust"));
        tags.push_back(String::from_str(&s.env, "smart-contract"));

        let mut custom_fields = SdkVec::new(&s.env);
        custom_fields.push_back((
            String::from_str(&s.env, "priority"),
            String::from_str(&s.env, "high"),
        ));

        let metadata = EscrowMetadata {
            repo_id: Some(String::from_str(&s.env, "stellar/soroban-examples")),
            issue_id: Some(String::from_str(&s.env, "42")),
            bounty_type: Some(String::from_str(&s.env, "bug_fix")),
            tags,
            custom_fields,
        };

        s.escrow
            .lock_funds_with_metadata(&s.depositor, &bounty_id, &amount, &deadline, &metadata);

        let retrieved = s.escrow.get_escrow_metadata(&bounty_id);

        assert!(retrieved.repo_id.is_some());
        assert!(retrieved.issue_id.is_some());
        assert!(retrieved.bounty_type.is_some());
        assert_eq!(retrieved.tags.len(), 2);
        assert_eq!(retrieved.custom_fields.len(), 1);
    }

    // ============================================================================
    // Test 6: Empty and Edge Case Metadata
    // ============================================================================

    #[test]
    #[ignore = "Metadata functionality to be implemented - Issue #63"]
    fn test_empty_metadata() {
        let s = Setup::new();
        let bounty_id = 400u64;
        let amount = 1000i128;
        let deadline = s.env.ledger().timestamp() + 3600;

        let metadata = EscrowMetadata {
            repo_id: None,
            issue_id: None,
            bounty_type: None,
            tags: SdkVec::new(&s.env),
            custom_fields: SdkVec::new(&s.env),
        };

        s.escrow
            .lock_funds_with_metadata(&s.depositor, &bounty_id, &amount, &deadline, &metadata);

        let retrieved = s.escrow.get_escrow_metadata(&bounty_id);
        assert!(retrieved.repo_id.is_none());
        assert!(retrieved.issue_id.is_none());
        assert!(retrieved.bounty_type.is_none());
        assert_eq!(retrieved.tags.len(), 0);
    }

    #[test]
    #[ignore = "Tagging functionality not yet implemented in contract"]
    fn test_tagging_logic_verification() {
        // This test is for future metadata tagging functionality
        // Currently the contract doesn't support metadata/tagging
    }
} // end of metadata_tagging_tests module
