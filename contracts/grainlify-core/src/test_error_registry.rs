//! Tests for the error code registry uniqueness system.
//!
//! Coverage:
//! - GRAINLIFY_CORE_REGISTRY has no duplicate numeric codes
//! - Every ContractError variant is present in the registry with the correct code
//! - lookup_name / is_registered return correct results for known and unknown codes
//! - has_duplicate_codes correctly identifies duplicates and clean registries
//! - Shared constants in errors.rs are unique within each range and globally

#[cfg(test)]
mod tests {
    extern crate std;
    use std::vec::Vec;

    use crate::{
        error_registry::{
            has_duplicate_codes, is_registered, lookup_name, registered_count,
            RegistryEntry, GRAINLIFY_CORE_REGISTRY,
        },
        errors,
        ContractError,
    };

    // ── Registry structure ────────────────────────────────────────────────────

    #[test]
    fn test_registry_has_no_duplicates() {
        assert!(
            !has_duplicate_codes(GRAINLIFY_CORE_REGISTRY),
            "GRAINLIFY_CORE_REGISTRY must not contain duplicate error codes"
        );
    }

    #[test]
    fn test_registry_names_are_nonempty() {
        for (code, name) in GRAINLIFY_CORE_REGISTRY {
            assert!(
                !name.is_empty(),
                "Error code {code} has an empty name in the registry"
            );
        }
    }

    #[test]
    fn test_registry_codes_are_positive() {
        for (code, _) in GRAINLIFY_CORE_REGISTRY {
            assert!(*code >= 1, "Error code must be >= 1; got {code}");
        }
    }

    #[test]
    fn test_registry_is_sorted_ascending() {
        let codes: Vec<u32> = GRAINLIFY_CORE_REGISTRY.iter().map(|(c, _)| *c).collect();
        let mut sorted = codes.clone();
        sorted.sort_unstable();
        assert_eq!(
            codes, sorted,
            "Registry entries must be ordered by code ascending for readability"
        );
    }

    #[test]
    fn test_registry_entry_count() {
        assert_eq!(
            registered_count(),
            10,
            "Expected exactly 10 entries in GRAINLIFY_CORE_REGISTRY (3 common + 7 governance)"
        );
    }

    // ── lookup_name ───────────────────────────────────────────────────────────

    #[test]
    fn test_lookup_already_initialized() {
        assert_eq!(lookup_name(1), Some("AlreadyInitialized"));
    }

    #[test]
    fn test_lookup_not_initialized() {
        assert_eq!(lookup_name(2), Some("NotInitialized"));
    }

    #[test]
    fn test_lookup_not_admin() {
        assert_eq!(lookup_name(3), Some("NotAdmin"));
    }

    #[test]
    fn test_lookup_threshold_not_met() {
        assert_eq!(lookup_name(101), Some("ThresholdNotMet"));
    }

    #[test]
    fn test_lookup_proposal_not_found() {
        assert_eq!(lookup_name(102), Some("ProposalNotFound"));
    }

    #[test]
    fn test_lookup_migration_commitment_not_found() {
        assert_eq!(lookup_name(103), Some("MigrationCommitmentNotFound"));
    }

    #[test]
    fn test_lookup_migration_hash_mismatch() {
        assert_eq!(lookup_name(104), Some("MigrationHashMismatch"));
    }

    #[test]
    fn test_lookup_timelock_delay_too_high() {
        assert_eq!(lookup_name(105), Some("TimelockDelayTooHigh"));
    }

    #[test]
    fn test_lookup_snapshot_restore_admin_pending() {
        assert_eq!(lookup_name(106), Some("SnapshotRestoreAdminPending"));
    }

    #[test]
    fn test_lookup_snapshot_pruned() {
        assert_eq!(lookup_name(107), Some("SnapshotPruned"));
    }

    #[test]
    fn test_lookup_code_zero_is_none() {
        assert_eq!(lookup_name(0), None, "code 0 is reserved and must not be registered");
    }

