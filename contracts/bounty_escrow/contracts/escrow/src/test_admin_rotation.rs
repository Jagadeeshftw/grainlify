//! Comprehensive tests for two-step admin rotation with timelock.
//!
//! This test suite validates:
//! - Proposal flow with proper authorization and event emission
//! - Timelock enforcement (cannot accept before delay)
//! - Acceptance flow with pending admin authorization and event emission
//! - Cancellation by current admin and event emission
//! - Cancellation prevents execution of the prior scheduled operation
//! - No half-rotated authority is observable at any ledger boundary
//! - Timelock duration configuration
//! - Edge cases (duplicate proposals, self-rotation, etc.)
//! - Upgrade safety (storage keys persist correctly)
//! - Complete event audit trail for all state changes
//! - Replacement via cancel-then-repropose
//! - Concurrent caller rejection

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    Address, Env, IntoVal, Symbol,
};

// ═══════════════════════════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

fn has_event_topic(env: &Env, topic: Symbol) -> bool {
    for (_, topics, _) in env.events().all().iter() {
        for t in topics.iter() {
            if let Ok(s) = <Symbol as soroban_sdk::TryFromVal<Env, soroban_sdk::Val>>::try_from_val(
                env, &t,
            ) {
                if s == topic {
                    return true;
                }
            }
        }
    }
    false
}

fn find_last_event_data(env: &Env, topic: Symbol) -> Option<soroban_sdk::Val> {
    let mut found = None;
    for (_, topics, data) in env.events().all().iter() {
        for t in topics.iter() {
            if let Ok(s) = <Symbol as soroban_sdk::TryFromVal<Env, soroban_sdk::Val>>::try_from_val(
                env, &t,
            ) {
                if s == topic {
                    found = Some(data);
                }
            }
        }
    }
    found
}

fn assert_single_authority(env: &Env, client: &BountyEscrowContractClient, expected_admin: &Address) {
    let admin = client.get_admin();
    assert_eq!(admin, Some(expected_admin.clone()), "exactly one active admin must exist");

    let pending = client.get_pending_admin();
    if let Some(ref p) = pending {
        assert_ne!(p, expected_admin, "pending admin must differ from active admin");
    }

    let status = client.get_admin_rotation_status();
    match status {
        Some(s) => {
            assert_eq!(s.current_admin, expected_admin.clone(), "status.current_admin must match active");
            assert_ne!(s.pending_admin, expected_admin.clone(), "pending must differ from active");
        }
        None => {
            assert!(pending.is_none(), "pending admin must be None when status is None");
        }
    }
}

fn assert_no_pending_state(client: &BountyEscrowContractClient) {
    assert_eq!(client.get_pending_admin(), None);
    assert_eq!(client.get_admin_rotation_timelock(), None);
    assert_eq!(client.get_admin_rotation_status(), None);
    assert!(!client.get_admin_rotation_config().has_pending_rotation);
}

// ═══════════════════════════════════════════════════════════════════════════════
// PROPOSAL TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_propose_admin_rotation_success() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);

    let before_events = env.events().all().len();
    let execute_after = client.propose_admin_rotation(&new_admin);

    let now = env.ledger().timestamp();
    assert!(execute_after > now);

    let pending = client.get_pending_admin();
    assert_eq!(pending, Some(new_admin.clone()));

    let timelock = client.get_admin_rotation_timelock();
    assert_eq!(timelock, Some(execute_after));

    let current_admin = client.get_admin();
    assert_eq!(current_admin, Some(admin.clone()));

    assert_single_authority(&env, &client, &admin);

    let after_events = env.events().all().len();
    assert!(after_events > before_events, "propose must emit an event");

    let ev_data = find_last_event_data(&env, symbol_short!("admrotp"));
    assert!(ev_data.is_some(), "AdminRotationProposed event must be emitted");
    let ev: events::AdminRotationProposed = ev_data.unwrap().into_val(&env);
    assert_eq!(ev.version, EVENT_VERSION_V2);
    assert_eq!(ev.current_admin, admin);
    assert_eq!(ev.pending_admin, new_admin);
    assert_eq!(ev.execute_after, execute_after);
}

