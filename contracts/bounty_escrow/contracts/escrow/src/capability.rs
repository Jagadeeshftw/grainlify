//! Capability token issuance, revocation, and scoped authorisation enforcement.



use soroban_sdk::{Address, BytesN, Env};
use crate::{
    events, reentrancy_guard,
    Capability, CapabilityAction, ClaimRecord, DataKey, Error, Escrow, EscrowStatus,
    COMMITMENT_LIVE_TTL, COMMITMENT_ARCHIVAL_TTL,
    events::{CriticalOperationOutcome, EVENT_VERSION_V2},
};

// ─────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────


pub(crate) fn next_capability_id(env: &Env) -> BytesN<32> {
    let mut id = [0u8; 32];
    let r1: u64 = env.prng().gen();
    let r2: u64 = env.prng().gen();
    let r3: u64 = env.prng().gen();
    let r4: u64 = env.prng().gen();
    id[0..8].copy_from_slice(&r1.to_be_bytes());
    id[8..16].copy_from_slice(&r2.to_be_bytes());
    id[16..24].copy_from_slice(&r3.to_be_bytes());
    id[24..32].copy_from_slice(&r4.to_be_bytes());
    BytesN::from_array(env, &id)
}


pub(crate) fn record_receipt(
    _env: &Env,
    _outcome: CriticalOperationOutcome,
    _bounty_id: u64,
    _amount: i128,
    _recipient: Address,
) {
    // Backward-compatible no-op until receipt storage/events are fully wired.
}


pub(crate) fn load_capability(env: &Env, capability_id: BytesN<32>) -> Result<Capability, Error> {
    let capability: Capability = env
        .storage()
        .persistent()
        .get(&DataKey::Capability(capability_id.clone()))
        .ok_or(Error::CapabilityNotFound)?;
    let archival = capability.revoked
        || capability.remaining_uses == 0
        || capability.remaining_amount == 0
        || env.ledger().timestamp() >= capability.expiry;
    crate::lock::renew_capability_record(env, &capability_id, archival);
    Ok(capability)
}


pub(crate) fn validate_capability_scope_at_issue(
    env: &Env,
    owner: &Address,
    action: &CapabilityAction,
    bounty_id: u64,
    amount_limit: i128,
) -> Result<(), Error> {
    if amount_limit <= 0 {
        return Err(Error::InvalidAmount);
    }

    match action {
        CapabilityAction::Claim => {
            let claim: ClaimRecord = env
                .storage()
                .persistent()
                .get(&DataKey::PendingClaim(bounty_id))
                .ok_or(Error::BountyNotFound)?;
            if claim.claimed {
                return Err(Error::FundsNotLocked);
            }
            if env.ledger().timestamp() > claim.expires_at {
                return Err(Error::DeadlineNotPassed);
            }
            if claim.recipient != owner.clone() {
                return Err(Error::Unauthorized);
            }
            if amount_limit > claim.amount {
                return Err(Error::CapabilityExceedsAuthority);
            }
        }
        CapabilityAction::Release => {
            let admin: Address = env
                .storage()
                .instance()
                .get(&DataKey::Admin)
                .ok_or(Error::NotInitialized)?;
            if admin != owner.clone() {
                return Err(Error::Unauthorized);
            }
            let escrow: Escrow = env
                .storage()
                .persistent()
                .get(&DataKey::Escrow(bounty_id))
                .ok_or(Error::BountyNotFound)?;
            if escrow.status != EscrowStatus::Locked {
                return Err(Error::FundsNotLocked);
            }
            if amount_limit > escrow.remaining_amount {
                return Err(Error::CapabilityExceedsAuthority);
            }
        }
        CapabilityAction::Refund => {
            let admin: Address = env
                .storage()
                .instance()
                .get(&DataKey::Admin)
                .ok_or(Error::NotInitialized)?;
            if admin != owner.clone() {
                return Err(Error::Unauthorized);
            }
            let escrow: Escrow = env
                .storage()
                .persistent()
                .get(&DataKey::Escrow(bounty_id))
                .ok_or(Error::BountyNotFound)?;
            if escrow.status != EscrowStatus::Locked
                && escrow.status != EscrowStatus::PartiallyRefunded
            {
                return Err(Error::FundsNotLocked);
            }
            if amount_limit > escrow.remaining_amount {
                return Err(Error::CapabilityExceedsAuthority);
            }
        }
    }

    Ok(())
}


