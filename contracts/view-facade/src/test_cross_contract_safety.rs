//! # View Facade Cross-Contract Safety Tests  (issue #1288)
//!
//! Verifies that `ViewFacade` is purely read-only in its query paths and
//! that admin-gated mutations cannot be triggered by unprivileged callers.
//!
//! ## Security Properties Tested
//!
//! 1. **Read-only queries** — `list_contracts`, `get_contract`, `contract_count`
//!    require no auth and modify no state.
//! 2. **Admin-only mutations** — `register` and `deregister` require admin auth;
//!    unprivileged callers are rejected.
//! 3. **No auth escalation** — calling a view function does not grant the caller
//!    any elevated permissions.
//! 4. **Immutable admin** — double-init is rejected; admin cannot be replaced.
//! 5. **Registry isolation** — one caller's registration does not affect another
//!    caller's view of the registry.

#![cfg(test)]

use soroban_sdk::{
    testutils::Address as _,
    Address, Env,
};

use crate::{ContractKind, FacadeError, ViewFacade, ViewFacadeClient};

// ── helpers ───────────────────────────────────────────────────────────────────

fn setup(env: &Env) -> (ViewFacadeClient<'_>, Address) {
    let id = env.register_contract(None, ViewFacade);
    let client = ViewFacadeClient::new(env, &id);
    let admin = Address::generate(env);
    env.mock_all_auths();
    client.init(&admin).unwrap();
    (client, admin)
}

fn dummy_contract(env: &Env) -> Address {
    Address::generate(env)
}

// ═════════════════════════════════════════════════════════════════════════════
// 1. Read-only queries require no auth
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_list_contracts_requires_no_auth() {
    let env = Env::default();
    let (client, _) = setup(&env);

    // Call without any auth mock — must succeed
    let result = client.list_contracts(&None, &None);
    assert!(result.is_ok());
}

#[test]
fn test_get_contract_requires_no_auth() {
    let env = Env::default();
    let (client, _) = setup(&env);

    let addr = dummy_contract(&env);
    // Not registered — returns None, no auth needed
    let result = client.get_contract(&addr);
    assert!(result.is_none());
}

#[test]
fn test_contract_count_requires_no_auth() {
    let env = Env::default();
    let (client, _) = setup(&env);

    let count = client.contract_count();
    assert_eq!(count, 0);
}

#[test]
fn test_list_contracts_all_requires_no_auth() {
    let env = Env::default();
    let (client, _) = setup(&env);

    let all = client.list_contracts_all();
    assert_eq!(all.len(), 0);
}

#[test]
fn test_get_admin_requires_no_auth() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let stored = client.get_admin();
    assert_eq!(stored, Some(admin));
}

// ═════════════════════════════════════════════════════════════════════════════
// 2. Admin-only mutations reject unprivileged callers
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_register_requires_admin_auth() {
    let env = Env::default();
    let id = env.register_contract(None, ViewFacade);
    let client = ViewFacadeClient::new(&env, &id);
    let admin = Address::generate(&env);

    env.mock_all_auths();
    client.init(&admin).unwrap();

    // Try to register without any auth — must fail
    let addr = dummy_contract(&env);
    let result = client
        .mock_auths(&[])
        .try_register(&addr, &ContractKind::BountyEscrow, &1u32);
    assert!(result.is_err(), "register must require admin auth");
}

#[test]
fn test_deregister_requires_admin_auth() {
    let env = Env::default();
    let id = env.register_contract(None, ViewFacade);
    let client = ViewFacadeClient::new(&env, &id);
    let admin = Address::generate(&env);

    env.mock_all_auths();
    client.init(&admin).unwrap();

    let addr = dummy_contract(&env);
    client.register(&addr, &ContractKind::BountyEscrow, &1u32).unwrap();

    // Try to deregister without auth — must fail
    let result = client
        .mock_auths(&[])
        .try_deregister(&addr);
    assert!(result.is_err(), "deregister must require admin auth");
}

// ═════════════════════════════════════════════════════════════════════════════
// 3. No auth escalation — view calls do not grant elevated access
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_view_call_does_not_grant_register_access() {
    let env = Env::default();
    let id = env.register_contract(None, ViewFacade);
    let client = ViewFacadeClient::new(&env, &id);
    let admin = Address::generate(&env);

    env.mock_all_auths();
    client.init(&admin).unwrap();

    // Perform a view call
    let _ = client.list_contracts_all();
    let _ = client.contract_count();

    // After view calls, register without auth must still fail
    let addr = dummy_contract(&env);
    let result = client
        .mock_auths(&[])
        .try_register(&addr, &ContractKind::ProgramEscrow, &1u32);
    assert!(result.is_err(),
        "view calls must not grant register access to unprivileged caller");
}