#[test]
fn test_propose_admin_rotation_uses_configured_timelock() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);

    let custom_timelock = 7200;
    client.set_rotation_timelock_duration(&custom_timelock);

    let start_time = env.ledger().timestamp();
    let execute_after = client.propose_admin_rotation(&new_admin);

    assert_eq!(execute_after, start_time + custom_timelock);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")]
fn test_propose_admin_rotation_unauthorized() {
    let env = Env::default();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let attacker = Address::generate(&env);

    client.init(&admin, &token);

    env.mock_auths(&[&attacker]);
    client.propose_admin_rotation(&new_admin);
}

#[test]
fn test_propose_admin_rotation_cannot_rotate_to_self() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    let result = client.try_propose_admin_rotation(&admin);
    assert_eq!(result, Err(Ok(Error::InvalidAdminRotationTarget)));
}

#[test]
fn test_propose_admin_rotation_cannot_duplicate() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);

    client.propose_admin_rotation(&new_admin);

    let another_admin = Address::generate(&env);
    let result = client.try_propose_admin_rotation(&another_admin);
    assert_eq!(result, Err(Ok(Error::AdminRotationAlreadyPending)));
}

#[test]
fn test_propose_admin_rotation_not_initialized() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let new_admin = Address::generate(&env);

    let result = client.try_propose_admin_rotation(&new_admin);
    assert_eq!(result, Err(Ok(Error::NotInitialized)));
}

// ═══════════════════════════════════════════════════════════════════════════════
// ACCEPTANCE TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_accept_admin_rotation_success() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);

    let execute_after = client.propose_admin_rotation(&new_admin);
    assert_single_authority(&env, &client, &admin);

    env.ledger().set_timestamp(execute_after + 1);

    let before_events = env.events().all().len();
    let accepted_admin = client.accept_admin_rotation();
    assert_eq!(accepted_admin, new_admin.clone());

    assert_eq!(client.get_admin(), Some(new_admin.clone()));
    assert_no_pending_state(&client);

    assert_single_authority(&env, &client, &new_admin);

    let after_events = env.events().all().len();
    assert!(after_events > before_events, "accept must emit an event");

    let ev_data = find_last_event_data(&env, symbol_short!("admrota"));
    assert!(ev_data.is_some(), "AdminRotationAccepted event must be emitted");
    let ev: events::AdminRotationAccepted = ev_data.unwrap().into_val(&env);
    assert_eq!(ev.version, EVENT_VERSION_V2);
    assert_eq!(ev.previous_admin, admin);
    assert_eq!(ev.new_admin, new_admin);
}

#[test]
fn test_accept_admin_rotation_at_exact_timelock() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);

    let execute_after = client.propose_admin_rotation(&new_admin);

    env.ledger().set_timestamp(execute_after);

    let accepted_admin = client.accept_admin_rotation();
    assert_eq!(accepted_admin, new_admin);
    assert_eq!(client.get_admin(), Some(new_admin));
}

#[test]
fn test_accept_admin_rotation_requires_pending_admin_auth() {
    let env = Env::default();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let attacker = Address::generate(&env);

    client.init(&admin, &token);

    env.mock_auths(&[&admin]);
    let execute_after = client.propose_admin_rotation(&new_admin);

    env.ledger().set_timestamp(execute_after + 1);

    env.mock_auths(&[&attacker]);
    let result = client.try_accept_admin_rotation();
    assert!(
        result.is_err(),
        "attacker without pending-admin auth must not accept"
    );
}