pub(crate) fn ensure_owner_still_authorized(
    env: &Env,
    capability: &Capability,
    requested_amount: i128,
) -> Result<(), Error> {
    if requested_amount <= 0 {
        return Err(Error::InvalidAmount);
    }

    match capability.action {
        CapabilityAction::Claim => {
            let claim: ClaimRecord = env
                .storage()
                .persistent()
                .get(&DataKey::PendingClaim(capability.bounty_id))
                .ok_or(Error::BountyNotFound)?;
            if claim.claimed {
                return Err(Error::FundsNotLocked);
            }
            if env.ledger().timestamp() > claim.expires_at {
                return Err(Error::DeadlineNotPassed);
            }
            if claim.recipient != capability.owner {
                return Err(Error::Unauthorized);
            }
            if requested_amount > claim.amount {
                return Err(Error::CapabilityExceedsAuthority);
            }
        }
        CapabilityAction::Release => {
            let admin: Address = env
                .storage()
                .instance()
                .get(&DataKey::Admin)
                .ok_or(Error::NotInitialized)?;
            if admin != capability.owner {
                return Err(Error::Unauthorized);
            }
            let escrow: Escrow = env
                .storage()
                .persistent()
                .get(&DataKey::Escrow(capability.bounty_id))
                .ok_or(Error::BountyNotFound)?;
            if escrow.status != EscrowStatus::Locked {
                return Err(Error::FundsNotLocked);
            }
            if requested_amount > escrow.remaining_amount {
                return Err(Error::CapabilityExceedsAuthority);
            }
        }
        CapabilityAction::Refund => {
            let admin: Address = env
                .storage()
                .instance()
                .get(&DataKey::Admin)
                .ok_or(Error::NotInitialized)?;
            if admin != capability.owner {
                return Err(Error::Unauthorized);
            }
            let escrow: Escrow = env
                .storage()
                .persistent()
                .get(&DataKey::Escrow(capability.bounty_id))
                .ok_or(Error::BountyNotFound)?;
            if escrow.status != EscrowStatus::Locked
                && escrow.status != EscrowStatus::PartiallyRefunded
            {
                return Err(Error::FundsNotLocked);
            }
            if requested_amount > escrow.remaining_amount {
                return Err(Error::CapabilityExceedsAuthority);
            }
        }
    }
    Ok(())
}


/// Validates and consumes a capability token for a specific action.
///
/// The capability token must be a secure `BytesN<32>` identifier explicitly issued
/// to the requested `holder` for the requested `bounty_id` and `expected_action`.
/// Consuming a capability securely updates its internal balance and usage counts,
/// protecting against replay attacks or brute-force forgery.
///
/// # Arguments
/// * `env` - The contract environment
/// * `holder` - The address attempting to consume the capability
/// * `capability_id` - The `BytesN<32>` unforgeable token identifier
/// * `expected_action` - The required action mapped to this capability
/// * `bounty_id` - The bounty ID relating to the action
/// * `amount` - The transaction value requested during this consumption limit
///
/// # Returns
/// The updated `Capability` struct successfully verified, or an `Error`.
pub(crate) fn consume_capability(
    env: &Env,
    holder: &Address,
    capability_id: BytesN<32>,
    expected_action: CapabilityAction,
    bounty_id: u64,
    amount: i128,
) -> Result<Capability, Error> {
    let mut capability = load_capability(env, capability_id.clone())?;

    if capability.revoked {
        return Err(Error::CapabilityRevoked);
    }
    if capability.action != expected_action {
        return Err(Error::CapabilityActionMismatch);
    }
    if capability.bounty_id != bounty_id {
        return Err(Error::CapabilityActionMismatch);
    }
    if capability.holder != holder.clone() {
        return Err(Error::Unauthorized);
    }
    if env.ledger().timestamp() > capability.expiry {
        return Err(Error::CapabilityExpired);
    }
    if capability.remaining_uses == 0 {
        return Err(Error::CapabilityUsesExhausted);
    }
    if amount > capability.remaining_amount {
        return Err(Error::CapabilityAmountExceeded);
    }

    holder.require_auth();
    ensure_owner_still_authorized(env, &capability, amount)?;

    capability.remaining_amount -= amount;
    capability.remaining_uses -= 1;
    env.storage()
        .persistent()
        .set(&DataKey::Capability(capability_id.clone()), &capability);
    let archival = capability.remaining_uses == 0 || capability.remaining_amount == 0;
    crate::lock::renew_capability_record(env, &capability_id, archival);

    events::emit_capability_used(
        env,
        events::CapabilityUsed {
            capability_id,
            holder: holder.clone(),
            action: capability.action.clone(),
            bounty_id,
            amount_used: amount,
            remaining_amount: capability.remaining_amount,
            remaining_uses: capability.remaining_uses,
            used_at: env.ledger().timestamp(),
        },
    );

    Ok(capability)
}

