//! Multisig approval engine used by Grainlify upgrade flows.
//!
//! Proposal identifiers are allocated from a monotonic counter and are treated
//! as stable handles for subsequent approval and execution steps.

use soroban_sdk::{contracttype, symbol_short, Address, Env, Vec};

/// =======================
/// Storage Keys
/// =======================
#[contracttype]
enum DataKey {
    Config,
    Proposal(u64),
    ProposalCounter,
    Paused,
}

/// =======================
/// Multisig Configuration
/// =======================
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiSigConfig {
    /// Ordered signer set authorized to create and approve proposals.
    pub signers: Vec<Address>,
    /// Minimum number of distinct signer approvals required for execution.
    pub threshold: u32,
}

/// =======================
/// Proposal Structure
/// =======================
#[contracttype]
#[derive(Clone)]
pub struct Proposal {
    /// Signers that have approved this proposal.
    pub approvals: Vec<Address>,
    /// Whether the proposal has already been consumed by execution.
    pub executed: bool,
    /// Expiry ledger timestamp in seconds (`0` means no expiry).
    pub expiry: u64,
    /// Whether the proposal has been explicitly cancelled.
    pub cancelled: bool,
}

/// =======================
/// Errors
/// =======================
#[derive(Debug)]
pub enum MultiSigError {
    NotSigner,
    AlreadyApproved,
    ProposalNotFound,
    ProposalAlreadyExists,
    AlreadyExecuted,
    AlreadyCancelled,
    ThresholdNotMet,
    InvalidThreshold,
    ProposalCancelled,
    ProposalExpired,
    /// Reserved for the paused-contract guard; kept so the error surface stays
    /// stable even when a build does not construct it.
    #[allow(dead_code)]
    ContractPaused,
    #[allow(dead_code)]
    StateInconsistent,
}

/// =======================
/// Public API
/// =======================
pub struct MultiSig;

impl MultiSig {
    /// Initializes the signer set and execution threshold.
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
    pub fn propose(env: &Env, proposer: Address, expiry: u64) -> u64 {
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
            expiry,
            cancelled: false,
        };

        if env.storage().instance().has(&DataKey::Proposal(counter)) {
            panic!("{:?}", MultiSigError::ProposalAlreadyExists);
        }

        env.storage()
            .instance()
            .set(&DataKey::Proposal(counter), &proposal);
        env.storage()
            .instance()
            .set(&DataKey::ProposalCounter, &counter);

        env.events().publish((symbol_short!("proposal"),), counter);

        counter
    }

    /// Records a signer approval for an existing proposal.
    pub fn approve(env: &Env, proposal_id: u64, signer: Address) {
        signer.require_auth();

        let config = Self::get_config(env);
        Self::assert_signer(&config, &signer);

        let mut proposal = Self::get_proposal(env, proposal_id);

        if proposal.executed {
            panic!("{:?}", MultiSigError::AlreadyExecuted);
        }
        if proposal.cancelled {
            panic!("{:?}", MultiSigError::ProposalCancelled);
        }
        if Self::is_expired(env, proposal_id) {
            panic!("{:?}", MultiSigError::ProposalExpired);
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

    /// Returns whether a proposal currently satisfies the execution threshold.
    pub fn can_execute(env: &Env, proposal_id: u64) -> bool {
        // First check if contract is in a healthy state
        if Self::is_contract_paused(env) || Self::is_state_inconsistent(env) {
            return false;
        }

        let config = Self::get_config(env);
        let proposal = Self::get_proposal(env, proposal_id);

        !proposal.executed
            && !proposal.cancelled
            && !Self::is_expired(env, proposal_id)
            && proposal.approvals.len() >= config.threshold
    }

    /// Marks a proposal as executed after the guarded action succeeds.
    pub fn mark_executed(env: &Env, proposal_id: u64) {
        let mut proposal = Self::get_proposal(env, proposal_id);

        if proposal.executed {
            panic!("{:?}", MultiSigError::AlreadyExecuted);
        }
        if proposal.cancelled {
            panic!("{:?}", MultiSigError::ProposalCancelled);
        }
        if Self::is_expired(env, proposal_id) {
            panic!("{:?}", MultiSigError::ProposalExpired);
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

    /// Cancels a proposal so it can no longer be approved or executed.
    pub fn cancel(env: &Env, proposal_id: u64, canceller: Address) {
        canceller.require_auth();

        let config = Self::get_config(env);
        Self::assert_signer(&config, &canceller);

        let mut proposal = Self::get_proposal(env, proposal_id);

        if proposal.executed {
            panic!("{:?}", MultiSigError::AlreadyExecuted);
        }
        if proposal.cancelled {
            panic!("{:?}", MultiSigError::AlreadyCancelled);
        }

        proposal.cancelled = true;
        env.storage()
            .instance()
            .set(&DataKey::Proposal(proposal_id), &proposal);

        env.events()
            .publish((symbol_short!("cancelled"),), (proposal_id, canceller));
    }

    /// Pauses multisig-protected execution paths.
    pub fn pause(env: &Env, signer: Address) {
        signer.require_auth();

        let config = Self::get_config(env);
        Self::assert_signer(&config, &signer);

        env.storage().instance().set(&DataKey::Paused, &true);
        env.events().publish((symbol_short!("paused"),), signer);
    }

    /// Unpauses multisig-protected execution paths.
    pub fn unpause(env: &Env, signer: Address) {
        signer.require_auth();

        let config = Self::get_config(env);
        Self::assert_signer(&config, &signer);

        env.storage().instance().set(&DataKey::Paused, &false);
        env.events().publish((symbol_short!("unpause"),), signer);
    }

    /// Returns whether multisig execution is paused.
    pub fn is_contract_paused(env: &Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false)
    }

    /// Returns true if the stored multisig configuration is invalid.
    pub fn is_state_inconsistent(env: &Env) -> bool {
        match Self::get_config_opt(env) {
            Some(config) => config.threshold == 0 || config.threshold > config.signers.len(),
            None => false,
        }
    }

    /// Returns the current multisig configuration, if initialized.
    pub fn get_config_opt(env: &Env) -> Option<MultiSigConfig> {
        env.storage().instance().get(&DataKey::Config)
    }

    /// Returns `true` if a proposal was cancelled.
    ///
    /// Retained as part of the query surface; the cancellation path currently
    /// rejects a cancelled proposal directly.
    #[allow(dead_code)]
    pub fn is_cancelled(env: &Env, proposal_id: u64) -> bool {
        Self::get_proposal_opt(env, proposal_id)
            .map(|p| p.cancelled)
            .unwrap_or(false)
    }

    /// Returns `true` if a proposal has expired at the current ledger timestamp.
    pub fn is_expired(env: &Env, proposal_id: u64) -> bool {
        let now = env.ledger().timestamp();
        Self::get_proposal_opt(env, proposal_id)
            .map(|p| p.expiry != 0 && now >= p.expiry)
            .unwrap_or(false)
    }

    /// Returns a proposal if present.
    pub fn get_proposal_opt(env: &Env, proposal_id: u64) -> Option<Proposal> {
        env.storage()
            .instance()
            .get(&DataKey::Proposal(proposal_id))
    }

    /// Sets the multisig configuration directly for controlled restore flows.
    pub fn set_config(env: &Env, config: MultiSigConfig) {
        if config.threshold == 0 || config.threshold > config.signers.len() {
            panic!("{:?}", MultiSigError::InvalidThreshold);
        }
        env.storage().instance().set(&DataKey::Config, &config);
    }

    /// Clears the multisig configuration for controlled restore flows.
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
