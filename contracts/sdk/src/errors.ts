/**
 * Base error class for all SDK errors
 */
export class SDKError extends Error {
  constructor(
    message: string,
    public readonly code: string,
  ) {
    super(message);
    this.name = "SDKError";
    Object.setPrototypeOf(this, SDKError.prototype);
  }
}

/**
 * Contract-specific errors that map to Soroban contract error codes
 */
export class ContractError extends SDKError {
  constructor(
    message: string,
    code: string,
    public readonly contractErrorCode?: number,
  ) {
    super(message, code);
    this.name = "ContractError";
    Object.setPrototypeOf(this, ContractError.prototype);
  }
}

/**
 * Network and transport-related errors
 */
export class NetworkError extends SDKError {
  constructor(
    message: string,
    public readonly statusCode?: number,
    public readonly cause?: Error,
  ) {
    super(message, "NETWORK_ERROR");
    this.name = "NetworkError";
    Object.setPrototypeOf(this, NetworkError.prototype);
  }
}

/**
 * Validation errors for input parameters
 */
export class ValidationError extends SDKError {
  constructor(
    message: string,
    public readonly field?: string,
  ) {
    super(message, "VALIDATION_ERROR");
    this.name = "ValidationError";
    Object.setPrototypeOf(this, ValidationError.prototype);
  }
}

// ---------------------------------------------------------------------------
// Contract error codes — Unified Registry
// ---------------------------------------------------------------------------

/**
 * Unified enum of every known contract error across all Grainlify contracts.
 *
 * Based on unified registry in contracts/grainlify-core/src/errors.rs
 */
export enum ContractErrorCode {
  // ── 1-99: Common Errors ──────────────────────────────────────────────
  ALREADY_INITIALIZED = "ALREADY_INITIALIZED", // 1
  NOT_INITIALIZED = "NOT_INITIALIZED", // 2
  UNAUTHORIZED = "UNAUTHORIZED", // 3
  INVALID_AMOUNT = "INVALID_AMOUNT", // 4
  INSUFFICIENT_FUNDS = "INSUFFICIENT_FUNDS", // 5
  DEADLINE_NOT_PASSED = "DEADLINE_NOT_PASSED", // 6
  INVALID_DEADLINE = "INVALID_DEADLINE", // 7
  CONTRACT_DEPRECATED = "CONTRACT_DEPRECATED", // 8
  MAINTENANCE_MODE = "MAINTENANCE_MODE", // 9
  PAUSED = "PAUSED", // 10
  OVERFLOW = "OVERFLOW", // 11
  UNDERFLOW = "UNDERFLOW", // 12
  INVALID_STATE = "INVALID_STATE", // 13
  NOT_PAUSED = "NOT_PAUSED", // 14
  INVALID_ASSET_ID = "INVALID_ASSET_ID", // 15
  INSUFFICIENT_BALANCE = "INSUFFICIENT_BALANCE", // 16
  EMPTY_BATCH = "EMPTY_BATCH", // 17
  LENGTH_MISMATCH = "LENGTH_MISMATCH", // 18
  AMOUNT_BELOW_MIN = "AMOUNT_BELOW_MIN", // 19
  AMOUNT_ABOVE_MAX = "AMOUNT_ABOVE_MAX", // 20

  // ── 100-199: Governance Errors ─────────────────────────────────────────
  GOV_THRESHOLD_NOT_MET = "GOV_THRESHOLD_NOT_MET", // 101
  GOV_PROPOSAL_NOT_FOUND = "GOV_PROPOSAL_NOT_FOUND", // 102
  GOV_INVALID_THRESHOLD = "GOV_INVALID_THRESHOLD", // 103
  GOV_THRESHOLD_TOO_LOW = "GOV_THRESHOLD_TOO_LOW", // 104
  GOV_INSUFFICIENT_STAKE = "GOV_INSUFFICIENT_STAKE", // 105
  GOV_PROPOSALS_NOT_FOUND = "GOV_PROPOSALS_NOT_FOUND", // 106
  GOV_PROPOSAL_NOT_ACTIVE = "GOV_PROPOSAL_NOT_ACTIVE", // 107
  GOV_VOTING_NOT_STARTED = "GOV_VOTING_NOT_STARTED", // 108
  GOV_VOTING_ENDED = "GOV_VOTING_ENDED", // 109
  GOV_VOTING_STILL_ACTIVE = "GOV_VOTING_STILL_ACTIVE", // 110
  GOV_ALREADY_VOTED = "GOV_ALREADY_VOTED", // 111
  GOV_PROPOSAL_NOT_APPROVED = "GOV_PROPOSAL_NOT_APPROVED", // 112
  GOV_EXECUTION_DELAY_NOT_MET = "GOV_EXECUTION_DELAY_NOT_MET", // 113
  GOV_PROPOSAL_EXPIRED = "GOV_PROPOSAL_EXPIRED", // 114

