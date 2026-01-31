//! # Indexing Integration Module
//!
//! Bridges the core contract logic with the indexing system and event emission.
//! This module handles the "side effects" of contract operations: updating indexes
//! and emitting enhanced events for off-chain trackers.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                 Indexing Integration Logic                  │
//! ├─────────────────────────────────────────────────────────────┤
//! │                                                             │
//! │   ┌────────────────────┐       ┌────────────────────┐       │
//! │   │   Main Contract    │       │     Event System   │       │
//! │   └─────────┬──────────┘       └─────────▲──────────┘       │
//! │             │                            │                  │
//! │             │ (calls)                    │ (emits)          │
//! │             ▼                            │                  │
//! │   on_funds_locked() ─────────────────────┤                  │
//! │             │                                               │
//! │             │ (updates)                                     │
//! │             ▼                                               │
//! │   ┌────────────────────┐                                    │
//! │   │   Indexed Storage  │                                    │
//! │   └────────────────────┘                                    │
//! │                                                             │
//! └─────────────────────────────────────────────────────────────┘
//! ```

use crate::indexed::enhanced_events::{
    _emit_bounty_activity, _emit_bounty_status_changed, _emit_enhanced_funds_locked,
    _emit_enhanced_funds_refunded, _emit_enhanced_funds_released, create_event_metadata,
    ActivityType, BountyActivity, BountyStatusChanged, EnhancedFundsLocked, EnhancedFundsRefunded,
    EnhancedFundsReleased,
};
use crate::indexed::indexed_storage::{
    index_bounty, update_bounty_status, BountyStatus, IndexedBounty,
};
use crate::RefundMode;
use soroban_sdk::{Address, Env};

/// Handler called when funds are locked in escrow.
///
/// Use this hook to update indexes and emit creation events.
///
/// # Arguments
/// * `env` - The contract environment
/// * `bounty_id` - ID of the new bounty
/// * `amount` - Amount locked
/// * `depositor` - Address of the depositor
/// * `deadline` - Timestamp when refund becomes possible
///
/// # State Changes
/// - Creates new `IndexedBounty` entry
/// - Emits `EnhancedFundsLocked` event
/// - Emits `BountyActivity` event
pub fn on_funds_locked(
    env: &Env,
    bounty_id: u64,
    amount: i128,
    depositor: &Address,
    deadline: u64,
) {
    let timestamp = env.ledger().timestamp();

    // Create and store indexed bounty
    let indexed_bounty = IndexedBounty {
        bounty_id,
        depositor: depositor.clone(),
        amount,
        deadline,
        status: BountyStatus::Locked,
        created_at: timestamp,
        updated_at: timestamp,
    };
    index_bounty(env, indexed_bounty);

    // Create event metadata
    let metadata = create_event_metadata(env);

    // _emit enhanced funds locked event
    let enhanced_event = EnhancedFundsLocked {
        bounty_id,
        amount,
        depositor: depositor.clone(),
        deadline,
        timestamp,
        metadata: metadata.clone(),
    };
    _emit_enhanced_funds_locked(env, enhanced_event);

    // _emit activity tracking event
    let activity = BountyActivity {
        bounty_id,
        activity_type: ActivityType::Locked,
        actor: depositor.clone(),
        amount: Some(amount),
        timestamp,
        metadata,
    };
    _emit_bounty_activity(env, activity);
}

