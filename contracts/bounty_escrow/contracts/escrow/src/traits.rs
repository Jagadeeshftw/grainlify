//! Standardized interface traits for Grainlify escrow contracts.
//!
//! ## Spec Alignment
//!
//! These traits serve as the project's internal contract interface standard,
//! analogous to Stellar Ecosystem Proposals (SEPs) for token and escrow
//! patterns. Any contract that manages locked funds, fees, or upgrades should
//! implement the relevant trait so that wallets, indexers, and the view-facade
//! can reason about any Grainlify contract uniformly.
//!
//! | Trait            | Purpose                                | Implemented by                       |
//! |------------------|----------------------------------------|--------------------------------------|
//! | EscrowInterface  | Core lock / release / refund lifecycle | BountyEscrowContract, EscrowContract  |
//! | UpgradeInterface | Version tracking & WASM upgrades       | BountyEscrowContract, GrainlifyContract |
//! | PauseInterface   | Granular per-operation pausing         | BountyEscrowContract                  |
//! | FeeInterface     | Fee config read/write                  | BountyEscrowContract                  |
//!
//! ## Adding a New Contract
//!
//! 1. Implement the required trait(s) for your contract struct.
//! 2. Add a row to the table above.
//! 3. Register the contract address in the view-facade via `ViewFacade::register`.

use soroban_sdk::{symbol_short, Address, Env, String, Symbol};

// ============================================================================
// EscrowInterface
// ============================================================================

/// Core lifecycle interface for all escrow contracts.
///
/// Captures the minimal surface that every escrow variant must expose so
/// cross-contract callers and the view-facade can treat all escrow types
/// interchangeably.
///
/// ### Alignment with Stellar patterns
/// The function signatures follow the lock-release-refund pattern common
/// across Stellar DeFi protocols.  Deadline-gated refunds and admin-only
/// release map to the trust model described in SEP-0007 (transaction
/// signing) and the broader Stellar escrow conventions.
#[allow(dead_code)]
pub trait EscrowInterface {
    /// Lock `amount` tokens from `depositor` for `bounty_id` until `deadline`.
    fn lock_funds(
        env: &Env,
        depositor: Address,
        bounty_id: u64,
        amount: i128,
        deadline: u64,
    ) -> Result<(), crate::Error>;

    /// Release the full locked amount to `contributor`. Admin-only.
    fn release_funds(env: &Env, bounty_id: u64, contributor: Address) -> Result<(), crate::Error>;

    /// Refund the remaining amount to the original depositor.
    /// Only callable after the escrow deadline has passed (or with admin approval).
    fn refund(env: &Env, bounty_id: u64) -> Result<(), crate::Error>;

    /// Return the current [`crate::Escrow`] record for `bounty_id`.
    fn get_escrow_info(env: &Env, bounty_id: u64) -> Result<crate::Escrow, crate::Error>;

    /// Return the contract's current token balance.
    fn get_balance(env: &Env) -> Result<i128, crate::Error>;
}

// ============================================================================
// UpgradeInterface
// ============================================================================

/// Version tracking interface for upgradeable contracts.
///
/// Every contract that may be upgraded via
/// `env.deployer().update_current_contract_wasm` should implement this
/// trait to allow tooling to gate on version numbers and surface upgrade
/// history to operators.
#[allow(dead_code)]
pub trait UpgradeInterface {
    /// Return the numeric version stored in instance storage.
    /// Defaults to `0` when the key has not yet been written.
    fn get_version(env: &Env) -> u32;

    /// Overwrite the stored version number. Admin-only in all implementations.
    ///
    /// Returns `Err(String)` with a human-readable message on failure.
    fn set_version(env: &Env, new_version: u32) -> Result<(), String>;
}

// ============================================================================
// PauseInterface
// ============================================================================

/// Granular per-operation pause interface.
///
/// Contracts that need emergency circuit-breakers without taking the whole
/// contract offline implement this trait.  Each boolean toggles a specific
/// operation class (`lock`, `release`, `refund`) independently.
///
/// ### Design rationale
/// Fine-grained pausing lets operators halt only the affected operation class
/// (e.g. stop new `lock_funds` calls during an audit) while keeping existing
/// releases and refunds live.  This is important for maintaining user trust
/// and for regulatory compliance in jurisdictions that may require a
/// controlled wind-down rather than a hard stop.
#[allow(dead_code)]
pub trait PauseInterface {
    /// Pause or unpause individual operation classes. `None` leaves the
    /// current state unchanged for that class.
    ///
    /// * `lock`    — controls `lock_funds` / `batch_lock_funds`
    /// * `release` — controls `release_funds` / `batch_release_funds` / `claim`
    /// * `refund`  — controls `refund` / `refund_with_capability`
    /// * `reason`  — optional human-readable explanation stored on-chain
    fn set_paused(
        env: &Env,
        lock: Option<bool>,
        release: Option<bool>,
        refund: Option<bool>,
        reason: Option<soroban_sdk::String>,
    ) -> Result<(), crate::Error>;

    /// Return the current [`crate::PauseFlags`] without mutating state.
    fn get_pause_flags(env: &Env) -> crate::PauseFlags;

    /// Return `true` when the given `operation` symbol is paused.
    ///
    /// Canonical operation symbols:
    /// * `symbol_short!("lock")`
    /// * `symbol_short!("release")`
    /// * `symbol_short!("refund")`
    fn is_operation_paused(env: &Env, operation: Symbol) -> bool;
}

// ============================================================================
// FeeInterface
// ============================================================================

/// Fee configuration interface.
///
/// Standardizes how fee rates and recipients are read and updated.  A
/// fee-aware wallet or dashboard can call `get_fee_config` on any contract
/// that implements this trait without needing to know its concrete type.
///
/// Fee rates are expressed as basis-point-style fixed-point integers where
/// `10_000` == 100 %.  See [`crate::token_math::MAX_FEE_RATE`] for the
/// enforced ceiling.
#[allow(dead_code)]
pub trait FeeInterface {
    /// Update one or more fee parameters.  Passing `None` for a field
    /// leaves it unchanged.  Admin-only.
    fn update_fee_config(
        env: &Env,
        lock_fee_rate: Option<i128>,
        release_fee_rate: Option<i128>,
        fee_recipient: Option<Address>,
        fee_enabled: Option<bool>,
    ) -> Result<(), crate::Error>;

    /// Return the current [`crate::FeeConfig`] without mutating state.
    fn get_fee_config(env: &Env) -> crate::FeeConfig;
}

// ============================================================================
// Blanket helpers (not traits — just free functions used by implementations)
// ============================================================================

/// Canonical operation symbol for `lock_funds`.
#[allow(dead_code)]
pub fn op_lock() -> Symbol {
    symbol_short!("lock")
}

/// Canonical operation symbol for `release_funds` / `claim`.
#[allow(dead_code)]
pub fn op_release() -> Symbol {
    symbol_short!("release")
}

/// Canonical operation symbol for `refund`.
#[allow(dead_code)]
pub fn op_refund() -> Symbol {
    symbol_short!("refund")
}