  // ── 200-299: Escrow Errors ─────────────────────────────────────────────
  BOUNTY_EXISTS = "BOUNTY_EXISTS", // 201
  BOUNTY_NOT_FOUND = "BOUNTY_NOT_FOUND", // 202
  BOUNTY_FUNDS_NOT_LOCKED = "BOUNTY_FUNDS_NOT_LOCKED", // 203
  BOUNTY_INVALID_FEE_RATE = "BOUNTY_INVALID_FEE_RATE", // 204
  BOUNTY_FEE_RECIPIENT_NOT_SET = "BOUNTY_FEE_RECIPIENT_NOT_SET", // 205
  BOUNTY_INVALID_BATCH_SIZE = "BOUNTY_INVALID_BATCH_SIZE", // 206
  BOUNTY_BATCH_SIZE_MISMATCH = "BOUNTY_BATCH_SIZE_MISMATCH", // 207
  BOUNTY_DUPLICATE_ID = "BOUNTY_DUPLICATE_ID", // 208
  BOUNTY_REFUND_NOT_APPROVED = "BOUNTY_REFUND_NOT_APPROVED", // 209
  BOUNTY_AMOUNT_BELOW_MIN = "BOUNTY_AMOUNT_BELOW_MIN", // 210
  BOUNTY_AMOUNT_ABOVE_MAX = "BOUNTY_AMOUNT_ABOVE_MAX", // 211
  BOUNTY_CLAIM_PENDING = "BOUNTY_CLAIM_PENDING", // 212
  BOUNTY_TICKET_NOT_FOUND = "BOUNTY_TICKET_NOT_FOUND", // 213
  BOUNTY_TICKET_ALREADY_USED = "BOUNTY_TICKET_ALREADY_USED", // 214
  BOUNTY_TICKET_EXPIRED = "BOUNTY_TICKET_EXPIRED", // 215
  BOUNTY_PARTICIPANT_BLOCKED = "BOUNTY_PARTICIPANT_BLOCKED", // 216
  BOUNTY_PARTICIPANT_NOT_ALLOWED = "BOUNTY_PARTICIPANT_NOT_ALLOWED", // 217
  BOUNTY_NOT_ANONYMOUS_ESCROW = "BOUNTY_NOT_ANONYMOUS_ESCROW", // 218
  BOUNTY_INVALID_SELECTION_INPUT = "BOUNTY_INVALID_SELECTION_INPUT", // 219
  BOUNTY_UPGRADE_SAFETY_CHECK_FAILED = "BOUNTY_UPGRADE_SAFETY_CHECK_FAILED", // 220
  BOUNTY_ALREADY_INITIALIZED = "BOUNTY_ALREADY_INITIALIZED", // 221
  BOUNTY_ANON_REFUND_RESOLVE = "BOUNTY_ANON_REFUND_RESOLVE", // 222
  BOUNTY_ANON_RESOLVER_NOT_SET = "BOUNTY_ANON_RESOLVER_NOT_SET", // 223
  BOUNTY_USE_INFO_V2 = "BOUNTY_USE_INFO_V2", // 225
  BOUNTY_FUNDS_PAUSED = "BOUNTY_FUNDS_PAUSED", // 224
  BOUNTY_CAP_NOT_FOUND = "BOUNTY_CAP_NOT_FOUND", // 226
  BOUNTY_CAP_EXPIRED = "BOUNTY_CAP_EXPIRED", // 227
  BOUNTY_CAP_REVOKED = "BOUNTY_CAP_REVOKED", // 228
  BOUNTY_CAP_ACTION_MISMATCH = "BOUNTY_CAP_ACTION_MISMATCH", // 229
  BOUNTY_CAP_AMOUNT_EXCEEDED = "BOUNTY_CAP_AMOUNT_EXCEEDED", // 230
  BOUNTY_CAP_USES_EXHAUSTED = "BOUNTY_CAP_USES_EXHAUSTED", // 231
  BOUNTY_CAP_EXCEEDS_AUTHORITY = "BOUNTY_CAP_EXCEEDS_AUTHORITY", // 232

  // Aliases — share the same string value as base codes so callers may use
  // contract-specific names (e.g. BOUNTY_DEADLINE_NOT_PASSED) interchangeably
  // with the generic names without duplicating messages.
  BOUNTY_DEADLINE_NOT_PASSED = "DEADLINE_NOT_PASSED",
  BOUNTY_INVALID_AMOUNT = "INVALID_AMOUNT",
  BOUNTY_INVALID_DEADLINE = "INVALID_DEADLINE",
  BOUNTY_INSUFFICIENT_FUNDS = "INSUFFICIENT_FUNDS",

  // ── 300-399: Identity / KYC ───────────────────────────────────────────
  IDENTITY_INVALID_SIGNATURE = "IDENTITY_INVALID_SIGNATURE", // 301
  IDENTITY_CLAIM_EXPIRED = "IDENTITY_CLAIM_EXPIRED", // 302
  IDENTITY_UNAUTH_ISSUER = "IDENTITY_UNAUTH_ISSUER", // 303
  IDENTITY_INVALID_FORMAT = "IDENTITY_INVALID_FORMAT", // 304
  IDENTITY_LIMIT_EXCEEDED = "IDENTITY_LIMIT_EXCEEDED", // 305
  IDENTITY_INVALID_RISK = "IDENTITY_INVALID_RISK", // 306

