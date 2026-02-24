#![cfg(test)]

use super::*;
use crate::PauseStateChanged;
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token, Address, Env, IntoVal, Symbol, TryIntoVal,
};

fn create_token_contract<'a>(
    e: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let contract_address = e
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    (
        token::Client::new(e, &contract_address),
        token::StellarAssetClient::new(e, &contract_address),
    )
}

fn create_escrow_contract<'a>(e: &Env) -> (BountyEscrowContractClient<'a>, Address) {
    let contract_id = e.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(e, &contract_id);
    (client, contract_id)
}

#[test]
fn test_granular_pause_lock() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let (token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    let (escrow_client, _escrow_address) = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);

    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.lock_paused, false);
    assert_eq!(flags.release_paused, false);
    assert_eq!(flags.refund_paused, false);

    token_admin_client.mint(&depositor, &1000);

    let bounty_id_1: u64 = 1;
    let deadline = env.ledger().timestamp() + 1000;
    escrow_client.lock_funds(&depositor, &bounty_id_1, &100, &deadline);

    escrow_client.set_paused(&Some(true), &None, &None, &None);
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.lock_paused, true);

    let bounty_id_2: u64 = 2;
    let res = escrow_client.try_lock_funds(&depositor, &bounty_id_2, &100, &deadline);
    assert!(res.is_err());

    escrow_client.set_paused(&Some(false), &None, &None, &None);
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.lock_paused, false);

    escrow_client.lock_funds(&depositor, &bounty_id_2, &100, &deadline);
}

#[test]
fn test_granular_pause_release() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let (token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    let (escrow_client, _) = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);
    token_admin_client.mint(&depositor, &1000);

    let bounty_id: u64 = 1;
    let deadline = env.ledger().timestamp() + 1000;
    escrow_client.lock_funds(&depositor, &bounty_id, &100, &deadline);

    escrow_client.set_paused(&None, &Some(true), &None, &None);
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.release_paused, true);

    let res = escrow_client.try_release_funds(&bounty_id, &contributor);
    assert!(res.is_err());

    escrow_client.set_paused(&None, &Some(false), &None, &None);
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.release_paused, false);

    escrow_client.release_funds(&bounty_id, &contributor);
}

#[test]
fn test_granular_pause_refund() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let (token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    let (escrow_client, _) = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);
    token_admin_client.mint(&depositor, &1000);

    let bounty_id: u64 = 1;
    let deadline = env.ledger().timestamp() + 1000;

    escrow_client.lock_funds(&depositor, &bounty_id, &100, &deadline);

    env.ledger().set_timestamp(deadline + 1);

    escrow_client.set_paused(&None, &None, &Some(true), &None);
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.refund_paused, true);

    let res = escrow_client.try_refund(&bounty_id);
    assert!(res.is_err());

    escrow_client.set_paused(&None, &None, &Some(false), &None);
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.refund_paused, false);

    escrow_client.refund(&bounty_id);
}

#[test]
fn test_mixed_pause_states() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (token_client, _) = create_token_contract(&env, &admin);
    let (escrow_client, _) = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);

    escrow_client.set_paused(&Some(true), &Some(true), &Some(false), &None);
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.lock_paused, true);
    assert_eq!(flags.release_paused, true);
    assert_eq!(flags.refund_paused, false);

    escrow_client.set_paused(&None, &Some(false), &None, &None);
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.lock_paused, true);
    assert_eq!(flags.release_paused, false);
    assert_eq!(flags.refund_paused, false);
}

// =========================================================================
// NEW NEGATIVE TESTS & EVENT EMISSIONS (Added for PR 353)
// =========================================================================

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn test_pause_by_non_admin_fails() {
    let env = Env::default();
    // Do NOT mock_all_auths — we want admin.require_auth() to fail

    let admin = Address::generate(&env);
    let (token_client, _) = create_token_contract(&env, &admin);
    let (escrow_client, _escrow_id) = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);

    // Try to pause without providing admin auth — should panic
    escrow_client.set_paused(&Some(true), &Some(true), &Some(true), &None);
}

