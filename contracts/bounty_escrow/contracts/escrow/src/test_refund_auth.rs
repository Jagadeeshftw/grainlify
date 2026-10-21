//! Authorization path test suite for `refund` (contracts/escrow/src/lib.rs:1724 / src/refund.rs:389).
//!
//! Validates:
//! 1. A caller without authorization is refused with `Error(Auth, InvalidAction)`.
//! 2. Missing either admin or depositor authorization is refused with `Error(Auth, InvalidAction)`.
//! 3. The refusal names the specific error rather than any generic error.
//! 4. No contract state or token balances are mutated by the refused call.
//! 5. The authorized caller succeeds unchanged after an unauthorized attempt.
//! 6. Admin approval before deadline still requires proper authorization and does not mutate state when refused.

#![cfg(test)]
extern crate std;

use crate::{
    BountyEscrowContract, BountyEscrowContractClient, DataKey, Error, EscrowStatus, RefundMode,
};
use soroban_sdk::{
    testutils::{Address as _, Ledger, MockAuth, MockAuthInvoke},
    token, Address, Env, IntoVal,
};

struct TestSetup<'a> {
    env: Env,
    admin: Address,
    depositor: Address,
    unauthorized: Address,
    token: token::Client<'a>,
    escrow: BountyEscrowContractClient<'a>,
}

impl<'a> TestSetup<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let unauthorized = Address::generate(&env);

        let sac = env.register_stellar_asset_contract_v2(admin.clone());
        let token = token::Client::new(&env, &sac.address());
        let token_admin = token::StellarAssetClient::new(&env, &sac.address());
        let escrow = BountyEscrowContractClient::new(
            &env,
            &env.register_contract(None, BountyEscrowContract),
        );

        escrow.init(&admin, &token.address);
        token_admin.mint(&depositor, &1_000_000);

        Self {
            env,
            admin,
            depositor,
            unauthorized,
            token,
            escrow,
        }
    }

    fn lock_bounty(&self, bounty_id: u64, amount: i128, deadline: u64) {
        self.escrow
            .lock_funds(&self.depositor, &bounty_id, &amount, &deadline);
    }
}

/// A caller without authorisation (unauthorized third party) is refused.
/// The refusal specifically names `Error(Auth, InvalidAction)`.
#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn test_refund_unauthorized_caller_refused() {
    let s = TestSetup::new();
    let bounty_id = 1u64;
    let amount = 1_000i128;
    let deadline = s.env.ledger().timestamp() + 100;
    s.lock_bounty(bounty_id, amount, deadline);

    // Advance time past deadline so refund would otherwise be valid
    s.env.ledger().set_timestamp(deadline + 1);

    // Mock authorization exclusively for the unauthorized address
    s.env.mock_auths(&[MockAuth {
        address: &s.unauthorized,
        invoke: &MockAuthInvoke {
            contract: &s.escrow.address,
            fn_name: "refund",
            args: (bounty_id,).into_val(&s.env),
            sub_invokes: &[],
        },
    }]);

    s.escrow.refund(&bounty_id);
}

/// A call with no authorisation mocked at all is refused.
/// The refusal specifically names `Error(Auth, InvalidAction)`.
#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn test_refund_missing_all_authorisation_refused() {
    let s = TestSetup::new();
    let bounty_id = 2u64;
    let amount = 2_000i128;
    let deadline = s.env.ledger().timestamp() + 100;
    s.lock_bounty(bounty_id, amount, deadline);

    s.env.ledger().set_timestamp(deadline + 1);

    // No authorisations provided
    s.env.mock_auths(&[]);

    s.escrow.refund(&bounty_id);
}

