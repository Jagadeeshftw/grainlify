#![cfg(test)]

//! Test-only SEP-41 token that re-enters `program-escrow` from `transfer`.
//!
//! Soroban rejects a direct call back into an instance already on the call
//! stack before the second contract body runs, and frame-local guard writes are
//! not visible from that nested frame. Each guarded transfer call site therefore
//! brackets each `transfer` with the test-only [`assert_escrow_guard`] marker;
//! the token then exercises the real same-instance reentry boundary with `try_*`.
//! The markers make guard acquisition and lifetime observable so the host alone
//! cannot let a test pass after a guard call site was removed.

use crate::{LockItem, ProgramEscrowContractClient, ReleaseItem};
use soroban_sdk::{contract, contractimpl, contracttype, vec, Address, Env, String};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReentryParams {
    pub program_id: String,
    pub recipient: Address,
    pub amount: i128,
    pub schedule_id: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReentryAction {
    None,
    BatchLock(ReentryParams),
    BatchRelease(ReentryParams),
    BatchPayout(ReentryParams),
    SinglePayout(ReentryParams),
    TriggerProgramReleases(ReentryParams),
}

#[contracttype]
enum TokenKey {
    Balance(Address),
    Escrow,
    Action,
}

pub fn assert_escrow_guard(env: &Env) {
    assert!(
        crate::reentrancy_guard::is_entered(env),
        "program-escrow guard was not held during token transfer"
    );
}

#[contract]
pub struct ReentrantToken;

#[contractimpl]
impl ReentrantToken {
    pub fn configure_reentry(env: Env, escrow: Address, action: ReentryAction) {
        env.storage().instance().set(&TokenKey::Escrow, &escrow);
        env.storage().instance().set(&TokenKey::Action, &action);
    }

    pub fn mint(env: Env, to: Address, amount: i128) {
        let balance: i128 = env
            .storage()
            .persistent()
            .get(&TokenKey::Balance(to.clone()))
            .unwrap_or(0);
        env.storage()
            .persistent()
            .set(&TokenKey::Balance(to), &(balance + amount));
    }

    pub fn balance(env: Env, owner: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&TokenKey::Balance(owner))
            .unwrap_or(0)
    }

    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        from.require_auth();

        let from_balance: i128 = env
            .storage()
            .persistent()
            .get(&TokenKey::Balance(from.clone()))
            .unwrap_or(0);
        assert!(
            from_balance >= amount,
            "ReentrantToken: insufficient balance"
        );

        let to_balance: i128 = env
            .storage()
            .persistent()
            .get(&TokenKey::Balance(to.clone()))
            .unwrap_or(0);
        env.storage()
            .persistent()
            .set(&TokenKey::Balance(from), &(from_balance - amount));
        env.storage()
            .persistent()
            .set(&TokenKey::Balance(to), &(to_balance + amount));

        let action: ReentryAction = env
            .storage()
            .instance()
            .get(&TokenKey::Action)
            .unwrap_or(ReentryAction::None);
        if matches!(&action, ReentryAction::None) {
            return;
        }

        let escrow: Address = env.storage().instance().get(&TokenKey::Escrow).unwrap();
        let client = ProgramEscrowContractClient::new(&env, &escrow);
        let callback_rejected = match action {
            ReentryAction::None => unreachable!(),
            ReentryAction::BatchLock(params) => {
                let items = vec![
                    &env,
                    LockItem {
                        program_id: params.program_id,
                        amount: params.amount,
                    },
                ];
                client.try_batch_lock(&items).is_err()
            }
            ReentryAction::BatchRelease(params) => {
                let items = vec![
                    &env,
                    ReleaseItem {
                        program_id: params.program_id,
                        schedule_id: params.schedule_id,
                    },
                ];
                client.try_batch_release(&items).is_err()
            }
            ReentryAction::BatchPayout(params) => {
                let recipients = vec![&env, params.recipient];
                let amounts = vec![&env, params.amount];
                client.try_batch_payout(&recipients, &amounts).is_err()
            }
            ReentryAction::SinglePayout(params) => {
                let idempotency_key: Option<String> = None;
                client
                    .try_single_payout(&params.recipient, &params.amount, &idempotency_key)
                    .is_err()
            }
            ReentryAction::TriggerProgramReleases(_) => {
                client.try_trigger_program_releases(&None).is_err()
            }
        };

        assert!(
            callback_rejected,
            "Soroban host must reject same-instance token reentry"
        );
    }
}