  // ── 400-499: Program Escrow ───────────────────────────────────────────
  PROGRAM_ALREADY_EXISTS = "PROGRAM_ALREADY_EXISTS", // 401
  PROGRAM_DUPLICATE_ID = "PROGRAM_DUPLICATE_ID", // 402
  PROGRAM_INVALID_BATCH_SIZE = "PROGRAM_INVALID_BATCH_SIZE", // 403

  // ── 1000+: Circuit-Breaker ────────────────────────────────────────────
  CIRCUIT_OPEN = "CIRCUIT_OPEN", // 1001
  CIRCUIT_TRANSFER_FAILED = "CIRCUIT_TRANSFER_FAILED", // 1002
  CIRCUIT_INSUFFICIENT_BALANCE = "INSUFFICIENT_BALANCE", // 1003 alias
}

// ---------------------------------------------------------------------------
// Human-readable message table
// ---------------------------------------------------------------------------

const CONTRACT_ERROR_MESSAGES: Record<ContractErrorCode, string> = {
  // Common
  [ContractErrorCode.ALREADY_INITIALIZED]: "Contract already initialized",
  [ContractErrorCode.NOT_INITIALIZED]: "Program not initialized",
  [ContractErrorCode.UNAUTHORIZED]: "Unauthorized",
  [ContractErrorCode.INVALID_AMOUNT]: "Invalid amount",
  [ContractErrorCode.INSUFFICIENT_FUNDS]: "Insufficient funds",
  [ContractErrorCode.DEADLINE_NOT_PASSED]: "Deadline has not passed",
  [ContractErrorCode.INVALID_DEADLINE]: "Invalid deadline",
  [ContractErrorCode.CONTRACT_DEPRECATED]: "Contract deprecated",
  [ContractErrorCode.MAINTENANCE_MODE]: "Maintenance mode active",
  [ContractErrorCode.PAUSED]: "Operation paused",
  [ContractErrorCode.OVERFLOW]: "Numeric overflow",
  [ContractErrorCode.UNDERFLOW]: "Numeric underflow",
  [ContractErrorCode.INVALID_STATE]: "Invalid operation for current state",
  [ContractErrorCode.NOT_PAUSED]: "Operation requires paused state",
  [ContractErrorCode.INVALID_ASSET_ID]: "Invalid asset identifier",
  [ContractErrorCode.INSUFFICIENT_BALANCE]: "Insufficient balance",
  [ContractErrorCode.EMPTY_BATCH]: "Cannot process empty batch",
  [ContractErrorCode.LENGTH_MISMATCH]:
    "Recipients and amounts must have the same length",
  [ContractErrorCode.AMOUNT_BELOW_MIN]: "Amount is below minimum",
  [ContractErrorCode.AMOUNT_ABOVE_MAX]: "Amount exceeds maximum allowed",

  // Governance
  [ContractErrorCode.GOV_THRESHOLD_NOT_MET]: "Threshold not met",
  [ContractErrorCode.GOV_PROPOSAL_NOT_FOUND]: "Proposal not found",
  [ContractErrorCode.GOV_INVALID_THRESHOLD]: "Invalid threshold value",
  [ContractErrorCode.GOV_THRESHOLD_TOO_LOW]: "Threshold too low",
  [ContractErrorCode.GOV_INSUFFICIENT_STAKE]: "Insufficient stake",
  [ContractErrorCode.GOV_PROPOSALS_NOT_FOUND]: "No proposals found",
  [ContractErrorCode.GOV_PROPOSAL_NOT_ACTIVE]: "Proposal not active",
  [ContractErrorCode.GOV_VOTING_NOT_STARTED]: "Voting not started",
  [ContractErrorCode.GOV_VOTING_ENDED]: "Voting period ended",
  [ContractErrorCode.GOV_VOTING_STILL_ACTIVE]: "Voting still active",
  [ContractErrorCode.GOV_ALREADY_VOTED]: "Already voted",
  [ContractErrorCode.GOV_PROPOSAL_NOT_APPROVED]: "Proposal not approved",
  [ContractErrorCode.GOV_EXECUTION_DELAY_NOT_MET]: "Execution delay not met",
  [ContractErrorCode.GOV_PROPOSAL_EXPIRED]: "Proposal expired",

  // Escrow
  [ContractErrorCode.BOUNTY_EXISTS]: "Bounty with this ID already exists",
  [ContractErrorCode.BOUNTY_NOT_FOUND]: "Bounty not found",
  [ContractErrorCode.BOUNTY_FUNDS_NOT_LOCKED]: "Bounty funds not locked",
  [ContractErrorCode.BOUNTY_INVALID_FEE_RATE]: "Invalid fee rate",
  [ContractErrorCode.BOUNTY_FEE_RECIPIENT_NOT_SET]: "Fee recipient not set",
  [ContractErrorCode.BOUNTY_INVALID_BATCH_SIZE]: "Invalid batch size",
  [ContractErrorCode.BOUNTY_BATCH_SIZE_MISMATCH]: "Batch size mismatch",
  [ContractErrorCode.BOUNTY_DUPLICATE_ID]: "Duplicate bounty ID",
  [ContractErrorCode.BOUNTY_REFUND_NOT_APPROVED]:
    "Refund not approved by admin",
  [ContractErrorCode.BOUNTY_AMOUNT_BELOW_MIN]: "Amount below minimum allowed",
  [ContractErrorCode.BOUNTY_AMOUNT_ABOVE_MAX]: "Amount above maximum allowed",
  [ContractErrorCode.BOUNTY_CLAIM_PENDING]: "Claim pending or under dispute",
  [ContractErrorCode.BOUNTY_TICKET_NOT_FOUND]: "Claim ticket not found",
  [ContractErrorCode.BOUNTY_TICKET_ALREADY_USED]: "Ticket already used",
  [ContractErrorCode.BOUNTY_TICKET_EXPIRED]: "Ticket expired",
  [ContractErrorCode.BOUNTY_PARTICIPANT_BLOCKED]: "Participant blocked",
  [ContractErrorCode.BOUNTY_PARTICIPANT_NOT_ALLOWED]:
    "Participant not on allowlist",
  [ContractErrorCode.BOUNTY_NOT_ANONYMOUS_ESCROW]:
    "Not an anonymous escrow variant",
  [ContractErrorCode.BOUNTY_INVALID_SELECTION_INPUT]:
    "Invalid deterministic selection input",
  [ContractErrorCode.BOUNTY_UPGRADE_SAFETY_CHECK_FAILED]:
    "Upgrade safety check failed",
  [ContractErrorCode.BOUNTY_ALREADY_INITIALIZED]:
    "Bounty escrow contract is already initialized",
  [ContractErrorCode.BOUNTY_ANON_REFUND_RESOLVE]:
    "Anonymous refund requires resolution",
  [ContractErrorCode.BOUNTY_ANON_RESOLVER_NOT_SET]:
    "Anonymous resolver address not set",
  [ContractErrorCode.BOUNTY_USE_INFO_V2]:
    "Use get_escrow_info_v2 for anonymous escrows",
  [ContractErrorCode.BOUNTY_FUNDS_PAUSED]: "Funds paused",
  [ContractErrorCode.BOUNTY_CAP_NOT_FOUND]: "Capability token not found",
  [ContractErrorCode.BOUNTY_CAP_EXPIRED]: "Capability token expired",
  [ContractErrorCode.BOUNTY_CAP_REVOKED]: "Capability token revoked",
  [ContractErrorCode.BOUNTY_CAP_ACTION_MISMATCH]: "Capability action mismatch",
  [ContractErrorCode.BOUNTY_CAP_AMOUNT_EXCEEDED]: "Capability amount exceeded",
  [ContractErrorCode.BOUNTY_CAP_USES_EXHAUSTED]: "Capability uses exhausted",
  [ContractErrorCode.BOUNTY_CAP_EXCEEDS_AUTHORITY]:
    "Capability exceeds authority",

  // Identity
  [ContractErrorCode.IDENTITY_INVALID_SIGNATURE]: "Invalid identity signature",
  [ContractErrorCode.IDENTITY_CLAIM_EXPIRED]: "Identity claim has expired",
  [ContractErrorCode.IDENTITY_UNAUTH_ISSUER]: "Unauthorized claim issuer",
  [ContractErrorCode.IDENTITY_INVALID_FORMAT]: "Invalid identity claim format",
  [ContractErrorCode.IDENTITY_LIMIT_EXCEEDED]:
    "Transaction exceeds identity limits",
  [ContractErrorCode.IDENTITY_INVALID_RISK]: "Invalid risk score/tier",

  // Program Escrow
  [ContractErrorCode.PROGRAM_ALREADY_EXISTS]:
    "Program with this ID already exists",
  [ContractErrorCode.PROGRAM_DUPLICATE_ID]: "Duplicate program ID in batch",
  [ContractErrorCode.PROGRAM_INVALID_BATCH_SIZE]:
    "Invalid batch size for program init",

  // Circuit Breaker
  [ContractErrorCode.CIRCUIT_OPEN]: "Circuit breaker is open",
  [ContractErrorCode.CIRCUIT_TRANSFER_FAILED]: "Token transfer failed",
};