    #[test]
    fn test_lookup_unassigned_code_4() {
        assert_eq!(lookup_name(4), None);
    }

    #[test]
    fn test_lookup_unassigned_code_100() {
        assert_eq!(lookup_name(100), None);
    }

    #[test]
    fn test_lookup_unassigned_code_108() {
        assert_eq!(lookup_name(108), None);
    }

    #[test]
    fn test_lookup_large_unknown_code() {
        assert_eq!(lookup_name(9999), None);
    }

    // ── is_registered ─────────────────────────────────────────────────────────

    #[test]
    fn test_is_registered_true_for_all_known_codes() {
        let known = [1u32, 2, 3, 101, 102, 103, 104, 105, 106, 107];
        for code in known {
            assert!(is_registered(code), "code {code} must be registered");
        }
    }

    #[test]
    fn test_is_registered_false_for_gaps() {
        let gaps = [0u32, 4, 99, 100, 108, 200, 9999];
        for code in gaps {
            assert!(!is_registered(code), "code {code} must NOT be registered");
        }
    }

    // ── ContractError enum coverage ───────────────────────────────────────────

    #[test]
    fn test_all_contract_error_variants_are_registered() {
        let variants: &[(u32, &str)] = &[
            (ContractError::AlreadyInitialized as u32, "AlreadyInitialized"),
            (ContractError::NotInitialized as u32, "NotInitialized"),
            (ContractError::NotAdmin as u32, "NotAdmin"),
            (ContractError::ThresholdNotMet as u32, "ThresholdNotMet"),
            (ContractError::ProposalNotFound as u32, "ProposalNotFound"),
            (ContractError::MigrationCommitmentNotFound as u32, "MigrationCommitmentNotFound"),
            (ContractError::MigrationHashMismatch as u32, "MigrationHashMismatch"),
            (ContractError::TimelockDelayTooHigh as u32, "TimelockDelayTooHigh"),
            (ContractError::SnapshotRestoreAdminPending as u32, "SnapshotRestoreAdminPending"),
            (ContractError::SnapshotPruned as u32, "SnapshotPruned"),
        ];
        for (code, name) in variants {
            assert!(
                is_registered(*code),
                "ContractError::{name} (code {code}) is missing from GRAINLIFY_CORE_REGISTRY"
            );
        }
    }

    #[test]
    fn test_contract_error_variant_names_match_registry() {
        let variants: &[(u32, &str)] = &[
            (ContractError::AlreadyInitialized as u32, "AlreadyInitialized"),
            (ContractError::NotInitialized as u32, "NotInitialized"),
            (ContractError::NotAdmin as u32, "NotAdmin"),
            (ContractError::ThresholdNotMet as u32, "ThresholdNotMet"),
            (ContractError::ProposalNotFound as u32, "ProposalNotFound"),
            (ContractError::MigrationCommitmentNotFound as u32, "MigrationCommitmentNotFound"),
            (ContractError::MigrationHashMismatch as u32, "MigrationHashMismatch"),
            (ContractError::TimelockDelayTooHigh as u32, "TimelockDelayTooHigh"),
            (ContractError::SnapshotRestoreAdminPending as u32, "SnapshotRestoreAdminPending"),
            (ContractError::SnapshotPruned as u32, "SnapshotPruned"),
        ];
        for (code, expected_name) in variants {
            assert_eq!(
                lookup_name(*code),
                Some(*expected_name),
                "Registry name mismatch for ContractError code {code}"
            );
        }
    }

    #[test]
    fn test_contract_error_enum_discriminants_are_unique() {
        let discriminants = [
            ContractError::AlreadyInitialized as u32,
            ContractError::NotInitialized as u32,
            ContractError::NotAdmin as u32,
            ContractError::ThresholdNotMet as u32,
            ContractError::ProposalNotFound as u32,
            ContractError::MigrationCommitmentNotFound as u32,
            ContractError::MigrationHashMismatch as u32,
            ContractError::TimelockDelayTooHigh as u32,
            ContractError::SnapshotRestoreAdminPending as u32,
            ContractError::SnapshotPruned as u32,
        ];
        for i in 0..discriminants.len() {
            for j in (i + 1)..discriminants.len() {
                assert_ne!(
                    discriminants[i], discriminants[j],
                    "ContractError discriminants[{i}]={} collides with discriminants[{j}]={}",
                    discriminants[i], discriminants[j]
                );
            }
        }
    }

