#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env, String, BytesN};
use crate::{EscrowViewFacade, EscrowViewFacadeClient, EscrowStatus};

// Dummy Escrow contract implementation to mock `BountyEscrow` calls
mod dummy_escrow {
    use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec, BytesN};
    use soroban_sdk::testutils::Address as _;
    
    // Use matching signatures to our defined binding
    #[contracttype]
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub enum EscrowStatus {
        Locked,
        Released,
        Refunded,
        PartiallyRefunded,
    }

    #[contracttype]
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct EscrowMetadata {
        pub repo_id: u64,
        pub issue_id: u64,
        pub bounty_type: String,
        pub risk_flags: u32,
        pub notification_prefs: u32,
        pub reference_hash: Option<soroban_sdk::Bytes>,
    }

    #[contracttype]
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct PauseFlags {
        pub lock_paused: bool,
        pub release_paused: bool,
        pub refund_paused: bool,
        pub pause_reason: Option<String>,
        pub paused_at: u64,
    }

    #[contracttype]
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct Escrow {
        pub depositor: Address,
        pub amount: i128,
        pub remaining_amount: i128,
        pub status: EscrowStatus,
        pub deadline: u64,
        pub schema_version: u32,
    }

    #[contracttype]
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct EscrowWithId {
        pub bounty_id: u64,
        pub escrow: Escrow,
    }

    #[contracttype]
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct ProgramDelegateInfo {
        pub program_id: String,
        pub delegate: Option<Address>,
        pub permissions: u32,
    }

    #[contract]
    pub struct DummyEscrow;

    #[contractimpl]
    impl DummyEscrow {
        pub fn get_escrow_info(env: Env, bounty_id: u64) -> Result<Escrow, soroban_sdk::Error> {
            if bounty_id == 1 {
                Ok(Escrow {
                    depositor: Address::generate(&env),
                    amount: 1000,
                    remaining_amount: 1000,
                    status: EscrowStatus::Locked,
                    deadline: 123456789,
                    schema_version: 1,
                })
            } else if bounty_id == 2 {
                Ok(Escrow {
                    depositor: Address::generate(&env),
                    amount: 500,
                    remaining_amount: 0,
                    status: EscrowStatus::Released,
                    deadline: 987654321,
                    schema_version: 1,
                })
            } else {
                Err(soroban_sdk::Error::from_contract_error(4)) // BountyNotFound imitation
            }
        }

        pub fn get_metadata(env: Env, bounty_id: u64) -> Result<EscrowMetadata, soroban_sdk::Error> {
            Ok(EscrowMetadata {
                repo_id: 42,
                issue_id: bounty_id * 10,
                bounty_type: String::from_str(&env, "bug-bounty"),
                risk_flags: 0,
                notification_prefs: 0,
                reference_hash: None,
            })
        }

        pub fn get_pause_flags(_env: Env) -> PauseFlags {
            PauseFlags {
                lock_paused: false,
                release_paused: false,
                refund_paused: false,
                pause_reason: None,
                paused_at: 0,
            }
        }

        pub fn query_escrows_by_depositor(
            env: Env,
            depositor: Address,
            _offset: u32,
            _limit: u32,
        ) -> Vec<EscrowWithId> {
            let mut result = Vec::new(&env);
             result.push_back(EscrowWithId {
                bounty_id: 1,
                escrow: Escrow {
                    depositor: depositor.clone(),
                    amount: 1000,
                    remaining_amount: 1000,
                    status: EscrowStatus::Locked,
                    deadline: 123456789,
                    schema_version: 1,
                }
             });
            result
        }

        pub fn query_all_delegates(env: Env, program_id: String) -> Vec<ProgramDelegateInfo> {
            let mut result = Vec::new(&env);
            let delegate = Address::generate(&env);
            result.push_back(ProgramDelegateInfo {
                program_id,
                delegate: Some(delegate),
                permissions: 0x3,
            });
            result
        }
    }
}

#[test]
fn test_get_escrow_summary() {
    let env = Env::default();
    
    // Register Dummy Escrow
    let escrow_contract = env.register_contract(None, dummy_escrow::DummyEscrow);
    
    // Register Facade
    let facade_contract = env.register_contract(None, EscrowViewFacade);
    let facade_client = EscrowViewFacadeClient::new(&env, &facade_contract);

    // Test a happy path: retrieve escrow 1
    let summary_opt = facade_client.get_escrow_summary(&escrow_contract, &1);
    assert!(summary_opt.is_some());
    
    let summary = summary_opt.unwrap();
    assert_eq!(summary.bounty_id, 1);
    assert_eq!(summary.amount, 1000);
    assert_eq!(summary.status, EscrowStatus::Locked);
    assert_eq!(summary.bounty_type, String::from_str(&env, "bug-bounty"));
    assert_eq!(summary.is_paused, false);
}

#[test]
fn test_get_escrow_summary_missing() {
    let env = Env::default();
    let escrow_contract = env.register_contract(None, dummy_escrow::DummyEscrow);
    let facade_contract = env.register_contract(None, EscrowViewFacade);
    let facade_client = EscrowViewFacadeClient::new(&env, &facade_contract);

    // Request non-existent escrow (3)
    let summary_opt = facade_client.get_escrow_summary(&escrow_contract, &3);
    assert!(summary_opt.is_none());
}

#[test]
fn test_get_escrow_summaries_batch() {
    let env = Env::default();
    let escrow_contract = env.register_contract(None, dummy_escrow::DummyEscrow);
    let facade_contract = env.register_contract(None, EscrowViewFacade);
    let facade_client = EscrowViewFacadeClient::new(&env, &facade_contract);

    let mut ids = soroban_sdk::Vec::new(&env);
    ids.push_back(1);
    ids.push_back(2);
    ids.push_back(3); // missing

    let summaries = facade_client.get_escrow_summaries(&escrow_contract, &ids);
    
    // Should skip missing 3
    assert_eq!(summaries.len(), 2);
    assert_eq!(summaries.get(0).unwrap().bounty_id, 1);
    assert_eq!(summaries.get(1).unwrap().bounty_id, 2);
}

#[test]
fn test_get_user_portfolio() {
     let env = Env::default();
    let escrow_contract = env.register_contract(None, dummy_escrow::DummyEscrow);
    let facade_contract = env.register_contract(None, EscrowViewFacade);
    let facade_client = EscrowViewFacadeClient::new(&env, &facade_contract);
    let user = Address::generate(&env);

    let portfolio = facade_client.get_user_portfolio(&escrow_contract, &user);

    // Dummy returns one locked iteration for any depositor
    assert_eq!(portfolio.as_depositor.len(), 1);
    assert_eq!(portfolio.as_depositor.get(0).unwrap().bounty_id, 1);
    
    // Beneficiary lists are empty out-of-the-box until tickets are aggregated
    assert_eq!(portfolio.as_beneficiary.len(), 0);
}

#[test]
fn test_query_all_delegates_via_facade() {
    let env = Env::default();
    let escrow_contract = env.register_contract(None, dummy_escrow::DummyEscrow);
    let facade_contract = env.register_contract(None, EscrowViewFacade);
    let facade_client = EscrowViewFacadeClient::new(&env, &facade_contract);
    let program_id = String::from_str(&env, "program-audit");

    let delegates = facade_client.query_all_delegates(&escrow_contract, &program_id);
    assert_eq!(delegates.len(), 1);
    let info = delegates.get(0).unwrap();
    assert_eq!(info.program_id, program_id);
    assert!(info.delegate.is_some());
    assert_eq!(info.permissions, 0x3);
}
