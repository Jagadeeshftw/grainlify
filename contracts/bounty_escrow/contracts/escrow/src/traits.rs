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
//! ## Trait Mapping Table
//!
//! | Trait                  | Purpose                              | Implemented by                |
//! |-----------------------------|--------------------------------------------------|---------------------------------------------|
//! | EscrowInterface    | Core lock / release / refund lifecycle       | BountyEscrowContract          |
//! | EscrowInterface    | Batch operations (lock, release)              | BountyEscrowContract          |
//! | EscrowInterface    | Partial release                             | BountyEscrowContract          |
//! | UpgradeInterface   | Version tracking & WASK upgrades             | BountyEscrowContract          |
//! | PauseInterface     | Granular per-operation pausing               | BountyEscrowContract          |
//! | FeeInterface       | Fee config read/write                        | BountyEscrowContract          |
//! | NotificationPreferencesInterface | Per-account notification preferences | BountyEscrowContract         |
//!
//! ## Trait-to-Lib Entry Point Mapping
//!
//! | Trait Method          | lib.rs Entry Point       | Notes                                            |
//!|-------------------------|------------------------------|------------------------------------------------------------------------|
//! | lock_funds            | lock_funds           | `non_transferable_rewards` always None  |
//! | release_funds         | release_funds         | Admin-only full release      |
//! | partial_release       | partial_release       | Admin-only partial release   |
//! | batch_lock_funds     | batch_lock_funds     | Batch variant of lock_funds  |
//! | batch_release_funds   | batch_release_funds   | Batch variant of release_funds |
//! | refund               | refund               | Deadline-gated refund        |
//! | get_escrow_info       | get_escrow_info       | Returns escrow state         |
//! | get_balance          | get_balance           | Returns contract token balance  |
//! | set_notification_preferences | set_notification_preferences | Per-account, authorized write   |
//! | get_notification_preferences | get_notification_preferences | Public read                    |
//!
//! ## Adding a New Contract
//!
//! 1. Implement the required trait(s) for your contract struct.
//! 2. Add a row to the table above.
//! 3. Register the contract address in the view-facade via `ViewFacade::register`.

use soroban_sdk::{symbol_short, Address, Env, Symbol, Vec};

// ==========================================================================================================================================
// EscrowInterface
// ==========================================================================================================================================

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
    /// Lock `}amount` tokens from `depositor` for `bounty_id` until `deadline`.
    /// Note: `non_transferable_rewards` is always set to `None` in this trait;
/// use the concrete contract to set it.
    fn lock_funds(
        env: &Env,
        depositor: Address,
        bounty_id: u64,
        amount: i128,
        deadline: u64,
    ) -> Result<(), crate::Error>;

    /// Release the full locked amount to `contributor`. Admin-only.
    fn release_funds(env: &Env, bounty_id: u64, contributor: Address) -> Result<(), crate::Error>;

    /// Release a partial `payout_amount` to `contributor`. Admin-only.
    fn partial_release(
        env: &Env,
        bounty_id: u64,
        contributor: Address,
        payout_amount: i128,
    ) -> Result<(), crate::Error>;

    /// Lock multiple escrows in a batch. Returns count of successfully locked items.
    fn batch_lock_funds(env: &Env, items: Vec<crate::LockFundsItem>) -> Result<u32, crate::Error>;

    /// Release multiple escrows in a batch. Returns count of successfully released items.
    fn batch_release_funds(
        env: &Env,
        items: Vec<crate::ReleaseFundsItem>,
    ) -> Result<u32, crate::Error>;

    /// Refund the remaining amount to the original depositor.
    /// Only callable after the escrow deadline has passed (or with admin approval).
    fn refund(env: &Env, bounty_id: u64) -> Result<(), crate::Error>;

    /// Return the current [crate::Escrow] record for `bounty_id`.
    fn get_escrow_info(env: &Env, bounty_id: u64) -> Result<crate::Escrow, crate::Error>;

    /// Return the contract's current token balance.
    fn get_balance(env: &Env) -> Result<i128, crate::Error>;
}

// ==========================================================================================================================================
// UpgradeInterface
// ==========================================================================================================================================

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
    fn set_version(env: &Env, new_version: u32) -> Result<(), crate::Error>;
}

// ==========================================================================================================================================
// PauseInterface
// ==========================================================================================================================================

/// Granular per-operation pause interface.
///
/// Contracts that need emergency circuit-breakers without taking the whole
/// contract offline implement this trait.  Each boolean toggles a specific
/// operation class (`lock`, `release`, `refund`) independently.
///
/// ### Design rationale
/// Fine-grained pausing lets operators halt only the affected operation
/// class (e.g. stop new `lock_funds` calls during an audit) while keeping
/// existing releases and refunds live.  This is important for maintaining user trust
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

    /// Return the current [crate::PauseFlags] without mutating state.
    fn get_pause_flags(env: &Env) -> crate::PauseFlags;

    /// Return `true` when the given `operation` symbol is paused.
    ///
    /// Canonical operation symbols:
    /// * `symbol_short!("lock")`
    /// * `symbol_short!("release")`
    /// * `symbol_short!("refund")`
    fn is_operation_paused(env: &Env, operation: Symbol) -> bool;
}

