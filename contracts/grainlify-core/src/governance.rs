use crate::asset;
use soroban_sdk::{contracttype, symbol_short, Address, BytesN, Env, Map, Symbol};

/// Represents the lifecycle stages of a governance proposal.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum ProposalStatus {
    Pending,
    Active,
    Approved,
    Rejected,
    Executed,
    Expired,
    Vetoed,
}

/// Types of votes a participant can cast.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum VoteType {
    For,
    Against,
    Abstain,
}

/// Determines how voting power is calculated.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum VotingScheme {
    OnePersonOneVote,
    TokenWeighted,
}

/// Distinct RBAC role identifiers used for authorization checks.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum Role {
    Admin,
    Emergency,
    Upgrade,
    Config,
}

/// Core data structure for a governance proposal.
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
    pub stake_amount: i128,
}

/// Immutable governance parameters set during `init_governance`.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct GovernanceConfig {
    pub voting_period: u64,
    pub execution_delay: u64,
    pub quorum_percentage: u32,
    pub approval_threshold: u32,
    pub min_proposal_stake: i128,
    pub voting_scheme: VotingScheme,
    pub governance_token: Address,
}

/// Recorded vote for a governance proposal.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Vote {
    pub voter: Address,
    pub proposal_id: u32,
    pub vote_type: VoteType,
    pub voting_power: i128,
    pub timestamp: u64,
}

/// Two-step pending admin rotation — awaiting confirmation from the new admin.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct PendingAdminRotation {
    pub proposed_admin: Address,
    pub initiated_at: u64,
    pub expires_at: u64,
}

// ---------------------------------------------------------------------------
// Storage keys
// ---------------------------------------------------------------------------
pub(crate) const PROPOSALS: Symbol = symbol_short!("PROPOSALS");
pub(crate) const PROPOSAL_COUNT: Symbol = symbol_short!("PROP_CNT");
pub(crate) const VOTES: Symbol = symbol_short!("VOTES");
pub(crate) const GOVERNANCE_CONFIG: Symbol = symbol_short!("GOV_CFG");
pub(crate) const SECURITY_COUNCIL: Symbol = symbol_short!("SEC_CNCL");
pub(crate) const ROLE_ADMIN: Symbol = symbol_short!("RL_ADM");
pub(crate) const ROLE_EMERGENCY: Symbol = symbol_short!("RL_EMG");
pub(crate) const ROLE_UPGRADE: Symbol = symbol_short!("RL_UPG");
pub(crate) const ROLE_CONFIG: Symbol = symbol_short!("RL_CFG");
pub(crate) const EMERGENCY_PAUSED: Symbol = symbol_short!("EMG_PAUSE");
pub(crate) const PENDING_ADMIN: Symbol = symbol_short!("PEND_ADM");

/// Default pending-admin rotation expiry (24 hours in ledger seconds).
pub(crate) const DEFAULT_ADMIN_ROTATION_TTL: u64 = 86_400;

/// Governance errors returned by the standalone governance contract.
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
    InsufficientBalance = 15,
    NotSecurityCouncil = 16,
    CannotVeto = 17,
    SecurityCouncilNotSet = 18,
    NotAuthorizedForRole = 100,
    InvalidRoleHolder = 101,
    EmergencyPaused = 102,
    NoPendingAdminRotation = 103,
    PendingAdminExpired = 104,
    AlreadyInitialized = 105,
}

// ===========================================================================
// Internal helpers — all guarantees: auth BEFORE state mutation
// ===========================================================================

fn storage_key_for_role(role: &Role) -> Symbol {
    match role {
        Role::Admin => ROLE_ADMIN,
        Role::Emergency => ROLE_EMERGENCY,
        Role::Upgrade => ROLE_UPGRADE,
        Role::Config => ROLE_CONFIG,
    }
}

fn require_not_zero_or_self(env: &Env, candidate: &Address) -> Result<(), Error> {
    let bytes = candidate.to_val().to_bytes();
    let all_zero = bytes.iter().all(|b| *b == 0);
    if all_zero {
        return Err(Error::InvalidRoleHolder);
    }
    if *candidate == env.current_contract_address() {
        return Err(Error::InvalidRoleHolder);
    }
    Ok(())
}

fn require_role(env: &Env, principal: &Address, role: Role) -> Result<(), Error> {
    let key = storage_key_for_role(&role);
    let holder: Option<Address> = env.storage().instance().get(&key);
    match holder {
        Some(ref stored) if stored == principal => Ok(()),
        _ => Err(Error::NotAuthorizedForRole),
    }
}

