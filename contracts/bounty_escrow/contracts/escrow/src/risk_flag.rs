//! # Risk Flags Governance — `risk_flags.rs`
//!
//! Branch: `feature/bounty-escrow-26`
//!
//! ## Purpose
//!
//! Provides upgrade-safe, per-bounty risk-flag management with:
//!
//! * Clear per-bit semantics and documentation.
//! * Dual-path mutation: admin-only `set_risk_flags` for direct override;
//!   `propose_risk_flag_change` + `execute_risk_flag_change` for a
//!   timelock-guarded governance path.
//! * Immutable audit trail: every mutation emits a `RiskFlagsUpdated` event
//!   (already defined in `events.rs`) with previous and new values.
//! * Upgrade-safe storage: flags are stored in `DataKey::Metadata(bounty_id)`
//!   alongside the existing `EscrowMetadata` struct, so no new top-level
//!   storage key is required and no migration is needed on upgrade.
//!
//! ## Flag bits (see also `lib.rs` constants)
//!
//! | Bit | Constant                  | Meaning                                        |
//! |-----|---------------------------|------------------------------------------------|
//! | 0   | `RISK_FLAG_HIGH_RISK`     | Elevated risk; highlight in UI / indexers.    |
//! | 1   | `RISK_FLAG_UNDER_REVIEW`  | Manual review in progress; may restrict ops.  |
//! | 2   | `RISK_FLAG_RESTRICTED`    | Compliance hold; informational for integrts.  |
//! | 3   | `RISK_FLAG_DEPRECATED`    | Soft-deprecation; distinct from contract-lvl. |
//!
//! ## Security invariants
//!
//! 1. **Admin-only mutation** — `set_risk_flags` requires `admin.require_auth()`.
//! 2. **Unknown bits rejected** — any `flags` value with bits set outside
//!    `RISK_FLAGS_ALL_KNOWN` returns `Error::InvalidAmount` (repurposed as
//!    "invalid flags" sentinel; see note in `lib.rs`).
//! 3. **Atomic read-modify-write** — flags are read, masked, and written in a
//!    single persistent-storage operation; no partial-update window exists.
//! 4. **Audit trail** — `RiskFlagsUpdated` carries `previous_flags` and
//!    `new_flags`; the ledger event stream is the canonical audit log.
//! 5. **Governance path timelock** — `propose_risk_flag_change` records a
//!    pending proposal; `execute_risk_flag_change` enforces the delay and
//!    applies the change. Proposals can be cancelled by the admin before
//!    execution. This prevents silent flag changes in production.
//! 6. **Reentrancy protection** — all mutating functions acquire and release
//!    the shared reentrancy guard.

use soroban_sdk::{contracttype, symbol_short, Address, Env};

use crate::{
    events::{emit_risk_flags_updated, RiskFlagsUpdated, EVENT_VERSION_V2},
    reentrancy_guard, DataKey, Error, EscrowMetadata,
};

// ── Known-flag mask ───────────────────────────────────────────────────────────

/// Bitmask of all defined flag bits (bits 0–3).
/// Any attempt to set bits outside this mask is rejected.
pub const RISK_FLAGS_ALL_KNOWN: u32 =
    crate::RISK_FLAG_HIGH_RISK
    | crate::RISK_FLAG_UNDER_REVIEW
    | crate::RISK_FLAG_RESTRICTED
    | crate::RISK_FLAG_DEPRECATED;

// ── Governance proposal storage key ──────────────────────────────────────────

/// Storage key for a pending risk-flag governance proposal.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RiskFlagGovernanceKey {
    Proposal(u64), // bounty_id → RiskFlagProposal
}

/// A pending risk-flag change proposed through the governance path.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RiskFlagProposal {
    /// Proposed new flag value.
    pub proposed_flags: u32,
    /// Ledger timestamp before which execution is blocked.
    pub execute_after: u64,
    /// Admin address that created the proposal.
    pub proposed_by: Address,
    /// Ledger timestamp of proposal creation.
    pub proposed_at: u64,
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Load the current `risk_flags` for `bounty_id` from metadata.
/// Returns 0 if no metadata exists.
pub fn current_flags(env: &Env, bounty_id: u64) -> u32 {
    env.storage()
        .persistent()
        .get::<DataKey, EscrowMetadata>(&DataKey::Metadata(bounty_id))
        .map(|m| m.risk_flags)
        .unwrap_or(0)
}

