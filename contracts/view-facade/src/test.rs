#![cfg(test)]

use crate::{ContractKind, FacadeError, ViewFacade, ViewFacadeClient};
use soroban_sdk::{testutils::Address as _, Address, Env};

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

    assert!(found, "Initialized event must be emitted with correct admin");
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

    let all = facade.list_contracts();
    assert_eq!(all.len(), 2);
    assert_eq!(all.get(0).unwrap().address, c1);
    assert_eq!(all.get(0).unwrap().kind, ContractKind::BountyEscrow);
    assert_eq!(all.get(1).unwrap().address, c2);
    assert_eq!(all.get(1).unwrap().kind, ContractKind::ProgramEscrow);
}

/// If duplicate addresses are registered, `get_contract` returns the first match.
#[test]
fn test_get_contract_returns_first_match_for_duplicate_addresses() {
    let (env, facade, admin) = setup();
    let duplicate = Address::generate(&env);

    facade.init(&admin);
    facade.register(&duplicate, &ContractKind::BountyEscrow, &1);
    facade.register(&duplicate, &ContractKind::ProgramEscrow, &2);

    let all = facade.list_contracts();
    assert_eq!(all.len(), 2);

    let entry = facade.get_contract(&duplicate).unwrap();
    assert_eq!(entry.kind, ContractKind::BountyEscrow);
    assert_eq!(entry.version, 1);
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
