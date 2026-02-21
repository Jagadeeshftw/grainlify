//! # Cross-Contract Interface Compatibility Tests
//!
//! These tests verify that contracts correctly implement the shared interfaces
//! and maintain ABI compatibility across versions.
//!
//! ## Test Categories
//!
//! 1. **Interface Version Tests**: Verify contracts report correct interface version
//! 2. **Signature Compatibility Tests**: Ensure function signatures match interface
//! 3. **Behavior Compatibility Tests**: Verify consistent behavior across implementations
//! 4. **Breaking Change Detection**: Catch ABI-breaking changes early

#![cfg(test)]

use soroban_sdk::{
    testutils::Address as _,
    Address, Env, String, Vec,
};

// Import the shared interfaces
use crate::{AdminManaged, BountyEscrowTrait, CommonError, EscrowStatus, Pausable, ProgramEscrowTrait, Versioned,
    INTERFACE_VERSION_MAJOR, INTERFACE_VERSION_MINOR, INTERFACE_VERSION_PATCH};

/// Test helper to create a test environment
fn create_test_env() -> Env {
    Env::default()
}

/// Test helper to create a test address
fn create_test_address(env: &Env) -> Address {
    Address::generate(env)
}

// ============================================================================
// Interface Version Tests
// ============================================================================

#[test]
fn test_interface_version_constants() {
    // Verify interface version is defined and follows semver
    assert_eq!(INTERFACE_VERSION_MAJOR, 1, "Major version should be 1");
    assert_eq!(INTERFACE_VERSION_MINOR, 0, "Minor version should be 0");
    assert_eq!(INTERFACE_VERSION_PATCH, 0, "Patch version should be 0");
}

#[test]
fn test_interface_version_stability() {
    // Interface version should remain stable across minor releases
    // Breaking changes should only come in major versions
    let current_major = INTERFACE_VERSION_MAJOR;
    
    // This test documents the expected stability guarantee
    // When updating the interface, update this test intentionally
    assert!(
        current_major >= 1,
        "Interface major version should be at least 1"
    );
}

// ============================================================================
// Common Error Tests
// ============================================================================

#[test]
fn test_common_error_codes() {
    // Verify error codes are defined consistently
    assert_eq!(CommonError::NotInitialized as u32, 100);
    assert_eq!(CommonError::AlreadyInitialized as u32, 101);
    assert_eq!(CommonError::Unauthorized as u32, 102);
    assert_eq!(CommonError::InvalidAmount as u32, 103);
    assert_eq!(CommonError::InsufficientBalance as u32, 104);
    assert_eq!(CommonError::Paused as u32, 105);
    assert_eq!(CommonError::NotFound as u32, 106);
}

// ============================================================================
// Escrow Status Tests
// ============================================================================

#[test]
fn test_escrow_status_variants() {
    // Verify all status variants exist
    let _locked = EscrowStatus::Locked;
    let _released = EscrowStatus::Released;
    let _refunded = EscrowStatus::Refunded;
    let _partial = EscrowStatus::PartiallyReleased;
}

#[test]
fn test_escrow_status_equality() {
    // Verify status equality works correctly
    assert_eq!(EscrowStatus::Locked, EscrowStatus::Locked);
    assert_ne!(EscrowStatus::Locked, EscrowStatus::Released);
    assert_ne!(EscrowStatus::Released, EscrowStatus::Refunded);
    assert_ne!(EscrowStatus::Locked, EscrowStatus::PartiallyReleased);
}

// ============================================================================
// Trait Bounds Tests
// ============================================================================

/// Mock implementation of BountyEscrowTrait for testing trait bounds
struct MockBountyEscrow;

impl BountyEscrowTrait for MockBountyEscrow {
    fn init(_env: Env, _admin: Address, _token: Address) -> Result<(), CommonError> {
        Ok(())
    }

    fn lock_funds(
        _env: Env,
        _bounty_id: u64,
        _amount: i128,
        _depositor: Address,
        _deadline: Option<u64>,
    ) -> Result<(), CommonError> {
        Ok(())
    }

    fn release_funds(_env: Env, _bounty_id: u64, _contributor: Address) -> Result<(), CommonError> {
        Ok(())
    }

    fn refund(_env: Env, _bounty_id: u64) -> Result<(), CommonError> {
        Ok(())
    }

    fn get_balance(_env: Env, _bounty_id: u64) -> i128 {
        0
    }

    fn get_status(_env: Env, _bounty_id: u64) -> Option<EscrowStatus> {
        Some(EscrowStatus::Locked)
    }
}

/// Mock implementation of ProgramEscrowTrait for testing trait bounds
struct MockProgramEscrow;