fn require_not_emergency_paused(env: &Env) -> Result<(), Error> {
    let paused: bool = env
        .storage()
        .instance()
        .get(&EMERGENCY_PAUSED)
        .unwrap_or(false);
    if paused {
        Err(Error::EmergencyPaused)
    } else {
        Ok(())
    }
}

fn store_role(env: &Env, role: Role, holder: &Address) {
    let key = storage_key_for_role(&role);
    env.storage().instance().set(&key, holder);
}

pub fn get_role_holder(env: &Env, role: Role) -> Option<Address> {
    let key = storage_key_for_role(&role);
    env.storage().instance().get(&key)
}

pub fn is_emergency_paused(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&EMERGENCY_PAUSED)
        .unwrap_or(false)
}

pub(crate) fn validate_config(config: &GovernanceConfig) -> Result<(), Error> {
    if config.quorum_percentage > 10000 || config.approval_threshold > 10000 {
        return Err(Error::InvalidThreshold);
    }

    if config.approval_threshold < 5000 {
        return Err(Error::ThresholdTooLow);
    }

    Ok(())
}

// ===========================================================================
// GovernanceContract — RBAC-hardened entrypoints
// ===========================================================================
pub struct GovernanceContract;

impl GovernanceContract {
    // -----------------------------------------------------------------------
    // Initialization
    // -----------------------------------------------------------------------

