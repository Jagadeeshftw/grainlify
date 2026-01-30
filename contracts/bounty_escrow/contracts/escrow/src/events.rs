//! # Bounty Escrow Events Module
//!
//! This module defines all events emitted by the Bounty Escrow contract.
//! Events provide an audit trail and enable off-chain indexing for monitoring
//! bounty lifecycle states.

use soroban_sdk::{contracttype, symbol_short, Address, Env};

// ============================================================================
// Contract Initialization Event
// ============================================================================

#[contracttype]
#[derive(Clone, Debug)]
pub struct BountyEscrowInitialized {
    pub admin: Address,
    pub token: Address,
    pub timestamp: u64,
}

pub fn emit_bounty_initialized(env: &Env, event: BountyEscrowInitialized) {
    let topics = (symbol_short!("init"),);
    env.events().publish(topics, event.clone());
}

// ============================================================================
// Funds Locked Event
// ============================================================================

#[contracttype]
#[derive(Clone, Debug)]
pub struct FundsLocked {
    pub bounty_id: u64,
    pub amount: i128,
    pub depositor: Address,
    pub deadline: u64,
    pub token_address: Address, // Token used for this lock
}

/// Emits a FundsLocked event.
///
/// # Arguments
/// * `env` - The contract environment
/// * `event` - The funds locked event data
///
/// # Event Structure
/// Topic: `(symbol_short!("f_lock"), event.bounty_id)`
/// Data: Complete `FundsLocked` struct
///
/// # Indexing Note
/// The bounty_id is included in topics for efficient filtering
pub fn emit_funds_locked(env: &Env, event: FundsLocked) {
    let topics = (symbol_short!("f_lock"), event.bounty_id);
    env.events().publish(topics, event.clone());
}

// ============================================================================
// Funds Released Event
// ============================================================================

#[contracttype]
#[derive(Clone, Debug)]
pub struct FundsReleased {
    pub bounty_id: u64,
    pub amount: i128,
    pub recipient: Address,
    pub token_address: Address, // Token used for this release
    pub timestamp: u64,
    pub remaining_amount: i128,
}

/// Emits a FundsReleased event.
///
/// # Arguments
/// * `env` - The contract environment
/// * `event` - The funds released event data
///
/// # Event Structure
/// Topic: `(symbol_short!("f_rel"), event.bounty_id)`
/// Data: Complete `FundsReleased` struct
pub fn emit_funds_released(env: &Env, event: FundsReleased) {
    let topics = (symbol_short!("f_rel"), event.bounty_id);
    env.events().publish(topics, event.clone());
}

// ============================================================================
// Funds Refunded Event
// ============================================================================

#[contracttype]
#[derive(Clone, Debug)]
pub struct FundsRefunded {
    pub bounty_id: u64,
    pub amount: i128,
    pub refund_to: Address,
    pub timestamp: u64,
    pub refund_mode: crate::RefundMode,
    pub remaining_amount: i128,
    pub token_address: Address, // Token used for this refund
}

