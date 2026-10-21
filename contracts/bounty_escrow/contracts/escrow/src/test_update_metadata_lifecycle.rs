#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Bytes, BytesN, Env, String,
};

/// Test harness for verifying `update_metadata` behavior across escrow lifecycle states.
struct TestSetup<'a> {
    env: Env,
    admin: Address,
    depositor: Address,
    contributor: Address,
    contract_id: Address,
    client: BountyEscrowContractClient<'a>,
    token_admin: token::StellarAssetClient<'a>,
}

impl<'a> TestSetup<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);

        let token_id = env
            .register_stellar_asset_contract_v2(admin.clone())
            .address();
        let token_admin = token::StellarAssetClient::new(&env, &token_id);

        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);

        client.init(&admin, &token_id);
        token_admin.mint(&depositor, &1_000_000);

        Self {
            env,
            admin,
            depositor,
            contributor,
            contract_id,
            client,
            token_admin,
        }
    }

    /// Direct storage fixture to establish an escrow in a precise lifecycle state.
    fn setup_escrow_in_state(&self, bounty_id: u64, amount: i128, status: EscrowStatus) -> Escrow {
        let deadline = self.env.ledger().timestamp() + 1000;
        let remaining_amount = match status {
            EscrowStatus::Released | EscrowStatus::Refunded => 0,
            EscrowStatus::PartiallyRefunded => amount / 2,
            EscrowStatus::Draft | EscrowStatus::Locked => amount,
        };
        let escrow = Escrow {
            depositor: self.depositor.clone(),
            amount,
            remaining_amount,
            status,
            deadline,
            refund_history: soroban_sdk::vec![&self.env],
            archived: false,
            archived_at: None,
        };

        self.token_admin.mint(&self.contract_id, &amount);

        self.env.as_contract(&self.contract_id, || {
            self.env
                .storage()
                .persistent()
                .set(&DataKey::Escrow(bounty_id), &escrow);
        });

        escrow
    }

    /// Direct storage fixture to establish an anonymous escrow in a precise lifecycle state.
    fn setup_anon_escrow_in_state(
        &self,
        bounty_id: u64,
        amount: i128,
        status: EscrowStatus,
    ) -> AnonymousEscrow {
        let deadline = self.env.ledger().timestamp() + 1000;
        let remaining_amount = match status {
            EscrowStatus::Released | EscrowStatus::Refunded => 0,
            EscrowStatus::PartiallyRefunded => amount / 2,
            EscrowStatus::Draft | EscrowStatus::Locked => amount,
        };
        let commitment = BytesN::from_array(&self.env, &[42u8; 32]);
        let anon = AnonymousEscrow {
            depositor_commitment: commitment,
            amount,
            remaining_amount,
            status,
            deadline,
            refund_history: soroban_sdk::vec![&self.env],
            archived: false,
            archived_at: None,
        };

        self.token_admin.mint(&self.contract_id, &amount);

        self.env.as_contract(&self.contract_id, || {
            self.env
                .storage()
                .persistent()
                .set(&DataKey::EscrowAnon(bounty_id), &anon);
        });

        anon
    }

    /// Read Escrow directly from storage.
    fn get_stored_escrow(&self, bounty_id: u64) -> Option<Escrow> {
        self.env.as_contract(&self.contract_id, || {
            self.env
                .storage()
                .persistent()
                .get(&DataKey::Escrow(bounty_id))
        })
    }

    /// Read AnonymousEscrow directly from storage.
    fn get_stored_anon_escrow(&self, bounty_id: u64) -> Option<AnonymousEscrow> {
        self.env.as_contract(&self.contract_id, || {
            self.env
                .storage()
                .persistent()
                .get(&DataKey::EscrowAnon(bounty_id))
        })
    }
}

// ============================================================================
// TESTS DRIVING EACH ESCROW STATUS ENUM VARIANT (INCLUDING TERMINAL STATES)
// ============================================================================

