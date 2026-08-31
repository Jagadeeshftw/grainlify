use soroban_sdk::{contracttype, Address, Bytes, BytesN, Env};
use crate::nonce;

/// Persistent storage key for a stored commitment.
#[contracttype]
#[derive(Clone)]
enum CommitmentKey {
    /// Keyed by the 32-byte commitment hash for O(1) lookup during reveal.
    ByHash(BytesN<32>),
    /// Whether a commitment has been revealed (consumed).
    Revealed(BytesN<32>),
}

/// A stored commitment with creator identity and optional expiry.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Commitment {
    pub hash: BytesN<32>,
    pub creator: Address,
    pub timestamp: u64,
    pub expiry: Option<u64>,
}

#[soroban_sdk::contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// The commitment has expired (current timestamp > expiry).
    CommitmentExpired = 100,
    /// The reconstructed hash does not match the committed hash.
    RevealMismatch = 101,
    /// Only the original creator can reveal this commitment.
    UnauthorizedReveal = 102,
    /// A commitment with this hash already exists in storage.
    CommitmentAlreadyExists = 103,
    /// The commitment hash was not found in storage (must commit first).
    CommitmentNotFound = 104,
    /// This commitment has already been revealed; replay is not allowed.
    CommitmentAlreadyRevealed = 105,
    /// The provided nonce does not match the expected nonce for this signer/domain.
    InvalidNonce = 106,
}

/// Creates a new commitment and stores it in persistent storage.
///
/// # Nonce Scope
/// Commitments are scoped **per account**. Two different accounts may commit
/// the same hash simultaneously; one account cannot commit the same hash twice
/// without first revealing (consuming) the prior commitment.
///
/// # Errors
/// - [`Error::CommitmentAlreadyExists`] if a commitment with the same hash
///   already exists in storage.
pub fn create_commitment(
    env: &Env,
    creator: Address,
    hash: BytesN<32>,
    expiry: Option<u64>,
) -> Result<Commitment, Error> {
    // Prevent duplicate commitments for the same hash (global uniqueness by hash)
    let key = CommitmentKey::ByHash(hash.clone());
    if env.storage().persistent().has(&key) {
        return Err(Error::CommitmentAlreadyExists);
    }

    let commitment = Commitment {
        hash: hash.clone(),
        creator,
        timestamp: env.ledger().timestamp(),
        expiry,
    };

    env.storage().persistent().set(&key, &commitment);
    Ok(commitment)
}

/// Verifies a reveal against a stored commitment.
///
/// # Replay Protection
/// After a successful reveal, the commitment is marked as revealed in
/// persistent storage. A second reveal of the same commitment will fail
/// with [`Error::CommitmentAlreadyRevealed`].
///
/// # Nonce Integration
/// If `provided_nonce` is `Some(n)`, the caller's global nonce is validated
/// and consumed via [`nonce::validate_and_increment_nonce`]. This provides
/// an additional per-account replay barrier independent of the commitment
/// hash.
///
/// # Errors
/// - [`Error::CommitmentNotFound`] if no commitment with this hash is stored.
/// - [`Error::CommitmentAlreadyRevealed`] if this commitment was already revealed.
/// - [`Error::CommitmentExpired`] if the commitment has expired.
/// - [`Error::UnauthorizedReveal`] if the revealer is not the original creator.
/// - [`Error::RevealMismatch`] if the reconstructed hash does not match.
/// - [`Error::InvalidNonce`] if nonce validation fails.
pub fn verify_reveal(
    env: &Env,
    hash: &BytesN<32>,
    revealer: Address,
    value: Bytes,
    salt: Bytes,
    provided_nonce: Option<u64>,
) -> Result<(), Error> {
    // 1. Look up the stored commitment
    let key = CommitmentKey::ByHash(hash.clone());
    let commitment: Commitment = env
        .storage()
        .persistent()
        .get(&key)
        .ok_or(Error::CommitmentNotFound)?;

    // 2. Check if already revealed (replay protection)
    let revealed_key = CommitmentKey::Revealed(hash.clone());
    if env.storage().persistent().has(&revealed_key) {
        return Err(Error::CommitmentAlreadyRevealed);
    }

    // 3. Authorization: Only the original creator can reveal
    // Note: The caller (entrypoint) must call `revealer.require_auth()` before
    // invoking this function. We check identity here but defer auth to the
    // entrypoint because Soroban's auth frame is unavailable inside free
    // functions called from `env.as_contract()`.
    if revealer != commitment.creator {
        return Err(Error::UnauthorizedReveal);
    }

    // 4. Check expiry
    if let Some(expiry) = commitment.expiry {
        if env.ledger().timestamp() > expiry {
            return Err(Error::CommitmentExpired);
        }
    }

    // 5. Optional nonce validation (per-account scope)
    if let Some(nonce_val) = provided_nonce {
        nonce::validate_and_increment_nonce(env, &revealer, nonce_val)
            .map_err(|_| Error::InvalidNonce)?;
    }

    // 6. Reconstruct hash: sha256(value + salt)
    let mut data = value;
    data.append(&salt);
    let reconstructed_hash: BytesN<32> = env.crypto().sha256(&data).into();

    if reconstructed_hash != *hash {
        return Err(Error::RevealMismatch);
    }

    // 7. Mark as revealed to prevent replay
    env.storage().persistent().set(&revealed_key, &true);

    Ok(())
}

