//! Claim tickets: single-use, expiring payout vouchers.
//!
//! The admin or the escrow depositor issues a ticket naming a beneficiary and
//! an amount; the beneficiary later redeems it for a direct token payout.
//!
//! ## Reservation
//! Issuing a ticket reserves its amount against the escrow
//! (`DataKey::TicketReserved(bounty_id)`). Release, refund and pending-claim
//! paths call [`ensure_unreserved`] so they cannot spend reserved funds, which
//! guarantees every outstanding ticket stays fully backed. The reservation is
//! returned when the ticket is redeemed or revoked; an expired ticket keeps its
//! reservation until the issuer revokes it.
//!
//! ## Redemption (CEI)
//! `claim_with_ticket` holds the reentrancy guard, marks the ticket `used` and
//! debits the escrow before the token transfer.

use crate::{
    events::{self, TicketClaimed, TicketIssued, TicketRevoked},
    multitoken_invariants, reentrancy_guard, ClaimRecord, ClaimTicket, DataKey, Error, Escrow,
    EscrowStatus, CLAIM_ARCHIVAL_TTL, CLAIM_LIVE_TTL, TTL_RENEWAL_DIVISOR,
};
use soroban_sdk::{symbol_short, token, Address, Env, Vec};

/// Maximum page size for [`get_beneficiary_tickets`].
pub const MAX_TICKET_PAGE_SIZE: u32 = 100;

/// Maximum ticket ids kept in a beneficiary's index. Beyond this, tickets are
/// still issued but only discoverable via `TicketIssued` events, so nobody can
/// block issuance to a beneficiary by filling their index entry.
pub const MAX_BENEFICIARY_INDEX: u32 = 200;

// ─────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────

/// Total amount reserved by outstanding (unredeemed, unrevoked) tickets.
pub(crate) fn reserved_amount(env: &Env, bounty_id: u64) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::TicketReserved(bounty_id))
        .unwrap_or(0)
}

fn set_reserved_amount(env: &Env, bounty_id: u64, amount: i128) {
    let key = DataKey::TicketReserved(bounty_id);
    if amount == 0 {
        env.storage().persistent().remove(&key);
    } else {
        env.storage().persistent().set(&key, &amount);
    }
}

fn outstanding_tickets(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::OutstandingTickets)
        .unwrap_or(0)
}

/// Tracks outstanding tickets in instance storage so payout paths can skip
/// the per-bounty reservation lookup when no tickets exist.
fn adjust_outstanding_tickets(env: &Env, increment: bool) {
    let current = outstanding_tickets(env);
    let next = if increment {
        current + 1
    } else {
        current.saturating_sub(1)
    };
    if next == 0 {
        env.storage()
            .instance()
            .remove(&DataKey::OutstandingTickets);
    } else {
        env.storage()
            .instance()
            .set(&DataKey::OutstandingTickets, &next);
    }
}

/// Fails with `InsufficientFunds` when spending `amount` from an escrow with
/// `remaining` would dip into funds reserved by outstanding claim tickets.
pub(crate) fn ensure_unreserved(
    env: &Env,
    bounty_id: u64,
    remaining: i128,
    amount: i128,
) -> Result<(), Error> {
    if outstanding_tickets(env) == 0 {
        return Ok(());
    }
    let reserved = reserved_amount(env, bounty_id);
    if reserved > 0 && amount > remaining.saturating_sub(reserved) {
        return Err(Error::InsufficientFunds);
    }
    Ok(())
}

fn load_ticket(env: &Env, ticket_id: u64) -> Result<ClaimTicket, Error> {
    env.storage()
        .persistent()
        .get(&DataKey::ClaimTicket(ticket_id))
        .ok_or(Error::TicketNotFound)
}

fn store_ticket(env: &Env, ticket: &ClaimTicket, archival: bool) {
    let key = DataKey::ClaimTicket(ticket.ticket_id);
    env.storage().persistent().set(&key, ticket);
    let ttl = if archival {
        CLAIM_ARCHIVAL_TTL
    } else {
        CLAIM_LIVE_TTL
    };
    env.storage()
        .persistent()
        .extend_ttl(&key, ttl / TTL_RENEWAL_DIVISOR, ttl);
}

fn load_escrow(env: &Env, bounty_id: u64) -> Result<Escrow, Error> {
    env.storage()
        .persistent()
        .get(&DataKey::Escrow(bounty_id))
        .ok_or(Error::BountyNotFound)
}

/// The admin and the escrow depositor are the only parties allowed to issue
/// or revoke tickets for a bounty.
fn ensure_issuer(env: &Env, issuer: &Address, escrow: &Escrow) -> Result<(), Error> {
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(Error::NotInitialized)?;
    if *issuer != admin && *issuer != escrow.depositor {
        return Err(Error::Unauthorized);
    }
    Ok(())
}

