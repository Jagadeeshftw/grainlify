//! # Multi-Signature Control Module
//!
//! A flexible multi-signature scheme for secure contract management.
//! This module enables N-of-M authorization for critical operations.
//!
//! ## Overview
//!
//! The MultiSig module provides:
//! 1. **Proposal Management**: Queue of actions requiring approval
//! 2. **Threshold Enforcement**: N-of-M signature verification
//! 3. **Execution Tracking**: Prevents double execution of proposals
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                   Multi-Signature Layout                    │
//! ├─────────────────────────────────────────────────────────────┤
//! │                                                             │
//! │  ┌────────────┐   ┌────────────┐   ┌────────────┐           │
//! │  │  Signer 1  │   │  Signer 2  │   │  Signer 3  │           │
//! │  └──────┬─────┘   └──────┬─────┘   └──────┬─────┘           │
//! │         │                │                │                 │
//! │         ▼                ▼                ▼                 │
//! │   approve(ID)       approve(ID)      approve(ID)            │
//! │         │                │                │                 │
//! │         └───────┬────────┴────────┬───────┘                 │
//! │                 │                 │                         │
//! │                 ▼                 ▼                         │
//! │          ┌───────────────────────────────────┐              │
//! │          │        MultiSig Contract          │              │
//! │          │  Threshold: 2 of 3 (Example)      │              │
//! │          └────────────────┬──────────────────┘              │
//! │                           │                                 │
//! │                           ▼                                 │
//! │                   mark_executed()                           │
//! │                           │                                 │
//! │                           ▼                                 │
//! │                   [Action Executed]                         │
//! │                                                             │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Security Model
//!
//! - **Threshold Security**: Compromise of (N-1) keys is safe
//! - **Signer Authority**: Only defined signers can approve
//! - **Execution One-Time**: Replay protection via `executed` flag
//!
//! ## Usage Example
//!
//! ```rust
//! // 1. Propose an action
//! let prop_id = MultiSig::propose(&env, proposer_addr);
//!
//! // 2. Signers approve
//! MultiSig::approve(&env, prop_id, signer1);
//! MultiSig::approve(&env, prop_id, signer2);
//!
//! // 3. Execute check
//! if MultiSig::can_execute(&env, prop_id) {
//!     MultiSig::mark_executed(&env, prop_id);
//!     // Perform action...
//! }
//! ```

use soroban_sdk::{
    contracttype, symbol_short, Address, Env, Vec,
};

/// =======================
/// Storage Keys
/// =======================
#[contracttype]
enum DataKey {
    Config,
    Proposal(u64),
    ProposalCounter,
}

/// =======================
/// Multisig Configuration
/// =======================
#[contracttype]
#[derive(Clone)]
pub struct MultiSigConfig {
    pub signers: Vec<Address>,
    pub threshold: u32,
}

/// =======================
/// Proposal Structure
/// =======================
#[contracttype]
#[derive(Clone)]
pub struct Proposal {
    pub approvals: Vec<Address>,
    pub executed: bool,
}

/// =======================
/// Errors
/// =======================
#[derive(Debug)]
pub enum MultiSigError {
    NotSigner,
    AlreadyApproved,
    ProposalNotFound,
    AlreadyExecuted,
    ThresholdNotMet,
    InvalidThreshold,
}

/// =======================
/// Public API
/// =======================
pub struct MultiSig;

impl MultiSig {
    /// Initialize multisig configuration.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `signers` - List of addresses authorized to sign
    /// * `threshold` - Minimum number of signatures required
    ///
    /// # Panics
    /// * If threshold is 0
    /// * If threshold > number of signers
    ///
    /// # State Changes
    /// - Stores `MultiSigConfig`
    /// - Initializes `ProposalCounter` to 0
    pub fn init(env: &Env, signers: Vec<Address>, threshold: u32) {
        if threshold == 0 || threshold > signers.len() as u32 {
            panic!("{:?}", MultiSigError::InvalidThreshold);
        }

        let config = MultiSigConfig { signers, threshold };
        env.storage().instance().set(&DataKey::Config, &config);
        env.storage()
            .instance()
            .set(&DataKey::ProposalCounter, &0u64);
    }

