//! Test setup helpers and TestSetup struct.
//!
//! Provides a comprehensive test setup structure that includes all commonly
//! needed components for testing escrow contracts.

use soroban_sdk::{token, Address, Env};
use super::factories::create_token_contract;

#[cfg(test)]
use bounty_escrow::BountyEscrowContractClient;
#[cfg(test)]
use super::factories::create_escrow_contract;

/// Comprehensive test setup structure.
///
/// This struct contains all commonly needed components for testing:
/// - Environment
/// - Admin, depositor, and contributor addresses
/// - Token contract and admin client
/// - Escrow contract client and address
///
/// # Example
/// ```rust,no_run
/// # use test_utils::setup::TestSetup;
/// let setup = TestSetup::new();
/// setup.escrow.lock_funds(&setup.depositor, &1, &1000, &10000);
/// ```
#[cfg(test)]
pub struct TestSetup<'a> {
    pub env: Env,
    pub admin: Address,
    pub depositor: Address,
    pub contributor: Address,
    pub token: token::Client<'a>,
    pub token_admin: token::StellarAssetClient<'a>,
    pub escrow: BountyEscrowContractClient<'a>,
    pub escrow_address: Address,
    pub token_address: Address,
}

#[cfg(test)]
impl<'a> TestSetup<'a> {
    /// Creates a new test setup with all components initialized.
    ///
    /// This will:
    /// - Create a new environment
    /// - Mock all auths
    /// - Generate admin, depositor, and contributor addresses
    /// - Create and initialize token contract
    /// - Create and initialize escrow contract
    /// - Mint tokens to depositor (1,000,000 by default)
    ///
    /// # Example
    /// ```rust,no_run
    /// # use test_utils::setup::TestSetup;
    /// let setup = TestSetup::new();
    /// ```
    pub fn new() -> Self {
        Self::with_mint_amount(1_000_000)
    }

    /// Creates a new test setup with a custom mint amount.
    ///
    /// # Arguments
    /// * `mint_amount` - Amount to mint to the depositor
    ///
    /// # Example
    /// ```rust,no_run
    /// # use test_utils::setup::TestSetup;
    /// let setup = TestSetup::with_mint_amount(5_000_000);
    /// ```
    pub fn with_mint_amount(mint_amount: i128) -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);

        let (token_address, token, token_admin) = create_token_contract(&env, &admin);
        let (escrow, escrow_address) = create_escrow_contract(&env);

        escrow.init(&admin, &token_address);

        // Mint tokens to depositor
        token_admin.mint(&depositor, &mint_amount);

        Self {
            env,
            admin,
            depositor,
            contributor,
            token,
            token_admin,
            escrow,
            escrow_address,
            token_address,
        }
    }

    /// Creates a test setup with multiple contributors.
    ///
    /// # Arguments
    /// * `contributor_count` - Number of contributors to generate
    ///
    /// # Returns
    /// A tuple containing the setup and a vector of contributor addresses
    ///
    /// # Example
    /// ```rust,no_run
    /// # use test_utils::setup::TestSetup;
    /// let (setup, contributors) = TestSetup::with_contributors(3);
    /// ```
    pub fn with_contributors(contributor_count: u32) -> (Self, Vec<Address>) {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        
        let mut contributors = Vec::new(&env);
        for _ in 0..contributor_count {
            contributors.push_back(Address::generate(&env));
        }

        let (token_address, token, token_admin) = create_token_contract(&env, &admin);
        let (escrow, escrow_address) = create_escrow_contract(&env);

        escrow.init(&admin, &token_address);

        // Mint tokens to depositor
        token_admin.mint(&depositor, &1_000_000);

        let first_contributor = contributors.get(0).unwrap().clone();
        let contributors_array = contributors.to_array();

        let setup = Self {
            env,
            admin,
            depositor,
            contributor: first_contributor,
            token,
            token_admin,
            escrow,
            escrow_address,
            token_address,
        };

        (setup, contributors_array)
    }

    /// Locks funds for a bounty (convenience method).
    ///
    /// # Arguments
    /// * `bounty_id` - The bounty ID
    /// * `amount` - The amount to lock
    /// * `deadline` - The deadline timestamp
    ///
    /// # Example
    /// ```rust,no_run
    /// # use test_utils::setup::TestSetup;
    /// # let setup = TestSetup::new();
    /// let deadline = setup.env.ledger().timestamp() + 1000;
    /// setup.lock_funds(1, 1000, deadline);
    /// ```
    pub fn lock_funds(&self, bounty_id: u64, amount: i128, deadline: u64) {
        self.escrow.lock_funds(&self.depositor, &bounty_id, &amount, &deadline);
    }

    /// Releases funds for a bounty (convenience method).
    ///
    /// # Arguments
    /// * `bounty_id` - The bounty ID
    /// * `contributor` - The contributor address (defaults to self.contributor)
    ///
    /// # Example
    /// ```rust,no_run
    /// # use test_utils::setup::TestSetup;
    /// # let setup = TestSetup::new();
    /// # let deadline = setup.env.ledger().timestamp() + 1000;
    /// # setup.lock_funds(1, 1000, deadline);
    /// setup.release_funds(1, None);
    /// ```
    pub fn release_funds(&self, bounty_id: u64, contributor: Option<&Address>) {
        let contributor_addr = contributor.unwrap_or(&self.contributor);
        self.escrow.release_funds(&bounty_id, contributor_addr);
    }
}
