//! # Token Allowlist Enforcement Tests
//!
//! Covers every branch of the token allowlist feature introduced in v2.5.0:
//!
//! - Default state: enforcement disabled (empty list → any token accepted)
//! - `add_allowed_token`: happy path, duplicate rejection, event fields
//! - `remove_allowed_token`: happy path, missing-token rejection, event fields
//! - `is_token_allowed`: view semantics with empty / non-empty list
//! - `get_allowed_tokens`: returns current list
//! - `get_token_allowlist_schema_version`: returns V1 after init
//! - `init_program` / `initialize_program`: accepted when on list, rejected when not
//! - `TokenRejectedEvent`: emitted on rejection
//! - `TokenAllowlistUpdatedEvent`: correct fields on add and remove
//! - `TokenAllowlistSchemaVersionSet`: emitted during init
//! - Multi-token list: correct membership checks
//! - Remove last token: enforcement re-disabled
//! - Admin-only guard on add / remove
//! - Deterministic ordering: rejection happens before program storage write

#![cfg(test)]

use crate::{
    ProgramEscrowContract, ProgramEscrowContractClient,
    TokenAllowlistUpdatedEvent, TokenRejectedEvent, TokenAllowlistSchemaVersionSet,
    TOKEN_ALLOWLIST_SCHEMA_VERSION_V1, EVENT_VERSION_V2,
};
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    Address, Env, String, Symbol, TryIntoVal,
};

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Minimal contract + admin setup. Does NOT call `init_program` so tests can
/// control which token they pass.
fn setup_contract(env: &Env) -> (ProgramEscrowContractClient<'static>, Address) {
    env.mock_all_auths();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize_contract(&admin);
    (client, admin)
}

/// Register a fresh SAC token and return its address.
fn make_token(env: &Env) -> Address {
    let token_admin = Address::generate(env);
    let sac = env.register_stellar_asset_contract_v2(token_admin);
    sac.address()
}

/// Call `init_program` with a given token.
fn init_with_token(client: &ProgramEscrowContractClient, env: &Env, token: &Address) {
    let program_id = String::from_str(env, "test-prog");
    let admin = Address::generate(env);
    client.init_program(&program_id, &admin, token, &admin, &None, &None);
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. Default state: enforcement disabled
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_default_allowlist_is_empty() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    assert_eq!(client.get_allowed_tokens().len(), 0);
}

#[test]
fn test_is_token_allowed_returns_true_when_list_empty() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let token = make_token(&env);
    // Empty list → enforcement off → any token is "allowed"
    assert!(client.is_token_allowed(&token));
}

#[test]
fn test_init_program_succeeds_with_any_token_when_list_empty() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let token = make_token(&env);
    // Should not panic
    init_with_token(&client, &env, &token);
    let program_id = String::from_str(&env, "test-prog");
    assert!(client.program_exists_by_id(&program_id));
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Schema version
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_schema_version_is_v1_after_init() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    assert_eq!(
        client.get_allowlist_schema_version(),
        TOKEN_ALLOWLIST_SCHEMA_VERSION_V1
    );
}

#[test]
fn test_schema_version_event_emitted_on_init() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    env.ledger().with_mut(|li| li.timestamp = 9_000);
    client.initialize_contract(&admin);

    let events = env.events().all();
    let schema_event = events.iter().find(|e| {
        if let Some(t0) = e.1.get(0) {
            let sym: Result<Symbol, _> = t0.try_into_val(&env);
            sym.map(|s| s == Symbol::new(&env, "TkAlSch")).unwrap_or(false)
        } else {
            false
        }
    });

    assert!(schema_event.is_some(), "TokenAllowlistSchemaVersionSet must be emitted on init");
    let payload: TokenAllowlistSchemaVersionSet =
        schema_event.unwrap().2.try_into_val(&env).unwrap();
    assert_eq!(payload.version, EVENT_VERSION_V2);
    assert_eq!(payload.schema_version, TOKEN_ALLOWLIST_SCHEMA_VERSION_V1);
    assert_eq!(payload.timestamp, 9_000);
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. add_allowed_token
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_add_allowed_token_happy_path() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let token = make_token(&env);

    client.add_allowed_token(&token);

    let list = client.get_allowed_tokens();
    assert_eq!(list.len(), 1);
    assert_eq!(list.get(0).unwrap(), token);
}