fn next_ticket_id(env: &Env) -> u64 {
    let next: u64 = env
        .storage()
        .instance()
        .get(&DataKey::ClaimTicketCounter)
        .unwrap_or(0)
        + 1;
    env.storage()
        .instance()
        .set(&DataKey::ClaimTicketCounter, &next);
    next
}

// ─────────────────────────────────────────────────────────────────
// Entry points (dispatcher targets)
// ─────────────────────────────────────────────────────────────────

/// Issue a single-use ticket entitling `beneficiary` to `amount` from
/// `bounty_id` until `expires_at` (exclusive). Returns the new ticket id.
///
/// # Access control
/// `issuer` must authorize and be the admin or the escrow depositor.
///
/// # Errors
/// `FundsPaused`, `NotInitialized`, `BountyNotFound`, `Unauthorized`,
/// `EscrowFrozen`, `AddressFrozen`, `FundsNotLocked`, `ClaimPending`, `InvalidAmount`
/// (non-positive or above the unreserved remaining amount), `InvalidDeadline`
/// (`expires_at <= now`).
pub fn issue_claim_ticket(
    env: Env,
    issuer: Address,
    bounty_id: u64,
    beneficiary: Address,
    amount: i128,
    expires_at: u64,
) -> Result<u64, Error> {
    if crate::pause_freeze::check_paused(&env, symbol_short!("release")) {
        return Err(Error::FundsPaused);
    }
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }
    issuer.require_auth();

    let escrow = load_escrow(&env, bounty_id)?;
    ensure_issuer(&env, &issuer, &escrow)?;
    crate::pause_freeze::ensure_escrow_not_frozen(&env, bounty_id)?;
    crate::pause_freeze::ensure_address_not_frozen(&env, &escrow.depositor)?;

    if escrow.status != EscrowStatus::Locked {
        return Err(Error::FundsNotLocked);
    }
    // An admin-authorized pending claim takes precedence over new tickets.
    if let Some(claim) = env
        .storage()
        .persistent()
        .get::<DataKey, ClaimRecord>(&DataKey::PendingClaim(bounty_id))
    {
        if !claim.claimed {
            return Err(Error::ClaimPending);
        }
    }
    if amount <= 0 {
        return Err(Error::InvalidAmount);
    }
    let now = env.ledger().timestamp();
    if expires_at <= now {
        return Err(Error::InvalidDeadline);
    }

    let reserved = reserved_amount(&env, bounty_id);
    let available = escrow.remaining_amount.saturating_sub(reserved);
    if amount > available {
        return Err(Error::InvalidAmount);
    }

    let ticket_id = next_ticket_id(&env);
    let ticket = ClaimTicket {
        ticket_id,
        bounty_id,
        beneficiary: beneficiary.clone(),
        amount,
        expires_at,
        used: false,
        issued_at: now,
        revoked: false,
    };
    store_ticket(&env, &ticket, false);
    set_reserved_amount(&env, bounty_id, reserved + amount);
    adjust_outstanding_tickets(&env, true);

    let index_key = DataKey::BeneficiaryTickets(beneficiary.clone());
    let mut index: Vec<u64> = env
        .storage()
        .persistent()
        .get(&index_key)
        .unwrap_or_else(|| Vec::new(&env));
    if index.len() < MAX_BENEFICIARY_INDEX {
        index.push_back(ticket_id);
        env.storage().persistent().set(&index_key, &index);
    }

    events::emit_ticket_issued(
        &env,
        TicketIssued {
            ticket_id,
            bounty_id,
            beneficiary,
            amount,
            expires_at,
            issued_at: now,
        },
    );
    Ok(ticket_id)
}

/// Redeem `ticket_id`, paying its amount from the escrow to `claimer`.
///
/// # Access control
/// `claimer` must authorize and be the ticket beneficiary.
///
/// # Errors
/// `FundsPaused`, `TicketNotFound`, `Unauthorized` (wrong beneficiary),
/// `TicketInvalid` (revoked), `TicketAlreadyUsed`, `TicketExpired`
/// (`now >= expires_at`), `BountyNotFound`, `EscrowFrozen`, `AddressFrozen`,
/// `FundsNotLocked`, `InsufficientFunds`.
pub fn claim_with_ticket(env: Env, ticket_id: u64, claimer: Address) -> Result<(), Error> {
    reentrancy_guard::acquire(&env);
    let result = redeem(&env, ticket_id, claimer);
    reentrancy_guard::release(&env);
    result
}

