use super::*;
use soroban_sdk::testutils::{Events, Ledger};
use soroban_sdk::{
    testutils::{Address as _, LedgerInfo, MockAuth, MockAuthInvoke},
    token, Address, Env, IntoVal, Symbol, Val,
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

struct RotationSetup<'a> {
    env: Env,
    admin: Address,
    pending_admin: Address,
    replacement_admin: Address,
    escrow: BountyEscrowContractClient<'a>,
}

impl<'a> RotationSetup<'a> {
    fn new() -> Self {
        let env = Env::default();
        let admin = Address::generate(&env);
        let pending_admin = Address::generate(&env);
        let replacement_admin = Address::generate(&env);
        let (token, _token_admin) = create_token_contract(&env, &admin);
        let escrow = create_escrow_contract(&env);

        authorize_contract_call(
            &env,
            &escrow,
            &admin,
            "init",
            soroban_sdk::vec![
                &env,
                admin.clone().into_val(&env),
                token.address.clone().into_val(&env),
            ],
        );
        escrow.init(&admin, &token.address);

        Self {
            env,
            admin,
            pending_admin,
            replacement_admin,
            escrow,
        }
    }

    fn authorize(&self, address: &Address, fn_name: &'static str, args: soroban_sdk::Vec<Val>) {
        authorize_contract_call(&self.env, &self.escrow, address, fn_name, args);
    }
}

fn authorize_contract_call(
    env: &Env,
    escrow: &BountyEscrowContractClient<'_>,
    address: &Address,
    fn_name: &'static str,
    args: soroban_sdk::Vec<Val>,
) {
    env.mock_auths(&[MockAuth {
        address,
        invoke: &MockAuthInvoke {
            contract: &escrow.address,
            fn_name,
            args,
            sub_invokes: &[],
        },
    }]);
}

#[test]
fn test_refund_eligibility_ineligible_before_deadline_without_approval() {
    let setup = TestSetup::new();
    let bounty_id = 99;
    let amount = 1_000;
    let deadline = setup.env.ledger().timestamp() + 500;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    let view = setup.escrow.get_refund_eligibility_view(&bounty_id);
    assert!(!view.eligible);
    assert_eq!(
        view.code,
        RefundEligibilityCode::IneligibleDeadlineNotPassed
    );
    assert_eq!(view.amount, 0);
    assert!(!view.approval_present);
}

#[test]
fn test_refund_eligibility_eligible_after_deadline() {
    let setup = TestSetup::new();
    let bounty_id = 100;
    let amount = 1_200;
    let deadline = setup.env.ledger().timestamp() + 100;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.env.ledger().set_timestamp(deadline + 1);

    let view = setup.escrow.get_refund_eligibility_view(&bounty_id);
    assert!(view.eligible);
    assert_eq!(view.code, RefundEligibilityCode::EligibleDeadlinePassed);
    assert_eq!(view.amount, amount);
    assert_eq!(view.recipient, Some(setup.depositor.clone()));
    assert!(!view.approval_present);
}

#[test]
fn test_refund_eligibility_eligible_with_admin_approval_before_deadline() {
    let setup = TestSetup::new();
    let bounty_id = 101;
    let amount = 2_000;
    let deadline = setup.env.ledger().timestamp() + 1_000;
    let custom_recipient = Address::generate(&setup.env);

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup
        .escrow
        .approve_refund(&bounty_id, &500, &custom_recipient, &RefundMode::Partial);

    let view = setup.escrow.get_refund_eligibility_view(&bounty_id);
    assert!(view.eligible);
    assert_eq!(view.code, RefundEligibilityCode::EligibleAdminApproval);
    assert_eq!(view.amount, 500);
    assert_eq!(view.recipient, Some(custom_recipient));
    assert!(view.approval_present);
}