/// When only the admin authorizes, but the depositor does not authorize,
/// the refund call is refused specifically naming `Error(Auth, InvalidAction)`.
#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn test_refund_missing_depositor_authorisation_refused() {
    let s = TestSetup::new();
    let bounty_id = 3u64;
    let amount = 3_000i128;
    let deadline = s.env.ledger().timestamp() + 100;
    s.lock_bounty(bounty_id, amount, deadline);

    s.env.ledger().set_timestamp(deadline + 1);

    // Only admin authorizes; depositor auth is missing
    s.env.mock_auths(&[MockAuth {
        address: &s.admin,
        invoke: &MockAuthInvoke {
            contract: &s.escrow.address,
            fn_name: "refund",
            args: (bounty_id,).into_val(&s.env),
            sub_invokes: &[],
        },
    }]);

    s.escrow.refund(&bounty_id);
}

/// When only the depositor authorizes, but the admin does not authorize,
/// the refund call is refused specifically naming `Error(Auth, InvalidAction)`.
#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn test_refund_missing_admin_authorisation_refused() {
    let s = TestSetup::new();
    let bounty_id = 4u64;
    let amount = 4_000i128;
    let deadline = s.env.ledger().timestamp() + 100;
    s.lock_bounty(bounty_id, amount, deadline);

    s.env.ledger().set_timestamp(deadline + 1);

    // Only depositor authorizes; admin auth is missing
    s.env.mock_auths(&[MockAuth {
        address: &s.depositor,
        invoke: &MockAuthInvoke {
            contract: &s.escrow.address,
            fn_name: "refund",
            args: (bounty_id,).into_val(&s.env),
            sub_invokes: &[],
        },
    }]);

    s.escrow.refund(&bounty_id);
}

/// Comprehensive test verifying:
/// 1. Unauthorized call is refused and panics.
/// 2. No state is mutated by the refused call (escrow, balances, refund history).
/// 3. The authorized caller succeeds unchanged afterwards.
#[test]
fn test_refund_unauthorized_call_mutates_no_state_and_authorized_caller_succeeds() {
    let s = TestSetup::new();
    let bounty_id = 5u64;
    let amount = 5_000i128;
    let deadline = s.env.ledger().timestamp() + 100;
    s.lock_bounty(bounty_id, amount, deadline);

    s.env.ledger().set_timestamp(deadline + 1);

    // Snapshot pre-refusal state
    let escrow_before = s.escrow.get_escrow_info(&bounty_id);
    let depositor_balance_before = s.token.balance(&s.depositor);
    let escrow_balance_before = s.token.balance(&s.escrow.address);
    let admin_balance_before = s.token.balance(&s.admin);
    let unauthorized_balance_before = s.token.balance(&s.unauthorized);

    assert_eq!(escrow_before.status, EscrowStatus::Locked);
    assert_eq!(escrow_before.remaining_amount, amount);
    assert_eq!(escrow_before.refund_history.len(), 0);
    assert_eq!(escrow_balance_before, amount);

    // Mock only the unauthorized caller
    s.env.mock_auths(&[MockAuth {
        address: &s.unauthorized,
        invoke: &MockAuthInvoke {
            contract: &s.escrow.address,
            fn_name: "refund",
            args: (bounty_id,).into_val(&s.env),
            sub_invokes: &[],
        },
    }]);

    // Attempt refund with unauthorized caller
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        s.escrow.refund(&bounty_id);
    }));

    // Must be refused (panic)
    assert!(result.is_err(), "Unauthorized refund must be refused");

    // Re-enable mock_all_auths to read views and verify state is completely untouched
    s.env.mock_all_auths();

    let escrow_after = s.escrow.get_escrow_info(&bounty_id);
    assert_eq!(
        escrow_after.status,
        EscrowStatus::Locked,
        "Escrow status must remain Locked"
    );
    assert_eq!(
        escrow_after.remaining_amount, amount,
        "Remaining amount must remain unchanged"
    );
    assert_eq!(
        escrow_after.refund_history.len(),
        0,
        "Refund history must not record refused attempts"
    );
    assert_eq!(
        s.token.balance(&s.depositor),
        depositor_balance_before,
        "Depositor balance must not change"
    );
    assert_eq!(
        s.token.balance(&s.escrow.address),
        escrow_balance_before,
        "Escrow balance must not change"
    );
    assert_eq!(
        s.token.balance(&s.admin),
        admin_balance_before,
        "Admin balance must not change"
    );
    assert_eq!(
        s.token.balance(&s.unauthorized),
        unauthorized_balance_before,
        "Unauthorized caller balance must not change"
    );

    // Now invoke refund with authorized caller; it must succeed unchanged
    s.escrow.refund(&bounty_id);

    let escrow_final = s.escrow.get_escrow_info(&bounty_id);
    assert_eq!(
        escrow_final.status,
        EscrowStatus::Refunded,
        "Authorized refund must transition escrow to Refunded"
    );
    assert_eq!(
        escrow_final.remaining_amount, 0,
        "Authorized refund must deplete remaining amount"
    );
    assert_eq!(
        escrow_final.refund_history.len(),
        1,
        "Authorized refund must add one refund record"
    );
    assert_eq!(
        s.token.balance(&s.depositor),
        depositor_balance_before + amount,
        "Depositor must receive full refund amount"
    );
    assert_eq!(
        s.token.balance(&s.escrow.address),
        0,
        "Escrow contract token balance must be 0"
    );
}

