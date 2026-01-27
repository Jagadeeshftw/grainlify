
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
    /// Initialize multisig configuration
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

    /// Create a new proposal
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

    /// Approve an existing proposal
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

    /// Check if proposal is executable
    pub fn can_execute(env: &Env, proposal_id: u64) -> bool {
        let config = Self::get_config(env);
        let proposal = Self::get_proposal(env, proposal_id);

        !proposal.executed && proposal.approvals.len() >= config.threshold
    }

    /// Mark proposal as executed (caller executes action externally)
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


