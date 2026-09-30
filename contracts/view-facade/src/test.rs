#![cfg(test)]

use crate::{ContractKind, FacadeError, ViewFacade, ViewFacadeClient};
use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    Address, Env, IntoVal,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Boot a fresh ViewFacade instance and return `(env, client, admin)`.
///
/// `env.mock_all_auths()` is called so that `admin.require_auth()` inside the
/// contract succeeds without needing real transaction signing in tests.
fn setup() -> (Env, ViewFacadeClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let facade_id = env.register_contract(None, ViewFacade);
    let facade = ViewFacadeClient::new(&env, &facade_id);

    let admin = Address::generate(&env);
    (env, facade, admin)
}

/// Boot a fresh ViewFacade instance without global auth mocks.
///
/// Use this helper when a test needs to prove that `register` / `deregister`
/// require the stored admin address specifically, rather than passing under
/// `mock_all_auths()`.
fn setup_without_auth_mocks() -> (Env, ViewFacadeClient<'static>, Address) {
    let env = Env::default();

    let facade_id = env.register_contract(None, ViewFacade);
    let facade = ViewFacadeClient::new(&env, &facade_id);

    let admin = Address::generate(&env);
    (env, facade, admin)
}

// ---------------------------------------------------------------------------
// Initialization
// ---------------------------------------------------------------------------

/// `init` stores the admin address durably; `get_admin` reflects it.
#[test]
fn test_init_stores_admin() {
    let (_, facade, admin) = setup();

    facade.init(&admin); // panics on Err — which is what we want for happy path

    assert_eq!(facade.get_admin(), Some(admin));
}

/// Before `init` is called, `get_admin` returns `None`.
#[test]
fn test_get_admin_before_init_returns_none() {
    let (_, facade, _) = setup();

    assert_eq!(facade.get_admin(), None);
}

/// A second call to `init` must return `AlreadyInitialized` and leave the
/// original admin untouched.
#[test]
fn test_double_init_rejected() {
    let (env, facade, admin) = setup();

    facade.init(&admin);

    let second_admin = Address::generate(&env);
    // try_init returns Result<Result<(), FacadeError>, InvokeError>
    let result = facade.try_init(&second_admin);

    assert_eq!(
        result,
        Err(Ok(FacadeError::AlreadyInitialized)),
        "second init must return AlreadyInitialized"
    );

    // Original admin must be unchanged.
    assert_eq!(facade.get_admin(), Some(admin));
}

/// `init` emits an `Initialized` event on the `("facade", "init")` topic
/// with the correct admin address in the payload.
#[test]
fn test_init_emits_initialized_event() {
    use crate::InitializedEvent;
    use soroban_sdk::{symbol_short, testutils::Events as _, vec, IntoVal};

    let (env, facade, admin) = setup();
    facade.init(&admin);

    let facade_id = facade.address.clone();

    let events = env.events().all();
    let found = events.iter().any(|(contract, topics, data)| {
        if contract != facade_id {
            return false;
        }
        let expected_topics = vec![
            &env,
            symbol_short!("facade").into_val(&env),
            symbol_short!("init").into_val(&env),
        ];
        if topics != expected_topics {
            return false;
        }
        let payload: InitializedEvent = data.into_val(&env);
        payload.admin == admin
    });

    assert!(
        found,
        "Initialized event must be emitted with correct admin"
    );
}

// ---------------------------------------------------------------------------
// Registry — register / lookup
// ---------------------------------------------------------------------------

/// Registering a contract and looking it up by address returns the correct entry.
#[test]
fn test_register_and_lookup_contract() {
    let (env, facade, admin) = setup();
    let bounty_contract = Address::generate(&env);

    facade.init(&admin);
    facade.register(&bounty_contract, &ContractKind::BountyEscrow, &1u32);

    let entry = facade.get_contract(&bounty_contract).unwrap();
    assert_eq!(entry.address, bounty_contract);
    assert_eq!(entry.kind, ContractKind::BountyEscrow);
    assert_eq!(entry.version, 1);
}

/// `get_contract` returns `None` for an address that was never registered.
#[test]
fn test_get_contract_not_found() {
    let (env, facade, admin) = setup();
    let unknown = Address::generate(&env);

    facade.init(&admin);

    assert_eq!(facade.get_contract(&unknown), None);
}