#[test]
fn test_add_multiple_tokens() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let t1 = make_token(&env);
    let t2 = make_token(&env);
    let t3 = make_token(&env);

    client.add_allowed_token(&t1);
    client.add_allowed_token(&t2);
    client.add_allowed_token(&t3);

    assert_eq!(client.get_allowed_tokens().len(), 3);
}

#[test]
#[should_panic(expected = "Token already on allowlist")]
fn test_add_duplicate_token_panics() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let token = make_token(&env);

    client.add_allowed_token(&token);
    client.add_allowed_token(&token); // must panic
}

#[test]
fn test_add_token_emits_event_with_correct_fields() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let token = make_token(&env);

    env.ledger().with_mut(|li| li.timestamp = 42_000);
    client.add_allowed_token(&token);

    let events = env.events().all();
    let ev = events.iter().find(|e| {
        if let Some(t0) = e.1.get(0) {
            let sym: Result<Symbol, _> = t0.try_into_val(&env);
            sym.map(|s| s == Symbol::new(&env, "TkAllow")).unwrap_or(false)
        } else {
            false
        }
    });

    assert!(ev.is_some(), "TokenAllowlistUpdatedEvent must be emitted on add");
    let payload: TokenAllowlistUpdatedEvent = ev.unwrap().2.try_into_val(&env).unwrap();
    assert_eq!(payload.version, EVENT_VERSION_V2);
    assert_eq!(payload.token, token);
    assert!(payload.added, "added must be true");
    assert_eq!(payload.updated_by, admin);
    assert_eq!(payload.timestamp, 42_000);
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. remove_allowed_token
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_remove_allowed_token_happy_path() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let token = make_token(&env);

    client.add_allowed_token(&token);
    assert_eq!(client.get_allowed_tokens().len(), 1);

    client.remove_allowed_token(&token);
    assert_eq!(client.get_allowed_tokens().len(), 0);
}

#[test]
fn test_remove_one_of_many_tokens() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let t1 = make_token(&env);
    let t2 = make_token(&env);
    let t3 = make_token(&env);

    client.add_allowed_token(&t1);
    client.add_allowed_token(&t2);
    client.add_allowed_token(&t3);

    client.remove_allowed_token(&t2);

    let list = client.get_allowed_tokens();
    assert_eq!(list.len(), 2);
    for item in list.iter() {
        assert_ne!(item, t2, "removed token must not remain in list");
    }
}

#[test]
#[should_panic(expected = "Token not in allowlist")]
fn test_remove_absent_token_panics() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let token = make_token(&env);
    client.remove_allowed_token(&token); // never added — must panic
}

#[test]
fn test_remove_token_emits_event_with_correct_fields() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let token = make_token(&env);

    client.add_allowed_token(&token);

    env.ledger().with_mut(|li| li.timestamp = 77_000);
    client.remove_allowed_token(&token);

    let events = env.events().all();
    // Find the last TkAllow event (the remove one)
    let ev = events.iter().rev().find(|e| {
        if let Some(t0) = e.1.get(0) {
            let sym: Result<Symbol, _> = t0.try_into_val(&env);
            sym.map(|s| s == Symbol::new(&env, "TkAllow")).unwrap_or(false)
        } else {
            false
        }
    });

    assert!(ev.is_some(), "TokenAllowlistUpdatedEvent must be emitted on remove");
    let payload: TokenAllowlistUpdatedEvent = ev.unwrap().2.try_into_val(&env).unwrap();
    assert_eq!(payload.version, EVENT_VERSION_V2);
    assert_eq!(payload.token, token);
    assert!(!payload.added, "added must be false on remove");
    assert_eq!(payload.updated_by, admin);
    assert_eq!(payload.timestamp, 77_000);
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. is_token_allowed view
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_is_token_allowed_true_for_listed_token() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let token = make_token(&env);

    client.add_allowed_token(&token);
    assert!(client.is_token_allowed(&token));
}

