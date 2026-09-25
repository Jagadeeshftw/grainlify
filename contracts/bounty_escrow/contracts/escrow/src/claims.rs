//! Claim ticket lifecycle: authorise, execute, capability-based claim, cancellation, and window config.



use soroban_sdk::{symbol_short, Address, BytesN, Env};
use crate::{
    events, reentrancy_guard,
    Capability, CapabilityAction, ClaimRecord, DataKey, Error, Escrow, EscrowStatus,
    events::{ClaimCancelled, ClaimCreated, ClaimExecuted, CriticalOperationOutcome, EVENT_VERSION_V2},
    DisputeReason,
};

// ─────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────


/// Validates that the current time is within the active claim window for `bounty_id`.
///
/// # Semantics
/// - If no claim window is configured (0 or unset), validation is skipped (permissive).
/// - If a `PendingClaim` exists for the bounty, `now` must be `<= expires_at`.
/// - If no `PendingClaim` exists, validation is skipped (window not yet started).
///
/// # Errors
/// Returns `Error::DeadlineNotPassed` when the claim window has expired.
pub(crate) fn validate_claim_window(env: Env, bounty_id: u64) -> Result<(), Error> {
    let claim_window: u64 = env
        .storage()
        .instance()
        .get(&DataKey::ClaimWindow)
        .unwrap_or(0);

    // No window configured — skip validation entirely.
    if claim_window == 0 {
        return Ok(());
    }

    // No pending claim — window hasn't started yet, skip.
    let claim: ClaimRecord = match env
        .storage()
        .persistent()
        .get(&DataKey::PendingClaim(bounty_id))
    {
        Some(c) => c,
        None => return Ok(()),
    };

    let now = env.ledger().timestamp();

    if now > claim.expires_at {
        events::emit_claim_window_expired(
            &env,
            events::ClaimWindowExpired {
                version: EVENT_VERSION_V2,
                bounty_id,
                now,
                expires_at: claim.expires_at,
            },
        );
        return Err(Error::DeadlineNotPassed);
    }

    events::emit_claim_window_validated(
        &env,
        events::ClaimWindowValidated {
            version: EVENT_VERSION_V2,
            bounty_id,
            now,
            expires_at: claim.expires_at,
        },
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────
// Public entry points (dispatcher targets)
// ─────────────────────────────────────────────────────────────────


/// Set the claim window duration (admin only).
/// `claim_window`: seconds a beneficiary has to claim after release is authorized.
/// Set to `0` to disable claim-window enforcement.
pub fn set_claim_window(env: Env, claim_window: u64) -> Result<(), Error> {
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }
    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();
    env.storage()
        .instance()
        .set(&DataKey::ClaimWindow, &claim_window);
    events::emit_claim_window_set(
        &env,
        events::ClaimWindowSet {
            version: EVENT_VERSION_V2,
            claim_window,
            set_by: admin,
            timestamp: env.ledger().timestamp(),
        },
    );
    Ok(())
}


/// Authorizes a pending claim instead of immediate transfer.
///
/// # Access Control
/// Admin-only.
///
/// # Front-running Behavior
/// Repeated authorizations are overwrite semantics: the latest successful authorization for
/// a locked bounty replaces the previous pending recipient/record.
pub fn authorize_claim(
    env: Env,
    bounty_id: u64,
    recipient: Address,
    reason: DisputeReason,
) -> Result<(), Error> {
    if crate::pause_freeze::check_paused(&env, symbol_short!("release")) {
        return Err(Error::FundsPaused);
    }
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }
    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();

    if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
        return Err(Error::BountyNotFound);
    }

    let escrow: Escrow = env
        .storage()
        .persistent()
        .get(&DataKey::Escrow(bounty_id))
        .unwrap();

    crate::pause_freeze::ensure_escrow_not_frozen(&env, bounty_id)?;
    crate::pause_freeze::ensure_address_not_frozen(&env, &escrow.depositor)?;

    if escrow.status != EscrowStatus::Locked {
        return Err(Error::FundsNotLocked);
    }

    // Centralize liability accounting for claims: a pending claim may only
    // reserve what is still owed after prior partial withdrawals/refunds.
    // Capturing the *current* remaining liability here (instead of the full
    // original `escrow.amount`) guarantees a claim can never be authorized
    // for more than the escrow actually still owes. (Issue #1809)
    let claim_amount = escrow.remaining_amount;
    if claim_amount <= 0 {
        return Err(Error::InvalidAmount);
    }

    let now = env.ledger().timestamp();
    let claim_window: u64 = env
        .storage()
        .instance()
        .get(&DataKey::ClaimWindow)
        .unwrap_or(0);
    let claim = ClaimRecord {
        bounty_id,
        recipient: recipient.clone(),
        amount: claim_amount,
        expires_at: now.saturating_add(claim_window),
        claimed: false,
        reason,
    };

    env.storage()
        .persistent()
        .set(&DataKey::PendingClaim(bounty_id), &claim);
    crate::lock::renew_claim_record(&env, bounty_id, false);

    env.events().publish(
        (symbol_short!("claim"), symbol_short!("created")),
        ClaimCreated {
            bounty_id,
            recipient,
            amount: escrow.amount,
            expires_at: claim.expires_at,
        },
    );
    Ok(())
}