/// All four `ContractKind` variants can be registered and their kinds are
/// preserved accurately in the registry.
#[test]
fn test_register_all_contract_kinds() {
    let (env, facade, admin) = setup();

    facade.init(&admin);

    let bounty = Address::generate(&env);
    let program = Address::generate(&env);
    let soroban = Address::generate(&env);
    let core = Address::generate(&env);

    facade.register(&bounty, &ContractKind::BountyEscrow, &1);
    facade.register(&program, &ContractKind::ProgramEscrow, &2);
    facade.register(&soroban, &ContractKind::SorobanEscrow, &3);
    facade.register(&core, &ContractKind::GrainlifyCore, &4);

    assert_eq!(
        facade.get_contract(&bounty).unwrap().kind,
        ContractKind::BountyEscrow
    );
    assert_eq!(
        facade.get_contract(&program).unwrap().kind,
        ContractKind::ProgramEscrow
    );
    assert_eq!(
        facade.get_contract(&soroban).unwrap().kind,
        ContractKind::SorobanEscrow
    );
    assert_eq!(
        facade.get_contract(&core).unwrap().kind,
        ContractKind::GrainlifyCore
    );
}

/// `register` on an uninitialized contract returns `NotInitialized`.
#[test]
fn test_register_before_init_rejected() {
    let (env, facade, _) = setup();
    let addr = Address::generate(&env);

    let result = facade.try_register(&addr, &ContractKind::BountyEscrow, &1);
    assert_eq!(result, Err(Ok(FacadeError::NotInitialized)));
}

/// The stored admin can authorize `register` with an explicit mocked auth entry.
#[test]
fn test_admin_can_register_with_explicit_auth() {
    let (env, facade, admin) = setup_without_auth_mocks();
    let contract = Address::generate(&env);
    let facade_id = facade.address.clone();

    facade.init(&admin);

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &facade_id,
            fn_name: "register",
            args: (contract.clone(), ContractKind::BountyEscrow, 1u32).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    facade.register(&contract, &ContractKind::BountyEscrow, &1);

    assert_eq!(
        facade.get_contract(&contract).unwrap().kind,
        ContractKind::BountyEscrow
    );
}