/// An administrative approval before deadline still refuses an unauthorized caller,
/// leaves approval unconsumed and state unchanged, and succeeds for authorized callers.
#[test]
fn test_refund_with_approval_unauthorized_caller_refused_leaves_approval_intact() {
    let s = TestSetup::new();
    let bounty_id = 6u64;
    let amount = 6_000i128;
    let deadline = s.env.ledger().timestamp() + 500;
    s.lock_bounty(bounty_id, amount, deadline);

    // Admin approves refund before deadline
    s.escrow
        .approve_refund(&bounty_id, &amount, &s.depositor, &RefundMode::Full);

    // Verify approval is in persistent storage
    let approval_present = s.env.as_contract(&s.escrow.address, || {
        s.env
            .storage()
            .persistent()
            .has(&DataKey::RefundApproval(bounty_id))
    });
    assert!(approval_present, "Approval must exist before refund");

    let depositor_balance_before = s.token.balance(&s.depositor);
    let escrow_balance_before = s.token.balance(&s.escrow.address);

    // Mock only unauthorized caller
    s.env.mock_auths(&[MockAuth {
        address: &s.unauthorized,
        invoke: &MockAuthInvoke {
            contract: &s.escrow.address,
            fn_name: "refund",
            args: (bounty_id,).into_val(&s.env),
            sub_invokes: &[],
        },
    }]);

    // Unauthorized caller attempts refund
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        s.escrow.refund(&bounty_id);
    }));
    assert!(
        result.is_err(),
        "Unauthorized refund with approval must still be refused"
    );

    // Verify state and approval remain unchanged
    s.env.mock_all_auths();
    let escrow_after_refused = s.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow_after_refused.status, EscrowStatus::Locked);
    assert_eq!(escrow_after_refused.remaining_amount, amount);
    assert_eq!(s.token.balance(&s.depositor), depositor_balance_before);
    assert_eq!(s.token.balance(&s.escrow.address), escrow_balance_before);

    let approval_still_present = s.env.as_contract(&s.escrow.address, || {
        s.env
            .storage()
            .persistent()
            .has(&DataKey::RefundApproval(bounty_id))
    });
    assert!(
        approval_still_present,
        "Approval must not be consumed by refused call"
    );

    // Authorized refund succeeds and consumes approval
    s.escrow.refund(&bounty_id);

    let escrow_final = s.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow_final.status, EscrowStatus::Refunded);
    assert_eq!(escrow_final.remaining_amount, 0);
    assert_eq!(
        s.token.balance(&s.depositor),
        depositor_balance_before + amount
    );
    assert_eq!(s.token.balance(&s.escrow.address), 0);

    let approval_consumed = s.env.as_contract(&s.escrow.address, || {
        s.env
            .storage()
            .persistent()
            .has(&DataKey::RefundApproval(bounty_id))
    });
    assert!(
        !approval_consumed,
        "Approval must be consumed by authorized refund"
    );
}
