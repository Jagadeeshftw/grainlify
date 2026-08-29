// Minimal explicit bindings for ProgramEscrow

pub use ::program_escrow::{PayoutRecord, ProgramDelegateInfo};
use soroban_sdk::{contractclient, Address, Env, String, Vec};

#[contractclient(name = "Client")]
pub trait ProgramEscrowContract {
    fn query_all_delegates(env: Env, program_id: String) -> Vec<ProgramDelegateInfo>;
    fn query_recipient_history(
        env: Env,
        program_id: String,
        recipient: Address,
    ) -> Vec<PayoutRecord>;
}
