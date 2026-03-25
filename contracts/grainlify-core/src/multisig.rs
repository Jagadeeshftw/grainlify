//! Multisig storage and execution primitives used by the upgrade flow.
//!
//! Proposal identifiers are allocated from a monotonic counter and are never
//! reused. Callers can safely bind proposal-specific payloads to the returned
//! identifier.

use soroban_sdk::{contracttype, symbol_short, Address, Env, Vec};

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
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiSigConfig {
    /// Addresses authorized to create and approve proposals.
    pub signers: Vec<Address>,
    /// Unique approvals required before execution.
    pub threshold: u32,
}

/// =======================
/// Proposal Structure
/// =======================
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proposal {
    /// Signer approvals collected for this proposal.
    pub approvals: Vec<Address>,
    /// Whether the proposal has already been executed.
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
    /// Initializes multisig configuration.
    pub fn init(env: &Env, signers: Vec<Address>, threshold: u32) {
        if threshold == 0 || threshold > signers.len() {
            panic!("{:?}", MultiSigError::InvalidThreshold);
        }

        let config = MultiSigConfig { signers, threshold };
        env.storage().instance().set(&DataKey::Config, &config);
        env.storage()
            .instance()
            .set(&DataKey::ProposalCounter, &0u64);
    }

    /// Creates a new proposal and returns its stable identifier.
    ///
    /// Proposal IDs are derived from a monotonic counter and are never reused.
    pub fn propose(env: &Env, proposer: Address) -> u64 {
        proposer.require_auth();

        let config = Self::get_config(env);
        Self::assert_signer(&config, &proposer);

        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::ProposalCounter)
            .unwrap_or(0);

        let proposal_id = counter.checked_add(1).expect("proposal id overflow");

        let proposal = Proposal {
            approvals: Vec::new(env),
            executed: false,
        };

        env.storage()
            .instance()
            .set(&DataKey::Proposal(proposal_id), &proposal);
        env.storage()
            .instance()
            .set(&DataKey::ProposalCounter, &proposal_id);

        env.events().publish((symbol_short!("proposal"),), proposal_id);

        proposal_id
    }

    /// Approves an existing proposal.
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

        env.events()
            .publish((symbol_short!("approved"),), (proposal_id, signer));
    }

    /// Returns whether a proposal has enough approvals to execute.
    pub fn can_execute(env: &Env, proposal_id: u64) -> bool {
        let config = Self::get_config(env);
        let proposal = Self::get_proposal(env, proposal_id);

        !proposal.executed && proposal.approvals.len() >= config.threshold
    }

    /// Marks a proposal as executed after the caller applies its side effects.
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

        env.events()
            .publish((symbol_short!("executed"),), proposal_id);
    }

    /// Gets current multisig config if initialized.
    pub fn get_config_opt(env: &Env) -> Option<MultiSigConfig> {
        env.storage().instance().get(&DataKey::Config)
    }

    /// Sets multisig config directly (used by controlled restore operations).
    pub fn set_config(env: &Env, config: MultiSigConfig) {
        if config.threshold == 0 || config.threshold > config.signers.len() as u32 {
            panic!("{:?}", MultiSigError::InvalidThreshold);
        }
        env.storage().instance().set(&DataKey::Config, &config);
    }

    /// Clears multisig config (used by controlled restore operations).
    pub fn clear_config(env: &Env) {
        env.storage().instance().remove(&DataKey::Config);
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
