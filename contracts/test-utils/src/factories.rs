//! Contract factory functions for creating test contracts.
//!
//! These functions simplify the creation of contracts and tokens for testing.

use soroban_sdk::{token, Address, Env};

/// Creates a token contract for testing.
///
/// # Arguments
/// * `env` - The contract environment
/// * `admin` - The admin address for the token
///
/// # Returns
/// A tuple containing:
/// - Token address
/// - Token client
/// - Token admin client
///
/// # Example
/// ```rust,no_run
/// # use soroban_sdk::{Address, Env};
/// # use test_utils::factories::create_token_contract;
/// # let env = Env::default();
/// # let admin = Address::generate(&env);
/// let (token_address, token_client, token_admin) = create_token_contract(&env, &admin);
/// ```
pub fn create_token_contract<'a>(
    env: &'a Env,
    admin: &Address,
) -> (Address, token::Client<'a>, token::StellarAssetClient<'a>) {
    let token_id = env.register_stellar_asset_contract_v2(admin.clone());
    let token = token_id.address();
    let token_client = token::Client::new(env, &token);
    let token_admin_client = token::StellarAssetClient::new(env, &token);
    (token, token_client, token_admin_client)
}

#[cfg(test)]
/// Creates an escrow contract for testing.
///
/// # Arguments
/// * `env` - The contract environment
///
/// # Returns
/// A tuple containing:
/// - Escrow contract client
/// - Escrow contract address
///
/// # Example
/// ```rust,no_run
/// # use soroban_sdk::Env;
/// # use test_utils::factories::create_escrow_contract;
/// # let env = Env::default();
/// let (escrow_client, escrow_address) = create_escrow_contract(&env);
/// ```
pub fn create_escrow_contract<'a>(
    env: &Env,
) -> (bounty_escrow::BountyEscrowContractClient<'a>, Address) {
    use bounty_escrow::{BountyEscrowContract, BountyEscrowContractClient};
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(env, &contract_id);
    (client, contract_id)
}

#[cfg(test)]
/// Creates a fully initialized escrow contract with token.
///
/// # Arguments
/// * `env` - The contract environment
/// * `admin` - The admin address
///
/// # Returns
/// A tuple containing:
/// - Escrow contract client
/// - Escrow contract address
/// - Token address
/// - Token client
/// - Token admin client
///
/// # Example
/// ```rust,no_run
/// # use soroban_sdk::{Address, Env};
/// # use test_utils::factories::create_initialized_escrow;
/// # let env = Env::default();
/// # let admin = Address::generate(&env);
/// let (escrow, escrow_addr, token_addr, token, token_admin) = 
///     create_initialized_escrow(&env, &admin);
/// ```
pub fn create_initialized_escrow<'a>(
    env: &'a Env,
    admin: &Address,
) -> (
    bounty_escrow::BountyEscrowContractClient<'a>,
    Address,
    Address,
    token::Client<'a>,
    token::StellarAssetClient<'a>,
) {
    use bounty_escrow::BountyEscrowContractClient;
    let (escrow, escrow_address) = create_escrow_contract(env);
    let (token_address, token_client, token_admin_client) = create_token_contract(env, admin);
    
    escrow.init(admin, &token_address);
    
    (escrow, escrow_address, token_address, token_client, token_admin_client)
}