/// Maintenance mode halts ALL state-mutating operations globally (lock, release, refund).
/// This is the hardened behavior: no state changes may occur during maintenance.
#[test]
fn test_maintenance_mode_blocks_all_operations() {
    let setup = TestSetup::new();
    let bounty_id = 202;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 100;

    setup.escrow.set_maintenance_mode(&true, &None);

    // Lock is blocked.
    let res = setup
        .escrow
        .try_lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    assert!(matches!(res, Err(Ok(Error::FundsPaused))));

    // Disable maintenance mode to lock, then re-enable to test release blocking.
    setup.escrow.set_maintenance_mode(&false, &None);
    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.set_maintenance_mode(&true, &None);

    // Release is also blocked in hardened maintenance mode.
    let res = setup.escrow.try_release_funds(&bounty_id, &setup.contributor);
    assert!(matches!(res, Err(Ok(Error::FundsPaused))));
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
#[should_panic(expected = "Error(Contract, #201)")]
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
#[should_panic(expected = "Error(Contract, #203)")]
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
#[should_panic(expected = "Error(Contract, #203)")]
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
#[should_panic(expected = "Error(Contract, #203)")]
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
#[should_panic(expected = "Error(Contract, #201)")]
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
#[should_panic(expected = "Error(Contract, #203)")]
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
#[should_panic(expected = "Error(Contract, #203)")]
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
#[should_panic(expected = "Error(Contract, #203)")]
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
#[should_panic(expected = "Error(Contract, #201)")]
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
#[should_panic(expected = "Error(Contract, #203)")]
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

// ============================================================================
// RISK FLAGS GOVERNANCE TESTS
// ============================================================================

#[test]
fn test_update_risk_flags_success() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    // Lock funds to create the initial escrow
    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Verify initial risk flags are 0 (no metadata existed yet, fallback applied)
    assert_eq!(setup.escrow.get_risk_flags(&bounty_id), 0);

    // Update risk flags (e.g., HIGH_RISK = 1, UNDER_REVIEW = 2) -> Bitmask 3
    let new_flags = 3;
    setup.escrow.update_risk_flags(&bounty_id, &new_flags);

    // Verify flags persisted in the EscrowMetadata struct
    assert_eq!(setup.escrow.get_risk_flags(&bounty_id), new_flags);
    
    // Clear the flags
    setup.escrow.update_risk_flags(&bounty_id, &0);
    assert_eq!(setup.escrow.get_risk_flags(&bounty_id), 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #202)")]
fn test_update_risk_flags_bounty_not_found() {
    let setup = TestSetup::new();
    let missing_bounty_id = 999;
    
    // Attempting to flag an escrow that does not exist should throw BountyNotFound (202)
    setup.escrow.update_risk_flags(&missing_bounty_id, &1);
}

#[test]
#[should_panic(expected = "Error(Contract, #202)")]
fn test_get_risk_flags_bounty_not_found() {
    let setup = TestSetup::new();
    let missing_bounty_id = 999;
    
    // Attempting to read flags from a missing escrow should fail
    setup.escrow.get_risk_flags(&missing_bounty_id);
}

// ============================================================================
// MAINTENANCE MODE HARDENING TESTS
// ============================================================================

#[test]
#[should_panic(expected = "Error(Contract, #18)")]
fn test_maintenance_mode_halts_lock() {
    let setup = TestSetup::new();
    let reason = soroban_sdk::String::from_str(&setup.env, "Emergency upgrade");
    setup.escrow.set_maintenance_mode(&true, &Some(reason));
    
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;
    
    // Should panic with FundsPaused (18)
    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
}

#[test]
#[should_panic(expected = "Error(Contract, #18)")]
fn test_maintenance_mode_halts_release() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;
    
    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    
    setup.escrow.set_maintenance_mode(&true, &None);
    
    // Should panic with FundsPaused (18)
    setup.escrow.release_funds(&bounty_id, &setup.contributor);
}

#[test]
#[should_panic(expected = "Error(Contract, #18)")]
fn test_maintenance_mode_halts_refund() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 100;
    
    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.env.ledger().set_timestamp(deadline + 1);
    
    setup.escrow.set_maintenance_mode(&true, &None);
    
    // Should panic with FundsPaused (18)
    setup.escrow.refund(&bounty_id);
}

#[test]
fn test_maintenance_mode_toggles_correctly() {
    let setup = TestSetup::new();
    let reason = soroban_sdk::String::from_str(&setup.env, "Routine sync");
    
    assert_eq!(setup.escrow.is_maintenance_mode(), false);
    
    setup.escrow.set_maintenance_mode(&true, &Some(reason));
    assert_eq!(setup.escrow.is_maintenance_mode(), true);
    
    setup.escrow.set_maintenance_mode(&false, &None);
    assert_eq!(setup.escrow.is_maintenance_mode(), false);
}

// ============================================================================
// CLAIM-WINDOW VALIDATION TESTS (Issue #1031)
// ============================================================================

/// Helper: lock a bounty and authorize a claim with a given window.
fn setup_claim_window_bounty(
    setup: &TestSetup,
    bounty_id: u64,
    amount: i128,
    claim_window_secs: u64,
) -> Address {
    let deadline = setup.env.ledger().timestamp() + 10_000;
    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.set_claim_window(&claim_window_secs);
    let recipient = Address::generate(&setup.env);
    setup.escrow.authorize_claim(&bounty_id, &recipient, &DisputeReason::Other);
    recipient
}

// --- set_claim_window ---

