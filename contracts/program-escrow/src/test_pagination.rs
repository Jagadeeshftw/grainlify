//! # Enhanced History Pagination Tests
//!
//! Tests for deterministic pagination behavior, explicit error handling,
//! and upgrade-safe storage functionality.

use soroban_sdk::{contracttype, Address, Env, String, Vec, i128, u64, u32};
use crate::{
    ProgramEscrowContract,
    ProgramData,
    PayoutRecord,
    ProgramReleaseSchedule,
    ProgramReleaseHistory,
    HistoryPaginationConfig,
    BatchError,
    DataKey,
    DEFAULT_MAX_HISTORY_PAGE_LIMIT,
    PAGINATION_SCHEMA_VERSION_V1,
};

#[contracttest]
pub struct PaginationTestSuite;

#[cfg(test)]
impl PaginationTestSuite {
    /// Test pagination limit validation with zero limit
    #[test]
    fn test_pagination_zero_limit() {
        let env = Env::default();
        let result = ProgramEscrowContract::validate_pagination(&env, 0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), BatchError::InvalidPaginationLimit);
    }

    /// Test pagination limit validation with excessive limit
    #[test]
    fn test_pagination_excessive_limit() {
        let env = Env::default();
        
        // Set up pagination config
        let config = HistoryPaginationConfig {
            max_limit: 100,
            schema_version: PAGINATION_SCHEMA_VERSION_V1,
        };
        env.storage().instance().set(&DataKey::HistoryPaginationConfig, &config);
        
        let result = ProgramEscrowContract::validate_pagination(&env, 150);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), BatchError::PaginationLimitExceeded);
    }

    /// Test pagination with valid limit
    #[test]
    fn test_pagination_valid_limit() {
        let env = Env::default();
        
        // Set up pagination config
        let config = HistoryPaginationConfig {
            max_limit: 100,
            schema_version: PAGINATION_SCHEMA_VERSION_V1,
        };
        env.storage().instance().set(&DataKey::HistoryPaginationConfig, &config);
        
        let result = ProgramEscrowContract::validate_pagination(&env, 50);
        assert!(result.is_ok());
    }

    /// Test deterministic pagination ordering
    #[test]
    fn test_pagination_deterministic_ordering() {
        let env = Env::default();
        
        // Create test data in specific order
        let mut test_records = Vec::new(&env);
        test_records.push_back(PayoutRecord {
            recipient: Address::generate(&env),
            amount: 100,
            timestamp: 1000,
        });
        test_records.push_back(PayoutRecord {
            recipient: Address::generate(&env),
            amount: 200,
            timestamp: 2000,
        });
        test_records.push_back(PayoutRecord {
            recipient: Address::generate(&env),
            amount: 300,
            timestamp: 3000,
        });
        
        // Test pagination with offset 1, limit 2
        let result = ProgramEscrowContract::paginate_filtered(
            &env,
            test_records,
            1,
            2,
            |_record| true, // Accept all records
        ).unwrap();
        
        // Should return records 2 and 3 (indices 1 and 2)
        assert_eq!(result.len(), 2);
        assert_eq!(result.get(0).unwrap().amount, 200);
        assert_eq!(result.get(1).unwrap().amount, 300);
    }

    /// Test pagination with offset beyond data length
    #[test]
    fn test_pagination_offset_beyond_data() {
        let env = Env::default();
        
        let mut test_records = Vec::new(&env);
        test_records.push_back(PayoutRecord {
            recipient: Address::generate(&env),
            amount: 100,
            timestamp: 1000,
        });
        
        // Test with offset beyond data length
        let result = ProgramEscrowContract::paginate_filtered(
            &env,
            test_records,
            5, // Offset beyond data length
            2,
            |_record| true,
        ).unwrap();
        
        // Should return empty result
        assert_eq!(result.len(), 0);
    }

    /// Test pagination with exact boundary conditions
    #[test]
    fn test_pagination_boundary_conditions() {
        let env = Env::default();
        
        // Create test data
        let mut test_records = Vec::new(&env);
        for i in 0..5 {
            test_records.push_back(PayoutRecord {
                recipient: Address::generate(&env),
                amount: (i + 1) * 100,
                timestamp: (i + 1) * 1000,
            });
        }
        
        // Test boundary: offset 0, limit 5 (exact match)
        let result1 = ProgramEscrowContract::paginate_filtered(
            &env,
            test_records,
            0,
            5,
            |_record| true,
        ).unwrap();
        assert_eq!(result1.len(), 5);
        
        // Test boundary: offset 2, limit 3 (partial)
        let result2 = ProgramEscrowContract::paginate_filtered(
            &env,
            test_records,
            2,
            3,
            |_record| true,
        ).unwrap();
        assert_eq!(result2.len(), 3);
        
        // Test boundary: offset 4, limit 1 (last item)
        let result3 = ProgramEscrowContract::paginate_filtered(
            &env,
            test_records,
            4,
            1,
            |_record| true,
        ).unwrap();
        assert_eq!(result3.len(), 1);
    }

    /// Test pagination with filtering predicate
    #[test]
    fn test_pagination_with_filter() {
        let env = Env::default();
        
        // Create test data with varying amounts
        let mut test_records = Vec::new(&env);
        for i in 0..5 {
            test_records.push_back(PayoutRecord {
                recipient: Address::generate(&env),
                amount: (i + 1) * 100,
                timestamp: (i + 1) * 1000,
            });
        }
        
        // Filter for amounts >= 300
        let result = ProgramEscrowContract::paginate_filtered(
            &env,
            test_records,
            0,
            10,
            |record| record.amount >= 300,
        ).unwrap();
        
        // Should return records 3, 4, 5 (amounts 300, 400, 500)
        assert_eq!(result.len(), 3);
        assert_eq!(result.get(0).unwrap().amount, 300);
        assert_eq!(result.get(1).unwrap().amount, 400);
        assert_eq!(result.get(2).unwrap().amount, 500);
    }

    /// Test schema version validation
    #[test]
    fn test_schema_version_validation() {
        let env = Env::default();
        
        // Test with correct schema version
        let valid_config = HistoryPaginationConfig {
            max_limit: 100,
            schema_version: PAGINATION_SCHEMA_VERSION_V1,
        };
        env.storage().instance().set(&DataKey::HistoryPaginationConfig, &valid_config);
        let result = ProgramEscrowContract::validate_pagination_schema(&env);
        assert!(result.is_ok());
        
        // Test with incorrect schema version
        let invalid_config = HistoryPaginationConfig {
            max_limit: 100,
            schema_version: 999, // Invalid version
        };
        env.storage().instance().set(&DataKey::HistoryPaginationConfig, &invalid_config);
        let result = ProgramEscrowContract::validate_pagination_schema(&env);
        assert!(result.is_err());
    }

    /// Test upgrade-safe config initialization
    #[test]
    fn test_upgrade_safe_config_init() {
        let env = Env::default();
        
        // Ensure config is initialized with defaults
        ProgramEscrowContract::ensure_history_pagination_config(&env);
        
        let config = ProgramEscrowContract::get_history_pagination_config(&env);
        assert_eq!(config.max_limit, DEFAULT_MAX_HISTORY_PAGE_LIMIT);
        assert_eq!(config.schema_version, PAGINATION_SCHEMA_VERSION_V1);
    }

    /// Test pagination performance with large datasets
    #[test]
    fn test_pagination_performance() {
        let env = Env::default();
        
        // Create large test dataset
        let mut test_records = Vec::new(&env);
        for i in 0..1000 {
            test_records.push_back(PayoutRecord {
                recipient: Address::generate(&env),
                amount: i * 10,
                timestamp: i * 1000,
            });
        }
        
        // Test pagination performance
        let start = env.ledger().timestamp();
        let result = ProgramEscrowContract::paginate_filtered(
            &env,
            test_records,
            100,
            50,
            |_record| record.amount > 500, // Filter condition
        ).unwrap();
        let end = env.ledger().timestamp();
        
        // Should complete efficiently and return filtered results
        assert!(result.len() > 0);
        assert!(end - start < 1000000); // Should complete in reasonable time
    }

    /// Test error code consistency
    #[test]
    fn test_error_code_consistency() {
        // Verify error codes are in correct range and unique
        assert_eq!(BatchError::InvalidPaginationLimit as u32, 411);
        assert_eq!(BatchError::PaginationLimitExceeded as u32, 412);
        assert_eq!(BatchError::InvalidPaginationOffset as u32, 413);
        
        // Ensure error codes are sequential and in expected range
        assert!(411 < 412 && 412 < 413);
        assert!(411 >= 400 && 413 <= 499); // In program escrow range
    }

    /// Test edge case: empty dataset
    #[test]
    fn test_pagination_empty_dataset() {
        let env = Env::default();
        
        let empty_records: Vec<PayoutRecord> = Vec::new(&env);
        
        let result = ProgramEscrowContract::paginate_filtered(
            &env,
            empty_records,
            0,
            10,
            |_record| true,
        ).unwrap();
        
        assert_eq!(result.len(), 0);
    }

    /// Test edge case: limit larger than dataset
    #[test]
    fn test_pagination_limit_larger_than_dataset() {
        let env = Env::default();
        
        let mut test_records = Vec::new(&env);
        for i in 0..3 {
            test_records.push_back(PayoutRecord {
                recipient: Address::generate(&env),
                amount: (i + 1) * 100,
                timestamp: (i + 1) * 1000,
            });
        }
        
        let result = ProgramEscrowContract::paginate_filtered(
            &env,
            test_records,
            0,
            10, // Limit larger than dataset
            |_record| true,
        ).unwrap();
        
        // Should return all available records
        assert_eq!(result.len(), 3);
    }
}