/// Drives `update_metadata` when the escrow record is in `EscrowStatus::Draft` state.
#[test]
fn test_update_metadata_driving_draft_state() {
    let setup = TestSetup::new();
    let bounty_id = 101;
    let amount = 5_000;

    let escrow_before = setup.setup_escrow_in_state(bounty_id, amount, EscrowStatus::Draft);
    assert_eq!(escrow_before.status, EscrowStatus::Draft);

    let repo_id = 1001;
    let issue_id = 2001;
    let bounty_type = String::from_str(&setup.env, "security_fix");
    let ref_hash = Some(Bytes::from_slice(&setup.env, &[10, 20, 30, 40]));

    let result = setup.client.try_update_metadata(
        &setup.admin,
        &bounty_id,
        &repo_id,
        &issue_id,
        &bounty_type,
        &ref_hash,
    );

    // Each outcome is asserted as a value or a typed error, not a panic
    assert!(result.is_ok(), "Expected Ok result, got {:?}", result);
    let outcome = result.unwrap();
    assert!(
        outcome.is_ok(),
        "Expected Ok(EscrowMetadata), got {:?}",
        outcome
    );
    let meta = outcome.unwrap();
    assert_eq!(meta.repo_id, repo_id);
    assert_eq!(meta.issue_id, issue_id);
    assert_eq!(meta.bounty_type, bounty_type);
    assert_eq!(meta.reference_hash, ref_hash);
    assert_eq!(meta.risk_flags, 0);
    assert_eq!(meta.notification_prefs, 0);

    // Record must not be mutated
    let escrow_after = setup.get_stored_escrow(bounty_id).unwrap();
    assert_eq!(
        escrow_after, escrow_before,
        "Escrow in Draft state must not be mutated by update_metadata"
    );
    assert_eq!(escrow_after.status, EscrowStatus::Draft);

    let fetched = setup.client.get_metadata(&bounty_id);
    assert_eq!(fetched, meta);
}

/// Drives `update_metadata` when the escrow record is in `EscrowStatus::Locked` state.
#[test]
fn test_update_metadata_driving_locked_state() {
    let setup = TestSetup::new();
    let bounty_id = 102;
    let amount = 8_000;
    let deadline = setup.env.ledger().timestamp() + 500;

    setup
        .client
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    let escrow_before = setup.client.get_escrow_info(&bounty_id);
    assert_eq!(escrow_before.status, EscrowStatus::Locked);

    let repo_id = 1002;
    let issue_id = 2002;
    let bounty_type = String::from_str(&setup.env, "feature");
    let ref_hash = None;

    let result = setup.client.try_update_metadata(
        &setup.admin,
        &bounty_id,
        &repo_id,
        &issue_id,
        &bounty_type,
        &ref_hash,
    );

    // Outcome asserted as a value or typed error, not a panic
    assert!(result.is_ok(), "Expected Ok result, got {:?}", result);
    let outcome = result.unwrap();
    assert!(
        outcome.is_ok(),
        "Expected Ok(EscrowMetadata), got {:?}",
        outcome
    );
    let meta = outcome.unwrap();
    assert_eq!(meta.repo_id, repo_id);
    assert_eq!(meta.issue_id, issue_id);
    assert_eq!(meta.bounty_type, bounty_type);
    assert_eq!(meta.reference_hash, ref_hash);

    let escrow_after = setup.client.get_escrow_info(&bounty_id);
    assert_eq!(
        escrow_after, escrow_before,
        "Escrow in Locked state must not be mutated by update_metadata"
    );
    assert_eq!(escrow_after.status, EscrowStatus::Locked);

    let fetched = setup.client.get_metadata(&bounty_id);
    assert_eq!(fetched, meta);
}