impl ProgramEscrowTrait for MockProgramEscrow {
    fn init_program(
        _env: Env,
        _program_id: String,
        _admin: Address,
        _token: Address,
    ) -> Result<(), CommonError> {
        Ok(())
    }

    fn lock_program_funds(_env: Env, _amount: i128) -> Result<(), CommonError> {
        Ok(())
    }

    fn batch_payout(
        _env: Env,
        _recipients: Vec<Address>,
        _amounts: Vec<i128>,
    ) -> Result<(), CommonError> {
        Ok(())
    }

    fn single_payout(_env: Env, _recipient: Address, _amount: i128) -> Result<(), CommonError> {
        Ok(())
    }

    fn get_remaining_balance(_env: Env) -> i128 {
        0
    }

    fn program_exists(_env: Env, _program_id: String) -> bool {
        true
    }
}

/// Mock implementation of Pausable for testing trait bounds
struct MockPausable;

impl Pausable for MockPausable {
    fn is_lock_paused(_env: &Env) -> bool {
        false
    }

    fn is_release_paused(_env: &Env) -> bool {
        false
    }

    fn is_refund_paused(_env: &Env) -> bool {
        false
    }

    fn set_paused(
        _env: Env,
        _lock: Option<bool>,
        _release: Option<bool>,
        _refund: Option<bool>,
    ) -> Result<(), CommonError> {
        Ok(())
    }
}

/// Mock implementation of AdminManaged for testing trait bounds
struct MockAdminManaged;

impl AdminManaged for MockAdminManaged {
    fn get_admin(_env: Env) -> Option<Address> {
        None
    }

    fn transfer_admin(_env: Env, _new_admin: Address) -> Result<(), CommonError> {
        Ok(())
    }
}

/// Mock implementation of Versioned for testing trait bounds
struct MockVersioned;

impl Versioned for MockVersioned {
    fn get_version(_env: Env) -> u32 {
        1
    }
}

#[test]
fn test_bounty_escrow_trait_bounds() {
    let env = create_test_env();
    let admin = create_test_address(&env);
    let token = create_test_address(&env);
    let depositor = create_test_address(&env);
    let contributor = create_test_address(&env);

    // Test that MockBountyEscrow implements BountyEscrowTrait
    let _ = MockBountyEscrow::init(env.clone(), admin.clone(), token.clone());
    let _ = MockBountyEscrow::lock_funds(env.clone(), 1, 100, depositor.clone(), None);
    let _ = MockBountyEscrow::release_funds(env.clone(), 1, contributor.clone());
    let _ = MockBountyEscrow::refund(env.clone(), 1);
    let _ = MockBountyEscrow::get_balance(env.clone(), 1);
    let _ = MockBountyEscrow::get_status(env.clone(), 1);
}

#[test]
fn test_program_escrow_trait_bounds() {
    let env = create_test_env();
    let admin = create_test_address(&env);
    let token = create_test_address(&env);
    let recipient = create_test_address(&env);

    // Test that MockProgramEscrow implements ProgramEscrowTrait
    let program_id = String::from_str(&env, "test-program");
    let _ = MockProgramEscrow::init_program(env.clone(), program_id.clone(), admin.clone(), token.clone());
    let _ = MockProgramEscrow::lock_program_funds(env.clone(), 1000);
    let _ = MockProgramEscrow::single_payout(env.clone(), recipient.clone(), 100);
    let _ = MockProgramEscrow::get_remaining_balance(env.clone());
    let _ = MockProgramEscrow::program_exists(env.clone(), program_id);
}

#[test]
fn test_pausable_trait_bounds() {
    let env = create_test_env();

    // Test that MockPausable implements Pausable
    let _ = MockPausable::is_lock_paused(&env);
    let _ = MockPausable::is_release_paused(&env);
    let _ = MockPausable::is_refund_paused(&env);
    let _ = MockPausable::set_paused(env.clone(), Some(true), Some(false), None);
}

#[test]
fn test_admin_managed_trait_bounds() {
    let env = create_test_env();
    let new_admin = create_test_address(&env);

    // Test that MockAdminManaged implements AdminManaged
    let _ = MockAdminManaged::get_admin(env.clone());
    let _ = MockAdminManaged::transfer_admin(env.clone(), new_admin);
}

#[test]
fn test_versioned_trait_bounds() {
    let env = create_test_env();

    // Test that MockVersioned implements Versioned
    let version = MockVersioned::get_version(env.clone());
    assert!(version > 0, "Version should be positive");

    let (major, minor, patch) = MockVersioned::get_interface_version(env.clone());
    assert_eq!(major, INTERFACE_VERSION_MAJOR);
    assert_eq!(minor, INTERFACE_VERSION_MINOR);
    assert_eq!(patch, INTERFACE_VERSION_PATCH);
}