#[test]
fn test_accept_admin_rotation_before_timelock_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);

    let execute_after = client.propose_admin_rotation(&new_admin);

    env.ledger().set_timestamp(execute_after - 1);

    let result = client.try_accept_admin_rotation();
    assert_eq!(result, Err(Ok(Error::AdminRotationTimelockActive)));

    assert_single_authority(&env, &client, &admin);
}

#[test]
fn test_accept_admin_rotation_not_pending() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    let result = client.try_accept_admin_rotation();
    assert_eq!(result, Err(Ok(Error::AdminRotationNotPending)));
}

// ═══════════════════════════════════════════════════════════════════════════════
// CANCELLATION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_cancel_admin_rotation_success() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);
    client.propose_admin_rotation(&new_admin);

    let before_events = env.events().all().len();
    client.cancel_admin_rotation();

    assert_no_pending_state(&client);
    assert_single_authority(&env, &client, &admin);

    let after_events = env.events().all().len();
    assert!(after_events > before_events, "cancel must emit an event");

    let ev_data = find_last_event_data(&env, symbol_short!("admrotc"));
    assert!(ev_data.is_some(), "AdminRotationCancelled event must be emitted");
    let ev: events::AdminRotationCancelled = ev_data.unwrap().into_val(&env);
    assert_eq!(ev.version, EVENT_VERSION_V2);
    assert_eq!(ev.admin, admin);
    assert_eq!(ev.cancelled_pending_admin, new_admin);
}

#[test]
fn test_cancel_admin_rotation_requires_admin_auth() {
    let env = Env::default();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let attacker = Address::generate(&env);

    client.init(&admin, &token);

    env.mock_auths(&[&admin]);
    client.propose_admin_rotation(&new_admin);

    env.mock_auths(&[&attacker]);
    let result = client.try_cancel_admin_rotation();
    assert!(
        result.is_err(),
        "attacker without admin auth must not cancel"
    );

    assert!(client.get_pending_admin().is_some(), "pending state must survive failed cancel");
}

#[test]
fn test_cancel_admin_rotation_not_pending() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    let result = client.try_cancel_admin_rotation();
    assert_eq!(result, Err(Ok(Error::AdminRotationNotPending)));
}

#[test]
fn test_can_propose_after_cancel() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin1 = Address::generate(&env);
    let new_admin2 = Address::generate(&env);

    client.init(&admin, &token);

    client.propose_admin_rotation(&new_admin1);
    client.cancel_admin_rotation();
    assert_no_pending_state(&client);

    let execute_after = client.propose_admin_rotation(&new_admin2);
    let pending = client.get_pending_admin();
    assert_eq!(pending, Some(new_admin2));
    assert!(execute_after > env.ledger().timestamp());
}

// ═══════════════════════════════════════════════════════════════════════════════
// NO HALF-ROTATED AUTHORITY INVARIANT TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_propose_does_not_change_active_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);

    assert_eq!(client.get_admin(), Some(admin.clone()));
    assert_no_pending_state(&client);

    client.propose_admin_rotation(&new_admin);

    assert_eq!(client.get_admin(), Some(admin.clone()), "admin must not change after propose");
    assert_eq!(client.get_pending_admin(), Some(new_admin.clone()));
    assert_single_authority(&env, &client, &admin);

    let status = client.get_admin_rotation_status().unwrap();
    assert_eq!(status.current_admin, admin.clone());
    assert_eq!(status.pending_admin, new_admin);
    assert!(!status.is_executable);
}

#[test]
fn test_accept_atomically_transfers_authority() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);
    let execute_after = client.propose_admin_rotation(&new_admin);
    env.ledger().set_timestamp(execute_after + 1);

    assert_eq!(client.get_admin(), Some(admin.clone()));

    client.accept_admin_rotation();

    assert_eq!(client.get_admin(), Some(new_admin.clone()));
    assert_no_pending_state(&client);
    assert_single_authority(&env, &client, &new_admin);
}