/// Drives `update_metadata` when the escrow record is in `EscrowStatus::Released` (terminal state).
/// Asserts the terminal record is strictly unmutated.
#[test]
fn test_update_metadata_driving_released_terminal_state() {
    let setup = TestSetup::new();
    let bounty_id = 103;
    let amount = 10_000;

    let escrow_before = setup.setup_escrow_in_state(bounty_id, amount, EscrowStatus::Released);
    assert_eq!(escrow_before.status, EscrowStatus::Released);
    assert_eq!(escrow_before.remaining_amount, 0);

    let repo_id = 1003;
    let issue_id = 2003;
    let bounty_type = String::from_str(&setup.env, "documentation");
    let ref_hash = Some(Bytes::from_slice(&setup.env, &[1, 2, 3]));

    let result = setup.client.try_update_metadata(
        &setup.admin,
        &bounty_id,
        &repo_id,
        &issue_id,
        &bounty_type,
        &ref_hash,
    );

    // Each outcome is asserted as a value or a typed error, not a panic
    match result {
        Ok(Ok(meta)) => {
            assert_eq!(meta.repo_id, repo_id);
            assert_eq!(meta.issue_id, issue_id);
            assert_eq!(meta.bounty_type, bounty_type);
            assert_eq!(meta.reference_hash, ref_hash);
        }
        Err(Ok(err)) => {
            // If refused, outcome must be a typed error
            assert!(
                matches!(err, Error::FundsNotLocked | Error::Unauthorized),
                "Unexpected typed error variant: {:?}",
                err
            );
        }
        other => panic!("Expected value or typed error, got panic: {:?}", other),
    }

    // A record in a terminal state is not mutated by a refused or accepted call
    let escrow_after = setup.get_stored_escrow(bounty_id).unwrap();
    assert_eq!(
        escrow_after, escrow_before,
        "Record in Released terminal state must not be mutated"
    );
    assert_eq!(escrow_after.status, EscrowStatus::Released);
    assert_eq!(escrow_after.remaining_amount, 0);
    assert_eq!(escrow_after.amount, amount);
    assert_eq!(escrow_after.depositor, setup.depositor);
}

/// Drives `update_metadata` when the escrow record is in `EscrowStatus::Refunded` (terminal state).
/// Asserts the terminal record is strictly unmutated.
#[test]
fn test_update_metadata_driving_refunded_terminal_state() {
    let setup = TestSetup::new();
    let bounty_id = 104;
    let amount = 6_000;

    let escrow_before = setup.setup_escrow_in_state(bounty_id, amount, EscrowStatus::Refunded);
    assert_eq!(escrow_before.status, EscrowStatus::Refunded);
    assert_eq!(escrow_before.remaining_amount, 0);

    let repo_id = 1004;
    let issue_id = 2004;
    let bounty_type = String::from_str(&setup.env, "bug_bounty");
    let ref_hash = None;

    let result = setup.client.try_update_metadata(
        &setup.admin,
        &bounty_id,
        &repo_id,
        &issue_id,
        &bounty_type,
        &ref_hash,
    );

    // Outcome is asserted as a value or typed error, not a panic
    match result {
        Ok(Ok(meta)) => {
            assert_eq!(meta.repo_id, repo_id);
            assert_eq!(meta.issue_id, issue_id);
            assert_eq!(meta.bounty_type, bounty_type);
            assert_eq!(meta.reference_hash, ref_hash);
        }
        Err(Ok(err)) => {
            assert!(
                matches!(err, Error::FundsNotLocked | Error::Unauthorized),
                "Unexpected typed error variant: {:?}",
                err
            );
        }
        other => panic!("Expected value or typed error, got panic: {:?}", other),
    }

    // A record in a terminal state is not mutated by a refused or accepted call
    let escrow_after = setup.get_stored_escrow(bounty_id).unwrap();
    assert_eq!(
        escrow_after, escrow_before,
        "Record in Refunded terminal state must not be mutated"
    );
    assert_eq!(escrow_after.status, EscrowStatus::Refunded);
    assert_eq!(escrow_after.remaining_amount, 0);
    assert_eq!(escrow_after.amount, amount);
    assert_eq!(escrow_after.depositor, setup.depositor);
}