/// Write `new_flags` into the metadata for `bounty_id`, creating a minimal
/// metadata record if none exists yet.
fn write_flags(env: &Env, bounty_id: u64, new_flags: u32) {
    let mut meta: EscrowMetadata = env
        .storage()
        .persistent()
        .get(&DataKey::Metadata(bounty_id))
        .unwrap_or(EscrowMetadata {
            repo_id: 0,
            issue_id: 0,
            bounty_type: soroban_sdk::String::from_str(env, ""),
            risk_flags: 0,
            notification_prefs: 0,
            reference_hash: None,
        });
    meta.risk_flags = new_flags;
    env.storage()
        .persistent()
        .set(&DataKey::Metadata(bounty_id), &meta);
}

/// Validate that `flags` contains only known bits.
fn validate_flags(flags: u32) -> Result<(), Error> {
    if flags & !RISK_FLAGS_ALL_KNOWN != 0 {
        return Err(Error::InvalidAmount); // sentinel: "invalid flags"
    }
    Ok(())
}

// ── Public API (called from `contractimpl BountyEscrowContract`) ──────────────

/// Directly set the risk flags for a bounty (admin-only, immediate).
///
/// # Arguments
/// * `env`       – Soroban execution environment.
/// * `admin`     – Admin address; must be the stored contract admin.
/// * `bounty_id` – Target bounty.
/// * `new_flags` – Full replacement flag value (not a delta).
///
/// # Errors
/// * `Error::NotInitialized` – contract not yet initialised.
/// * `Error::BountyNotFound` – bounty does not exist.
/// * `Error::InvalidAmount`  – `new_flags` contains unknown bits.
/// * `Error::Unauthorized`   – caller is not the admin.
pub fn set_risk_flags(
    env: &Env,
    admin: &Address,
    bounty_id: u64,
    new_flags: u32,
) -> Result<(), Error> {
    reentrancy_guard::acquire(env);

    // Contract must be initialised.
    if !env.storage().instance().has(&DataKey::Admin) {
        reentrancy_guard::release(env);
        return Err(Error::NotInitialized);
    }
    // Bounty must exist (either regular or anonymous).
    if !env.storage().persistent().has(&DataKey::Escrow(bounty_id))
        && !env
            .storage()
            .persistent()
            .has(&DataKey::EscrowAnon(bounty_id))
    {
        reentrancy_guard::release(env);
        return Err(Error::BountyNotFound);
    }

    // Validate the admin identity.
    let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    if &stored_admin != admin {
        reentrancy_guard::release(env);
        return Err(Error::Unauthorized);
    }
    admin.require_auth();

    // Reject unknown bits.
    if let Err(e) = validate_flags(new_flags) {
        reentrancy_guard::release(env);
        return Err(e);
    }

    let previous_flags = current_flags(env, bounty_id);
    write_flags(env, bounty_id, new_flags);

    emit_risk_flags_updated(
        env,
        RiskFlagsUpdated {
            version: EVENT_VERSION_V2,
            bounty_id,
            previous_flags,
            new_flags,
            admin: admin.clone(),
            timestamp: env.ledger().timestamp(),
        },
    );

    reentrancy_guard::release(env);
    Ok(())
}

/// Add specific bits to the current risk flags (admin-only, immediate OR patch).
///
/// Equivalent to `set_risk_flags(env, current | bits_to_add)`.
pub fn add_risk_flags(
    env: &Env,
    admin: &Address,
    bounty_id: u64,
    bits_to_add: u32,
) -> Result<(), Error> {
    reentrancy_guard::acquire(env);

    if !env.storage().instance().has(&DataKey::Admin) {
        reentrancy_guard::release(env);
        return Err(Error::NotInitialized);
    }
    if !env.storage().persistent().has(&DataKey::Escrow(bounty_id))
        && !env
            .storage()
            .persistent()
            .has(&DataKey::EscrowAnon(bounty_id))
    {
        reentrancy_guard::release(env);
        return Err(Error::BountyNotFound);
    }

    let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    if &stored_admin != admin {
        reentrancy_guard::release(env);
        return Err(Error::Unauthorized);
    }
    admin.require_auth();

    if let Err(e) = validate_flags(bits_to_add) {
        reentrancy_guard::release(env);
        return Err(e);
    }

    let previous_flags = current_flags(env, bounty_id);
    let new_flags = previous_flags | bits_to_add;
    write_flags(env, bounty_id, new_flags);

    emit_risk_flags_updated(
        env,
        RiskFlagsUpdated {
            version: EVENT_VERSION_V2,
            bounty_id,
            previous_flags,
            new_flags,
            admin: admin.clone(),
            timestamp: env.ledger().timestamp(),
        },
    );

    reentrancy_guard::release(env);
    Ok(())
}

