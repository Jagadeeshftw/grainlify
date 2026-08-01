//! # Cross-Contract Event Correlation ID Module
//!
//! Provides a standardized `CorrelationId` convention and deterministic helper functions
//! to correlate events emitted across multiple smart contracts (`grainlify-core`, `program-escrow`, `bounty_escrow`).
//!
//! ## Overview
//! In multi-contract flows (e.g. governance upgrade triggering escrow behavior or facade-driven multi-escrow actions),
//! off-chain indexers require a unique identifier to correlate event streams without relying on timestamps.
//!
//! ## Deterministic Helper
//! `generate_correlation_id` derives a unique 32-byte hash from:
//! - Initiator (`Address`)
//! - Nonce (`u64`)
//! - Optional domain tag (`Symbol`)
//! - Target/Contract context (`env.current_contract_address()`)

use soroban_sdk::xdr::ToXdr;
use soroban_sdk::{Address, Bytes, BytesN, Env, Symbol};

/// Shared correlation identifier type convention across all Grainlify contracts.
/// Encapsulates a 32-byte SHA-256 hash identifying a single logical multi-contract action.
pub type CorrelationId = Bytes;

/// Generates a correlation ID deterministically from a caller-supplied nonce,
/// initiating user address, optional domain symbol, and transaction/contract context.
///
/// # Arguments
/// * `env` - Soroban environment for cryptographic operations
/// * `initiator` - The address initiating the cross-contract action
/// * `nonce` - Caller-supplied or account sequence/nonce for replay protection and uniqueness
/// * `domain` - Optional domain tag to prevent cross-domain collisions (e.g., `symbol_short!("payout")`)
///
/// # Returns
/// * `CorrelationId` - Deterministic correlation identifier containing 32-byte SHA256 digest
pub fn generate_correlation_id(
    env: &Env,
    initiator: &Address,
    nonce: u64,
    domain: Option<&Symbol>,
) -> CorrelationId {
    let mut data = Bytes::new(env);
    data.append(&initiator.to_xdr(env));
    data.append(&nonce.to_xdr(env));
    if let Some(dom) = domain {
        data.append(&dom.to_xdr(env));
    }
    // Include current contract address context if running within a contract invocation
    let contract_addr = env.current_contract_address();
    data.append(&contract_addr.to_xdr(env));

    let hash: BytesN<32> = env.crypto().sha256(&data).into();
    let mut corr_id = Bytes::new(env);
    corr_id.append(&hash.to_xdr(env));
    corr_id
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::contract;
    use soroban_sdk::symbol_short;
    use soroban_sdk::testutils::Address as _;

    #[contract]
    pub struct TestContract;

    fn setup_test_env() -> (Env, Address) {
        let env = Env::default();
        let contract_id = env.register_contract(None, TestContract);
        (env, contract_id)
    }

    #[test]
    fn test_deterministic_generation() {
        let (env, contract_id) = setup_test_env();
        let initiator = Address::generate(&env);
        let nonce = 42u64;
        let domain = symbol_short!("test");

        env.as_contract(&contract_id, || {
            let id1 = generate_correlation_id(&env, &initiator, nonce, Some(&domain));
            let id2 = generate_correlation_id(&env, &initiator, nonce, Some(&domain));

            assert_eq!(id1, id2);
        });
    }

    #[test]
    fn test_collision_resistance_on_nonce_change() {
        let (env, contract_id) = setup_test_env();
        let initiator = Address::generate(&env);
        let domain = symbol_short!("test");

        env.as_contract(&contract_id, || {
            let id1 = generate_correlation_id(&env, &initiator, 1, Some(&domain));
            let id2 = generate_correlation_id(&env, &initiator, 2, Some(&domain));

            assert_ne!(id1, id2);
        });
    }

    #[test]
    fn test_collision_resistance_on_initiator_change() {
        let (env, contract_id) = setup_test_env();
        let initiator1 = Address::generate(&env);
        let initiator2 = Address::generate(&env);
        let nonce = 100u64;

        env.as_contract(&contract_id, || {
            let id1 = generate_correlation_id(&env, &initiator1, nonce, None);
            let id2 = generate_correlation_id(&env, &initiator2, nonce, None);

            assert_ne!(id1, id2);
        });
    }

    #[test]
    fn test_domain_separation() {
        let (env, contract_id) = setup_test_env();
        let initiator = Address::generate(&env);
        let nonce = 55u64;
        let domain1 = symbol_short!("payout");
        let domain2 = symbol_short!("refund");

        env.as_contract(&contract_id, || {
            let id1 = generate_correlation_id(&env, &initiator, nonce, Some(&domain1));
            let id2 = generate_correlation_id(&env, &initiator, nonce, Some(&domain2));
            let id3 = generate_correlation_id(&env, &initiator, nonce, None);

            assert_ne!(id1, id2);
            assert_ne!(id1, id3);
            assert_ne!(id2, id3);
        });
    }
}
