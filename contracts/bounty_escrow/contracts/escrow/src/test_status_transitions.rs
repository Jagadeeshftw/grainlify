use super::*;
use soroban_sdk::testutils::Ledger;
use soroban_sdk::{
    testutils::{Address as _, LedgerInfo},
    token, Address, Env,
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

fn create_escrow_contract<'a>(e: &Env) -> BountyEscrowContractClient<'a> {
    let contract_id = e.register_contract(None, BountyEscrowContract);
    BountyEscrowContractClient::new(e, &contract_id)
}

struct TestSetup<'a> {
    env: Env,
    #[allow(dead_code)]
    admin: Address,
    depositor: Address,
    contributor: Address,
    #[allow(dead_code)]
    token: token::Client<'a>,
    #[allow(dead_code)]
    token_admin: token::StellarAssetClient<'a>,
    escrow: BountyEscrowContractClient<'a>,
}

impl<'a> TestSetup<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);

        let (token, token_admin) = create_token_contract(&env, &admin);
        let escrow = create_escrow_contract(&env);

        escrow.init(&admin, &token.address);
        token_admin.mint(&depositor, &1_000_000);

        Self {
            env,
            admin,
            depositor,
            contributor,
            token,
            token_admin,
            escrow,
        }
    }
}

// Valid transitions: Locked → Released
#[test]
fn test_locked_to_released() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    assert_eq!(
        setup.escrow.get_escrow_info(&bounty_id).status,
        EscrowStatus::Locked
    );

    setup.escrow.release_funds(&bounty_id, &setup.contributor);
    assert_eq!(
        setup.escrow.get_escrow_info(&bounty_id).status,
        EscrowStatus::Released
    );
}

// Valid transitions: Locked → Refunded
#[test]
fn test_locked_to_refunded() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 100;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    assert_eq!(
        setup.escrow.get_escrow_info(&bounty_id).status,
        EscrowStatus::Locked
    );

    setup.env.ledger().set_timestamp(deadline + 1);
    setup.escrow.refund(&bounty_id);
    assert_eq!(
        setup.escrow.get_escrow_info(&bounty_id).status,
        EscrowStatus::Refunded
    );
}

// Valid transitions: Locked → PartiallyRefunded
#[test]
fn test_locked_to_partially_refunded() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 100;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    assert_eq!(
        setup.escrow.get_escrow_info(&bounty_id).status,
        EscrowStatus::Locked
    );

    // Approve partial refund before deadline
    setup
        .escrow
        .approve_refund(&bounty_id, &500, &setup.depositor, &RefundMode::Partial);
    setup.escrow.refund(&bounty_id);
    assert_eq!(
        setup.escrow.get_escrow_info(&bounty_id).status,
        EscrowStatus::PartiallyRefunded
    );
}

// Valid transitions: PartiallyRefunded → Refunded
#[test]
fn test_partially_refunded_to_refunded() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 100;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // First partial refund
    setup
        .escrow
        .approve_refund(&bounty_id, &500, &setup.depositor, &RefundMode::Partial);
    setup.escrow.refund(&bounty_id);
    assert_eq!(
        setup.escrow.get_escrow_info(&bounty_id).status,
        EscrowStatus::PartiallyRefunded
    );

    // Second refund completes it
    setup.env.ledger().set_timestamp(deadline + 1);
    setup.escrow.refund(&bounty_id);
    assert_eq!(
        setup.escrow.get_escrow_info(&bounty_id).status,
        EscrowStatus::Refunded
    );
}

// Invalid transition: Released → Locked
#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_released_to_locked_fails() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.release_funds(&bounty_id, &setup.contributor);

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
}

// Invalid transition: Released → Released
#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_released_to_released_fails() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.release_funds(&bounty_id, &setup.contributor);

    setup.escrow.release_funds(&bounty_id, &setup.contributor);
}

// Invalid transition: Released → Refunded
#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_released_to_refunded_fails() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 100;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.release_funds(&bounty_id, &setup.contributor);

    setup.env.ledger().set_timestamp(deadline + 1);
    setup.escrow.refund(&bounty_id);
}

// Invalid transition: Released → PartiallyRefunded
#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_released_to_partially_refunded_fails() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 100;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.release_funds(&bounty_id, &setup.contributor);

    setup.env.ledger().set_timestamp(deadline + 1);
    setup
        .escrow
        .partial_release(&bounty_id, &setup.contributor, &500);
}

// Invalid transition: Refunded → Locked
#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_refunded_to_locked_fails() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 100;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.env.ledger().set(LedgerInfo {
        timestamp: deadline + 1,
        protocol_version: 20,
        sequence_number: 0,
        network_id: Default::default(),
        base_reserve: 0,
        min_temp_entry_ttl: 0,
        min_persistent_entry_ttl: 0,
        max_entry_ttl: 0,
    });
    setup.escrow.refund(&bounty_id);

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
}

// Invalid transition: Refunded → Released
#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_refunded_to_released_fails() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 100;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.env.ledger().set(LedgerInfo {
        timestamp: deadline + 1,
        protocol_version: 20,
        sequence_number: 0,
        network_id: Default::default(),
        base_reserve: 0,
        min_temp_entry_ttl: 0,
        min_persistent_entry_ttl: 0,
        max_entry_ttl: 0,
    });
    setup.escrow.refund(&bounty_id);

    setup.escrow.release_funds(&bounty_id, &setup.contributor);
}

// Invalid transition: Refunded → Refunded
#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_refunded_to_refunded_fails() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 100;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.env.ledger().set(LedgerInfo {
        timestamp: deadline + 1,
        protocol_version: 20,
        sequence_number: 0,
        network_id: Default::default(),
        base_reserve: 0,
        min_temp_entry_ttl: 0,
        min_persistent_entry_ttl: 0,
        max_entry_ttl: 0,
    });
    setup.escrow.refund(&bounty_id);

    setup.escrow.refund(&bounty_id);
}

// Invalid transition: Refunded → PartiallyRefunded
#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_refunded_to_partially_refunded_fails() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 100;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.env.ledger().set(LedgerInfo {
        timestamp: deadline + 1,
        protocol_version: 20,
        sequence_number: 0,
        network_id: Default::default(),
        base_reserve: 0,
        min_temp_entry_ttl: 0,
        min_persistent_entry_ttl: 0,
        max_entry_ttl: 0,
    });
    setup.escrow.refund(&bounty_id);

    setup
        .escrow
        .partial_release(&bounty_id, &setup.contributor, &100);
}

// Invalid transition: PartiallyRefunded → Locked
#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_partially_refunded_to_locked_fails() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 100;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup
        .escrow
        .approve_refund(&bounty_id, &500, &setup.depositor, &RefundMode::Partial);
    setup.escrow.refund(&bounty_id);

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
}

// Invalid transition: PartiallyRefunded → Released
#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_partially_refunded_to_released_fails() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 100;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup
        .escrow
        .approve_refund(&bounty_id, &500, &setup.depositor, &RefundMode::Partial);
    setup.escrow.refund(&bounty_id);

    setup.escrow.release_funds(&bounty_id, &setup.contributor);
}
