use soroban_sdk::{Address, Env, String};

/// Shared interface for escrow functionality
/// Both bounty_escrow and program-escrow should implement this
#[allow(dead_code)]
pub trait EscrowInterface {
    /// Lock funds for a bounty
    fn lock_funds(
        env: &Env,
        depositor: Address,
        bounty_id: u64,
        amount: i128,
        deadline: u64,
    ) -> Result<(), crate::Error>;

    /// Release funds to contributor
    fn release_funds(env: &Env, bounty_id: u64, contributor: Address) -> Result<(), crate::Error>;

    /// Refund funds to depositor
    fn refund(env: &Env, bounty_id: u64) -> Result<(), crate::Error>;

    /// Get escrow info
    fn get_escrow_info(env: &Env, bounty_id: u64) -> Result<crate::Escrow, crate::Error>;

    /// Get contract balance
    fn get_balance(env: &Env) -> Result<i128, crate::Error>;
}

/// Shared interface for contract upgrades
#[allow(dead_code)]
pub trait UpgradeInterface {
    /// Get contract version
    fn get_version(env: &Env) -> u32;

    /// Set contract version
    fn set_version(env: &Env, new_version: u32) -> Result<(), String>;
}
