// Package errors provides a centralised mapping from on-chain contract error
// codes to human-readable messages.  This keeps every layer (API handlers,
// background workers, webhooks) consistent in what it reports to callers.
package errors

import "fmt"

// ContractKind identifies which contract produced the error so the same
// numeric code (e.g. 1 = "AlreadyInitialized" in bounty-escrow vs.
// 1 = "NotInitialized" in governance) resolves unambiguously.
type ContractKind string

const (
	BountyEscrow   ContractKind = "bounty_escrow"
	Governance     ContractKind = "governance"
	CircuitBreaker ContractKind = "circuit_breaker"
)

type contractErrorEntry struct {
	Name    string // e.g. "AlreadyInitialized"
	Message string // human-readable explanation
}

// ---------------------------------------------------------------------------
// Bounty-escrow  (contracts/bounty_escrow/contracts/escrow/src/lib.rs)
// #[contracterror] #[repr(u32)]
// ---------------------------------------------------------------------------

var bountyEscrowErrors = map[uint32]contractErrorEntry{
	1:  {"AlreadyInitialized", "Bounty escrow contract is already initialized"},
	2:  {"NotInitialized", "Bounty escrow contract has not been initialized"},
	3:  {"BountyExists", "A bounty with this ID already exists"},
	4:  {"BountyNotFound", "Bounty not found"},
	5:  {"FundsNotLocked", "Bounty funds have not been locked yet"},
	6:  {"DeadlineNotPassed", "Bounty deadline has not passed yet"},
	7:  {"Unauthorized", "Unauthorized: caller is not allowed to perform this bounty operation"},
	8:  {"InvalidFeeRate", "Fee rate is invalid (must be between 0 and 5000 basis points)"},
	9:  {"FeeRecipientNotSet", "Fee recipient address has not been configured"},
	10: {"InvalidBatchSize", "Batch size is invalid (must be between 1 and 20)"},
	11: {"BatchSizeMismatch", "Number of bounty IDs does not match the number of recipients"},
	12: {"DuplicateBountyId", "Duplicate bounty ID found in batch"},
	13: {"InvalidAmount", "Bounty amount is invalid (zero, negative, or exceeds available)"},
	14: {"InvalidDeadline", "Bounty deadline is invalid (in the past or too far in the future)"},
	// 15 is intentionally absent in the contract enum.
	16: {"InsufficientFunds", "Insufficient funds in the escrow for this operation"},
	17: {"RefundNotApproved", "Refund has not been approved by an admin"},
	18: {"FundsPaused", "Bounty fund operations are currently paused"},
}

// ---------------------------------------------------------------------------
// Governance  (contracts/grainlify-core/src/governance.rs)
// #[soroban_sdk::contracterror] #[repr(u32)]
// ---------------------------------------------------------------------------

var governanceErrors = map[uint32]contractErrorEntry{
	1:  {"NotInitialized", "Governance contract has not been initialized"},
	2:  {"InvalidThreshold", "Governance threshold value is invalid"},
	3:  {"ThresholdTooLow", "Governance threshold is too low"},
	4:  {"InsufficientStake", "Insufficient stake to perform this governance action"},
	5:  {"ProposalsNotFound", "No proposals found"},
	6:  {"ProposalNotFound", "Proposal not found"},
	7:  {"ProposalNotActive", "Proposal is not currently active"},
	8:  {"VotingNotStarted", "Voting has not started yet for this proposal"},
	9:  {"VotingEnded", "Voting period has ended for this proposal"},
	10: {"VotingStillActive", "Voting is still active; cannot execute proposal yet"},
	11: {"AlreadyVoted", "You have already voted on this proposal"},
	12: {"ProposalNotApproved", "Proposal has not been approved"},
	13: {"ExecutionDelayNotMet", "Execution delay period has not elapsed yet"},
	14: {"ProposalExpired", "Proposal has expired"},
}

// ---------------------------------------------------------------------------
// Circuit-breaker / error-recovery
// (contracts/program-escrow/src/error_recovery.rs — u32 constants)
// ---------------------------------------------------------------------------

var circuitBreakerErrors = map[uint32]contractErrorEntry{
	0:    {"None", "Operation succeeded"},
	1001: {"CircuitOpen", "Circuit breaker is open; operation rejected without attempting"},
	1002: {"TransferFailed", "Token transfer failed (transient error)"},
	1003: {"InsufficientBalance", "Insufficient contract balance for transfer"},
}

// registry groups all maps by ContractKind.
var registry = map[ContractKind]map[uint32]contractErrorEntry{
	BountyEscrow:   bountyEscrowErrors,
	Governance:     governanceErrors,
	CircuitBreaker: circuitBreakerErrors,
}

// ContractErrorMessage returns a human-readable message for the given
// numeric error code and contract kind.  If the code is unknown it
// returns a descriptive fallback rather than an empty string.
func ContractErrorMessage(kind ContractKind, code uint32) string {
	if m, ok := registry[kind]; ok {
		if entry, ok := m[code]; ok {
			return entry.Message
		}
	}
	return fmt.Sprintf("Unknown %s contract error (code %d)", kind, code)
}

// ContractErrorName returns the Rust enum variant name (e.g.
// "BountyNotFound") for logging and debugging.
func ContractErrorName(kind ContractKind, code uint32) string {
	if m, ok := registry[kind]; ok {
		if entry, ok := m[code]; ok {
			return entry.Name
		}
	}
	return fmt.Sprintf("Unknown(%d)", code)
}

// AllCodes returns every registered numeric code for the given contract.
// Useful for completeness assertions in tests.
func AllCodes(kind ContractKind) []uint32 {
	m := registry[kind]
	codes := make([]uint32, 0, len(m))
	for c := range m {
		codes = append(codes, c)
	}
	return codes
}
