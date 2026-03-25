#![allow(unused)]
//! # Identity Module for Escrow Contract
//!
//! Handles off-chain identity claims, signature verification, tier-based limits,
//! and **address binding rules** to prevent spoofed identities.
//!
//! ## Address Binding Rules
//!
//! - Each address may hold **at most one** active identity binding at a time.
//! - Binding is created via [`bind_identity`] (requires admin auth).
//! - Binding can be revoked via [`unbind_identity`] (requires admin auth).
//! - [`validate_binding`] checks that a claim's address matches a valid binding
//!   before the claim is accepted, preventing spoofed or replayed claims.
//! - A nonce per address prevents replay of old binding payloads.
//!
//! ## Security Assumptions
//!
//! 1. Only authorized issuers may produce valid claim signatures.
//! 2. `bind_identity` and `unbind_identity` are admin-only operations.
//! 3. The nonce is incremented on every bind, so old binding payloads cannot be
//!    replayed even if an attacker intercepts them.
//! 4. Expired claims are treated as `Unverified` regardless of binding state.

use soroban_sdk::xdr::ToXdr;
use soroban_sdk::{contracttype, Address, Bytes, BytesN, Env};

use crate::Error;

// ============================================================================
// Types
// ============================================================================

/// Identity tier levels for KYC verification.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum IdentityTier {
    Unverified = 0,
    Basic = 1,
    Verified = 2,
    Premium = 3,
}

/// Identity claim structure signed by authorized issuers.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityClaim {
    pub address: Address,
    pub tier: IdentityTier,
    pub risk_score: u32, // 0-100
    pub expiry: u64,     // Unix timestamp
    pub issuer: Address, // Issuer public key
}

/// Stored identity data for an address.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AddressIdentity {
    pub tier: IdentityTier,
    pub risk_score: u32,
    pub expiry: u64,
    pub last_updated: u64,
}

/// Configuration for tier-based transaction limits.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TierLimits {
    pub unverified_limit: i128,
    pub basic_limit: i128,
    pub verified_limit: i128,
    pub premium_limit: i128,
}

/// Configuration for risk-based limit adjustments.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RiskThresholds {
    pub high_risk_threshold: u32,  // e.g., 70
    pub high_risk_multiplier: u32, // e.g., 50 (50% of tier limit)
}

/// On-chain record that an address has been bound to an identity.
///
/// Stored under `DataKey::IdentityBinding(address)`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityBinding {
    /// The issuer who vouched for this address.
    pub bound_issuer: Address,
    /// Ledger timestamp when the binding was created.
    pub bound_at: u64,
    /// Monotonic counter — prevents replay of stale bind payloads.
    pub nonce: u64,
    /// Whether the binding is currently active.
    pub active: bool,
}

// ============================================================================
// Default Implementations
// ============================================================================

impl Default for AddressIdentity {
    fn default() -> Self {
        Self {
            tier: IdentityTier::Unverified,
            risk_score: 0,
            expiry: 0,
            last_updated: 0,
        }
    }
}

impl Default for TierLimits {
    fn default() -> Self {
        Self {
            unverified_limit: 100_0000000, // 100 tokens (7 decimals)
            basic_limit: 1000_0000000,     // 1,000 tokens
            verified_limit: 10000_0000000, // 10,000 tokens
            premium_limit: 100000_0000000, // 100,000 tokens
        }
    }
}

impl Default for RiskThresholds {
    fn default() -> Self {
        Self {
            high_risk_threshold: 70,
            high_risk_multiplier: 50, // 50% of tier limit
        }
    }
}

// ============================================================================
// Address Binding — anti-spoofing
// ============================================================================

/// Create or refresh an identity binding for `address`.
///
/// # Arguments
/// * `env`    — contract environment.
/// * `address` — the user address to bind.
/// * `issuer`  — the authorized issuer vouching for this binding.
///
/// # Returns
/// The new [`IdentityBinding`] record, or an error if the issuer is not
/// authorized.
///
/// # Security
/// Caller must have already verified admin authorization before invoking this
/// function (the check lives in `EscrowContract::bind_identity`).
pub fn bind_identity(
    env: &Env,
    address: &Address,
    issuer: &Address,
) -> Result<IdentityBinding, Error> {
    use crate::DataKey;

    // Check issuer authorization
    let is_authorized: bool = env
        .storage()
        .persistent()
        .get(&DataKey::AuthorizedIssuer(issuer.clone()))
        .unwrap_or(false);
    if !is_authorized {
        return Err(Error::UnauthorizedIssuer);
    }

    // Load existing binding (if any) to increment nonce
    let prev_nonce: u64 = env
        .storage()
        .persistent()
        .get::<_, IdentityBinding>(&DataKey::IdentityBinding(address.clone()))
        .map(|b| b.nonce)
        .unwrap_or(0);

    let binding = IdentityBinding {
        bound_issuer: issuer.clone(),
        bound_at: env.ledger().timestamp(),
        nonce: prev_nonce + 1,
        active: true,
    };

    env.storage()
        .persistent()
        .set(&DataKey::IdentityBinding(address.clone()), &binding);

    // Emit binding event
    env.events().publish(
        (
            soroban_sdk::symbol_short!("bind"),
            address.clone(),
        ),
        (issuer.clone(), binding.nonce),
    );

    Ok(binding)
}

