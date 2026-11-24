# Unified Error Registry

This document defines the centralized error code space for all Grainlify smart contracts. By maintaining a unified registry, we ensure that error codes are consistent, non-overlapping across different contracts, and easily mapped to human-readable messages in the SDK and backend.

## Registry Ranges

To prevent collisions between different functional modules, error codes are assigned within specific ranges:

| Range | Module / Kind | Description |
|-------|---------------|-------------|
| 1 - 99 | **Common** | General purpose errors used by all contracts (Initialisation, Auth, State, etc.) |
| 100 - 199 | **Governance** | Errors related to voting, proposals, and thresholds. |
| 200 - 299 | **Escrow** | Errors related to fund locking, bounty management, and payouts. |
| 300 - 399 | **Identity / KYC** | Errors related to claims, signatures, and transaction limits. |
| 400 - 499 | **Program Escrow** | Specific errors for batch program initialisation and management. |
| 1000+ | **External** | Errors from third-party integrations or circuit breakers. |

## Canonical Error Codes

### 1-99: Common Errors
| Code | Name | Description |
|------|------|-------------|
| 1 | `AlreadyInitialized` | Contract is already initialized; init* cannot be called again. |
| 2 | `NotInitialized` | Contract has not been initialized yet. |
| 3 | `Unauthorized` | Caller is not allowed to perform this operation (e.g. not admin). |
| 4 | `InvalidAmount` | Amount is zero, negative, or otherwise invalid. |
| 5 | `InsufficientFunds` | Contract or user has insufficient balance for the operation. |
| 6 | `DeadlineNotPassed` | Requested operation cannot be performed before the deadline. |
| 7 | `InvalidDeadline` | Deadline is in the past or too far in the future. |
| 8 | `ContractDeprecated` | Contract is no longer accepting new operations. |
| 9 | `MaintenanceMode` | Operations are temporarily disabled by the admin. |
| 10 | `Paused` | Contract fund operations are currently paused. |
| 11 | `Overflow` | Numeric result exceeds storage capacity. |
| 12 | `Underflow` | Numeric result is less than zero for unsigned types. |
| 13 | `InvalidState` | Contract is not in the required state for this operation. |
| 14 | `NotPaused` | Operation requires the contract to be paused. |
| 15 | `InvalidAssetId` | Provided asset identifier is malformed or unsupported. |

### 100-199: Governance Errors
| Code | Name | Description |
|------|------|-------------|
| 101 | `ThresholdNotMet` | Required vote/multisig threshold has not been reached. |
| 102 | `ProposalNotFound` | Referenced proposal ID does not exist. |
| 103 | `InvalidThreshold` | Configuration threshold is out of allowed range. |
| 104 | `ThresholdTooLow` | Threshold is below security minimums. |
| 105 | `InsufficientStake` | Caller does not have enough voting power/stake. |
| 106 | `ProposalsNotFound` | Query returned no proposals. |
| 107 | `ProposalNotActive` | Proposal is in a state where action is prohibited. |
| 108 | `VotingNotStarted` | Voting period has not commenced. |
| 109 | `VotingEnded` | Voting period has already closed. |
| 110 | `VotingStillActive` | Cannot execute while voting is still underway. |
| 111 | `AlreadyVoted` | Caller has already cast a vote for this proposal. |
| 112 | `ProposalNotApproved` | Proposal was rejected or failed to reach consensus. |
| 113 | `ExecutionDelayNotMet` | Time-delay between approval and execution has not passed. |
| 114 | `ProposalExpired` | Proposal has exceeded its validity period. |

