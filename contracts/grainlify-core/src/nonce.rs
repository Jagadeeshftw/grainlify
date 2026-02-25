use soroban_sdk::{contracterror, contracttype, Address, Env, Symbol};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum NonceError {
    InvalidNonce = 100,
}

#[contracttype]
pub enum NonceKey {
    Signer(Address),
    SignerWithDomain(Address, Symbol),
}

/// Get the current nonce for a signer
pub fn get_nonce(env: &Env, signer: &Address) -> u64 {
    let key = NonceKey::Signer(signer.clone());
    env.storage().persistent().get(&key).unwrap_or(0)
}

/// Get the current nonce for a signer with a specific domain
pub fn get_nonce_with_domain(env: &Env, signer: &Address, domain: Symbol) -> u64 {
    let key = NonceKey::SignerWithDomain(signer.clone(), domain);
    env.storage().persistent().get(&key).unwrap_or(0)
}

/// Validate and increment nonce for a signer.
/// Returns Ok(()) if valid, otherwise Err(NonceError::InvalidNonce).
pub fn validate_and_increment_nonce(
    env: &Env,
    signer: &Address,
    provided_nonce: u64,
) -> Result<(), NonceError> {
    let current_nonce = get_nonce(env, signer);

    if provided_nonce != current_nonce {
        return Err(NonceError::InvalidNonce);
    }

    let key = NonceKey::Signer(signer.clone());
    env.storage().persistent().set(&key, &(current_nonce + 1));
    Ok(())
}

/// Validate and increment nonce for a signer within a specific domain.
pub fn validate_and_increment_nonce_with_domain(
    env: &Env,
    signer: &Address,
    domain: Symbol,
    provided_nonce: u64,
) -> Result<(), NonceError> {
    let current_nonce = get_nonce_with_domain(env, signer, domain.clone());

    if provided_nonce != current_nonce {
        return Err(NonceError::InvalidNonce);
    }

    let key = NonceKey::SignerWithDomain(signer.clone(), domain);
    env.storage().persistent().set(&key, &(current_nonce + 1));
    Ok(())
}