    #[test]
    fn test_registry_covers_every_contract_error_discriminant() {
        // The registry length must equal the number of ContractError variants.
        // If they diverge, a variant was added to the enum but not the registry
        // (or vice-versa).
        let enum_count = 10; // update when ContractError grows
        assert_eq!(
            registered_count(),
            enum_count,
            "Registry count ({}) does not match ContractError variant count ({}). \
             Add the new variant to GRAINLIFY_CORE_REGISTRY.",
            registered_count(),
            enum_count
        );
    }

    // ── has_duplicate_codes helper ────────────────────────────────────────────

    #[test]
    fn test_duplicate_detection_finds_duplicate() {
        let dup: &[RegistryEntry] = &[(1, "Alpha"), (2, "Beta"), (1, "AlphaDuplicate")];
        assert!(has_duplicate_codes(dup));
    }

    #[test]
    fn test_duplicate_detection_no_false_positive() {
        let clean: &[RegistryEntry] = &[(1, "Alpha"), (2, "Beta"), (3, "Gamma")];
        assert!(!has_duplicate_codes(clean));
    }

    #[test]
    fn test_duplicate_detection_empty_registry() {
        assert!(!has_duplicate_codes(&[]));
    }

    #[test]
    fn test_duplicate_detection_single_entry() {
        assert!(!has_duplicate_codes(&[(42, "Only")]));
    }

    #[test]
    fn test_duplicate_detection_adjacent_pair() {
        let adj: &[RegistryEntry] = &[(5, "A"), (5, "B")];
        assert!(has_duplicate_codes(adj));
    }

    #[test]
    fn test_duplicate_detection_same_code_different_names() {
        let same: &[RegistryEntry] = &[(10, "Original"), (11, "Other"), (10, "Renamed")];
        assert!(has_duplicate_codes(same));
    }

    #[test]
    fn test_duplicate_detection_large_clean_registry() {
        // 100-entry registry with codes 1..=100 — must be clean
        let entries: Vec<RegistryEntry> = (1u32..=100).map(|c| (c, "X")).collect();
        assert!(!has_duplicate_codes(&entries));
    }

    #[test]
    fn test_duplicate_detection_last_entry_duplicate() {
        let entries: Vec<RegistryEntry> = {
            let mut v: Vec<RegistryEntry> = (1u32..=10).map(|c| (c, "X")).collect();
            v.push((1, "Dup")); // duplicate of first
            v
        };
        assert!(has_duplicate_codes(&entries));
    }

    // ── Shared constants (errors.rs) uniqueness ───────────────────────────────

    #[test]
    fn test_shared_constants_common_range_unique() {
        let codes: Vec<RegistryEntry> = [
            errors::ALREADY_INITIALIZED,
            errors::NOT_INITIALIZED,
            errors::UNAUTHORIZED,
            errors::INVALID_AMOUNT,
            errors::INSUFFICIENT_FUNDS,
            errors::DEADLINE_NOT_PASSED,
            errors::INVALID_DEADLINE,
            errors::CONTRACT_DEPRECATED,
            errors::MAINTENANCE_MODE,
            errors::PAUSED,
            errors::OVERFLOW,
            errors::UNDERFLOW,
            errors::INVALID_STATE,
            errors::NOT_PAUSED,
            errors::INVALID_ASSET_ID,
        ]
        .iter()
        .map(|&c| (c, ""))
        .collect();
        assert!(!has_duplicate_codes(&codes), "Duplicate in shared common constants (1-99)");
    }

