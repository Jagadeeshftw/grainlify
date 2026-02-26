use crate::{CapabilityAction, DisputeOutcome, DisputeReason};
use soroban_sdk::{contracttype, symbol_short, Address, Env};

pub const EVENT_VERSION_V2: u32 = 2;

#[contracttype]
#[derive(Clone, Debug)]
pub struct BountyEscrowInitialized {
    pub version: u32,
    pub admin: Address,
    pub token: Address,
    pub timestamp: u64,
}

pub fn emit_bounty_initialized(env: &Env, event: BountyEscrowInitialized) {
    let topics = (symbol_short!("init"),);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct FundsLocked {
    pub version: u32,
    pub bounty_id: u64,
    pub amount: i128,
    pub depositor: Address,
    pub deadline: u64,
}

pub fn emit_funds_locked(env: &Env, event: FundsLocked) {
    let topics = (symbol_short!("f_lock"), event.bounty_id);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct FundsReleased {
    pub version: u32,
    pub bounty_id: u64,
    pub amount: i128,
    pub recipient: Address,
    pub timestamp: u64,
}

pub fn emit_funds_released(env: &Env, event: FundsReleased) {
    let topics = (symbol_short!("f_rel"), event.bounty_id);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct FundsRefunded {
    pub version: u32,
    pub bounty_id: u64,
    pub amount: i128,
    pub refund_to: Address,
    pub timestamp: u64,
}

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
    pub version: u32,
    pub operation_type: FeeOperationType,
    pub amount: i128,
    pub fee_rate: i128,
    pub effective_fee_rate: i128,  // Actual rate used (after overrides)
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
    pub version: u32,
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
    pub version: u32,
    pub count: u32,
    pub total_amount: i128,
    pub timestamp: u64,
}

pub fn emit_batch_funds_released(env: &Env, event: BatchFundsReleased) {
    let topics = (symbol_short!("b_rel"),);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct ApprovalAdded {
    pub bounty_id: u64,
    pub contributor: Address,
    pub approver: Address,
    pub timestamp: u64,
}

pub fn emit_approval_added(env: &Env, event: ApprovalAdded) {
    let topics = (symbol_short!("approval"), event.bounty_id);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimCreated {
    pub bounty_id: u64, // use program_id+schedule_id equivalent in program-escrow
    pub recipient: Address,
    pub amount: i128,
    pub expires_at: u64,
    pub reason: DisputeReason,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimExecuted {
    pub bounty_id: u64,
    pub recipient: Address,
    pub amount: i128,
    pub claimed_at: u64,
    pub outcome: DisputeOutcome,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimCancelled {
    pub bounty_id: u64,
    pub recipient: Address,
    pub amount: i128,
    pub cancelled_at: u64,
    pub cancelled_by: Address,
    pub outcome: DisputeOutcome,
}

/// Event emitted when a claim ticket is issued to a bounty winner
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TicketIssued {
    pub ticket_id: u64,
    pub bounty_id: u64,
    pub beneficiary: Address,
    pub amount: i128,
    pub expires_at: u64,
    pub issued_at: u64,
}

pub fn emit_ticket_issued(env: &Env, event: TicketIssued) {
    let topics = (symbol_short!("tkt_iss"), event.ticket_id);
    env.events().publish(topics, event.clone());
}

/// Event emitted when a beneficiary claims their reward using a ticket
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TicketClaimed {
    pub ticket_id: u64,
    pub bounty_id: u64,
    pub beneficiary: Address,
    pub amount: i128,
    pub claimed_at: u64,
}

pub fn emit_ticket_claimed(env: &Env, event: TicketClaimed) {
    let topics = (symbol_short!("tkt_clm"), event.ticket_id);
    env.events().publish(topics, event.clone());
}

pub fn emit_pause_state_changed(env: &Env, event: crate::PauseStateChanged) {
    let topics = (symbol_short!("pause"), event.operation.clone());
    env.events().publish(topics, event);
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct EmergencyWithdrawEvent {
    pub admin: Address,
    pub recipient: Address,
    pub amount: i128,
    pub timestamp: u64,
}

pub fn emit_emergency_withdraw(env: &Env, event: EmergencyWithdrawEvent) {
    let topics = (symbol_short!("em_wtd"),);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct PromotionalPeriodCreated {
    pub id: u64,
    pub name: soroban_sdk::String,
    pub start_time: u64,
    pub end_time: u64,
    pub lock_fee_rate: i128,
    pub release_fee_rate: i128,
    pub is_global: bool,
    pub timestamp: u64,
}

pub fn emit_promotional_period_created(env: &Env, event: PromotionalPeriodCreated) {
    let topics = (symbol_short!("promo_c"), event.id);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct PromotionalPeriodUpdated {
    pub id: u64,
    pub enabled: bool,
    pub timestamp: u64,
}

pub fn emit_promotional_period_updated(env: &Env, event: PromotionalPeriodUpdated) {
    let topics = (symbol_short!("promo_u"), event.id);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct PromotionalPeriodActivated {
    pub id: u64,
    pub name: soroban_sdk::String,
    pub lock_fee_rate: i128,
    pub release_fee_rate: i128,
    pub timestamp: u64,
}

pub fn emit_promotional_period_activated(env: &Env, event: PromotionalPeriodActivated) {
    let topics = (symbol_short!("promo_a"), event.id);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct PromotionalPeriodExpired {
    pub id: u64,
    pub name: soroban_sdk::String,
    pub timestamp: u64,
}

pub fn emit_promotional_period_expired(env: &Env, event: PromotionalPeriodExpired) {
    let topics = (symbol_short!("promo_e"), event.id);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityIssued {
    pub capability_id: u64,
    pub owner: Address,
    pub holder: Address,
    pub action: CapabilityAction,
    pub bounty_id: u64,
    pub amount_limit: i128,
    pub expires_at: u64,
    pub max_uses: u32,
    pub timestamp: u64,
}

pub fn emit_capability_issued(env: &Env, event: CapabilityIssued) {
    let topics = (symbol_short!("cap_new"), event.capability_id);
    env.events().publish(topics, event);
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityUsed {
    pub capability_id: u64,
    pub holder: Address,
    pub action: CapabilityAction,
    pub bounty_id: u64,
    pub amount_used: i128,
    pub remaining_amount: i128,
    pub remaining_uses: u32,
    pub used_at: u64,
}

pub fn emit_capability_used(env: &Env, event: CapabilityUsed) {
    let topics = (symbol_short!("cap_use"), event.capability_id);
    env.events().publish(topics, event);
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityRevoked {
    pub capability_id: u64,
    pub owner: Address,
    pub revoked_at: u64,
}

pub fn emit_capability_revoked(env: &Env, event: CapabilityRevoked) {
    let topics = (symbol_short!("cap_rev"), event.capability_id);
    env.events().publish(topics, event);
}

// ==================== Event Batching (Issue #676) ====================
// Compact action summary for batch events. Indexers can decode a single
// EventBatch instead of N individual events during high-volume periods.
// action_type: 1=Lock, 2=Release, 3=Refund (u32 for Soroban contracttype)
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionSummary {
    pub bounty_id: u64,
    pub action_type: u32,
    pub amount: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct EventBatch {
    pub version: u32,
    pub batch_type: u32, // 1=lock, 2=release
    pub actions: soroban_sdk::Vec<ActionSummary>,
    pub total_amount: i128,
    pub timestamp: u64,
}

pub fn emit_event_batch(env: &Env, event: EventBatch) {
    let topics = (symbol_short!("ev_batch"), event.batch_type);
    env.events().publish(topics, event.clone());
}

// ==================== Owner Lock (Issue #675) ====================
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowLockedEvent {
    pub bounty_id: u64,
    pub locked_by: Address,
    pub locked_until: Option<u64>,
    pub reason: Option<soroban_sdk::String>,
    pub timestamp: u64,
}

pub fn emit_escrow_locked(env: &Env, event: EscrowLockedEvent) {
    let topics = (symbol_short!("esc_lock"), event.bounty_id);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowUnlockedEvent {
    pub bounty_id: u64,
    pub unlocked_by: Address,
    pub timestamp: u64,
}

pub fn emit_escrow_unlocked(env: &Env, event: EscrowUnlockedEvent) {
    let topics = (symbol_short!("esc_unl"), event.bounty_id);
    env.events().publish(topics, event.clone());
}

// ==================== Clone/Fork (Issue #678) ====================
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowClonedEvent {
    pub source_bounty_id: u64,
    pub new_bounty_id: u64,
    pub new_owner: Address,
    pub timestamp: u64,
}

pub fn emit_escrow_cloned(env: &Env, event: EscrowClonedEvent) {
    let topics = (symbol_short!("esc_clone"), event.new_bounty_id);
    env.events().publish(topics, event.clone());
}

// ==================== Archive on Completion (Issue #684) ====================
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowArchivedEvent {
    pub bounty_id: u64,
    pub reason: soroban_sdk::String, // e.g. "completed", "released", "refunded"
    pub archived_at: u64,
}

pub fn emit_escrow_archived(env: &Env, event: EscrowArchivedEvent) {
    let topics = (symbol_short!("esc_arch"), event.bounty_id);
    env.events().publish(topics, event.clone());
}
