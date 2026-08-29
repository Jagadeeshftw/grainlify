//! # Event Definitions — Bounty Escrow Contract
//!
//! ## Logging Level Design  (Issue #615)
//!
//! Every event is tagged with a `LogLevel` that is encoded as the **first
//! topic** in the Soroban event tuple.  Indexers and observability pipelines
//! can filter on topic[0] without deserialising the event payload at all.
//!
//! ### Level semantics
//!
//! | Level   | u32 | When to use |
//! |---------|-----|-------------|
//! | `Debug` |  0  | High-frequency operational detail (fee collection, batch item) |
//! | `Info`  |  1  | Normal state transitions (lock, release, refund, init) |
//! | `Warn`  |  2  | Unexpected-but-handled situations (approval added to multisig) |
//! | `Error` |  3  | Reserved — contract panics before reaching emit, so unused today |
//!
//! ### Why topics, not payload fields
//!
//! Soroban's event model lets nodes index on topics without touching the data
//! blob.  Putting `LogLevel` in the topics tuple means a downstream filter of
//! "only WARN and above" costs a single topic comparison — no deserialisation
//! required.  It also avoids the `#[contracttype]` ambiguous-associated-item
//! compiler warning that arises when an enum variant name clashes with a
//! struct field name of the same type (Rust issue #57644).
//!
//! ### Topic layout per event
//!
//! ```text
//! topic[0]  LogLevel  (always)
//! topic[1]  Symbol    (event category, e.g. "escrow", "fee", "batch")
//! topic[2]  Symbol    (event name,     e.g. "init",  "locked")
//! topic[3]  u64       (bounty_id where applicable)
//! ```
//!
//! ### Mapping of existing events to levels
//!
//! | Emit function            | Level |
//! |--------------------------|-------|
//! | `emit_bounty_initialized`| Info  |
//! | `emit_funds_locked`      | Info  |
//! | `emit_funds_released`    | Info  |
//! | `emit_funds_refunded`    | Info  |
//! | `emit_approval_added`    | Warn  |
//! | `emit_fee_collected`     | Debug |


// Module-level allow: this file defines the event schema for future logging-level
// support (issue #615). Not all variants/functions are consumed yet; a narrow
// per-item annotation would be noisier than a single module-level note.
#![allow(dead_code)]


use soroban_sdk::{contracttype, symbol_short, Address, Env};

// ============================================================================
// LogLevel — encoded as topics[0] in every event
// ============================================================================

/// Logging level attached as the first topic to every contract event.
///
/// Using `u32` (via `#[repr(u32)]`) keeps the XDR encoding compact and
/// stable across SDK upgrades.  The numeric values are intentionally
/// non-sequential gaps so new levels can be inserted without renumbering.
#[allow(dead_code)] // LogLevel-based filtering API; reserved for future indexer integration
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum LogLevel {
    /// High-frequency detail useful during development or deep debugging.
    Debug = 0,
    /// Normal operational events — the default for state transitions.
    Info  = 1,
    /// Something noteworthy that didn't block execution.
    Warn  = 2,
    /// Severe condition.  Currently reserved; contract panics before emit.
    Error = 3,
}

// ============================================================================
// Event data structs  (fields unchanged from pre-#615 — only topics changed)
// ============================================================================

/// Schema version constant shared with indexers; not yet wired to active module.
#[allow(dead_code)] // Declared in events_log (staging module); not yet added to module tree
pub const EVENT_VERSION_V2: u32 = 2;

/// Payload for the bounty-initialised event.
#[allow(dead_code)] // Declared in events_log (staging module); not yet added to module tree
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BountyEscrowInitialized {
    pub admin: Address,
    pub token: Address,
    pub timestamp: u64,
}

/// Payload for the funds-locked event.
#[allow(dead_code)] // Declared in events_log (staging module); not yet added to module tree
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FundsLocked {
    pub bounty_id: u64,
    pub amount: i128,
    pub depositor: Address,
    pub deadline: u64,
}

/// Payload for the funds-released event.
#[allow(dead_code)] // Declared in events_log (staging module); not yet added to module tree
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FundsReleased {
    pub bounty_id: u64,
    pub amount: i128,
    pub recipient: Address,
    pub timestamp: u64,
}