// ─────────────────────────────────────────────────────────────────
// Public entry points (dispatcher targets)
// ─────────────────────────────────────────────────────────────────


/// Issues a new capability token for a specific action on a bounty.
///
/// The capability token is represented by a secure, unforgeable `BytesN<32>` identifier
/// generated using the Soroban environment's pseudo-random number generator (PRNG).
/// This ensures that capability tokens cannot be predicted or forged by arbitrary addresses.
///
/// # Arguments
/// * `env` - The contract environment
/// * `owner` - The address delegating authority (e.g. the bounty admin or depositor)
/// * `holder` - The address receiving the capability token
/// * `action` - The specific action authorized (`Release`, `Refund`, etc.)
/// * `bounty_id` - The bounty this capability applies to
/// * `amount_limit` - The maximum amount of funds authorized by this capability
/// * `expiry` - The ledger timestamp when this capability expires
/// * `max_uses` - The maximum number of times this capability can be consumed
///
/// # Returns
/// The generated `BytesN<32>` capability identifier, or an `Error` if issuance fails.
pub fn issue_capability(
    env: Env,
    owner: Address,
    holder: Address,
    action: CapabilityAction,
    bounty_id: u64,
    amount_limit: i128,
    expiry: u64,
    max_uses: u32,
) -> Result<BytesN<32>, Error> {
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }
    if max_uses == 0 {
        return Err(Error::InvalidAmount);
    }

    let now = env.ledger().timestamp();
    if expiry <= now {
        return Err(Error::InvalidDeadline);
    }

    owner.require_auth();
    validate_capability_scope_at_issue(&env, &owner, &action, bounty_id, amount_limit)?;

    let capability_id = next_capability_id(&env);
    let capability = Capability {
        owner: owner.clone(),
        holder: holder.clone(),
        action: action.clone(),
        bounty_id,
        amount_limit,
        remaining_amount: amount_limit,
        expiry,
        remaining_uses: max_uses,
        revoked: false,
    };

    env.storage()
        .persistent()
        .set(&DataKey::Capability(capability_id.clone()), &capability);
    crate::lock::renew_capability_record(&env, &capability_id, false);

    events::emit_capability_issued(
        &env,
        events::CapabilityIssued {
            capability_id: capability_id.clone(),
            owner,
            holder,
            action,
            bounty_id,
            amount_limit,
            expires_at: expiry,
            max_uses,
            timestamp: now,
        },
    );

    Ok(capability_id.clone())
}


pub fn revoke_capability(
    env: Env,
    owner: Address,
    capability_id: BytesN<32>,
) -> Result<(), Error> {
    let mut capability = load_capability(&env, capability_id.clone())?;
    if capability.owner != owner {
        return Err(Error::Unauthorized);
    }
    owner.require_auth();

    if capability.revoked {
        return Ok(());
    }

    capability.revoked = true;
    env.storage()
        .persistent()
        .set(&DataKey::Capability(capability_id.clone()), &capability);
    crate::lock::renew_capability_record(&env, &capability_id, true);

    events::emit_capability_revoked(
        &env,
        events::CapabilityRevoked {
            capability_id,
            owner,
            revoked_at: env.ledger().timestamp(),
        },
    );

    Ok(())
}


pub fn get_capability(env: Env, capability_id: BytesN<32>) -> Result<Capability, Error> {
    load_capability(&env, capability_id.clone())
}