    pub fn init_governance_state(
        env: Env,
        admin: Address,
        config: GovernanceConfig,
    ) -> Result<(), Error> {
        if env.storage().instance().has(&ROLE_ADMIN) {
            return Err(Error::AlreadyInitialized);
        }

        admin.require_auth();
        require_not_zero_or_self(&env, &admin)?;
        validate_config(&config)?;

        store_role(&env, Role::Admin, &admin);
        store_role(&env, Role::Emergency, &admin);
        store_role(&env, Role::Upgrade, &admin);
        store_role(&env, Role::Config, &admin);

        env.storage().instance().set(&GOVERNANCE_CONFIG, &config);
        env.storage().instance().set(&PROPOSAL_COUNT, &0u32);
        env.storage().instance().set(&EMERGENCY_PAUSED, &false);

        env.events().publish(
            (symbol_short!("gov_init"),),
            (admin.clone(), config.voting_period, config.execution_delay),
        );

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Role management — ALL require Admin authorization + validation FIRST
    // -----------------------------------------------------------------------

    pub fn rotate_admin(env: Env, current_admin: Address, new_admin: Address) -> Result<(), Error> {
        current_admin.require_auth();
        require_not_zero_or_self(&env, &new_admin)?;
        require_role(&env, &current_admin, Role::Admin)?;

        let now = env.ledger().timestamp();
        let pending = PendingAdminRotation {
            proposed_admin: new_admin.clone(),
            initiated_at: now,
            expires_at: now.saturating_add(DEFAULT_ADMIN_ROTATION_TTL),
        };
        env.storage().instance().set(&PENDING_ADMIN, &pending);

        env.events().publish(
            (symbol_short!("adm_rot_p"),),
            (new_admin.clone(), pending.expires_at),
        );
        Ok(())
    }

    pub fn confirm_admin_rotation(env: Env, new_admin: Address) -> Result<(), Error> {
        new_admin.require_auth();
        require_not_zero_or_self(&env, &new_admin)?;

        let pending: PendingAdminRotation = env
            .storage()
            .instance()
            .get(&PENDING_ADMIN)
            .ok_or(Error::NoPendingAdminRotation)?;

        if pending.proposed_admin != new_admin {
            return Err(Error::NoPendingAdminRotation);
        }
        if env.ledger().timestamp() > pending.expires_at {
            env.storage().instance().remove(&PENDING_ADMIN);
            return Err(Error::PendingAdminExpired);
        }

        let old_admin: Option<Address> = get_role_holder(&env, Role::Admin);
        store_role(&env, Role::Admin, &new_admin);
        env.storage().instance().remove(&PENDING_ADMIN);

        env.events().publish(
            (symbol_short!("adm_rot_c"),),
            (old_admin, new_admin.clone()),
        );
        Ok(())
    }

    pub fn set_emergency_role(
        env: Env,
        admin: Address,
        new_holder: Address,
    ) -> Result<(), Error> {
        admin.require_auth();
        require_not_zero_or_self(&env, &new_holder)?;
        require_role(&env, &admin, Role::Admin)?;

        let previous = get_role_holder(&env, Role::Emergency);
        store_role(&env, Role::Emergency, &new_holder);

        env.events().publish(
            (symbol_short!("emg_role"),),
            (previous, new_holder.clone()),
        );
        Ok(())
    }

    pub fn set_upgrade_role(env: Env, admin: Address, new_holder: Address) -> Result<(), Error> {
        admin.require_auth();
        require_not_zero_or_self(&env, &new_holder)?;
        require_role(&env, &admin, Role::Admin)?;

        let previous = get_role_holder(&env, Role::Upgrade);
        store_role(&env, Role::Upgrade, &new_holder);

        env.events().publish(
            (symbol_short!("upg_role"),),
            (previous, new_holder.clone()),
        );
        Ok(())
    }

    pub fn set_config_role(env: Env, admin: Address, new_holder: Address) -> Result<(), Error> {
        admin.require_auth();
        require_not_zero_or_self(&env, &new_holder)?;
        require_role(&env, &admin, Role::Admin)?;

        let previous = get_role_holder(&env, Role::Config);
        store_role(&env, Role::Config, &new_holder);

        env.events().publish(
            (symbol_short!("cfg_role"),),
            (previous, new_holder.clone()),
        );
        Ok(())
    }

    pub fn set_security_council(
        env: Env,
        admin: Address,
        security_council: Address,
    ) -> Result<(), Error> {
        admin.require_auth();
        require_not_zero_or_self(&env, &security_council)?;
        require_role(&env, &admin, Role::Admin)?;

        let previous: Option<Address> = env.storage().instance().get(&SECURITY_COUNCIL);
        env.storage()
            .instance()
            .set(&SECURITY_COUNCIL, &security_council);

        env.events().publish(
            (symbol_short!("sec_cncl"),),
            (previous, security_council.clone()),
        );
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Emergency pause controls — Emergency role only
    // -----------------------------------------------------------------------

    pub fn emergency_pause(env: Env, caller: Address) -> Result<(), Error> {
        caller.require_auth();
        require_role(&env, &caller, Role::Emergency)?;

        env.storage().instance().set(&EMERGENCY_PAUSED, &true);

        env.events()
            .publish((symbol_short!("emg_pause"),), caller.clone());
        Ok(())
    }

    pub fn emergency_unpause(env: Env, caller: Address) -> Result<(), Error> {
        caller.require_auth();
        require_role(&env, &caller, Role::Emergency)?;

        env.storage().instance().set(&EMERGENCY_PAUSED, &false);

        env.events()
            .publish((symbol_short!("emg_unp"),), caller.clone());
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Config update — Config role only
    // -----------------------------------------------------------------------

    pub fn update_governance_config(
        env: Env,
        caller: Address,
        new_voting_period: u64,
        new_execution_delay: u64,
        new_quorum_percentage: u32,
        new_approval_threshold: u32,
        new_min_proposal_stake: i128,
    ) -> Result<(), Error> {
        caller.require_auth();
        require_role(&env, &caller, Role::Config)?;
        require_not_emergency_paused(&env)?;

        let existing: GovernanceConfig = env
            .storage()
            .instance()
            .get(&GOVERNANCE_CONFIG)
            .ok_or(Error::NotInitialized)?;

        let updated = GovernanceConfig {
            voting_period: new_voting_period,
            execution_delay: new_execution_delay,
            quorum_percentage: new_quorum_percentage,
            approval_threshold: new_approval_threshold,
            min_proposal_stake: new_min_proposal_stake,
            voting_scheme: existing.voting_scheme.clone(),
            governance_token: existing.governance_token.clone(),
        };
        validate_config(&updated)?;

        env.storage().instance().set(&GOVERNANCE_CONFIG, &updated);

        env.events().publish(
            (symbol_short!("gov_cfgup"),),
            (
                new_voting_period,
                new_execution_delay,
                new_quorum_percentage,
                new_approval_threshold,
                new_min_proposal_stake,
            ),
        );
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Proposal / voting / execution — RBAC-gated where required
    // -----------------------------------------------------------------------

    pub fn create_proposal(
        env: Env,
        proposer: Address,
        new_wasm_hash: BytesN<32>,
        description: Symbol,
    ) -> Result<u32, Error> {
        proposer.require_auth();
        require_not_emergency_paused(&env)?;

        let config: GovernanceConfig = env
            .storage()
            .instance()
            .get(&GOVERNANCE_CONFIG)
            .ok_or(Error::NotInitialized)?;

        if config.min_proposal_stake > 0 {
            let balance = asset::balance(&env, &config.governance_token, &proposer)
                .map_err(|_| Error::InsufficientBalance)?;
            if balance < config.min_proposal_stake {
                return Err(Error::InsufficientStake);
            }
            asset::transfer_exact(
                &env,
                &config.governance_token,
                &proposer,
                &env.current_contract_address(),
                config.min_proposal_stake,
            )
            .map_err(|_| Error::InsufficientBalance)?;
        }

        let proposal_id: u32 = env.storage().instance().get(&PROPOSAL_COUNT).unwrap_or(0);
        let current_time = env.ledger().timestamp();

        let proposal = Proposal {
            id: proposal_id,
            proposer: proposer.clone(),
            new_wasm_hash,
            description,
            created_at: current_time,
            voting_start: current_time,
            voting_end: current_time + config.voting_period,
            execution_delay: config.execution_delay,
            status: ProposalStatus::Active,
            votes_for: 0,
            votes_against: 0,
            votes_abstain: 0,
            total_votes: 0,
            stake_amount: config.min_proposal_stake,
        };

        let mut proposals: Map<u32, Proposal> = env
            .storage()
            .instance()
            .get(&PROPOSALS)
            .unwrap_or(Map::new(&env));
        proposals.set(proposal_id, proposal.clone());
        env.storage().instance().set(&PROPOSALS, &proposals);
        env.storage()
            .instance()
            .set(&PROPOSAL_COUNT, &(proposal_id + 1));
        env.events()
            .publish((symbol_short!("gov_prop"),), proposal.clone());

        Ok(proposal_id)
    }

    pub fn cast_vote(
        env: Env,
        voter: Address,
        proposal_id: u32,
        vote_type: VoteType,
    ) -> Result<(), Error> {
        voter.require_auth();
        require_not_emergency_paused(&env)?;

        let mut proposals: Map<u32, Proposal> = env
            .storage()
            .instance()
            .get(&PROPOSALS)
            .ok_or(Error::ProposalsNotFound)?;
        let mut proposal = proposals.get(proposal_id).ok_or(Error::ProposalNotFound)?;

        if proposal.status != ProposalStatus::Active {
            return Err(Error::ProposalNotActive);
        }

        let current_time = env.ledger().timestamp();
        if current_time > proposal.voting_end {
            return Err(Error::VotingEnded);
        }

        let mut votes: Map<(u32, Address), Vote> = env
            .storage()
            .instance()
            .get(&VOTES)
            .unwrap_or(Map::new(&env));
        if votes.contains_key((proposal_id, voter.clone())) {
            return Err(Error::AlreadyVoted);
        }

        let config: GovernanceConfig = env
            .storage()
            .instance()
            .get(&GOVERNANCE_CONFIG)
            .ok_or(Error::NotInitialized)?;

        let voting_power = match config.voting_scheme {
            VotingScheme::OnePersonOneVote => 1i128,
            VotingScheme::TokenWeighted => asset::balance(&env, &config.governance_token, &voter)
                .map_err(|_| Error::InsufficientBalance)?,
        };

        match vote_type {
            VoteType::For => proposal.votes_for += voting_power,
            VoteType::Against => proposal.votes_against += voting_power,
            VoteType::Abstain => proposal.votes_abstain += voting_power,
        }
        proposal.total_votes += 1;

        votes.set(
            (proposal_id, voter.clone()),
            Vote {
                voter: voter.clone(),
                proposal_id,
                vote_type: vote_type.clone(),
                voting_power,
                timestamp: current_time,
            },
        );

        proposals.set(proposal_id, proposal);
        env.storage().instance().set(&PROPOSALS, &proposals);
        env.storage().instance().set(&VOTES, &votes);
        env.events().publish(
            (symbol_short!("gov_vote"),),
            Vote {
                voter,
                proposal_id,
                vote_type: vote_type.clone(),
                voting_power,
                timestamp: current_time,
            },
        );
        Ok(())
    }

    pub fn finalize_proposal(env: Env, proposal_id: u32) -> Result<ProposalStatus, Error> {
        require_not_emergency_paused(&env)?;

        let mut proposals: Map<u32, Proposal> = env
            .storage()
            .instance()
            .get(&PROPOSALS)
            .ok_or(Error::ProposalsNotFound)?;
        let mut proposal = proposals.get(proposal_id).ok_or(Error::ProposalNotFound)?;
        let config: GovernanceConfig = env
            .storage()
            .instance()
            .get(&GOVERNANCE_CONFIG)
            .ok_or(Error::NotInitialized)?;

        if env.ledger().timestamp() <= proposal.voting_end {
            return Err(Error::VotingStillActive);
        }

        let total_possible_votes = match config.voting_scheme {
            VotingScheme::OnePersonOneVote => 100i128,
            VotingScheme::TokenWeighted => 1000000i128,
        };

        let total_cast = proposal.votes_for + proposal.votes_against + proposal.votes_abstain;
        let quorum_met =
            (total_cast * 10000) / total_possible_votes >= config.quorum_percentage as i128;

        if !quorum_met {
            proposal.status = ProposalStatus::Rejected;
        } else {
            let total_decisive = proposal.votes_for + proposal.votes_against;
            if total_decisive == 0 {
                proposal.status = ProposalStatus::Rejected;
            } else {
                let approval_bps = (proposal.votes_for * 10000) / total_decisive;
                if approval_bps >= config.approval_threshold as i128 {
                    proposal.status = ProposalStatus::Approved;
                } else {
                    proposal.status = ProposalStatus::Rejected;
                }
            }
        }

        if proposal.stake_amount > 0 {
            asset::transfer_exact(
                &env,
                &config.governance_token,
                &env.current_contract_address(),
                &proposal.proposer,
                proposal.stake_amount,
            )
            .map_err(|_| Error::InsufficientBalance)?;
        }

        proposals.set(proposal_id, proposal.clone());
        env.storage().instance().set(&PROPOSALS, &proposals);
        env.events().publish(
            (symbol_short!("gov_final"),),
            (
                proposal_id,
                proposal.status.clone(),
                proposal.votes_for,
                proposal.votes_against,
                proposal.votes_abstain,
            ),
        );
        Ok(proposal.status)
    }

    pub fn execute_proposal(
        env: Env,
        executor: Address,
        proposal_id: u32,
    ) -> Result<(), Error> {
        executor.require_auth();
        require_role(&env, &executor, Role::Upgrade)?;
        require_not_emergency_paused(&env)?;

        let mut proposals: Map<u32, Proposal> = env
            .storage()
            .instance()
            .get(&PROPOSALS)
            .ok_or(Error::ProposalsNotFound)?;
        let mut proposal = proposals.get(proposal_id).ok_or(Error::ProposalNotFound)?;

        if proposal.status != ProposalStatus::Approved {
            return Err(Error::ProposalNotApproved);
        }

        if env.ledger().timestamp() < proposal.voting_end + proposal.execution_delay {
            return Err(Error::ExecutionDelayNotMet);
        }

        let mut is_dummy = true;
        for b in proposal.new_wasm_hash.iter() {
            if b != 0 {
                is_dummy = false;
                break;
            }
        }

        if !is_dummy {
            env.deployer()
                .update_current_contract_wasm(proposal.new_wasm_hash.clone());
        }

        proposal.status = ProposalStatus::Executed;
        proposals.set(proposal_id, proposal);
        env.storage().instance().set(&PROPOSALS, &proposals);

        env.events().publish(
            (symbol_short!("gov_exec"),),
            (proposal_id, executor.clone()),
        );
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Veto — Security Council dedicated path
    // -----------------------------------------------------------------------

    pub fn veto_proposal(
        env: Env,
        security_council: Address,
        proposal_id: u32,
    ) -> Result<(), Error> {
        security_council.require_auth();

        let stored_council: Address = env
            .storage()
            .instance()
            .get(&SECURITY_COUNCIL)
            .ok_or(Error::SecurityCouncilNotSet)?;

        if stored_council != security_council {
            return Err(Error::NotSecurityCouncil);
        }

        let mut proposals: Map<u32, Proposal> = env
            .storage()
            .instance()
            .get(&PROPOSALS)
            .ok_or(Error::ProposalsNotFound)?;
        let mut proposal = proposals.get(proposal_id).ok_or(Error::ProposalNotFound)?;

        if proposal.status != ProposalStatus::Approved {
            return Err(Error::CannotVeto);
        }

        let current_time = env.ledger().timestamp();
        if current_time >= proposal.voting_end + proposal.execution_delay {
            return Err(Error::CannotVeto);
        }

        proposal.status = ProposalStatus::Vetoed;
        proposals.set(proposal_id, proposal);
        env.storage().instance().set(&PROPOSALS, &proposals);

        env.events().publish(
            (symbol_short!("gov_veto"),),
            (proposal_id, security_council.clone()),
        );

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Read-only
    // -----------------------------------------------------------------------

    pub fn get_config(env: Env) -> Result<GovernanceConfig, Error> {
        env.storage()
            .instance()
            .get(&GOVERNANCE_CONFIG)
            .ok_or(Error::NotInitialized)
    }

    pub fn get_security_council(env: Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&SECURITY_COUNCIL)
            .ok_or(Error::SecurityCouncilNotSet)
    }

    pub fn get_pending_admin_rotation(env: Env) -> Option<PendingAdminRotation> {
        env.storage().instance().get(&PENDING_ADMIN)
    }
}