/// Emits a FundsRefunded event.
///
/// # Arguments
/// * `env` - The contract environment
/// * `event` - The funds refunded event data
///
/// # Event Structure
/// Topic: `(symbol_short!("f_ref"), event.bounty_id)`
/// Data: Complete `FundsRefunded` struct
pub fn emit_funds_refunded(env: &Env, event: FundsRefunded) {
    let topics = (symbol_short!("f_ref"), event.bounty_id);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FeeOperationType {
    Lock,
    Release,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct FeeCollected {
    pub operation_type: FeeOperationType,
    pub amount: i128,
    pub fee_rate: i128,
    pub recipient: Address,
    pub timestamp: u64,
}

pub fn emit_fee_collected(env: &Env, event: FeeCollected) {
    let topics = (symbol_short!("fee"),);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct BatchFundsLocked {
    pub count: u32,
    pub total_amount: i128,
    pub timestamp: u64,
}

pub fn emit_batch_funds_locked(env: &Env, event: BatchFundsLocked) {
    let topics = (symbol_short!("b_lock"),);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct FeeConfigUpdated {
    pub lock_fee_rate: i128,
    pub release_fee_rate: i128,
    pub fee_recipient: Address,
    pub fee_enabled: bool,
    pub timestamp: u64,
}

pub fn emit_fee_config_updated(env: &Env, event: FeeConfigUpdated) {
    let topics = (symbol_short!("fee_cfg"),);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct BatchFundsReleased {
    pub count: u32,
    pub total_amount: i128,
    pub timestamp: u64,
}

pub fn emit_batch_funds_released(env: &Env, event: BatchFundsReleased) {
    let topics = (symbol_short!("b_rel"),);
    env.events().publish(topics, event.clone());
}

// ============================================================================
// Contract Pause Events
// ============================================================================

#[contracttype]
#[derive(Clone, Debug)]
pub struct ContractPaused {
    pub paused_by: Address,
    pub timestamp: u64,
}

pub fn emit_contract_paused(env: &Env, event: ContractPaused) {
    let topics = (symbol_short!("pause"),);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct ContractUnpaused {
    pub unpaused_by: Address,
    pub timestamp: u64,
}

pub fn emit_contract_unpaused(env: &Env, event: ContractUnpaused) {
    let topics = (symbol_short!("unpause"),);
    env.events().publish(topics, event.clone());
}

/// Event emitted when a specific operation's pause state changes.
///
/// This event provides granular tracking of pause state changes for individual
/// operations (lock, release, refund) rather than the global pause state.
#[contracttype]
#[derive(Clone, Debug)]
pub struct OperationPauseChanged {
    pub operation: soroban_sdk::Symbol,
    pub paused: bool,
    pub changed_by: Address,
    pub timestamp: u64,
}

pub fn emit_operation_pause_changed(env: &Env, event: OperationPauseChanged) {
    let topics = (symbol_short!("op_pause"), event.operation.clone());
    env.events().publish(topics, event.clone());
}

// ============================================================================
// Emergency Withdrawal Event
// ============================================================================

#[contracttype]
#[derive(Clone, Debug)]
pub struct EmergencyWithdrawal {
    pub withdrawn_by: Address,
    pub amount: i128,
    pub recipient: Address,
    pub timestamp: u64,
}

pub fn emit_emergency_withdrawal(env: &Env, event: EmergencyWithdrawal) {
    let topics = (symbol_short!("ewith"),);
    env.events().publish(topics, event.clone());
}

// ============================================================================
// Admin Configuration Events
// ============================================================================

/// Event emitted when admin is updated.
#[contracttype]
#[derive(Clone, Debug)]
pub struct AdminUpdated {
    pub old_admin: Address,
    pub new_admin: Address,
    pub updated_by: Address,
    pub timestamp: u64,
}

pub fn emit_admin_updated(env: &Env, event: AdminUpdated) {
    let topics = (symbol_short!("adm_upd"),);
    env.events().publish(topics, event.clone());
}

/// Event emitted when authorized payout key is updated.
#[contracttype]
#[derive(Clone, Debug)]
pub struct PayoutKeyUpdated {
    pub old_key: Option<Address>,
    pub new_key: Address,
    pub updated_by: Address,
    pub timestamp: u64,
}

pub fn emit_payout_key_updated(env: &Env, event: PayoutKeyUpdated) {
    let topics = (symbol_short!("pay_upd"),);
    env.events().publish(topics, event.clone());
}

/// Event emitted when configuration limits are updated.
#[contracttype]
#[derive(Clone, Debug)]
pub struct ConfigLimitsUpdated {
    pub max_bounty_amount: Option<i128>,
    pub min_bounty_amount: Option<i128>,
    pub max_deadline_duration: Option<u64>,
    pub min_deadline_duration: Option<u64>,
    pub updated_by: Address,
    pub timestamp: u64,
}

pub fn emit_config_limits_updated(env: &Env, event: ConfigLimitsUpdated) {
    let topics = (symbol_short!("cfg_lmt"),);
    env.events().publish(topics, event.clone());
}

/// Event emitted when an admin action is proposed (for time-lock).
#[contracttype]
#[derive(Clone, Debug)]
pub struct AdminActionProposed {
    pub action_id: u64,
    pub action_type: crate::AdminActionType,
    pub proposed_by: Address,
    pub execution_time: u64,
    pub timestamp: u64,
}

pub fn emit_admin_action_proposed(env: &Env, event: AdminActionProposed) {
    let topics = (symbol_short!("adm_prop"),);
    env.events().publish(topics, event.clone());
}

/// Event emitted when an admin action is executed.
#[contracttype]
#[derive(Clone, Debug)]
pub struct AdminActionExecuted {
    pub action_id: u64,
    pub action_type: crate::AdminActionType,
    pub executed_by: Address,
    pub timestamp: u64,
}

pub fn emit_admin_action_executed(env: &Env, event: AdminActionExecuted) {
    let topics = (symbol_short!("adm_exec"),);
    env.events().publish(topics, event.clone());
}

/// Event emitted when an admin action is cancelled.
#[contracttype]
#[derive(Clone, Debug)]
pub struct AdminActionCancelled {
    pub action_id: u64,
    pub action_type: crate::AdminActionType,
    pub cancelled_by: Address,
    pub timestamp: u64,
}

pub fn emit_admin_action_cancelled(env: &Env, event: AdminActionCancelled) {
    let topics = (symbol_short!("adm_cncl"),);
    env.events().publish(topics, event.clone());
}

// ============================================================================
// Deadline Extended Event
// ============================================================================

/// Event emitted when the refund deadline for an escrow is extended.
///
/// # Fields
/// * `bounty_id` - The bounty identifier
/// * `old_deadline` - The previous deadline timestamp
/// * `new_deadline` - The new deadline timestamp
/// * `extended_by` - Address that extended the deadline (admin or depositor)
/// * `timestamp` - Unix timestamp when the extension occurred
///
/// # Event Topic
/// Symbol: `deadline_ext`
/// Indexed: `bounty_id`
///
/// # Usage
/// Emitted when an authorized party (admin or depositor) extends the refund
/// deadline for a bounty. This allows bounties to be extended without
/// migrating funds to a new escrow.
///
/// # Conditions
/// - New deadline must be strictly greater than old deadline
/// - Escrow must be in Locked or PartiallyRefunded state
/// - Only admin or depositor can extend deadline
///
/// # Security Considerations
/// - Deadline can only be extended, never shortened
/// - Prevents indefinite locking by requiring extension
/// - Full audit trail of all deadline changes
#[contracttype]
#[derive(Clone, Debug)]
pub struct DeadlineExtended {
    pub bounty_id: u64,
    pub old_deadline: u64,
    pub new_deadline: u64,
    pub extended_by: Address,
    pub timestamp: u64,
}

/// Emits a DeadlineExtended event.
///
/// # Arguments
/// * `env` - The contract environment
/// * `event` - The deadline extended event data
///
/// # Event Structure
/// Topic: `(symbol_short!("dead_ext"), event.bounty_id)`
/// Data: Complete `DeadlineExtended` struct
pub fn emit_deadline_extended(env: &Env, event: DeadlineExtended) {
    let topics = (symbol_short!("dead_ext"), event.bounty_id);
    env.events().publish(topics, event.clone());
}

// ============================================================================
// Escrow Expired Event
// ============================================================================

/// Event emitted when an escrow expires and is automatically refunded.
///
/// # Fields
/// * `bounty_id` - The bounty identifier
/// * `amount` - Amount refunded
/// * `refunded_to` - Address receiving the refund (original depositor)
/// * `triggered_by` - Address that triggered the expiration
/// * `timestamp` - Unix timestamp of expiration
#[contracttype]
#[derive(Clone, Debug)]
pub struct EscrowExpired {
    pub bounty_id: u64,
    pub amount: i128,
    pub refunded_to: Address,
    pub triggered_by: Address,
    pub timestamp: u64,
}

pub fn emit_escrow_expired(env: &Env, event: EscrowExpired) {
    let topics = (symbol_short!("expired"), event.bounty_id);
    env.events().publish(topics, event.clone());
}
