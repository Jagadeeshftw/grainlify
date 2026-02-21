//! # Shared Interfaces for Grainlify Contracts
//!
//! This module defines standard traits/interfaces that all Grainlify escrow contracts
//! must implement. This ensures ABI compatibility and allows for cross-contract
//! interactions through trait objects.
//!
//! ## Overview
//!
//! The shared interfaces provide:
//! - Standard escrow operations (lock, release, refund)
//! - Balance queries
//! - Pause functionality
//! - Admin management
//!
//! ## Versioning
//!
//! Interface versions follow semantic versioning:
//! - MAJOR: Breaking changes to function signatures
//! - MINOR: New functions added (backward compatible)
//! - PATCH: Documentation or internal changes
//!
//! Current interface version: 1.0.0

#![no_std]

use soroban_sdk::{Address, Env, String};

/// Interface version information
pub const INTERFACE_VERSION_MAJOR: u32 = 1;
pub const INTERFACE_VERSION_MINOR: u32 = 0;
pub const INTERFACE_VERSION_PATCH: u32 = 0;

/// Common error codes shared across all escrow contracts
#[repr(u32)]
pub enum CommonError {
    /// Contract not initialized
    NotInitialized = 100,
    /// Already initialized
    AlreadyInitialized = 101,
    /// Unauthorized access
    Unauthorized = 102,
    /// Invalid amount
    InvalidAmount = 103,
    /// Insufficient balance
    InsufficientBalance = 104,
    /// Operation paused
    Paused = 105,
    /// Not found
    NotFound = 106,
}

/// Escrow status shared across contracts
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EscrowStatus {
    /// Funds are locked and waiting for release
    Locked,
    /// Funds have been released to recipient
    Released,
    /// Funds have been refunded to depositor
    Refunded,
    /// Partial release - some funds released, some remaining
    PartiallyReleased,
}

/// Trait defining the standard escrow interface for bounty escrows
/// 
/// This trait must be implemented by all bounty escrow contracts to ensure
/// compatibility with the Grainlify platform.
/// 
/// # Example Implementation
/// 
/// ```rust,ignore
/// use shared_interfaces::BountyEscrowTrait;
/// use soroban_sdk::{Address, Env, String};
/// 
/// struct MyBountyEscrow;
/// 
/// impl BountyEscrowTrait for MyBountyEscrow {
///     // Implement required methods...
/// }
/// ```
pub trait BountyEscrowTrait {
    /// Initialize the escrow contract
    /// 
    /// # Arguments
    /// * `env` - Soroban environment
    /// * `admin` - Admin address with privileged access
    /// * `token` - Token contract address for escrowed funds
    /// 
    /// # Errors
    /// * `AlreadyInitialized` if called more than once
    fn init(env: Env, admin: Address, token: Address) -> Result<(), CommonError>;

    /// Lock funds for a specific bounty
    /// 
    /// # Arguments
    /// * `env` - Soroban environment
    /// * `bounty_id` - Unique identifier for the bounty
    /// * `amount` - Amount to lock in escrow
    /// * `depositor` - Address locking the funds
    /// * `deadline` - Optional deadline for refund eligibility
    /// 
    /// # Errors
    /// * `NotInitialized` if contract not initialized
    /// * `InvalidAmount` if amount <= 0
    /// * `Paused` if lock operations are paused
    fn lock_funds(
        env: Env,
        bounty_id: u64,
        amount: i128,
        depositor: Address,
        deadline: Option<u64>,
    ) -> Result<(), CommonError>;

    /// Release funds to a contributor
    /// 
    /// # Arguments
    /// * `env` - Soroban environment
    /// * `bounty_id` - Bounty identifier
    /// * `contributor` - Address to receive the funds
    /// 
    /// # Errors
    /// * `NotInitialized` if contract not initialized
    /// * `NotFound` if bounty doesn't exist
    /// * `Unauthorized` if caller is not authorized
    /// * `Paused` if release operations are paused
    fn release_funds(
        env: Env,
        bounty_id: u64,
        contributor: Address,
    ) -> Result<(), CommonError>;

    /// Refund funds to the original depositor
    /// 
    /// # Arguments
    /// * `env` - Soroban environment
    /// * `bounty_id` - Bounty identifier
    /// 
    /// # Errors
    /// * `NotInitialized` if contract not initialized
    /// * `NotFound` if bounty doesn't exist
    /// * `Unauthorized` if caller is not depositor or admin
    /// * `Paused` if refund operations are paused
    fn refund(env: Env, bounty_id: u64) -> Result<(), CommonError>;

    /// Get the current balance for a bounty
    /// 
    /// # Arguments
    /// * `env` - Soroban environment
    /// * `bounty_id` - Bounty identifier
    /// 
    /// # Returns
    /// The remaining balance for the bounty
    fn get_balance(env: Env, bounty_id: u64) -> i128;

    /// Get the status of a bounty escrow
    /// 
    /// # Arguments
    /// * `env` - Soroban environment
    /// * `bounty_id` - Bounty identifier
    /// 
    /// # Returns
    /// The current status of the escrow
    fn get_status(env: Env, bounty_id: u64) -> Option<EscrowStatus>;
}