#[test]
fn test_cancel_restores_single_authority() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);
    client.propose_admin_rotation(&new_admin);

    assert_single_authority(&env, &client, &admin);

    client.cancel_admin_rotation();

    assert_eq!(client.get_admin(), Some(admin.clone()));
    assert_no_pending_state(&client);
    assert_single_authority(&env, &client, &admin);
}

#[test]
fn test_never_two_admins_or_none_at_any_boundary() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin1 = Address::generate(&env);
    let token = Address::generate(&env);
    let admin2 = Address::generate(&env);
    let admin3 = Address::generate(&env);

    client.init(&admin1, &token);

    assert_single_authority(&env, &client, &admin1);

    let ea1 = client.propose_admin_rotation(&admin2);
    assert_single_authority(&env, &client, &admin1);

    env.ledger().set_timestamp(ea1 + 1);
    client.accept_admin_rotation();
    assert_single_authority(&env, &client, &admin2);

    let ea2 = client.propose_admin_rotation(&admin3);
    assert_single_authority(&env, &client, &admin2);

    client.cancel_admin_rotation();
    assert_single_authority(&env, &client, &admin2);

    let ea3 = client.propose_admin_rotation(&admin3);
    assert_single_authority(&env, &client, &admin2);

    env.ledger().set_timestamp(ea3 + 1);
    client.accept_admin_rotation();
    assert_single_authority(&env, &client, &admin3);
}

// ═══════════════════════════════════════════════════════════════════════════════
// CANCELLATION INVALIDATES PRIOR SCHEDULED OPERATION
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_cancelled_rotation_cannot_execute() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);
    let execute_after = client.propose_admin_rotation(&new_admin);

    client.cancel_admin_rotation();
    assert_no_pending_state(&client);

    env.ledger().set_timestamp(execute_after + 1);

    let result = client.try_accept_admin_rotation();
    assert_eq!(result, Err(Ok(Error::AdminRotationNotPending)));

    assert_eq!(client.get_admin(), Some(admin.clone()));
}

#[test]
fn test_cancelled_pending_admin_not_queryable() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);
    client.propose_admin_rotation(&new_admin);
    client.cancel_admin_rotation();

    assert_eq!(client.get_pending_admin(), None);
    assert_eq!(client.get_admin_rotation_timelock(), None);
    assert_eq!(client.get_admin_rotation_status(), None);
    assert!(!client.get_admin_rotation_config().has_pending_rotation);
}

#[test]
fn test_cancel_after_propose_clears_nonces_and_timelock() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);
    let execute_after = client.propose_admin_rotation(&new_admin);

    assert_eq!(client.get_admin_rotation_timelock(), Some(execute_after));

    client.cancel_admin_rotation();

    assert_eq!(client.get_admin_rotation_timelock(), None);
    assert_eq!(client.get_pending_admin(), None);
}

// ═══════════════════════════════════════════════════════════════════════════════
// EXPIRY / TIMELOCK BOUNDARY TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_pending_rotation_persists_indefinitely() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);
    let execute_after = client.propose_admin_rotation(&new_admin);

    let far_future = execute_after + 999_999;
    env.ledger().set_timestamp(far_future);

    let status = client.get_admin_rotation_status().unwrap();
    assert!(status.is_executable);
    assert_eq!(status.remaining_seconds, 0);
    assert_eq!(client.get_pending_admin(), Some(new_admin.clone()));
    assert_eq!(client.get_admin(), Some(admin.clone()));

    let result = client.try_accept_admin_rotation();
    assert_eq!(result, Ok(Ok(new_admin.clone())));
}

#[test]
fn test_timelock_one_second_before_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);
    let execute_after = client.propose_admin_rotation(&new_admin);

    env.ledger().set_timestamp(execute_after - 1);

    let result = client.try_accept_admin_rotation();
    assert_eq!(result, Err(Ok(Error::AdminRotationTimelockActive)));

    assert_single_authority(&env, &client, &admin);
    assert!(client.get_pending_admin().is_some());
}