    #[test]
    fn test_shared_constants_governance_range_unique() {
        let codes: Vec<RegistryEntry> = [
            errors::THRESHOLD_NOT_MET,
            errors::PROPOSAL_NOT_FOUND,
            errors::INVALID_THRESHOLD,
            errors::THRESHOLD_TOO_LOW,
            errors::INSUFFICIENT_STAKE,
            errors::PROPOSALS_NOT_FOUND,
            errors::PROPOSAL_NOT_ACTIVE,
            errors::VOTING_NOT_STARTED,
            errors::VOTING_ENDED,
            errors::VOTING_STILL_ACTIVE,
            errors::ALREADY_VOTED,
            errors::PROPOSAL_NOT_APPROVED,
            errors::EXECUTION_DELAY_NOT_MET,
            errors::PROPOSAL_EXPIRED,
        ]
        .iter()
        .map(|&c| (c, ""))
        .collect();
        assert!(!has_duplicate_codes(&codes), "Duplicate in shared governance constants (100-199)");
    }

    #[test]
    fn test_shared_constants_escrow_range_unique() {
        let codes: Vec<RegistryEntry> = [
            errors::BOUNTY_EXISTS,
            errors::BOUNTY_NOT_FOUND,
            errors::FUNDS_NOT_LOCKED,
            errors::INVALID_FEE_RATE,
            errors::FEE_RECIPIENT_NOT_SET,
            errors::INVALID_BATCH_SIZE,
            errors::BATCH_SIZE_MISMATCH,
            errors::DUPLICATE_BOUNTY_ID,
            errors::REFUND_NOT_APPROVED,
            errors::AMOUNT_BELOW_MINIMUM,
            errors::AMOUNT_ABOVE_MAXIMUM,
            errors::CLAIM_PENDING,
            errors::TICKET_NOT_FOUND,
            errors::TICKET_ALREADY_USED,
            errors::TICKET_EXPIRED,
            errors::PARTICIPANT_BLOCKED,
            errors::PARTICIPANT_NOT_ALLOWED,
            errors::NOT_ANONYMOUS_ESCROW,
            errors::INVALID_SELECTION_INPUT,
            errors::UPGRADE_SAFETY_CHECK_FAILED,
            errors::BOUNTY_ALREADY_INITIALIZED,
            errors::ANON_REFUND_REQUIRED,
            errors::ANON_RESOLVER_NOT_SET,
            errors::NOT_ANON_VARIANT,
            errors::USE_INFO_V2_FOR_ANON,
            errors::INVALID_LABEL,
            errors::TOO_MANY_LABELS,
            errors::LABEL_NOT_ALLOWED,
        ]
        .iter()
        .map(|&c| (c, ""))
        .collect();
        assert!(!has_duplicate_codes(&codes), "Duplicate in shared escrow constants (200-299)");
    }

    #[test]
    fn test_shared_constants_identity_range_unique() {
        let codes: Vec<RegistryEntry> = [
            errors::INVALID_SIGNATURE,
            errors::CLAIM_EXPIRED,
            errors::UNAUTHORIZED_ISSUER,
            errors::INVALID_CLAIM_FORMAT,
            errors::TRANSACTION_EXCEEDS_LIMIT,
            errors::INVALID_RISK_SCORE,
            errors::INVALID_TIER,
        ]
        .iter()
        .map(|&c| (c, ""))
        .collect();
        assert!(!has_duplicate_codes(&codes), "Duplicate in shared identity constants (300-399)");
    }

    #[test]
    fn test_shared_constants_program_escrow_range_unique() {
        let codes: Vec<RegistryEntry> = [
            errors::PROGRAM_ALREADY_EXISTS,
            errors::DUPLICATE_PROGRAM_ID,
            errors::INVALID_BATCH_SIZE_PROGRAM,
            errors::PROGRAM_NOT_FOUND,
            errors::SCHEDULE_NOT_FOUND,
            errors::ALREADY_RELEASED,
            errors::FUNDS_PAUSED,
            errors::DUPLICATE_SCHEDULE_ID,
        ]
        .iter()
        .map(|&c| (c, ""))
        .collect();
        assert!(!has_duplicate_codes(&codes), "Duplicate in shared program-escrow constants (400-499)");
    }