#[test]
fn test_set_paused_emits_events() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (token_client, _) = create_token_contract(&env, &admin);
    let (escrow_client, escrow_id) = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);

    // Pause lock
    escrow_client.set_paused(&Some(true), &None, &None, &None);

    let events = env.events().all();
    let emitted = events.iter().last().unwrap();
    assert_eq!(emitted.0, escrow_id);
    let topics = emitted.1;
    let topic_0: Symbol = topics.get(0).unwrap().into_val(&env);
    let topic_1: Symbol = topics.get(1).unwrap().into_val(&env);
    assert_eq!(topic_0, Symbol::new(&env, "pause"));
    assert_eq!(topic_1, Symbol::new(&env, "lock"));
    let data = emitted.2;
    // Data is a struct PauseStateChanged, we need to deserialize it properly
    let pause_state: PauseStateChanged = data.try_into_val(&env).unwrap();
    assert_eq!(pause_state.paused, true);
    assert_eq!(pause_state.admin, admin);
}

#[test]
fn test_batch_lock_funds_while_paused_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let (token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    let (escrow_client, _) = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);
    token_admin_client.mint(&depositor, &1000);

    escrow_client.set_paused(&Some(true), &None, &None, &None);

    let deadline = env.ledger().timestamp() + 1000;
    let items = soroban_sdk::vec![
        &env,
        LockFundsItem {
            bounty_id: 1,
            amount: 100,
            depositor: depositor.clone(),
            deadline
        },
        LockFundsItem {
            bounty_id: 2,
            amount: 100,
            depositor: depositor.clone(),
            deadline
        }
    ];

    let res = escrow_client.try_batch_lock_funds(&items);
    assert!(res.is_err());
}

#[test]
fn test_batch_release_funds_while_paused_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let (token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    let (escrow_client, _) = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);
    token_admin_client.mint(&depositor, &1000);

    let deadline = env.ledger().timestamp() + 1000;
    escrow_client.lock_funds(&depositor, &1u64, &100, &deadline);

    // Pause release
    escrow_client.set_paused(&None, &Some(true), &None, &None);

    let items = soroban_sdk::vec![
        &env,
        ReleaseFundsItem {
            bounty_id: 1,
            contributor: contributor.clone()
        }
    ];

    let res = escrow_client.try_batch_release_funds(&items);
    assert!(res.is_err());
}

#[test]
fn test_operations_resume_after_unpause() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let (token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    let (escrow_client, _) = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);
    token_admin_client.mint(&depositor, &1000);

    // Pause everything
    escrow_client.set_paused(&Some(true), &Some(true), &Some(true), &None);

    let deadline = env.ledger().timestamp() + 1000;
    let res_lock = escrow_client.try_lock_funds(&depositor, &1u64, &100, &deadline);
    assert!(res_lock.is_err());

    // Unpause lock
    escrow_client.set_paused(&Some(false), &None, &None, &None);

    // Now it works
    escrow_client.lock_funds(&depositor, &1u64, &100, &deadline);

    // Release still paused though
    let contributor = Address::generate(&env);
    let res_release = escrow_client.try_release_funds(&1u64, &contributor);
    assert!(res_release.is_err());

    // Unpause release
    escrow_client.set_paused(&None, &Some(false), &None, &None);

    // Now release works
    escrow_client.release_funds(&1u64, &contributor);
}

#[test]
fn test_lock_funds_while_paused_no_state_change() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let (token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    let (escrow_client, _) = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);
    token_admin_client.mint(&depositor, &1000);

    escrow_client.set_paused(&Some(true), &None, &None, &None);

    let deadline = env.ledger().timestamp() + 1000;
    let _ = escrow_client.try_lock_funds(&depositor, &1u64, &100, &deadline);

    // Verify token balance didn't change and escrow wasn't created
    assert_eq!(token_client.balance(&depositor), 1000);
    assert!(escrow_client.try_get_escrow_info(&1u64).is_err());
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn test_emergency_withdraw_non_admin_fails() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let (token_client, _) = create_token_contract(&env, &admin);
    let (escrow_client, _) = create_escrow_contract(&env);

    let target = Address::generate(&env);
    escrow_client.init(&admin, &token_client.address);
    escrow_client.emergency_withdraw(&target);
}