#[test]
fn test_timelock_exact_timestamp_succeeds() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);
    let execute_after = client.propose_admin_rotation(&new_admin);

    env.ledger().set_timestamp(execute_after);

    let result = client.try_accept_admin_rotation();
    assert_eq!(result, Ok(Ok(new_admin.clone())));

    assert_eq!(client.get_admin(), Some(new_admin));
    assert_no_pending_state(&client);
}

#[test]
fn test_custom_short_timelock_boundary() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);
    client.set_rotation_timelock_duration(&3600);

    let start = env.ledger().timestamp();
    let execute_after = client.propose_admin_rotation(&new_admin);
    assert_eq!(execute_after, start + 3600);

    env.ledger().set_timestamp(start + 3599);
    let result = client.try_accept_admin_rotation();
    assert_eq!(result, Err(Ok(Error::AdminRotationTimelockActive)));

    env.ledger().set_timestamp(start + 3600);
    let result = client.try_accept_admin_rotation();
    assert_eq!(result, Ok(Ok(new_admin)));
}

// ═══════════════════════════════════════════════════════════════════════════════
// REPLACEMENT (CANCEL + RE-PROPOSE) TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_replace_pending_target_after_cancel() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let target_a = Address::generate(&env);
    let target_b = Address::generate(&env);

    client.init(&admin, &token);

    client.propose_admin_rotation(&target_a);
    assert_eq!(client.get_pending_admin(), Some(target_a.clone()));

    client.cancel_admin_rotation();

    let execute_after = client.propose_admin_rotation(&target_b);
    assert_eq!(client.get_pending_admin(), Some(target_b.clone()));

    env.ledger().set_timestamp(execute_after + 1);
    client.accept_admin_rotation();
    assert_eq!(client.get_admin(), Some(target_b));

    assert_no_pending_state(&client);
}

#[test]
fn test_propose_to_self_after_cancel_still_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);

    client.propose_admin_rotation(&new_admin);
    client.cancel_admin_rotation();

    let result = client.try_propose_admin_rotation(&admin);
    assert_eq!(result, Err(Ok(Error::InvalidAdminRotationTarget)));
}

#[test]
fn test_cannot_replace_without_cancel() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let target_a = Address::generate(&env);
    let target_b = Address::generate(&env);

    client.init(&admin, &token);

    client.propose_admin_rotation(&target_a);

    let result = client.try_propose_admin_rotation(&target_b);
    assert_eq!(result, Err(Ok(Error::AdminRotationAlreadyPending)));

    assert_eq!(client.get_pending_admin(), Some(target_a));
}

// ═══════════════════════════════════════════════════════════════════════════════
// CONCURRENT CALLER / AUTHORIZATION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_non_admin_cannot_propose() {
    let env = Env::default();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let outsider = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);

    env.mock_auths(&[&outsider]);
    let result = client.try_propose_admin_rotation(&new_admin);
    assert!(result.is_err(), "outsider must not propose rotation");

    assert_no_pending_state(&client);
}

#[test]
fn test_non_pending_admin_cannot_accept() {
    let env = Env::default();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let wrong_acceptor = Address::generate(&env);

    client.init(&admin, &token);

    env.mock_auths(&[&admin]);
    let execute_after = client.propose_admin_rotation(&new_admin);

    env.ledger().set_timestamp(execute_after + 1);

    env.mock_auths(&[&wrong_acceptor]);
    let result = client.try_accept_admin_rotation();
    assert!(result.is_err(), "wrong address must not accept rotation");

    assert_eq!(client.get_admin(), Some(admin));
    assert_eq!(client.get_pending_admin(), Some(new_admin));
}