// ---------------------------------------------------------------------------
// Numeric code → ContractErrorCode look-up table (One for all)
// ---------------------------------------------------------------------------

export const UNIFIED_ERROR_MAP: Record<number, ContractErrorCode> = {
  // Common
  1: ContractErrorCode.ALREADY_INITIALIZED,
  2: ContractErrorCode.NOT_INITIALIZED,
  3: ContractErrorCode.UNAUTHORIZED,
  4: ContractErrorCode.INVALID_AMOUNT,
  5: ContractErrorCode.INSUFFICIENT_FUNDS,
  6: ContractErrorCode.DEADLINE_NOT_PASSED,
  7: ContractErrorCode.INVALID_DEADLINE,
  8: ContractErrorCode.CONTRACT_DEPRECATED,
  9: ContractErrorCode.MAINTENANCE_MODE,
  10: ContractErrorCode.PAUSED,
  11: ContractErrorCode.OVERFLOW,
  12: ContractErrorCode.UNDERFLOW,
  13: ContractErrorCode.INVALID_STATE,
  14: ContractErrorCode.NOT_PAUSED,
  15: ContractErrorCode.INVALID_ASSET_ID,
  16: ContractErrorCode.INSUFFICIENT_FUNDS,
  18: ContractErrorCode.BOUNTY_FUNDS_PAUSED,
  21: ContractErrorCode.NOT_PAUSED,
  22: ContractErrorCode.BOUNTY_CLAIM_PENDING,
  23: ContractErrorCode.BOUNTY_TICKET_NOT_FOUND,
  24: ContractErrorCode.INVALID_STATE,
  25: ContractErrorCode.INVALID_STATE,
  26: ContractErrorCode.BOUNTY_CAP_NOT_FOUND,
  27: ContractErrorCode.BOUNTY_CAP_EXPIRED,
  28: ContractErrorCode.BOUNTY_CAP_REVOKED,
  29: ContractErrorCode.BOUNTY_CAP_ACTION_MISMATCH,
  30: ContractErrorCode.BOUNTY_CAP_AMOUNT_EXCEEDED,
  31: ContractErrorCode.BOUNTY_CAP_USES_EXHAUSTED,
  32: ContractErrorCode.BOUNTY_CAP_EXCEEDS_AUTHORITY,

  // Governance
  101: ContractErrorCode.GOV_THRESHOLD_NOT_MET,
  102: ContractErrorCode.GOV_PROPOSAL_NOT_FOUND,
  103: ContractErrorCode.GOV_INVALID_THRESHOLD,
  104: ContractErrorCode.GOV_THRESHOLD_TOO_LOW,
  105: ContractErrorCode.GOV_INSUFFICIENT_STAKE,
  106: ContractErrorCode.GOV_PROPOSALS_NOT_FOUND,
  107: ContractErrorCode.GOV_PROPOSAL_NOT_ACTIVE,
  108: ContractErrorCode.GOV_VOTING_NOT_STARTED,
  109: ContractErrorCode.GOV_VOTING_ENDED,
  110: ContractErrorCode.GOV_VOTING_STILL_ACTIVE,
  111: ContractErrorCode.GOV_ALREADY_VOTED,
  112: ContractErrorCode.GOV_PROPOSAL_NOT_APPROVED,
  113: ContractErrorCode.GOV_EXECUTION_DELAY_NOT_MET,
  114: ContractErrorCode.GOV_PROPOSAL_EXPIRED,

  // Escrow
  201: ContractErrorCode.BOUNTY_EXISTS,
  202: ContractErrorCode.BOUNTY_NOT_FOUND,
  203: ContractErrorCode.BOUNTY_FUNDS_NOT_LOCKED,
  204: ContractErrorCode.BOUNTY_INVALID_FEE_RATE,
  205: ContractErrorCode.BOUNTY_FEE_RECIPIENT_NOT_SET,
  206: ContractErrorCode.BOUNTY_INVALID_BATCH_SIZE,
  207: ContractErrorCode.BOUNTY_BATCH_SIZE_MISMATCH,
  208: ContractErrorCode.BOUNTY_DUPLICATE_ID,
  209: ContractErrorCode.BOUNTY_REFUND_NOT_APPROVED,
  210: ContractErrorCode.BOUNTY_AMOUNT_BELOW_MIN,
  211: ContractErrorCode.BOUNTY_AMOUNT_ABOVE_MAX,
  212: ContractErrorCode.BOUNTY_CLAIM_PENDING,
  213: ContractErrorCode.BOUNTY_TICKET_NOT_FOUND,
  214: ContractErrorCode.BOUNTY_TICKET_ALREADY_USED,
  215: ContractErrorCode.BOUNTY_TICKET_EXPIRED,
  216: ContractErrorCode.BOUNTY_PARTICIPANT_BLOCKED,
  217: ContractErrorCode.BOUNTY_PARTICIPANT_NOT_ALLOWED,
  218: ContractErrorCode.BOUNTY_NOT_ANONYMOUS_ESCROW,
  219: ContractErrorCode.BOUNTY_INVALID_SELECTION_INPUT,
  220: ContractErrorCode.BOUNTY_UPGRADE_SAFETY_CHECK_FAILED,
  221: ContractErrorCode.BOUNTY_ALREADY_INITIALIZED,
  222: ContractErrorCode.BOUNTY_ANON_REFUND_RESOLVE,
  223: ContractErrorCode.BOUNTY_ANON_RESOLVER_NOT_SET,
  225: ContractErrorCode.BOUNTY_USE_INFO_V2,

  // Identity
  301: ContractErrorCode.IDENTITY_INVALID_SIGNATURE,
  302: ContractErrorCode.IDENTITY_CLAIM_EXPIRED,
  303: ContractErrorCode.IDENTITY_UNAUTH_ISSUER,
  304: ContractErrorCode.IDENTITY_INVALID_FORMAT,
  305: ContractErrorCode.IDENTITY_LIMIT_EXCEEDED,
  306: ContractErrorCode.IDENTITY_INVALID_RISK,

  // Program Escrow
  401: ContractErrorCode.PROGRAM_ALREADY_EXISTS,
  402: ContractErrorCode.PROGRAM_DUPLICATE_ID,
  403: ContractErrorCode.PROGRAM_INVALID_BATCH_SIZE,

  // Circuit Breaker
  1001: ContractErrorCode.CIRCUIT_OPEN,
  1002: ContractErrorCode.CIRCUIT_TRANSFER_FAILED,
  1003: ContractErrorCode.INSUFFICIENT_BALANCE,
};