/// Drives `update_metadata` when the escrow record is in `EscrowStatus::PartiallyRefunded` state.
#[test]
fn test_update_metadata_driving_partially_refunded_state() {
    let setup = TestSetup::new();
    let bounty_id = 105;
    let amount = 12_000;

    let escrow_before =
        setup.setup_escrow_in_state(bounty_id, amount, EscrowStatus::PartiallyRefunded);
    assert_eq!(escrow_before.status, EscrowStatus::PartiallyRefunded);
    assert_eq!(escrow_before.remaining_amount, amount / 2);

    let repo_id = 1005;
    let issue_id = 2005;
    let bounty_type = String::from_str(&setup.env, "maintenance");
    let ref_hash = Some(Bytes::from_slice(&setup.env, &[9, 8, 7]));

    let result = setup.client.try_update_metadata(
        &setup.admin,
        &bounty_id,
        &repo_id,
        &issue_id,
        &bounty_type,
        &ref_hash,
    );

    // Outcome is asserted as a value or typed error, not a panic
    assert!(result.is_ok(), "Expected Ok result, got {:?}", result);
    let outcome = result.unwrap();
    assert!(
        outcome.is_ok(),
        "Expected Ok(EscrowMetadata), got {:?}",
        outcome
    );
    let meta = outcome.unwrap();
    assert_eq!(meta.repo_id, repo_id);
    assert_eq!(meta.issue_id, issue_id);
    assert_eq!(meta.bounty_type, bounty_type);
    assert_eq!(meta.reference_hash, ref_hash);

    let escrow_after = setup.get_stored_escrow(bounty_id).unwrap();
    assert_eq!(
        escrow_after, escrow_before,
        "Escrow in PartiallyRefunded state must not be mutated"
    );
    assert_eq!(escrow_after.status, EscrowStatus::PartiallyRefunded);
    assert_eq!(escrow_after.remaining_amount, amount / 2);
}

// ============================================================================
// TESTS DRIVING TERMINAL STATES WITH EXISTING RISK FLAGS AND PREFERENCES
// ============================================================================

/// Drives `update_metadata` on a record in `Released` terminal state that already has
/// risk flags and notification preferences set, asserting they are preserved and
/// the terminal record is not mutated.
#[test]
fn test_update_metadata_driving_released_terminal_state_preserves_existing_risk_flags() {
    let setup = TestSetup::new();
    let bounty_id = 106;
    let amount = 15_000;

    let escrow_before = setup.setup_escrow_in_state(bounty_id, amount, EscrowStatus::Released);
    assert_eq!(escrow_before.status, EscrowStatus::Released);

    // Seed prior metadata with non-zero risk flags and notification preferences
    let initial_flags = RISK_FLAG_HIGH_RISK | RISK_FLAG_RESTRICTED;
    let initial_prefs = NOTIFY_ON_LOCK | NOTIFY_ON_RELEASE;
    setup.env.as_contract(&setup.contract_id, || {
        setup.env.storage().persistent().set(
            &DataKey::Metadata(bounty_id),
            &EscrowMetadata {
                repo_id: 999,
                issue_id: 888,
                bounty_type: String::from_str(&setup.env, "old_type"),
                risk_flags: initial_flags,
                notification_prefs: initial_prefs,
                reference_hash: None,
            },
        );
    });

    let repo_id = 2001;
    let issue_id = 3001;
    let new_bounty_type = String::from_str(&setup.env, "new_type");
    let new_hash = Some(Bytes::from_slice(&setup.env, &[0xAA, 0xBB]));

    let outcome = setup.client.try_update_metadata(
        &setup.admin,
        &bounty_id,
        &repo_id,
        &issue_id,
        &new_bounty_type,
        &new_hash,
    );

    // Assert as value or typed error, not panic
    match outcome {
        Ok(Ok(updated)) => {
            assert_eq!(updated.repo_id, repo_id);
            assert_eq!(updated.issue_id, issue_id);
            assert_eq!(updated.bounty_type, new_bounty_type);
            assert_eq!(updated.reference_hash, new_hash);
            // Risk flags and notification preferences must be preserved
            assert_eq!(updated.risk_flags, initial_flags);
            assert_eq!(updated.notification_prefs, initial_prefs);
        }
        Err(Ok(err)) => {
            assert!(matches!(err, Error::FundsNotLocked | Error::Unauthorized));
        }
        other => panic!("Expected value or typed error, got panic: {:?}", other),
    }

    // Terminal record must not be mutated
    let escrow_after = setup.get_stored_escrow(bounty_id).unwrap();
    assert_eq!(
        escrow_after, escrow_before,
        "Released terminal escrow must remain immutable"
    );
    assert_eq!(escrow_after.status, EscrowStatus::Released);
}