/// Revoke the identity binding for `address`.
///
/// After unbinding the address is treated as `Unverified` and any stored
/// identity data is removed.
///
/// # Security
/// Caller must have already verified admin authorization.
pub fn unbind_identity(env: &Env, address: &Address) -> Result<(), Error> {
    use crate::DataKey;

    let maybe_binding: Option<IdentityBinding> = env
        .storage()
        .persistent()
        .get(&DataKey::IdentityBinding(address.clone()));

    match maybe_binding {
        Some(mut binding) => {
            binding.active = false;
            env.storage()
                .persistent()
                .set(&DataKey::IdentityBinding(address.clone()), &binding);

            // Also clear stored identity so the address reverts to Unverified
            env.storage()
                .persistent()
                .remove(&DataKey::AddressIdentity(address.clone()));

            env.events().publish(
                (
                    soroban_sdk::symbol_short!("unbind"),
                    address.clone(),
                ),
                soroban_sdk::symbol_short!("revoked"),
            );
            Ok(())
        }
        None => {
            // No binding exists — nothing to revoke; treat as success.
            Ok(())
        }
    }
}

/// Validate that `address` has an active binding whose issuer matches the
/// claim issuer.
///
/// Returns `Ok(nonce)` on success so the caller can audit the binding
/// generation, or an appropriate error if:
/// - no binding exists (`InvalidClaimFormat`)
/// - binding is inactive (`Unauthorized`)
/// - claim issuer ≠ bound issuer (`UnauthorizedIssuer`)
pub fn validate_binding(
    env: &Env,
    address: &Address,
    claim_issuer: &Address,
) -> Result<u64, Error> {
    use crate::DataKey;

    let binding: IdentityBinding = env
        .storage()
        .persistent()
        .get(&DataKey::IdentityBinding(address.clone()))
        .ok_or(Error::InvalidClaimFormat)?;

    if !binding.active {
        return Err(Error::Unauthorized);
    }

    if binding.bound_issuer != *claim_issuer {
        return Err(Error::UnauthorizedIssuer);
    }

    Ok(binding.nonce)
}

/// Query the current binding for an address (if any).
pub fn get_binding(env: &Env, address: &Address) -> Option<IdentityBinding> {
    use crate::DataKey;
    env.storage()
        .persistent()
        .get(&DataKey::IdentityBinding(address.clone()))
}

// ============================================================================
// Claim Serialization & Verification
// ============================================================================

/// Serialize an identity claim for signature verification.
///
/// Uses deterministic XDR encoding to ensure consistent signatures.
pub fn serialize_claim(env: &Env, claim: &IdentityClaim) -> Bytes {
    let mut bytes = Bytes::new(env);

    // Serialize each field in order
    bytes.append(&claim.address.clone().to_xdr(env));
    bytes.append(&Bytes::from_array(
        env,
        &[
            (claim.tier.clone() as u32).to_be_bytes()[0],
            (claim.tier.clone() as u32).to_be_bytes()[1],
            (claim.tier.clone() as u32).to_be_bytes()[2],
            (claim.tier.clone() as u32).to_be_bytes()[3],
        ],
    ));
    bytes.append(&Bytes::from_array(env, &claim.risk_score.to_be_bytes()));
    bytes.append(&Bytes::from_array(env, &claim.expiry.to_be_bytes()));
    bytes.append(&claim.issuer.clone().to_xdr(env));

    bytes
}

/// Verify the Ed25519 signature of an identity claim.
///
/// # Errors
/// Panics (via `ed25519_verify`) if the signature is invalid — Soroban
/// treats verification failure as a trap rather than a recoverable error.
pub fn verify_claim_signature(
    env: &Env,
    claim: &IdentityClaim,
    signature: &BytesN<64>,
    issuer_pubkey: &BytesN<32>,
) -> Result<(), Error> {
    let message = serialize_claim(env, claim);
    env.crypto()
        .ed25519_verify(issuer_pubkey, &message, signature);
    Ok(())
}

/// Check if a claim has expired.
pub fn is_claim_expired(env: &Env, expiry: u64) -> bool {
    let now = env.ledger().timestamp();
    now >= expiry
}

/// Validate claim format and fields.
pub fn validate_claim(claim: &IdentityClaim) -> Result<(), Error> {
    if claim.risk_score > 100 {
        return Err(Error::InvalidRiskScore);
    }
    Ok(())
}

/// Calculate effective transaction limit based on tier and risk score.
pub fn calculate_effective_limit(
    _env: &Env,
    identity: &AddressIdentity,
    tier_limits: &TierLimits,
    risk_thresholds: &RiskThresholds,
) -> i128 {
    let tier_limit = match identity.tier {
        IdentityTier::Unverified => tier_limits.unverified_limit,
        IdentityTier::Basic => tier_limits.basic_limit,
        IdentityTier::Verified => tier_limits.verified_limit,
        IdentityTier::Premium => tier_limits.premium_limit,
    };

    if identity.risk_score >= risk_thresholds.high_risk_threshold {
        let multiplier = risk_thresholds.high_risk_multiplier as i128;
        (tier_limit * multiplier) / 100
    } else {
        tier_limit
    }
}
