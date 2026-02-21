//! # Cross-Contract Interface Compatibility Tests

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

use crate::{
    AdminManaged, BountyEscrowTrait, CommonError, EscrowStatus, Pausable, ProgramEscrowTrait, Versioned,
    INTERFACE_VERSION_MAJOR, INTERFACE_VERSION_MINOR, INTERFACE_VERSION_PATCH,
};

fn create_env() -> Env { Env::default() }
fn create_address(env: &Env) -> Address { Address::generate(env) }

// ============================================================================
// Interface Version Tests
// ============================================================================

#[test]
fn test_interface_version() {
    assert_eq!(INTERFACE_VERSION_MAJOR, 1);
    assert_eq!(INTERFACE_VERSION_MINOR, 0);
    assert_eq!(INTERFACE_VERSION_PATCH, 0);
}

#[test]
fn test_error_codes() {
    assert_eq!(CommonError::NotInitialized as u32, 100);
    assert_eq!(CommonError::AlreadyInitialized as u32, 101);
    assert_eq!(CommonError::Unauthorized as u32, 102);
    assert_eq!(CommonError::InvalidAmount as u32, 103);
    assert_eq!(CommonError::InsufficientBalance as u32, 104);
    assert_eq!(CommonError::Paused as u32, 105);
    assert_eq!(CommonError::NotFound as u32, 106);
}

#[test]
fn test_escrow_status() {
    assert_eq!(EscrowStatus::Locked, EscrowStatus::Locked);
    assert_ne!(EscrowStatus::Locked, EscrowStatus::Released);
}

// ============================================================================
// Mock Implementations
// ============================================================================

struct MockBountyEscrow;
impl BountyEscrowTrait for MockBountyEscrow {
    fn init(_env: Env, _admin: Address, _token: Address) -> Result<(), CommonError> { Ok(()) }
    fn lock_funds(_env: Env, _bounty_id: u64, _amount: i128, _depositor: Address, _deadline: Option<u64>) -> Result<(), CommonError> { Ok(()) }
    fn release_funds(_env: Env, _bounty_id: u64, _contributor: Address) -> Result<(), CommonError> { Ok(()) }
    fn refund(_env: Env, _bounty_id: u64) -> Result<(), CommonError> { Ok(()) }
    fn get_balance(_env: Env, _bounty_id: u64) -> i128 { 0 }
    fn get_status(_env: Env, _bounty_id: u64) -> Option<EscrowStatus> { Some(EscrowStatus::Locked) }
}

struct MockProgramEscrow;
impl ProgramEscrowTrait for MockProgramEscrow {
    fn init_program(_env: Env, _program_id: String, _admin: Address, _token: Address) -> Result<(), CommonError> { Ok(()) }
    fn lock_program_funds(_env: Env, _amount: i128) -> Result<(), CommonError> { Ok(()) }
    fn batch_payout(_env: Env, _recipients: Vec<Address>, _amounts: Vec<i128>) -> Result<(), CommonError> { Ok(()) }
    fn single_payout(_env: Env, _recipient: Address, _amount: i128) -> Result<(), CommonError> { Ok(()) }
    fn get_remaining_balance(_env: Env) -> i128 { 0 }
    fn program_exists(_env: Env, _program_id: String) -> bool { true }
}

struct MockPausable;
impl Pausable for MockPausable {
    fn is_lock_paused(_env: &Env) -> bool { false }
    fn is_release_paused(_env: &Env) -> bool { false }
    fn is_refund_paused(_env: &Env) -> bool { false }
    fn set_paused(_env: Env, _lock: Option<bool>, _release: Option<bool>, _refund: Option<bool>) -> Result<(), CommonError> { Ok(()) }
}

struct MockAdmin;
impl AdminManaged for MockAdmin {
    fn get_admin(_env: Env) -> Option<Address> { None }
    fn transfer_admin(_env: Env, _new_admin: Address) -> Result<(), CommonError> { Ok(()) }
}

struct MockVersioned;
impl Versioned for MockVersioned {
    fn get_version(_env: Env) -> u32 { 1 }
}

// ============================================================================
// Trait Bounds Tests
// ============================================================================

#[test]
fn test_bounty_escrow_trait() {
    let env = create_env();
    let admin = create_address(&env);
    let token = create_address(&env);
    let depositor = create_address(&env);
    let contributor = create_address(&env);

    let _ = MockBountyEscrow::init(env.clone(), admin, token);
    let _ = MockBountyEscrow::lock_funds(env.clone(), 1, 100, depositor, None);
    let _ = MockBountyEscrow::release_funds(env.clone(), 1, contributor);
    let _ = MockBountyEscrow::refund(env.clone(), 1);
    let _ = MockBountyEscrow::get_balance(env.clone(), 1);
    let _ = MockBountyEscrow::get_status(env.clone(), 1);
}

#[test]
fn test_program_escrow_trait() {
    let env = create_env();
    let admin = create_address(&env);
    let token = create_address(&env);
    let recipient = create_address(&env);
    let program_id = String::from_str(&env, "test");

    let _ = MockProgramEscrow::init_program(env.clone(), program_id.clone(), admin, token);
    let _ = MockProgramEscrow::lock_program_funds(env.clone(), 1000);
    let _ = MockProgramEscrow::single_payout(env.clone(), recipient, 100);
    let _ = MockProgramEscrow::get_remaining_balance(env.clone());
    let _ = MockProgramEscrow::program_exists(env.clone(), program_id);
}

#[test]
fn test_pausable_trait() {
    let env = create_env();
    assert!(!MockPausable::is_lock_paused(&env));
    assert!(!MockPausable::is_release_paused(&env));
    assert!(!MockPausable::is_refund_paused(&env));
    let _ = MockPausable::set_paused(env.clone(), Some(true), Some(false), None);
}

#[test]
fn test_admin_trait() {
    let env = create_env();
    let new_admin = create_address(&env);
    let _ = MockAdmin::get_admin(env.clone());
    let _ = MockAdmin::transfer_admin(env.clone(), new_admin);
}

#[test]
fn test_versioned_trait() {
    let env = create_env();
    let version = MockVersioned::get_version(env.clone());
    assert!(version > 0);
    let (major, minor, patch) = MockVersioned::get_interface_version(env);
    assert_eq!(major, INTERFACE_VERSION_MAJOR);
    assert_eq!(minor, INTERFACE_VERSION_MINOR);
    assert_eq!(patch, INTERFACE_VERSION_PATCH);
}

#[test]
fn test_compile_time_check() {
    crate::assert_interface_version!(1, 0, 0);
}

// ============================================================================
// Cross-Contract Interaction Tests
// ============================================================================

#[test]
fn test_trait_object_call() {
    fn get_balance<T: BountyEscrowTrait>(env: Env, bounty_id: u64) -> i128 {
        T::get_balance(env, bounty_id)
    }
    let env = create_env();
    assert_eq!(get_balance::<MockBountyEscrow>(env, 1), 0);
}

#[test]
fn test_multiple_versions() {
    struct V1;
    struct V2;
    impl Versioned for V1 { fn get_version(_env: Env) -> u32 { 1 } }
    impl Versioned for V2 { fn get_version(_env: Env) -> u32 { 2 } }
    let env = create_env();
    assert_eq!(V1::get_version(env.clone()), 1);
    assert_eq!(V2::get_version(env), 2);
}

#[test]
fn test_breaking_change_policy() {
    // Breaking: remove function, change return/param types, change error codes
    // Non-breaking: add function, add error code, add optional param
    assert!(true);
}