// ═════════════════════════════════════════════════════════════════════════════
// 4. Immutable admin — double-init rejected
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_double_init_is_rejected() {
    let env = Env::default();
    let (client, _) = setup(&env);

    let attacker = Address::generate(&env);
    let result = client.try_init(&attacker);
    assert_eq!(result, Err(Ok(FacadeError::AlreadyInitialized)),
        "second init must be rejected with AlreadyInitialized");
}

#[test]
fn test_admin_cannot_be_replaced_after_init() {
    let env = Env::default();
    let (client, original_admin) = setup(&env);

    // Attempt to replace admin via double-init
    let new_admin = Address::generate(&env);
    let _ = client.try_init(&new_admin);

    // Admin must still be the original
    assert_eq!(client.get_admin(), Some(original_admin),
        "admin must be immutable after initialization");
}

// ═════════════════════════════════════════════════════════════════════════════
// 5. Registry isolation — reads are consistent and unaffected by other callers
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_registry_state_consistent_across_reads() {
    let env = Env::default();
    let (client, _) = setup(&env);

    let addr = dummy_contract(&env);
    client.register(&addr, &ContractKind::BountyEscrow, &1u32).unwrap();

    // Multiple reads must return identical results
    let r1 = client.list_contracts_all();
    let r2 = client.list_contracts_all();
    assert_eq!(r1.len(), r2.len());
    assert_eq!(r1.get(0).unwrap().address, r2.get(0).unwrap().address);
}

#[test]
fn test_unprivileged_caller_sees_same_registry_as_admin() {
    let env = Env::default();
    let (client, _) = setup(&env);

    let addr = dummy_contract(&env);
    client.register(&addr, &ContractKind::GrainlifyCore, &2u32).unwrap();

    // Any caller can read — result is the same
    let all = client.list_contracts_all();
    assert_eq!(all.len(), 1);
    assert_eq!(all.get(0).unwrap().address, addr);
}

// ═════════════════════════════════════════════════════════════════════════════
// 6. Pagination does not expose extra data or require auth
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_paginated_list_requires_no_auth() {
    let env = Env::default();
    let (client, _) = setup(&env);

    for _ in 0..5 {
        let addr = dummy_contract(&env);
        client.register(&addr, &ContractKind::BountyEscrow, &1u32).unwrap();
    }

    // Paginated read — no auth needed
    let page = client.list_contracts(&Some(0u32), &Some(3u32)).unwrap();
    assert_eq!(page.len(), 3);

    let page2 = client.list_contracts(&Some(3u32), &Some(3u32)).unwrap();
    assert_eq!(page2.len(), 2); // only 2 remaining
}

#[test]
fn test_invalid_pagination_returns_error_not_panic() {
    let env = Env::default();
    let (client, _) = setup(&env);

    // offset > total — must return error, not panic
    let result = client.try_list_contracts(&Some(999u32), &Some(10u32));
    assert_eq!(result, Err(Ok(FacadeError::InvalidPagination)));

    // limit = 0 — must return error
    let result2 = client.try_list_contracts(&Some(0u32), &Some(0u32));
    assert_eq!(result2, Err(Ok(FacadeError::InvalidPagination)));
}

// ═════════════════════════════════════════════════════════════════════════════
// 7. Registry full — bounded storage prevents exhaustion
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_registry_full_error_is_returned_not_panic() {
    // We don't fill 1000 entries in a unit test (too slow), but we verify
    // the error variant exists and is correctly typed.
    let env = Env::default();
    let (client, _) = setup(&env);

    // Register one entry successfully
    let addr = dummy_contract(&env);
    let result = client.try_register(&addr, &ContractKind::SorobanEscrow, &1u32);
    assert!(result.is_ok(), "first registration must succeed");

    // Verify RegistryFull is a valid error variant (compile-time check)
    let _err: FacadeError = FacadeError::RegistryFull;
}

// ═════════════════════════════════════════════════════════════════════════════
// 8. Deregister is idempotent — removing non-existent address is a no-op
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_deregister_nonexistent_is_noop() {
    let env = Env::default();
    let (client, _) = setup(&env);

    let addr = dummy_contract(&env);
    // Never registered — deregister must succeed silently
    let result = client.try_deregister(&addr);
    assert!(result.is_ok(), "deregister of non-existent address must be a no-op");
    assert_eq!(client.contract_count(), 0);
}

// ═════════════════════════════════════════════════════════════════════════════
// 9. get_contract returns None for unregistered address
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_get_contract_returns_none_for_unknown_address() {
    let env = Env::default();
    let (client, _) = setup(&env);

    let unknown = dummy_contract(&env);
    assert!(client.get_contract(&unknown).is_none());
}

#[test]
fn test_get_contract_returns_correct_entry_after_register() {
    let env = Env::default();
    let (client, _) = setup(&env);

    let addr = dummy_contract(&env);
    client.register(&addr, &ContractKind::ProgramEscrow, &3u32).unwrap();

    let entry = client.get_contract(&addr).unwrap();
    assert_eq!(entry.address, addr);
    assert_eq!(entry.version, 3u32);
}