/// Drives `update_metadata` on a record in `Refunded` terminal state that already has
/// risk flags and notification preferences set, asserting they are preserved and
/// the terminal record is not mutated.
#[test]
fn test_update_metadata_driving_refunded_terminal_state_preserves_existing_risk_flags() {
    let setup = TestSetup::new();
    let bounty_id = 107;
    let amount = 18_000;

    let escrow_before = setup.setup_escrow_in_state(bounty_id, amount, EscrowStatus::Refunded);
    assert_eq!(escrow_before.status, EscrowStatus::Refunded);

    let initial_flags = RISK_FLAG_UNDER_REVIEW | RISK_FLAG_DEPRECATED;
    let initial_prefs = NOTIFY_ON_DISPUTE | NOTIFY_ON_EXPIRATION;
    setup.env.as_contract(&setup.contract_id, || {
        setup.env.storage().persistent().set(
            &DataKey::Metadata(bounty_id),
            &EscrowMetadata {
                repo_id: 111,
                issue_id: 222,
                bounty_type: String::from_str(&setup.env, "old_type_2"),
                risk_flags: initial_flags,
                notification_prefs: initial_prefs,
                reference_hash: None,
            },
        );
    });

    let repo_id = 4001;
    let issue_id = 5001;
    let new_bounty_type = String::from_str(&setup.env, "reviewed_fix");
    let new_hash = Some(Bytes::from_slice(&setup.env, &[0xDE, 0xAD]));

    let outcome = setup.client.try_update_metadata(
        &setup.admin,
        &bounty_id,
        &repo_id,
        &issue_id,
        &new_bounty_type,
        &new_hash,
    );

    match outcome {
        Ok(Ok(updated)) => {
            assert_eq!(updated.repo_id, repo_id);
            assert_eq!(updated.issue_id, issue_id);
            assert_eq!(updated.bounty_type, new_bounty_type);
            assert_eq!(updated.reference_hash, new_hash);
            assert_eq!(updated.risk_flags, initial_flags);
            assert_eq!(updated.notification_prefs, initial_prefs);
        }
        Err(Ok(err)) => {
            assert!(matches!(err, Error::FundsNotLocked | Error::Unauthorized));
        }
        other => panic!("Expected value or typed error, got panic: {:?}", other),
    }

    let escrow_after = setup.get_stored_escrow(bounty_id).unwrap();
    assert_eq!(
        escrow_after, escrow_before,
        "Refunded terminal escrow must remain immutable"
    );
    assert_eq!(escrow_after.status, EscrowStatus::Refunded);
}

// ============================================================================
// TESTS DRIVING TRANSITIONS VIA NATURAL CONTRACT LIFECYCLE
// ============================================================================

/// Drives `update_metadata` along natural state progression:
/// Locked -> PartiallyRefunded -> Refunded (terminal state).
#[test]
fn test_update_metadata_driving_natural_lifecycle_refund_progression() {
    let setup = TestSetup::new();
    let bounty_id = 108;
    let amount = 20_000;
    let deadline = setup.env.ledger().timestamp() + 500;

    // 1. Locked state
    setup
        .client
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    let locked_escrow = setup.client.get_escrow_info(&bounty_id);
    assert_eq!(locked_escrow.status, EscrowStatus::Locked);

    let res_locked = setup.client.try_update_metadata(
        &setup.admin,
        &bounty_id,
        &101,
        &201,
        &String::from_str(&setup.env, "step_locked"),
        &None,
    );
    assert!(res_locked.is_ok() && res_locked.unwrap().is_ok());
    assert_eq!(setup.client.get_escrow_info(&bounty_id), locked_escrow);

    // 2. Transition to PartiallyRefunded state
    setup
        .client
        .approve_refund(&bounty_id, &5_000, &setup.depositor, &RefundMode::Partial);
    setup.client.refund(&bounty_id);
    let partial_escrow = setup.client.get_escrow_info(&bounty_id);
    assert_eq!(partial_escrow.status, EscrowStatus::PartiallyRefunded);

    let res_partial = setup.client.try_update_metadata(
        &setup.admin,
        &bounty_id,
        &102,
        &202,
        &String::from_str(&setup.env, "step_partial"),
        &None,
    );
    assert!(res_partial.is_ok() && res_partial.unwrap().is_ok());
    assert_eq!(setup.client.get_escrow_info(&bounty_id), partial_escrow);

    // 3. Transition to Refunded terminal state (advance time past deadline)
    setup.env.ledger().set_timestamp(deadline + 10);
    setup.client.refund(&bounty_id);
    let terminal_escrow = setup.client.get_escrow_info(&bounty_id);
    assert_eq!(terminal_escrow.status, EscrowStatus::Refunded);
    assert_eq!(terminal_escrow.remaining_amount, 0);

    let res_refunded = setup.client.try_update_metadata(
        &setup.admin,
        &bounty_id,
        &103,
        &203,
        &String::from_str(&setup.env, "step_terminal_refunded"),
        &None,
    );
    match res_refunded {
        Ok(Ok(meta)) => {
            assert_eq!(meta.repo_id, 103);
            assert_eq!(meta.issue_id, 203);
        }
        Err(Ok(err)) => {
            assert!(matches!(err, Error::FundsNotLocked | Error::Unauthorized));
        }
        other => panic!("Expected value or typed error, got panic: {:?}", other),
    }

    // Terminal escrow must remain completely unmutated
    let terminal_after = setup.client.get_escrow_info(&bounty_id);
    assert_eq!(terminal_after, terminal_escrow);
    assert_eq!(terminal_after.status, EscrowStatus::Refunded);
    assert_eq!(terminal_after.remaining_amount, 0);
}

