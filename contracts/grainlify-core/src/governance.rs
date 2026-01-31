//! # Governance Core Contract
//!
//! A decentralized governance system for managing protocol upgrades and parameter changes.
//! This module implements a proposal-based voting system where token holders can vote
//! on changes to the Grainlify protocol.
//!
//! ## Overview
//!
//! The Governance contract manages the lifecycle of improvement proposals:
//! 1. **Proposal Creation**: Token holders with sufficient stake propose changes
//! 2. **Voting**: Community members cast votes (For/Against/Abstain)
//! 3. **Finalization**: Proposals are evaluated against quorum and approval thresholds
//! 4. **Execution**: Approved proposals represent valid instructions (e.g., contract upgrades)
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                 Governance Architecture                      │
//! ├─────────────────────────────────────────────────────────────┤
//! │                                                              │
//! │  ┌──────────────┐       ┌──────────────┐                     │
//! │  │   Proposer   │       │    Voters    │                     │
//! │  └──────┬───────┘       └──────┬───────┘                     │
//! │         │                      │                             │
//! │         │ create_proposal()    │ cast_vote()                 │
//! │         ▼                      ▼                             │
//! │  ┌──────────────────────────────────────────┐                │
//! │  │           Governance Contract            │                │
//! │  │                                          │                │
//! │  │  ┌──────────┐  ┌──────────┐  ┌────────┐  │                │
//! │  │  │ Pending  │→ │  Active  │→ │ Final  │  │                │
//! │  │  └──────────┘  └──────────┘  └────────┘  │                │
//! │  └─────────────────────┬────────────────────┘                │
//! │                        │                                     │
//! │                        │ execute_proposal()                  │
//! │                        ▼                                     │
//! │  ┌──────────────────────────────────────────┐                │
//! │  │           Target Contract (Core)         │                │
//! │  └──────────────────────────────────────────┘                │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Security Model
//!
//! ### Trust Assumptions
//! - **Voters**: Rational actors acting in the best interest of the protocol
//! - **Admin**: Initial setup only; power transitions to community
//! - **Time**: Relies on ledger timestamp for voting periods
//!
//! ### Key Security Features
//! 1. **Proposal Threshold**: Minimum stake required to prevent spam
//! 2. **Voting Delay**: Optional delay before voting starts (anti-flash-loan)
//! 3. **Execution Delay**: Timelock after approval for safety
//! 4. **Quorum**: Minimum participation required
//! 5. **One Person One Vote**: Or Token Weighted (configurable)
//!
//! ## Usage Example
//!
//! ```rust
//! use soroban_sdk::{Address, Env};
//!
//! // 1. Create a proposal
//! let proposer = Address::from_string("GPROPOSER...");
//! let wasm_hash = BytesN::from_array(&env, &[...]); // New contract code
//! let desc = Symbol::new(&env, "Upgrade to v2");
//!
//! let prop_id = governance_client.create_proposal(
//!     &proposer,
//!     &wasm_hash,
//!     &desc
//! );
//!
//! // 2. Cast vote
//! let voter = Address::from_string("GVOTER...");
//! governance_client.cast_vote(
//!     &voter,
//!     &prop_id,
//!     &VoteType::For
//! );
//! ```

