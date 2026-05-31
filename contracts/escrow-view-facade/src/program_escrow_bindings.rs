// Minimal explicit bindings for ProgramEscrow

use soroban_sdk::{contractclient, contracttype, Address, Env, Error, String, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramDelegateInfo {
    pub program_id: String,
    pub delegate: Option<Address>,
    pub permissions: u32,
}

#[contractclient(name = "Client")]
pub trait ProgramEscrowContract {
    fn query_all_delegates(env: Env, program_id: String) -> Vec<ProgramDelegateInfo>;
}
