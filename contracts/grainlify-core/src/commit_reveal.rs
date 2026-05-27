use soroban_sdk::{contracttype, Address, Bytes, BytesN, Env};

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
    CommitmentExpired = 100,
    RevealMismatch = 101,
    UnauthorizedReveal = 102,
}

/// Creates a new commitment.
pub fn create_commitment(
    env: &Env,
    creator: Address,
    hash: BytesN<32>,
    expiry: Option<u64>,
) -> Commitment {
    Commitment {
        hash,
        creator,
        timestamp: env.ledger().timestamp(),
        expiry,
    }
}

/// Verifies a reveal against a commitment.
/// Includes a check to ensure only the creator can reveal (Front-running protection).
pub fn verify_reveal(
    env: &Env,
    commitment: &Commitment,
    revealer: Address,
    value: Bytes,
    salt: Bytes,
) -> Result<(), Error> {
    // 1. Authorization: Only the original creator can reveal this commitment
    if revealer != commitment.creator {
        return Err(Error::UnauthorizedReveal);
    }
    
    // Ensure the revealer has authorized this call
    revealer.require_auth();

    // 2. Check expiry
    if let Some(expiry) = commitment.expiry {
        if env.ledger().timestamp() > expiry {
            return Err(Error::CommitmentExpired);
        }
    }

    // 3. Reconstruct hash: sha256(value + salt)
    let mut data = value;
    data.append(&salt);
    
    let reconstructed_hash: BytesN<32> = env.crypto().sha256(&data).into();

    if reconstructed_hash != commitment.hash {
        return Err(Error::RevealMismatch);
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};

    #[test]
    fn test_unauthorized_reveal() {
        let env = Env::default();
        let creator = Address::generate(&env);
        let attacker = Address::generate(&env);

        let value = Bytes::from_array(&env, &[1]);
        let salt = Bytes::from_array(&env, &[2]);
        
        let mut data = value.clone();
        data.append(&salt);
        let hash: BytesN<32> = env.crypto().sha256(&data).into();

        let commitment = create_commitment(&env, creator.clone(), hash, None);

        env.mock_all_auths();
        let result = verify_reveal(&env, &commitment, attacker, value, salt);
        assert_eq!(result, Err(Error::UnauthorizedReveal));
    }
}
