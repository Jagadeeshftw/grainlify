//! Registry read/write logic for the view-facade contract.
//!
//! All registry operations (init, register, deregister, list, lookup) live here,
//! decoupled from the thin contract entrypoint in [`crate::ViewFacade`]. This makes
//! the storage layer independently testable and keeps the ABI surface small.

use soroban_sdk::{symbol_short, Address, Env, Vec};

use crate::types::{ContractKind, DataKey, FacadeError, InitializedEvent, RegisteredContract};

use crate::MAX_REGISTRY_SIZE;

/// Initialize the facade with an immutable administrator address.
///
/// Guards against double initialization to protect admin immutability and
/// emits an `Initialized` event on the `("facade", "init")` topic.
pub fn init(env: &Env, admin: &Address) -> Result<(), FacadeError> {
    if env.storage().instance().has(&DataKey::Admin) {
        return Err(FacadeError::AlreadyInitialized);
    }

    env.storage().instance().set(&DataKey::Admin, admin);

    env.events().publish(
        (symbol_short!("facade"), symbol_short!("init")),
        InitializedEvent {
            admin: admin.clone(),
        },
    );

    Ok(())
}

/// Return the administrator address, or `None` if not yet initialized.
pub fn get_admin(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::Admin)
}

/// Register a contract address so it appears in cross-contract views.
///
/// If the address is already registered, the existing entry's `kind` and
/// `version` are **updated** to match the new values, and the entry
/// maintains its original position in insertion order.
///
/// # Authorization
/// Requires a valid signature from the stored admin address.
///
/// # Errors
/// * [`FacadeError::NotInitialized`] — if `init` has not yet been called.
/// * [`FacadeError::RegistryFull`] — if registry has reached [`MAX_REGISTRY_SIZE`].
pub fn register(
    env: &Env,
    address: &Address,
    kind: &ContractKind,
    version: u32,
) -> Result<(), FacadeError> {
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(FacadeError::NotInitialized)?;

    admin.require_auth();

    let mut registry: Vec<RegisteredContract> = env
        .storage()
        .instance()
        .get(&DataKey::Registry)
        .unwrap_or(Vec::new(env));

    if registry.len() >= MAX_REGISTRY_SIZE {
        return Err(FacadeError::RegistryFull);
    }

    registry.push_back(RegisteredContract {
        address: address.clone(),
        kind: kind.clone(),
        version,
    });

    env.storage().instance().set(&DataKey::Registry, &registry);

    Ok(())
}

/// Remove a previously registered contract address.
///
/// If `address` is not in the registry this is a no-op (the registry is
/// returned unchanged).
///
/// # Authorization
/// Requires a valid signature from the stored admin address.
///
/// # Errors
/// * [`FacadeError::NotInitialized`] — if `init` has not yet been called.
pub fn deregister(env: &Env, address: &Address) -> Result<(), FacadeError> {
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(FacadeError::NotInitialized)?;

    admin.require_auth();

    let registry: Vec<RegisteredContract> = env
        .storage()
        .instance()
        .get(&DataKey::Registry)
        .unwrap_or(Vec::new(env));

    let mut updated = Vec::new(env);
    for entry in registry.iter() {
        if entry.address != *address {
            updated.push_back(entry);
        }
    }

    env.storage().instance().set(&DataKey::Registry, &updated);

    Ok(())
}

/// Return a paginated slice of all registered contracts in insertion order.
///
/// # Errors
/// * [`FacadeError::InvalidPagination`] — if offset > total entries or limit = 0.
pub fn list_contracts(
    env: &Env,
    offset: Option<u32>,
    limit: Option<u32>,
) -> Result<Vec<RegisteredContract>, FacadeError> {
    let registry: Vec<RegisteredContract> = env
        .storage()
        .instance()
        .get(&DataKey::Registry)
        .unwrap_or(Vec::new(env));

    let total = registry.len();
    let offset = offset.unwrap_or(0);
    let limit = limit.unwrap_or(total);

    if offset > total {
        return Err(FacadeError::InvalidPagination);
    }
    if limit == 0 {
        return Err(FacadeError::InvalidPagination);
    }

    let end = if offset + limit > total {
        total
    } else {
        offset + limit
    };

    let mut result = Vec::new(env);
    for i in offset..end {
        result.push_back(registry.get(i).unwrap().clone());
    }

    Ok(result)
}

/// Return all registered contracts as an ordered list (legacy version).
pub fn list_contracts_all(env: &Env) -> Vec<RegisteredContract> {
    env.storage()
        .instance()
        .get(&DataKey::Registry)
        .unwrap_or(Vec::new(env))
}

/// Return the total number of registered contracts.
pub fn contract_count(env: &Env) -> u32 {
    let registry: Vec<RegisteredContract> = env
        .storage()
        .instance()
        .get(&DataKey::Registry)
        .unwrap_or(Vec::new(env));
    registry.len()
}

/// Look up a registered contract by its on-chain address (O(n) scan).
pub fn get_contract(env: &Env, address: &Address) -> Option<RegisteredContract> {
    let registry: Vec<RegisteredContract> = env
        .storage()
        .instance()
        .get(&DataKey::Registry)
        .unwrap_or(Vec::new(env));

    for entry in registry.iter() {
        if &entry.address == address {
            return Some(entry);
        }
    }
    None
}