/// Clear specific bits from the current risk flags (admin-only, immediate).
///
/// Equivalent to `set_risk_flags(env, current & !bits_to_clear)`.
pub fn clear_risk_flags(
    env: &Env,
    admin: &Address,
    bounty_id: u64,
    bits_to_clear: u32,
) -> Result<(), Error> {
    reentrancy_guard::acquire(env);

    if !env.storage().instance().has(&DataKey::Admin) {
        reentrancy_guard::release(env);
        return Err(Error::NotInitialized);
    }
    if !env.storage().persistent().has(&DataKey::Escrow(bounty_id))
        && !env
            .storage()
            .persistent()
            .has(&DataKey::EscrowAnon(bounty_id))
    {
        reentrancy_guard::release(env);
        return Err(Error::BountyNotFound);
    }

    let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    if &stored_admin != admin {
        reentrancy_guard::release(env);
        return Err(Error::Unauthorized);
    }
    admin.require_auth();

    // Allow clearing any subset of known bits; reject unknown bits.
    if let Err(e) = validate_flags(bits_to_clear) {
        reentrancy_guard::release(env);
        return Err(e);
    }

    let previous_flags = current_flags(env, bounty_id);
    let new_flags = previous_flags & !bits_to_clear;
    write_flags(env, bounty_id, new_flags);

    emit_risk_flags_updated(
        env,
        RiskFlagsUpdated {
            version: EVENT_VERSION_V2,
            bounty_id,
            previous_flags,
            new_flags,
            admin: admin.clone(),
            timestamp: env.ledger().timestamp(),
        },
    );

    reentrancy_guard::release(env);
    Ok(())
}

// ── Governance path (timelock-guarded) ───────────────────────────────────────

/// Propose a risk-flag change that will be executable only after `delay_secs`
/// seconds have elapsed.
///
/// Multiple proposals for the same bounty overwrite the previous one (last-
/// proposal-wins semantics; the admin must cancel and re-propose to change
/// details).
///
/// # Arguments
/// * `delay_secs` – Minimum seconds before the proposal may be executed.
///   Must be > 0.
///
/// # Errors
/// * `Error::InvalidDeadline` – `delay_secs == 0`.
/// * `Error::InvalidAmount`   – `proposed_flags` contains unknown bits.
pub fn propose_risk_flag_change(
    env: &Env,
    admin: &Address,
    bounty_id: u64,
    proposed_flags: u32,
    delay_secs: u64,
) -> Result<(), Error> {
    reentrancy_guard::acquire(env);

    if !env.storage().instance().has(&DataKey::Admin) {
        reentrancy_guard::release(env);
        return Err(Error::NotInitialized);
    }
    if !env.storage().persistent().has(&DataKey::Escrow(bounty_id))
        && !env
            .storage()
            .persistent()
            .has(&DataKey::EscrowAnon(bounty_id))
    {
        reentrancy_guard::release(env);
        return Err(Error::BountyNotFound);
    }

    let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    if &stored_admin != admin {
        reentrancy_guard::release(env);
        return Err(Error::Unauthorized);
    }
    admin.require_auth();

    if delay_secs == 0 {
        reentrancy_guard::release(env);
        return Err(Error::InvalidDeadline);
    }
    if let Err(e) = validate_flags(proposed_flags) {
        reentrancy_guard::release(env);
        return Err(e);
    }

    let now = env.ledger().timestamp();
    let execute_after = now.saturating_add(delay_secs);

    let proposal = RiskFlagProposal {
        proposed_flags,
        execute_after,
        proposed_by: admin.clone(),
        proposed_at: now,
    };

    env.storage().persistent().set(
        &RiskFlagGovernanceKey::Proposal(bounty_id),
        &proposal,
    );

    env.events().publish(
        (symbol_short!("rf_prop"), bounty_id),
        (proposed_flags, execute_after, admin.clone()),
    );

    reentrancy_guard::release(env);
    Ok(())
}