    #[test]
    fn test_shared_constants_globally_unique() {
        // Every constant across every range — no cross-range collisions.
        let codes: Vec<RegistryEntry> = [
            // Common
            errors::ALREADY_INITIALIZED, errors::NOT_INITIALIZED,
            errors::UNAUTHORIZED, errors::INVALID_AMOUNT,
            errors::INSUFFICIENT_FUNDS, errors::DEADLINE_NOT_PASSED,
            errors::INVALID_DEADLINE, errors::CONTRACT_DEPRECATED,
            errors::MAINTENANCE_MODE, errors::PAUSED,
            errors::OVERFLOW, errors::UNDERFLOW,
            errors::INVALID_STATE, errors::NOT_PAUSED,
            errors::INVALID_ASSET_ID,
            // Governance
            errors::THRESHOLD_NOT_MET, errors::PROPOSAL_NOT_FOUND,
            errors::INVALID_THRESHOLD, errors::THRESHOLD_TOO_LOW,
            errors::INSUFFICIENT_STAKE, errors::PROPOSALS_NOT_FOUND,
            errors::PROPOSAL_NOT_ACTIVE, errors::VOTING_NOT_STARTED,
            errors::VOTING_ENDED, errors::VOTING_STILL_ACTIVE,
            errors::ALREADY_VOTED, errors::PROPOSAL_NOT_APPROVED,
            errors::EXECUTION_DELAY_NOT_MET, errors::PROPOSAL_EXPIRED,
            // Escrow
            errors::BOUNTY_EXISTS, errors::BOUNTY_NOT_FOUND,
            errors::FUNDS_NOT_LOCKED, errors::INVALID_FEE_RATE,
            errors::FEE_RECIPIENT_NOT_SET, errors::INVALID_BATCH_SIZE,
            errors::BATCH_SIZE_MISMATCH, errors::DUPLICATE_BOUNTY_ID,
            errors::REFUND_NOT_APPROVED, errors::AMOUNT_BELOW_MINIMUM,
            errors::AMOUNT_ABOVE_MAXIMUM, errors::CLAIM_PENDING,
            errors::TICKET_NOT_FOUND, errors::TICKET_ALREADY_USED,
            errors::TICKET_EXPIRED, errors::PARTICIPANT_BLOCKED,
            errors::PARTICIPANT_NOT_ALLOWED, errors::NOT_ANONYMOUS_ESCROW,
            errors::INVALID_SELECTION_INPUT, errors::UPGRADE_SAFETY_CHECK_FAILED,
            errors::BOUNTY_ALREADY_INITIALIZED, errors::ANON_REFUND_REQUIRED,
            errors::ANON_RESOLVER_NOT_SET, errors::NOT_ANON_VARIANT,
            errors::USE_INFO_V2_FOR_ANON, errors::INVALID_LABEL,
            errors::TOO_MANY_LABELS, errors::LABEL_NOT_ALLOWED,
            // Identity
            errors::INVALID_SIGNATURE, errors::CLAIM_EXPIRED,
            errors::UNAUTHORIZED_ISSUER, errors::INVALID_CLAIM_FORMAT,
            errors::TRANSACTION_EXCEEDS_LIMIT, errors::INVALID_RISK_SCORE,
            errors::INVALID_TIER,
            // Program Escrow
            errors::PROGRAM_ALREADY_EXISTS, errors::DUPLICATE_PROGRAM_ID,
            errors::INVALID_BATCH_SIZE_PROGRAM, errors::PROGRAM_NOT_FOUND,
            errors::SCHEDULE_NOT_FOUND, errors::ALREADY_RELEASED,
            errors::FUNDS_PAUSED, errors::DUPLICATE_SCHEDULE_ID,
            // Circuit Breaker
            errors::CIRCUIT_OPEN,
        ]
        .iter()
        .map(|&c| (c, ""))
        .collect();

        assert!(
            !has_duplicate_codes(&codes),
            "Duplicate found across all shared error constants in errors.rs"
        );
    }