#[test]
fn test_set_claim_window_success() {
    let setup = TestSetup::new();
    // Should not panic; no return value to assert beyond no error.
    setup.escrow.set_claim_window(&3600_u64);
}

#[test]
fn test_set_claim_window_zero_disables_enforcement() {
    let setup = TestSetup::new();
    let bounty_id = 300;
    let amount = 1_000;
    // Set window to 0 — enforcement disabled.
    setup.escrow.set_claim_window(&0_u64);
    let deadline = setup.env.ledger().timestamp() + 10_000;
    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor, &DisputeReason::Other);
    // Advance time far past any window — should still succeed because window == 0.
    setup.env.ledger().set_timestamp(setup.env.ledger().timestamp() + 999_999);
    setup.escrow.release_funds(&bounty_id, &setup.contributor);
    assert_eq!(
        setup.escrow.get_escrow_info(&bounty_id).status,
        EscrowStatus::Released
    );
}

// --- validate_claim_window: no pending claim ---

#[test]
fn test_release_without_pending_claim_skips_window_check() {
    let setup = TestSetup::new();
    let bounty_id = 301;
    let amount = 1_000;
    let deadline = setup.env.ledger().timestamp() + 10_000;
    setup.escrow.set_claim_window(&60_u64);
    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    // No authorize_claim called — no PendingClaim exists.
    // release_funds should succeed regardless of window.
    setup.escrow.release_funds(&bounty_id, &setup.contributor);
    assert_eq!(
        setup.escrow.get_escrow_info(&bounty_id).status,
        EscrowStatus::Released
    );
}

// --- validate_claim_window: claim within window ---

#[test]
fn test_claim_within_window_succeeds() {
    let setup = TestSetup::new();
    let bounty_id = 302;
    let amount = 1_000;
    let recipient = setup_claim_window_bounty(&setup, bounty_id, amount, 3_600);
    // Still within the window — claim should succeed.
    setup.escrow.claim(&bounty_id);
    assert_eq!(
        setup.escrow.get_escrow_info(&bounty_id).status,
        EscrowStatus::Released
    );
    let _ = recipient; // used via authorize_claim
}

#[test]
fn test_release_within_window_succeeds() {
    let setup = TestSetup::new();
    let bounty_id = 303;
    let amount = 1_000;
    let _recipient = setup_claim_window_bounty(&setup, bounty_id, amount, 3_600);
    // Admin releases within the window — should succeed.
    setup.escrow.release_funds(&bounty_id, &setup.contributor);
    assert_eq!(
        setup.escrow.get_escrow_info(&bounty_id).status,
        EscrowStatus::Released
    );
}

// --- validate_claim_window: claim at exact boundary ---

#[test]
fn test_claim_at_exact_window_boundary_succeeds() {
    let setup = TestSetup::new();
    let bounty_id = 304;
    let amount = 1_000;
    let window = 3_600_u64;
    let now = setup.env.ledger().timestamp();
    let deadline = now + 10_000;
    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.set_claim_window(&window);
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor, &DisputeReason::Other);
    // Advance to exactly expires_at (now + window).
    setup.env.ledger().set_timestamp(now + window);
    // At the boundary (now == expires_at) the window is still valid.
    setup.escrow.claim(&bounty_id);
    assert_eq!(
        setup.escrow.get_escrow_info(&bounty_id).status,
        EscrowStatus::Released
    );
}

// --- validate_claim_window: expired window ---

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn test_claim_after_window_expires_fails() {
    let setup = TestSetup::new();
    let bounty_id = 305;
    let amount = 1_000;
    let window = 60_u64;
    let now = setup.env.ledger().timestamp();
    let deadline = now + 10_000;
    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.set_claim_window(&window);
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor, &DisputeReason::Other);
    // Advance past the window.
    setup.env.ledger().set_timestamp(now + window + 1);
    // Should panic with DeadlineNotPassed (#6).
    setup.escrow.claim(&bounty_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn test_release_after_window_expires_fails() {
    let setup = TestSetup::new();
    let bounty_id = 306;
    let amount = 1_000;
    let window = 60_u64;
    let now = setup.env.ledger().timestamp();
    let deadline = now + 10_000;
    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.set_claim_window(&window);
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor, &DisputeReason::Other);
    // Advance past the window.
    setup.env.ledger().set_timestamp(now + window + 1);
    // Should panic with DeadlineNotPassed (#6).
    setup.escrow.release_funds(&bounty_id, &setup.contributor);
}

// --- validate_claim_window: window not configured ---

