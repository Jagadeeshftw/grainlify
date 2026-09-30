//! Escrow renewal (deadline extension + top-up) and cycle-chain management.



use soroban_sdk::{symbol_short, token, Address, Env, Vec};
use crate::{
    events, reentrancy_guard, multitoken_invariants,
    CycleLink, DataKey, Error, Escrow, EscrowStatus, FeeConfig, RefundMode, RefundRecord,
    RenewalRecord,
    events::{CriticalOperationOutcome, EVENT_VERSION_V2},
};

// ─────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────


pub(crate) fn default_cycle_link() -> CycleLink {
    CycleLink {
        previous_id: 0,
        next_id: 0,
        cycle: 0,
    }
}

// ─────────────────────────────────────────────────────────────────
// Public entry points (dispatcher targets)
// ─────────────────────────────────────────────────────────────────


/// Extends the deadline of an active escrow and optionally tops up locked funds.
///
/// # Security assumptions
/// - Only `Locked` escrows are renewable.
/// - Renewal is only allowed before the current deadline elapses.
/// - New deadline must strictly increase the current deadline.
/// - Top-ups transfer tokens from the original depositor into this contract.
pub fn renew_escrow(
    env: Env,
    bounty_id: u64,
    new_deadline: u64,
    additional_amount: i128,
) -> Result<(), Error> {
    if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
        return Err(Error::BountyNotFound);
    }

    let mut escrow: Escrow = env
        .storage()
        .persistent()
        .get(&DataKey::Escrow(bounty_id))
        .unwrap();

    crate::pause_freeze::ensure_escrow_not_frozen(&env, bounty_id)?;
    crate::pause_freeze::ensure_address_not_frozen(&env, &escrow.depositor)?;

    if escrow.status != EscrowStatus::Locked {
        return Err(Error::FundsNotLocked);
    }

    let now = env.ledger().timestamp();
    if now >= escrow.deadline {
        return Err(Error::DeadlineNotPassed);
    }
    if new_deadline <= escrow.deadline {
        return Err(Error::InvalidDeadline);
    }
    if additional_amount < 0 {
        return Err(Error::InvalidAmount);
    }

    // The original depositor must authorize every renewal and any top-up transfer.
    escrow.depositor.require_auth();

    if additional_amount > 0 {
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        client.transfer(
            &escrow.depositor,
            &env.current_contract_address(),
            &additional_amount,
        );

        escrow.amount = escrow
            .amount
            .checked_add(additional_amount)
            .ok_or(Error::InvalidAmount)?;
        escrow.remaining_amount = escrow
            .remaining_amount
            .checked_add(additional_amount)
            .ok_or(Error::InvalidAmount)?;
    }

    let old_deadline = escrow.deadline;
    escrow.deadline = new_deadline;
    env.storage()
        .persistent()
        .set(&DataKey::Escrow(bounty_id), &escrow);
    crate::lock::renew_escrow_record(&env, bounty_id, false);

    let mut history: Vec<RenewalRecord> = env
        .storage()
        .persistent()
        .get(&DataKey::RenewalHistory(bounty_id))
        .unwrap_or(Vec::new(&env));
    let cycle = history.len().saturating_add(1);
    history.push_back(RenewalRecord {
        cycle,
        old_deadline,
        new_deadline,
        additional_amount,
        renewed_at: now,
    });
    env.storage()
        .persistent()
        .set(&DataKey::RenewalHistory(bounty_id), &history);

    // INV-2: Verify aggregate balance matches token balance after anon refund
    multitoken_invariants::assert_after_disbursement(&env);
    Ok(())
}