/// Drives `update_metadata` along natural state progression:
/// Locked -> Released (terminal state).
#[test]
fn test_update_metadata_driving_natural_lifecycle_release_progression() {
    let setup = TestSetup::new();
    let bounty_id = 109;
    let amount = 10_000;
    let deadline = setup.env.ledger().timestamp() + 500;

    setup
        .client
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.client.release_funds(&bounty_id, &setup.contributor);

    let released_escrow = setup.client.get_escrow_info(&bounty_id);
    assert_eq!(released_escrow.status, EscrowStatus::Released);
    assert_eq!(released_escrow.remaining_amount, 0);

    let outcome = setup.client.try_update_metadata(
        &setup.admin,
        &bounty_id,
        &888,
        &999,
        &String::from_str(&setup.env, "step_terminal_released"),
        &None,
    );

    match outcome {
        Ok(Ok(meta)) => {
            assert_eq!(meta.repo_id, 888);
            assert_eq!(meta.issue_id, 999);
        }
        Err(Ok(err)) => {
            assert!(matches!(err, Error::FundsNotLocked | Error::Unauthorized));
        }
        other => panic!("Expected value or typed error, got panic: {:?}", other),
    }

    let released_after = setup.client.get_escrow_info(&bounty_id);
    assert_eq!(
        released_after, released_escrow,
        "Terminal Released escrow must not be mutated"
    );
    assert_eq!(released_after.status, EscrowStatus::Released);
    assert_eq!(released_after.remaining_amount, 0);
}

// ============================================================================
// TESTS DRIVING ANONYMOUS ESCROW TERMINAL STATES
// ============================================================================

/// Drives `update_metadata` when an anonymous escrow is in `Released` terminal state.
#[test]
fn test_update_metadata_driving_anonymous_escrow_released_terminal_state() {
    let setup = TestSetup::new();
    let bounty_id = 110;
    let amount = 7_000;

    let anon_before = setup.setup_anon_escrow_in_state(bounty_id, amount, EscrowStatus::Released);
    assert_eq!(anon_before.status, EscrowStatus::Released);
    assert_eq!(anon_before.remaining_amount, 0);

    let repo_id = 3333;
    let issue_id = 4444;
    let bounty_type = String::from_str(&setup.env, "anon_terminal_release");

    let result = setup.client.try_update_metadata(
        &setup.admin,
        &bounty_id,
        &repo_id,
        &issue_id,
        &bounty_type,
        &None,
    );

    match result {
        Ok(Ok(meta)) => {
            assert_eq!(meta.repo_id, repo_id);
            assert_eq!(meta.issue_id, issue_id);
            assert_eq!(meta.bounty_type, bounty_type);
        }
        Err(Ok(err)) => {
            assert!(matches!(err, Error::FundsNotLocked | Error::Unauthorized));
        }
        other => panic!("Expected value or typed error, got panic: {:?}", other),
    }

    let anon_after = setup.get_stored_anon_escrow(bounty_id).unwrap();
    assert_eq!(
        anon_after, anon_before,
        "Anonymous record in Released terminal state must not be mutated"
    );
    assert_eq!(anon_after.status, EscrowStatus::Released);
    assert_eq!(anon_after.remaining_amount, 0);
}

