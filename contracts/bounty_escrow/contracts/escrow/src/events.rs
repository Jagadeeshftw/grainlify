//! # Bounty Escrow Events Module
//!
//! This module defines all events emitted by the Bounty Escrow contract.
//! Events provide an audit trail and enable off-chain indexing for monitoring
//! bounty lifecycle states.
//!
//! ## Event Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Event Flow Diagram                        │
//! ├─────────────────────────────────────────────────────────────┤
//! │                                                              │
//! │  Contract Init → BountyEscrowInitialized                    │
//! │       ↓                                                      │
//! │  Lock Funds    → FundsLocked                                │
//! │       ↓                                                      │
//! │  ┌──────────┐                                               │
//! │  │ Decision │                                               │
//! │  └────┬─────┘                                               │
//! │       ├─────→ Release → FundsReleased                       │
//! │       └─────→ Refund  → FundsRefunded                       │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Event Versioning
//!
//! All events include a version field to support backward compatibility:
//! - v1: Initial implementation
//! - v2: Added metadata and enhanced indexing
//!
//! ## Indexing Strategy
//!
//! Events are designed for efficient off-chain indexing:
//! - Primary index: bounty_id (in topic for O(1) lookups)
//! - Secondary indexes: depositor, recipient, status, timestamp
//! - Full-text search: event_type, contract_address

use soroban_sdk::{contracttype, symbol_short, Address, Env, String};

// ============================================================================
// Contract Initialization Event
// ============================================================================

/// Event emitted when the Bounty Escrow contract is initialized.
///
/// # Fields
/// * `admin` - The administrator address with release authorization
/// * `token` - The token contract address (typically XLM/USDC)
/// * `timestamp` - Unix timestamp of initialization
/// * `version` - Event schema version (for backward compatibility)
/// * `contract_version` - Contract implementation version
///
/// # Event Topic
/// Symbol: `init`
///
/// # Usage
/// This event is emitted once during contract deployment and signals
/// that the contract is ready to accept bounty escrows.
///
/// # Security Considerations
/// - Only emitted once; subsequent init attempts should fail
/// - Admin address should be a secure backend service
/// - Token address must be a valid Stellar token contract
///
/// # Example Off-chain Indexing
/// ```javascript
/// // Listen for initialization events
/// stellar.events.on('init', (event) => {
///   console.log(`Contract initialized by ${event.admin}`);
///   console.log(`Using token: ${event.token}`);
/// });
/// ```
#[contracttype]
#[derive(Clone, Debug)]
pub struct BountyEscrowInitialized {
    pub admin: Address,
    pub token: Address,
    pub timestamp: u64,
    pub version: u32,
    pub contract_version: String,
}

/// Emits a BountyEscrowInitialized event.
///
/// # Arguments
/// * `env` - The contract environment
/// * `event` - The initialization event data
///
/// # Event Structure
/// Topic: `(symbol_short!("init"),)`
/// Data: Complete `BountyEscrowInitialized` struct
pub fn emit_bounty_initialized(env: &Env, event: BountyEscrowInitialized) {
    let topics = (symbol_short!("init"),);
    env.events().publish(topics, event.clone());
}

// ============================================================================
// Funds Locked Event
// ============================================================================

