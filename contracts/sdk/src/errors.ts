/**
 * Base error class for all SDK errors
 */
export class SDKError extends Error {
  constructor(message: string, public readonly code: string) {
    super(message);
    this.name = 'SDKError';
    Object.setPrototypeOf(this, SDKError.prototype);
  }
}

/**
 * Contract-specific errors that map to Soroban contract error codes
 */
export class ContractError extends SDKError {
  constructor(message: string, code: string, public readonly contractErrorCode?: number) {
    super(message, code);
    this.name = 'ContractError';
    Object.setPrototypeOf(this, ContractError.prototype);
  }
}

/**
 * Network and transport-related errors
 */
export class NetworkError extends SDKError {
  constructor(message: string, public readonly statusCode?: number, public readonly cause?: Error) {
    super(message, 'NETWORK_ERROR');
    this.name = 'NetworkError';
    Object.setPrototypeOf(this, NetworkError.prototype);
  }
}

/**
 * Validation errors for input parameters
 */
export class ValidationError extends SDKError {
  constructor(message: string, public readonly field?: string) {
    super(message, 'VALIDATION_ERROR');
    this.name = 'ValidationError';
    Object.setPrototypeOf(this, ValidationError.prototype);
  }
}

// ---------------------------------------------------------------------------
// Contract error codes — every variant from every on-chain contract
// ---------------------------------------------------------------------------

/**
 * Unified enum of every known contract error across all Grainlify contracts.
 *
 * Naming convention:
 *   - Program-escrow (original SDK) ........... no prefix
 *   - Bounty-escrow ...........................  BOUNTY_*
 *   - Governance ..............................  GOV_*
 *   - Circuit-breaker / error-recovery .......  CIRCUIT_*
 *
 * AMOUNT_BELOW_MIN and AMOUNT_ABOVE_MAX map to the on-chain errors
 *   Error::AmountBelowMinimum = 8
 *   Error::AmountAboveMaximum = 9
 */
export enum ContractErrorCode {
  // ── Program-Escrow (original) ──────────────────────────────────────────
  NOT_INITIALIZED          = 'NOT_INITIALIZED',
  UNAUTHORIZED             = 'UNAUTHORIZED',
  INSUFFICIENT_BALANCE     = 'INSUFFICIENT_BALANCE',
  INVALID_AMOUNT           = 'INVALID_AMOUNT',
  ALREADY_INITIALIZED      = 'ALREADY_INITIALIZED',
  EMPTY_BATCH              = 'EMPTY_BATCH',
  LENGTH_MISMATCH          = 'LENGTH_MISMATCH',
  OVERFLOW                 = 'OVERFLOW',
  AMOUNT_BELOW_MIN         = 'AMOUNT_BELOW_MIN',
  AMOUNT_ABOVE_MAX         = 'AMOUNT_ABOVE_MAX',

  // ── Bounty-Escrow (contracts/bounty_escrow) ────────────────────────────
  BOUNTY_ALREADY_INITIALIZED = 'BOUNTY_ALREADY_INITIALIZED',   // 1
  BOUNTY_NOT_INITIALIZED     = 'BOUNTY_NOT_INITIALIZED',       // 2
  BOUNTY_EXISTS              = 'BOUNTY_EXISTS',                 // 3
  BOUNTY_NOT_FOUND           = 'BOUNTY_NOT_FOUND',             // 4
  BOUNTY_FUNDS_NOT_LOCKED    = 'BOUNTY_FUNDS_NOT_LOCKED',      // 5
  BOUNTY_DEADLINE_NOT_PASSED = 'BOUNTY_DEADLINE_NOT_PASSED',   // 6
  BOUNTY_UNAUTHORIZED        = 'BOUNTY_UNAUTHORIZED',          // 7
  BOUNTY_INVALID_FEE_RATE    = 'BOUNTY_INVALID_FEE_RATE',     // 8
  BOUNTY_FEE_RECIPIENT_NOT_SET = 'BOUNTY_FEE_RECIPIENT_NOT_SET', // 9
  BOUNTY_INVALID_BATCH_SIZE  = 'BOUNTY_INVALID_BATCH_SIZE',   // 10
  BOUNTY_BATCH_SIZE_MISMATCH = 'BOUNTY_BATCH_SIZE_MISMATCH',  // 11
  BOUNTY_DUPLICATE_ID        = 'BOUNTY_DUPLICATE_ID',          // 12
  BOUNTY_INVALID_AMOUNT      = 'BOUNTY_INVALID_AMOUNT',        // 13
  BOUNTY_INVALID_DEADLINE    = 'BOUNTY_INVALID_DEADLINE',      // 14
  BOUNTY_INSUFFICIENT_FUNDS  = 'BOUNTY_INSUFFICIENT_FUNDS',    // 16
  BOUNTY_REFUND_NOT_APPROVED = 'BOUNTY_REFUND_NOT_APPROVED',   // 17
  BOUNTY_FUNDS_PAUSED        = 'BOUNTY_FUNDS_PAUSED',          // 18