    #[test]
    fn test_shared_constants_respect_range_boundaries() {
        // Spot-check that each constant falls inside its declared range.
        assert!(errors::ALREADY_INITIALIZED < 100, "common range: must be < 100");
        assert!(errors::THRESHOLD_NOT_MET >= 100 && errors::THRESHOLD_NOT_MET < 200, "governance range");
        assert!(errors::BOUNTY_EXISTS >= 200 && errors::BOUNTY_EXISTS < 300, "escrow range");
        assert!(errors::INVALID_SIGNATURE >= 300 && errors::INVALID_SIGNATURE < 400, "identity range");
        assert!(errors::PROGRAM_ALREADY_EXISTS >= 400 && errors::PROGRAM_ALREADY_EXISTS < 500, "program range");
        assert!(errors::CIRCUIT_OPEN >= 1000, "circuit-breaker range");
    }

    // ── Enhanced Cross-Contract Validation Tests ───────────────────────────────────

    #[test]
    fn test_no_cross_contract_conflicts_with_clean_registry() {
        use crate::error_registry::no_cross_contract_conflicts;
        
        let other_registry: &[crate::error_registry::RegistryEntry] = &[
            (1000, "OtherError"),
            (1001, "AnotherError"),
        ];
        
        assert!(
            no_cross_contract_conflicts(other_registry),
            "Should have no conflicts with clean registry"
        );
    }

    #[test]
    fn test_no_cross_contract_conflicts_detects_conflicts() {
        use crate::error_registry::no_cross_contract_conflicts;
        
        let conflicting_registry: &[crate::error_registry::RegistryEntry] = &[
            (1, "ConflictingError"), // Conflicts with AlreadyInitialized
            (1000, "OtherError"),
        ];
        
        assert!(
            !no_cross_contract_conflicts(conflicting_registry),
            "Should detect conflict with error code 1"
        );
    }

    #[test]
    fn test_no_cross_contract_conflicts_empty_registry() {
        use crate::error_registry::no_cross_contract_conflicts;
        
        let empty_registry: &[crate::error_registry::RegistryEntry] = &[];
        
        assert!(
            no_cross_contract_conflicts(empty_registry),
            "Empty registry should have no conflicts"
        );
    }

    #[test]
    fn test_validate_ranges_all_valid() {
        use crate::error_registry::validate_ranges;
        
        assert!(
            validate_ranges(),
            "GRAINLIFY_CORE_REGISTRY should have all valid ranges"
        );
    }

    #[test]
    fn test_get_range_category_all_codes() {
        use crate::error_registry::get_range_category;
        
        // Test common range
        assert_eq!(get_range_category(1), "common");
        assert_eq!(get_range_category(50), "common");
        assert_eq!(get_range_category(99), "common");
        
        // Test governance range
        assert_eq!(get_range_category(100), "governance");
        assert_eq!(get_range_category(150), "governance");
        assert_eq!(get_range_category(199), "governance");
        
        // Test escrow range
        assert_eq!(get_range_category(200), "escrow");
        assert_eq!(get_range_category(250), "escrow");
        assert_eq!(get_range_category(299), "escrow");
        
        // Test identity range
        assert_eq!(get_range_category(300), "identity");
        assert_eq!(get_range_category(350), "identity");
        assert_eq!(get_range_category(399), "identity");
        
        // Test program escrow range
        assert_eq!(get_range_category(400), "program_escrow");
        assert_eq!(get_range_category(450), "program_escrow");
        assert_eq!(get_range_category(499), "program_escrow");
        
        // Test system range
        assert_eq!(get_range_category(1000), "system");
        assert_eq!(get_range_category(5000), "system");
        assert_eq!(get_range_category(9999), "system");
        
        // Test unknown codes
        assert_eq!(get_range_category(0), "unknown");
        assert_eq!(get_range_category(100), "governance"); // Boundary
        assert_eq!(get_range_category(999), "unknown");
    }

