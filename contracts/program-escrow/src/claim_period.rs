// ============================================================
// FILE: contracts/program-escrow/src/claim_period.rs
//
// This module implements claim period support for Issue #66.
//
// To integrate:
// - Add `mod claim_period;` in lib.rs
// - Expose the relevant functions inside the `ProgramEscrowContract` impl block
//
// The required DataKey variants are already defined in lib.rs:
//
//   DataKey::PendingClaim(String, u64)
//     → Maps (program_id, schedule_id) to a ClaimRecord
//
//   DataKey::ClaimWindow
//     → Stores the global claim window duration (in seconds)
//
// ============================================================

use crate::{DataKey, ProgramData, PROGRAM_DATA};
use soroban_sdk::{contracttype, symbol_short, Address, Env, String, Symbol};

/// The status of a pending claim record.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClaimStatus {
    Pending,
    Completed,
    Cancelled,
}

/// Created when an admin approves a payout.
/// This record exists in the window between authorization
/// and the funds being transferred.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimRecord {
    pub claim_id: u64,
    pub program_id: String,
    pub recipient: Address,
    pub amount: i128,
    pub claim_deadline: u64, // UNIX timestamp  shows after which claim expires
    pub created_at: u64,
    pub status: ClaimStatus,
}

// Event symbols
const CLAIM_CREATED: Symbol = symbol_short!("ClmCrtd");
const CLAIM_EXECUTED: Symbol = symbol_short!("ClmExec");
const CLAIM_CANCELLED: Symbol = symbol_short!("ClmCncl");

// Storage key for auto-incrementing claim IDs
const NEXT_CLAIM_ID: Symbol = symbol_short!("NxtClmId");

fn next_claim_id(env: &Env) -> u64 {
    let id: u64 = env
        .storage()
        .instance()
        .get(&NEXT_CLAIM_ID)
        .unwrap_or(1_u64);
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

// ── Public functions ─────────────────────────────────────────
// These functions should be called from the ProgramEscrowContract impl.

/// Creates a new pending claim.
///
/// The authorized payout key reserves `amount` from the escrow balance,
/// moving the payout into a pending state. The recipient must call
/// `execute_claim` before `claim_deadline`, otherwise the claim expires.
///
/// Returns the generated `claim_id`.
pub fn create_pending_claim(
    env: &Env,
    program_id: &String,
    recipient: &Address,
    amount: i128,
    claim_deadline: u64,
) -> u64 {
    let mut program = get_program(env);

    // Only the authorized payout key can create a claim.

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
    // Reserve the funds (deduct from remaining balance)
    program.remaining_balance -= amount;
    save_program(env, &program);

    let claim_id = next_claim_id(env);
    let now = env.ledger().timestamp();

    let record = ClaimRecord {
        claim_id,
        program_id: program_id.clone(),
        recipient: recipient.clone(),
        amount,
        claim_deadline,
        created_at: now,
        status: ClaimStatus::Pending,
    };

    env.storage()
        .persistent()
        .set(&claim_key(program_id, claim_id), &record);

    env.events().publish(
        (CLAIM_CREATED,),
        (
            program_id.clone(),
            claim_id,
            recipient.clone(),
            amount,
            claim_deadline,
        ),
    );

    claim_id
}

// Executes (redeems) a pending claim before its deadline.
//
// Transfers the reserved escrowed funds to the recipient.

pub fn execute_claim(env: &Env, program_id: &String, claim_id: u64, caller: &Address) {
    caller.require_auth();

    let key = claim_key(program_id, claim_id);
    let mut record: ClaimRecord = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic!("Claim not found"));
    // only the designated recipient can execute their own claim
    if record.recipient != *caller {
        panic!("Unauthorized: only the claim recipient can execute this claim");
    }

    // checks if is still pending.
    match record.status {
        ClaimStatus::Pending => {}
        _ => panic!("ClaimAlreadyProcessed"),
    }

    // checks if claim deadline has not expired
    if env.ledger().timestamp() > record.claim_deadline {
        panic!("ClaimExpired");
    }

    // transfer funds to recipient
    let program = get_program(env);
    let token_client = soroban_sdk::token::Client::new(env, &program.token_address);
    token_client.transfer(
        &env.current_contract_address(),
        &record.recipient,
        &record.amount,
    );

    // marks the claim as completed and persist the update.
    record.status = ClaimStatus::Completed;
    env.storage().persistent().set(&key, &record);

    env.events().publish(
        (CLAIM_EXECUTED,),
        (
            program_id.clone(),
            claim_id,
            record.recipient.clone(),
            record.amount,
        ),
    );
}
/// Admin cancels a claim pending or expired and returns reserved funds to escrow.
pub fn cancel_claim(env: &Env, program_id: &String, claim_id: u64, admin: &Address) {
    // Only contract admin can cancel
    let stored_admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic!("Not initialized"));

    if *admin != stored_admin {
        panic!("Unauthorized: only admin can cancel claims");
    }
    admin.require_auth();

    let key = claim_key(program_id, claim_id);
    let mut record: ClaimRecord = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic!("Claim not found"));

    // can only cancel Pending claims (completed claims are final)
    match record.status {
        ClaimStatus::Pending => {}
        _ => panic!("ClaimAlreadyProcessed"),
    }
    // return reserved funds to escrow balance
    let mut program = get_program(env);
    program.remaining_balance += record.amount;
    save_program(env, &program);

    // mark claim as cancelled
    record.status = ClaimStatus::Cancelled;
    env.storage().persistent().set(&key, &record);

    env.events().publish(
        (CLAIM_CANCELLED,),
        (
            program_id.clone(),
            claim_id,
            record.recipient.clone(),
            record.amount,
        ),
    );
}

/// Returns a claim record by its ID.
///
/// Panics if the claim does not exist.
pub fn get_claim(env: &Env, program_id: &String, claim_id: u64) -> ClaimRecord {
    env.storage()
        .persistent()
        .get(&claim_key(program_id, claim_id))
        .unwrap_or_else(|| panic!("Claim not found"))
}

/// Set the global default claim window in seconds.
/// Admin only.
pub fn set_claim_window(env: &Env, admin: &Address, window_seconds: u64) {
    let stored_admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic!("Not initialized"));
    if *admin != stored_admin {
        panic!("Unauthorized");
    }
    admin.require_auth();
    env.storage()
        .instance()
        .set(&DataKey::ClaimWindow, &window_seconds);
}

/// Returns the global default claim window in seconds (default: 86400 = 24h).
pub fn get_claim_window(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&DataKey::ClaimWindow)
        .unwrap_or(86_400_u64)
}
