//! Escrow Status Transition Matrix Tests
//!
//! Verifies all valid and invalid status transitions for the bounty escrow contract.
//!
//! FROM        | TO          | EXPECTED RESULT
//! ------------|-------------|----------------
//! Locked      | Locked      | Err (BountyExists)
//! Locked      | Released    | Ok  (allowed)
//! Locked      | Refunded    | Ok  (allowed)
//! Released    | Locked      | Err (BountyExists)
//! Released    | Released    | Err (FundsNotLocked)
//! Released    | Refunded    | Err (FundsNotLocked)
//! Refunded    | Locked      | Err (BountyExists)
//! Refunded    | Released    | Err (FundsNotLocked)
//! Refunded    | Refunded    | Err (FundsNotLocked)

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

/// Construct a fresh Escrow instance with the specified status.
fn create_escrow_with_status(
    env: &Env,
    depositor: Address,
    amount: i128,
    status: EscrowStatus,
    deadline: u64,
) -> Escrow {
    Escrow {
        depositor,
        amount,
        remaining_amount: amount,
        status,
        deadline,
        refund_history: vec![env],
        archived: false,
        archived_at: None,
    }
}

/// Test setup holding environment, clients, and addresses.
#[allow(dead_code)]
struct TestEnv {
    env: Env,
    contract_id: Address,
    client: BountyEscrowContractClient<'static>,
    token_admin: token::StellarAssetClient<'static>,
    admin: Address,
    depositor: Address,
    contributor: Address,
}

impl TestEnv {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);

        let token_id = env.register_stellar_asset_contract_v2(admin.clone()).address();
        let token_admin = token::StellarAssetClient::new(&env, &token_id);

        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);

        client.init(&admin, &token_id);

        Self {
            env,
            contract_id,
            client,
            token_admin,
            admin,
            depositor,
            contributor,
        }
    }

    /// Write escrow directly to persistent storage, bypassing lock_funds business logic.
    fn setup_escrow_in_state(&self, status: EscrowStatus, bounty_id: u64, amount: i128) {
        let deadline = self.env.ledger().timestamp() + 1000;
        let escrow = create_escrow_with_status(
            &self.env,
            self.depositor.clone(),
            amount,
            status,
            deadline,
        );

        // Mint tokens directly to the contract so that valid release/refund transfers succeed.
        self.token_admin.mint(&self.contract_id, &amount);

        self.env.as_contract(&self.contract_id, || {
            self.env
                .storage()
                .persistent()
                .set(&DataKey::Escrow(bounty_id), &escrow);
        });
    }
}

#[derive(Clone, Debug)]
enum TransitionAction {
    Lock,
    Release,
    Refund,
}

struct TransitionTestCase {
    label: &'static str,
    from: EscrowStatus,
    action: TransitionAction,
    expected_result: Result<(), Error>,
}