### 200-299: Escrow Errors
| Code | Name | Description |
|------|------|-------------|
| 201 | `BountyExists` | A bounty with the same ID already exists. |
| 202 | `BountyNotFound` | Referenced bounty ID does not exist. |
| 203 | `FundsNotLocked` | Funds must be locked before this operation. |
| 204 | `InvalidFeeRate` | Fee basis points are invalid (e.g. > 100%). |
| 205 | `FeeRecipientNotSet` | Fee recipient address is missing from configuration. |
| 206 | `InvalidBatchSize` | Batch contains too many or too few items. |
| 207 | `BatchSizeMismatch` | Discrepancy between vectors of IDs and amounts. |
| 208 | `DuplicateBountyId` | Same ID appears multiple times in a batch. |
| 209 | `RefundNotApproved` | Admin approval required before refunding this bounty. |
| 210 | `AmountBelowMinimum` | Requested lock amount is below policy minimum. |
| 211 | `AmountAboveMaximum` | Requested lock amount exceeds policy maximum. |
| 212 | `ClaimPending` | Payout blocked by a pending claim or dispute resolution. |
| 213 | `TicketNotFound` | Referenced claim ticket is invalid. |
| 214 | `TicketAlreadyUsed` | Replay protection: ticket has already been redeemed. |
| 215 | `TicketExpired` | Claim ticket is no longer valid. |
| 216 | `ParticipantBlocked` | Participant is on the blocklist. |
| 217 | `ParticipantNotAllowed` | Participant is not on the allowlist. |
| 218 | `NotAnonymousEscrow` | Expected an anonymous escrow but found standard one. |
| 219 | `InvalidSelectionInput` | Input used for deterministic random selection is invalid. |
| 220 | `UpgradeSafetyCheckFailed` | Pre-flight safety check failed during contract upgrade. |
| 221 | `BountyAlreadyInitialized` | Legacy/Specific: Bounty contract already initialized. |
| 229 | `TicketInvalid` | Claim ticket was revoked and can no longer be redeemed or revoked. |

> **Bounty escrow claim tickets** (`bounty_escrow` `Error` discriminants): `TicketNotFound = 23`, `TicketAlreadyUsed = 24`, `TicketExpired = 25`, `TicketInvalid = 62`. Redemption also returns `Unauthorized` for a non-beneficiary claimer, and issuance returns `InvalidAmount` when the amount exceeds the escrow's unreserved remaining amount.

### 300-399: Identity / KYC Errors
| Code | Name | Description |
|------|------|-------------|
| 301 | `InvalidSignature` | Identity claim signature verification failed. |
| 302 | `ClaimExpired` | Identity claim is too old and must be renewed. |
| 303 | `UnauthorizedIssuer` | Claim was issued by a non-whitelisted source. |
| 304 | `InvalidClaimFormat` | Claim data is malformed or missing required fields. |
| 305 | `TransactionExceedsLimit` | Amount is higher than allowed for the current tier/risk score. |
| 306 | `InvalidRiskScore` | Risk score provided in the claim is invalid or too high. |
| 307 | `InvalidTier` | Requested identity tier level is unrecognized. |

### 400-499: Program Escrow Errors
| Code | Name | Description |
|------|------|-------------|
| 401 | `ProgramAlreadyExists` | A program with the same ID already exists. |
| 402 | `DuplicateProgramId` | Duplicate program ID found in initialisation batch. |
| 403 | `InvalidBatchSizeProgram` | Too many or too few programs in the initialisation batch. |

### 1000+: External & Circuit-Breaker Errors
| Code | Name | Description |
|------|------|-------------|
| 1001 | `CircuitOpen` | Circuit breaker is active; all operations are rejected for safety. |

## Adding New Errors

1.  **Select Range**: Identify the most relevant module for your error.
2.  **Assign Code**: Use the next available integer in that range.
3.  **Update `contracts/grainlify-core/src/errors.rs`**: Add the new variant to the `Error` enum.
4.  **Update Backend**: Add the mapping to `backend/internal/errors/contract_errors.go`.
5.  **Update SDK**: Add the variant to `ContractErrorCode` and its numeric mapping in `contracts/sdk/src/errors.ts`.
6.  **Document**: Update this table with the new code and description.

## Implementation Details

### Rust (On-Chain)
Contracts should use `#[contracterror]` and re-export the unified error type:
```rust
pub use grainlify_core::errors::Error as ContractError;
```

### Go (Backend)
All codes are stored in a single unified map:
```go
var unifiedErrors = map[uint32]contractErrorEntry{
    // ... entries ...
}
```

### TypeScript (SDK)
The SDK uses a unified enum and a global numeric lookup map:
```typescript
export enum ContractErrorCode {
  BOUNTY_EXISTS = 'BOUNTY_EXISTS',
  // ...
}

export const UNIFIED_ERROR_MAP: Record<number, ContractErrorCode> = {
  201: ContractErrorCode.BOUNTY_EXISTS,
  // ...
}
```