/// Drives `update_metadata` when an anonymous escrow is in `Refunded` terminal state.
#[test]
fn test_update_metadata_driving_anonymous_escrow_refunded_terminal_state() {
    let setup = TestSetup::new();
    let bounty_id = 111;
    let amount = 9_000;

    let anon_before = setup.setup_anon_escrow_in_state(bounty_id, amount, EscrowStatus::Refunded);
    assert_eq!(anon_before.status, EscrowStatus::Refunded);
    assert_eq!(anon_before.remaining_amount, 0);

    let repo_id = 5555;
    let issue_id = 6666;
    let bounty_type = String::from_str(&setup.env, "anon_terminal_refund");

    let result = setup.client.try_update_metadata(
        &setup.admin,
        &bounty_id,
        &repo_id,
        &issue_id,
        &bounty_type,
        &None,
    );

    match result {
        Ok(Ok(meta)) => {
            assert_eq!(meta.repo_id, repo_id);
            assert_eq!(meta.issue_id, issue_id);
            assert_eq!(meta.bounty_type, bounty_type);
        }
        Err(Ok(err)) => {
            assert!(matches!(err, Error::FundsNotLocked | Error::Unauthorized));
        }
        other => panic!("Expected value or typed error, got panic: {:?}", other),
    }

    let anon_after = setup.get_stored_anon_escrow(bounty_id).unwrap();
    assert_eq!(
        anon_after, anon_before,
        "Anonymous record in Refunded terminal state must not be mutated"
    );
    assert_eq!(anon_after.status, EscrowStatus::Refunded);
    assert_eq!(anon_after.remaining_amount, 0);
}

// ============================================================================
// TESTS DRIVING ARCHIVED AND MISSING/NONEXISTENT STATES
// ============================================================================

/// Drives `update_metadata` when the escrow record is explicitly archived.
#[test]
fn test_update_metadata_driving_archived_terminal_state() {
    let setup = TestSetup::new();
    let bounty_id = 112;
    let amount = 4_000;

    let mut escrow = setup.setup_escrow_in_state(bounty_id, amount, EscrowStatus::Refunded);
    escrow.archived = true;
    escrow.archived_at = Some(setup.env.ledger().timestamp());
    setup.env.as_contract(&setup.contract_id, || {
        setup
            .env
            .storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);
    });

    let repo_id = 7777;
    let issue_id = 8888;
    let bounty_type = String::from_str(&setup.env, "archived_metadata");

    let outcome = setup.client.try_update_metadata(
        &setup.admin,
        &bounty_id,
        &repo_id,
        &issue_id,
        &bounty_type,
        &None,
    );

    match outcome {
        Ok(Ok(meta)) => {
            assert_eq!(meta.repo_id, repo_id);
            assert_eq!(meta.issue_id, issue_id);
        }
        Err(Ok(err)) => {
            assert!(matches!(err, Error::FundsNotLocked | Error::Unauthorized));
        }
        other => panic!("Expected value or typed error, got panic: {:?}", other),
    }

    let escrow_after = setup.get_stored_escrow(bounty_id).unwrap();
    assert_eq!(
        escrow_after, escrow,
        "Archived escrow record must not be mutated"
    );
    assert!(escrow_after.archived);
}

/// Drives `update_metadata` when no escrow record exists at all for the bounty_id.
#[test]
fn test_update_metadata_driving_missing_nonexistent_state() {
    let setup = TestSetup::new();
    let non_existent_bounty = 99_999;

    assert!(setup.get_stored_escrow(non_existent_bounty).is_none());

    let repo_id = 12;
    let issue_id = 34;
    let bounty_type = String::from_str(&setup.env, "standalone_metadata");

    let result = setup.client.try_update_metadata(
        &setup.admin,
        &non_existent_bounty,
        &repo_id,
        &issue_id,
        &bounty_type,
        &None,
    );

    assert!(result.is_ok());
    let outcome = result.unwrap();
    assert!(outcome.is_ok());
    let meta = outcome.unwrap();
    assert_eq!(meta.repo_id, repo_id);
    assert_eq!(meta.issue_id, issue_id);

    // No phantom escrow created
    assert!(setup.get_stored_escrow(non_existent_bounty).is_none());
    assert_eq!(setup.client.get_metadata(&non_existent_bounty), meta);
}