#[test]
fn test_is_token_allowed_false_for_unlisted_token() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let listed = make_token(&env);
    let unlisted = make_token(&env);

    client.add_allowed_token(&listed);
    assert!(!client.is_token_allowed(&unlisted));
}

#[test]
fn test_is_token_allowed_true_after_list_cleared() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let token = make_token(&env);
    let other = make_token(&env);

    client.add_allowed_token(&token);
    client.remove_allowed_token(&token);

    // List is now empty → enforcement off → any token is "allowed"
    assert!(client.is_token_allowed(&other));
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. init_program enforcement
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_init_program_succeeds_with_listed_token() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let token = make_token(&env);

    client.add_allowed_token(&token);
    init_with_token(&client, &env, &token); // must not panic

    let program_id = String::from_str(&env, "test-prog");
    assert!(client.program_exists_by_id(&program_id));
}

#[test]
#[should_panic(expected = "Token not on allowlist")]
fn test_init_program_rejected_with_unlisted_token() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let listed = make_token(&env);
    let unlisted = make_token(&env);

    client.add_allowed_token(&listed);
    init_with_token(&client, &env, &unlisted); // must panic
}

#[test]
fn test_init_program_succeeds_after_token_added_to_list() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let token = make_token(&env);
    let other = make_token(&env);

    // Enable enforcement with a different token first
    client.add_allowed_token(&other);
    // Now add the target token
    client.add_allowed_token(&token);

    init_with_token(&client, &env, &token); // must not panic
    let program_id = String::from_str(&env, "test-prog");
    assert!(client.program_exists_by_id(&program_id));
}

#[test]
fn test_init_program_succeeds_after_list_cleared() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let token = make_token(&env);
    let sentinel = make_token(&env);

    // Enable enforcement with a sentinel token, then remove it
    client.add_allowed_token(&sentinel);
    client.remove_allowed_token(&sentinel);

    // List empty → any token accepted
    init_with_token(&client, &env, &token);
    let program_id = String::from_str(&env, "test-prog");
    assert!(client.program_exists_by_id(&program_id));
}

// ─────────────────────────────────────────────────────────────────────────────
// 7. TokenRejectedEvent emitted on rejection
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_token_rejected_event_emitted_on_rejection() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let listed = make_token(&env);
    let unlisted = make_token(&env);

    client.add_allowed_token(&listed);

    env.ledger().with_mut(|li| li.timestamp = 55_000);

    // Attempt init with unlisted token — will panic, but events are still recorded
    let result = client.try_init_program(
        &String::from_str(&env, "test-prog"),
        &Address::generate(&env),
        &unlisted,
        &Address::generate(&env),
        &None,
        &None,
    );
    assert!(result.is_err(), "init with unlisted token must fail");

    // Verify TokenRejectedEvent was emitted
    let events = env.events().all();
    let ev = events.iter().find(|e| {
        if let Some(t0) = e.1.get(0) {
            let sym: Result<Symbol, _> = t0.try_into_val(&env);
            sym.map(|s| s == Symbol::new(&env, "TkReject")).unwrap_or(false)
        } else {
            false
        }
    });

    assert!(ev.is_some(), "TokenRejectedEvent must be emitted on rejection");
    let payload: TokenRejectedEvent = ev.unwrap().2.try_into_val(&env).unwrap();
    assert_eq!(payload.version, EVENT_VERSION_V2);
    assert_eq!(payload.token, unlisted);
    assert_eq!(payload.timestamp, 55_000);
}

#[test]
fn test_rejected_program_not_stored_in_registry() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let listed = make_token(&env);
    let unlisted = make_token(&env);

    client.add_allowed_token(&listed);

    let program_id = String::from_str(&env, "test-prog");

    // Attempt init with unlisted token — ignore the error
    let _ = client.try_init_program(
        &program_id,
        &Address::generate(&env),
        &unlisted,
        &Address::generate(&env),
        &None,
        &None,
    );

    // The program must NOT have been registered
    assert!(
        !client.program_exists_by_id(&program_id),
        "rejected program must not be stored"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 8. Multi-token list membership
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_multi_token_list_correct_membership() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);

    let t1 = make_token(&env);
    let t2 = make_token(&env);
    let t3 = make_token(&env);
    let t4 = make_token(&env);
    let t5 = make_token(&env);
    let outside = make_token(&env);

    for t in [&t1, &t2, &t3, &t4, &t5] {
        client.add_allowed_token(t);
    }

    for t in [&t1, &t2, &t3, &t4, &t5] {
        assert!(client.is_token_allowed(t), "listed token must be allowed");
    }
    assert!(!client.is_token_allowed(&outside), "unlisted token must be rejected");
}