  // ── Governance (contracts/grainlify-core/governance) ───────────────────
  GOV_NOT_INITIALIZED        = 'GOV_NOT_INITIALIZED',          // 1
  GOV_INVALID_THRESHOLD      = 'GOV_INVALID_THRESHOLD',        // 2
  GOV_THRESHOLD_TOO_LOW      = 'GOV_THRESHOLD_TOO_LOW',        // 3
  GOV_INSUFFICIENT_STAKE     = 'GOV_INSUFFICIENT_STAKE',       // 4
  GOV_PROPOSALS_NOT_FOUND    = 'GOV_PROPOSALS_NOT_FOUND',      // 5
  GOV_PROPOSAL_NOT_FOUND     = 'GOV_PROPOSAL_NOT_FOUND',       // 6
  GOV_PROPOSAL_NOT_ACTIVE    = 'GOV_PROPOSAL_NOT_ACTIVE',      // 7
  GOV_VOTING_NOT_STARTED     = 'GOV_VOTING_NOT_STARTED',       // 8
  GOV_VOTING_ENDED           = 'GOV_VOTING_ENDED',             // 9
  GOV_VOTING_STILL_ACTIVE    = 'GOV_VOTING_STILL_ACTIVE',      // 10
  GOV_ALREADY_VOTED          = 'GOV_ALREADY_VOTED',            // 11
  GOV_PROPOSAL_NOT_APPROVED  = 'GOV_PROPOSAL_NOT_APPROVED',    // 12
  GOV_EXECUTION_DELAY_NOT_MET = 'GOV_EXECUTION_DELAY_NOT_MET', // 13
  GOV_PROPOSAL_EXPIRED       = 'GOV_PROPOSAL_EXPIRED',         // 14

  // ── Circuit-Breaker / Error-Recovery ────────────────────────────────────
  CIRCUIT_OPEN               = 'CIRCUIT_OPEN',                 // 1001
  CIRCUIT_TRANSFER_FAILED    = 'CIRCUIT_TRANSFER_FAILED',      // 1002
  CIRCUIT_INSUFFICIENT_BALANCE = 'CIRCUIT_INSUFFICIENT_BALANCE', // 1003
}

// ---------------------------------------------------------------------------
// Human-readable message table (one entry per ContractErrorCode)
// ---------------------------------------------------------------------------

