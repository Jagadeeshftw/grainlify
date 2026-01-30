use crate::RefundMode;
use soroban_sdk::{contracttype, symbol_short, Address, Env, String};

const EVENT_VERSION: u32 = 1;

// ============================================================================
// Event Metadata
// ============================================================================

#[contracttype]
#[derive(Clone, Debug)]
pub struct EventMetadata {
    pub version: u32,
    pub block_number: u32,
    pub transaction_hash: String,
}

pub fn create_event_metadata(env: &Env) -> EventMetadata {
    EventMetadata {
        version: EVENT_VERSION,
        block_number: env.ledger().sequence(),
        transaction_hash: String::from_str(env, ""),
    }
}

// ============================================================================
// Contract Initialization Events
// ============================================================================

#[contracttype]
#[derive(Clone, Debug)]
pub struct BountyEscrowInitialized {
    pub admin: Address,
    pub token: Address,
    pub timestamp: u64,
}

/// _emits a BountyEscrowInitialized event.
pub fn _emit_bounty_initialized(env: &Env, event: BountyEscrowInitialized) {
    let topics = (symbol_short!("init"),);
    env.events().publish(topics, event.clone());
}

/// Enhanced version with metadata
pub fn _emit_enhanced_bounty_initialized(env: &Env, event: BountyEscrowInitialized) {
    let topics = (symbol_short!("init"),);
    env.events().publish(topics, event.clone());
}

// ============================================================================
// Enhanced Fund Lifecycle Events
// ============================================================================

#[contracttype]
#[derive(Clone, Debug)]
pub struct EnhancedFundsLocked {
    pub bounty_id: u64,
    pub amount: i128,
    pub depositor: Address,
    pub deadline: u64,
    pub timestamp: u64,
    pub metadata: EventMetadata,
}

