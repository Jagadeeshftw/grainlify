#![no_std]
//! View Facade — read-only aggregation layer for cross-contract queries.
//!
//! Registers known escrow and core contract addresses so dashboards,
//! indexers, and wallets can discover and interrogate them through a
//! single endpoint without coupling to a specific contract type.
//!
//! This contract holds NO funds and writes NO state to other contracts.
//!
//! Spec alignment: Grainlify View Interface v1 (Issue #574)

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContractKind {
    BountyEscrow,
    ProgramEscrow,
    SorobanEscrow,
    GrainlifyCore,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegisteredContract {
    pub address: Address,
    pub kind: ContractKind,
    /// Numeric version reported by the contract at registration time.
    pub version: u32,
}

#[contracttype]
pub enum DataKey {
    Registry,
    Admin,
}

#[contract]
pub struct ViewFacade;

#[contractimpl]
impl ViewFacade {
    /// Initialize the facade with an admin who may register contracts.
    pub fn init(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    /// Register a contract address so it appears in cross-contract views.
    /// Admin-only.
    pub fn register(env: Env, address: Address, kind: ContractKind, version: u32) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Not initialized");
        admin.require_auth();

        let mut registry: Vec<RegisteredContract> = env
            .storage()
            .instance()
            .get(&DataKey::Registry)
            .unwrap_or(Vec::new(&env));

        registry.push_back(RegisteredContract {
            address,
            kind,
            version,
        });
        env.storage().instance().set(&DataKey::Registry, &registry);
    }

    /// Remove a previously registered contract address. Admin-only.
    pub fn deregister(env: Env, address: Address) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Not initialized");
        admin.require_auth();

        let registry: Vec<RegisteredContract> = env
            .storage()
            .instance()
            .get(&DataKey::Registry)
            .unwrap_or(Vec::new(&env));

        let mut updated = Vec::new(&env);
        for entry in registry.iter() {
            if entry.address != address {
                updated.push_back(entry);
            }
        }
        env.storage().instance().set(&DataKey::Registry, &updated);
    }

    /// List all registered contracts.
    pub fn list_contracts(env: Env) -> Vec<RegisteredContract> {
        env.storage()
            .instance()
            .get(&DataKey::Registry)
            .unwrap_or(Vec::new(&env))
    }

    /// Return the count of registered contracts.
    pub fn contract_count(env: Env) -> u32 {
        let registry: Vec<RegisteredContract> = env
            .storage()
            .instance()
            .get(&DataKey::Registry)
            .unwrap_or(Vec::new(&env));
        registry.len()
    }

    /// Look up a registered contract by address.
    pub fn get_contract(env: Env, address: Address) -> Option<RegisteredContract> {
        let registry: Vec<RegisteredContract> = env
            .storage()
            .instance()
            .get(&DataKey::Registry)
            .unwrap_or(Vec::new(&env));

        for entry in registry.iter() {
            if entry.address == address {
                return Some(entry);
            }
        }
        None
    }
}

#[cfg(test)]
mod test;