const CONTRACT_ERROR_MESSAGES: Record<ContractErrorCode, string> = {
  // Program-Escrow
  [ContractErrorCode.NOT_INITIALIZED]:           'Program not initialized',
  [ContractErrorCode.UNAUTHORIZED]:              'Unauthorized: caller does not have permission',
  [ContractErrorCode.INSUFFICIENT_BALANCE]:      'Insufficient balance for this operation',
  [ContractErrorCode.INVALID_AMOUNT]:            'Amount must be greater than zero',
  [ContractErrorCode.ALREADY_INITIALIZED]:       'Program already initialized',
  [ContractErrorCode.EMPTY_BATCH]:               'Cannot process empty batch',
  [ContractErrorCode.LENGTH_MISMATCH]:           'Recipients and amounts vectors must have the same length',
  [ContractErrorCode.OVERFLOW]:                  'Payout amount overflow',
  [ContractErrorCode.AMOUNT_BELOW_MIN]:          'Amount is below the minimum allowed by policy',
  [ContractErrorCode.AMOUNT_ABOVE_MAX]:          'Amount exceeds the maximum allowed by policy',

  // Bounty-Escrow
  [ContractErrorCode.BOUNTY_ALREADY_INITIALIZED]: 'Bounty escrow contract is already initialized',
  [ContractErrorCode.BOUNTY_NOT_INITIALIZED]:     'Bounty escrow contract has not been initialized',
  [ContractErrorCode.BOUNTY_EXISTS]:              'A bounty with this ID already exists',
  [ContractErrorCode.BOUNTY_NOT_FOUND]:           'Bounty not found',
  [ContractErrorCode.BOUNTY_FUNDS_NOT_LOCKED]:    'Bounty funds have not been locked yet',
  [ContractErrorCode.BOUNTY_DEADLINE_NOT_PASSED]: 'Bounty deadline has not passed yet',
  [ContractErrorCode.BOUNTY_UNAUTHORIZED]:        'Unauthorized: caller is not allowed to perform this bounty operation',
  [ContractErrorCode.BOUNTY_INVALID_FEE_RATE]:   'Fee rate is invalid (must be between 0 and 5000 basis points)',
  [ContractErrorCode.BOUNTY_FEE_RECIPIENT_NOT_SET]: 'Fee recipient address has not been configured',
  [ContractErrorCode.BOUNTY_INVALID_BATCH_SIZE]: 'Batch size is invalid (must be between 1 and 20)',
  [ContractErrorCode.BOUNTY_BATCH_SIZE_MISMATCH]: 'Number of bounty IDs does not match the number of recipients',
  [ContractErrorCode.BOUNTY_DUPLICATE_ID]:        'Duplicate bounty ID found in batch',
  [ContractErrorCode.BOUNTY_INVALID_AMOUNT]:      'Bounty amount is invalid (zero, negative, or exceeds available)',
  [ContractErrorCode.BOUNTY_INVALID_DEADLINE]:    'Bounty deadline is invalid (in the past or too far in the future)',
  [ContractErrorCode.BOUNTY_INSUFFICIENT_FUNDS]:  'Insufficient funds in the escrow for this operation',
  [ContractErrorCode.BOUNTY_REFUND_NOT_APPROVED]: 'Refund has not been approved by an admin',
  [ContractErrorCode.BOUNTY_FUNDS_PAUSED]:        'Bounty fund operations are currently paused',

  // Governance
  [ContractErrorCode.GOV_NOT_INITIALIZED]:        'Governance contract has not been initialized',
  [ContractErrorCode.GOV_INVALID_THRESHOLD]:      'Governance threshold value is invalid',
  [ContractErrorCode.GOV_THRESHOLD_TOO_LOW]:      'Governance threshold is too low',
  [ContractErrorCode.GOV_INSUFFICIENT_STAKE]:     'Insufficient stake to perform this governance action',
  [ContractErrorCode.GOV_PROPOSALS_NOT_FOUND]:    'No proposals found',
  [ContractErrorCode.GOV_PROPOSAL_NOT_FOUND]:     'Proposal not found',
  [ContractErrorCode.GOV_PROPOSAL_NOT_ACTIVE]:    'Proposal is not currently active',
  [ContractErrorCode.GOV_VOTING_NOT_STARTED]:     'Voting has not started yet for this proposal',
  [ContractErrorCode.GOV_VOTING_ENDED]:           'Voting period has ended for this proposal',
  [ContractErrorCode.GOV_VOTING_STILL_ACTIVE]:    'Voting is still active; cannot execute proposal yet',
  [ContractErrorCode.GOV_ALREADY_VOTED]:          'You have already voted on this proposal',
  [ContractErrorCode.GOV_PROPOSAL_NOT_APPROVED]:  'Proposal has not been approved',
  [ContractErrorCode.GOV_EXECUTION_DELAY_NOT_MET]: 'Execution delay period has not elapsed yet',
  [ContractErrorCode.GOV_PROPOSAL_EXPIRED]:       'Proposal has expired',

  // Circuit-Breaker
  [ContractErrorCode.CIRCUIT_OPEN]:               'Circuit breaker is open; operation rejected without attempting',
  [ContractErrorCode.CIRCUIT_TRANSFER_FAILED]:    'Token transfer failed (transient error)',
  [ContractErrorCode.CIRCUIT_INSUFFICIENT_BALANCE]: 'Insufficient contract balance for transfer',
};

// ---------------------------------------------------------------------------
// Numeric code → ContractErrorCode look-up tables (per contract)
// ---------------------------------------------------------------------------

