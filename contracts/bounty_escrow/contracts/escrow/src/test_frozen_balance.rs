#![cfg(test)]

use super::*;
use soroban_sdk::testutils::Events;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

// ── Test helpers ─────────────────────────────────────────────────────────────

fn setup(env: &Env) -> (Address, Address, token::Client, BountyEscrowContractClient) {
    let admin = Address::generate(env);
    let token_admin = Address::generate(env);

    let token_contract = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_client = token::Client::new(env, &token_contract);
    let token_admin_client = token::StellarAssetClient::new(env, &token_contract);

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(env, &contract_id);
    client.init(&admin, &token_contract);

    let depositor = Address::generate(env);
    token_admin_client.mint(&depositor, &10_000);

    (admin, depositor, token_client, client)
}

fn lock(
    env: &Env,
    client: &BountyEscrowContractClient,
    depositor: &Address,
    bounty_id: u64,
    amount: i128,
) {
    let deadline = env.ledger().timestamp() + 5000;
    client.lock_funds(depositor, &bounty_id, &amount, &deadline);
}

// ── Escrow-level freeze ───────────────────────────────────────────────────────

#[test]
fn test_freeze_escrow_blocks_release() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, depositor, _token, client) = setup(&env);

    lock(&env, &client, &depositor, 1, 500);

    let reason = soroban_sdk::String::from_str(&env, "regulatory hold");
    client.freeze_escrow(&1u64, &reason);

    let contributor = Address::generate(&env);
    let result = client.try_release_funds(&1u64, &contributor);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), Error::EscrowFrozen);
}

#[test]
fn test_freeze_escrow_blocks_refund() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, depositor, _token, client) = setup(&env);

    lock(&env, &client, &depositor, 1, 500);
    env.ledger().set_timestamp(env.ledger().timestamp() + 6000);

    let reason = soroban_sdk::String::from_str(&env, "under investigation");
    client.freeze_escrow(&1u64, &reason);

    let result = client.try_refund(&1u64);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), Error::EscrowFrozen);
}

#[test]
fn test_freeze_escrow_allows_read_access() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, depositor, _token, client) = setup(&env);

    lock(&env, &client, &depositor, 1, 500);

    let reason = soroban_sdk::String::from_str(&env, "hold");
    client.freeze_escrow(&1u64, &reason);

    // Read-only queries must still work
    let info = client.get_escrow_info(&1u64);
    assert_eq!(info.amount, 500);
    assert_eq!(info.status, EscrowStatus::Locked);
}

#[test]
fn test_unfreeze_escrow_allows_release() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, depositor, token_client, client) = setup(&env);

    lock(&env, &client, &depositor, 1, 500);

    let reason = soroban_sdk::String::from_str(&env, "temporary hold");
    client.freeze_escrow(&1u64, &reason);

    // Confirm it is blocked
    let contributor = Address::generate(&env);
    assert!(client.try_release_funds(&1u64, &contributor).is_err());

    client.unfreeze_escrow(&1u64);

    // Now release should succeed
    client.release_funds(&1u64, &contributor);
    assert_eq!(token_client.balance(&contributor), 500);
}

#[test]
fn test_unfreeze_escrow_allows_refund() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, depositor, token_client, client) = setup(&env);

    lock(&env, &client, &depositor, 1, 500);
    env.ledger().set_timestamp(env.ledger().timestamp() + 6000);

    let reason = soroban_sdk::String::from_str(&env, "AML review");
    client.freeze_escrow(&1u64, &reason);
    assert!(client.try_refund(&1u64).is_err());

    client.unfreeze_escrow(&1u64);
    client.refund(&1u64);

    assert_eq!(token_client.balance(&depositor), 10000); // 10000 - 500 locked + 500 refunded
}

#[test]
fn test_freeze_one_escrow_does_not_affect_another() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, depositor, token_client, client) = setup(&env);

    lock(&env, &client, &depositor, 1, 300);
    lock(&env, &client, &depositor, 2, 200);

    let reason = soroban_sdk::String::from_str(&env, "hold on bounty 1 only");
    client.freeze_escrow(&1u64, &reason);

    // Bounty 2 is not frozen — release should work
    let contributor = Address::generate(&env);
    client.release_funds(&2u64, &contributor);
    assert_eq!(token_client.balance(&contributor), 200);

    // Bounty 1 is still frozen
    assert!(client.try_release_funds(&1u64, &contributor).is_err());
}

