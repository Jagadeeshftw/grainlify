# Error Mapping Documentation

This document describes how errors from all Grainlify Soroban smart contracts are mapped to typed SDK errors and backend messages.

## Error Flow

```
On-chain contract error
  ├── (numeric code) → parseContractErrorByCode() → Typed ContractError
  └── (panic string) → parseContractError()       → Typed ContractError

Network / RPC error   → NetworkError   → Application
Invalid input         → ValidationError → Application
```

## Error Types

### 1. ValidationError

**When thrown:** Before making any contract call, when input validation fails.

**Properties:**
- `code`: Always `'VALIDATION_ERROR'`
- `field`: The name of the invalid field (optional)
- `message`: Description of what's invalid

### 2. ContractError

**When thrown:** When the smart contract execution fails or returns an error.

**Properties:**
- `code`: One of the `ContractErrorCode` enum values
- `contractErrorCode`: The numeric error code from the contract (optional)
- `message`: Human-readable error description

### 3. NetworkError

**When thrown:** When there are network, transport, or RPC server issues.

**Properties:**
- `code`: Always `'NETWORK_ERROR'`
- `statusCode`: HTTP status code (if applicable)
- `cause`: The original error that caused the network failure
- `message`: Description of the network issue

---

## Complete Contract Error Mapping

### Program-Escrow Contract

| SDK Code | Panic / String Match | Message |
|---|---|---|
| `NOT_INITIALIZED` | "not initialized", "Program not initialized" | Program not initialized |
| `UNAUTHORIZED` | "require_auth", "Unauthorized" | Unauthorized: caller does not have permission |
| `INSUFFICIENT_BALANCE` | "Insufficient balance" | Insufficient balance for this operation |
| `INVALID_AMOUNT` | "must be greater than zero" | Amount must be greater than zero |
| `ALREADY_INITIALIZED` | "already initialized" | Program already initialized |
| `EMPTY_BATCH` | "empty batch" | Cannot process empty batch |
| `LENGTH_MISMATCH` | "same length" | Recipients and amounts vectors must have the same length |
| `OVERFLOW` | "overflow" | Payout amount overflow |

### Bounty-Escrow Contract

Source: `contracts/bounty_escrow/contracts/escrow/src/lib.rs`

| Code | SDK Code | Rust Variant | Message |
|---:|---|---|---|
| 1 | `BOUNTY_ALREADY_INITIALIZED` | AlreadyInitialized | Bounty escrow contract is already initialized |
| 2 | `BOUNTY_NOT_INITIALIZED` | NotInitialized | Bounty escrow contract has not been initialized |
| 3 | `BOUNTY_EXISTS` | BountyExists | A bounty with this ID already exists |
| 4 | `BOUNTY_NOT_FOUND` | BountyNotFound | Bounty not found |
| 5 | `BOUNTY_FUNDS_NOT_LOCKED` | FundsNotLocked | Bounty funds have not been locked yet |
| 6 | `BOUNTY_DEADLINE_NOT_PASSED` | DeadlineNotPassed | Bounty deadline has not passed yet |
| 7 | `BOUNTY_UNAUTHORIZED` | Unauthorized | Unauthorized: caller is not allowed to perform this bounty operation |
| 8 | `BOUNTY_INVALID_FEE_RATE` | InvalidFeeRate | Fee rate is invalid (must be between 0 and 5000 basis points) |
| 9 | `BOUNTY_FEE_RECIPIENT_NOT_SET` | FeeRecipientNotSet | Fee recipient address has not been configured |
| 10 | `BOUNTY_INVALID_BATCH_SIZE` | InvalidBatchSize | Batch size is invalid (must be between 1 and 20) |
| 11 | `BOUNTY_BATCH_SIZE_MISMATCH` | BatchSizeMismatch | Number of bounty IDs does not match the number of recipients |
| 12 | `BOUNTY_DUPLICATE_ID` | DuplicateBountyId | Duplicate bounty ID found in batch |
| 13 | `BOUNTY_INVALID_AMOUNT` | InvalidAmount | Bounty amount is invalid (zero, negative, or exceeds available) |
| 14 | `BOUNTY_INVALID_DEADLINE` | InvalidDeadline | Bounty deadline is invalid (in the past or too far in the future) |
| — | *(gap at 15)* | — | — |
| 16 | `BOUNTY_INSUFFICIENT_FUNDS` | InsufficientFunds | Insufficient funds in the escrow for this operation |
| 17 | `BOUNTY_REFUND_NOT_APPROVED` | RefundNotApproved | Refund has not been approved by an admin |
| 18 | `BOUNTY_FUNDS_PAUSED` | FundsPaused | Bounty fund operations are currently paused |

### Governance Contract

Source: `contracts/grainlify-core/src/governance.rs`

