use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, BytesN, Env, Map, Symbol,
};

// --- Enums y Structs permanecen igual ---
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
    pub quorum_percentage: u32,
    pub approval_threshold: u32,
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

// --- CLAVE: Añadir macro #[contract] ---
// Note: Commented out to avoid symbol conflicts when used as a module
// #[contract]
pub struct GovernanceContract;

// #[contractimpl]
impl GovernanceContract {
    pub fn init_governance(
        env: Env,
        admin: Address,
        config: GovernanceConfig,
    ) -> Result<(), Error> {
        admin.require_auth();
        if config.quorum_percentage > 10000 || config.approval_threshold > 10000 {
            return Err(Error::InvalidThreshold);
        }
        if config.approval_threshold < 5000 {
            return Err(Error::ThresholdTooLow);
        }
        env.storage().instance().set(&GOVERNANCE_CONFIG, &config);
        env.storage().instance().set(&PROPOSAL_COUNT, &0u32);
        Ok(())
    }

    pub fn create_proposal(
        env: Env,
        proposer: Address,
        new_wasm_hash: BytesN<32>,
        description: Symbol,
    ) -> Result<u32, Error> {
        proposer.require_auth();
        let config: GovernanceConfig = env
            .storage()
            .instance()
            .get(&GOVERNANCE_CONFIG)
            .ok_or(Error::NotInitialized)?;

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
            VotingScheme::TokenWeighted => 100i128, // Simplificado para el test
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

        // Lógica de umbral (Threshold)
        let total_cast = proposal.votes_for + proposal.votes_against;
        if total_cast == 0 {
            proposal.status = ProposalStatus::Rejected;
        } else {
            let approval_bps = (proposal.votes_for * 10000) / total_cast;
            if approval_bps >= config.approval_threshold as i128 {
                proposal.status = ProposalStatus::Approved;
            } else {
                proposal.status = ProposalStatus::Rejected;
            }
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
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Events, Ledger};

    fn setup_test(env: &Env) -> (GovernanceContractClient<'_>, Address, Address) {
        let contract_id = env.register_contract(None, GovernanceContract);
        let client = GovernanceContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        let user = Address::generate(env);

        let config = GovernanceConfig {
            voting_period: 100,
            execution_delay: 0,
            quorum_percentage: 1000,
            approval_threshold: 5000,
            min_proposal_stake: 0,
            voting_scheme: VotingScheme::OnePersonOneVote,
        };

        env.mock_all_auths();
        client.init_governance(&admin, &config);
        (client, admin, user)
    }

    #[test]
    fn test_edge_case_double_voting() {
        let env = Env::default();
        let (client, _, user) = setup_test(&env);
        let prop_id = client.create_proposal(
            &user,
            &BytesN::from_array(&env, &[0u8; 32]),
            &symbol_short!("test"),
        );

        client.cast_vote(&user, &prop_id, &VoteType::For);

        let result = client.try_cast_vote(&user, &prop_id, &VoteType::For);
        assert_eq!(result, Err(Ok(Error::AlreadyVoted)));
    }

    #[test]
    fn test_edge_case_voting_after_expiration() {
        let env = Env::default();
        let (client, _, user) = setup_test(&env);
        let prop_id = client.create_proposal(
            &user,
            &BytesN::from_array(&env, &[0u8; 32]),
            &symbol_short!("test"),
        );

        env.ledger().with_mut(|li| li.timestamp = 200); // Saltamos al futuro (periodo era 100)

        let result = client.try_cast_vote(&user, &prop_id, &VoteType::For);
        assert_eq!(result, Err(Ok(Error::VotingEnded)));
    }

    #[test]
    fn test_edge_case_exact_threshold() {
        let env = Env::default();
        let (client, _, user1) = setup_test(&env);
        let user2 = Address::generate(&env);

        let prop_id = client.create_proposal(
            &user1,
            &BytesN::from_array(&env, &[0u8; 32]),
            &symbol_short!("test"),
        );

        // 1 voto a favor, 1 en contra = 50% exacto. El threshold es 5000 (50%).
        client.cast_vote(&user1, &prop_id, &VoteType::For);
        client.cast_vote(&user2, &prop_id, &VoteType::Against);

        env.ledger().with_mut(|li| li.timestamp = 200);
        let status = client.finalize_proposal(&prop_id);

        assert_eq!(status, ProposalStatus::Approved);
    }

    #[test]
    fn test_edge_case_below_threshold() {
        let env = Env::default();
        let (client, _, user1) = setup_test(&env);
        let user2 = Address::generate(&env);
        let user3 = Address::generate(&env);

        let prop_id = client.create_proposal(
            &user1,
            &BytesN::from_array(&env, &[0u8; 32]),
            &symbol_short!("test"),
        );

        // 1 voto a favor, 2 en contra = 33.3%. El threshold es 50%.
        client.cast_vote(&user1, &prop_id, &VoteType::For);
        client.cast_vote(&user2, &prop_id, &VoteType::Against);
        client.cast_vote(&user3, &prop_id, &VoteType::Against);

        env.ledger().with_mut(|li| li.timestamp = 200);
        let status = client.finalize_proposal(&prop_id);

        assert_eq!(status, ProposalStatus::Rejected);
    }

    #[test]
    fn test_events_emitted_for_proposal_vote_and_finalize() {
        let env = Env::default();
        let (client, _, proposer) = setup_test(&env);
        let voter_for = Address::generate(&env);
        let voter_against = Address::generate(&env);
        let prop_id = client.create_proposal(
            &proposer,
            &BytesN::from_array(&env, &[9u8; 32]),
            &symbol_short!("events"),
        );
        let e0 = env.events().all().len();
        client.cast_vote(&voter_for, &prop_id, &VoteType::For);
        let e1 = env.events().all().len();
        assert!(e1 > e0);
        client.cast_vote(&voter_against, &prop_id, &VoteType::Against);
        env.ledger().with_mut(|li| li.timestamp = 200);
        let _ = client.finalize_proposal(&prop_id);
        let e2 = env.events().all().len();
        assert!(e2 > e1);
    }

    #[test]
    fn test_no_vote_event_emitted_on_expired_vote_attempt() {
        let env = Env::default();
        let (client, _, proposer) = setup_test(&env);
        let voter = Address::generate(&env);
        let prop_id = client.create_proposal(
            &proposer,
            &BytesN::from_array(&env, &[7u8; 32]),
            &symbol_short!("noevent"),
        );
        env.ledger().with_mut(|li| li.timestamp = 1000);
        let before = env.events().all().len();
        let res = client.try_cast_vote(&voter, &prop_id, &VoteType::For);
        assert_eq!(res, Err(Ok(Error::VotingEnded)));
        let after = env.events().all().len();
        assert_eq!(before, after);
    }

    #[test]
    fn test_double_vote_emits_single_vote_event() {
        let env = Env::default();
        let (client, _, proposer) = setup_test(&env);
        let voter = Address::generate(&env);
        let prop_id = client.create_proposal(
            &proposer,
            &BytesN::from_array(&env, &[8u8; 32]),
            &symbol_short!("dblvote"),
        );
        client.cast_vote(&voter, &prop_id, &VoteType::For);
        let before = env.events().all().len();
        let res = client.try_cast_vote(&voter, &prop_id, &VoteType::For);
        assert_eq!(res, Err(Ok(Error::AlreadyVoted)));
        let after = env.events().all().len();
        assert_eq!(before, after);
    }
    #[test]
    fn test_vote_ordering_for_against_then_for_for_is_approved() {
        let env = Env::default();
        let (client, _, proposer) = setup_test(&env);
        let voter_for_1 = Address::generate(&env);
        let voter_for_2 = Address::generate(&env);
        let voter_against = Address::generate(&env);

        let proposal_id = client.create_proposal(
            &proposer,
            &BytesN::from_array(&env, &[1u8; 32]),
            &symbol_short!("ordera"),
        );

        client.cast_vote(&voter_against, &proposal_id, &VoteType::Against);
        client.cast_vote(&voter_for_1, &proposal_id, &VoteType::For);
        client.cast_vote(&voter_for_2, &proposal_id, &VoteType::For);

        env.ledger().with_mut(|li| li.timestamp = 200);
        let status = client.finalize_proposal(&proposal_id);
        assert_eq!(status, ProposalStatus::Approved);
    }

    #[test]
    fn test_vote_ordering_for_for_then_against_is_approved() {
        let env = Env::default();
        let (client, _, proposer) = setup_test(&env);
        let voter_for_1 = Address::generate(&env);
        let voter_for_2 = Address::generate(&env);
        let voter_against = Address::generate(&env);

        let proposal_id = client.create_proposal(
            &proposer,
            &BytesN::from_array(&env, &[2u8; 32]),
            &symbol_short!("orderb"),
        );

        client.cast_vote(&voter_for_1, &proposal_id, &VoteType::For);
        client.cast_vote(&voter_for_2, &proposal_id, &VoteType::For);
        client.cast_vote(&voter_against, &proposal_id, &VoteType::Against);

        env.ledger().with_mut(|li| li.timestamp = 200);
        let status = client.finalize_proposal(&proposal_id);
        assert_eq!(status, ProposalStatus::Approved);
    }
}