    #[test]
    fn test_registry_entries_have_correct_ranges() {
        use crate::error_registry::get_range_category;
        
        for (code, name) in GRAINLIFY_CORE_REGISTRY {
            let category = get_range_category(*code);
            assert_ne!(
                category, "unknown",
                "Error code {code} ({name}) should have a valid range category"
            );
            
            // Verify the category makes sense for the error name
            match *code {
                1..=99 => assert_eq!(category, "common"),
                100..=199 => assert_eq!(category, "governance"),
                200..=299 => assert_eq!(category, "escrow"),
                300..=399 => assert_eq!(category, "identity"),
                400..=499 => assert_eq!(category, "program_escrow"),
                1000..=9999 => assert_eq!(category, "system"),
                _ => panic!("Error code {code} ({name}) is out of all valid ranges"),
            }
        }
    }

    #[test]
    fn test_registry_coverage_completeness() {
        // Ensure all error codes from ContractError are in the registry
        let contract_codes = [
            ContractError::AlreadyInitialized as u32,
            ContractError::NotInitialized as u32,
            ContractError::NotAdmin as u32,
            ContractError::ThresholdNotMet as u32,
            ContractError::ProposalNotFound as u32,
            ContractError::MigrationCommitmentNotFound as u32,
            ContractError::MigrationHashMismatch as u32,
            ContractError::TimelockDelayTooHigh as u32,
            ContractError::SnapshotRestoreAdminPending as u32,
            ContractError::SnapshotPruned as u32,
        ];
        
        for code in contract_codes {
            assert!(
                is_registered(code),
                "ContractError with code {code} must be in the registry"
            );
        }
        
        // Ensure registry doesn't have extra codes
        assert_eq!(
            registered_count(),
            contract_codes.len(),
            "Registry should have exactly the same number of entries as ContractError variants"
        );
    }

    #[test]
    fn test_error_code_sequentiality_within_ranges() {
        // While not strictly required, error codes should be reasonably sequential
        // within their ranges to avoid gaps that could cause confusion
        
        let mut common_codes: Vec<u32> = Vec::new();
        let mut governance_codes: Vec<u32> = Vec::new();
        
        for (code, _) in GRAINLIFY_CORE_REGISTRY {
            if *code <= 99 {
                common_codes.push(*code);
            } else if *code <= 199 {
                governance_codes.push(*code);
            }
        }
        
        // Sort to check for large gaps
        common_codes.sort();
        governance_codes.sort();
        
        // Check that there are no huge gaps in the common range
        for i in 1..common_codes.len() {
            let gap = common_codes[i] - common_codes[i-1];
            assert!(
                gap <= 10, // Allow some gaps but not huge ones
                "Large gap of {gap} in common error codes between {} and {}",
                common_codes[i-1], common_codes[i]
            );
        }
        
        // Similar check for governance range
        for i in 1..governance_codes.len() {
            let gap = governance_codes[i] - governance_codes[i-1];
            assert!(
                gap <= 10,
                "Large gap of {gap} in governance error codes between {} and {}",
                governance_codes[i-1], governance_codes[i]
            );
        }
    }

    #[test]
    fn test_error_names_follow_conventions() {
        for (code, name) in GRAINLIFY_CORE_REGISTRY {
            // Names should be PascalCase
            assert!(
                name.chars().next().unwrap().is_uppercase(),
                "Error code {code} name '{name}' should start with uppercase"
            );
            
            // Names should not contain underscores (use PascalCase instead)
            assert!(
                !name.contains('_'),
                "Error code {code} name '{name}' should use PascalCase, not underscores"
            );
            
            // Names should be descriptive
            assert!(
                name.len() >= 5,
                "Error code {code} name '{name}' should be at least 5 characters"
            );
        }
    }
}