/** Bounty-escrow #[contracterror] discriminants → SDK code */
export const BOUNTY_ESCROW_ERROR_MAP: Record<number, ContractErrorCode> = {
  1:  ContractErrorCode.BOUNTY_ALREADY_INITIALIZED,
  2:  ContractErrorCode.BOUNTY_NOT_INITIALIZED,
  3:  ContractErrorCode.BOUNTY_EXISTS,
  4:  ContractErrorCode.BOUNTY_NOT_FOUND,
  5:  ContractErrorCode.BOUNTY_FUNDS_NOT_LOCKED,
  6:  ContractErrorCode.BOUNTY_DEADLINE_NOT_PASSED,
  7:  ContractErrorCode.BOUNTY_UNAUTHORIZED,
  8:  ContractErrorCode.BOUNTY_INVALID_FEE_RATE,
  9:  ContractErrorCode.BOUNTY_FEE_RECIPIENT_NOT_SET,
  10: ContractErrorCode.BOUNTY_INVALID_BATCH_SIZE,
  11: ContractErrorCode.BOUNTY_BATCH_SIZE_MISMATCH,
  12: ContractErrorCode.BOUNTY_DUPLICATE_ID,
  13: ContractErrorCode.BOUNTY_INVALID_AMOUNT,
  14: ContractErrorCode.BOUNTY_INVALID_DEADLINE,
  16: ContractErrorCode.BOUNTY_INSUFFICIENT_FUNDS,
  17: ContractErrorCode.BOUNTY_REFUND_NOT_APPROVED,
  18: ContractErrorCode.BOUNTY_FUNDS_PAUSED,
};

/** Governance #[contracterror] discriminants → SDK code */
export const GOVERNANCE_ERROR_MAP: Record<number, ContractErrorCode> = {
  1:  ContractErrorCode.GOV_NOT_INITIALIZED,
  2:  ContractErrorCode.GOV_INVALID_THRESHOLD,
  3:  ContractErrorCode.GOV_THRESHOLD_TOO_LOW,
  4:  ContractErrorCode.GOV_INSUFFICIENT_STAKE,
  5:  ContractErrorCode.GOV_PROPOSALS_NOT_FOUND,
  6:  ContractErrorCode.GOV_PROPOSAL_NOT_FOUND,
  7:  ContractErrorCode.GOV_PROPOSAL_NOT_ACTIVE,
  8:  ContractErrorCode.GOV_VOTING_NOT_STARTED,
  9:  ContractErrorCode.GOV_VOTING_ENDED,
  10: ContractErrorCode.GOV_VOTING_STILL_ACTIVE,
  11: ContractErrorCode.GOV_ALREADY_VOTED,
  12: ContractErrorCode.GOV_PROPOSAL_NOT_APPROVED,
  13: ContractErrorCode.GOV_EXECUTION_DELAY_NOT_MET,
  14: ContractErrorCode.GOV_PROPOSAL_EXPIRED,
};

/** Circuit-breaker u32 error constants → SDK code */
export const CIRCUIT_BREAKER_ERROR_MAP: Record<number, ContractErrorCode> = {
  1001: ContractErrorCode.CIRCUIT_OPEN,
  1002: ContractErrorCode.CIRCUIT_TRANSFER_FAILED,
  1003: ContractErrorCode.CIRCUIT_INSUFFICIENT_BALANCE,
};

// ---------------------------------------------------------------------------
// Factory + parsing helpers
// ---------------------------------------------------------------------------

/**
 * Create a typed ContractError from a known error code.
 */
export function createContractError(errorCode: ContractErrorCode, details?: string): ContractError {
  const message = details
    ? `${CONTRACT_ERROR_MESSAGES[errorCode]}: ${details}`
    : CONTRACT_ERROR_MESSAGES[errorCode];
  return new ContractError(message, errorCode);
}

/**
 * Resolve a numeric on-chain error code to a typed ContractError.
 *
 * @param numericCode  The u32 error discriminant from the contract.
 * @param contract     Which contract produced the error — determines the look-up table.
 */