| Code | SDK Code | Rust Variant | Message |
|---:|---|---|---|
| 1 | `GOV_NOT_INITIALIZED` | NotInitialized | Governance contract has not been initialized |
| 2 | `GOV_INVALID_THRESHOLD` | InvalidThreshold | Governance threshold value is invalid |
| 3 | `GOV_THRESHOLD_TOO_LOW` | ThresholdTooLow | Governance threshold is too low |
| 4 | `GOV_INSUFFICIENT_STAKE` | InsufficientStake | Insufficient stake to perform this governance action |
| 5 | `GOV_PROPOSALS_NOT_FOUND` | ProposalsNotFound | No proposals found |
| 6 | `GOV_PROPOSAL_NOT_FOUND` | ProposalNotFound | Proposal not found |
| 7 | `GOV_PROPOSAL_NOT_ACTIVE` | ProposalNotActive | Proposal is not currently active |
| 8 | `GOV_VOTING_NOT_STARTED` | VotingNotStarted | Voting has not started yet for this proposal |
| 9 | `GOV_VOTING_ENDED` | VotingEnded | Voting period has ended for this proposal |
| 10 | `GOV_VOTING_STILL_ACTIVE` | VotingStillActive | Voting is still active; cannot execute proposal yet |
| 11 | `GOV_ALREADY_VOTED` | AlreadyVoted | You have already voted on this proposal |
| 12 | `GOV_PROPOSAL_NOT_APPROVED` | ProposalNotApproved | Proposal has not been approved |
| 13 | `GOV_EXECUTION_DELAY_NOT_MET` | ExecutionDelayNotMet | Execution delay period has not elapsed yet |
| 14 | `GOV_PROPOSAL_EXPIRED` | ProposalExpired | Proposal has expired |

### Circuit-Breaker (Error-Recovery)

Source: `contracts/program-escrow/src/error_recovery.rs`

| Code | SDK Code | Constant | Message |
|---:|---|---|---|
| 0 | *(success)* | ERR_NONE | Operation succeeded |
| 1001 | `CIRCUIT_OPEN` | ERR_CIRCUIT_OPEN | Circuit breaker is open; operation rejected without attempting |
| 1002 | `CIRCUIT_TRANSFER_FAILED` | ERR_TRANSFER_FAILED | Token transfer failed (transient error) |
| 1003 | `CIRCUIT_INSUFFICIENT_BALANCE` | ERR_INSUFFICIENT_BALANCE | Insufficient contract balance for transfer |

---

## Resolving Errors

### By numeric code (preferred when available)

Use `parseContractErrorByCode()` when you have the u32 error discriminant and know which contract produced it:

```typescript
import { parseContractErrorByCode } from '@grainlify/contracts-sdk';

const error = parseContractErrorByCode(4, 'bounty_escrow');
// → ContractError { code: 'BOUNTY_NOT_FOUND', message: 'Bounty not found', contractErrorCode: 4 }
```

### By error message string (fallback)

Use `parseContractError()` when you only have a panic/error message:

```typescript
import { parseContractError } from '@grainlify/contracts-sdk';

const error = parseContractError(new Error('BountyNotFound'));
// → ContractError { code: 'BOUNTY_NOT_FOUND', message: 'Bounty not found' }
```

### In Go backend

```go
import "github.com/jagadeesh/grainlify/backend/internal/errors"

msg := errors.ContractErrorMessage(errors.BountyEscrow, 4)
// → "Bounty not found"

name := errors.ContractErrorName(errors.Governance, 11)
// → "AlreadyVoted"
```

---

## Error Handling Best Practices

### Handle specific error types

```typescript
try {
  await client.singlePayout(recipient, amount, keypair);
} catch (error) {
  if (error instanceof ValidationError) {
    console.error('Invalid input:', error.field, error.message);
  } else if (error instanceof ContractError) {
    switch (error.code) {
      case ContractErrorCode.NOT_INITIALIZED:
        // Initialize the program first
        break;
      case ContractErrorCode.BOUNTY_NOT_FOUND:
        // Bounty doesn't exist
        break;
      case ContractErrorCode.GOV_ALREADY_VOTED:
        // User already voted
        break;
    }
  } else if (error instanceof NetworkError) {
    console.error('Network issue:', error.statusCode);
  }
}
```

### Implement retry logic for network errors

```typescript
async function withRetry<T>(fn: () => Promise<T>, maxRetries = 3): Promise<T> {
  for (let i = 0; i < maxRetries; i++) {
    try {
      return await fn();
    } catch (error) {
      if (error instanceof NetworkError && i < maxRetries - 1) {
        await new Promise(resolve => setTimeout(resolve, 1000 * (i + 1)));
        continue;
      }
      throw error;
    }
  }
  throw new Error('Max retries exceeded');
}
```

---

## Testing

The SDK includes three test suites for error handling:

| Test file | Coverage |
|---|---|
| `src/__tests__/error-handling.test.ts` | Validation errors, original 8 contract error codes, error factory, type hierarchy |
| `src/__tests__/error-mapping.test.ts` | **Complete mapping verification** — every enum value, every numeric look-up table, string parsing for all contracts, cross-layer consistency, count regression guards |
| `src/__tests__/network-errors.test.ts` | Connection errors, HTTP status codes, error properties, retry scenarios |

The backend Go tests are at `internal/errors/contract_errors_test.go` and cover:
- Completeness of every on-chain error code
- Non-empty human-readable messages
- Unknown code fallbacks
- Specific message spot-checks
- Count regression guards

Run SDK tests: `cd contracts/sdk && npx jest`
Run backend tests: `cd backend && go test ./internal/errors/...`

---

## Adding New Errors

When a new error variant is added to any contract:

1. **Contract**: Add the variant to the Rust enum
2. **SDK `errors.ts`**: Add to `ContractErrorCode`, `CONTRACT_ERROR_MESSAGES`, the appropriate `*_ERROR_MAP`, and a string match in `parseContractError()`
3. **Backend `contract_errors.go`**: Add to the appropriate map
4. **Tests**: Update the authoritative discriminant lists and count assertions in both `error-mapping.test.ts` and `contract_errors_test.go`
5. **This document**: Add a row to the relevant table above