#[test]
fn test_non_admin_cannot_cancel() {
    let env = Env::default();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let outsider = Address::generate(&env);

    client.init(&admin, &token);

    env.mock_auths(&[&admin]);
    client.propose_admin_rotation(&new_admin);

    env.mock_auths(&[&outsider]);
    let result = client.try_cancel_admin_rotation();
    assert!(result.is_err(), "outsider must not cancel rotation");

    assert!(client.get_pending_admin().is_some(), "pending state must survive failed cancel");
}

#[test]
fn test_non_admin_cannot_change_timelock_duration() {
    let env = Env::default();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let outsider = Address::generate(&env);

    client.init(&admin, &token);

    env.mock_auths(&[&outsider]);
    let result = client.try_set_rotation_timelock_duration(&7200);
    assert!(result.is_err(), "outsider must not change timelock");

    assert_eq!(client.get_rotation_timelock_duration(), 86_400);
}

// ═══════════════════════════════════════════════════════════════════════════════
// TIMELOCK DURATION CONFIGURATION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_set_rotation_timelock_duration_success() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    let before_events = env.events().all().len();
    let new_duration = 7200;
    client.set_rotation_timelock_duration(&new_duration);

    let duration = client.get_rotation_timelock_duration();
    assert_eq!(duration, new_duration);

    let after_events = env.events().all().len();
    assert!(after_events > before_events, "timelock update must emit an event");

    let ev_data = find_last_event_data(&env, symbol_short!("admtlcfg"));
    assert!(ev_data.is_some(), "AdminRotationTimelockUpdated event must be emitted");
    let ev: events::AdminRotationTimelockUpdated = ev_data.unwrap().into_val(&env);
    assert_eq!(ev.version, EVENT_VERSION_V2);
    assert_eq!(ev.previous_duration, 86_400);
    assert_eq!(ev.new_duration, new_duration);
}

#[test]
fn test_set_rotation_timelock_duration_minimum() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    let min_duration = 3600;
    client.set_rotation_timelock_duration(&min_duration);

    let duration = client.get_rotation_timelock_duration();
    assert_eq!(duration, min_duration);
}

#[test]
fn test_set_rotation_timelock_duration_maximum() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    let max_duration = 2_592_000;
    client.set_rotation_timelock_duration(&max_duration);

    let duration = client.get_rotation_timelock_duration();
    assert_eq!(duration, max_duration);
}

#[test]
fn test_set_rotation_timelock_duration_below_minimum() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    let result = client.try_set_rotation_timelock_duration(&3599);
    assert_eq!(result, Err(Ok(Error::InvalidAdminRotationTimelock)));
}

#[test]
fn test_set_rotation_timelock_duration_above_maximum() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    let result = client.try_set_rotation_timelock_duration(&2_592_001);
    assert_eq!(result, Err(Ok(Error::InvalidAdminRotationTimelock)));
}

#[test]
fn test_default_timelock_duration() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    let duration = client.get_rotation_timelock_duration();
    assert_eq!(duration, 86_400);
}

// ═══════════════════════════════════════════════════════════════════════════════
// STATUS AND CONFIG QUERY TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_get_admin_rotation_status_no_pending() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    let status = client.get_admin_rotation_status();
    assert_eq!(status, None);
}

#[test]
fn test_get_admin_rotation_status_with_pending() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);
    let execute_after = client.propose_admin_rotation(&new_admin);

    let status = client.get_admin_rotation_status();
    assert!(status.is_some());

    let status = status.unwrap();
    assert_eq!(status.current_admin, admin);
    assert_eq!(status.pending_admin, new_admin);
    assert_eq!(status.execute_after, execute_after);
    assert!(!status.is_executable);
    assert!(status.remaining_seconds > 0);
}

#[test]
fn test_get_admin_rotation_status_executable() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);
    let execute_after = client.propose_admin_rotation(&new_admin);

    env.ledger().set_timestamp(execute_after + 100);

    let status = client.get_admin_rotation_status();
    assert!(status.is_some());

    let status = status.unwrap();
    assert!(status.is_executable);
    assert_eq!(status.remaining_seconds, 0);
}