    /// Create a new proposal requiring multisig approval.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `proposer` - Address creating the proposal (must be a signer)
    ///
    /// # Returns
    /// * `u64` - The unique ID of the created proposal
    ///
    /// # Security Considerations
    /// - Proposer must be in the signer set
    /// - Requires authentication of proposer
    ///
    /// # Events
    /// Emits: `proposal(id)`
    pub fn propose(env: &Env, proposer: Address) -> u64 {
        proposer.require_auth();

        let config = Self::get_config(env);
        Self::assert_signer(&config, &proposer);

        let mut counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::ProposalCounter)
            .unwrap_or(0);

        counter += 1;

        let proposal = Proposal {
            approvals: Vec::new(env),
            executed: false,
        };

        env.storage()
            .instance()
            .set(&DataKey::Proposal(counter), &proposal);
        env.storage()
            .instance()
            .set(&DataKey::ProposalCounter, &counter);

        env.events().publish(
            (symbol_short!("proposal"),),
            counter,
        );

        counter
    }

    /// Approve an existing proposal.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `proposal_id` - ID of the proposal to approve
    /// * `signer` - Address approving the proposal
    ///
    /// # Panics
    /// * If signer not authorized
    /// * If proposal already executed
    /// * If signer already approved
    ///
    /// # Events
    /// Emits: `approved(proposal_id, signer)`
    pub fn approve(env: &Env, proposal_id: u64, signer: Address) {
        signer.require_auth();

        let config = Self::get_config(env);
        Self::assert_signer(&config, &signer);

        let mut proposal = Self::get_proposal(env, proposal_id);

        if proposal.executed {
            panic!("{:?}", MultiSigError::AlreadyExecuted);
        }

        if proposal.approvals.contains(&signer) {
            panic!("{:?}", MultiSigError::AlreadyApproved);
        }

        proposal.approvals.push_back(signer.clone());

        env.storage()
            .instance()
            .set(&DataKey::Proposal(proposal_id), &proposal);

        env.events().publish(
            (symbol_short!("approved"),),
            (proposal_id, signer),
        );
    }

    /// Check if a proposal has met requirements for execution.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `proposal_id` - ID of the proposal to check
    ///
    /// # Returns
    /// * `bool` - True if threshold met and not executed
    pub fn can_execute(env: &Env, proposal_id: u64) -> bool {
        let config = Self::get_config(env);
        let proposal = Self::get_proposal(env, proposal_id);

        !proposal.executed && proposal.approvals.len() >= config.threshold
    }

    /// Mark a proposal as executed.
    ///
    /// This should be called by the consuming contract when performing the action.
    /// It verifies the threshold is met and prevents replay.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `proposal_id` - ID of the proposal being executed
    ///
    /// # Panics
    /// * If already executed
    /// * If threshold not met
    ///
    /// # Events
    /// Emits: `executed(proposal_id)`
    pub fn mark_executed(env: &Env, proposal_id: u64) {
        let mut proposal = Self::get_proposal(env, proposal_id);

        if proposal.executed {
            panic!("{:?}", MultiSigError::AlreadyExecuted);
        }

        if !Self::can_execute(env, proposal_id) {
            panic!("{:?}", MultiSigError::ThresholdNotMet);
        }

        proposal.executed = true;

        env.storage()
            .instance()
            .set(&DataKey::Proposal(proposal_id), &proposal);

        env.events().publish(
            (symbol_short!("executed"),),
            proposal_id,
        );
    }

    /// =======================
    /// Internal Helpers
    /// =======================

    fn get_config(env: &Env) -> MultiSigConfig {
        env.storage()
            .instance()
            .get(&DataKey::Config)
            .expect("multisig not initialized")
    }

    fn get_proposal(env: &Env, proposal_id: u64) -> Proposal {
        env.storage()
            .instance()
            .get(&DataKey::Proposal(proposal_id))
            .unwrap_or_else(|| panic!("{:?}", MultiSigError::ProposalNotFound))
    }

    fn assert_signer(config: &MultiSigConfig, signer: &Address) {
        if !config.signers.contains(signer) {
            panic!("{:?}", MultiSigError::NotSigner);
        }
    }
}