export const BOUNTY_ESCROW_ERROR_MAP: Record<number, ContractErrorCode> = {
  1: ContractErrorCode.ALREADY_INITIALIZED,
  2: ContractErrorCode.NOT_INITIALIZED,
  6: ContractErrorCode.DEADLINE_NOT_PASSED,
  7: ContractErrorCode.UNAUTHORIZED,
  8: ContractErrorCode.BOUNTY_INVALID_FEE_RATE,
  9: ContractErrorCode.BOUNTY_FEE_RECIPIENT_NOT_SET,
  10: ContractErrorCode.BOUNTY_INVALID_BATCH_SIZE,
  11: ContractErrorCode.BOUNTY_BATCH_SIZE_MISMATCH,
  12: ContractErrorCode.BOUNTY_DUPLICATE_ID,
  13: ContractErrorCode.INVALID_AMOUNT,
  14: ContractErrorCode.INVALID_DEADLINE,
  16: ContractErrorCode.INSUFFICIENT_FUNDS,
  17: ContractErrorCode.BOUNTY_REFUND_NOT_APPROVED,
  19: ContractErrorCode.BOUNTY_AMOUNT_BELOW_MIN,
  20: ContractErrorCode.BOUNTY_AMOUNT_ABOVE_MAX,
  18: ContractErrorCode.BOUNTY_FUNDS_PAUSED,
  21: ContractErrorCode.NOT_PAUSED,
  22: ContractErrorCode.BOUNTY_CLAIM_PENDING,
  23: ContractErrorCode.BOUNTY_TICKET_NOT_FOUND,
  24: ContractErrorCode.INVALID_STATE,
  25: ContractErrorCode.INVALID_STATE,
  26: ContractErrorCode.BOUNTY_CAP_NOT_FOUND,
  27: ContractErrorCode.BOUNTY_CAP_EXPIRED,
  28: ContractErrorCode.BOUNTY_CAP_REVOKED,
  29: ContractErrorCode.BOUNTY_CAP_ACTION_MISMATCH,
  30: ContractErrorCode.BOUNTY_CAP_AMOUNT_EXCEEDED,
  31: ContractErrorCode.BOUNTY_CAP_USES_EXHAUSTED,
  32: ContractErrorCode.BOUNTY_CAP_EXCEEDS_AUTHORITY,
  33: ContractErrorCode.INVALID_ASSET_ID,
  34: ContractErrorCode.CONTRACT_DEPRECATED,
  35: ContractErrorCode.BOUNTY_PARTICIPANT_BLOCKED,
  36: ContractErrorCode.BOUNTY_PARTICIPANT_NOT_ALLOWED,
  37: ContractErrorCode.BOUNTY_USE_INFO_V2,
  39: ContractErrorCode.BOUNTY_ANON_REFUND_RESOLVE,
  40: ContractErrorCode.BOUNTY_ANON_RESOLVER_NOT_SET,
  41: ContractErrorCode.BOUNTY_NOT_ANONYMOUS_ESCROW,
  43: ContractErrorCode.BOUNTY_UPGRADE_SAFETY_CHECK_FAILED,
  44: ContractErrorCode.INVALID_STATE,
  45: ContractErrorCode.INVALID_STATE,
  46: ContractErrorCode.INVALID_STATE,
  47: ContractErrorCode.INVALID_STATE,
  48: ContractErrorCode.INVALID_STATE,
  49: ContractErrorCode.INVALID_STATE,
  50: ContractErrorCode.INVALID_STATE,
  51: ContractErrorCode.INVALID_STATE,
  52: ContractErrorCode.INVALID_STATE,
  53: ContractErrorCode.INVALID_STATE,
  54: ContractErrorCode.INVALID_STATE,
  55: ContractErrorCode.INVALID_STATE,
  56: ContractErrorCode.INVALID_STATE,
  57: ContractErrorCode.BOUNTY_FUNDS_NOT_LOCKED,
  58: ContractErrorCode.INVALID_STATE,
  59: ContractErrorCode.INVALID_STATE,
  60: ContractErrorCode.INVALID_STATE,
  61: ContractErrorCode.INVALID_STATE,
  201: ContractErrorCode.BOUNTY_EXISTS,
  202: ContractErrorCode.BOUNTY_NOT_FOUND,
  203: ContractErrorCode.BOUNTY_FUNDS_NOT_LOCKED,
};

