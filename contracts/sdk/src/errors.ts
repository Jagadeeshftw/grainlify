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

/**
 * Specific contract error types based on the program-escrow contract.
 *
 * AMOUNT_BELOW_MIN and AMOUNT_ABOVE_MAX map to the on-chain errors introduced
 * by Issue #62 (configurable min/max amount policy):
 *   Error::AmountBelowMinimum = 8
 *   Error::AmountAboveMaximum = 9
 */
export enum ContractErrorCode {
  NOT_INITIALIZED = 'NOT_INITIALIZED',
  UNAUTHORIZED = 'UNAUTHORIZED',
  INSUFFICIENT_BALANCE = 'INSUFFICIENT_BALANCE',
  INVALID_AMOUNT = 'INVALID_AMOUNT',
  ALREADY_INITIALIZED = 'ALREADY_INITIALIZED',
  EMPTY_BATCH = 'EMPTY_BATCH',
  LENGTH_MISMATCH = 'LENGTH_MISMATCH',
  OVERFLOW = 'OVERFLOW',
  // min/max amount policy enforcement
  AMOUNT_BELOW_MIN = 'AMOUNT_BELOW_MIN',
  AMOUNT_ABOVE_MAX = 'AMOUNT_ABOVE_MAX',
}

/**
 * Factory function to create typed contract errors
 */
export function createContractError(errorCode: ContractErrorCode, details?: string): ContractError {
  const messages: Record<ContractErrorCode, string> = {
    [ContractErrorCode.NOT_INITIALIZED]: 'Program not initialized',
    [ContractErrorCode.UNAUTHORIZED]: 'Unauthorized: caller does not have permission',
    [ContractErrorCode.INSUFFICIENT_BALANCE]: 'Insufficient balance for this operation',
    [ContractErrorCode.INVALID_AMOUNT]: 'Amount must be greater than zero',
    [ContractErrorCode.ALREADY_INITIALIZED]: 'Program already initialized',
    [ContractErrorCode.EMPTY_BATCH]: 'Cannot process empty batch',
    [ContractErrorCode.LENGTH_MISMATCH]: 'Recipients and amounts vectors must have the same length',
    [ContractErrorCode.OVERFLOW]: 'Payout amount overflow',
    // min/max policy
    [ContractErrorCode.AMOUNT_BELOW_MIN]: 'Amount is below the minimum allowed by policy',
    [ContractErrorCode.AMOUNT_ABOVE_MAX]: 'Amount exceeds the maximum allowed by policy',
  };

  const message = details ? `${messages[errorCode]}: ${details}` : messages[errorCode];
  return new ContractError(message, errorCode);
}

/**
 * Parse contract error from Soroban response.
 *
 * Checks are ordered from most-specific to least-specific so that the more
 * descriptive min/max messages are matched before the generic INVALID_AMOUNT
 * fallback.
 */
export function parseContractError(error: any): ContractError {
  // Check for panic messages from the contract
  const errorMessage = error?.message || error?.toString() || 'Unknown contract error';
  
  if (errorMessage.includes('not initialized') || errorMessage.includes('Program not initialized')) {
    return createContractError(ContractErrorCode.NOT_INITIALIZED);
  }
  
  if (errorMessage.includes('require_auth') || errorMessage.includes('Unauthorized')) {
    return createContractError(ContractErrorCode.UNAUTHORIZED);
  }
  
  if (errorMessage.includes('Insufficient balance')) {
    return createContractError(ContractErrorCode.INSUFFICIENT_BALANCE);
  }
  
  // Issue #62 – match min/max policy errors before the generic INVALID_AMOUNT
  // check so the more precise code is returned to callers.
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

  // Generic contract error – preserves the original message for debugging.
  return new ContractError(`Contract error (unknown): ${errorMessage}`, 'CONTRACT_ERROR');
}