pub fn _emit_enhanced_funds_locked(env: &Env, event: EnhancedFundsLocked) {
    let topics = (symbol_short!("f_lock"), event.bounty_id);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct EnhancedFundsReleased {
    pub bounty_id: u64,
    pub amount: i128,
    pub recipient: Address,
    pub timestamp: u64,
    pub remaining_amount: i128,
    pub metadata: EventMetadata,
    pub is_partial: bool,
}

pub fn _emit_enhanced_funds_released(env: &Env, event: EnhancedFundsReleased) {
    let topics = (symbol_short!("f_rel"), event.bounty_id);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct EnhancedFundsRefunded {
    pub bounty_id: u64,
    pub amount: i128,
    pub refund_to: Address,
    pub timestamp: u64,
    pub remaining_amount: i128,
    pub metadata: EventMetadata,
    pub refund_reason: RefundMode,
    pub triggered_by: Address,
}

pub fn _emit_enhanced_funds_refunded(env: &Env, event: EnhancedFundsRefunded) {
    let topics = (symbol_short!("f_ref"), event.bounty_id);
    env.events().publish(topics, event.clone());
}

// ============================================================================
// Status Change Events
// ============================================================================

#[contracttype]
#[derive(Clone, Debug)]
pub struct BountyStatusChanged {
    pub bounty_id: u64,
    pub old_status: String,
    pub new_status: String,
    pub changed_by: Address,
    pub timestamp: u64,
    pub metadata: EventMetadata,
}

pub fn _emit_bounty_status_changed(env: &Env, event: BountyStatusChanged) {
    let topics = (symbol_short!("status"), event.bounty_id);
    env.events().publish(topics, event.clone());
}

// ============================================================================
// Activity Tracking Events
// ============================================================================

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActivityType {
    Created,
    Locked,
    Released,
    Refunded,
    PartialRelease,
    PartialRefund,
    Cancelled,
    DeadlineExtended,
    AmountIncreased,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct BountyActivity {
    pub bounty_id: u64,
    pub activity_type: ActivityType,
    pub actor: Address,
    pub amount: Option<i128>,
    pub timestamp: u64,
    pub metadata: EventMetadata,
}

pub fn _emit_bounty_activity(env: &Env, event: BountyActivity) {
    let topics = (
        symbol_short!("activity"),
        event.bounty_id,
        event.activity_type.clone(),
    );
    env.events().publish(topics, event.clone());
}

// ============================================================================
// Analytics Events
// ============================================================================

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnalyticsType {
    DailyVolume,
    WeeklyVolume,
    MonthlyVolume,
    TotalLocked,
    TotalReleased,
    TotalRefunded,
    ActiveBounties,
    CompletionRate,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct AnalyticsEvent {
    pub event_type: AnalyticsType,
    pub count: u32,
    pub total_amount: i128,
    pub timestamp: u64,
}

pub fn _emit_analytics_event(env: &Env, event: AnalyticsEvent) {
    let topics = (symbol_short!("analytic"), event.event_type.clone());
    env.events().publish(topics, event.clone());
}

// ============================================================================
// Fee Events
// ============================================================================

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FeeOperationType {
    Lock,
    Release,
    Refund,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct FeeCollected {
    pub operation_type: FeeOperationType,
    pub bounty_id: u64,
    pub amount: i128,
    pub fee_rate: i128,
    pub recipient: Address,
    pub timestamp: u64,
    pub metadata: EventMetadata,
}

pub fn _emit_fee_collected(env: &Env, event: FeeCollected) {
    let topics = (symbol_short!("fee"), event.bounty_id);
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
    pub metadata: EventMetadata,
}

pub fn _emit_fee_config_updated(env: &Env, event: FeeConfigUpdated) {
    let topics = (symbol_short!("fee_cfg"),);
    env.events().publish(topics, event.clone());
}

// ============================================================================
// Batch Operation Events
// ============================================================================

#[contracttype]
#[derive(Clone, Debug)]
pub struct BatchFundsLocked {
    pub bounty_ids: soroban_sdk::Vec<u64>,
    pub count: u32,
    pub total_amount: i128,
    pub depositor: Address,
    pub timestamp: u64,
    pub metadata: EventMetadata,
}

pub fn _emit_batch_funds_locked(env: &Env, event: BatchFundsLocked) {
    let topics = (symbol_short!("b_lock"),);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct BatchFundsReleased {
    pub bounty_ids: soroban_sdk::Vec<u64>,
    pub count: u32,
    pub total_amount: i128,
    pub recipient: Address,
    pub timestamp: u64,
    pub metadata: EventMetadata,
}

pub fn _emit_batch_funds_released(env: &Env, event: BatchFundsReleased) {
    let topics = (symbol_short!("b_rel"),);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct BatchFundsRefunded {
    pub bounty_ids: soroban_sdk::Vec<u64>,
    pub count: u32,
    pub total_amount: i128,
    pub refund_to: Address,
    pub timestamp: u64,
    pub metadata: EventMetadata,
}

pub fn _emit_batch_funds_refunded(env: &Env, event: BatchFundsRefunded) {
    let topics = (symbol_short!("b_ref"),);
    env.events().publish(topics, event.clone());
}

// ============================================================================
// Admin Events
// ============================================================================

#[contracttype]
#[derive(Clone, Debug)]
pub struct AdminChanged {
    pub old_admin: Address,
    pub new_admin: Address,
    pub changed_by: Address,
    pub timestamp: u64,
    pub metadata: EventMetadata,
}

pub fn _emit_admin_changed(env: &Env, event: AdminChanged) {
    let topics = (symbol_short!("admin_ch"),);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct ContractPaused {
    pub paused: bool,
    pub paused_by: Address,
    pub timestamp: u64,
    pub metadata: EventMetadata,
}

pub fn _emit_contract_paused(env: &Env, event: ContractPaused) {
    let topics = (symbol_short!("pause"),);
    env.events().publish(topics, event.clone());
}

// ============================================================================
// Bounty Modification Events
// ============================================================================

#[contracttype]
#[derive(Clone, Debug)]
pub struct BountyDeadlineExtended {
    pub bounty_id: u64,
    pub old_deadline: u64,
    pub new_deadline: u64,
    pub extended_by: Address,
    pub timestamp: u64,
    pub metadata: EventMetadata,
}

pub fn _emit_bounty_deadline_extended(env: &Env, event: BountyDeadlineExtended) {
    let topics = (symbol_short!("dl_ext"), event.bounty_id);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct BountyAmountIncreased {
    pub bounty_id: u64,
    pub old_amount: i128,
    pub new_amount: i128,
    pub increase_amount: i128,
    pub increased_by: Address,
    pub timestamp: u64,
    pub metadata: EventMetadata,
}

pub fn _emit_bounty_amount_increased(env: &Env, event: BountyAmountIncreased) {
    let topics = (symbol_short!("amt_inc"), event.bounty_id);
    env.events().publish(topics, event.clone());
}

// ============================================================================
// Error/Warning Events
// ============================================================================

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErrorType {
    InsufficientBalance,
    InvalidDeadline,
    UnauthorizedAccess,
    BountyNotFound,
    InvalidStatus,
    DeadlineNotPassed,
    DeadlinePassed,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct ErrorOccurred {
    pub error_type: ErrorType,
    pub bounty_id: Option<u64>,
    pub actor: Address,
    pub message: String,
    pub timestamp: u64,
    pub metadata: EventMetadata,
}

pub fn _emit_error_occurred(env: &Env, event: ErrorOccurred) {
    let topics = (symbol_short!("error"), event.error_type.clone());
    env.events().publish(topics, event.clone());
}

// ============================================================================
// Milestone Events (for multi-milestone bounties)
// ============================================================================

#[contracttype]
#[derive(Clone, Debug)]
pub struct MilestoneCompleted {
    pub bounty_id: u64,
    pub milestone_id: u32,
    pub amount_released: i128,
    pub recipient: Address,
    pub timestamp: u64,
    pub metadata: EventMetadata,
}

pub fn _emit_milestone_completed(env: &Env, event: MilestoneCompleted) {
    let topics = (
        symbol_short!("mile_cmp"),
        event.bounty_id,
        event.milestone_id,
    );
    env.events().publish(topics, event.clone());
}

// ============================================================================
// Notification Events
// ============================================================================

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NotificationType {
    DeadlineApproaching,
    BountyExpired,
    FundsReceived,
    BountyCompleted,
    RefundAvailable,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct NotificationEvent {
    pub notification_type: NotificationType,
    pub bounty_id: u64,
    pub recipient: Address,
    pub message: String,
    pub timestamp: u64,
    pub metadata: EventMetadata,
}

pub fn _emit_notification(env: &Env, event: NotificationEvent) {
    let topics = (
        symbol_short!("notify"),
        event.bounty_id,
        event.notification_type.clone(),
    );
    env.events().publish(topics, event.clone());
}