/// Trait defining the standard escrow interface for program escrows
/// 
/// This trait must be implemented by all program escrow contracts to ensure
/// compatibility with the Grainlify platform.
pub trait ProgramEscrowTrait {
    /// Initialize a program escrow
    /// 
    /// # Arguments
    /// * `env` - Soroban environment
    /// * `program_id` - Unique identifier for the program
    /// * `admin` - Admin address with privileged access
    /// * `token` - Token contract address for escrowed funds
    /// 
    /// # Errors
    /// * `AlreadyInitialized` if program already exists
    fn init_program(
        env: Env,
        program_id: String,
        admin: Address,
        token: Address,
    ) -> Result<(), CommonError>;

    /// Lock funds for a program
    /// 
    /// # Arguments
    /// * `env` - Soroban environment
    /// * `amount` - Amount to lock
    /// 
    /// # Errors
    /// * `NotInitialized` if program not initialized
    /// * `InvalidAmount` if amount <= 0
    fn lock_program_funds(env: Env, amount: i128) -> Result<(), CommonError>;

    /// Perform a batch payout to multiple recipients
    /// 
    /// # Arguments
    /// * `env` - Soroban environment
    /// * `recipients` - List of recipient addresses
    /// * `amounts` - List of amounts corresponding to each recipient
    /// 
    /// # Errors
    /// * `NotInitialized` if program not initialized
    /// * `InsufficientBalance` if total exceeds available balance
    fn batch_payout(
        env: Env,
        recipients: soroban_sdk::Vec<Address>,
        amounts: soroban_sdk::Vec<i128>,
    ) -> Result<(), CommonError>;

    /// Perform a single payout
    /// 
    /// # Arguments
    /// * `env` - Soroban environment
    /// * `recipient` - Recipient address
    /// * `amount` - Amount to payout
    /// 
    /// # Errors
    /// * `NotInitialized` if program not initialized
    /// * `InsufficientBalance` if amount exceeds available balance
    fn single_payout(env: Env, recipient: Address, amount: i128) -> Result<(), CommonError>;

    /// Get remaining balance for the program
    /// 
    /// # Arguments
    /// * `env` - Soroban environment
    /// 
    /// # Returns
    /// The remaining balance
    fn get_remaining_balance(env: Env) -> i128;

    /// Check if a program exists
    /// 
    /// # Arguments
    /// * `env` - Soroban environment
    /// * `program_id` - Program identifier
    /// 
    /// # Returns
    /// True if program exists, false otherwise
    fn program_exists(env: Env, program_id: String) -> bool;
}

/// Trait for pause functionality
/// 
/// Provides granular pause control for different operations
pub trait Pausable {
    /// Check if lock operations are paused
    fn is_lock_paused(env: &Env) -> bool;

    /// Check if release operations are paused
    fn is_release_paused(env: &Env) -> bool;

    /// Check if refund operations are paused
    fn is_refund_paused(env: &Env) -> bool;

    /// Set pause state for operations (admin only)
    fn set_paused(
        env: Env,
        lock: Option<bool>,
        release: Option<bool>,
        refund: Option<bool>,
    ) -> Result<(), CommonError>;
}

/// Trait for admin management
pub trait AdminManaged {
    /// Get the current admin address
    fn get_admin(env: Env) -> Option<Address>;

    /// Transfer admin to a new address
    /// 
    /// # Arguments
    /// * `env` - Soroban environment
    /// * `new_admin` - New admin address
    /// 
    /// # Errors
    /// * `Unauthorized` if caller is not current admin
    fn transfer_admin(env: Env, new_admin: Address) -> Result<(), CommonError>;
}

/// Trait for version information
pub trait Versioned {
    /// Get the contract version
    fn get_version(env: Env) -> u32;

    /// Get the interface version this contract implements
    fn get_interface_version(env: Env) -> (u32, u32, u32) {
        (INTERFACE_VERSION_MAJOR, INTERFACE_VERSION_MINOR, INTERFACE_VERSION_PATCH)
    }
}

/// Compile-time interface version check
/// 
/// This macro ensures that a contract implements the correct interface version.
/// Use this in your contract's test module to verify compatibility.
#[macro_export]
macro_rules! assert_interface_version {
    ($major:expr, $minor:expr, $patch:expr) => {
        const _: () = assert!(
            $major == $crate::INTERFACE_VERSION_MAJOR,
            "Interface major version mismatch"
        );
        const _: () = assert!(
            $minor <= $crate::INTERFACE_VERSION_MINOR,
            "Interface minor version too new"
        );
    };
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_interface_version() {
        assert_eq!(INTERFACE_VERSION_MAJOR, 1);
        assert_eq!(INTERFACE_VERSION_MINOR, 0);
        assert_eq!(INTERFACE_VERSION_PATCH, 0);
    }

    #[test]
    fn test_version_tuple() {
        let (major, minor, patch) = (INTERFACE_VERSION_MAJOR, INTERFACE_VERSION_MINOR, INTERFACE_VERSION_PATCH);
        assert_eq!(major, 1);
        assert_eq!(minor, 0);
        assert_eq!(patch, 0);
    }
}

#[cfg(test)]
mod interface_tests;
