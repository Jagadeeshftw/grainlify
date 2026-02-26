
//! # Malicious Reentrant Contract
//!
//! This is a test-only contract that attempts to perform reentrancy attacks
//! on the ProgramEscrow contract. It's used to verify that reentrancy guards
//! are working correctly.
//!
//! ## Attack Scenarios
//!
//! 1. **Payout Callback Attack**: When receiving a payout, immediately call
//!    back into the escrow contract to request another payout
//! 2. **Nested Batch Attack**: During a batch payout, attempt to trigger
//!    another batch payout
//! 3. **Schedule Release Attack**: During schedule release, attempt to
//!    release another schedule or modify state
//! 4. **Cross-Contract Chain**: Chain multiple malicious contracts together
//! 5. **Nested Depth Attack**: Attempt reentrancy at multiple depth levels

#![cfg(test)]

use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, Vec};

/// Interface for the ProgramEscrow contract (simplified for testing)
pub trait ProgramEscrowTrait {
    fn single_payout(env: Env, recipient: Address, amount: i128);
    fn batch_payout(env: Env, recipients: Vec<Address>, amounts: Vec<i128>);
    fn trigger_program_releases(env: Env) -> u32;
}

/// Attack modes for the malicious contract
#[contracttype]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AttackMode {
    /// No attack (normal behavior)
    None = 0,
    /// Reenter on single_payout
    SinglePayoutReentrant = 1,
    /// Reenter on batch_payout
    BatchPayoutReentrant = 2,
    /// Reenter on trigger_releases
    TriggerReleasesReentrant = 3,
    /// Nested reentrancy (multiple levels deep)
    NestedReentrant = 4,
    /// Cross-contract chain reentrancy
    ChainReentrant = 5,
    /// Cross-function reentrancy (single -> batch)
    CrossFunctionSingleToBatch = 6,
    /// Cross-function reentrancy (batch -> single)
    CrossFunctionBatchToSingle = 7,
}

impl AttackMode {
    pub fn from_u32(value: u32) -> Self {
        match value {
            1 => AttackMode::SinglePayoutReentrant,
            2 => AttackMode::BatchPayoutReentrant,
            3 => AttackMode::TriggerReleasesReentrant,
            4 => AttackMode::NestedReentrant,
            5 => AttackMode::ChainReentrant,
            6 => AttackMode::CrossFunctionSingleToBatch,
            7 => AttackMode::CrossFunctionBatchToSingle,
            _ => AttackMode::None,
        }
    }

    pub fn to_u32(&self) -> u32 {
        *self as u32
    }
}

#[contract]
pub struct MaliciousReentrantContract;

#[contractimpl]
impl MaliciousReentrantContract {
    /// Initialize the malicious contract with the target escrow contract address
    pub fn init(env: Env, target_contract: Address) {
        env.storage()
            .instance()
            .set(&symbol_short!("TARGET"), &target_contract);
    }

    /// Get the target contract address
    pub fn get_target(env: &Env) -> Address {
        env.storage()
            .instance()
            .get(&symbol_short!("TARGET"))
            .unwrap()
    }

    /// Set attack mode
    pub fn set_attack_mode(env: &Env, mode: AttackMode) {
        env.storage()
            .instance()
            .set(&symbol_short!("MODE"), &mode.to_u32());
    }

    /// Get current attack mode
    pub fn get_attack_mode(env: &Env) -> AttackMode {
        let mode: u32 = env
            .storage()
            .instance()
            .get(&symbol_short!("MODE"))
            .unwrap_or(0);
        AttackMode::from_u32(mode)
    }

    /// Set next contract in chain (for chain attacks)
    pub fn set_next_contract(env: &Env, next_contract: Address) {
        env.storage()
            .instance()
            .set(&symbol_short!("NEXT"), &next_contract);
    }

    /// Get next contract in chain
    pub fn get_next_contract(env: &Env) -> Option<Address> {
        env.storage().instance().get(&symbol_short!("NEXT"))
    }

    /// Set nested attack depth
    pub fn set_nested_depth(env: &Env, depth: u32) {
        env.storage()
            .instance()
            .set(&symbol_short!("DEPTH"), &depth);
    }

