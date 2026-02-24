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

#![cfg(test)]

use soroban_sdk::{contract, contractimpl, Address, Env, String};

/// Interface for the ProgramEscrow contract (simplified for testing)
pub trait ProgramEscrowTrait {
    fn single_payout(env: Env, recipient: Address, amount: i128);
    fn batch_payout(
        env: Env,
        recipients: soroban_sdk::Vec<Address>,
        amounts: soroban_sdk::Vec<i128>,
    );
    fn trigger_program_releases(env: Env) -> u32;
}

#[contract]
pub struct MaliciousReentrantContract;

#[contractimpl]
impl MaliciousReentrantContract {
    /// Initialize the malicious contract with the target escrow contract address
    pub fn init(env: Env, target_contract: Address) {
        env.storage()
            .instance()
            .set(&soroban_sdk::symbol_short!("Target"), &target_contract);
    }

    /// Get the target contract address
    pub fn get_target(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&soroban_sdk::symbol_short!("Target"))
            .unwrap()
    }

    /// Set attack mode
    /// - 0: No attack (normal behavior)
    /// - 1: Reenter on single_payout
    /// - 2: Reenter on batch_payout
    /// - 3: Reenter on trigger_releases
    pub fn set_attack_mode(env: Env, mode: u32) {
        env.storage()
            .instance()
            .set(&soroban_sdk::symbol_short!("AttackMd"), &mode);
    }

    /// Get current attack mode
    pub fn get_attack_mode(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&soroban_sdk::symbol_short!("AttackMd"))
            .unwrap_or(0)
    }

    /// Increment attack counter
    fn increment_attack_count(env: &Env) {
        let count: u32 = env
            .storage()
            .instance()
            .get(&soroban_sdk::symbol_short!("AttackCt"))
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&soroban_sdk::symbol_short!("AttackCt"), &(count + 1));
    }

    /// Get attack counter (how many times reentrancy was attempted)
    pub fn get_attack_count(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&soroban_sdk::symbol_short!("AttackCt"))
            .unwrap_or(0)
    }

    /// Reset attack counter
    pub fn reset_attack_count(env: Env) {
        env.storage()
            .instance()
            .set(&soroban_sdk::symbol_short!("AttackCt"), &0u32);
    }

    /// This function is called when the contract receives tokens
    /// It will attempt reentrancy based on the attack mode
    pub fn on_token_received(env: Env, _from: Address, amount: i128) {
        let attack_mode = Self::get_attack_mode(env.clone());

        // Only attack once to avoid infinite loops in tests
        let attack_count = Self::get_attack_count(env.clone());
        if attack_count > 0 {
            return;
        }

        Self::increment_attack_count(&env);

        match attack_mode {
            1 => {
                // Attack mode 1: Reenter single_payout
                let target = Self::get_target(env.clone());
                let attacker = env.current_contract_address();

                // Attempt to call single_payout again (reentrancy)
                // This should be blocked by the reentrancy guard
                let client = crate::ProgramEscrowContractClient::new(&env, &target);
                client.single_payout(&attacker, &amount);
            }
            2 => {
                // Attack mode 2: Reenter batch_payout
                let target = Self::get_target(env.clone());
                let attacker = env.current_contract_address();

                let recipients = soroban_sdk::vec![&env, attacker.clone()];
                let amounts = soroban_sdk::vec![&env, amount];

                // Attempt to call batch_payout again (reentrancy)
                let client = crate::ProgramEscrowContractClient::new(&env, &target);
                client.batch_payout(&recipients, &amounts);
            }
            3 => {
                // Attack mode 3: Reenter trigger_program_releases
                let target = Self::get_target(env.clone());

                // Attempt to trigger releases again (reentrancy)
                let client = crate::ProgramEscrowContractClient::new(&env, &target);
                client.trigger_program_releases();
            }
            _ => {
                // No attack, normal behavior
            }
        }
    }

    /// Attempt direct reentrancy attack on single_payout
    pub fn attack_single_payout(env: Env, recipient: Address, amount: i128) {
        let target = Self::get_target(env.clone());
        Self::increment_attack_count(&env);

        let client = crate::ProgramEscrowContractClient::new(&env, &target);
        client.single_payout(&recipient, &amount);
    }

    /// Attempt direct reentrancy attack on batch_payout
    pub fn attack_batch_payout(
        env: Env,
        recipients: soroban_sdk::Vec<Address>,
        amounts: soroban_sdk::Vec<i128>,
    ) {
        let target = Self::get_target(env.clone());
        Self::increment_attack_count(&env);

        let client = crate::ProgramEscrowContractClient::new(&env, &target);
        client.batch_payout(&recipients, &amounts);
    }

    /// Attempt nested call during execution
    pub fn nested_attack(env: Env) {
        let target = Self::get_target(env.clone());
        let attacker = env.current_contract_address();
        let amount = 1000i128;

        Self::increment_attack_count(&env);

        // First call
        let client = crate::ProgramEscrowContractClient::new(&env, &target);

        // Set attack mode to trigger reentrancy on callback
        Self::set_attack_mode(env.clone(), 1);

        // This should trigger the callback which will attempt reentrancy
        client.single_payout(&attacker, &amount);
    }
}