/// Payload for the funds-refunded event.
#[allow(dead_code)] // Declared in events_log (staging module); not yet added to module tree
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FundsRefunded {
    pub bounty_id: u64,
    pub amount: i128,
    pub refund_to: Address,
    pub timestamp: u64,
    pub refund_mode: crate::RefundMode,
    pub remaining_amount: i128,
}

/// Payload for the multisig-approval-added event.
#[allow(dead_code)] // Declared in events_log (staging module); not yet added to module tree
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovalAdded {
    pub bounty_id: u64,
    pub contributor: Address,
    pub approver: Address,
    pub timestamp: u64,
}

/// Discriminant for fee operation events.
#[allow(dead_code)] // Declared in events_log (staging module); not yet added to module tree
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FeeOperationType {
    Lock,
    Release,
}

/// Payload for the fee-collected event.
#[allow(dead_code)] // Declared in events_log (staging module); not yet added to module tree
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeCollected {
    pub operation_type: FeeOperationType,
    pub amount: i128,
    pub fee_rate: i128,
    /// Fixed fee component (token smallest units) applied in addition to percentage.
    pub fee_fixed: i128,
    pub recipient: Address,
    pub timestamp: u64,
}

// ============================================================================
// Emit helpers — LogLevel is topics[0], everything else follows
// ============================================================================
//
// Topic layout:  (LogLevel, category_symbol, event_symbol [, bounty_id])
// Payload:       the typed struct above
//
// This layout lets indexers filter cheaply:
//   - "all WARN+"    → topic[0] >= LogLevel::Warn
//   - "all escrow events" → topic[1] == "escrow"
//   - "this bounty"  → topic[3] == bounty_id  (where present)

/// Contract initialised — INFO, happens once per deployment.
#[allow(dead_code)] // Declared in events_log (staging module); not yet added to module tree
pub fn emit_bounty_initialized(env: &Env, data: BountyEscrowInitialized) {
    env.events().publish(
        (LogLevel::Info, symbol_short!("escrow"), symbol_short!("init")),
        data,
    );
}

/// Funds successfully locked into escrow — INFO.
#[allow(dead_code)] // Declared in events_log (staging module); not yet added to module tree
pub fn emit_funds_locked(env: &Env, data: FundsLocked) {
    env.events().publish(
        (
            LogLevel::Info,
            symbol_short!("escrow"),
            symbol_short!("locked"),
            data.bounty_id,
        ),
        data,
    );
}

/// Funds released to contributor — INFO.
#[allow(dead_code)] // Declared in events_log (staging module); not yet added to module tree
pub fn emit_funds_released(env: &Env, data: FundsReleased) {
    env.events().publish(
        (
            LogLevel::Info,
            symbol_short!("escrow"),
            symbol_short!("released"),
            data.bounty_id,
        ),
        data,
    );
}

/// Funds refunded to depositor — INFO.
#[allow(dead_code)] // Declared in events_log (staging module); not yet added to module tree
pub fn emit_funds_refunded(env: &Env, data: FundsRefunded) {
    env.events().publish(
        (
            LogLevel::Info,
            symbol_short!("escrow"),
            symbol_short!("refunded"),
            data.bounty_id,
        ),
        data,
    );
}

/// Multisig approval recorded — WARN (requires human attention to track
/// whether quorum is being reached).
#[allow(dead_code)] // Declared in events_log (staging module); not yet added to module tree
pub fn emit_approval_added(env: &Env, data: ApprovalAdded) {
    env.events().publish(
        (
            LogLevel::Warn,
            symbol_short!("escrow"),
            symbol_short!("approved"),
            data.bounty_id,
        ),
        data,
    );
}

/// Fee deducted during lock or release — DEBUG (high-frequency, low
/// importance for most consumers).
#[allow(dead_code)] // Declared in events_log (staging module); not yet added to module tree
pub fn emit_fee_collected(env: &Env, data: FeeCollected) {
    env.events().publish(
        (LogLevel::Debug, symbol_short!("fee"), symbol_short!("collect")),
        data,
    );
}