export function parseContractErrorByCode(
  numericCode: number,
  contract: 'bounty_escrow' | 'governance' | 'circuit_breaker',
): ContractError {
  const maps: Record<string, Record<number, ContractErrorCode>> = {
    bounty_escrow:   BOUNTY_ESCROW_ERROR_MAP,
    governance:      GOVERNANCE_ERROR_MAP,
    circuit_breaker: CIRCUIT_BREAKER_ERROR_MAP,
  };

  const errorCode = maps[contract]?.[numericCode];
  if (errorCode) {
    const err = createContractError(errorCode);
    return new ContractError(err.message, err.code, numericCode);
  }

  return new ContractError(
    `Unknown ${contract} contract error (code ${numericCode})`,
    'CONTRACT_ERROR',
    numericCode,
  );
}

/**
 * Parse a contract error from a Soroban response by matching the error message.
 * Falls back to a generic ContractError when no pattern matches.
 *
 * Checks are ordered from most-specific to least-specific so that the more
 * descriptive min/max messages are matched before the generic INVALID_AMOUNT
 * fallback.
 */
export function parseContractError(error: any): ContractError {
  const errorMessage = error?.message || error?.toString() || 'Unknown contract error';

  // ── Program-escrow patterns ────────────────────────────────────────────
  if (errorMessage.includes('not initialized') || errorMessage.includes('Program not initialized')) {
    return createContractError(ContractErrorCode.NOT_INITIALIZED);
  }
  if (errorMessage.includes('require_auth') || errorMessage.includes('Unauthorized')) {
    return createContractError(ContractErrorCode.UNAUTHORIZED);
  }
  if (errorMessage.includes('Insufficient balance')) {
    return createContractError(ContractErrorCode.INSUFFICIENT_BALANCE);
  }
  if (/below.*min(imum)?|AmountBelowMinimum/i.test(errorMessage)) {
    return createContractError(ContractErrorCode.AMOUNT_BELOW_MIN);
  }
  if (/above.*max(imum)?|exceed.*max|AmountAboveMaximum/i.test(errorMessage)) {
    return createContractError(ContractErrorCode.AMOUNT_ABOVE_MAX);
  }
  if (errorMessage.includes('must be greater than zero')) {
    return createContractError(ContractErrorCode.INVALID_AMOUNT);
  }
  if (errorMessage.includes('already initialized')) {
    return createContractError(ContractErrorCode.ALREADY_INITIALIZED);
  }
  if (errorMessage.includes('empty batch')) {
    return createContractError(ContractErrorCode.EMPTY_BATCH);
  }
  if (errorMessage.includes('same length')) {
    return createContractError(ContractErrorCode.LENGTH_MISMATCH);
  }
  if (errorMessage.includes('overflow')) {
    return createContractError(ContractErrorCode.OVERFLOW);
  }

  // ── Bounty-escrow patterns ─────────────────────────────────────────────
  if (errorMessage.includes('BountyExists') || errorMessage.includes('bounty with this ID already exists')) {
    return createContractError(ContractErrorCode.BOUNTY_EXISTS);
  }
  if (errorMessage.includes('BountyNotFound') || errorMessage.includes('Bounty not found')) {
    return createContractError(ContractErrorCode.BOUNTY_NOT_FOUND);
  }
  if (errorMessage.includes('FundsNotLocked') || errorMessage.includes('funds have not been locked')) {
    return createContractError(ContractErrorCode.BOUNTY_FUNDS_NOT_LOCKED);
  }
  if (errorMessage.includes('DeadlineNotPassed') || errorMessage.includes('deadline has not passed')) {
    return createContractError(ContractErrorCode.BOUNTY_DEADLINE_NOT_PASSED);
  }
  if (errorMessage.includes('InvalidFeeRate') || errorMessage.includes('Fee rate is invalid')) {
    return createContractError(ContractErrorCode.BOUNTY_INVALID_FEE_RATE);
  }
  if (errorMessage.includes('FeeRecipientNotSet') || errorMessage.includes('Fee recipient')) {
    return createContractError(ContractErrorCode.BOUNTY_FEE_RECIPIENT_NOT_SET);
  }
  if (errorMessage.includes('InvalidBatchSize') || errorMessage.includes('Batch size is invalid')) {
    return createContractError(ContractErrorCode.BOUNTY_INVALID_BATCH_SIZE);
  }
  if (errorMessage.includes('BatchSizeMismatch')) {
    return createContractError(ContractErrorCode.BOUNTY_BATCH_SIZE_MISMATCH);
  }
  if (errorMessage.includes('DuplicateBountyId') || errorMessage.includes('Duplicate bounty')) {
    return createContractError(ContractErrorCode.BOUNTY_DUPLICATE_ID);
  }
  if (errorMessage.includes('InvalidAmount') || errorMessage.includes('amount is invalid')) {
    return createContractError(ContractErrorCode.BOUNTY_INVALID_AMOUNT);
  }
  if (errorMessage.includes('InvalidDeadline') || errorMessage.includes('deadline is invalid')) {
    return createContractError(ContractErrorCode.BOUNTY_INVALID_DEADLINE);
  }
  if (errorMessage.includes('InsufficientFunds') || errorMessage.includes('Insufficient funds')) {
    return createContractError(ContractErrorCode.BOUNTY_INSUFFICIENT_FUNDS);
  }
  if (errorMessage.includes('RefundNotApproved') || errorMessage.includes('Refund has not been approved')) {
    return createContractError(ContractErrorCode.BOUNTY_REFUND_NOT_APPROVED);
  }
  if (errorMessage.includes('FundsPaused') || errorMessage.includes('funds are currently paused')) {
    return createContractError(ContractErrorCode.BOUNTY_FUNDS_PAUSED);
  }

  // ── Governance patterns ────────────────────────────────────────────────
  if (errorMessage.includes('ProposalNotFound') || errorMessage.includes('Proposal not found')) {
    return createContractError(ContractErrorCode.GOV_PROPOSAL_NOT_FOUND);
  }
  if (errorMessage.includes('ProposalNotActive') || errorMessage.includes('not currently active')) {
    return createContractError(ContractErrorCode.GOV_PROPOSAL_NOT_ACTIVE);
  }
  if (errorMessage.includes('VotingNotStarted') || errorMessage.includes('Voting has not started')) {
    return createContractError(ContractErrorCode.GOV_VOTING_NOT_STARTED);
  }
  if (errorMessage.includes('VotingEnded') || errorMessage.includes('Voting period has ended')) {
    return createContractError(ContractErrorCode.GOV_VOTING_ENDED);
  }
  if (errorMessage.includes('VotingStillActive') || errorMessage.includes('Voting is still active')) {
    return createContractError(ContractErrorCode.GOV_VOTING_STILL_ACTIVE);
  }
  if (errorMessage.includes('AlreadyVoted') || errorMessage.includes('already voted')) {
    return createContractError(ContractErrorCode.GOV_ALREADY_VOTED);
  }
  if (errorMessage.includes('ProposalNotApproved') || errorMessage.includes('not been approved')) {
    return createContractError(ContractErrorCode.GOV_PROPOSAL_NOT_APPROVED);
  }
  if (errorMessage.includes('ExecutionDelayNotMet') || errorMessage.includes('delay period has not elapsed')) {
    return createContractError(ContractErrorCode.GOV_EXECUTION_DELAY_NOT_MET);
  }
  if (errorMessage.includes('ProposalExpired') || errorMessage.includes('Proposal has expired')) {
    return createContractError(ContractErrorCode.GOV_PROPOSAL_EXPIRED);
  }
  if (errorMessage.includes('InsufficientStake') || errorMessage.includes('Insufficient stake')) {
    return createContractError(ContractErrorCode.GOV_INSUFFICIENT_STAKE);
  }
  if (errorMessage.includes('InvalidThreshold') || errorMessage.includes('threshold value is invalid')) {
    return createContractError(ContractErrorCode.GOV_INVALID_THRESHOLD);
  }
  if (errorMessage.includes('ThresholdTooLow') || errorMessage.includes('threshold is too low')) {
    return createContractError(ContractErrorCode.GOV_THRESHOLD_TOO_LOW);
  }

  // ── Circuit-breaker patterns ───────────────────────────────────────────
  if (errorMessage.includes('circuit breaker is open') || errorMessage.includes('Circuit breaker')) {
    return createContractError(ContractErrorCode.CIRCUIT_OPEN);
  }
  if (errorMessage.includes('transfer failed') || errorMessage.includes('Transfer failed')) {
    return createContractError(ContractErrorCode.CIRCUIT_TRANSFER_FAILED);
  }

  // Generic fallback
  return new ContractError(errorMessage, 'CONTRACT_ERROR');
}

/**
 * Retrieve the human-readable message for any ContractErrorCode.
 * Useful for logging and UI display.
 */
export function getContractErrorMessage(code: ContractErrorCode): string {
  return CONTRACT_ERROR_MESSAGES[code];
}