#[test]
fn test_freeze_escrow_blocks_partial_release() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, depositor, _token, client) = setup(&env);

    lock(&env, &client, &depositor, 1, 1000);

    let reason = soroban_sdk::String::from_str(&env, "hold");
    client.freeze_escrow(&1u64, &reason);

    let contributor = Address::generate(&env);
    let result = client.try_partial_release(&1u64, &contributor, &300i128);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), Error::EscrowFrozen);
}

#[test]
fn test_freeze_escrow_blocks_batch_release() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, depositor, _token, client) = setup(&env);

    lock(&env, &client, &depositor, 1, 500);
    lock(&env, &client, &depositor, 2, 500);

    let reason = soroban_sdk::String::from_str(&env, "batch freeze test");
    client.freeze_escrow(&1u64, &reason);

    let contributor = Address::generate(&env);
    let items = soroban_sdk::vec![
        &env,
        ReleaseFundsItem {
            bounty_id: 1,
            contributor: contributor.clone()
        },
        ReleaseFundsItem {
            bounty_id: 2,
            contributor: contributor.clone()
        },
    ];
    let result = client.try_batch_release_funds(&items);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), Error::EscrowFrozen);

    // Neither should have been released (atomicity)
    assert_eq!(client.get_escrow_info(&1u64).status, EscrowStatus::Locked);
    assert_eq!(client.get_escrow_info(&2u64).status, EscrowStatus::Locked);
}

#[test]
fn test_get_escrow_freeze_record_returns_correct_data() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, depositor, _token, client) = setup(&env);

    lock(&env, &client, &depositor, 1, 500);

    // Before freeze
    assert!(client.get_escrow_freeze_record(&1u64).is_none());

    let reason = soroban_sdk::String::from_str(&env, "KYC freeze");
    client.freeze_escrow(&1u64, &reason);

    let record = client.get_escrow_freeze_record(&1u64).unwrap();
    assert!(record.frozen);
    assert_eq!(record.reason, reason);

    client.unfreeze_escrow(&1u64);
    assert!(client.get_escrow_freeze_record(&1u64).is_none());
}

#[test]
fn test_non_admin_cannot_freeze_escrow() {
    let env = Env::default();
    env.mock_all_auths();

    let (_admin, depositor, _token, client) = setup(&env);
    lock(&env, &client, &depositor, 1, 500);

    // Switch to only authorizing a random non-admin address —
    // the contract's admin.require_auth() will now find no matching auth
    // and escalate to a panic (which try_freeze_escrow catches as Err).
    let non_admin = Address::generate(&env);
    env.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &non_admin,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &client.address,
            fn_name: "freeze_escrow",
            args: soroban_sdk::vec![
                &env,
                soroban_sdk::IntoVal::into_val(&1u64, &env),
                soroban_sdk::IntoVal::into_val(
                    &soroban_sdk::String::from_str(&env, "unauthorized freeze"),
                    &env,
                ),
            ]
            .into(),
            sub_invokes: &[],
        },
    }]);

    let reason = soroban_sdk::String::from_str(&env, "unauthorized freeze");
    let result = client.try_freeze_escrow(&1u64, &reason);
    assert!(
        result.is_err(),
        "Non-admin should not be able to freeze escrow"
    );
}

#[test]
fn test_freeze_address_blocks_refund() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, depositor, _token, client) = setup(&env);

    lock(&env, &client, &depositor, 10, 400);
    env.ledger().set_timestamp(env.ledger().timestamp() + 6000);

    let reason = soroban_sdk::String::from_str(&env, "sanctions");
    client.freeze_address(&depositor, &reason);

    assert_eq!(
        client.try_refund(&10u64).unwrap_err().unwrap(),
        Error::AddressFrozen
    );
}