fn redeem(env: &Env, ticket_id: u64, claimer: Address) -> Result<(), Error> {
    // CHECKS
    if crate::pause_freeze::check_paused(env, symbol_short!("release")) {
        return Err(Error::FundsPaused);
    }
    let mut ticket = load_ticket(env, ticket_id)?;
    claimer.require_auth();
    if claimer != ticket.beneficiary {
        return Err(Error::Unauthorized);
    }
    if ticket.revoked {
        return Err(Error::TicketInvalid);
    }
    if ticket.used {
        return Err(Error::TicketAlreadyUsed);
    }
    let now = env.ledger().timestamp();
    if now >= ticket.expires_at {
        return Err(Error::TicketExpired);
    }

    let bounty_id = ticket.bounty_id;
    let mut escrow = load_escrow(env, bounty_id)?;
    crate::pause_freeze::ensure_escrow_not_frozen(env, bounty_id)?;
    crate::pause_freeze::ensure_address_not_frozen(env, &escrow.depositor)?;
    if escrow.status != EscrowStatus::Locked && escrow.status != EscrowStatus::PartiallyRefunded {
        return Err(Error::FundsNotLocked);
    }
    let reserved = reserved_amount(env, bounty_id);
    if ticket.amount > escrow.remaining_amount || ticket.amount > reserved {
        return Err(Error::InsufficientFunds);
    }

    // EFFECTS: consume the ticket and debit the escrow before transferring.
    ticket.used = true;
    store_ticket(env, &ticket, true);
    set_reserved_amount(env, bounty_id, reserved - ticket.amount);
    adjust_outstanding_tickets(env, false);

    escrow.remaining_amount -= ticket.amount;
    if escrow.remaining_amount == 0 {
        escrow.status = EscrowStatus::Released;
    }
    env.storage()
        .persistent()
        .set(&DataKey::Escrow(bounty_id), &escrow);
    crate::lock::renew_escrow_record(env, bounty_id, escrow.status == EscrowStatus::Released);

    // INTERACTION
    let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
    token::Client::new(env, &token_addr).transfer(
        &env.current_contract_address(),
        &claimer,
        &ticket.amount,
    );

    events::emit_ticket_claimed(
        env,
        TicketClaimed {
            ticket_id,
            bounty_id,
            claimer,
            claimed_at: now,
            amount: ticket.amount,
        },
    );

    multitoken_invariants::assert_after_disbursement(env);
    Ok(())
}

/// Revoke an outstanding ticket and return its reservation to the escrow.
/// Expired tickets may be revoked to free their reservation.
///
/// # Access control
/// `caller` must authorize and be the admin or the escrow depositor.
///
/// # Errors
/// `TicketNotFound`, `BountyNotFound`, `Unauthorized`, `TicketInvalid`
/// (already revoked), `TicketAlreadyUsed`.
pub fn revoke_claim_ticket(env: Env, caller: Address, ticket_id: u64) -> Result<(), Error> {
    let mut ticket = load_ticket(&env, ticket_id)?;
    caller.require_auth();
    let escrow = load_escrow(&env, ticket.bounty_id)?;
    ensure_issuer(&env, &caller, &escrow)?;
    if ticket.revoked {
        return Err(Error::TicketInvalid);
    }
    if ticket.used {
        return Err(Error::TicketAlreadyUsed);
    }

    ticket.revoked = true;
    store_ticket(&env, &ticket, true);
    adjust_outstanding_tickets(&env, false);
    let reserved = reserved_amount(&env, ticket.bounty_id);
    set_reserved_amount(
        &env,
        ticket.bounty_id,
        reserved.saturating_sub(ticket.amount).max(0),
    );

    events::emit_ticket_revoked(
        &env,
        TicketRevoked {
            ticket_id,
            bounty_id: ticket.bounty_id,
            beneficiary: ticket.beneficiary,
            amount: ticket.amount,
            revoked_by: caller,
            revoked_at: env.ledger().timestamp(),
        },
    );
    Ok(())
}

/// View: fetch a ticket by id.
pub fn get_claim_ticket(env: Env, ticket_id: u64) -> Result<ClaimTicket, Error> {
    load_ticket(&env, ticket_id)
}

/// View: page through the ticket ids issued to `beneficiary`, in issue order
/// (first [`MAX_BENEFICIARY_INDEX`] tickets only).
/// `limit` is capped at [`MAX_TICKET_PAGE_SIZE`].
pub fn get_beneficiary_tickets(
    env: Env,
    beneficiary: Address,
    offset: u32,
    limit: u32,
) -> Vec<u64> {
    let all: Vec<u64> = env
        .storage()
        .persistent()
        .get(&DataKey::BeneficiaryTickets(beneficiary))
        .unwrap_or_else(|| Vec::new(&env));
    let mut page = Vec::new(&env);
    let end = offset
        .saturating_add(limit.min(MAX_TICKET_PAGE_SIZE))
        .min(all.len());
    let mut i = offset;
    while i < end {
        page.push_back(all.get_unchecked(i));
        i += 1;
    }
    page
}

/// View: amount of `bounty_id` reserved by outstanding tickets.
pub fn get_ticket_reserved_amount(env: Env, bounty_id: u64) -> i128 {
    reserved_amount(&env, bounty_id)
}
