// Error handling utilities for Grainlify SDK

export class GrainlifyError extends Error {
  public code?: string;

  constructor(message: string, code?: string) {
    super(message);
    this.name = 'GrainlifyError';
    this.code = code;
  }
}

export function handleTransactionResult<T>(tx: any): T {
  if (tx.result.isOk()) {
    return tx.result.unwrap();
  } else {
    throw new GrainlifyError('Transaction failed', tx.result.unwrapErr());
  }
}

export function logError(error: any): void {
  console.error('Grainlify SDK Error:', error.message);
  if (error.code) {
    console.error('Error Code:', error.code);
  }
}