use crate::{DataKey, ProgramData, PROGRAM_DATA};
use soroban_sdk::{contracttype, symbol_short, Address, Env, String, Symbol};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClaimStatus {
    Pending,
    Completed,
    Cancelled,
    Expired,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimRecord {
    pub claim_id: u64,
    pub program_id: String,
    pub recipient: Address,
    pub amount: i128,
    pub claim_deadline: u64,
    pub created_at: u64,
    pub status: ClaimStatus,
}

const CLAIM_CREATED: Symbol = symbol_short!("ClmCrtd");
const CLAIM_EXECUTED: Symbol = symbol_short!("ClmExec");
const CLAIM_CANCELLED: Symbol = symbol_short!("ClmCncl");
const NEXT_CLAIM_ID: Symbol = symbol_short!("NxtClmId");

fn next_claim_id(env: &Env) -> u64 {
    let id: u64 = env.storage().instance().get(&NEXT_CLAIM_ID).unwrap_or(1_u64);
    env.storage().instance().set(&NEXT_CLAIM_ID, &(id + 1));
    id
}

fn get_program(env: &Env) -> ProgramData {
    env.storage()
        .instance()
        .get(&PROGRAM_DATA)
        .unwrap_or_else(|| panic!("Program not initialized"))
}

fn save_program(env: &Env, data: &ProgramData) {
    env.storage().instance().set(&PROGRAM_DATA, data);
}

fn claim_key(program_id: &String, claim_id: u64) -> DataKey {
    DataKey::PendingClaim(program_id.clone(), claim_id)
}

fn require_admin(env: &Env, caller: &Address) {
    let stored_admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic!("Not initialized"));
    if *caller != stored_admin {
        panic!("Unauthorized");
    }
    caller.require_auth();
}

fn is_claim_active(record: &ClaimRecord, env: &Env) -> bool {
    record.status == ClaimStatus::Pending && env.ledger().timestamp() < record.claim_deadline
}

fn claim_has_expired(record: &ClaimRecord, env: &Env) -> bool {
    record.status == ClaimStatus::Expired
        || (record.status == ClaimStatus::Pending && env.ledger().timestamp() >= record.claim_deadline)
}

/// Creates a pending claim, reserving `amount` from the escrow balance.
/// The recipient must call `execute_claim` before `claim_deadline`.
/// Returns the generated `claim_id`.
pub fn create_pending_claim(
    env: &Env,
    program_id: &String,
    recipient: &Address,
    amount: i128,
    claim_deadline: u64,
) -> u64 {
    let mut program = get_program(env);
    program.authorized_payout_key.require_auth();

    if amount <= 0 {
        panic!("Amount must be greater than zero");
    }
    if amount > program.remaining_balance {
        panic!("Insufficient escrow balance");
    }
    if claim_deadline <= env.ledger().timestamp() {
        panic!("Claim deadline must be in the future");
    }

    program.remaining_balance -= amount;
    save_program(env, &program);

    let claim_id = next_claim_id(env);
    let record = ClaimRecord {
        claim_id,
        program_id: program_id.clone(),
        recipient: recipient.clone(),
        amount,
        claim_deadline,
        created_at: env.ledger().timestamp(),
        status: ClaimStatus::Pending,
    };

    env.storage().persistent().set(&claim_key(program_id, claim_id), &record);
    env.events().publish(
        (CLAIM_CREATED,),
        (program_id.clone(), claim_id, recipient.clone(), amount, claim_deadline),
    );

    claim_id
}

/// Executes a pending claim before its deadline, transferring funds to the recipient.
pub fn execute_claim(env: &Env, program_id: &String, claim_id: u64, caller: &Address) {
    caller.require_auth();

    let key = claim_key(program_id, claim_id);
    let mut record: ClaimRecord = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic!("Claim not found"));

    if record.recipient != *caller {
        panic!("Unauthorized: only the claim recipient can execute this claim");
    }
    if claim_has_expired(&record, env) {
        panic!("ClaimExpired");
    }
    if !is_claim_active(&record, env) {
        panic!("ClaimAlreadyProcessed");
    }

    let program = get_program(env);
    soroban_sdk::token::Client::new(env, &program.token_address).transfer(
        &env.current_contract_address(),
        &record.recipient,
        &record.amount,
    );

    record.status = ClaimStatus::Completed;
    env.storage().persistent().set(&key, &record);
    env.events().publish(
        (CLAIM_EXECUTED,),
        (program_id.clone(), claim_id, record.recipient.clone(), record.amount),
    );
}

/// Admin cancels a pending or expired claim, returning reserved funds to the escrow balance.
pub fn cancel_claim(env: &Env, program_id: &String, claim_id: u64, admin: &Address) {
    require_admin(env, admin);

    let key = claim_key(program_id, claim_id);
    let mut record: ClaimRecord = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic!("Claim not found"));

    match record.status {
        ClaimStatus::Pending | ClaimStatus::Expired => {}
        _ => panic!("ClaimAlreadyProcessed"),
    }

    let mut program = get_program(env);
    program.remaining_balance += record.amount;
    save_program(env, &program);

    record.status = ClaimStatus::Cancelled;
    env.storage().persistent().set(&key, &record);
    env.events().publish(
        (CLAIM_CANCELLED,),
        (program_id.clone(), claim_id, record.recipient.clone(), record.amount),
    );
}

/// Returns a claim record by its ID. Panics if the claim does not exist.
pub fn get_claim(env: &Env, program_id: &String, claim_id: u64) -> ClaimRecord {
    let key = claim_key(program_id, claim_id);
    let mut record: ClaimRecord = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic!("Claim not found"));
    if record.status == ClaimStatus::Pending && env.ledger().timestamp() >= record.claim_deadline {
        record.status = ClaimStatus::Expired;
        env.storage().persistent().set(&key, &record);
    }
    record
}

/// Sets the global default claim window in seconds. Admin only.
pub fn set_claim_window(env: &Env, admin: &Address, window_seconds: u64) {
    require_admin(env, admin);
    env.storage().instance().set(&DataKey::ClaimWindow, &window_seconds);
}

/// Returns the global default claim window in seconds (default: 86400).
pub fn get_claim_window(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&DataKey::ClaimWindow)
        .unwrap_or(86_400_u64)
}