/// Table-driven exhaustive transition matrix test.
#[test]
fn test_all_status_transitions() {
    let cases = [
        TransitionTestCase {
            label: "Locked to Locked (Lock)",
            from: EscrowStatus::Locked,
            action: TransitionAction::Lock,
            expected_result: Err(Error::BountyExists),
        },
        TransitionTestCase {
            label: "Locked to Released (Release)",
            from: EscrowStatus::Locked,
            action: TransitionAction::Release,
            expected_result: Ok(()),
        },
        TransitionTestCase {
            label: "Locked to Refunded (Refund)",
            from: EscrowStatus::Locked,
            action: TransitionAction::Refund,
            expected_result: Ok(()),
        },
        TransitionTestCase {
            label: "Released to Locked (Lock)",
            from: EscrowStatus::Released,
            action: TransitionAction::Lock,
            expected_result: Err(Error::BountyExists),
        },
        TransitionTestCase {
            label: "Released to Released (Release)",
            from: EscrowStatus::Released,
            action: TransitionAction::Release,
            expected_result: Err(Error::FundsNotLocked),
        },
        TransitionTestCase {
            label: "Released to Refunded (Refund)",
            from: EscrowStatus::Released,
            action: TransitionAction::Refund,
            expected_result: Err(Error::FundsNotLocked),
        },
        TransitionTestCase {
            label: "Refunded to Locked (Lock)",
            from: EscrowStatus::Refunded,
            action: TransitionAction::Lock,
            expected_result: Err(Error::BountyExists),
        },
        TransitionTestCase {
            label: "Refunded to Released (Release)",
            from: EscrowStatus::Refunded,
            action: TransitionAction::Release,
            expected_result: Err(Error::FundsNotLocked),
        },
        TransitionTestCase {
            label: "Refunded to Refunded (Refund)",
            from: EscrowStatus::Refunded,
            action: TransitionAction::Refund,
            expected_result: Err(Error::FundsNotLocked),
        },
    ];

    for case in cases {
        let setup = TestEnv::new();
        let bounty_id = 99;
        let amount = 1000;

        setup.setup_escrow_in_state(case.from.clone(), bounty_id, amount);
        if let TransitionAction::Refund = case.action {
            setup
                .env
                .ledger()
                .set_timestamp(setup.env.ledger().timestamp() + 2000);
        }

        match case.action {
            TransitionAction::Lock => {
                let deadline = setup.env.ledger().timestamp() + 1000;
                let result = setup.client.try_lock_funds(
                    &setup.depositor,
                    &bounty_id,
                    &amount,
                    &deadline,
                );
                match case.expected_result {
                    Ok(_) => assert!(
                        result.is_ok(),
                        "Transition '{}' failed: expected Ok but got {:?}",
                        case.label,
                        result
                    ),
                    Err(expected_err) => {
                        assert!(
                            result.is_err(),
                            "Transition '{}' failed: expected Err but got Ok",
                            case.label
                        );
                        assert_eq!(
                            result.unwrap_err().unwrap(),
                            expected_err,
                            "Transition '{}' failed: mismatched error variant",
                            case.label
                        );
                    }
                }
            }
            TransitionAction::Release => {
                let result = setup
                    .client
                    .try_release_funds(&bounty_id, &setup.contributor);
                match case.expected_result {
                    Ok(_) => assert!(
                        result.is_ok(),
                        "Transition '{}' failed: expected Ok but got {:?}",
                        case.label,
                        result
                    ),
                    Err(expected_err) => {
                        assert!(
                            result.is_err(),
                            "Transition '{}' failed: expected Err but got Ok",
                            case.label
                        );
                        assert_eq!(
                            result.unwrap_err().unwrap(),
                            expected_err,
                            "Transition '{}' failed: mismatched error variant",
                            case.label
                        );
                    }
                }
            }
            TransitionAction::Refund => {
                let result = setup.client.try_refund(&bounty_id);
                match case.expected_result {
                    Ok(_) => assert!(
                        result.is_ok(),
                        "Transition '{}' failed: expected Ok but got {:?}",
                        case.label,
                        result
                    ),
                    Err(expected_err) => {
                        assert!(
                            result.is_err(),
                            "Transition '{}' failed: expected Err but got Ok",
                            case.label
                        );
                        assert_eq!(
                            result.unwrap_err().unwrap(),
                            expected_err,
                            "Transition '{}' failed: mismatched error variant",
                            case.label
                        );
                    }
                }
            }
        }
    }
}

/// Verifies allowed transition from Locked to Released succeeds.
#[test]
fn test_locked_to_released_succeeds() {
    let setup = TestEnv::new();
    let bounty_id = 1;
    let amount = 1000;
    setup.setup_escrow_in_state(EscrowStatus::Locked, bounty_id, amount);
    setup.client.release_funds(&bounty_id, &setup.contributor);
    let stored_escrow = setup.client.get_escrow_info(&bounty_id);
    assert_eq!(
        stored_escrow.status,
        EscrowStatus::Released,
        "Escrow status did not transition to Released"
    );
}