#[test]
#[should_panic(expected = "Error(Contract, #21)")]
fn test_emergency_withdraw_unpaused_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let (token_client, _) = create_token_contract(&env, &admin);
    let (escrow_client, _) = create_escrow_contract(&env);
    let target = Address::generate(&env);

    escrow_client.init(&admin, &token_client.address);
    escrow_client.emergency_withdraw(&target);
}

#[test]
fn test_emergency_withdraw_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token_client, token_admin_client) = create_token_contract(&env, &admin);
    let (escrow_client, _) = create_escrow_contract(&env);
    let target = Address::generate(&env);

    escrow_client.init(&admin, &token_client.address);
    token_admin_client.mint(&depositor, &1000);

    let deadline = env.ledger().timestamp() + 1000;
    escrow_client.lock_funds(&depositor, &1u64, &500i128, &deadline);

    assert_eq!(token_client.balance(&escrow_client.address), 500);

    let reason = soroban_sdk::String::from_str(&env, "Hacked");
    escrow_client.set_paused(&Some(true), &None, &None, &Some(reason));

    escrow_client.emergency_withdraw(&target);

    assert_eq!(token_client.balance(&escrow_client.address), 0);
    assert_eq!(token_client.balance(&target), 500);
}

// =========================================================================
// RBAC + EMERGENCY WITHDRAW TESTS (Issue #389)
// =========================================================================

/// Helper: sets up env, admin, operator, token, and escrow contract.
/// Returns (env, admin, operator, token_client, escrow_client)
fn setup_rbac_env<'a>(
    env: &'a Env,
) -> (
    Address,
    Address,
    token::Client<'a>,
    BountyEscrowContractClient<'a>,
) {
    let admin = Address::generate(env);
    let operator = Address::generate(env);
    let token_admin = Address::generate(env);

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let escrow_client = BountyEscrowContractClient::new(env, &contract_id);

    let token_contract = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_client = token::Client::new(env, &token_contract);
    let token_admin_client = token::StellarAssetClient::new(env, &token_contract);

    escrow_client.init(&admin, &token_client.address);

    let depositor = Address::generate(env);
    token_admin_client.mint(&depositor, &1000);
    let deadline = env.ledger().timestamp() + 1000;
    escrow_client.lock_funds(&depositor, &1u64, &500i128, &deadline);

    (admin, operator, token_client, escrow_client)
}

/// Admin CAN perform emergency_withdraw when contract is paused.
#[test]
fn test_rbac_admin_can_emergency_withdraw_when_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let (_admin, _, token_client, escrow_client) = setup_rbac_env(&env);
    let target = Address::generate(&env);

    escrow_client.set_paused(&Some(true), &None, &None, &None);

    assert_eq!(token_client.balance(&escrow_client.address), 500);

    escrow_client.emergency_withdraw(&target);

    assert_eq!(token_client.balance(&escrow_client.address), 0);
    assert_eq!(token_client.balance(&target), 500);
}

/// Operator/non-admin role CANNOT perform emergency_withdraw — auth rejected.
#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn test_rbac_operator_cannot_emergency_withdraw() {
    let env = Env::default();

    let (_, _operator, _token_client, escrow_client) = setup_rbac_env(&env);
    let target = Address::generate(&env);

    escrow_client.set_paused(&Some(true), &None, &None, &None);
    escrow_client.emergency_withdraw(&target);
}

/// emergency_withdraw FAILS even for admin when contract is NOT paused.
#[test]
#[should_panic(expected = "Error(Contract, #21)")]
fn test_rbac_admin_emergency_withdraw_requires_paused_state() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, _, escrow_client) = setup_rbac_env(&env);
    let target = Address::generate(&env);

    escrow_client.emergency_withdraw(&target);
}

/// emergency_withdraw emits the correct event with admin address and amount.
#[test]
fn test_rbac_emergency_withdraw_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (admin, _, _token_client, escrow_client) = setup_rbac_env(&env);
    let target = Address::generate(&env);

    escrow_client.set_paused(&Some(true), &None, &None, &None);
    escrow_client.emergency_withdraw(&target);

    let all_events = env.events().all();
    let last_event = all_events.last().unwrap();

    assert_eq!(
        vec![&env, last_event],
        vec![
            &env,
            (
                escrow_client.address.clone(),
                (symbol_short!("em_wtd"),).into_val(&env),
                events::EmergencyWithdrawEvent {
                    admin: admin.clone(),
                    recipient: target.clone(),
                    amount: 500i128,
                    timestamp: env.ledger().timestamp(),
                }
                .into_val(&env)
            ),
        ]
    );
}