    /// Get nested attack depth
    pub fn get_nested_depth(env: &Env) -> u32 {
        env.storage()
            .instance()
            .get(&symbol_short!("DEPTH"))
            .unwrap_or(1)
    }

    /// Get current recursion depth for nested attacks
    fn get_current_depth(env: &Env) -> u32 {
        env.storage()
            .instance()
            .get(&symbol_short!("CURDEPTH"))
            .unwrap_or(0)
    }

    /// Set current recursion depth
    fn set_current_depth(env: &Env, depth: u32) {
        env.storage()
            .instance()
            .set(&symbol_short!("CURDEPTH"), &depth);
    }

    /// Increment attack counter
    fn increment_attack_count(env: &Env) {
        let count: u32 = env
            .storage()
            .instance()
            .get(&symbol_short!("COUNT"))
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&symbol_short!("COUNT"), &(count + 1));
    }

    /// Get attack counter (how many times reentrancy was attempted)
    pub fn get_attack_count(env: &Env) -> u32 {
        env.storage()
            .instance()
            .get(&symbol_short!("COUNT"))
            .unwrap_or(0)
    }

    /// Reset attack counter
    pub fn reset_attack_count(env: &Env) {
        env.storage().instance().set(&symbol_short!("COUNT"), &0u32);
        env.storage()
            .instance()
            .set(&symbol_short!("CURDEPTH"), &0u32);
    }

    /// This function is called when the contract receives tokens
    /// It will attempt reentrancy based on the attack mode
    pub fn on_token_received(env: Env, _from: Address, amount: i128) {
        let attack_mode = Self::get_attack_mode(&env);

        // Only attack if we haven't exceeded max attempts
        let attack_count = Self::get_attack_count(&env);
        let max_depth = Self::get_nested_depth(&env);

        if attack_count >= max_depth {
            return;
        }

        Self::increment_attack_count(&env);

        match attack_mode {
            AttackMode::SinglePayoutReentrant => {
                Self::attempt_single_payout_reentrancy(&env, amount);
            }
            AttackMode::BatchPayoutReentrant => {
                Self::attempt_batch_payout_reentrancy(&env, amount);
            }
            AttackMode::TriggerReleasesReentrant => {
                Self::attempt_trigger_releases_reentrancy(&env);
            }
            AttackMode::NestedReentrant => {
                Self::attempt_nested_reentrancy(&env, amount);
            }
            AttackMode::ChainReentrant => {
                Self::attempt_chain_reentrancy(&env, amount);
            }
            AttackMode::CrossFunctionSingleToBatch => {
                Self::attempt_cross_function_single_to_batch(&env, amount);
            }
            AttackMode::CrossFunctionBatchToSingle => {
                Self::attempt_cross_function_batch_to_single(&env, amount);
            }
            AttackMode::None => {
                // No attack, normal behavior
            }
        }
    }

    /// Attempt reentrancy on single_payout
    fn attempt_single_payout_reentrancy(env: &Env, amount: i128) {
        let target = Self::get_target(env);
        let attacker = env.current_contract_address();

        // This should be blocked by the reentrancy guard
        let client = crate::ProgramEscrowContractClient::new(env, &target);
        client.single_payout(&attacker, &amount);
    }

    /// Attempt reentrancy on batch_payout
    fn attempt_batch_payout_reentrancy(env: &Env, amount: i128) {
        let target = Self::get_target(env);
        let attacker = env.current_contract_address();

        let recipients = Vec::from_array(env, [attacker.clone()]);
        let amounts = Vec::from_array(env, [amount]);

        let client = crate::ProgramEscrowContractClient::new(env, &target);
        client.batch_payout(&recipients, &amounts);
    }

    /// Attempt reentrancy on trigger_program_releases
    fn attempt_trigger_releases_reentrancy(env: &Env) {
        let target = Self::get_target(env);

        let client = crate::ProgramEscrowContractClient::new(env, &target);
        client.trigger_program_releases();
    }

    /// Attempt nested reentrancy with depth tracking
    fn attempt_nested_reentrancy(env: &Env, amount: i128) {
        let target = Self::get_target(env);
        let attacker = env.current_contract_address();

        // Track current depth
        let current_depth = Self::get_current_depth(env);
        Self::set_current_depth(env, current_depth + 1);

        // Call single_payout which will trigger on_token_received again
        let client = crate::ProgramEscrowContractClient::new(env, &target);
        client.single_payout(&attacker, &amount);
    }

    /// Attempt chain reentrancy through multiple contracts
    fn attempt_chain_reentrancy(env: &Env, amount: i128) {
        // For now, reuse the primary single-payout reentrancy path. Tests that
        // care about cross-contract chains can deploy multiple malicious
        // instances and configure their targets accordingly, without requiring
        // a dedicated client type here.
        let _ = Self::get_next_contract(env);
        Self::attempt_single_payout_reentrancy(env, amount);
    }

    /// Attempt cross-function reentrancy: single_payout -> batch_payout
    fn attempt_cross_function_single_to_batch(env: &Env, amount: i128) {
        let target = Self::get_target(env);
        let attacker = env.current_contract_address();

        // Instead of calling single_payout again, try batch_payout
        let recipients = Vec::from_array(env, [attacker]);
        let amounts = Vec::from_array(env, [amount]);

        let client = crate::ProgramEscrowContractClient::new(env, &target);
        client.batch_payout(&recipients, &amounts);
    }

    /// Attempt cross-function reentrancy: batch_payout -> single_payout
    fn attempt_cross_function_batch_to_single(env: &Env, amount: i128) {
        let target = Self::get_target(env);
        let attacker = env.current_contract_address();

        // Instead of calling batch_payout again, try single_payout
        let client = crate::ProgramEscrowContractClient::new(env, &target);
        client.single_payout(&attacker, &amount);
    }

    /// Public function to start a single_payout attack
    pub fn attack_single_payout(env: Env, recipient: Address, amount: i128) {
        let target = Self::get_target(&env);
        Self::reset_attack_count(&env);
        Self::set_attack_mode(&env, AttackMode::SinglePayoutReentrant);

        let client = crate::ProgramEscrowContractClient::new(&env, &target);
        client.single_payout(&recipient, &amount);
    }

    /// Public function to start a batch_payout attack
    pub fn attack_batch_payout(env: Env, recipients: Vec<Address>, amounts: Vec<i128>) {
        let target = Self::get_target(&env);
        Self::reset_attack_count(&env);
        Self::set_attack_mode(&env, AttackMode::BatchPayoutReentrant);

        let client = crate::ProgramEscrowContractClient::new(&env, &target);
        client.batch_payout(&recipients, &amounts);
    }

    /// Public function to start a nested attack
    pub fn attack_nested(env: Env, recipient: Address, amount: i128, depth: u32) {
        let target = Self::get_target(&env);
        Self::reset_attack_count(&env);
        Self::set_attack_mode(&env, AttackMode::NestedReentrant);
        Self::set_nested_depth(&env, depth);

        let client = crate::ProgramEscrowContractClient::new(&env, &target);
        client.single_payout(&recipient, &amount);
    }

    /// Public function to start a chain attack
    pub fn start_chain_attack(env: Env, recipient: Address, amount: i128) {
        let target = Self::get_target(&env);
        Self::reset_attack_count(&env);
        Self::set_attack_mode(&env, AttackMode::ChainReentrant);

        let client = crate::ProgramEscrowContractClient::new(&env, &target);
        client.single_payout(&recipient, &amount);
    }

    /// Public function to start a cross-function attack
    pub fn attack_cross_function(
        env: Env,
        recipient: Address,
        amount: i128,
        from_single_to_batch: bool,
    ) {
        let target = Self::get_target(&env);
        Self::reset_attack_count(&env);

        let mode = if from_single_to_batch {
            AttackMode::CrossFunctionSingleToBatch
        } else {
            AttackMode::CrossFunctionBatchToSingle
        };
        Self::set_attack_mode(&env, mode);

        let client = crate::ProgramEscrowContractClient::new(&env, &target);
        client.single_payout(&recipient, &amount);
    }
}
