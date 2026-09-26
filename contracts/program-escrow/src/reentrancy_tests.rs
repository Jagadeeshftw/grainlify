#![cfg(test)]

//! Guard regression tests driven by a token that re-enters during `transfer`.
//!
//! Every test covers one guarded transfer boundary and performs two sequential
//! calls. The first call fails if its acquisition guard is missing; the second
//! fails if its success-path release is missing. Removing either half of any
//! guard therefore fails the uniquely named test for that entry point.

use crate::{
    malicious_reentrant::{ReentrantToken, ReentrantTokenClient, ReentryAction, ReentryParams},
    LockItem, ProgramEscrowContract, ProgramEscrowContractClient, ReleaseItem,
};
use soroban_sdk::{testutils::Address as _, vec, Address, Env, String};

const INITIAL_BALANCE: i128 = 10_000;
const OPERATION_AMOUNT: i128 = 1_000;

struct Harness<'a> {
    env: Env,
    admin: Address,
    recipient: Address,
    program_id: String,
    token: ReentrantTokenClient<'a>,
    escrow: ProgramEscrowContractClient<'a>,
}

impl<'a> Harness<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let recipient = Address::generate(&env);
        let program_id = String::from_str(&env, "reentrancy-guard");
        let token_address = env.register_contract(None, ReentrantToken);
        let token = ReentrantTokenClient::new(&env, &token_address);
        let escrow_address = env.register_contract(None, ProgramEscrowContract);
        let escrow = ProgramEscrowContractClient::new(&env, &escrow_address);

        token.mint(&admin, &INITIAL_BALANCE);
        escrow.init_program(
            &program_id,
            &admin,
            &token_address,
            &admin,
            &Some(INITIAL_BALANCE),
            &None,
        );
        escrow.publish_program(&program_id, &admin);

        Self {
            env,
            admin,
            recipient,
            program_id,
            token,
            escrow,
        }
    }

    fn arm(&self, action: ReentryAction) {
        self.token.configure_reentry(&self.escrow.address, &action);
    }

    fn params(&self, schedule_id: u64) -> ReentryParams {
        ReentryParams {
            program_id: self.program_id.clone(),
            recipient: self.recipient.clone(),
            amount: OPERATION_AMOUNT,
            schedule_id,
        }
    }
}

#[test]
fn test_batch_lock_guard_blocks_reentrant_token_transfer() {
    let h = Harness::new();
    h.escrow.update_fee_config(
        &Some(100),
        &None,
        &None,
        &None,
        &Some(h.admin.clone()),
        &Some(true),
        &Some(0),
    );
    h.arm(ReentryAction::BatchLock(h.params(0)));
    let items = vec![
        &h.env,
        LockItem {
            program_id: h.program_id.clone(),
            amount: OPERATION_AMOUNT,
        },
    ];

    assert!(
        h.escrow.try_batch_lock(&items).is_ok(),
        "batch_lock must hold its guard during the fee transfer"
    );
    assert!(
        h.escrow.try_batch_lock(&items).is_ok(),
        "batch_lock must release its guard after success"
    );
}

#[test]
fn test_batch_release_guard_blocks_reentrant_token_transfer() {
    let h = Harness::new();
    h.escrow
        .create_program_release_schedule(&h.recipient, &OPERATION_AMOUNT, &0);
    h.escrow
        .create_program_release_schedule(&h.recipient, &OPERATION_AMOUNT, &0);

    h.arm(ReentryAction::BatchRelease(h.params(1)));
    let first = vec![
        &h.env,
        ReleaseItem {
            program_id: h.program_id.clone(),
            schedule_id: 1,
        },
    ];
    let second = vec![
        &h.env,
        ReleaseItem {
            program_id: h.program_id.clone(),
            schedule_id: 2,
        },
    ];

    assert!(
        h.escrow.try_batch_release(&first).is_ok(),
        "batch_release must hold its guard during token transfer"
    );
    h.arm(ReentryAction::BatchRelease(h.params(2)));
    assert!(
        h.escrow.try_batch_release(&second).is_ok(),
        "batch_release must release its guard after success"
    );
}

#[test]
fn test_batch_payout_guard_blocks_reentrant_token_transfer() {
    let h = Harness::new();
    h.arm(ReentryAction::BatchPayout(h.params(0)));
    let recipients = vec![&h.env, h.recipient.clone()];
    let amounts = vec![&h.env, OPERATION_AMOUNT];

    assert!(
        h.escrow.try_batch_payout(&recipients, &amounts).is_ok(),
        "batch_payout must hold its guard during token transfer"
    );
    assert!(
        h.escrow.try_batch_payout(&recipients, &amounts).is_ok(),
        "batch_payout must release its guard after success"
    );
}

#[test]
fn test_single_payout_guard_blocks_reentrant_token_transfer() {
    let h = Harness::new();
    let second_recipient = Address::generate(&h.env);
    h.arm(ReentryAction::SinglePayout(h.params(0)));

    assert!(
        h.escrow
            .try_single_payout(&h.recipient, &OPERATION_AMOUNT, &None)
            .is_ok(),
        "single_payout must hold its guard during token transfer"
    );
    h.arm(ReentryAction::SinglePayout(ReentryParams {
        recipient: second_recipient.clone(),
        ..h.params(0)
    }));
    assert!(
        h.escrow
            .try_single_payout(&second_recipient, &OPERATION_AMOUNT, &None)
            .is_ok(),
        "single_payout must release its guard after success"
    );
}

#[test]
fn test_trigger_program_releases_guard_blocks_reentrant_token_transfer() {
    let h = Harness::new();
    h.escrow
        .create_program_release_schedule(&h.recipient, &OPERATION_AMOUNT, &0);
    h.arm(ReentryAction::TriggerProgramReleases(h.params(1)));

    assert_eq!(
        h.escrow
            .try_trigger_program_releases(&None)
            .unwrap()
            .unwrap(),
        1,
        "trigger_program_releases must hold its guard during token transfer"
    );
    h.escrow
        .create_program_release_schedule(&h.recipient, &OPERATION_AMOUNT, &0);
    h.arm(ReentryAction::TriggerProgramReleases(h.params(2)));
    assert_eq!(
        h.escrow
            .try_trigger_program_releases(&None)
            .unwrap()
            .unwrap(),
        1,
        "trigger_program_releases must release its guard after success"
    );
}