/// Handler called when funds are released to a recipient.
///
/// # Arguments
/// * `env` - The contract environment
/// * `bounty_id` - ID of the bounty
/// * `amount` - Amount released in this transaction
/// * `recipient` - Address receiving the funds
/// * `remaining_amount` - Funds remaining in escrow (0 for full release)
/// * `is_partial` - True if this is a partial release
///
/// # State Changes
/// - Updates bounty status filters
/// - Emits `EnhancedFundsReleased` event
/// - Emits `BountyStatusChanged` event
/// - Emits `BountyActivity` event
pub fn on_funds_released(
    env: &Env,
    bounty_id: u64,
    amount: i128,
    recipient: &Address,
    remaining_amount: i128,
    is_partial: bool,
) {
    let timestamp = env.ledger().timestamp();

    // Update bounty status
    let new_status = if remaining_amount > 0 {
        BountyStatus::PartiallyReleased
    } else {
        BountyStatus::Released
    };
    update_bounty_status(env, bounty_id, new_status.clone());

    // Create event metadata
    let metadata = create_event_metadata(env);

    // _emit enhanced funds released event
    let enhanced_event = EnhancedFundsReleased {
        bounty_id,
        amount,
        recipient: recipient.clone(),
        timestamp,
        remaining_amount,
        metadata: metadata.clone(),
        is_partial,
    };
    _emit_enhanced_funds_released(env, enhanced_event);

    // _emit status change event
    let old_status_str = if is_partial { "Locked" } else { "Locked" };
    let new_status_str = if is_partial {
        "PartiallyReleased"
    } else {
        "Released"
    };

    let status_event = BountyStatusChanged {
        bounty_id,
        old_status: soroban_sdk::String::from_str(env, old_status_str),
        new_status: soroban_sdk::String::from_str(env, new_status_str),
        changed_by: recipient.clone(),
        timestamp,
        metadata: metadata.clone(),
    };
    _emit_bounty_status_changed(env, status_event);

    // _emit activity tracking event
    let activity_type = if is_partial {
        ActivityType::PartialRelease
    } else {
        ActivityType::Released
    };

    let activity = BountyActivity {
        bounty_id,
        activity_type,
        actor: recipient.clone(),
        amount: Some(amount),
        timestamp,
        metadata,
    };
    _emit_bounty_activity(env, activity);
}

/// Handler called when funds are refunded to the depositor.
///
/// # Arguments
/// * `env` - The contract environment
/// * `bounty_id` - ID of the bounty
/// * `amount` - Amount refunded
/// * `refund_to` - Address receiving the refund
/// * `remaining_amount` - Funds remaining in escrow
/// * `refund_mode` - Type of refund (Full, Partial, etc.)
/// * `triggered_by` - Address triggering the refund (admin or depositor)
///
/// # State Changes
/// - Updates bounty status filters
/// - Emits `EnhancedFundsRefunded` event
/// - Emits `BountyStatusChanged` event
/// - Emits `BountyActivity` event
pub fn on_funds_refunded(
    env: &Env,
    bounty_id: u64,
    amount: i128,
    refund_to: &Address,
    remaining_amount: i128,
    refund_mode: RefundMode,
    triggered_by: &Address,
) {
    let timestamp = env.ledger().timestamp();

    // Update bounty status
    let new_status = if remaining_amount > 0 {
        BountyStatus::PartiallyReleased
    } else {
        BountyStatus::Refunded
    };
    update_bounty_status(env, bounty_id, new_status);

    // Create event metadata
    let metadata = create_event_metadata(env);

    // _emit enhanced funds refunded event
    let enhanced_event = EnhancedFundsRefunded {
        bounty_id,
        amount,
        refund_to: refund_to.clone(),
        timestamp,
        remaining_amount,
        metadata: metadata.clone(),
        refund_reason: refund_mode,
        triggered_by: triggered_by.clone(),
    };
    _emit_enhanced_funds_refunded(env, enhanced_event);

    // _emit status change event
    let new_status_str = if remaining_amount > 0 {
        "PartiallyRefunded"
    } else {
        "Refunded"
    };

    let status_event = BountyStatusChanged {
        bounty_id,
        old_status: soroban_sdk::String::from_str(env, "Locked"),
        new_status: soroban_sdk::String::from_str(env, new_status_str),
        changed_by: triggered_by.clone(),
        timestamp,
        metadata: metadata.clone(),
    };
    _emit_bounty_status_changed(env, status_event);

    // _emit activity tracking event
    let activity_type = if remaining_amount > 0 {
        ActivityType::PartialRefund
    } else {
        ActivityType::Refunded
    };

    let activity = BountyActivity {
        bounty_id,
        activity_type,
        actor: triggered_by.clone(),
        amount: Some(amount),
        timestamp,
        metadata,
    };
    _emit_bounty_activity(env, activity);
}