// ==========================================================================================================================================
// FeeInterface
// ==========================================================================================================================================

/// Fee configuration interface.
///
/// Standardizes how fee rates and recipients are read and updated.  A
/// fee-aware wallet or dashboard can call `get_fee_config` on any contract
/// that implements this trait without needing to know its concrete type.
///
/// Fee rates are expressed as basis-point-style fixed-point integers where
/// `10_000` == 100 %.  See [crate::token_math::MAX_FEE_RATE] for the
/// enforced ceiling.
#[allow(dead_code)]
pub trait FeeInterface {
    /// Update one or more fee parameters.  Passing `None` for a field
    /// leaves it unchanged.  Admin-only.
    fn update_fee_config(
        env: &Env,
        lock_fee_rate: Option<i128>,
        release_fee_rate: Option<i128>,
        lock_fixed_fee: Option<i128>,
        release_fixed_fee: Option<i128>,
        fee_recipient: Option<Address>,
        fee_enabled: Option<bool>,
    ) -> Result<(), crate::Error>;

    /// Return the current [crate::FeeConfig] without mutating state.
    fn get_fee_config(env: &Env) -> crate::FeeConfig;
}

// ==========================================================================================================================================
// NotificationPreferencesInterface
// ==========================================================================================================================================

/// On-chain per-account notification preferences.
///
/// The struct is bounded to control storage costs:
/// * `email` — optional, UTf-8, max [MaxNotificationEmailLength] chars
/// * `webhook_url` — optional, UTf-8, max [MaxNotificationWebhookLength] chars
/// * boolean flags control which lifecycle events generate notifications.
#[soroban_sdk::contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationPreferences {
    pub email: Option<soroban_sdk::String>,
    pub webhook_url: Option<soroban_sdk::String>,
    pub notify_on_release: bool,
    pub notify_on_refund: bool,
    pub notify_on_deadline: bool,
}

/// Maximum length of notification email address (characters).
pub const MaxNotificationEmailLength: u32 = 256;

/// Maximum length of notification webhook URL (characters).
pub const MaxNotificationWebhookLength: u32 = 512;

/// Interface for per-account notification preference storage.
///
/// ### Design Decision
/// Notification preferences are stored **on-chain** because:
/// 1. They are small (one ledger entry per account).
/// 2. Users control their own data with an explicit `set` transaction.
/// 3. Indexers can read on-chain state directly without an off-chain DB.
//.
/// ### Storage
/// The contract stores a single `NotificationPreferences` value per account
/// under the key `DataKey::NotificationPreferences(account)`.
///
/// ### Authorization
/// * `set_notification_preferences` may be called by the account itself.
///   No other caller can modify an account's preferences.
/// * `get_notification_preferences` is public; it returns `Ok(None)` when
///   no preferences have been stored.
///
/// ### Events
/// `set_notification_preferences` emits
/// `NotificationPreferencesUpdated(account, old_preferences, new_preferences)`
/// to allow off-chain notification systems to react without polling.
///
/// ### Limits & Upgrades
/// * `email` and `webhook_url` are length-limited to avoid storage abuse.
/// * Fields may be added in future upgrades; unknown fields are ignored.
/// * Prefer using `Option` for new fields to maintain backward compatibility.
///
/// **Changelog:**
/// - Vault this interface implementation in the contract and remove the test todo.
#[allow(dead_code)]
pub trait NotificationPreferencesInterface {
    /// Set the notification preferences for `account`.
    ///
    /// #Arguments
    /// * `account` – the address that owns the preferences.
    /// * `preferences` – new values. All fields are caller-supplied; there
    ///   is no merge semantics — the stored value is replaced atomically.
    ///
    /// #Returns
///     Ok(account, old_preferences) containing the previously stored value.
///     Err(InvalidAuth) when the caller is not the account.
///     Err(InvalidArgument) if `notify_on_release` is false but the previous
    fn set_notification_preferences(
        env: &Env,
        caller: Address,
        account: Address,
        preferences: NotificationPreferences,
    ) -> Result<Option<NotificationPreferences>, crate::Error>;

    /// Return the stored preferences for `account`, or `None` if unset.
    /// No authorization required to read public notification preferences.
    fn get_notification_preferences(
        env: &Env,
        account: Address,
    ) -> Result<Option<NotificationPreferences>, crate::Error>;
}

// ==========================================================================================================================================
// Blanket helpers (not traits — just free functions used by implementations)
// ==========================================================================================================================================

/// Canonical operation symbol for `lock_funds`.
#[allow(dead_code)]
pub fn op_lock() -> Symbol {
    symbol_short!("lock")
}

/// Canonical operation symbol for `release_funds` / claim`.
#[allow(dead_code)]
pub fn op_release() -> Symbol {
    symbol_short!("release")
}

/// Canonical operation symbol for `refund`.
#[allow(dead_code)]
pub fn op_refund() -> Symbol {
    symbol_short!("refund")
}