#[test]
fn test_release_with_no_window_configured_succeeds() {
    let setup = TestSetup::new();
    let bounty_id = 307;
    let amount = 1_000;
    let deadline = setup.env.ledger().timestamp() + 10_000;
    // No set_claim_window call — defaults to 0 (disabled).
    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor, &DisputeReason::Other);
    // Advance time significantly — no window enforcement.
    setup.env.ledger().set_timestamp(setup.env.ledger().timestamp() + 999_999);
    setup.escrow.release_funds(&bounty_id, &setup.contributor);
    assert_eq!(
        setup.escrow.get_escrow_info(&bounty_id).status,
        EscrowStatus::Released
    );
}

// --- cancel then re-authorize ---

#[test]
fn test_cancel_expired_claim_then_authorize_new_window() {
    let setup = TestSetup::new();
    let bounty_id = 308;
    let amount = 1_000;
    let window = 60_u64;
    let now = setup.env.ledger().timestamp();
    let deadline = now + 10_000;
    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.set_claim_window(&window);
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor, &DisputeReason::Other);
    // Expire the first window.
    setup.env.ledger().set_timestamp(now + window + 1);
    // Admin cancels the stale claim.
    setup.escrow.cancel_pending_claim(&bounty_id, &DisputeOutcome::CancelledByAdmin);
    // Re-authorize with a fresh window.
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor, &DisputeReason::Other);
    // Claim should now succeed within the new window.
    setup.escrow.claim(&bounty_id);
    assert_eq!(
        setup.escrow.get_escrow_info(&bounty_id).status,
        EscrowStatus::Released
    );
}

// --- isolation: window on one bounty does not affect another ---

#[test]
fn test_claim_window_isolation_between_bounties() {
    let setup = TestSetup::new();
    let bounty_a = 309;
    let bounty_b = 310;
    let amount = 1_000;
    let window = 60_u64;
    let now = setup.env.ledger().timestamp();
    let deadline = now + 10_000;

    setup.escrow.set_claim_window(&window);

    // Lock both bounties.
    setup.escrow.lock_funds(&setup.depositor, &bounty_a, &amount, &deadline);
    setup.escrow.lock_funds(&setup.depositor, &bounty_b, &amount, &deadline);

    // Authorize claim on bounty_a only.
    setup.escrow.authorize_claim(&bounty_a, &setup.contributor, &DisputeReason::Other);

    // Advance past the window for bounty_a.
    setup.env.ledger().set_timestamp(now + window + 1);

    // bounty_b has no pending claim — release should succeed.
    setup.escrow.release_funds(&bounty_b, &setup.contributor);
    assert_eq!(
        setup.escrow.get_escrow_info(&bounty_b).status,
        EscrowStatus::Released
    );
}

// --- audit event emission ---

#[test]
fn test_set_claim_window_emits_event() {
    let setup = TestSetup::new();
    setup.escrow.set_claim_window(&7200_u64);
    let events = setup.env.events().all();
    let expected = soroban_sdk::Symbol::new(&setup.env, "clm_set");
    let found = events.iter().any(|(_, topics, _)| {
        topics.len() >= 1
            && topics
                .get(0)
                .and_then(|t| <Symbol as soroban_sdk::TryFromVal<Env, soroban_sdk::Val>>::try_from_val(&setup.env, &t).ok())
                .map(|s| s == expected)
                .unwrap_or(false)
    });
    assert!(found, "ClaimWindowSet event not emitted");
}

#[test]
fn test_claim_window_validated_event_emitted_on_success() {
    let setup = TestSetup::new();
    let bounty_id = 311;
    let amount = 1_000;
    let _recipient = setup_claim_window_bounty(&setup, bounty_id, amount, 3_600);
    setup.escrow.claim(&bounty_id);
    let events = setup.env.events().all();
    let expected = soroban_sdk::Symbol::new(&setup.env, "clm_ok");
    let found = events.iter().any(|(_, topics, _)| {
        topics.len() >= 1
            && topics
                .get(0)
                .and_then(|t| <Symbol as soroban_sdk::TryFromVal<Env, soroban_sdk::Val>>::try_from_val(&setup.env, &t).ok())
                .map(|s| s == expected)
                .unwrap_or(false)
    });
    assert!(found, "ClaimWindowValidated event not emitted");
}

#[test]
fn test_claim_window_expired_event_emitted_on_failure() {
    let setup = TestSetup::new();
    let bounty_id = 312;
    let amount = 1_000;
    let window = 60_u64;
    let now = setup.env.ledger().timestamp();
    let deadline = now + 10_000;
    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.set_claim_window(&window);
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor, &DisputeReason::Other);
    setup.env.ledger().set_timestamp(now + window + 1);
    // Attempt claim — will fail, but the expired event should be emitted.
    let _ = setup.escrow.try_claim(&bounty_id);
    let events = setup.env.events().all();
    let expected = soroban_sdk::Symbol::new(&setup.env, "clm_exp");
    let found = events.iter().any(|(_, topics, _)| {
        topics.len() >= 1
            && topics
                .get(0)
                .and_then(|t| <Symbol as soroban_sdk::TryFromVal<Env, soroban_sdk::Val>>::try_from_val(&setup.env, &t).ok())
                .map(|s| s == expected)
                .unwrap_or(false)
    });
    assert!(found, "ClaimWindowExpired event not emitted");
}