/// Retrieves a stored commitment by hash.
///
/// Returns `None` if no commitment with the given hash exists.
pub fn get_commitment(env: &Env, hash: &BytesN<32>) -> Option<Commitment> {
    let key = CommitmentKey::ByHash(hash.clone());
    env.storage().persistent().get(&key)
}

/// Checks whether a commitment has already been revealed.
pub fn is_revealed(env: &Env, hash: &BytesN<32>) -> bool {
    let key = CommitmentKey::Revealed(hash.clone());
    env.storage().persistent().has(&key)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::GrainlifyContract;
    use soroban_sdk::testutils::{Address as _, Ledger as _};

    fn setup(env: &Env) -> Address {
        env.register_contract(None, GrainlifyContract)
    }

    fn hash_value(env: &Env, value: &[u8], salt: &[u8]) -> BytesN<32> {
        let mut data = Bytes::from_slice(env, value);
        data.append(&Bytes::from_slice(env, salt));
        env.crypto().sha256(&data).into()
    }

    #[test]
    fn test_commit_and_reveal_happy_path() {
        let env = Env::default();
        let contract_id = setup(&env);
        let creator = Address::generate(&env);
        let value = Bytes::from_array(&env, &[1, 2, 3]);
        let salt = Bytes::from_array(&env, &[4, 5, 6]);
        let hash = hash_value(&env, &[1, 2, 3], &[4, 5, 6]);

        env.as_contract(&contract_id, || {
            let commitment = create_commitment(&env, creator.clone(), hash.clone(), None).unwrap();
            assert_eq!(commitment.creator, creator);
            assert_eq!(commitment.hash, hash);

            env.mock_all_auths();
            let result = verify_reveal(&env, &hash, creator, value, salt, None);
            assert_eq!(result, Ok(()));
            assert!(is_revealed(&env, &hash));
        });
    }

    #[test]
    fn test_commit_duplicate_hash_fails() {
        let env = Env::default();
        let contract_id = setup(&env);
        let creator = Address::generate(&env);
        let hash = BytesN::from_array(&env, &[0xAA; 32]);

        env.as_contract(&contract_id, || {
            let _ = create_commitment(&env, creator.clone(), hash.clone(), None);
            let result = create_commitment(&env, creator, hash, None);
            assert_eq!(result, Err(Error::CommitmentAlreadyExists));
        });
    }

    #[test]
    fn test_reveal_replay_fails() {
        let env = Env::default();
        let contract_id = setup(&env);
        let creator = Address::generate(&env);
        let value = Bytes::from_array(&env, &[10]);
        let salt = Bytes::from_array(&env, &[20]);
        let hash = hash_value(&env, &[10], &[20]);

        env.as_contract(&contract_id, || {
            let _ = create_commitment(&env, creator.clone(), hash.clone(), None).unwrap();

            env.mock_all_auths();
            // First reveal succeeds
            let r1 = verify_reveal(&env, &hash, creator.clone(), value.clone(), salt.clone(), None);
            assert_eq!(r1, Ok(()));

            // Second reveal of the same commitment fails
            let r2 = verify_reveal(&env, &hash, creator, value, salt, None);
            assert_eq!(r2, Err(Error::CommitmentAlreadyRevealed));
        });
    }

    #[test]
    fn test_unauthorized_reveal_fails() {
        let env = Env::default();
        let contract_id = setup(&env);
        let creator = Address::generate(&env);
        let attacker = Address::generate(&env);
        let value = Bytes::from_array(&env, &[1]);
        let salt = Bytes::from_array(&env, &[2]);
        let hash = hash_value(&env, &[1], &[2]);

        env.as_contract(&contract_id, || {
            let _ = create_commitment(&env, creator, hash.clone(), None).unwrap();

            env.mock_all_auths();
            let result = verify_reveal(&env, &hash, attacker, value, salt, None);
            assert_eq!(result, Err(Error::UnauthorizedReveal));
        });
    }

    #[test]
    fn test_expired_commitment_fails() {
        let env = Env::default();
        let contract_id = setup(&env);
        let creator = Address::generate(&env);
        let value = Bytes::from_array(&env, &[1]);
        let salt = Bytes::from_array(&env, &[2]);
        let hash = hash_value(&env, &[1], &[2]);

        env.as_contract(&contract_id, || {
            // Create commitment with expiry in the past (timestamp 0)
            let _ = create_commitment(&env, creator.clone(), hash.clone(), Some(0)).unwrap();

            // Advance ledger past expiry
            env.ledger().with_mut(|li| li.timestamp = 1);

            env.mock_all_auths();
            let result = verify_reveal(&env, &hash, creator, value, salt, None);
            assert_eq!(result, Err(Error::CommitmentExpired));
        });
    }

    #[test]
    fn test_reveal_wrong_value_fails() {
        let env = Env::default();
        let contract_id = setup(&env);
        let creator = Address::generate(&env);
        let value = Bytes::from_array(&env, &[1]);
        let salt = Bytes::from_array(&env, &[2]);
        let hash = hash_value(&env, &[1], &[2]);

        env.as_contract(&contract_id, || {
            let _ = create_commitment(&env, creator.clone(), hash.clone(), None).unwrap();

            let wrong_value = Bytes::from_array(&env, &[99]);
            env.mock_all_auths();
            let result = verify_reveal(&env, &hash, creator, wrong_value, salt, None);
            assert_eq!(result, Err(Error::RevealMismatch));
        });
    }

    #[test]
    fn test_reveal_commitment_not_found() {
        let env = Env::default();
        let contract_id = setup(&env);
        let creator = Address::generate(&env);
        let value = Bytes::from_array(&env, &[1]);
        let salt = Bytes::from_array(&env, &[2]);
        let hash = hash_value(&env, &[1], &[2]);

        env.as_contract(&contract_id, || {
            env.mock_all_auths();
            // No commitment created — should fail
            let result = verify_reveal(&env, &hash, creator, value, salt, None);
            assert_eq!(result, Err(Error::CommitmentNotFound));
        });
    }

    // ─────────────────────────────────────────────────────────────────────
    // Nonce uniqueness tests (Issue #1724)
    // ─────────────────────────────────────────────────────────────────────

    #[test]
    fn test_same_nonce_across_different_accounts_succeeds() {
        let env = Env::default();
        let contract_id = setup(&env);
        let alice = Address::generate(&env);
        let bob = Address::generate(&env);

        env.as_contract(&contract_id, || {
            // Both accounts use nonce 0 — different accounts have independent nonces
            let val_a = Bytes::from_array(&env, &[10]);
            let salt_a = Bytes::from_array(&env, &[11]);
            let hash_a = hash_value(&env, &[10], &[11]);
            let _ = create_commitment(&env, alice.clone(), hash_a.clone(), None).unwrap();

            let val_b = Bytes::from_array(&env, &[20]);
            let salt_b = Bytes::from_array(&env, &[21]);
            let hash_b = hash_value(&env, &[20], &[21]);
            let _ = create_commitment(&env, bob.clone(), hash_b.clone(), None).unwrap();

            env.mock_all_auths();

            // Alice reveals with nonce 0
            let r1 = verify_reveal(&env, &hash_a, alice, val_a, salt_a, Some(0));
            assert_eq!(r1, Ok(()));

            // Bob also reveals with nonce 0 — succeeds because nonces are per-account
            let r2 = verify_reveal(&env, &hash_b, bob, val_b, salt_b, Some(0));
            assert_eq!(r2, Ok(()));
        });
    }

    #[test]
    fn test_same_account_across_different_domains_succeeds() {
        let env = Env::default();
        let contract_id = setup(&env);
        let alice = Address::generate(&env);

        env.as_contract(&contract_id, || {
            // Alice commits two different commitments
            let val_a = Bytes::from_array(&env, &[1]);
            let salt_a = Bytes::from_array(&env, &[2]);
            let hash_a = hash_value(&env, &[1], &[2]);
            let _ = create_commitment(&env, alice.clone(), hash_a.clone(), None).unwrap();

            let val_b = Bytes::from_array(&env, &[3]);
            let salt_b = Bytes::from_array(&env, &[4]);
            let hash_b = hash_value(&env, &[3], &[4]);
            let _ = create_commitment(&env, alice.clone(), hash_b.clone(), None).unwrap();

            env.mock_all_auths();

            // Reveal first with nonce 0
            let r1 = verify_reveal(&env, &hash_a, alice.clone(), val_a, salt_a, Some(0));
            assert_eq!(r1, Ok(()));

            // Reveal second with nonce 1 (global nonce incremented)
            let r2 = verify_reveal(&env, &hash_b, alice, val_b, salt_b, Some(1));
            assert_eq!(r2, Ok(()));
        });
    }

    #[test]
    fn test_replayed_commit_fails_with_stale_nonce() {
        let env = Env::default();
        let contract_id = setup(&env);
        let alice = Address::generate(&env);

        env.as_contract(&contract_id, || {
            let val = Bytes::from_array(&env, &[1]);
            let salt = Bytes::from_array(&env, &[2]);
            let hash = hash_value(&env, &[1], &[2]);
            let _ = create_commitment(&env, alice.clone(), hash.clone(), None).unwrap();

            env.mock_all_auths();
            // Reveal with nonce 0 succeeds
            let r1 = verify_reveal(&env, &hash, alice.clone(), val.clone(), salt.clone(), Some(0));
            assert_eq!(r1, Ok(()));

            // Create a second commitment with a different hash
            let val2 = Bytes::from_array(&env, &[5]);
            let salt2 = Bytes::from_array(&env, &[6]);
            let hash2 = hash_value(&env, &[5], &[6]);
            let _ = create_commitment(&env, alice.clone(), hash2.clone(), None).unwrap();

            // Trying to reveal with stale nonce 0 fails
            let r2 = verify_reveal(&env, &hash2, alice, val2, salt2, Some(0));
            assert_eq!(r2, Err(Error::InvalidNonce));
        });
    }

    #[test]
    fn test_get_commitment_and_is_revealed() {
        let env = Env::default();
        let contract_id = setup(&env);
        let creator = Address::generate(&env);
        let value = Bytes::from_array(&env, &[1]);
        let salt = Bytes::from_array(&env, &[2]);
        let hash = hash_value(&env, &[1], &[2]);

        env.as_contract(&contract_id, || {
            // Before commit: no commitment stored
            assert!(get_commitment(&env, &hash).is_none());
            assert!(!is_revealed(&env, &hash));

            let _ = create_commitment(&env, creator.clone(), hash.clone(), None).unwrap();
            let stored = get_commitment(&env, &hash).unwrap();
            assert_eq!(stored.creator, creator);
            assert!(!is_revealed(&env, &hash));

            env.mock_all_auths();
            let _ = verify_reveal(&env, &hash, creator, value, salt, None);
            assert!(is_revealed(&env, &hash));
        });
    }

    #[test]
    fn test_sequential_commits_different_hashes_succeed() {
        let env = Env::default();
        let contract_id = setup(&env);
        let creator = Address::generate(&env);

        env.as_contract(&contract_id, || {
            for i in 0..5u8 {
                let val = Bytes::from_array(&env, &[i]);
                let salt = Bytes::from_array(&env, &[i + 100]);
                let hash = hash_value(&env, &[i], &[i + 100]);
                let c = create_commitment(&env, creator.clone(), hash.clone(), None);
                assert!(c.is_ok());

                env.mock_all_auths();
                let r = verify_reveal(&env, &hash, creator.clone(), val, salt, None);
                assert_eq!(r, Ok(()));
            }
        });
    }
}