/// Internal handler for bounty cancellation.
///
/// # Arguments
/// * `env` - The contract environment
/// * `bounty_id` - ID of the bounty to cancel
/// * `cancelled_by` - Address initiating cancel
pub fn _on_bounty_cancelled(env: &Env, bounty_id: u64, cancelled_by: &Address) {
    let timestamp = env.ledger().timestamp();

    // Update bounty status to refunded
    update_bounty_status(env, bounty_id, BountyStatus::Refunded);

    // Create event metadata
    let metadata = create_event_metadata(env);

    // _emit status change event
    let status_event = BountyStatusChanged {
        bounty_id,
        old_status: soroban_sdk::String::from_str(env, "Locked"),
        new_status: soroban_sdk::String::from_str(env, "Cancelled"),
        changed_by: cancelled_by.clone(),
        timestamp,
        metadata: metadata.clone(),
    };
    _emit_bounty_status_changed(env, status_event);

    // _emit activity tracking event
    let activity = BountyActivity {
        bounty_id,
        activity_type: ActivityType::Cancelled,
        actor: cancelled_by.clone(),
        amount: None,
        timestamp,
        metadata,
    };
    _emit_bounty_activity(env, activity);
}

/// Internal handler for deadline extension.
///
/// # Arguments
/// * `env` - The contract environment
/// * `bounty_id` - ID of the bounty
/// * `old_deadline` - Previous deadline timestamp
/// * `new_deadline` - new deadline timestamp
/// * `extended_by` - Address changing the deadline
pub fn _on_deadline_extended(
    env: &Env,
    bounty_id: u64,
    old_deadline: u64,
    new_deadline: u64,
    extended_by: &Address,
) {
    use crate::indexed::enhanced_events::{BountyDeadlineExtended, _emit_bounty_deadline_extended};

    let timestamp = env.ledger().timestamp();
    let metadata = create_event_metadata(env);

    // _emit deadline extended event
    let event = BountyDeadlineExtended {
        bounty_id,
        old_deadline,
        new_deadline,
        extended_by: extended_by.clone(),
        timestamp,
        metadata: metadata.clone(),
    };
    _emit_bounty_deadline_extended(env, event);

    // _emit activity tracking event
    let activity = BountyActivity {
        bounty_id,
        activity_type: ActivityType::DeadlineExtended,
        actor: extended_by.clone(),
        amount: None,
        timestamp,
        metadata,
    };
    _emit_bounty_activity(env, activity);
}

/// Internal handler for amount increase.
///
/// # Arguments
/// * `env` - The contract environment
/// * `bounty_id` - ID of the bounty
/// * `old_amount` - Previous locked amount
/// * `increase_amount` - Amount added
/// * `increased_by` - Address adding funds
pub fn _on_amount_increased(
    env: &Env,
    bounty_id: u64,
    old_amount: i128,
    increase_amount: i128,
    increased_by: &Address,
) {
    use crate::indexed::enhanced_events::{BountyAmountIncreased, _emit_bounty_amount_increased};

    let timestamp = env.ledger().timestamp();
    let metadata = create_event_metadata(env);
    let new_amount = old_amount + increase_amount;

    // _emit amount increased event
    let event = BountyAmountIncreased {
        bounty_id,
        old_amount,
        new_amount,
        increase_amount,
        increased_by: increased_by.clone(),
        timestamp,
        metadata: metadata.clone(),
    };
    _emit_bounty_amount_increased(env, event);

    // _emit activity tracking event
    let activity = BountyActivity {
        bounty_id,
        activity_type: ActivityType::AmountIncreased,
        actor: increased_by.clone(),
        amount: Some(increase_amount),
        timestamp,
        metadata,
    };
    _emit_bounty_activity(env, activity);
}