// ============================================================================
// BATCH SIZE CAPS TESTS (#04)
// ============================================================================

/// Helper: build a Vec of LockFundsItem for batch tests.
fn make_lock_items(setup: &TestSetup, start_id: u64, count: u32) -> soroban_sdk::Vec<LockFundsItem> {
    let mut items = soroban_sdk::Vec::new(&setup.env);
    let deadline = setup.env.ledger().timestamp() + 10_000;
    for i in 0..count {
        items.push_back(LockFundsItem {
            bounty_id: start_id + i as u64,
            depositor: setup.depositor.clone(),
            amount: 100,
            deadline,
        });
    }
    items
}

/// Helper: build a Vec of ReleaseFundsItem for batch tests.
fn make_release_items(setup: &TestSetup, start_id: u64, count: u32) -> soroban_sdk::Vec<ReleaseFundsItem> {
    let mut items = soroban_sdk::Vec::new(&setup.env);
    for i in 0..count {
        items.push_back(ReleaseFundsItem {
            bounty_id: start_id + i as u64,
            contributor: setup.contributor.clone(),
        });
    }
    items
}

// --- get_batch_size_caps: defaults ---

#[test]
fn test_get_batch_size_caps_defaults_to_max() {
    let setup = TestSetup::new();
    let caps = setup.escrow.get_batch_size_caps();
    // Default must equal the compile-time hard limit (20).
    assert_eq!(caps.lock_cap, 20);
    assert_eq!(caps.release_cap, 20);
}

// --- set_batch_size_caps: happy path ---

#[test]
fn test_set_batch_size_caps_success() {
    let setup = TestSetup::new();
    setup.escrow.set_batch_size_caps(&5_u32, &3_u32);
    let caps = setup.escrow.get_batch_size_caps();
    assert_eq!(caps.lock_cap, 5);
    assert_eq!(caps.release_cap, 3);
}

// --- set_batch_size_caps: emits BatchSizeCapsUpdated event ---

#[test]
fn test_set_batch_size_caps_emits_event() {
    let setup = TestSetup::new();
    setup.escrow.set_batch_size_caps(&4_u32, &2_u32);
    let events = setup.env.events().all();
    let found = events.iter().any(|(_, topics, _)| {
        topics.len() >= 1
            && topics
                .get(0)
                .map(|t| {
                    t == soroban_sdk::Symbol::new(&setup.env, "bcapcfg").into_val(&setup.env)
                })
                .unwrap_or(false)
    });
    assert!(found, "BatchSizeCapsUpdated event not emitted");
}

// --- set_batch_size_caps: boundary values ---

#[test]
fn test_set_batch_size_caps_min_boundary() {
    let setup = TestSetup::new();
    // cap = 1 is the minimum valid value.
    setup.escrow.set_batch_size_caps(&1_u32, &1_u32);
    let caps = setup.escrow.get_batch_size_caps();
    assert_eq!(caps.lock_cap, 1);
    assert_eq!(caps.release_cap, 1);
}

#[test]
fn test_set_batch_size_caps_max_boundary() {
    let setup = TestSetup::new();
    // cap = 20 (MAX_BATCH_SIZE) is the maximum valid value.
    setup.escrow.set_batch_size_caps(&20_u32, &20_u32);
    let caps = setup.escrow.get_batch_size_caps();
    assert_eq!(caps.lock_cap, 20);
    assert_eq!(caps.release_cap, 20);
}

// --- set_batch_size_caps: invalid inputs ---

#[test]
fn test_set_batch_size_caps_zero_lock_cap_rejected() {
    let setup = TestSetup::new();
    let res = setup.escrow.try_set_batch_size_caps(&0_u32, &5_u32);
    assert!(matches!(res, Err(Ok(Error::InvalidBatchSizeCap))));
}

#[test]
fn test_set_batch_size_caps_zero_release_cap_rejected() {
    let setup = TestSetup::new();
    let res = setup.escrow.try_set_batch_size_caps(&5_u32, &0_u32);
    assert!(matches!(res, Err(Ok(Error::InvalidBatchSizeCap))));
}