/// Event emitted when funds are locked in escrow for a bounty.
///
/// # Fields
/// * `bounty_id` - Unique identifier for the bounty
/// * `amount` - Amount of tokens locked (in stroops for XLM)
/// * `depositor` - Address that deposited the funds
/// * `deadline` - Unix timestamp after which refunds are allowed
/// * `timestamp` - Unix timestamp when funds were locked
/// * `version` - Event schema version
/// * `metadata` - Additional context (bounty title, description, etc.)
///
/// # Event Topic
/// Symbol: `f_lock`
/// Indexed: `bounty_id` (allows filtering by specific bounty)
///
/// # State Transition
/// ```text
/// NONE → LOCKED
/// ```
///
/// # Usage
/// Emitted when a bounty creator locks funds for a task. The depositor
/// transfers tokens to the contract, which holds them until release or refund.
///
/// # Security Considerations
/// - Amount must be positive and within depositor's balance
/// - Bounty ID must be unique (no duplicates allowed)
/// - Deadline must be in the future
/// - Depositor must authorize the transaction
///
/// # Example Usage
/// ```rust
/// // Lock 1000 XLM for bounty #42, deadline in 30 days
/// let deadline = env.ledger().timestamp() + (30 * 24 * 60 * 60);
/// escrow_client.lock_funds(&depositor, &42, &10_000_000_000, &deadline);
/// // → Emits FundsLocked event
/// ```
#[contracttype]
#[derive(Clone, Debug)]
pub struct FundsLocked {
    pub bounty_id: u64,
    pub amount: i128,
    pub depositor: Address,
    pub deadline: u64,
    pub timestamp: u64,
    pub version: u32,
    pub metadata: String,
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

/// Event emitted when escrowed funds are released to a contributor.
///
/// # Fields
/// * `bounty_id` - The bounty identifier
/// * `amount` - Amount transferred to recipient
/// * `recipient` - Address receiving the funds (contributor)
/// * `timestamp` - Unix timestamp of release
/// * `depositor` - Original depositor address (for tracking)
/// * `version` - Event schema version
/// * `metadata` - Release notes or reason
///
/// # Event Topic
/// Symbol: `f_rel`
/// Indexed: `bounty_id`
///
/// # State Transition
/// ```text
/// LOCKED → RELEASED (final state)
/// ```
///
/// # Usage
/// Emitted when the admin releases funds to a contributor who completed
/// the bounty task. This is a final, irreversible action.
///
/// # Authorization
/// - Only the contract admin can trigger fund release
/// - Funds must be in LOCKED state
/// - Cannot release funds that were already released or refunded
///
/// # Security Considerations
/// - Admin authorization is critical (should be secure backend)
/// - Recipient address should be verified off-chain before release
/// - Once released, funds cannot be retrieved
/// - Atomic operation: transfer + state update
///
/// # Example Usage
/// ```rust
/// // Admin releases 1000 XLM to contributor for bounty #42
/// escrow_client.release_funds(&42, &contributor_address);
/// // → Transfers tokens
/// // → Updates state to Released
/// // → Emits FundsReleased event
/// ```
#[contracttype]
#[derive(Clone, Debug)]
pub struct FundsReleased {
    pub bounty_id: u64,
    pub amount: i128,
    pub recipient: Address,
    pub timestamp: u64,
    pub depositor: Address,
    pub version: u32,
    pub metadata: String,
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

/// Event emitted when escrowed funds are refunded to the depositor.
///
/// # Fields
/// * `bounty_id` - The bounty identifier
/// * `amount` - Amount refunded to depositor
/// * `refund_to` - Address receiving the refund (original depositor)
/// * `timestamp` - Unix timestamp of refund
///
/// # Event Topic
/// Symbol: `f_ref`
/// Indexed: `bounty_id`
///
/// # State Transition
/// ```text
/// LOCKED → REFUNDED (final state)
/// ```
///
/// # Usage
/// Emitted when funds are returned to the depositor after the deadline
/// has passed without the bounty being completed. This mechanism prevents
/// funds from being locked indefinitely.
///
/// # Conditions
/// - Deadline must have passed (timestamp > deadline)
/// - Funds must still be in LOCKED state
/// - Can be triggered by anyone (permissionless but conditional)
///
/// # Security Considerations
/// - Time-based protection ensures funds aren't stuck
/// - Permissionless refund prevents admin monopoly
/// - Original depositor always receives refund
/// - Cannot refund if already released or refunded
///
/// # Example Usage
/// ```rust
/// // After deadline passes, anyone can trigger refund
/// // Deadline was January 1, 2025
/// // Current time: January 15, 2025
/// escrow_client.refund(&42);
/// // → Transfers tokens back to depositor
/// // → Updates state to Refunded
/// // → Emits FundsRefunded event
/// ```
///
/// # Design Rationale
/// Permissionless refunds ensure that:
/// 1. Depositors don't lose funds if they lose their keys
/// 2. No admin action needed for legitimate refunds
/// 3. System remains trustless and decentralized
#[contracttype]
#[derive(Clone, Debug)]
pub struct FundsRefunded {
    pub bounty_id: u64,
    pub amount: i128,
    pub refund_to: Address,
    pub timestamp: u64,
    pub refund_mode: crate::RefundMode,
    pub remaining_amount: i128,
    pub version: u32,
    pub metadata: String,
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
#[derive(Clone, Debug)]
pub struct BatchFundsLocked {
    pub count: u32,
    pub total_amount: i128,
    pub timestamp: u64,
    pub version: u32,
    pub batch_id: String,
}

pub fn emit_batch_funds_locked(env: &Env, event: BatchFundsLocked) {
    let topics = (symbol_short!("b_lock"),);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct BatchFundsReleased {
    pub count: u32,
    pub total_amount: i128,
    pub timestamp: u64,
    pub version: u32,
    pub batch_id: String,
}

pub fn emit_batch_funds_released(env: &Env, event: BatchFundsReleased) {
    let topics = (symbol_short!("b_rel"),);
    env.events().publish(topics, event.clone());
}

// ============================================================================
// Event Indexing Support
// ============================================================================

/// Event index record for efficient querying
///
/// # Fields
/// * `event_type` - Type of event (Init, Lock, Release, Refund)
/// * `bounty_id` - Bounty identifier (0 for non-bounty events)
/// * `address` - Primary address involved (depositor/recipient)
/// * `timestamp` - Event timestamp for chronological ordering
/// * `amount` - Amount involved in transaction
/// * `status` - Status after event
#[contracttype]
#[derive(Clone, Debug)]
pub struct EventIndex {
    pub event_type: EventType,
    pub bounty_id: u64,
    pub address: Address,
    pub timestamp: u64,
    pub amount: i128,
    pub block_height: u32,
}

/// Event types for indexing
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EventType {
    Initialized,
    Locked,
    Released,
    Refunded,
    BatchLocked,
    BatchReleased,
}

/// Query filter for event searches
#[contracttype]
#[derive(Clone, Debug)]
pub struct EventFilter {
    pub event_types: Option<soroban_sdk::Vec<EventType>>,
    pub bounty_id: Option<u64>,
    pub address: Option<Address>,
    pub from_timestamp: Option<u64>,
    pub to_timestamp: Option<u64>,
    pub min_amount: Option<i128>,
    pub max_amount: Option<i128>,
}

/// Paginated query result
#[contracttype]
#[derive(Clone, Debug)]
pub struct EventQueryResult {
    pub events: soroban_sdk::Vec<EventIndex>,
    pub total_count: u32,
    pub has_more: bool,
    pub next_cursor: Option<u64>,
}