/// Starts a new bounty cycle from a completed prior cycle without mutating prior records.
///
/// # Security assumptions
/// - Previous cycle must be finalized (`Released` or `Refunded`).
/// - A cycle can have at most one direct successor.
/// - New cycle funds are transferred from the original depositor.
pub fn create_next_cycle(
    env: Env,
    previous_bounty_id: u64,
    new_bounty_id: u64,
    amount: i128,
    deadline: u64,
) -> Result<(), Error> {
    if amount <= 0 {
        return Err(Error::InvalidAmount);
    }
    if deadline <= env.ledger().timestamp() {
        return Err(Error::InvalidDeadline);
    }
    if previous_bounty_id == new_bounty_id {
        return Err(Error::BountyExists);
    }
    if env
        .storage()
        .persistent()
        .has(&DataKey::Escrow(new_bounty_id))
    {
        return Err(Error::BountyExists);
    }
    if !env
        .storage()
        .persistent()
        .has(&DataKey::Escrow(previous_bounty_id))
    {
        return Err(Error::BountyNotFound);
    }

    let previous: Escrow = env
        .storage()
        .persistent()
        .get(&DataKey::Escrow(previous_bounty_id))
        .unwrap();
    if previous.status != EscrowStatus::Released && previous.status != EscrowStatus::Refunded {
        return Err(Error::FundsNotLocked);
    }
    crate::lock::renew_escrow_record(&env, previous_bounty_id, true);

    let mut prev_link: CycleLink = env
        .storage()
        .persistent()
        .get(&DataKey::CycleLink(previous_bounty_id))
        .unwrap_or(default_cycle_link());
    if prev_link.next_id != 0 {
        return Err(Error::BountyExists);
    }

    previous.depositor.require_auth();
    let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
    let client = token::Client::new(&env, &token_addr);
    client.transfer(
        &previous.depositor,
        &env.current_contract_address(),
        &amount,
    );

    let new_escrow = Escrow {
        depositor: previous.depositor.clone(),
        amount,
        remaining_amount: amount,
        status: EscrowStatus::Locked,
        deadline,
        refund_history: Vec::new(&env),
        archived: false,
        archived_at: None,
    };
    env.storage()
        .persistent()
        .set(&DataKey::Escrow(new_bounty_id), &new_escrow);
    crate::lock::renew_escrow_record(&env, new_bounty_id, false);

    let mut index: Vec<u64> = env
        .storage()
        .persistent()
        .get(&DataKey::EscrowIndex)
        .unwrap_or(Vec::new(&env));
    index.push_back(new_bounty_id);
    env.storage()
        .persistent()
        .set(&DataKey::EscrowIndex, &index);
    crate::lock::renew_escrow_index(&env, false);

    let mut depositor_index: Vec<u64> = env
        .storage()
        .persistent()
        .get(&DataKey::DepositorIndex(previous.depositor.clone()))
        .unwrap_or(Vec::new(&env));
    depositor_index.push_back(new_bounty_id);
    env.storage().persistent().set(
        &DataKey::DepositorIndex(previous.depositor.clone()),
        &depositor_index,
    );
    crate::lock::renew_depositor_index(&env, &previous.depositor, false);

    prev_link.next_id = new_bounty_id;
    env.storage()
        .persistent()
        .set(&DataKey::CycleLink(previous_bounty_id), &prev_link);

    let new_link = CycleLink {
        previous_id: previous_bounty_id,
        next_id: 0,
        cycle: prev_link.cycle.saturating_add(1),
    };
    env.storage()
        .persistent()
        .set(&DataKey::CycleLink(new_bounty_id), &new_link);

    Ok(())
}


/// Returns the immutable renewal history for `bounty_id`.
pub fn get_renewal_history(env: Env, bounty_id: u64) -> Result<Vec<RenewalRecord>, Error> {
    if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
        return Err(Error::BountyNotFound);
    }

    Ok(env
        .storage()
        .persistent()
        .get(&DataKey::RenewalHistory(bounty_id))
        .unwrap_or(Vec::new(&env)))
}


/// Returns the rollover link metadata for `bounty_id`.
///
/// Returns a default root link with `cycle=1` when no explicit cycle record exists yet.
pub fn get_cycle_info(env: Env, bounty_id: u64) -> Result<CycleLink, Error> {
    if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
        return Err(Error::BountyNotFound);
    }

    let link: CycleLink = env
        .storage()
        .persistent()
        .get(&DataKey::CycleLink(bounty_id))
        .unwrap_or(default_cycle_link());

    if link.previous_id == 0 && link.next_id == 0 && link.cycle == 0 {
        return Ok(CycleLink {
            previous_id: 0,
            next_id: 0,
            cycle: 1,
        });
    }

    Ok(link)
}

