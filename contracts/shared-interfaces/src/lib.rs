//! # Shared Interfaces for Grainlify Contracts
//!
//! Standard traits/interfaces for cross-contract compatibility.

#![no_std]

use soroban_sdk::{Address, Env, String, Vec};

/// Interface version: 1.0.0
pub const INTERFACE_VERSION_MAJOR: u32 = 1;
pub const INTERFACE_VERSION_MINOR: u32 = 0;
pub const INTERFACE_VERSION_PATCH: u32 = 0;

/// Common error codes (100-199)
#[repr(u32)]
pub enum CommonError {
    NotInitialized = 100,
    AlreadyInitialized = 101,
    Unauthorized = 102,
    InvalidAmount = 103,
    InsufficientBalance = 104,
    Paused = 105,
    NotFound = 106,
}

/// Escrow status
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EscrowStatus {
    Locked,
    Released,
    Refunded,
    PartiallyReleased,
}

/// Bounty escrow interface
pub trait BountyEscrowTrait {
    fn init(env: Env, admin: Address, token: Address) -> Result<(), CommonError>;
    fn lock_funds(env: Env, bounty_id: u64, amount: i128, depositor: Address, deadline: Option<u64>) -> Result<(), CommonError>;
    fn release_funds(env: Env, bounty_id: u64, contributor: Address) -> Result<(), CommonError>;
    fn refund(env: Env, bounty_id: u64) -> Result<(), CommonError>;
    fn get_balance(env: Env, bounty_id: u64) -> i128;
    fn get_status(env: Env, bounty_id: u64) -> Option<EscrowStatus>;
}

/// Program escrow interface
pub trait ProgramEscrowTrait {
    fn init_program(env: Env, program_id: String, admin: Address, token: Address) -> Result<(), CommonError>;
    fn lock_program_funds(env: Env, amount: i128) -> Result<(), CommonError>;
    fn batch_payout(env: Env, recipients: Vec<Address>, amounts: Vec<i128>) -> Result<(), CommonError>;
    fn single_payout(env: Env, recipient: Address, amount: i128) -> Result<(), CommonError>;
    fn get_remaining_balance(env: Env) -> i128;
    fn program_exists(env: Env, program_id: String) -> bool;
}

/// Pause functionality
pub trait Pausable {
    fn is_lock_paused(env: &Env) -> bool;
    fn is_release_paused(env: &Env) -> bool;
    fn is_refund_paused(env: &Env) -> bool;
    fn set_paused(env: Env, lock: Option<bool>, release: Option<bool>, refund: Option<bool>) -> Result<(), CommonError>;
}

/// Admin management
pub trait AdminManaged {
    fn get_admin(env: Env) -> Option<Address>;
    fn transfer_admin(env: Env, new_admin: Address) -> Result<(), CommonError>;
}

/// Version tracking
pub trait Versioned {
    fn get_version(env: Env) -> u32;
    fn get_interface_version(_env: Env) -> (u32, u32, u32) {
        (INTERFACE_VERSION_MAJOR, INTERFACE_VERSION_MINOR, INTERFACE_VERSION_PATCH)
    }
}

/// Compile-time version check
#[macro_export]
macro_rules! assert_interface_version {
    ($major:expr, $minor:expr, $patch:expr) => {
        const _: () = assert!($major == $crate::INTERFACE_VERSION_MAJOR, "Major version mismatch");
        const _: () = assert!($minor <= $crate::INTERFACE_VERSION_MINOR, "Minor version too new");
    };
}

#[cfg(test)]
mod interface_tests;