#[test]
fn test_unfreeze_address_restores_operations() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, depositor, token_client, client) = setup(&env);

    lock(&env, &client, &depositor, 10, 400);

    let reason = soroban_sdk::String::from_str(&env, "hold");
    client.freeze_address(&depositor, &reason);

    let contributor = Address::generate(&env);
    assert!(client.try_release_funds(&10u64, &contributor).is_err());

    client.unfreeze_address(&depositor);
    client.release_funds(&10u64, &contributor);
    assert_eq!(token_client.balance(&contributor), 400);
}

#[test]
fn test_freeze_address_allows_read_queries() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, depositor, _token, client) = setup(&env);

    lock(&env, &client, &depositor, 10, 400);

    let reason = soroban_sdk::String::from_str(&env, "AML");
    client.freeze_address(&depositor, &reason);

    let info = client.get_escrow_info(&10u64);
    assert_eq!(info.status, EscrowStatus::Locked);

    let balance = client.get_balance();
    assert_eq!(balance, 400);
}

#[test]
fn test_freeze_address_does_not_affect_different_depositor() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, depositor1, token_client, client) = setup(&env);

    // Create a second depositor
    let depositor2 = Address::generate(&env);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_client.address);
    token_admin_client.mint(&depositor2, &1000);

    lock(&env, &client, &depositor1, 20, 300);
    lock(&env, &client, &depositor2, 21, 300);

    let reason = soroban_sdk::String::from_str(&env, "freeze depositor1 only");
    client.freeze_address(&depositor1, &reason);

    // depositor2's escrow is unaffected
    let contributor = Address::generate(&env);
    client.release_funds(&21u64, &contributor);
    assert_eq!(token_client.balance(&contributor), 300);

    // depositor1's escrow is blocked
    assert!(client.try_release_funds(&20u64, &contributor).is_err());
}

#[test]
fn test_get_address_freeze_record() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, depositor, _token, client) = setup(&env);

    assert!(client.get_address_freeze_record(&depositor).is_none());

    let reason = soroban_sdk::String::from_str(&env, "OFAC");
    client.freeze_address(&depositor, &reason);

    let record = client.get_address_freeze_record(&depositor).unwrap();
    assert!(record.frozen);
    assert_eq!(record.reason, reason);

    client.unfreeze_address(&depositor);
    assert!(client.get_address_freeze_record(&depositor).is_none());
}

// ── Freeze does NOT block lock (issue spec: freeze = outflow only) ─────────────

#[test]
fn test_freeze_escrow_does_not_block_new_lock_on_different_id() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, depositor, _token, client) = setup(&env);

    lock(&env, &client, &depositor, 1, 500);

    let reason = soroban_sdk::String::from_str(&env, "hold");
    client.freeze_escrow(&1u64, &reason);

    // A different bounty can still be locked — freeze is per-escrow, not global
    lock(&env, &client, &depositor, 2, 200);
    let info = client.get_escrow_info(&2u64);
    assert_eq!(info.status, EscrowStatus::Locked);
}

// ── Event emission ────────────────────────────────────────────────────────────

#[test]
fn test_freeze_escrow_emits_event() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, depositor, _token, client) = setup(&env);

    lock(&env, &client, &depositor, 1, 500);

    let reason = soroban_sdk::String::from_str(&env, "audit");
    client.freeze_escrow(&1u64, &reason);

    let events = env.events().all();
    let last = events.last().unwrap();
    let topic0: soroban_sdk::Symbol = soroban_sdk::IntoVal::into_val(&last.1.get(0).unwrap(), &env);
    assert_eq!(topic0, soroban_sdk::Symbol::new(&env, "frz"));
}

#[test]
fn test_unfreeze_escrow_emits_event() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, depositor, _token, client) = setup(&env);

    lock(&env, &client, &depositor, 1, 500);
    let reason = soroban_sdk::String::from_str(&env, "audit");
    client.freeze_escrow(&1u64, &reason);
    client.unfreeze_escrow(&1u64);

    let events = env.events().all();
    let last = events.last().unwrap();
    let topic0: soroban_sdk::Symbol = soroban_sdk::IntoVal::into_val(&last.1.get(0).unwrap(), &env);
    assert_eq!(topic0, soroban_sdk::Symbol::new(&env, "unfrz"));
}
