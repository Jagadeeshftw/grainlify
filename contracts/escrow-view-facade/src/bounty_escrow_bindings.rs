// Minimal explicit bindings for BountyEscrow

use soroban_sdk::{contractclient, contracttype, Address, String, Vec, Env, Error, BytesN};

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
pub enum AnonymousParty {
    Address(Address),
    Commitment(BytesN<32>),
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

#[contractclient(name = "Client")]
pub trait BountyEscrowContract {
    fn get_escrow_info(env: Env, bounty_id: u64) -> Result<Escrow, Error>;
    fn get_metadata(env: Env, bounty_id: u64) -> Result<EscrowMetadata, Error>;
    fn get_pause_flags(env: Env) -> PauseFlags;
    fn query_escrows_by_depositor(
        env: Env,
        depositor: Address,
        offset: u32,
        limit: u32,
    ) -> Vec<EscrowWithId>;

    fn query_escrows_by_beneficiary(
        env: Env,
        beneficiary: Address,
        offset: u32,
        limit: u32,
    ) -> Vec<EscrowWithId>;
}
