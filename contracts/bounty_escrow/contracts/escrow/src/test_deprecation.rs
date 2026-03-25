//! Tests for controlled kill switch (deprecation) and migration path.
//! When deprecated is set: new lock_funds and batch_lock_funds are blocked;
//! existing escrows can still release, refund, or migrate.

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger as _},
    token, vec, Address, Env, Vec,
};

fn create_token_contract<'a>(
    e: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let contract = e.register_stellar_asset_contract_v2(admin.clone());
    let contract_address = contract.address();
    (
        token::Client::new(e, &contract_address),
        token::StellarAssetClient::new(e, &contract_address),
    )
}

fn default_setup<'a>(
    env: &Env,
) -> (
    BountyEscrowContractClient<'a>,
    Address,
    Address,
    token::Client<'a>,
    token::StellarAssetClient<'a>,
) {
    env.mock_all_auths();
    let admin = Address::generate(env);
    let depositor = Address::generate(env);
    let contributor = Address::generate(env);
    let (token, token_admin) = create_token_contract(env, &admin);
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let escrow = BountyEscrowContractClient::new(env, &contract_id);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);
    (escrow, admin, depositor, token, token_admin)
}

#[test]
fn test_get_deprecation_status_default_not_deprecated() {
    let env = Env::default();
    let (escrow, admin, _depositor, _token, _token_admin) = default_setup(&env);
    let status = escrow.get_deprecation_status();
    assert!(!status.deprecated);
    assert!(status.migration_target.is_none());
}

#[test]
fn test_set_deprecated_admin_only() {
    let env = Env::default();
    let (escrow, _admin, _depositor, _token, _token_admin) = default_setup(&env);
    escrow.set_deprecated(&true, &None);
    let status = escrow.get_deprecation_status();
    assert!(status.deprecated);
    assert!(status.migration_target.is_none());
}

#[test]
fn test_set_deprecated_with_migration_target() {
    let env = Env::default();
    let (escrow, _admin, _depositor, _token, _token_admin) = default_setup(&env);
    let migration = Address::generate(&env);
    escrow.set_deprecated(&true, &Some(migration.clone()));
    let status = escrow.get_deprecation_status();
    assert!(status.deprecated);
    assert_eq!(status.migration_target, Some(migration));
}

#[test]
#[should_panic(expected = "Error(Contract, #34)")]
fn test_lock_funds_fails_when_deprecated() {
    let env = Env::default();
    let (escrow, _admin, depositor, _token, _token_admin) = default_setup(&env);
    escrow.set_deprecated(&true, &None);

    let bounty_id = 1u64;
    let amount = 1000i128;
    let deadline = env.ledger().timestamp() + 1000;

    escrow.lock_funds(&depositor, &bounty_id, &amount, &deadline);
}

#[test]
#[should_panic(expected = "Error(Contract, #34)")]
fn test_batch_lock_funds_fails_when_deprecated() {
    let env = Env::default();
    let (escrow, _admin, depositor, _token, _token_admin) = default_setup(&env);
    escrow.set_deprecated(&true, &None);

    let mut items = Vec::new(&env);
    items.push_back(LockFundsItem {
        bounty_id: 1,
        depositor: depositor.clone(),
        amount: 500,
        deadline: env.ledger().timestamp() + 1000,
    });

    escrow.batch_lock_funds(&items);
}

#[test]
fn test_release_still_works_when_deprecated() {
    let env = Env::default();
    let (escrow, _admin, depositor, _token, _token_admin) = default_setup(&env);
    let contributor = Address::generate(&env);

    let bounty_id = 1u64;
    let amount = 1000i128;
    let deadline = env.ledger().timestamp() + 1000;
    escrow.lock_funds(&depositor, &bounty_id, &amount, &deadline);

    escrow.set_deprecated(&true, &None);

    escrow.release_funds(&bounty_id, &contributor);
    let info = escrow.get_escrow_info(&bounty_id);
    assert_eq!(info.status, EscrowStatus::Released);
}

#[test]
fn test_refund_still_works_when_deprecated() {
    let env = Env::default();
    env.ledger().set_timestamp(10_000); // ensure we can use a past deadline without underflow
    let (escrow, _admin, depositor, _token, _token_admin) = default_setup(&env);
    let bounty_id = 1u64;
    let amount = 1000i128;
    let deadline = 9_999u64; // already passed (now is 10_000)
    escrow.lock_funds(&depositor, &bounty_id, &amount, &deadline);

    escrow.set_deprecated(&true, &None);

    escrow.refund(&bounty_id);
    let info = escrow.get_escrow_info(&bounty_id);
    assert_eq!(info.status, EscrowStatus::Refunded);
}

#[test]
fn test_unset_deprecated_restores_lock() {
    let env = Env::default();
    let (escrow, _admin, depositor, _token, _token_admin) = default_setup(&env);
    escrow.set_deprecated(&true, &None);
    escrow.set_deprecated(&false, &None);

    let bounty_id = 1u64;
    let amount = 1000i128;
    let deadline = env.ledger().timestamp() + 1000;
    escrow.lock_funds(&depositor, &bounty_id, &amount, &deadline);
    let info = escrow.get_escrow_info(&bounty_id);
    assert_eq!(info.status, EscrowStatus::Locked);
}

#[test]
fn test_deprecation_event_emitted() {
    let env = Env::default();
    let (escrow, _admin, _depositor, _token, _token_admin) = default_setup(&env);
    use soroban_sdk::testutils::Events;
    let before = env.events().all().len();
    escrow.set_deprecated(&true, &None);
    let after = env.events().all().len();
    assert!(after > before, "deprecation event should be emitted");
}
