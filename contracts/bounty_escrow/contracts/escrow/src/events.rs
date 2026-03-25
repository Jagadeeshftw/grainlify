use crate::CapabilityAction;
use soroban_sdk::{contracttype, symbol_short, Address, BytesN, Env};

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
pub struct FeeRoutingUpdated {
    pub bounty_id: u64,
    pub treasury_recipient: Address,
    pub treasury_bps: i128,
    pub partner_recipient: Option<Address>,
    pub partner_bps: i128,
    pub timestamp: u64,
}

pub fn emit_fee_routing_updated(env: &Env, event: FeeRoutingUpdated) {
    let topics = (symbol_short!("fee_rte"), event.bounty_id);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct FeeRouted {
    pub bounty_id: u64,
    pub operation_type: FeeOperationType,
    pub gross_amount: i128,
    pub total_fee: i128,
    pub fee_rate: i128,
    pub treasury_recipient: Address,
    pub treasury_fee: i128,
    pub partner_recipient: Option<Address>,
    pub partner_fee: i128,
    pub timestamp: u64,
}

pub fn emit_fee_routed(env: &Env, event: FeeRouted) {
    let topics = (symbol_short!("fee_rt"), event.bounty_id);
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
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimExecuted {
    pub bounty_id: u64,
    pub recipient: Address,
    pub amount: i128,
    pub claimed_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimCancelled {
    pub bounty_id: u64,
    pub recipient: Address,
    pub amount: i128,
    pub cancelled_at: u64,
    pub cancelled_by: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CriticalOperationOutcome {
    Released,
    Refunded,
}

/// Event emitted when deterministic pseudo-random winner selection is derived.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeterministicSelectionDerived {
    pub bounty_id: u64,
    pub selected_index: u32,
    pub candidate_count: u32,
    pub selected_beneficiary: Address,
    pub seed_hash: BytesN<32>,
    pub winner_score: BytesN<32>,
    pub timestamp: u64,
}

pub fn emit_deterministic_selection(env: &Env, event: DeterministicSelectionDerived) {
    let topics = (symbol_short!("prng_sel"), event.bounty_id);
    env.events().publish(topics, event);
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FundsLockedAnon {
    pub version: u32,
    pub bounty_id: u64,
    pub amount: i128,
    pub depositor_commitment: BytesN<32>,
    pub deadline: u64,
}

pub fn emit_funds_locked_anon(env: &Env, event: FundsLockedAnon) {
    let topics = (symbol_short!("f_lkanon"), event.bounty_id);
    env.events().publish(topics, event);
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeprecationStateChanged {
    pub deprecated: bool,
    pub migration_target: Option<Address>,
    pub admin: Address,
    pub timestamp: u64,
}

pub fn emit_deprecation_state_changed(env: &Env, event: DeprecationStateChanged) {
    let topics = (symbol_short!("deprec"),);
    env.events().publish(topics, event);
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaintenanceModeChanged {
    pub enabled: bool,
    pub admin: Address,
    pub timestamp: u64,
}

pub fn emit_maintenance_mode_changed(env: &Env, event: MaintenanceModeChanged) {
    let topics = (symbol_short!("maint"),);
    env.events().publish(topics, event);
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParticipantFilterModeChanged {
    pub previous_mode: crate::ParticipantFilterMode,
    pub new_mode: crate::ParticipantFilterMode,
    pub admin: Address,
    pub timestamp: u64,
}

pub fn emit_participant_filter_mode_changed(env: &Env, event: ParticipantFilterModeChanged) {
    let topics = (symbol_short!("pf_mode"),);
    env.events().publish(topics, event);
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RiskFlagsUpdated {
    pub version: u32,
    pub bounty_id: u64,
    pub previous_flags: u32,
    pub new_flags: u32,
    pub admin: Address,
    pub timestamp: u64,
}

pub fn emit_risk_flags_updated(env: &Env, event: RiskFlagsUpdated) {
    let topics = (symbol_short!("risk"), event.bounty_id);
    env.events().publish(topics, event);
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationPreferencesUpdated {
    pub version: u32,
    pub bounty_id: u64,
    pub previous_prefs: u32,
    pub new_prefs: u32,
    pub actor: Address,
    pub created: bool,
    pub timestamp: u64,
}

pub fn emit_notification_preferences_updated(env: &Env, event: NotificationPreferencesUpdated) {
    let topics = (symbol_short!("npref"), event.bounty_id);
    env.events().publish(topics, event);
}

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
    let topics = (symbol_short!("ticket_i"), event.ticket_id);
    env.events().publish(topics, event);
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TicketClaimed {
    pub ticket_id: u64,
    pub bounty_id: u64,
    pub claimer: Address,
    pub claimed_at: u64,
}

pub fn emit_ticket_claimed(env: &Env, event: TicketClaimed) {
    let topics = (symbol_short!("ticket_c"), event.ticket_id);
    env.events().publish(topics, event);
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