export const GOVERNANCE_ERROR_MAP: Record<number, ContractErrorCode> = {
  1: ContractErrorCode.NOT_INITIALIZED,
  2: ContractErrorCode.GOV_INVALID_THRESHOLD,
  3: ContractErrorCode.GOV_THRESHOLD_TOO_LOW,
  4: ContractErrorCode.GOV_INSUFFICIENT_STAKE,
  5: ContractErrorCode.GOV_PROPOSALS_NOT_FOUND,
  6: ContractErrorCode.GOV_PROPOSAL_NOT_FOUND,
  7: ContractErrorCode.GOV_PROPOSAL_NOT_ACTIVE,
  8: ContractErrorCode.GOV_VOTING_NOT_STARTED,
  9: ContractErrorCode.GOV_VOTING_ENDED,
  10: ContractErrorCode.GOV_VOTING_STILL_ACTIVE,
  11: ContractErrorCode.GOV_ALREADY_VOTED,
  12: ContractErrorCode.GOV_PROPOSAL_NOT_APPROVED,
  13: ContractErrorCode.GOV_EXECUTION_DELAY_NOT_MET,
  14: ContractErrorCode.GOV_PROPOSAL_EXPIRED,
  15: ContractErrorCode.INSUFFICIENT_BALANCE,
  16: ContractErrorCode.UNAUTHORIZED,
  17: ContractErrorCode.INVALID_STATE,
  18: ContractErrorCode.INVALID_STATE,
  100: ContractErrorCode.UNAUTHORIZED,
  101: ContractErrorCode.INVALID_STATE,
  102: ContractErrorCode.PAUSED,
  103: ContractErrorCode.INVALID_STATE,
  104: ContractErrorCode.INVALID_STATE,
  105: ContractErrorCode.ALREADY_INITIALIZED,
};

