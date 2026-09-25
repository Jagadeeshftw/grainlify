//! Trait implementations for [`BountyEscrowContract`].
//!
//! Implements [`crate::traits::EscrowInterface`], [`crate::traits::UpgradeInterface`],
//! [`crate::traits::PauseInterface`], and [`crate::traits::FeeInterface`] by delegating
//! to the corresponding `#[contractimpl]` entry points.
//!
//! This module is included in `lib.rs` via `include!(...)` so it shares the parent
//! namespace without requiring an extra module qualifier.
//! All types and imports from lib.rs are directly available.

impl traits::EscrowInterface for BountyEscrowContract {
    /// Lock funds for a bounty through the trait interface
    fn lock_funds(
        env: &Env,
        depositor: Address,
        bounty_id: u64,
        amount: i128,
        deadline: u64,
    ) -> Result<(), crate::Error> {
        let entrypoint: fn(Env, Address, u64, i128, u64) -> Result<(), crate::Error> =
            BountyEscrowContract::lock_funds;
        entrypoint(env.clone(), depositor, bounty_id, amount, deadline)
    }

    /// Release funds to contributor through the trait interface
    fn release_funds(env: &Env, bounty_id: u64, contributor: Address) -> Result<(), crate::Error> {
        crate::claims::validate_claim_window(env.clone(), bounty_id)?;
        let entrypoint: fn(Env, u64, Address) -> Result<(), crate::Error> =
            BountyEscrowContract::release_funds;
        entrypoint(env.clone(), bounty_id, contributor)
    }

    /// Partial release through the trait interface
    fn partial_release(
        env: &Env,
        bounty_id: u64,
        contributor: Address,
        payout_amount: i128,
    ) -> Result<(), crate::Error> {
        let entrypoint: fn(Env, u64, Address, i128) -> Result<(), crate::Error> =
            BountyEscrowContract::partial_release;
        entrypoint(env.clone(), bounty_id, contributor, payout_amount)
    }

    /// Batch lock funds through the trait interface
    fn batch_lock_funds(env: &Env, items: Vec<LockFundsItem>) -> Result<u32, crate::Error> {
        let entrypoint: fn(Env, Vec<LockFundsItem>) -> Result<u32, crate::Error> =
            BountyEscrowContract::batch_lock_funds;
        entrypoint(env.clone(), items)
    }

    /// Batch release funds through the trait interface
    fn batch_release_funds(env: &Env, items: Vec<ReleaseFundsItem>) -> Result<u32, crate::Error> {
        let entrypoint: fn(Env, Vec<ReleaseFundsItem>) -> Result<u32, crate::Error> =
            BountyEscrowContract::batch_release_funds;
        entrypoint(env.clone(), items)
    }

    /// Refund funds to depositor through the trait interface
    fn refund(env: &Env, bounty_id: u64) -> Result<(), crate::Error> {
        let entrypoint: fn(Env, u64) -> Result<(), crate::Error> = BountyEscrowContract::refund;
        entrypoint(env.clone(), bounty_id)
    }

    /// Get escrow information through the trait interface
    fn get_escrow_info(env: &Env, bounty_id: u64) -> Result<crate::Escrow, crate::Error> {
        let entrypoint: fn(Env, u64) -> Result<crate::Escrow, crate::Error> =
            BountyEscrowContract::get_escrow_info;
        entrypoint(env.clone(), bounty_id)
    }

    /// Get contract balance through the trait interface
    fn get_balance(env: &Env) -> Result<i128, crate::Error> {
        let token_addr: Address = env
            .storage()
            .instance()
            .get(&DataKey::Token)
            .ok_or(Error::NotInitialized)?;
        let client = token::Client::new(env, &token_addr);
        Ok(client.balance(&env.current_contract_address()))
    }
}

impl traits::UpgradeInterface for BountyEscrowContract {
    /// Get contract version
    fn get_version(env: &Env) -> u32 {
        let entrypoint: fn(Env) -> u32 = BountyEscrowContract::get_version;
        entrypoint(env.clone())
    }

    /// Set contract version (admin only)
    fn set_version(env: &Env, new_version: u32) -> Result<(), crate::Error> {
        let entrypoint: fn(Env, u32) -> Result<(), crate::Error> =
            BountyEscrowContract::set_version;
        entrypoint(env.clone(), new_version)
    }
}

impl traits::PauseInterface for BountyEscrowContract {
    fn set_paused(
        env: &Env,
        lock: Option<bool>,
        release: Option<bool>,
        refund: Option<bool>,
        reason: Option<soroban_sdk::String>,
    ) -> Result<(), crate::Error> {
        #[allow(clippy::type_complexity)]
        let entrypoint: fn(
            Env,
            Option<bool>,
            Option<bool>,
            Option<bool>,
            Option<soroban_sdk::String>,
        ) -> Result<(), crate::Error> = BountyEscrowContract::set_paused;
        entrypoint(env.clone(), lock, release, refund, reason)
    }

    fn get_pause_flags(env: &Env) -> crate::PauseFlags {
        env.storage()
            .instance()
            .get(&DataKey::PauseFlags)
            .unwrap_or(PauseFlags {
                lock_paused: false,
                release_paused: false,
                refund_paused: false,
                pause_reason: None,
                paused_at: 0,
            })
    }

    fn is_operation_paused(env: &Env, operation: soroban_sdk::Symbol) -> bool {
        crate::pause_freeze::check_paused(env, operation)
    }
}

impl traits::FeeInterface for BountyEscrowContract {
    fn update_fee_config(
        env: &Env,
        lock_fee_rate: Option<i128>,
        release_fee_rate: Option<i128>,
        lock_fixed_fee: Option<i128>,
        release_fixed_fee: Option<i128>,
        fee_recipient: Option<Address>,
        fee_enabled: Option<bool>,
    ) -> Result<(), crate::Error> {
        #[allow(clippy::type_complexity)]
        let entrypoint: fn(
            Env,
            Option<i128>,
            Option<i128>,
            Option<i128>,
            Option<i128>,
            Option<Address>,
            Option<bool>,
        ) -> Result<(), crate::Error> = BountyEscrowContract::update_fee_config;
        entrypoint(
            env.clone(),
            lock_fee_rate,
            release_fee_rate,
            lock_fixed_fee,
            release_fixed_fee,
            fee_recipient,
            fee_enabled,
        )
    }

    fn get_fee_config(env: &Env) -> crate::FeeConfig {
        let entrypoint: fn(Env) -> crate::FeeConfig = BountyEscrowContract::get_fee_config;
        entrypoint(env.clone())
    }
}