/// Drives `update_metadata` when the contract itself is not initialized.
/// Asserts outcome is a typed error `Error::NotInitialized`, not a panic.
#[test]
fn test_update_metadata_driving_uninitialized_contract_state() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    let result = client.try_update_metadata(
        &admin,
        &1u64,
        &10u64,
        &20u64,
        &String::from_str(&env, "uninit"),
        &None,
    );

    // Assert outcome is a typed error, not a panic
    assert_eq!(result, Err(Ok(Error::NotInitialized)));
}

// ============================================================================
// TABLE-DRIVEN MATRIX EXERCISING EVERY ENUM STATE DEFINED BY EscrowStatus
// ============================================================================

struct StateTestCase {
    label: &'static str,
    status: EscrowStatus,
    is_terminal: bool,
}

/// Comprehensive matrix test driving `update_metadata` across every state defined by `EscrowStatus`.
#[test]
fn test_update_metadata_driving_all_states_matrix() {
    let setup = TestSetup::new();

    let test_cases = [
        StateTestCase {
            label: "EscrowStatus::Draft",
            status: EscrowStatus::Draft,
            is_terminal: false,
        },
        StateTestCase {
            label: "EscrowStatus::Locked",
            status: EscrowStatus::Locked,
            is_terminal: false,
        },
        StateTestCase {
            label: "EscrowStatus::Released",
            status: EscrowStatus::Released,
            is_terminal: true,
        },
        StateTestCase {
            label: "EscrowStatus::Refunded",
            status: EscrowStatus::Refunded,
            is_terminal: true,
        },
        StateTestCase {
            label: "EscrowStatus::PartiallyRefunded",
            status: EscrowStatus::PartiallyRefunded,
            is_terminal: false,
        },
    ];

    for (idx, case) in test_cases.iter().enumerate() {
        let bounty_id = 200 + idx as u64;
        let amount = 10_000 + (idx as i128) * 1_000;

        let escrow_before = setup.setup_escrow_in_state(bounty_id, amount, case.status.clone());
        assert_eq!(
            escrow_before.status, case.status,
            "Failed establishing state for {}",
            case.label
        );

        let repo_id = 500 + idx as u64;
        let issue_id = 600 + idx as u64;
        let bounty_type = String::from_str(&setup.env, "matrix_test");
        let ref_hash = Some(Bytes::from_slice(&setup.env, &[idx as u8; 4]));

        let result = setup.client.try_update_metadata(
            &setup.admin,
            &bounty_id,
            &repo_id,
            &issue_id,
            &bounty_type,
            &ref_hash,
        );

        // Each outcome must be asserted as a value or typed error, not a panic
        match result {
            Ok(Ok(meta)) => {
                assert_eq!(meta.repo_id, repo_id, "{}: repo_id mismatch", case.label);
                assert_eq!(meta.issue_id, issue_id, "{}: issue_id mismatch", case.label);
                assert_eq!(
                    meta.bounty_type, bounty_type,
                    "{}: bounty_type mismatch",
                    case.label
                );
                assert_eq!(
                    meta.reference_hash, ref_hash,
                    "{}: reference_hash mismatch",
                    case.label
                );
            }
            Err(Ok(err)) => {
                assert!(
                    case.is_terminal,
                    "{}: Non-terminal state unexpectedly refused: {:?}",
                    case.label, err
                );
            }
            other => panic!(
                "{}: Unexpected panic or invoke error: {:?}",
                case.label, other
            ),
        }

        // The record in storage must never be mutated
        let escrow_after = setup.get_stored_escrow(bounty_id).unwrap();
        assert_eq!(
            escrow_after, escrow_before,
            "{}: Record was mutated after update_metadata",
            case.label
        );
        assert_eq!(escrow_after.status, case.status);

        if case.is_terminal {
            assert_eq!(
                escrow_after.remaining_amount, 0,
                "{}: Terminal state must have remaining_amount == 0",
                case.label
            );
        }
    }
}