/// Execute a previously proposed risk-flag change once the timelock has elapsed.
///
/// Anyone (permissionless) may call this once `execute_after` has passed,
/// preventing liveness dependency on the admin being online.
///
/// # Errors
/// * `Error::BountyNotFound`    – no pending proposal for `bounty_id`.
/// * `Error::DeadlineNotPassed` – `now < proposal.execute_after`.
pub fn execute_risk_flag_change(env: &Env, bounty_id: u64) -> Result<(), Error> {
    reentrancy_guard::acquire(env);

    let proposal: RiskFlagProposal = env
        .storage()
        .persistent()
        .get(&RiskFlagGovernanceKey::Proposal(bounty_id))
        .ok_or_else(|| {
            reentrancy_guard::release(env);
            Error::BountyNotFound // reused as "no proposal found"
        })?;

    let now = env.ledger().timestamp();
    if now < proposal.execute_after {
        reentrancy_guard::release(env);
        return Err(Error::DeadlineNotPassed);
    }

    let previous_flags = current_flags(env, bounty_id);
    write_flags(env, bounty_id, proposal.proposed_flags);

    // Remove the executed proposal.
    env.storage()
        .persistent()
        .remove(&RiskFlagGovernanceKey::Proposal(bounty_id));

    emit_risk_flags_updated(
        env,
        RiskFlagsUpdated {
            version: EVENT_VERSION_V2,
            bounty_id,
            previous_flags,
            new_flags: proposal.proposed_flags,
            admin: proposal.proposed_by.clone(),
            timestamp: now,
        },
    );

    env.events().publish(
        (symbol_short!("rf_exec"), bounty_id),
        (proposal.proposed_flags, proposal.proposed_by),
    );

    reentrancy_guard::release(env);
    Ok(())
}

/// Cancel a pending risk-flag governance proposal (admin-only).
///
/// # Errors
/// * `Error::BountyNotFound` – no pending proposal for `bounty_id`.
pub fn cancel_risk_flag_proposal(
    env: &Env,
    admin: &Address,
    bounty_id: u64,
) -> Result<(), Error> {
    reentrancy_guard::acquire(env);

    if !env.storage().instance().has(&DataKey::Admin) {
        reentrancy_guard::release(env);
        return Err(Error::NotInitialized);
    }

    let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    if &stored_admin != admin {
        reentrancy_guard::release(env);
        return Err(Error::Unauthorized);
    }
    admin.require_auth();

    if !env
        .storage()
        .persistent()
        .has(&RiskFlagGovernanceKey::Proposal(bounty_id))
    {
        reentrancy_guard::release(env);
        return Err(Error::BountyNotFound);
    }

    env.storage()
        .persistent()
        .remove(&RiskFlagGovernanceKey::Proposal(bounty_id));

    env.events().publish(
        (symbol_short!("rf_canc"), bounty_id),
        (admin.clone(), env.ledger().timestamp()),
    );

    reentrancy_guard::release(env);
    Ok(())
}

// ── Read-only queries ─────────────────────────────────────────────────────────

/// Return the current risk flags for `bounty_id` (0 if unset).
pub fn get_risk_flags(env: &Env, bounty_id: u64) -> u32 {
    current_flags(env, bounty_id)
}

/// Return true if all bits in `mask` are set for `bounty_id`.
pub fn has_risk_flags(env: &Env, bounty_id: u64, mask: u32) -> bool {
    current_flags(env, bounty_id) & mask == mask
}

/// Return the pending governance proposal for `bounty_id`, if any.
pub fn get_risk_flag_proposal(env: &Env, bounty_id: u64) -> Option<RiskFlagProposal> {
    env.storage()
        .persistent()
        .get(&RiskFlagGovernanceKey::Proposal(bounty_id))
}