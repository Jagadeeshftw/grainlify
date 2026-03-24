//! Nonce-based replay protection for Grainlify contracts.
//!
//! Each signer maintains a monotonically increasing counter stored in persistent
//! contract storage. Callers must supply the current nonce value; the module
//! validates it and atomically increments it so the same value can never be
//! accepted again.
//!
//! # Lifecycle
//!
//! 1. A signer's nonce starts at `0` (implicit – no storage entry required).
//! 2. Before executing a protected operation the caller queries [`get_nonce`].
//! 3. The caller passes that value to the contract entrypoint.
//! 4. The entrypoint calls [`validate_and_increment_nonce`], which:
//!    - Rejects the call if `provided_nonce != current_nonce`.
//!    - Writes `current_nonce + 1` atomically on success.
//! 5. Any replay of the same nonce is rejected with [`NonceError::InvalidNonce`].
//!
//! # Security assumptions
//!
//! - Soroban's persistent storage is authoritative; there is no off-chain state.
//! - Nonce validation must happen *before* any state-mutating logic in the
//!   entrypoint so a rejected call leaves the contract unchanged.
//! - `u64` gives 1.8 × 10¹⁹ operations per signer before overflow; the
//!   increment uses [`u64::checked_add`] and panics on overflow rather than
//!   wrapping (which would reset replay protection).

use soroban_sdk::{contracterror, contracttype, Address, Env, Symbol};

/// Errors returned by nonce validation.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum NonceError {
    /// The provided nonce does not match the expected value.
    /// Either a replay attempt or the caller used a stale nonce.
    InvalidNonce = 100,
}

/// Storage keys for nonce entries.
#[contracttype]
pub enum NonceKey {
    /// Per-signer nonce (shared across all entrypoints for that signer).
    Signer(Address),
    /// Per-signer, per-domain nonce for contracts that need isolated counters.
    SignerWithDomain(Address, Symbol),
}

/// Returns the current nonce for `signer`.
///
/// Returns `0` when no nonce has been recorded yet (i.e. the signer has never
/// executed a protected operation on this contract).
pub fn get_nonce(env: &Env, signer: &Address) -> u64 {
    env.storage()
        .persistent()
        .get(&NonceKey::Signer(signer.clone()))
        .unwrap_or(0)
}

/// Returns the current nonce for `signer` scoped to `domain`.
///
/// Useful when a contract needs independent nonce sequences per operation
/// category while still sharing a single signer key.
pub fn get_nonce_with_domain(env: &Env, signer: &Address, domain: Symbol) -> u64 {
    env.storage()
        .persistent()
        .get(&NonceKey::SignerWithDomain(signer.clone(), domain))
        .unwrap_or(0)
}

/// Validates `provided_nonce` against the stored nonce for `signer` and
/// increments it atomically on success.
///
/// # Errors
///
/// Returns [`NonceError::InvalidNonce`] if `provided_nonce` does not equal the
/// current stored nonce (replay, out-of-order, or skipped nonce).
///
/// # Panics
///
/// Panics if the nonce counter overflows `u64::MAX`. This is intentional:
/// wrapping would silently reset replay protection.
pub fn validate_and_increment_nonce(
    env: &Env,
    signer: &Address,
    provided_nonce: u64,
) -> Result<(), NonceError> {
    let current = get_nonce(env, signer);
    if provided_nonce != current {
        return Err(NonceError::InvalidNonce);
    }
    let next = current.checked_add(1).expect("nonce overflow");
    env.storage()
        .persistent()
        .set(&NonceKey::Signer(signer.clone()), &next);
    Ok(())
}

/// Validates `provided_nonce` against the stored nonce for `signer` within
/// `domain` and increments it atomically on success.
///
/// # Errors
///
/// Returns [`NonceError::InvalidNonce`] if `provided_nonce` does not equal the
/// current stored nonce for this signer/domain pair.
///
/// # Panics
///
/// Panics on `u64` overflow (same rationale as [`validate_and_increment_nonce`]).
pub fn validate_and_increment_nonce_with_domain(
    env: &Env,
    signer: &Address,
    domain: Symbol,
    provided_nonce: u64,
) -> Result<(), NonceError> {
    let current = get_nonce_with_domain(env, signer, domain.clone());
    if provided_nonce != current {
        return Err(NonceError::InvalidNonce);
    }
    let next = current.checked_add(1).expect("nonce overflow");
    env.storage()
        .persistent()
        .set(&NonceKey::SignerWithDomain(signer.clone(), domain), &next);
    Ok(())
}