#[test]
fn test_set_batch_size_caps_exceeds_max_lock_rejected() {
    let setup = TestSetup::new();
    // 21 > MAX_BATCH_SIZE (20)
    let res = setup.escrow.try_set_batch_size_caps(&21_u32, &5_u32);
    assert!(matches!(res, Err(Ok(Error::InvalidBatchSizeCap))));
}

#[test]
fn test_set_batch_size_caps_exceeds_max_release_rejected() {
    let setup = TestSetup::new();
    let res = setup.escrow.try_set_batch_size_caps(&5_u32, &21_u32);
    assert!(matches!(res, Err(Ok(Error::InvalidBatchSizeCap))));
}

// --- batch_lock_funds: respects configured lock cap ---

#[test]
fn test_batch_lock_funds_within_cap_succeeds() {
    let setup = TestSetup::new();
    // Mint enough tokens for the batch.
    setup.token_admin.mint(&setup.depositor, &10_000);
    setup.escrow.set_batch_size_caps(&3_u32, &20_u32);
    let items = make_lock_items(&setup, 1000, 3);
    let count = setup.escrow.batch_lock_funds(&items);
    assert_eq!(count, 3);
}

#[test]
fn test_batch_lock_funds_exceeds_cap_rejected() {
    let setup = TestSetup::new();
    setup.token_admin.mint(&setup.depositor, &10_000);
    // Set lock cap to 2, then try to lock 3 items.
    setup.escrow.set_batch_size_caps(&2_u32, &20_u32);
    let items = make_lock_items(&setup, 2000, 3);
    let res = setup.escrow.try_batch_lock_funds(&items);
    assert!(matches!(res, Err(Ok(Error::InvalidBatchSize))));
}

#[test]
fn test_batch_lock_funds_exactly_at_cap_succeeds() {
    let setup = TestSetup::new();
    setup.token_admin.mint(&setup.depositor, &10_000);
    setup.escrow.set_batch_size_caps(&2_u32, &20_u32);
    let items = make_lock_items(&setup, 3000, 2);
    let count = setup.escrow.batch_lock_funds(&items);
    assert_eq!(count, 2);
}

// --- batch_release_funds: respects configured release cap ---

#[test]
fn test_batch_release_funds_within_cap_succeeds() {
    let setup = TestSetup::new();
    setup.token_admin.mint(&setup.depositor, &10_000);
    // Lock 3 bounties first.
    let lock_items = make_lock_items(&setup, 4000, 3);
    setup.escrow.batch_lock_funds(&lock_items);
    // Set release cap to 3 and release all.
    setup.escrow.set_batch_size_caps(&20_u32, &3_u32);
    let release_items = make_release_items(&setup, 4000, 3);
    let count = setup.escrow.batch_release_funds(&release_items);
    assert_eq!(count, 3);
}

#[test]
fn test_batch_release_funds_exceeds_cap_rejected() {
    let setup = TestSetup::new();
    setup.token_admin.mint(&setup.depositor, &10_000);
    let lock_items = make_lock_items(&setup, 5000, 3);
    setup.escrow.batch_lock_funds(&lock_items);
    // Set release cap to 2, then try to release 3.
    setup.escrow.set_batch_size_caps(&20_u32, &2_u32);
    let release_items = make_release_items(&setup, 5000, 3);
    let res = setup.escrow.try_batch_release_funds(&release_items);
    assert!(matches!(res, Err(Ok(Error::InvalidBatchSize))));
}

// --- lock and release caps are independent ---

#[test]
fn test_lock_and_release_caps_are_independent() {
    let setup = TestSetup::new();
    setup.token_admin.mint(&setup.depositor, &10_000);
    // lock_cap=5, release_cap=2
    setup.escrow.set_batch_size_caps(&5_u32, &2_u32);

    // Locking 4 items should succeed (4 <= 5).
    let lock_items = make_lock_items(&setup, 6000, 4);
    let count = setup.escrow.batch_lock_funds(&lock_items);
    assert_eq!(count, 4);

    // Releasing 3 items should fail (3 > 2).
    let release_items = make_release_items(&setup, 6000, 3);
    let res = setup.escrow.try_batch_release_funds(&release_items);
    assert!(matches!(res, Err(Ok(Error::InvalidBatchSize))));

    // Releasing 2 items should succeed (2 <= 2).
    let release_items_ok = make_release_items(&setup, 6000, 2);
    let released = setup.escrow.batch_release_funds(&release_items_ok);
    assert_eq!(released, 2);
}

// --- cap update is idempotent ---

#[test]
fn test_set_batch_size_caps_idempotent() {
    let setup = TestSetup::new();
    setup.escrow.set_batch_size_caps(&5_u32, &5_u32);
    setup.escrow.set_batch_size_caps(&5_u32, &5_u32);
    let caps = setup.escrow.get_batch_size_caps();
    assert_eq!(caps.lock_cap, 5);
    assert_eq!(caps.release_cap, 5);
}