export const CIRCUIT_BREAKER_ERROR_MAP: Record<number, ContractErrorCode> = {
  1001: ContractErrorCode.CIRCUIT_OPEN,
  1002: ContractErrorCode.CIRCUIT_TRANSFER_FAILED,
  1003: ContractErrorCode.INSUFFICIENT_BALANCE,
};

/**
 * Resolve a numeric on-chain error code to a typed ContractError.
 */
export function resolveContractError(code: number): ContractError {
  const errorCode = UNIFIED_ERROR_MAP[code];
  if (errorCode) {
    const message = CONTRACT_ERROR_MESSAGES[errorCode];
    return new ContractError(message, errorCode, code);
  }
  return new ContractError(
    `Unknown contract error (code ${code})`,
    "CONTRACT_ERROR",
    code,
  );
}

export function parseContractErrorByCode(
  numericCode: number,
  contract: string,
): ContractError {
  const contractMap = (() => {
    switch (contract) {
      case "bounty_escrow":
        return BOUNTY_ESCROW_ERROR_MAP;
      case "governance":
        return GOVERNANCE_ERROR_MAP;
      case "circuit_breaker":
        return CIRCUIT_BREAKER_ERROR_MAP;
      default:
        return UNIFIED_ERROR_MAP;
    }
  })();

  const errorCode = contractMap[numericCode];
  if (errorCode) {
    return new ContractError(
      CONTRACT_ERROR_MESSAGES[errorCode],
      errorCode,
      numericCode,
    );
  }

  return new ContractError(
    `Unknown contract error (code ${numericCode})`,
    "CONTRACT_ERROR",
    numericCode,
  );
}

export function createContractError(
  errorCode: ContractErrorCode,
  details?: string,
): ContractError {
  const message = details
    ? `${CONTRACT_ERROR_MESSAGES[errorCode]}: ${details}`
    : CONTRACT_ERROR_MESSAGES[errorCode];
  return new ContractError(message, errorCode);
}

export function parseContractError(error: any): ContractError {
  const errorMessage =
    error?.message || error?.toString() || "Unknown contract error";

  // Primary: match Soroban-style short error strings and policy message
  // variants before the generic message table so specific policy errors do not
  // get swallowed by broader escrow message strings.
  for (const [pattern, code] of SOROBAN_ERROR_PATTERNS) {
    if (errorMessage.includes(pattern)) {
      return createContractError(code);
    }
  }

  // Secondary: match by human-readable message substrings
  for (const [codeStr, msg] of Object.entries(CONTRACT_ERROR_MESSAGES)) {
    if (errorMessage.includes(msg)) {
      return createContractError(codeStr as ContractErrorCode);
    }
  }

  // Tertiary: numeric code embedded in Soroban host error string
  if (errorMessage.includes("Error(Contract, ")) {
    const match = errorMessage.match(/Error\(Contract, (\d+)\)/);
    if (match) {
      return resolveContractError(parseInt(match[1], 10));
    }
  }

  return new ContractError(errorMessage, "CONTRACT_ERROR");
}