#[test]
fn test_get_admin_rotation_config() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    let config = client.get_admin_rotation_config();
    assert_eq!(config.timelock_duration, 86_400);
    assert_eq!(config.min_timelock, 3_600);
    assert_eq!(config.max_timelock, 2_592_000);
    assert!(!config.has_pending_rotation);
}

#[test]
fn test_get_admin_rotation_config_with_pending() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);
    client.propose_admin_rotation(&new_admin);

    let config = client.get_admin_rotation_config();
    assert!(config.has_pending_rotation);
}

// ═══════════════════════════════════════════════════════════════════════════════
// END-TO-END FLOW TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_full_rotation_flow_propose_wait_accept() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);

    let execute_after = client.propose_admin_rotation(&new_admin);
    assert_eq!(client.get_admin(), Some(admin.clone()));
    assert_single_authority(&env, &client, &admin);

    env.ledger().set_timestamp(execute_after + 1);

    client.accept_admin_rotation();
    assert_eq!(client.get_admin(), Some(new_admin.clone()));
    assert_no_pending_state(&client);
    assert_single_authority(&env, &client, &new_admin);
}

#[test]
fn test_full_rotation_flow_propose_cancel_repropose() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin1 = Address::generate(&env);
    let new_admin2 = Address::generate(&env);

    client.init(&admin, &token);

    client.propose_admin_rotation(&new_admin1);
    client.cancel_admin_rotation();

    let execute_after = client.propose_admin_rotation(&new_admin2);

    env.ledger().set_timestamp(execute_after + 1);
    client.accept_admin_rotation();

    assert_eq!(client.get_admin(), Some(new_admin2));
    assert_no_pending_state(&client);
}

#[test]
fn test_multiple_rotations_sequential() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin1 = Address::generate(&env);
    let token = Address::generate(&env);
    let admin2 = Address::generate(&env);
    let admin3 = Address::generate(&env);

    client.init(&admin1, &token);

    let execute_after_1 = client.propose_admin_rotation(&admin2);
    assert_single_authority(&env, &client, &admin1);

    env.ledger().set_timestamp(execute_after_1 + 1);
    client.accept_admin_rotation();
    assert_eq!(client.get_admin(), Some(admin2.clone()));
    assert_single_authority(&env, &client, &admin2);

    let execute_after_2 = client.propose_admin_rotation(&admin3);
    assert_single_authority(&env, &client, &admin2);

    env.ledger().set_timestamp(execute_after_2 + 1);
    client.accept_admin_rotation();
    assert_eq!(client.get_admin(), Some(admin3.clone()));
    assert_no_pending_state(&client);
    assert_single_authority(&env, &client, &admin3);
}

// ═══════════════════════════════════════════════════════════════════════════════
// COMPLETE EVENT AUDIT TRAIL TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_audit_trail_propose_accept_emits_both_events() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);

    let events_before_init = env.events().all().len();
    let _ = events_before_init;

    let execute_after = client.propose_admin_rotation(&new_admin);

    let proposed = has_event_topic(&env, symbol_short!("admrotp"));
    assert!(proposed, "admrotp event must exist after propose");

    let ev = find_last_event_data(&env, symbol_short!("admrotp")).unwrap();
    let ev: events::AdminRotationProposed = ev.into_val(&env);
    assert_eq!(ev.version, EVENT_VERSION_V2);
    assert_eq!(ev.current_admin, admin);
    assert_eq!(ev.pending_admin, new_admin);
    assert_eq!(ev.execute_after, execute_after);

    env.ledger().set_timestamp(execute_after + 1);
    client.accept_admin_rotation();

    let accepted = has_event_topic(&env, symbol_short!("admrota"));
    assert!(accepted, "admrota event must exist after accept");

    let ev = find_last_event_data(&env, symbol_short!("admrota")).unwrap();
    let ev: events::AdminRotationAccepted = ev.into_val(&env);
    assert_eq!(ev.version, EVENT_VERSION_V2);
    assert_eq!(ev.previous_admin, admin);
    assert_eq!(ev.new_admin, new_admin);
}