// --- upgrade-safe: caps survive a re-read after storage write ---

#[test]
fn test_batch_size_caps_persist_in_storage() {
    let setup = TestSetup::new();
    setup.escrow.set_batch_size_caps(&7_u32, &3_u32);
    // Read back via the public view — must match what was written.
    let caps = setup.escrow.get_batch_size_caps();
    assert_eq!(caps.lock_cap, 7);
    assert_eq!(caps.release_cap, 3);
}

// ============================================================================
// HIGH-VALUE RELEASE TIMELOCK QUEUE TESTS
// ============================================================================

#[test]
fn test_set_high_value_config_stores_correctly() {
    let setup = TestSetup::new();
    let threshold: i128 = 5_000;
    let duration: u64 = 3_600;

    setup.escrow.set_high_value_config(&threshold, &duration);

    let cfg = setup.escrow.get_high_value_config().unwrap();
    assert_eq!(cfg.threshold, threshold);
    assert_eq!(cfg.duration, duration);
}

#[test]
fn test_set_high_value_config_zero_threshold_rejected() {
    let setup = TestSetup::new();
    let res = setup.escrow.try_set_high_value_config(&0, &3_600);
    assert!(matches!(res, Err(Ok(Error::InvalidAmount))));
}

#[test]
fn test_release_below_threshold_executes_immediately() {
    let setup = TestSetup::new();
    let bounty_id = 900;
    let amount: i128 = 1_000;
    let deadline = setup.env.ledger().timestamp() + 1_000;

    // Threshold is 5_000 — amount is below it.
    setup.escrow.set_high_value_config(&5_000, &3_600);
    setup.token_admin.mint(&setup.depositor, &amount);
    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.escrow.release_funds(&bounty_id, &setup.contributor);

    // Should be Released immediately (no queue).
    assert_eq!(
        setup.escrow.get_escrow_info(&bounty_id).status,
        EscrowStatus::Released
    );
    assert!(setup.escrow.get_queued_release(&bounty_id).is_none());
}

#[test]
fn test_release_at_threshold_queues_release() {
    let setup = TestSetup::new();
    let bounty_id = 901;
    let threshold: i128 = 5_000;
    let duration: u64 = 3_600;
    let deadline = setup.env.ledger().timestamp() + 10_000;

    setup.escrow.set_high_value_config(&threshold, &duration);
    setup.token_admin.mint(&setup.depositor, &threshold);
    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &threshold, &deadline);

    let now = setup.env.ledger().timestamp();
    setup.escrow.release_funds(&bounty_id, &setup.contributor);

    // Escrow should still be Locked (queued, not released).
    assert_eq!(
        setup.escrow.get_escrow_info(&bounty_id).status,
        EscrowStatus::Locked
    );

    let queued = setup.escrow.get_queued_release(&bounty_id).unwrap();
    assert_eq!(queued.contributor, setup.contributor);
    assert_eq!(queued.amount, threshold);
    assert_eq!(queued.executable_at, now + duration);
}

#[test]
fn test_execute_queued_release_before_timelock_fails() {
    let setup = TestSetup::new();
    let bounty_id = 902;
    let threshold: i128 = 5_000;
    let duration: u64 = 3_600;
    let deadline = setup.env.ledger().timestamp() + 10_000;

    setup.escrow.set_high_value_config(&threshold, &duration);
    setup.token_admin.mint(&setup.depositor, &threshold);
    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &threshold, &deadline);
    setup.escrow.release_funds(&bounty_id, &setup.contributor);

    // Try to execute before timelock elapses.
    let res = setup.escrow.try_execute_queued_release(&bounty_id);
    assert!(matches!(res, Err(Ok(Error::TimelockNotElapsed))));
}

#[test]
fn test_execute_queued_release_after_timelock_succeeds() {
    let setup = TestSetup::new();
    let bounty_id = 903;
    let threshold: i128 = 5_000;
    let duration: u64 = 3_600;
    let deadline = setup.env.ledger().timestamp() + 10_000;

    setup.escrow.set_high_value_config(&threshold, &duration);
    setup.token_admin.mint(&setup.depositor, &threshold);
    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &threshold, &deadline);
    setup.escrow.release_funds(&bounty_id, &setup.contributor);

    // Advance time past the timelock.
    setup.env.ledger().set_timestamp(setup.env.ledger().timestamp() + duration + 1);
    setup.escrow.execute_queued_release(&bounty_id);

    assert_eq!(
        setup.escrow.get_escrow_info(&bounty_id).status,
        EscrowStatus::Released
    );
    assert!(setup.escrow.get_queued_release(&bounty_id).is_none());
}