// ---------------------------------------------------------------------------
// Soroban-style error pattern table
// ---------------------------------------------------------------------------
// Maps the short CamelCase or descriptive strings emitted directly by contract
// panics / host traps to the canonical ContractErrorCode.  Checked in order —
// more-specific patterns must appear before shorter ones that are substrings.
// ---------------------------------------------------------------------------
const SOROBAN_ERROR_PATTERNS: ReadonlyArray<[string, ContractErrorCode]> = [
  // Program-escrow patterns
  ["Program not initialized", ContractErrorCode.NOT_INITIALIZED],
  ["Program already initialized", ContractErrorCode.ALREADY_INITIALIZED],
  ["require_auth failed", ContractErrorCode.UNAUTHORIZED],
  ["Amount must be greater than zero", ContractErrorCode.INVALID_AMOUNT],
  [
    "Recipients and amounts vectors must have the same length",
    ContractErrorCode.LENGTH_MISMATCH,
  ],
  ["Amount below minimum allowed", ContractErrorCode.AMOUNT_BELOW_MIN],
  ["amount is below the minimum", ContractErrorCode.AMOUNT_BELOW_MIN],
  ["below min", ContractErrorCode.AMOUNT_BELOW_MIN],
  ["amount exceeds maximum", ContractErrorCode.AMOUNT_ABOVE_MAX],
  ["above max", ContractErrorCode.AMOUNT_ABOVE_MAX],
  ["Bounty amount is invalid", ContractErrorCode.INVALID_AMOUNT],
  ["Bounty deadline is invalid", ContractErrorCode.INVALID_DEADLINE],
  [
    "Fee recipient address not set",
    ContractErrorCode.BOUNTY_FEE_RECIPIENT_NOT_SET,
  ],
  ["Payout amount overflow", ContractErrorCode.OVERFLOW],
  ["AmountBelowMinimum", ContractErrorCode.AMOUNT_BELOW_MIN],
  ["AmountAboveMaximum", ContractErrorCode.AMOUNT_ABOVE_MAX],

  // Bounty-escrow CamelCase patterns (order: longer/more-specific first)
  ["DuplicateBountyId", ContractErrorCode.BOUNTY_DUPLICATE_ID],
  ["BatchSizeMismatch", ContractErrorCode.BOUNTY_BATCH_SIZE_MISMATCH],
  ["InvalidBatchSize", ContractErrorCode.BOUNTY_INVALID_BATCH_SIZE],
  ["InvalidFeeRate", ContractErrorCode.BOUNTY_INVALID_FEE_RATE],
  ["RefundNotApproved", ContractErrorCode.BOUNTY_REFUND_NOT_APPROVED],
  ["DeadlineNotPassed", ContractErrorCode.DEADLINE_NOT_PASSED],
  ["FundsNotLocked", ContractErrorCode.BOUNTY_FUNDS_NOT_LOCKED],
  ["FundsPaused", ContractErrorCode.BOUNTY_FUNDS_PAUSED],
  ["BountyExists", ContractErrorCode.BOUNTY_EXISTS],
  ["InsufficientFunds", ContractErrorCode.INSUFFICIENT_FUNDS],

  // Governance CamelCase patterns
  ["ExecutionDelayNotMet", ContractErrorCode.GOV_EXECUTION_DELAY_NOT_MET],
  ["ProposalNotApproved", ContractErrorCode.GOV_PROPOSAL_NOT_APPROVED],
  ["VotingStillActive", ContractErrorCode.GOV_VOTING_STILL_ACTIVE],
  ["VotingNotStarted", ContractErrorCode.GOV_VOTING_NOT_STARTED],
  ["VotingEnded", ContractErrorCode.GOV_VOTING_ENDED],
  ["AlreadyVoted", ContractErrorCode.GOV_ALREADY_VOTED],
  ["ProposalNotActive", ContractErrorCode.GOV_PROPOSAL_NOT_ACTIVE],
  ["ProposalNotFound", ContractErrorCode.GOV_PROPOSAL_NOT_FOUND],
  ["ProposalExpired", ContractErrorCode.GOV_PROPOSAL_EXPIRED],
  ["InsufficientStake", ContractErrorCode.GOV_INSUFFICIENT_STAKE],
  ["InvalidThreshold", ContractErrorCode.GOV_INVALID_THRESHOLD],
  ["ThresholdTooLow", ContractErrorCode.GOV_THRESHOLD_TOO_LOW],
];

export function getContractErrorMessage(code: ContractErrorCode): string {
  return CONTRACT_ERROR_MESSAGES[code];
}