/// Claims an existing pending authorization.
///
/// # Access Control
/// Only the authorized pending `recipient` can claim.
///
/// # Front-running Behavior
/// Claim is single-use: once marked claimed and escrow is released, subsequent calls fail.
pub fn claim(env: Env, bounty_id: u64) -> Result<(), Error> {
    validate_claim_window(env.clone(), bounty_id)?;
    // GUARD: acquire reentrancy lock
    reentrancy_guard::acquire(&env);

    if crate::pause_freeze::check_paused(&env, symbol_short!("release")) {
        reentrancy_guard::release(&env);
        return Err(Error::FundsPaused);
    }
    if !env
        .storage()
        .persistent()
        .has(&DataKey::PendingClaim(bounty_id))
    {
        reentrancy_guard::release(&env);
        return Err(Error::BountyNotFound);
    }
    let mut claim: ClaimRecord = env
        .storage()
        .persistent()
        .get(&DataKey::PendingClaim(bounty_id))
        .unwrap();

    claim.recipient.require_auth();

    let now = env.ledger().timestamp();
    if now > claim.expires_at {
        return Err(Error::DeadlineNotPassed); // reuse or add ClaimExpired error
    }
    if claim.claimed {
        reentrancy_guard::release(&env);
        return Err(Error::FundsNotLocked);
    }

    // EFFECTS: update state before external call (CEI)
    let mut escrow: Escrow = env
        .storage()
        .persistent()
        .get(&DataKey::Escrow(bounty_id))
        .unwrap();
    crate::pause_freeze::ensure_escrow_not_frozen(&env, bounty_id)?;
    crate::pause_freeze::ensure_address_not_frozen(&env, &escrow.depositor)?;

    // Centralize liability decrement (Issue #1809): a claim may only draw
    // against the remaining liability still held by the escrow. If a partial
    // withdrawal/refund already reduced `remaining_amount` below the claimed
    // amount, executing the claim would overdraw the escrow's on-chain
    // balance, so reject it (INV-2: sum of remaining == contract balance).
    if claim.amount > escrow.remaining_amount {
        reentrancy_guard::release(&env);
        return Err(Error::InsufficientFunds);
    }
    escrow.remaining_amount = escrow.remaining_amount.checked_sub(claim.amount).unwrap();
    if escrow.remaining_amount == 0 {
        escrow.status = EscrowStatus::Released;
    }
    env.storage()
        .persistent()
        .set(&DataKey::Escrow(bounty_id), &escrow);
    crate::lock::renew_escrow_record(
        &env,
        bounty_id,
        escrow.remaining_amount == 0,
    );

    claim.claimed = true;
    env.storage()
        .persistent()
        .set(&DataKey::PendingClaim(bounty_id), &claim);
    crate::lock::renew_claim_record(&env, bounty_id, true);

    // INTERACTION: external token transfer is last
    let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
    let client = token::Client::new(&env, &token_addr);
    client.transfer(
        &env.current_contract_address(),
        &claim.recipient,
        &claim.amount,
    );

    env.events().publish(
        (symbol_short!("claim"), symbol_short!("done")),
        ClaimExecuted {
            bounty_id,
            recipient: claim.recipient.clone(),
            amount: claim.amount,
            claimed_at: now,
        },
    );

    // INV-2: Verify aggregate balance matches token balance after claim
    multitoken_invariants::assert_after_disbursement(&env);

    // GUARD: release reentrancy lock
    reentrancy_guard::release(&env);
    Ok(())
}