// ============================================================================
// Compile-Time Interface Check Tests
// ============================================================================

#[test]
fn test_compile_time_interface_check() {
    // This test verifies the compile-time macro works
    crate::assert_interface_version!(1, 0, 0);
}

// ============================================================================
// ABI Compatibility Tests
// ============================================================================

/// Test that function signatures remain stable
/// 
/// This test documents the expected function signatures for the interface.
/// Any changes to these signatures would be breaking changes.
#[test]
fn test_bounty_escrow_signature_stability() {
    // Document expected function signatures
    // These are the stable ABI signatures that must not change
    
    // init: (Env, Address, Address) -> Result<(), CommonError>
    // lock_funds: (Env, u64, i128, Address, Option<u64>) -> Result<(), CommonError>
    // release_funds: (Env, u64, Address) -> Result<(), CommonError>
    // refund: (Env, u64) -> Result<(), CommonError>
    // get_balance: (Env, u64) -> i128
    // get_status: (Env, u64) -> Option<EscrowStatus>
    
    // This test serves as documentation - the actual signature
    // compatibility is verified at compile time by the trait system
    assert!(true, "Signature stability documented");
}

/// Test that error codes remain stable
#[test]
fn test_error_code_stability() {
    // Error codes must remain stable for ABI compatibility
    // New error codes can be added, but existing codes must not change
    
    assert_eq!(CommonError::NotInitialized as u32, 100);
    assert_eq!(CommonError::AlreadyInitialized as u32, 101);
    assert_eq!(CommonError::Unauthorized as u32, 102);
    assert_eq!(CommonError::InvalidAmount as u32, 103);
    assert_eq!(CommonError::InsufficientBalance as u32, 104);
    assert_eq!(CommonError::Paused as u32, 105);
    assert_eq!(CommonError::NotFound as u32, 106);
    
    // Document: Error codes 100-199 are reserved for common errors
    // Contract-specific errors should use codes 200+
}

// ============================================================================
// Version Compatibility Tests
// ============================================================================

/// Test interface version compatibility check
#[test]
fn test_version_compatibility() {
    let env = create_test_env();
    
    // A contract implementing Versioned should report its version
    let version = MockVersioned::get_version(env.clone());
    
    // Version should be positive
    assert!(version > 0);
    
    // Interface version should match what we expect
    let (major, minor, patch) = MockVersioned::get_interface_version(env);
    
    // Major version must match for compatibility
    assert_eq!(major, INTERFACE_VERSION_MAJOR);
    
    // Minor version should be <= interface minor for backward compatibility
    assert!(minor <= INTERFACE_VERSION_MINOR);
    
    // Patch version can be anything
    let _ = patch;
}

// ============================================================================
// Cross-Contract Interaction Tests
// ============================================================================

/// Test that contracts can interact through trait objects
#[test]
fn test_trait_object_interaction() {
    // This test verifies that we can use trait objects for cross-contract calls
    fn call_through_trait<T: BountyEscrowTrait>(env: Env, bounty_id: u64) -> i128 {
        T::get_balance(env, bounty_id)
    }
    
    let env = create_test_env();
    let balance = call_through_trait::<MockBountyEscrow>(env, 1);
    assert_eq!(balance, 0);
}

/// Test that multiple implementations can coexist
#[test]
fn test_multiple_implementations() {
    // Verify that different implementations of the same trait can coexist
    struct EscrowV1;
    struct EscrowV2;
    
    impl Versioned for EscrowV1 {
        fn get_version(_env: Env) -> u32 { 1 }
    }
    
    impl Versioned for EscrowV2 {
        fn get_version(_env: Env) -> u32 { 2 }
    }
    
    let env = create_test_env();
    
    let v1 = EscrowV1::get_version(env.clone());
    let v2 = EscrowV2::get_version(env);
    
    assert_eq!(v1, 1);
    assert_eq!(v2, 2);
}

// ============================================================================
// Breaking Change Detection Tests
// ============================================================================

/// These tests document what would constitute a breaking change
#[test]
fn test_breaking_change_documentation() {
    // The following changes would be BREAKING and require a major version bump:
    //
    // 1. Removing a function from a trait
    // 2. Changing a function's return type
    // 3. Changing a function's parameter types
    // 4. Changing a function's parameter order
    // 5. Adding a required parameter (not Option)
    // 6. Changing error code values
    // 7. Removing an error code variant
    //
    // The following changes are NON-BREAKING and only require a minor version bump:
    //
    // 1. Adding a new function to a trait
    // 2. Adding a new error code variant
    // 3. Adding a new status variant
    // 4. Adding an optional parameter
    
    assert!(true, "Breaking change criteria documented");
}
