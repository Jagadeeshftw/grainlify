//! # Program Escrow Events Module
//!
//! This module defines all events emitted by the Program Escrow contract.
//! Events provide an audit trail and enable off-chain indexing for monitoring
//! program prize distributions and fund management.
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
//! - Primary index: program_id (in topic for O(1) lookups)
//! - Secondary indexes: recipient, payout_key, timestamp
//! - Full-text search: event_type, contract_address

use soroban_sdk::{contracttype, symbol_short, Address, Env, String};

// ============================================================================
// Program Registration Event
// ============================================================================

/// Event emitted when a program is registered/initialized
#[contracttype]
#[derive(Clone, Debug)]
pub struct ProgramRegistered {
    pub program_id: String,
    pub authorized_payout_key: Address,
    pub token_address: Address,
    pub timestamp: u64,
    pub version: u32,
    pub contract_version: String,
}

pub fn emit_program_registered(env: &Env, event: ProgramRegistered) {
    let topics = (symbol_short!("ProgReg"),);
    env.events().publish(topics, event.clone());
}

// ============================================================================
// Funds Locked Event
// ============================================================================

/// Event emitted when funds are locked in a program
#[contracttype]
#[derive(Clone, Debug)]
pub struct FundsLocked {
    pub program_id: String,
    pub amount: i128,
    pub total_funds: i128,
    pub remaining_balance: i128,
    pub timestamp: u64,
    pub version: u32,
    pub metadata: String,
}

pub fn emit_funds_locked(env: &Env, event: FundsLocked) {
    let topics = (symbol_short!("FundsLock"),);
    env.events().publish(topics, event.clone());
}

// ============================================================================
// Payout Events
// ============================================================================

/// Event emitted for a single payout
#[contracttype]
#[derive(Clone, Debug)]
pub struct PayoutEvent {
    pub program_id: String,
    pub recipient: Address,
    pub amount: i128,
    pub remaining_balance: i128,
    pub timestamp: u64,
    pub version: u32,
    pub metadata: String,
}

pub fn emit_payout(env: &Env, event: PayoutEvent) {
    let topics = (symbol_short!("Payout"),);
    env.events().publish(topics, event.clone());
}

/// Event emitted for batch payouts
#[contracttype]
#[derive(Clone, Debug)]
pub struct BatchPayoutEvent {
    pub program_id: String,
    pub recipient_count: u32,
    pub total_amount: i128,
    pub remaining_balance: i128,
    pub timestamp: u64,
    pub version: u32,
    pub batch_id: String,
}

pub fn emit_batch_payout(env: &Env, event: BatchPayoutEvent) {
    let topics = (symbol_short!("BatchPay"),);
    env.events().publish(topics, event.clone());
}

// ============================================================================
// Event Indexing Support
// ============================================================================

/// Event index record for efficient querying
#[contracttype]
#[derive(Clone, Debug)]
pub struct EventIndex {
    pub event_type: EventType,
    pub program_id: String,
    pub address: Address,
    pub timestamp: u64,
    pub amount: i128,
    pub block_height: u32,
}

/// Event types for indexing
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EventType {
    ProgramRegistered,
    FundsLocked,
    Payout,
    BatchPayout,
}

/// Query filter for event searches
#[contracttype]
#[derive(Clone, Debug)]
pub struct EventFilter {
    pub event_types: Option<soroban_sdk::Vec<EventType>>,
    pub program_id: Option<String>,
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