/// Verifies allowed transition from Locked to Refunded succeeds.
#[test]
fn test_locked_to_refunded_succeeds() {
    let setup = TestEnv::new();
    let bounty_id = 1;
    let amount = 1000;
    setup.setup_escrow_in_state(EscrowStatus::Locked, bounty_id, amount);
    setup
        .env
        .ledger()
        .set_timestamp(setup.env.ledger().timestamp() + 2000);
    setup.client.refund(&bounty_id);
    let stored_escrow = setup.client.get_escrow_info(&bounty_id);
    assert_eq!(
        stored_escrow.status,
        EscrowStatus::Refunded,
        "Escrow status did not transition to Refunded"
    );
}

/// Verifies disallowed transition from Released to Locked fails.
#[test]
fn test_released_to_locked_fails() {
    let setup = TestEnv::new();
    let bounty_id = 1;
    let amount = 1000;
    setup.setup_escrow_in_state(EscrowStatus::Released, bounty_id, amount);
    let deadline = setup.env.ledger().timestamp() + 1000;
    let result = setup
        .client
        .try_lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    assert!(result.is_err(), "Expected locking an already released bounty to fail");
    assert_eq!(
        result.unwrap_err().unwrap(),
        Error::BountyExists,
        "Expected BountyExists when attempting to Lock Released escrow."
    );
    let stored = setup.client.get_escrow_info(&bounty_id);
    assert_eq!(
        stored.status,
        EscrowStatus::Released,
        "Escrow status mutated after failed transition"
    );
}

/// Verifies disallowed transition from Refunded to Released fails.
#[test]
fn test_refunded_to_released_fails() {
    let setup = TestEnv::new();
    let bounty_id = 1;
    let amount = 1000;
    setup.setup_escrow_in_state(EscrowStatus::Refunded, bounty_id, amount);
    let result = setup
        .client
        .try_release_funds(&bounty_id, &setup.contributor);
    assert!(result.is_err(), "Expected releasing a refunded bounty to fail");
    assert_eq!(
        result.unwrap_err().unwrap(),
        Error::FundsNotLocked,
        "Expected FundsNotLocked error variant"
    );
    let stored = setup.client.get_escrow_info(&bounty_id);
    assert_eq!(
        stored.status,
        EscrowStatus::Refunded,
        "Escrow status mutated after failed transition"
    );
}

/// Verifies release on nonexistent bounty returns BountyNotFound.
#[test]
fn test_transition_from_uninitialized_state() {
    let setup = TestEnv::new();
    let bounty_id = 999;
    let result = setup
        .client
        .try_release_funds(&bounty_id, &setup.contributor);
    assert!(result.is_err(), "Expected release_funds on nonexistent to fail");
    assert_eq!(
        result.unwrap_err().unwrap(),
        Error::BountyNotFound,
        "Expected BountyNotFound error variant"
    );
}

/// Verifies idempotent transition attempt fails properly.
#[test]
fn test_idempotent_transition_attempt() {
    let setup = TestEnv::new();
    let bounty_id = 1;
    let amount = 1000;
    setup.setup_escrow_in_state(EscrowStatus::Locked, bounty_id, amount);
    setup.client.release_funds(&bounty_id, &setup.contributor);
    let result = setup
        .client
        .try_release_funds(&bounty_id, &setup.contributor);
    assert!(result.is_err(), "Expected idempotent transition attempt to fail");
    assert_eq!(
        result.unwrap_err().unwrap(),
        Error::FundsNotLocked,
        "Expected FundsNotLocked on idempotent attempt"
    );
}

/// Verifies status field is unchanged after a failed transition.
#[test]
fn test_status_field_unchanged_on_error() {
    let setup = TestEnv::new();
    let bounty_id = 1;
    let amount = 1000;
    setup.setup_escrow_in_state(EscrowStatus::Released, bounty_id, amount);
    setup
        .env
        .ledger()
        .set_timestamp(setup.env.ledger().timestamp() + 2000);
    let result = setup.client.try_refund(&bounty_id);
    assert!(result.is_err(), "Expected refund on Released state to fail");
    let stored = setup.client.get_escrow_info(&bounty_id);
    assert_eq!(
        stored.status,
        EscrowStatus::Released,
        "Escrow status should remain strictly unchanged"
    );
}