#[test]
fn test_execute_queued_release_at_exact_boundary_succeeds() {
    let setup = TestSetup::new();
    let bounty_id = 904;
    let threshold: i128 = 5_000;
    let duration: u64 = 3_600;
    let start = setup.env.ledger().timestamp();
    let deadline = start + 10_000;

    setup.escrow.set_high_value_config(&threshold, &duration);
    setup.token_admin.mint(&setup.depositor, &threshold);
    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &threshold, &deadline);
    setup.escrow.release_funds(&bounty_id, &setup.contributor);

    // Advance to exactly executable_at.
    setup.env.ledger().set_timestamp(start + duration);
    setup.escrow.execute_queued_release(&bounty_id);

    assert_eq!(
        setup.escrow.get_escrow_info(&bounty_id).status,
        EscrowStatus::Released
    );
}

#[test]
fn test_double_queue_same_bounty_rejected() {
    let setup = TestSetup::new();
    let bounty_id = 905;
    let threshold: i128 = 5_000;
    let deadline = setup.env.ledger().timestamp() + 10_000;

    setup.escrow.set_high_value_config(&threshold, &3_600);
    setup.token_admin.mint(&setup.depositor, &threshold);
    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &threshold, &deadline);
    setup.escrow.release_funds(&bounty_id, &setup.contributor);

    // Second call should fail with ReleaseAlreadyQueued.
    let res = setup.escrow.try_release_funds(&bounty_id, &setup.contributor);
    assert!(matches!(res, Err(Ok(Error::ReleaseAlreadyQueued))));
}

#[test]
fn test_cancel_queued_release_restores_locked_state() {
    let setup = TestSetup::new();
    let bounty_id = 906;
    let threshold: i128 = 5_000;
    let deadline = setup.env.ledger().timestamp() + 10_000;

    setup.escrow.set_high_value_config(&threshold, &3_600);
    setup.token_admin.mint(&setup.depositor, &threshold);
    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &threshold, &deadline);
    setup.escrow.release_funds(&bounty_id, &setup.contributor);

    // Cancel the queued release.
    setup.escrow.cancel_queued_release(&bounty_id);

    // Queue entry should be gone; escrow still Locked.
    assert!(setup.escrow.get_queued_release(&bounty_id).is_none());
    assert_eq!(
        setup.escrow.get_escrow_info(&bounty_id).status,
        EscrowStatus::Locked
    );
}

#[test]
fn test_cancel_queued_release_allows_re_queue() {
    let setup = TestSetup::new();
    let bounty_id = 907;
    let threshold: i128 = 5_000;
    let duration: u64 = 3_600;
    let deadline = setup.env.ledger().timestamp() + 10_000;

    setup.escrow.set_high_value_config(&threshold, &duration);
    setup.token_admin.mint(&setup.depositor, &threshold);
    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &threshold, &deadline);
    setup.escrow.release_funds(&bounty_id, &setup.contributor);

    // Cancel, then queue again.
    setup.escrow.cancel_queued_release(&bounty_id);
    setup.escrow.release_funds(&bounty_id, &setup.contributor);

    assert!(setup.escrow.get_queued_release(&bounty_id).is_some());
}

#[test]
fn test_execute_nonexistent_queued_release_fails() {
    let setup = TestSetup::new();
    let res = setup.escrow.try_execute_queued_release(&9999);
    assert!(matches!(res, Err(Ok(Error::BountyNotFound))));
}

#[test]
fn test_cancel_nonexistent_queued_release_fails() {
    let setup = TestSetup::new();
    let res = setup.escrow.try_cancel_queued_release(&9999);
    assert!(matches!(res, Err(Ok(Error::BountyNotFound))));
}

#[test]
fn test_get_queued_release_returns_none_when_not_queued() {
    let setup = TestSetup::new();
    let bounty_id = 908;
    let amount: i128 = 1_000;
    let deadline = setup.env.ledger().timestamp() + 1_000;

    setup.token_admin.mint(&setup.depositor, &amount);
    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    assert!(setup.escrow.get_queued_release(&bounty_id).is_none());
}

#[test]
fn test_high_value_config_not_set_releases_immediately() {
    let setup = TestSetup::new();
    let bounty_id = 909;
    let amount: i128 = 999_999;
    let deadline = setup.env.ledger().timestamp() + 1_000;

    // No high-value config set — any amount releases immediately.
    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.release_funds(&bounty_id, &setup.contributor);

    assert_eq!(
        setup.escrow.get_escrow_info(&bounty_id).status,
        EscrowStatus::Released
    );
}