/// Drain is idempotent: second emergency_withdraw on empty contract does nothing (no panic).
#[test]
fn test_rbac_emergency_withdraw_on_empty_contract_is_safe() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, token_client, escrow_client) = setup_rbac_env(&env);
    let target = Address::generate(&env);

    escrow_client.set_paused(&Some(true), &None, &None, &None);
    escrow_client.emergency_withdraw(&target); // drains 500
    escrow_client.emergency_withdraw(&target); // balance = 0, should NOT panic

    assert_eq!(token_client.balance(&escrow_client.address), 0);
}

/// Paused state is preserved after a successful emergency_withdraw.
#[test]
fn test_rbac_pause_state_preserved_after_emergency_withdraw() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, _, escrow_client) = setup_rbac_env(&env);
    let target = Address::generate(&env);

    escrow_client.set_paused(&Some(true), &None, &None, &None);
    escrow_client.emergency_withdraw(&target);

    let depositor = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 2000;
    let res = escrow_client.try_lock_funds(&depositor, &99u64, &100i128, &deadline);
    assert!(res.is_err(), "lock should still be paused after withdraw");
}

/// Partial pause: only lock paused, release still works — emergency_withdraw still requires lock_paused.
#[test]
#[should_panic(expected = "Error(Contract, #21)")]
fn test_rbac_emergency_withdraw_requires_lock_paused_not_release_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, _, escrow_client) = setup_rbac_env(&env);
    let target = Address::generate(&env);

    escrow_client.set_paused(&None, &Some(true), &None, &None);
    escrow_client.emergency_withdraw(&target);
}

/// Partial pause: only refund paused — emergency_withdraw still requires lock_paused.
#[test]
#[should_panic(expected = "Error(Contract, #21)")]
fn test_rbac_emergency_withdraw_requires_lock_paused_not_refund_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, _, escrow_client) = setup_rbac_env(&env);
    let target = Address::generate(&env);

    escrow_client.set_paused(&None, &None, &Some(true), &None);
    escrow_client.emergency_withdraw(&target);
}

/// Admin withdraws correct amount when multiple bounties are locked.
#[test]
fn test_rbac_emergency_withdraw_drains_all_bounties() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let escrow_client = BountyEscrowContractClient::new(&env, &contract_id);
    let token_contract = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_client = token::Client::new(&env, &token_contract);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract);

    escrow_client.init(&admin, &token_client.address);
    token_admin_client.mint(&depositor, &3000);

    let deadline = env.ledger().timestamp() + 1000;

    escrow_client.lock_funds(&depositor, &1u64, &500i128, &deadline);
    escrow_client.lock_funds(&depositor, &2u64, &700i128, &deadline);
    escrow_client.lock_funds(&depositor, &3u64, &300i128, &deadline);

    assert_eq!(token_client.balance(&escrow_client.address), 1500);

    let target = Address::generate(&env);
    escrow_client.set_paused(&Some(true), &None, &None, &None);
    escrow_client.emergency_withdraw(&target);

    assert_eq!(token_client.balance(&escrow_client.address), 0);
    assert_eq!(token_client.balance(&target), 1500);
}

/// After emergency_withdraw, admin can unpause and normal ops resume (but escrows are empty).
#[test]
fn test_rbac_after_emergency_withdraw_can_unpause_and_reuse() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, token_client, escrow_client) = setup_rbac_env(&env);
    let target = Address::generate(&env);

    escrow_client.set_paused(&Some(true), &None, &None, &None);
    escrow_client.emergency_withdraw(&target);

    escrow_client.set_paused(&Some(false), &None, &None, &None);
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.lock_paused, false);

    let new_depositor = Address::generate(&env);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_client.address);
    token_admin_client.mint(&new_depositor, &500);

    let deadline = env.ledger().timestamp() + 2000;
    escrow_client.lock_funds(&new_depositor, &99u64, &200i128, &deadline);
    assert_eq!(token_client.balance(&escrow_client.address), 200);
}
