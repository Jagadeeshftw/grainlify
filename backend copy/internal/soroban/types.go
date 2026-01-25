package soroban

import (
	"fmt"
	"time"

	"github.com/stellar/go/xdr"
)

// Network represents the Stellar network (testnet or mainnet)
type Network string

const (
	NetworkTestnet Network = "testnet"
	NetworkMainnet Network = "mainnet"
)

// EscrowStatus represents the status of an escrow
type EscrowStatus string

const (
	EscrowStatusLocked   EscrowStatus = "Locked"
	EscrowStatusReleased EscrowStatus = "Released"
	EscrowStatusRefunded EscrowStatus = "Refunded"
)

// EscrowData represents escrow information from the contract
type EscrowData struct {
	Depositor string       `json:"depositor"`
	Amount    int64        `json:"amount"`
	Status    EscrowStatus `json:"status"`
	Deadline  int64        `json:"deadline"`
}

// ProgramEscrowData represents program escrow information
type ProgramEscrowData struct {
	ProgramID           string `json:"program_id"`
	TotalFunds          int64  `json:"total_funds"`
	RemainingBalance    int64  `json:"remaining_balance"`
	AuthorizedPayoutKey string `json:"authorized_payout_key"`
	TokenAddress        string `json:"token_address"`
}

// TransactionResult represents the result of a transaction submission
type TransactionResult struct {
	Hash      string    `json:"hash"`
	Ledger    uint32    `json:"ledger,omitempty"`
	Status    string    `json:"status"`
	Submitted time.Time `json:"submitted"`
	Confirmed time.Time `json:"confirmed,omitempty"`
}

// ContractAddress represents a Soroban contract address
type ContractAddress struct {
	xdr.ScAddress
}

// String returns the string representation of the contract address
func (ca *ContractAddress) String() string {
	// Convert ScAddress to string representation
	if ca.ContractId != nil {
		return fmt.Sprintf("%x", ca.ContractId[:])
	}
	return ""
}

// RetryConfig configures retry behavior for transactions
type RetryConfig struct {
	MaxRetries      int
	InitialDelay    time.Duration
	MaxDelay        time.Duration
	BackoffMultiplier float64
}

// DefaultRetryConfig returns a default retry configuration
func DefaultRetryConfig() RetryConfig {
	return RetryConfig{
		MaxRetries:        3,
		InitialDelay:      time.Second,
		MaxDelay:          30 * time.Second,
		BackoffMultiplier: 2.0,
	}
}
