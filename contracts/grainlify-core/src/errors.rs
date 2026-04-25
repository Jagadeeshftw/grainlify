//! # Shared Error Registry (Constants)
//!
//! Centralized error code definitions. Contracts should define their own
//! `#[contracterror]` enums using these constants to ensure uniqueness.

// 1-99: Common Errors
pub const ALREADY_INITIALIZED: u32 = 1;
pub const NOT_INITIALIZED: u32 = 2;
pub const UNAUTHORIZED: u32 = 3;
pub const INVALID_AMOUNT: u32 = 4;
pub const INSUFFICIENT_FUNDS: u32 = 5;
pub const DEADLINE_NOT_PASSED: u32 = 6;
pub const INVALID_DEADLINE: u32 = 7;
pub const CONTRACT_DEPRECATED: u32 = 8;
pub const MAINTENANCE_MODE: u32 = 9;
pub const PAUSED: u32 = 10;
pub const OVERFLOW: u32 = 11;
pub const UNDERFLOW: u32 = 12;
pub const INVALID_STATE: u32 = 13;
pub const NOT_PAUSED: u32 = 14;
pub const INVALID_ASSET_ID: u32 = 15;

// 100-199: Governance Errors
pub const THRESHOLD_NOT_MET: u32 = 101;
pub const PROPOSAL_NOT_FOUND: u32 = 102;
pub const INVALID_THRESHOLD: u32 = 103;
pub const THRESHOLD_TOO_LOW: u32 = 104;
pub const INSUFFICIENT_STAKE: u32 = 105;
pub const PROPOSALS_NOT_FOUND: u32 = 106;
pub const PROPOSAL_NOT_ACTIVE: u32 = 107;
pub const VOTING_NOT_STARTED: u32 = 108;
pub const VOTING_ENDED: u32 = 109;
pub const VOTING_STILL_ACTIVE: u32 = 110;
pub const ALREADY_VOTED: u32 = 111;
pub const PROPOSAL_NOT_APPROVED: u32 = 112;
pub const EXECUTION_DELAY_NOT_MET: u32 = 113;
pub const PROPOSAL_EXPIRED: u32 = 114;

// 200-299: Escrow Errors
pub const BOUNTY_EXISTS: u32 = 201;
pub const BOUNTY_NOT_FOUND: u32 = 202;
pub const FUNDS_NOT_LOCKED: u32 = 203;
pub const INVALID_FEE_RATE: u32 = 204;
pub const FEE_RECIPIENT_NOT_SET: u32 = 205;
pub const INVALID_BATCH_SIZE: u32 = 206;
pub const BATCH_SIZE_MISMATCH: u32 = 207;
pub const DUPLICATE_BOUNTY_ID: u32 = 208;
pub const REFUND_NOT_APPROVED: u32 = 209;
pub const AMOUNT_BELOW_MINIMUM: u32 = 210;
pub const AMOUNT_ABOVE_MAXIMUM: u32 = 211;
pub const CLAIM_PENDING: u32 = 212;
pub const TICKET_NOT_FOUND: u32 = 213;
pub const TICKET_ALREADY_USED: u32 = 214;
pub const TICKET_EXPIRED: u32 = 215;
pub const PARTICIPANT_BLOCKED: u32 = 216;
pub const PARTICIPANT_NOT_ALLOWED: u32 = 217;
pub const NOT_ANONYMOUS_ESCROW: u32 = 218;
pub const INVALID_SELECTION_INPUT: u32 = 219;
pub const UPGRADE_SAFETY_CHECK_FAILED: u32 = 220;
pub const BOUNTY_ALREADY_INITIALIZED: u32 = 221;
pub const ANON_REFUND_REQUIRED: u32 = 222;
pub const ANON_RESOLVER_NOT_SET: u32 = 223;
pub const NOT_ANON_VARIANT: u32 = 224;
pub const USE_INFO_V2_FOR_ANON: u32 = 225;
pub const INVALID_LABEL: u32 = 226;
pub const TOO_MANY_LABELS: u32 = 227;
pub const LABEL_NOT_ALLOWED: u32 = 228;

// 300-399: Identity / KYC Errors
pub const INVALID_SIGNATURE: u32 = 301;
pub const CLAIM_EXPIRED: u32 = 302;
pub const UNAUTHORIZED_ISSUER: u32 = 303;
pub const INVALID_CLAIM_FORMAT: u32 = 304;
pub const TRANSACTION_EXCEEDS_LIMIT: u32 = 305;
pub const INVALID_RISK_SCORE: u32 = 306;
pub const INVALID_TIER: u32 = 307;

// 400-499: Program Escrow Errors
pub const PROGRAM_ALREADY_EXISTS: u32 = 401;
pub const DUPLICATE_PROGRAM_ID: u32 = 402;
pub const INVALID_BATCH_SIZE_PROGRAM: u32 = 403;
pub const PROGRAM_NOT_FOUND: u32 = 404;
pub const SCHEDULE_NOT_FOUND: u32 = 405;
pub const ALREADY_RELEASED: u32 = 406;
pub const FUNDS_PAUSED: u32 = 407;
pub const DUPLICATE_SCHEDULE_ID: u32 = 408;

// 1000+: Circuit Breaker
pub const CIRCUIT_OPEN: u32 = 1001;

// ─────────────────────────────────────────────────────────────────────────────
// Shared ContractError enum
//
// Used by program-escrow tests that assert on `grainlify_core::errors::ContractError`.
// Variants map 1-to-1 to the constants above so error codes are stable.
// ─────────────────────────────────────────────────────────────────────────────
use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    // General
    Unauthorized        = 3,
    InvalidAmount       = 4,
    InvalidBatchSize    = 206,
    DuplicateEntry      = 208,
    // Program Escrow
    ProgramAlreadyExists       = 401,
    DuplicateProgramId         = 402,
    InvalidBatchSizeProgram    = 403,
    ProgramNotFound            = 404,
    ScheduleNotFound           = 405,
    AlreadyReleased            = 406,
    FundsPaused                = 407,
    DuplicateScheduleId        = 408,
    // Circuit Breaker
    CircuitOpen         = 1001,
}