// ─────────────────────────────────────────────────────────────────────────────
// 9. Admin-only guard
// ─────────────────────────────────────────────────────────────────────────────

#[test]
#[should_panic]
fn test_add_allowed_token_requires_admin() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    env.mock_all_auths();
    client.initialize_contract(&admin);

    let token = make_token(&env);
    // Call without any auth mock — must panic (Unauthorized)
    client.mock_auths(&[]).add_allowed_token(&token);
}

#[test]
#[should_panic]
fn test_remove_allowed_token_requires_admin() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    env.mock_all_auths();
    client.initialize_contract(&admin);

    let token = make_token(&env);
    client.add_allowed_token(&token);

    // Remove without auth — must panic
    client.mock_auths(&[]).remove_allowed_token(&token);
}

// ─────────────────────────────────────────────────────────────────────────────
// 10. Idempotency: add → remove → add same token
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_add_remove_add_same_token() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let token = make_token(&env);

    client.add_allowed_token(&token);
    assert_eq!(client.get_allowed_tokens().len(), 1);

    client.remove_allowed_token(&token);
    assert_eq!(client.get_allowed_tokens().len(), 0);

    // Re-add must succeed
    client.add_allowed_token(&token);
    assert_eq!(client.get_allowed_tokens().len(), 1);
    assert!(client.is_token_allowed(&token));
}

// ─────────────────────────────────────────────────────────────────────────────
// 11. initialize_program (alias) also enforces allowlist
// ─────────────────────────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "Token not on allowlist")]
fn test_initialize_program_alias_also_enforces_allowlist() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let listed = make_token(&env);
    let unlisted = make_token(&env);

    client.add_allowed_token(&listed);

    let program_id = String::from_str(&env, "alias-prog");
    let admin = Address::generate(&env);
    // initialize_program is the canonical entrypoint — must also enforce
    client.initialize_program(&program_id, &admin, &unlisted, &admin, &None, &None);
}

// ─────────────────────────────────────────────────────────────────────────────
// 12. get_allowed_tokens returns correct snapshot
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_get_allowed_tokens_snapshot() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let t1 = make_token(&env);
    let t2 = make_token(&env);

    assert_eq!(client.get_allowed_tokens().len(), 0);

    client.add_allowed_token(&t1);
    assert_eq!(client.get_allowed_tokens().len(), 1);

    client.add_allowed_token(&t2);
    assert_eq!(client.get_allowed_tokens().len(), 2);

    client.remove_allowed_token(&t1);
    let list = client.get_allowed_tokens();
    assert_eq!(list.len(), 1);
    assert_eq!(list.get(0).unwrap(), t2);
}

// ─────────────────────────────────────────────────────────────────────────────
// 13. Enforcement disabled when list is empty (boundary)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_enforcement_disabled_boundary_empty_list() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);

    // Add 3 tokens then remove all — enforcement must be off
    let t1 = make_token(&env);
    let t2 = make_token(&env);
    let t3 = make_token(&env);

    client.add_allowed_token(&t1);
    client.add_allowed_token(&t2);
    client.add_allowed_token(&t3);

    client.remove_allowed_token(&t1);
    client.remove_allowed_token(&t2);
    client.remove_allowed_token(&t3);

    assert_eq!(client.get_allowed_tokens().len(), 0);

    // Any token should now be accepted
    let random = make_token(&env);
    assert!(client.is_token_allowed(&random));
    init_with_token(&client, &env, &random);
    let program_id = String::from_str(&env, "test-prog");
    assert!(client.program_exists_by_id(&program_id));
}
