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

use soroban_sdk::{Address, Env, String, Vec};

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
pub trait BountyEscrowTrait {
    /// Initialize the escrow contract
    fn init(env: Env, admin: Address, token: Address) -> Result<(), CommonError>;

    /// Lock funds for a specific bounty
    fn lock_funds(
        env: Env,
        bounty_id: u64,
        amount: i128,
        depositor: Address,
        deadline: Option<u64>,
    ) -> Result<(), CommonError>;

    /// Release funds to a contributor
    fn release_funds(
        env: Env,
        bounty_id: u64,
        contributor: Address,
    ) -> Result<(), CommonError>;

    /// Refund funds to the original depositor
    fn refund(env: Env, bounty_id: u64) -> Result<(), CommonError>;

    /// Get the current balance for a bounty
    fn get_balance(env: Env, bounty_id: u64) -> i128;

    /// Get the status of a bounty escrow
    fn get_status(env: Env, bounty_id: u64) -> Option<EscrowStatus>;
}

/// Trait defining the standard escrow interface for program escrows
pub trait ProgramEscrowTrait {
    /// Initialize a program escrow
    fn init_program(
        env: Env,
        program_id: String,
        admin: Address,
        token: Address,
    ) -> Result<(), CommonError>;

    /// Lock funds for a program
    fn lock_program_funds(env: Env, amount: i128) -> Result<(), CommonError>;

    /// Perform a batch payout to multiple recipients
    fn batch_payout(
        env: Env,
        recipients: Vec<Address>,
        amounts: Vec<i128>,
    ) -> Result<(), CommonError>;

    /// Perform a single payout
    fn single_payout(env: Env, recipient: Address, amount: i128) -> Result<(), CommonError>;

    /// Get remaining balance for the program
    fn get_remaining_balance(env: Env) -> i128;

    /// Check if a program exists
    fn program_exists(env: Env, program_id: String) -> bool;
}

/// Trait for pause functionality
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
    fn transfer_admin(env: Env, new_admin: Address) -> Result<(), CommonError>;
}

/// Trait for version information
pub trait Versioned {
    /// Get the contract version
    fn get_version(env: Env) -> u32;

    /// Get the interface version this contract implements
    fn get_interface_version(_env: Env) -> (u32, u32, u32) {
        (INTERFACE_VERSION_MAJOR, INTERFACE_VERSION_MINOR, INTERFACE_VERSION_PATCH)
    }
}

/// Compile-time interface version check
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