/// A non-admin auth entry must not satisfy the admin gate on `register`.
#[test]
#[should_panic]
fn test_non_admin_cannot_register() {
    let (env, facade, admin) = setup_without_auth_mocks();
    let outsider = Address::generate(&env);
    let contract = Address::generate(&env);
    let facade_id = facade.address.clone();

    facade.init(&admin);

    env.mock_auths(&[MockAuth {
        address: &outsider,
        invoke: &MockAuthInvoke {
            contract: &facade_id,
            fn_name: "register",
            args: (contract.clone(), ContractKind::BountyEscrow, 1u32).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    facade.register(&contract, &ContractKind::BountyEscrow, &1);
}

// ---------------------------------------------------------------------------
// Registry — list / count
// ---------------------------------------------------------------------------

/// After `init`, before any registration, `contract_count` is zero.
#[test]
fn test_contract_count_initially_zero() {
    let (_, facade, admin) = setup();

    facade.init(&admin);

    assert_eq!(facade.contract_count(), 0);
}

/// `list_contracts` and `contract_count` are consistent after multiple registrations.
#[test]
fn test_list_and_count_contracts() {
    let (env, facade, admin) = setup();

    facade.init(&admin);

    let c1 = Address::generate(&env);
    let c2 = Address::generate(&env);

    facade.register(&c1, &ContractKind::BountyEscrow, &1);
    facade.register(&c2, &ContractKind::ProgramEscrow, &2);

    assert_eq!(facade.contract_count(), 2);

    let all = facade.list_contracts_all();
    assert_eq!(all.len(), 2);
    assert_eq!(all.get(0).unwrap().address, c1);
    assert_eq!(all.get(0).unwrap().kind, ContractKind::BountyEscrow);
    assert_eq!(all.get(1).unwrap().address, c2);
    assert_eq!(all.get(1).unwrap().kind, ContractKind::ProgramEscrow);
}

/// If duplicate addresses are registered, the existing entry is updated in-place,
/// not duplicated. The list length remains 1 and reflects the updated metadata.
#[test]
fn test_duplicate_register_updates_existing_entry() {
    let (env, facade, admin) = setup();
    let duplicate = Address::generate(&env);

    facade.init(&admin);
    facade.register(&duplicate, &ContractKind::BountyEscrow, &1);
    assert_eq!(facade.contract_count(), 1);

    // Re-register the same address with different metadata.
    facade.register(&duplicate, &ContractKind::ProgramEscrow, &2);

    let all = facade.list_contracts_all();
    assert_eq!(all.len(), 2);

    // The entry must be updated with new metadata.
    let entry = facade.get_contract(&duplicate).unwrap();
    assert_eq!(entry.kind, ContractKind::ProgramEscrow);
    assert_eq!(entry.version, 2);
}

/// Re-registering with a different kind updates the kind while preserving the address.
#[test]
fn test_duplicate_register_updates_kind() {
    let (env, facade, admin) = setup();
    let addr = Address::generate(&env);

    facade.init(&admin);
    facade.register(&addr, &ContractKind::BountyEscrow, &1);

    let old_entry = facade.get_contract(&addr).unwrap();
    assert_eq!(old_entry.kind, ContractKind::BountyEscrow);

    // Re-register with a different kind.
    facade.register(&addr, &ContractKind::GrainlifyCore, &1);

    let new_entry = facade.get_contract(&addr).unwrap();
    assert_eq!(new_entry.kind, ContractKind::GrainlifyCore);
    assert_eq!(new_entry.version, 1); // version unchanged
}

/// Re-registering with a higher version updates the version while preserving the kind.
#[test]
fn test_duplicate_register_updates_version() {
    let (env, facade, admin) = setup();
    let addr = Address::generate(&env);

    facade.init(&admin);
    facade.register(&addr, &ContractKind::SorobanEscrow, &1);

    let old_entry = facade.get_contract(&addr).unwrap();
    assert_eq!(old_entry.version, 1);

    // Re-register with a higher version.
    facade.register(&addr, &ContractKind::SorobanEscrow, &5);

    let new_entry = facade.get_contract(&addr).unwrap();
    assert_eq!(new_entry.kind, ContractKind::SorobanEscrow); // kind unchanged
    assert_eq!(new_entry.version, 5);
}

/// When an address is re-registered, it maintains its original position in
/// insertion order, not moved to the end.
#[test]
fn test_duplicate_register_maintains_insertion_order() {
    let (env, facade, admin) = setup();

    facade.init(&admin);

    let c1 = Address::generate(&env);
    let c2 = Address::generate(&env);
    let c3 = Address::generate(&env);

    // Register in order: c1, c2, c3.
    facade.register(&c1, &ContractKind::BountyEscrow, &1);
    facade.register(&c2, &ContractKind::ProgramEscrow, &1);
    facade.register(&c3, &ContractKind::SorobanEscrow, &1);

    // Re-register c2 with new metadata.
    facade.register(&c2, &ContractKind::GrainlifyCore, &2);

    // List should still be c1, c2 (updated), c3 in the same order.
    let all = facade.list_contracts();
    assert_eq!(all.len(), 3);
    assert_eq!(all.get(0).unwrap().address, c1);
    assert_eq!(all.get(1).unwrap().address, c2);
    assert_eq!(all.get(1).unwrap().kind, ContractKind::GrainlifyCore);
    assert_eq!(all.get(1).unwrap().version, 2);
    assert_eq!(all.get(2).unwrap().address, c3);
}

/// Edge case: deregister then re-register should create a new entry at the end.
#[test]
fn test_deregister_then_register_appends_to_end() {
    let (env, facade, admin) = setup();

    facade.init(&admin);

    let c1 = Address::generate(&env);
    let c2 = Address::generate(&env);

    facade.register(&c1, &ContractKind::BountyEscrow, &1);
    facade.register(&c2, &ContractKind::ProgramEscrow, &1);

    // Deregister c1.
    facade.deregister(&c1);
    assert_eq!(facade.contract_count(), 1);

    // Re-register c1 with new metadata.
    facade.register(&c1, &ContractKind::GrainlifyCore, &3);

    // Now we should have c2, then c1 (reappended).
    let all = facade.list_contracts();
    assert_eq!(all.len(), 2);
    assert_eq!(all.get(0).unwrap().address, c2);
    assert_eq!(all.get(1).unwrap().address, c1);
    assert_eq!(all.get(1).unwrap().kind, ContractKind::GrainlifyCore);
    assert_eq!(all.get(1).unwrap().version, 3);
}

// ---------------------------------------------------------------------------
// Registry — deregister
// ---------------------------------------------------------------------------

/// Deregistering a known contract removes it from the registry.
#[test]
fn test_deregister_contract() {
    let (env, facade, admin) = setup();
    let contract = Address::generate(&env);

    facade.init(&admin);
    facade.register(&contract, &ContractKind::GrainlifyCore, &3);
    assert_eq!(facade.contract_count(), 1);

    facade.deregister(&contract);

    assert_eq!(facade.contract_count(), 0);
    assert_eq!(facade.get_contract(&contract), None);
}

/// Deregistering an address that was never registered is a no-op;
/// existing entries remain intact and the call does not panic.
#[test]
fn test_deregister_nonexistent_is_noop() {
    let (env, facade, admin) = setup();
    let registered = Address::generate(&env);
    let ghost = Address::generate(&env);

    facade.init(&admin);
    facade.register(&registered, &ContractKind::SorobanEscrow, &1);

    facade.deregister(&ghost); // must not panic

    assert_eq!(facade.contract_count(), 1);
    assert!(facade.get_contract(&registered).is_some());
}

/// `deregister` on an uninitialized contract returns `NotInitialized`.
#[test]
fn test_deregister_before_init_rejected() {
    let (env, facade, _) = setup();
    let addr = Address::generate(&env);

    let result = facade.try_deregister(&addr);
    assert_eq!(result, Err(Ok(FacadeError::NotInitialized)));
}

/// The stored admin can authorize `deregister` with an explicit mocked auth entry.
#[test]
fn test_admin_can_deregister_with_explicit_auth() {
    let (env, facade, admin) = setup_without_auth_mocks();
    let contract = Address::generate(&env);
    let facade_id = facade.address.clone();

    facade.init(&admin);

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &facade_id,
            fn_name: "register",
            args: (contract.clone(), ContractKind::ProgramEscrow, 2u32).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    facade.register(&contract, &ContractKind::ProgramEscrow, &2);

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &facade_id,
            fn_name: "deregister",
            args: (contract.clone(),).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    facade.deregister(&contract);

    assert_eq!(facade.get_contract(&contract), None);
}

/// A non-admin auth entry must not satisfy the admin gate on `deregister`.
#[test]
#[should_panic]
fn test_non_admin_cannot_deregister() {
    let (env, facade, admin) = setup_without_auth_mocks();
    let outsider = Address::generate(&env);
    let contract = Address::generate(&env);
    let facade_id = facade.address.clone();

    facade.init(&admin);

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &facade_id,
            fn_name: "register",
            args: (contract.clone(), ContractKind::GrainlifyCore, 3u32).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    facade.register(&contract, &ContractKind::GrainlifyCore, &3);

    env.mock_auths(&[MockAuth {
        address: &outsider,
        invoke: &MockAuthInvoke {
            contract: &facade_id,
            fn_name: "deregister",
            args: (contract.clone(),).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    facade.deregister(&contract);
}

// ---------------------------------------------------------------------------
// Registry Limits and Capacity Tests
// ---------------------------------------------------------------------------

/// Registering contracts up to MAX_REGISTRY_SIZE should succeed.
#[test]
fn test_register_up_to_max_capacity() {
    use crate::MAX_REGISTRY_SIZE;

    let (env, facade, admin) = setup();
    facade.init(&admin);

    // Register exactly MAX_REGISTRY_SIZE contracts
    for i in 0..MAX_REGISTRY_SIZE {
        let contract = Address::generate(&env);
        facade
            .register(&contract, &ContractKind::BountyEscrow, &i)
            .unwrap();
    }

    assert_eq!(facade.contract_count(), MAX_REGISTRY_SIZE);
}

/// Registering beyond MAX_REGISTRY_SIZE should fail with RegistryFull error.
#[test]
fn test_register_beyond_max_capacity_fails() {
    use crate::MAX_REGISTRY_SIZE;

    let (env, facade, admin) = setup();
    facade.init(&admin);

    // Fill the registry to capacity
    for i in 0..MAX_REGISTRY_SIZE {
        let contract = Address::generate(&env);
        facade
            .register(&contract, &ContractKind::BountyEscrow, &i)
            .unwrap();
    }

    // Try to register one more - should fail
    let extra_contract = Address::generate(&env);
    let result = facade.try_register(
        &extra_contract,
        &ContractKind::BountyEscrow,
        &MAX_REGISTRY_SIZE,
    );
    assert_eq!(result, Err(Ok(crate::FacadeError::RegistryFull)));

    // Registry size should remain at max capacity
    assert_eq!(facade.contract_count(), MAX_REGISTRY_SIZE);
}

/// Deregistering contracts should free up slots for new registrations.
#[test]
fn test_deregister_frees_slots_for_new_registrations() {
    use crate::MAX_REGISTRY_SIZE;

    let (env, facade, admin) = setup();
    facade.init(&admin);

    // Fill the registry to capacity
    let mut contracts = Vec::new(&env);
    for i in 0..MAX_REGISTRY_SIZE {
        let contract = Address::generate(&env);
        contracts.push_back(contract.clone());
        facade
            .register(&contract, &ContractKind::BountyEscrow, &i)
            .unwrap();
    }

    assert_eq!(facade.contract_count(), MAX_REGISTRY_SIZE);

    // Deregister one contract
    let removed_contract = contracts.get(0).unwrap().clone();
    facade.deregister(&removed_contract);

    assert_eq!(facade.contract_count(), MAX_REGISTRY_SIZE - 1);

    // Should be able to register a new contract now
    let new_contract = Address::generate(&env);
    facade
        .register(&new_contract, &ContractKind::ProgramEscrow, &999)
        .unwrap();

    assert_eq!(facade.contract_count(), MAX_REGISTRY_SIZE);
}

/// RegistryFull error should be returned even for admin when registry is full.
#[test]
fn test_registry_full_error_for_admin() {
    use crate::MAX_REGISTRY_SIZE;

    let (env, facade, admin) = setup_without_auth_mocks();
    let facade_id = facade.address.clone();
    facade.init(&admin);

    // Fill the registry to capacity with explicit auth for each registration
    for i in 0..MAX_REGISTRY_SIZE {
        let contract = Address::generate(&env);

        env.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &facade_id,
                fn_name: "register",
                args: (contract.clone(), ContractKind::BountyEscrow, i).into_val(&env),
                sub_invokes: &[],
            },
        }]);

        facade
            .register(&contract, &ContractKind::BountyEscrow, &i)
            .unwrap();
    }

    // Try to register one more with admin auth - should still fail
    let extra_contract = Address::generate(&env);
    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &facade_id,
            fn_name: "register",
            args: (
                extra_contract.clone(),
                ContractKind::BountyEscrow,
                MAX_REGISTRY_SIZE,
            )
                .into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let result = facade.try_register(
        &extra_contract,
        &ContractKind::BountyEscrow,
        &MAX_REGISTRY_SIZE,
    );
    assert_eq!(result, Err(Ok(crate::FacadeError::RegistryFull)));
}

// ---------------------------------------------------------------------------
// Pagination Tests
// ---------------------------------------------------------------------------

/// list_contracts with no parameters should return all entries.
#[test]
fn test_list_contracts_no_parameters_returns_all() {
    let (env, facade, admin) = setup();
    facade.init(&admin);

    let c1 = Address::generate(&env);
    let c2 = Address::generate(&env);
    let c3 = Address::generate(&env);

    facade.register(&c1, &ContractKind::BountyEscrow, &1);
    facade.register(&c2, &ContractKind::ProgramEscrow, &2);
    facade.register(&c3, &ContractKind::GrainlifyCore, &3);

    let all = facade.list_contracts(None, None).unwrap();
    assert_eq!(all.len(), 3);
    assert_eq!(all.get(0).unwrap().address, c1);
    assert_eq!(all.get(1).unwrap().address, c2);
    assert_eq!(all.get(2).unwrap().address, c3);
}

/// list_contracts with offset should skip the specified number of entries.
#[test]
fn test_list_contracts_with_offset() {
    let (env, facade, admin) = setup();
    facade.init(&admin);

    let c1 = Address::generate(&env);
    let c2 = Address::generate(&env);
    let c3 = Address::generate(&env);
    let c4 = Address::generate(&env);

    facade.register(&c1, &ContractKind::BountyEscrow, &1);
    facade.register(&c2, &ContractKind::ProgramEscrow, &2);
    facade.register(&c3, &ContractKind::GrainlifyCore, &3);
    facade.register(&c4, &ContractKind::SorobanEscrow, &4);

    // Skip first 2 entries
    let page = facade.list_contracts(Some(2), None).unwrap();
    assert_eq!(page.len(), 2);
    assert_eq!(page.get(0).unwrap().address, c3);
    assert_eq!(page.get(1).unwrap().address, c4);
}

/// list_contracts with limit should return at most that many entries.
#[test]
fn test_list_contracts_with_limit() {
    let (env, facade, admin) = setup();
    facade.init(&admin);

    let c1 = Address::generate(&env);
    let c2 = Address::generate(&env);
    let c3 = Address::generate(&env);
    let c4 = Address::generate(&env);

    facade.register(&c1, &ContractKind::BountyEscrow, &1);
    facade.register(&c2, &ContractKind::ProgramEscrow, &2);
    facade.register(&c3, &ContractKind::GrainlifyCore, &3);
    facade.register(&c4, &ContractKind::SorobanEscrow, &4);

    // Limit to 2 entries
    let page = facade.list_contracts(None, Some(2)).unwrap();
    assert_eq!(page.len(), 2);
    assert_eq!(page.get(0).unwrap().address, c1);
    assert_eq!(page.get(1).unwrap().address, c2);
}

/// list_contracts with both offset and limit should work correctly.
#[test]
fn test_list_contracts_with_offset_and_limit() {
    let (env, facade, admin) = setup();
    facade.init(&admin);

    let contracts: Vec<Address> = (0..10).map(|_| Address::generate(&env)).collect();

    for (i, contract) in contracts.iter().enumerate() {
        facade.register(contract, &ContractKind::BountyEscrow, &(i as u32));
    }

    // Get page 2: offset=4, limit=3
    let page = facade.list_contracts(Some(4), Some(3)).unwrap();
    assert_eq!(page.len(), 3);
    assert_eq!(page.get(0).unwrap().address, contracts.get(4).unwrap());
    assert_eq!(page.get(1).unwrap().address, contracts.get(5).unwrap());
    assert_eq!(page.get(2).unwrap().address, contracts.get(6).unwrap());
}

/// list_contracts with offset beyond total should return InvalidPagination error.
#[test]
fn test_list_contracts_offset_beyond_total_fails() {
    let (env, facade, admin) = setup();
    facade.init(&admin);

    let c1 = Address::generate(&env);
    facade.register(&c1, &ContractKind::BountyEscrow, &1);

    // Offset beyond total entries
    let result = facade.try_list_contracts(Some(5), None);
    assert_eq!(result, Err(Ok(crate::FacadeError::InvalidPagination)));
}

/// list_contracts with limit = 0 should return InvalidPagination error.
#[test]
fn test_list_contracts_zero_limit_fails() {
    let (env, facade, admin) = setup();
    facade.init(&admin);

    let c1 = Address::generate(&env);
    facade.register(&c1, &ContractKind::BountyEscrow, &1);

    // Zero limit
    let result = facade.try_list_contracts(None, Some(0));
    assert_eq!(result, Err(Ok(crate::FacadeError::InvalidPagination)));
}

/// list_contracts should handle edge case where offset + limit exceeds total.
#[test]
fn test_list_contracts_offset_plus_limit_exceeds_total() {
    let (env, facade, admin) = setup();
    facade.init(&admin);

    let contracts: Vec<Address> = (0..5).map(|_| Address::generate(&env)).collect();

    for (i, contract) in contracts.iter().enumerate() {
        facade.register(contract, &ContractKind::BountyEscrow, &(i as u32));
    }

    // Request more entries than exist starting from offset 3
    let page = facade.list_contracts(Some(3), Some(10)).unwrap();
    assert_eq!(page.len(), 2); // Only 2 entries remain from offset 3
    assert_eq!(page.get(0).unwrap().address, contracts.get(3).unwrap());
    assert_eq!(page.get(1).unwrap().address, contracts.get(4).unwrap());
}

/// list_contracts_all should return the complete registry for compatibility.
#[test]
fn test_list_contracts_all_compatibility() {
    let (env, facade, admin) = setup();
    facade.init(&admin);

    let contracts: Vec<Address> = (0..5).map(|_| Address::generate(&env)).collect();

    for (i, contract) in contracts.iter().enumerate() {
        facade.register(contract, &ContractKind::BountyEscrow, &(i as u32));
    }

    let all = facade.list_contracts_all();
    assert_eq!(all.len(), 5);

    for (i, entry) in all.iter().enumerate() {
        assert_eq!(entry.address, contracts.get(i).unwrap());
    }
}

/// contract_count should be consistent with list_contracts_all length.
#[test]
fn test_contract_count_consistency() {
    let (env, facade, admin) = setup();
    facade.init(&admin);

    // Initially empty
    assert_eq!(facade.contract_count(), 0);
    assert_eq!(facade.list_contracts_all().len(), 0);

    // Add some contracts
    let contracts: Vec<Address> = (0..7).map(|_| Address::generate(&env)).collect();

    for (i, contract) in contracts.iter().enumerate() {
        facade.register(contract, &ContractKind::BountyEscrow, &(i as u32));
        assert_eq!(facade.contract_count(), (i + 1) as u32);
        assert_eq!(facade.contract_count(), facade.list_contracts_all().len());
    }

    // Remove some contracts
    for i in 0..3 {
        facade.deregister(contracts.get(i).unwrap());
        assert_eq!(facade.contract_count(), (7 - i - 1) as u32);
        assert_eq!(facade.contract_count(), facade.list_contracts_all().len());
    }
}

// ---------------------------------------------------------------------------
// Pagination Integration Tests
// ---------------------------------------------------------------------------

/// Full pagination workflow test simulating real indexer usage.
#[test]
fn test_full_pagination_workflow() {
    let (env, facade, admin) = setup();
    facade.init(&admin);

    // Create a registry with 23 entries
    let contracts: Vec<Address> = (0..23).map(|_| Address::generate(&env)).collect();
    for (i, contract) in contracts.iter().enumerate() {
        facade.register(contract, &ContractKind::BountyEscrow, &(i as u32));
    }

    // Simulate pagination with page size 10
    let page_size = 10u32;
    let total = facade.contract_count();
    assert_eq!(total, 23);

    let mut all_collected = Vec::new(&env);
    let mut offset = 0u32;

    loop {
        let page = facade
            .list_contracts(Some(offset), Some(page_size))
            .unwrap();
        all_collected.extend_from_slice(&page);

        // If we got fewer than page_size, we're done
        if page.len() < page_size as usize {
            break;
        }

        offset += page_size;
    }

    // Verify we collected all entries
    assert_eq!(all_collected.len(), 23);
    for (i, entry) in all_collected.iter().enumerate() {
        assert_eq!(entry.address, contracts.get(i).unwrap());
    }
}

/// Pagination should work correctly after deregistration operations.
#[test]
fn test_pagination_after_deregistration() {
    let (env, facade, admin) = setup();
    facade.init(&admin);

    // Create initial registry
    let contracts: Vec<Address> = (0..15).map(|_| Address::generate(&env)).collect();
    for (i, contract) in contracts.iter().enumerate() {
        facade.register(contract, &ContractKind::BountyEscrow, &(i as u32));
    }

    // Remove some entries from the middle
    facade.deregister(contracts.get(3).unwrap());
    facade.deregister(contracts.get(7).unwrap());
    facade.deregister(contracts.get(11).unwrap());

    // Pagination should still work correctly
    let total = facade.contract_count();
    assert_eq!(total, 12);

    let page1 = facade.list_contracts(Some(0), Some(5)).unwrap();
    assert_eq!(page1.len(), 5);

    let page2 = facade.list_contracts(Some(5), Some(5)).unwrap();
    assert_eq!(page2.len(), 5);

    let page3 = facade.list_contracts(Some(10), Some(5)).unwrap();
    assert_eq!(page3.len(), 2);

    // Verify no duplicates across pages
    let mut all_addresses = Vec::new(&env);
    all_addresses.extend_from_slice(&page1);
    all_addresses.extend_from_slice(&page2);
    all_addresses.extend_from_slice(&page3);

    // Should have 12 unique addresses
    assert_eq!(all_addresses.len(), 12);

    // Check uniqueness (simple approach - no duplicates in positions)
    for i in 0..all_addresses.len() {
        for j in (i + 1)..all_addresses.len() {
            assert_ne!(all_addresses.get(i).unwrap(), all_addresses.get(j).unwrap());
        }
    }
}