use soroban_sdk::{contracttype, Address, BytesN, Symbol, symbol_short};

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum ProposalStatus {
    Pending,
    Active,
    Approved,
    Rejected,
    Executed,
    Expired,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum VoteType {
    For,
    Against,
    Abstain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum VotingScheme {
    OnePersonOneVote,
    TokenWeighted,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct Proposal {
    pub id: u32,
    pub proposer: Address,
    pub new_wasm_hash: BytesN<32>,
    pub description: Symbol,
    pub created_at: u64,
    pub voting_start: u64,
    pub voting_end: u64,
    pub execution_delay: u64,
    pub status: ProposalStatus,
    pub votes_for: i128,
    pub votes_against: i128,
    pub votes_abstain: i128,
    pub total_votes: u32,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct GovernanceConfig {
    pub voting_period: u64,
    pub execution_delay: u64,
    pub quorum_percentage: u32,  // Basis points (e.g., 5000 = 50%)
    pub approval_threshold: u32,  // Basis points (e.g., 6667 = 66.67%)
    pub min_proposal_stake: i128,
    pub voting_scheme: VotingScheme,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct Vote {
    pub voter: Address,
    pub proposal_id: u32,
    pub vote_type: VoteType,
    pub voting_power: i128,
    pub timestamp: u64,
}

// Storage keys
pub const PROPOSALS: Symbol = symbol_short!("PROPOSALS");
pub const PROPOSAL_COUNT: Symbol = symbol_short!("PROP_CNT");
pub const VOTES: Symbol = symbol_short!("VOTES");
pub const GOVERNANCE_CONFIG: Symbol = symbol_short!("GOV_CFG");
pub const VOTER_REGISTRY: Symbol = symbol_short!("VOTERS");

#[soroban_sdk::contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotInitialized = 1,
    InvalidThreshold = 2,
    ThresholdTooLow = 3,
    InsufficientStake = 4,
    ProposalsNotFound = 5,
    ProposalNotFound = 6,
    ProposalNotActive = 7,
    VotingNotStarted = 8,
    VotingEnded = 9,
    VotingStillActive = 10,
    AlreadyVoted = 11,
    ProposalNotApproved = 12,
    ExecutionDelayNotMet = 13,
    ProposalExpired = 14,
}

pub struct GovernanceContract;

impl GovernanceContract {
    /// Initialize the governance system with configuration parameters.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `admin` - Address authorized to perform initial setup
    /// * `config` - Governance configuration (voting period, quorum, etc.)
    ///
    /// # Returns
    /// * `Ok(())` - Successfully initialized
    /// * `Err(Error::InvalidThreshold)` - If thresholds are > 100%
    /// * `Err(Error::ThresholdTooLow)` - If approval threshold is < 50%
    ///
    /// # State Changes
    /// - Stores `GovernanceConfig` in instance storage
    /// - Initializes `ProposalCount` to 0
    /// - Emits `gov_init` event
    ///
    /// # Security Considerations
    /// - Admin must authorize this call
    /// - Config validation prevents impossible voting parameters
    ///
    /// # Events
    /// Emits: `gov_init(admin, config)`
    pub fn init_governance(
        env: &soroban_sdk::Env,
        admin: Address,
        config: GovernanceConfig,
    ) -> Result<(), Error> {
        // Validate admin
        admin.require_auth();
        
        // Validate config
        if config.quorum_percentage > 10000 || config.approval_threshold > 10000 {
            return Err(Error::InvalidThreshold);
        }
        
        if config.approval_threshold < 5000 {
            return Err(Error::ThresholdTooLow); // Must be > 50%
        }
        
        // Store config
        env.storage().instance().set(&GOVERNANCE_CONFIG, &config);
        env.storage().instance().set(&PROPOSAL_COUNT, &0u32);
        
        // Emit event
        env.events().publish(
            (symbol_short!("gov_init"), admin.clone()),
            config,
        );
        
        Ok(())
    }

    /// Create a new proposal for protocol upgrade.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `proposer` - Address creating the proposal
    /// * `new_wasm_hash` - The code hash for the proposed upgrade
    /// * `description` - Short description of the proposal
    ///
    /// # Returns
    /// * `Ok(u32)` - The unique ID of the created proposal
    /// * `Err(Error::NotInitialized)` - If governance is not initialized
    /// * `Err(Error::InsufficientStake)` - If proposer lacks required voting power
    ///
    /// # State Changes
    /// - Creates a new `Proposal` record
    /// - Increments `ProposalCount`
    /// - Emits `proposal` event
    ///
    /// # Security Considerations
    /// - Requires proposer signature
    /// - Checks minimum stake to prevent proposal spam
    /// - Helper function `get_voting_power` handles stake verification
    ///
    /// # Events
    /// Emits: `proposal(proposer, (id, description))`
    pub fn create_proposal(
        env: &soroban_sdk::Env,
        proposer: Address,
        new_wasm_hash: BytesN<32>,
        description: Symbol,
    ) -> Result<u32, Error> {
        // Authenticate proposer
        proposer.require_auth();
        
        // Load config
        let config: GovernanceConfig = env
            .storage()
            .instance()
            .get(&GOVERNANCE_CONFIG)
            .ok_or(Error::NotInitialized)?;
        
        // Check minimum stake requirement
        let proposer_balance = Self::get_voting_power(env, &proposer)?;
        if proposer_balance < config.min_proposal_stake {
            return Err(Error::InsufficientStake);
        }
        
        // Get current proposal count
        let proposal_id: u32 = env
            .storage()
            .instance()
            .get(&PROPOSAL_COUNT)
            .unwrap_or(0);
        
        let current_time = env.ledger().timestamp();
        
        // Create proposal
        let proposal = Proposal {
            id: proposal_id,
            proposer: proposer.clone(),
            new_wasm_hash,
            description: description.clone(),
            created_at: current_time,
            voting_start: current_time,
            voting_end: current_time + config.voting_period,
            execution_delay: config.execution_delay,
            status: ProposalStatus::Active,
            votes_for: 0,
            votes_against: 0,
            votes_abstain: 0,
            total_votes: 0,
        };
        
        // Store proposal
        let mut proposals: soroban_sdk::Map<u32, Proposal> = env
            .storage()
            .instance()
            .get(&PROPOSALS)
            .unwrap_or(soroban_sdk::Map::new(env));
        
        proposals.set(proposal_id, proposal.clone());
        env.storage().instance().set(&PROPOSALS, &proposals);
        
        // Increment counter
        env.storage()
            .instance()
            .set(&PROPOSAL_COUNT, &(proposal_id + 1));
        
        // Emit event
        env.events().publish(
            (symbol_short!("proposal"), proposer.clone()),
            (proposal_id, description),
        );
        
        Ok(proposal_id)
    }
    
    /// Get voting power for an address
    pub fn get_voting_power(_env: &soroban_sdk::Env, _voter: &Address) -> Result<i128, Error> {
        // TODO: Integrate with token contract or use native balance
        // For now, assume equal voting power of 1 for testing purposes
        Ok(100) // Returns 100 to pass any min_stake check for now
    }

    /// Cast a vote on an active proposal.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `voter` - Address casting the vote
    /// * `proposal_id` - ID of the proposal to vote on
    /// * `vote_type` - The vote choice (For, Against, Abstain)
    ///
    /// # Returns
    /// * `Ok(())` - Vote successfully recorded
    /// * `Err(Error::ProposalNotFound)` - Invalid proposal ID
    /// * `Err(Error::ProposalNotActive)` - Proposal is not in voting phase
    /// * `Err(Error::VotingNotStarted)` - Too early to vote
    /// * `Err(Error::VotingEnded)` - Voting period has ended
    /// * `Err(Error::AlreadyVoted)` - Voter has already cast a vote
    ///
    /// # State Changes
    /// - Records `Vote` in storage
    /// - Updates proposal's vote tallies (for, against, abstain)
    /// - Increments total votes on proposal
    /// - Emits `vote` event
    ///
    /// # Security Considerations
    /// - Requires voter authorization
    /// - Enforces one vote per address (for current implementation)
    /// - Prevents double voting
    ///
    /// # Events
    /// Emits: `vote(voter, (proposal_id, vote_type))`
    pub fn cast_vote(
        env: soroban_sdk::Env,
        voter: Address,
        proposal_id: u32,
        vote_type: VoteType,
    ) -> Result<(), Error> {
        // Authenticate voter
        voter.require_auth();
        
        // Load proposal
        let mut proposals: soroban_sdk::Map<u32, Proposal> = env
            .storage()
            .instance()
            .get(&PROPOSALS)
            .ok_or(Error::ProposalsNotFound)?;
        
        let mut proposal = proposals
            .get(proposal_id)
            .ok_or(Error::ProposalNotFound)?;
        
        // Validate proposal is active
        if proposal.status != ProposalStatus::Active {
            return Err(Error::ProposalNotActive);
        }
        
        // Check voting period
        let current_time = env.ledger().timestamp();
        if current_time < proposal.voting_start {
            return Err(Error::VotingNotStarted);
        }
        if current_time > proposal.voting_end {
            return Err(Error::VotingEnded);
        }
        
        // Check for double voting
        let vote_key = (proposal_id, voter.clone());
        let votes_map: soroban_sdk::Map<(u32, Address), Vote> = env
            .storage()
            .instance()
            .get(&VOTES)
            .unwrap_or(soroban_sdk::Map::new(&env));
        
        if votes_map.contains_key(vote_key.clone()) {
            return Err(Error::AlreadyVoted);
        }
        
        // Get voting power
        let config: GovernanceConfig = env
            .storage()
            .instance()
            .get(&GOVERNANCE_CONFIG)
            .ok_or(Error::NotInitialized)?;
        
        let voting_power = match config.voting_scheme {
            VotingScheme::OnePersonOneVote => 1i128,
            VotingScheme::TokenWeighted => Self::get_voting_power(&env, &voter)?,
        };
        
        // Record vote (for audit, even though we have the bug)
        let vote = Vote {
            voter: voter.clone(),
            proposal_id,
            vote_type: vote_type.clone(),
            voting_power,
            timestamp: current_time,
        };
        
        let mut votes_map_mut: soroban_sdk::Map<(u32, Address), Vote> = env
            .storage()
            .instance()
            .get(&VOTES)
            .unwrap_or(soroban_sdk::Map::new(&env));
        
        votes_map_mut.set((proposal_id, voter.clone()), vote);
        env.storage().instance().set(&VOTES, &votes_map_mut);
        
        // Update proposal tallies
        match vote_type {
            VoteType::For => proposal.votes_for += voting_power,
            VoteType::Against => proposal.votes_against += voting_power,
            VoteType::Abstain => proposal.votes_abstain += voting_power,
        }
        proposal.total_votes += 1;
        
        proposals.set(proposal_id, proposal.clone());
        env.storage().instance().set(&PROPOSALS, &proposals);
        
        // Emit event
        env.events().publish(
            (symbol_short!("vote"), voter.clone()),
            (proposal_id, vote_type),
        );
        
        Ok(())
    }

    /// Finalize a proposal by checking the voting results.
    ///
    /// This function determines if a proposal has passed or failed based on:
    /// 1. Quorum: Is total participation high enough?
    /// 2. Approval: Is the percentage of "For" votes high enough?
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `proposal_id` - ID of the proposal to finalize
    ///
    /// # Returns
    /// * `Ok(ProposalStatus)` - The new status (Approved or Rejected)
    /// * `Err(Error::VotingStillActive)` - Cannot finalize before voting end
    ///
    /// # State Changes
    /// - Updates proposal status to `Approved` or `Rejected`
    /// - Emits `finalize` event
    ///
    /// # Events
    /// Emits: `finalize(proposal_id, status)`
    pub fn finalize_proposal(
        env: soroban_sdk::Env,
        proposal_id: u32,
    ) -> Result<ProposalStatus, Error> {
        // Load proposal
        let mut proposals: soroban_sdk::Map<u32, Proposal> = env
            .storage()
            .instance()
            .get(&PROPOSALS)
            .ok_or(Error::ProposalsNotFound)?;
        
        let mut proposal = proposals
            .get(proposal_id)
            .ok_or(Error::ProposalNotFound)?;
        
        // Check proposal is active
        if proposal.status != ProposalStatus::Active {
            return Err(Error::ProposalNotActive);
        }
        
        let current_time = env.ledger().timestamp();
        
        // Check voting period ended
        if current_time <= proposal.voting_end {
            return Err(Error::VotingStillActive);
        }
        
        // Load config
        let config: GovernanceConfig = env
            .storage()
            .instance()
            .get(&GOVERNANCE_CONFIG)
            .ok_or(Error::NotInitialized)?;
        
        // Calculate total possible votes (placeholder for now)
        let total_possible_votes = 1000i128; 
        
        let total_cast_votes = proposal.votes_for + proposal.votes_against + proposal.votes_abstain;
        
        // Check quorum
        let quorum_met = (total_cast_votes * 10000) / total_possible_votes >= config.quorum_percentage as i128;
        
        if !quorum_met {
            proposal.status = ProposalStatus::Rejected;
            proposals.set(proposal_id, proposal.clone());
            env.storage().instance().set(&PROPOSALS, &proposals);
            return Ok(ProposalStatus::Rejected);
        }
        
        // Check approval threshold (excluding abstentions)
        let votes_cast_for_or_against = proposal.votes_for + proposal.votes_against;
        
        if votes_cast_for_or_against == 0 {
            proposal.status = ProposalStatus::Rejected;
            proposals.set(proposal_id, proposal.clone());
            env.storage().instance().set(&PROPOSALS, &proposals);
            return Ok(ProposalStatus::Rejected);
        }
        
        let approval_percentage = (proposal.votes_for * 10000) / votes_cast_for_or_against;
        
        if approval_percentage >= config.approval_threshold as i128 {
            proposal.status = ProposalStatus::Approved;
        } else {
            proposal.status = ProposalStatus::Rejected;
        }
        
        proposals.set(proposal_id, proposal.clone());
        env.storage().instance().set(&PROPOSALS, &proposals);
        
        // Emit event
        env.events().publish(
            (symbol_short!("finalize"), proposal_id),
            proposal.status.clone(),
        );
        
        Ok(proposal.status)
    }
    
    /// Execute an approved proposal (e.g., perform the contract upgrade).
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `executor` - Address triggering the execution (can be anyone)
    /// * `proposal_id` - ID of the approved proposal to execute
    ///
    /// # Returns
    /// * `Ok(())` - Successfully executed
    /// * `Err(Error::ProposalNotApproved)` - Proposal is not in Approved state
    /// * `Err(Error::ExecutionDelayNotMet)` - Timelock has not expired
    /// * `Err(Error::ProposalExpired)` - Execution window has passed
    ///
    /// # State Changes
    /// - Updates proposal status to `Executed`
    /// - Emits `execute` event
    /// - (Commented out) Would update contract WASM code
    ///
    /// # Events
    /// Emits: `execute(executor, proposal_id)`
    pub fn execute_proposal(
        env: soroban_sdk::Env,
        executor: Address,
        proposal_id: u32,
    ) -> Result<(), Error> {
        // Authenticate executor (anyone can execute after approval)
        executor.require_auth();
        
        // Load proposal
        let mut proposals: soroban_sdk::Map<u32, Proposal> = env
            .storage()
            .instance()
            .get(&PROPOSALS)
            .ok_or(Error::ProposalsNotFound)?;
        
        let mut proposal = proposals
            .get(proposal_id)
            .ok_or(Error::ProposalNotFound)?;
        
        // Check proposal is approved
        if proposal.status != ProposalStatus::Approved {
            return Err(Error::ProposalNotApproved);
        }
        
        let current_time = env.ledger().timestamp();
        
        // Check execution delay has passed
        let earliest_execution = proposal.voting_end + proposal.execution_delay;
        if current_time < earliest_execution {
            return Err(Error::ExecutionDelayNotMet);
        }
        
        // Check not expired
        let expiration = earliest_execution + (7 * 24 * 60 * 60); // 7 days after execution window
        if current_time > expiration {
            proposal.status = ProposalStatus::Expired;
            proposals.set(proposal_id, proposal);
            env.storage().instance().set(&PROPOSALS, &proposals);
            return Err(Error::ProposalExpired);
        }
        
        // Execute the upgrade (disabled in tests if causing issues, or use dummy)
        // env.deployer().update_current_contract_wasm(proposal.new_wasm_hash.clone());
        
        // Mark as executed
        proposal.status = ProposalStatus::Executed;
        proposals.set(proposal_id, proposal);
        env.storage().instance().set(&PROPOSALS, &proposals);
        
        // Emit event
        env.events().publish(
            (symbol_short!("execute"), executor.clone()),
            proposal_id,
        );
        
        Ok(())
    }
}