/// Delegated claim execution using a capability.
/// Funds are still transferred to the pending claim recipient.
pub fn claim_with_capability(
    env: Env,
    bounty_id: u64,
    holder: Address,
    capability_id: BytesN<32>,
) -> Result<(), Error> {
    // GUARD: acquire reentrancy lock
    reentrancy_guard::acquire(&env);

    if crate::pause_freeze::check_paused(&env, symbol_short!("release")) {
        reentrancy_guard::release(&env);
        return Err(Error::FundsPaused);
    }
    if !env
        .storage()
        .persistent()
        .has(&DataKey::PendingClaim(bounty_id))
    {
        reentrancy_guard::release(&env);
        return Err(Error::BountyNotFound);
    }

    let mut claim: ClaimRecord = env
        .storage()
        .persistent()
        .get(&DataKey::PendingClaim(bounty_id))
        .unwrap();

    let now = env.ledger().timestamp();
    if now > claim.expires_at {
        reentrancy_guard::release(&env);
        return Err(Error::DeadlineNotPassed);
    }
    if claim.claimed {
        reentrancy_guard::release(&env);
        return Err(Error::FundsNotLocked);
    }

    crate::capability::consume_capability(
        &env,
        &holder,
        capability_id,
        CapabilityAction::Claim,
        bounty_id,
        claim.amount,
    )?;

    // EFFECTS: update state before external call (CEI)
    let mut escrow: Escrow = env
        .storage()
        .persistent()
        .get(&DataKey::Escrow(bounty_id))
        .unwrap();
    crate::pause_freeze::ensure_escrow_not_frozen(&env, bounty_id)?;
    crate::pause_freeze::ensure_address_not_frozen(&env, &escrow.depositor)?;

    // Centralize liability decrement (Issue #1809): see `claim`.
    if claim.amount > escrow.remaining_amount {
        reentrancy_guard::release(&env);
        return Err(Error::InsufficientFunds);
    }
    escrow.remaining_amount = escrow.remaining_amount.checked_sub(claim.amount).unwrap();
    if escrow.remaining_amount == 0 {
        escrow.status = EscrowStatus::Released;
    }
    env.storage()
        .persistent()
        .set(&DataKey::Escrow(bounty_id), &escrow);
    crate::lock::renew_escrow_record(
        &env,
        bounty_id,
        escrow.remaining_amount == 0,
    );

    claim.claimed = true;
    env.storage()
        .persistent()
        .set(&DataKey::PendingClaim(bounty_id), &claim);
    crate::lock::renew_claim_record(&env, bounty_id, true);

    // INTERACTION: external token transfer is last
    let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
    let client = token::Client::new(&env, &token_addr);
    client.transfer(
        &env.current_contract_address(),
        &claim.recipient,
        &claim.amount,
    );

    env.events().publish(
        (symbol_short!("claim"), symbol_short!("done")),
        ClaimExecuted {
            bounty_id,
            recipient: claim.recipient,
            amount: claim.amount,
            claimed_at: now,
        },
    );

    // INV-2: Verify aggregate balance matches token balance after claim
    multitoken_invariants::assert_after_disbursement(&env);

    // GUARD: release reentrancy lock
    reentrancy_guard::release(&env);
    Ok(())
}


/// Admin can cancel an expired or unwanted pending claim, returning escrow to Locked.
pub fn cancel_pending_claim(
    env: Env,
    bounty_id: u64,
    _outcome: DisputeOutcome,
) -> Result<(), Error> {
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }
    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();

    // Idempotency (Issue #1809): cancellation is a no-op if the claim was
    // already cancelled/consumed — as long as the bounty itself still exists.
    // A missing escrow means the bounty never existed, which is a real error.
    let escrow_exists = env.storage().persistent().has(&DataKey::Escrow(bounty_id))
        || env.storage().persistent().has(&DataKey::EscrowAnon(bounty_id));
    if !env
        .storage()
        .persistent()
        .has(&DataKey::PendingClaim(bounty_id))
    {
        if escrow_exists {
            return Ok(());
        }
        return Err(Error::BountyNotFound);
    }
    let claim: ClaimRecord = env
        .storage()
        .persistent()
        .get(&DataKey::PendingClaim(bounty_id))
        .unwrap();

    let now = env.ledger().timestamp(); // Added this line
    let recipient = claim.recipient.clone(); // Added this line
    let amount = claim.amount; // Added this line

    env.storage()
        .persistent()
        .remove(&DataKey::PendingClaim(bounty_id));
    env.storage()
        .persistent()
        .remove(&DataKey::ClaimTtl(bounty_id));

    env.events().publish(
        (symbol_short!("claim"), symbol_short!("cancel")),
        ClaimCancelled {
            bounty_id,
            recipient,
            amount,
            cancelled_at: now,
            cancelled_by: admin,
        },
    );
    Ok(())
}


/// View: get pending claim for a bounty.
pub fn get_pending_claim(env: Env, bounty_id: u64) -> Result<ClaimRecord, Error> {
    let claim: ClaimRecord = env
        .storage()
        .persistent()
        .get(&DataKey::PendingClaim(bounty_id))
        .ok_or(Error::BountyNotFound)?;
    let archival = claim.claimed || env.ledger().timestamp() >= claim.expires_at;
    crate::lock::renew_claim_record(&env, bounty_id, archival);
    Ok(claim)
}

