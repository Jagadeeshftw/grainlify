//! Public data structures exposed by the escrow-view-facade contract.
//!
//! These `#[contracttype]` definitions are part of the contract's on-chain
//! data model. They are kept separate from the query adapters ([`crate::query`])
//! and the thin entrypoint ([`crate::EscrowViewFacade`]) so the read-only ABI
//! surface stays easy to audit.

use soroban_sdk::{contracttype, Address, String, Vec};

/// Must match `EscrowStatus` in BountyEscrow.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EscrowStatus {
    Locked,
    Released,
    Refunded,
    PartiallyRefunded,
}

/// A simplified summary of an escrow designed for frontend consumption.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowSummary {
    pub bounty_id: u64,
    pub depositor: Address,
    pub amount: i128,
    pub remaining_amount: i128,
    pub status: EscrowStatus,
    pub deadline: u64,
    pub repo_id: u64,
    pub issue_id: u64,
    pub bounty_type: String,
    pub is_paused: bool,
}

/// A user's aggregated portfolio showing escrows they funded and escrows
/// where they are listed as a beneficiary (if applicable).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserPortfolio {
    /// Escrows funded by this user
    pub as_depositor: Vec<EscrowSummary>,
    /// Escrows where this user is the designated beneficiary/contributor
    pub as_beneficiary: Vec<EscrowSummary>,
}
