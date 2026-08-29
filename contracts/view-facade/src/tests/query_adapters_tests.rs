#![cfg(test)]
//! # Query Adapter Tests
//!
//! Regression tests for the split of the view-facade into a thin entrypoint
//! ([`crate::ViewFacade`]) and the query-specific adapters in [`crate::query`].
//!
//! These tests exercise the query adapter functions through the public
//! entrypoint to prove the refactor preserved behavior for:
//!
//! 1. **Missing records** — registry lookups / payouts for addresses that were
//!    never registered or never received a payout.
//! 2. **Pagination** — offset/limit slicing of the registry.
//! 3. **Cross-contract error mapping** — behavior when the target escrow
//!    contract is missing or the underlying query fails.

use crate::{ContractKind, FacadeError, ViewFacade, ViewFacadeClient};
use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    Address, Env, IntoVal, String,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Boot a fresh `ViewFacade` instance, initialized with `admin`.
fn setup() -> (Env, ViewFacadeClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let facade_id = env.register_contract(None, ViewFacade);
    let facade = ViewFacadeClient::new(&env, &facade_id);

    let admin = Address::generate(&env);
    facade.init(&admin);

    (env, facade, admin)
}

/// Register `count` distinct contracts and return their addresses.
fn register_many(env: &Env, facade: &ViewFacadeClient, count: u32) -> Vec<Address> {
    let mut addresses = Vec::new(&env);
    for i in 0..count {
        let addr = Address::generate(env);
        facade.register(&addr, &ContractKind::BountyEscrow, &(i + 1));
        addresses.push_back(addr);
    }
    addresses
}

// ---------------------------------------------------------------------------
// Missing records
// ---------------------------------------------------------------------------

/// `get_contract` on a registry with a never-registered address returns `None`.
#[test]
fn test_missing_record_get_contract_returns_none() {
    let (env, facade, _admin) = setup();
    let unknown = Address::generate(&env);

    assert_eq!(facade.get_contract(&unknown), None);
}

/// `query_recipient_history` against a missing escrow contract propagates the
/// cross-contract failure (the facade does not swallow it for the raw query).
#[test]
#[should_panic]
fn test_missing_escrow_panics_in_recipient_history() {
    let (env, facade, _admin) = setup();
    let ghost_escrow = Address::generate(&env);
    let program_id = String::from_str(&env, "ghost-program");
    let recipient = Address::generate(&env);

    // The address is not a real ProgramEscrow contract — the raw adapter call
    // must panic rather than silently returning data.
    facade.query_recipient_history(&ghost_escrow, &program_id, &recipient);
}

// ---------------------------------------------------------------------------
// Pagination
// ---------------------------------------------------------------------------

/// Pagination across the registry: `offset`/`limit` slicing must be stable.
#[test]
fn test_pagination_slices_registry_stably() {
    let (env, facade, _admin) = setup();
    let addresses = register_many(&env, &facade, 7);

    let page1 = facade.list_contracts(Some(0), Some(3)).unwrap();
    let page2 = facade.list_contracts(Some(3), Some(3)).unwrap();
    let page3 = facade.list_contracts(Some(6), Some(3)).unwrap();

    assert_eq!(page1.len(), 3);
    assert_eq!(page1.get(0).unwrap().address, addresses.get(0).unwrap());
    assert_eq!(page2.len(), 3);
    assert_eq!(page2.get(0).unwrap().address, addresses.get(3).unwrap());
    assert_eq!(page3.len(), 1);
    assert_eq!(page3.get(0).unwrap().address, addresses.get(6).unwrap());
}

/// Invalid pagination (offset beyond total) returns an error, not a panic.
#[test]
fn test_pagination_offset_beyond_total_errors() {
    let (env, facade, _admin) = setup();
    register_many(&env, &facade, 2);

    let result = facade.try_list_contracts(Some(5), Some(1));
    assert_eq!(result, Err(Ok(FacadeError::InvalidPagination)));
}

/// Pagination combined with `contract_count` covers the full registry once.
#[test]
fn test_pagination_covers_full_registry() {
    let (env, facade, _admin) = setup();
    let addresses = register_many(&env, &facade, 10);

    let total = facade.contract_count();
    assert_eq!(total, 10);

    let mut collected = Vec::new(&env);
    let page_size = 4u32;
    let mut offset = 0u32;
    loop {
        let page = facade.list_contracts(Some(offset), Some(page_size)).unwrap();
        collected.extend_from_slice(&page);
        if page.len() < page_size as usize {
            break;
        }
        offset += page_size;
    }

    assert_eq!(collected.len(), 10);
    for (i, entry) in collected.iter().enumerate() {
        assert_eq!(entry.address, addresses.get(i).unwrap());
    }
}

// ---------------------------------------------------------------------------
// Cross-contract error mapping
// ---------------------------------------------------------------------------

/// The cached program-data query must panic (propagate) when the escrow
/// address is not a real ProgramEscrow contract — proving the adapter does
/// not silently mask cross-contract failures.
#[test]
#[should_panic]
fn test_cached_query_panics_on_non_escrow_address() {
    let (env, facade, _admin) = setup();
    let fake_escrow = Address::generate(&env);
    let program_id = String::from_str(&env, "program-x");

    facade.query_program_data_cached(&fake_escrow, &program_id);
}

/// Admin-gated registration still requires the stored admin address after
/// the split (authorization behavior unchanged).
#[test]
fn test_register_still_requires_admin_auth() {
    let env = Env::default();
    let facade_id = env.register_contract(None, ViewFacade);
    let facade = ViewFacadeClient::new(&env, &facade_id);
    let admin = Address::generate(&env);

    env.mock_all_auths();
    facade.init(&admin);

    let outsider = Address::generate(&env);
    let contract = Address::generate(&env);

    env.mock_auths(&[MockAuth {
        address: &outsider,
        invoke: &MockAuthInvoke {
            contract: &facade_id,
            fn_name: "register",
            args: (contract.clone(), ContractKind::BountyEscrow, 1u32).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let result = facade.try_register(&contract, &ContractKind::BountyEscrow, &1u32);
    assert!(result.is_err(), "non-admin must not be able to register");
}