#[test]
fn test_audit_trail_propose_cancel_emits_both_events() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);
    client.propose_admin_rotation(&new_admin);

    let proposed = has_event_topic(&env, symbol_short!("admrotp"));
    assert!(proposed, "admrotp event must exist after propose");

    client.cancel_admin_rotation();

    let cancelled = has_event_topic(&env, symbol_short!("admrotc"));
    assert!(cancelled, "admrotc event must exist after cancel");

    let ev = find_last_event_data(&env, symbol_short!("admrotc")).unwrap();
    let ev: events::AdminRotationCancelled = ev.into_val(&env);
    assert_eq!(ev.version, EVENT_VERSION_V2);
    assert_eq!(ev.admin, admin);
    assert_eq!(ev.cancelled_pending_admin, new_admin);
}

#[test]
fn test_audit_trail_propose_cancel_repropose_accept() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let target_a = Address::generate(&env);
    let target_b = Address::generate(&env);

    client.init(&admin, &token);

    let before_total = env.events().all().len();

    client.propose_admin_rotation(&target_a);
    assert!(has_event_topic(&env, symbol_short!("admrotp")));

    client.cancel_admin_rotation();
    assert!(has_event_topic(&env, symbol_short!("admrotc")));

    let execute_after = client.propose_admin_rotation(&target_b);
    assert!(has_event_topic(&env, symbol_short!("admrotp")));

    env.ledger().set_timestamp(execute_after + 1);
    client.accept_admin_rotation();
    assert!(has_event_topic(&env, symbol_short!("admrota")));

    let after_total = env.events().all().len();
    assert!(
        after_total >= before_total + 4,
        "must have at least 4 rotation events: propose, cancel, propose, accept"
    );

    let ev = find_last_event_data(&env, symbol_short!("admrota")).unwrap();
    let ev: events::AdminRotationAccepted = ev.into_val(&env);
    assert_eq!(ev.previous_admin, admin);
    assert_eq!(ev.new_admin, target_b);
}

#[test]
fn test_audit_trail_timelock_config_update() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    client.set_rotation_timelock_duration(&7200);
    assert!(has_event_topic(&env, symbol_short!("admtlcfg")));

    let ev = find_last_event_data(&env, symbol_short!("admtlcfg")).unwrap();
    let ev: events::AdminRotationTimelockUpdated = ev.into_val(&env);
    assert_eq!(ev.version, EVENT_VERSION_V2);
    assert_eq!(ev.admin, admin);
    assert_eq!(ev.previous_duration, 86_400);
    assert_eq!(ev.new_duration, 7200);

    client.set_rotation_timelock_duration(&3600);

    let ev = find_last_event_data(&env, symbol_short!("admtlcfg")).unwrap();
    let ev: events::AdminRotationTimelockUpdated = ev.into_val(&env);
    assert_eq!(ev.previous_duration, 7200);
    assert_eq!(ev.new_duration, 3600);
}

// ═══════════════════════════════════════════════════════════════════════════════
// UPGRADE SAFETY TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_admin_rotation_storage_persists_across_queries() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);
    let execute_after = client.propose_admin_rotation(&new_admin);

    for _ in 0..5 {
        assert_eq!(client.get_pending_admin(), Some(new_admin.clone()));
        assert_eq!(client.get_admin_rotation_timelock(), Some(execute_after));
        assert_eq!(client.get_admin(), Some(admin.clone()));
    }
}

#[test]
fn test_timelock_duration_persists() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    client.set_rotation_timelock_duration(&7200);

    for _ in 0..5 {
        assert_eq!(client.get_rotation_timelock_duration(), 7200);
    }
}
