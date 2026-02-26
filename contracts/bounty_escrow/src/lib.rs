use crate::feature_flags::{is_enabled};
use soroban_sdk::{contractimpl, Env, Symbol, Address};

pub struct BountyEscrowContract;

#[contractimpl]
impl BountyEscrowContract {
    pub fn payout(e: Env, to: Address, amount: i128) {
        let flag_key = Symbol::new(&e, "gradual_payout_rollout");

        if is_enabled(&e, flag_key) {
            // NEW LOGIC: e.g., 2-step verification or different fee
            Self::execute_new_payout_logic(&e, to, amount);
        } else {
            // LEGACY LOGIC: Direct transfer
            Self::execute_legacy_payout(&e, to, amount);
        }
    }
}