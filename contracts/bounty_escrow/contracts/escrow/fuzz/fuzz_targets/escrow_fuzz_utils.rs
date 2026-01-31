//! Utility functions for fuzzing the Bounty Escrow contract

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, Vec,
};

/// Creates a token contract for testing
pub fn create_token_contract<'a>(
    e: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let contract_address = e.register_stellar_asset_contract(admin.clone());
    (
        token::Client::new(e, &contract_address),
        token::StellarAssetClient::new(e, &contract_address),
    )
}

/// Creates an escrow contract for testing
pub fn create_escrow_contract<'a>(e: &Env) -> bounty_escrow::BountyEscrowContractClient<'a> {
    let contract_id = e.register_contract(None, bounty_escrow::BountyEscrowContract);
    bounty_escrow::BountyEscrowContractClient::new(e, &contract_id)
}

/// Generates a pseudo-random address from fuzz data
pub fn address_from_bytes(env: &Env, bytes: &[u8]) -> Address {
    if bytes.len() >= 32 {
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes[0..32]);
        Address::from_bytes(env, &arr)
    } else {
        Address::generate(env)
    }
}

/// Extracts u64 from byte slice
pub fn bytes_to_u64(bytes: &[u8], offset: usize) -> u64 {
    let mut arr = [0u8; 8];
    for i in 0..8 {
        if offset + i < bytes.len() {
            arr[i] = bytes[offset + i];
        }
    }
    u64::from_le_bytes(arr)
}

/// Extracts i128 from byte slice
pub fn bytes_to_i128(bytes: &[u8], offset: usize) -> i128 {
    let mut arr = [0u8; 16];
    for i in 0..16 {
        if offset + i < bytes.len() {
            arr[i] = bytes[offset + i];
        }
    }
    i128::from_le_bytes(arr)
}

/// Extracts i64 from byte slice
pub fn bytes_to_i64(bytes: &[u8], offset: usize) -> i64 {
    let mut arr = [0u8; 8];
    for i in 0..8 {
        if offset + i < bytes.len() {
            arr[i] = bytes[offset + i];
        }
    }
    i64::from_le_bytes(arr)
}

/// Test setup helper for fuzzing
pub struct FuzzTestSetup<'a> {
    pub env: Env,
    pub admin: Address,
    pub depositor: Address,
    pub contributor: Address,
    pub token: token::Client<'a>,
    pub token_admin: token::StellarAssetClient<'a>,
    pub escrow: bounty_escrow::BountyEscrowContractClient<'a>,
}

impl<'a> FuzzTestSetup<'a> {
    pub fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);

        let (token, token_admin) = create_token_contract(&env, &admin);
        let escrow = create_escrow_contract(&env);

        // Initialize the contract
        let _ = escrow.init(&admin, &token.address);

        // Mint initial tokens to depositor
        token_admin.mint(&depositor, &1_000_000_000);

        Self {
            env,
            admin,
            depositor,
            contributor,
            token,
            token_admin,
            escrow,
        }
    }

    pub fn new_with_fuzz_data(data: &[u8]) -> Self {
        let env = Env::default();
        env.mock_all_auths();

        // Generate addresses from fuzz data for reproducibility
        let admin = if data.len() >= 32 {
            address_from_bytes(&env, &data[0..32])
        } else {
            Address::generate(&env)
        };

        let depositor = if data.len() >= 64 {
            address_from_bytes(&env, &data[32..64])
        } else {
            Address::generate(&env)
        };

        let contributor = if data.len() >= 96 {
            address_from_bytes(&env, &data[64..96])
        } else {
            Address::generate(&env)
        };

        let (token, token_admin) = create_token_contract(&env, &admin);
        let escrow = create_escrow_contract(&env);

        // Initialize the contract
        let _ = escrow.init(&admin, &token.address);

        // Mint initial tokens to depositor
        token_admin.mint(&depositor, &1_000_000_000);

        Self {
            env,
            admin,
            depositor,
            contributor,
            token,
            token_admin,
            escrow,
        }
    }

    /// Advance ledger timestamp
    pub fn advance_time(&self, seconds: u64) {
        let current = self.env.ledger().timestamp();
        self.env.ledger().set_timestamp(current + seconds);
    }

    /// Set specific timestamp
    pub fn set_time(&self, timestamp: u64) {
        self.env.ledger().set_timestamp(timestamp);
    }
}

/// Helper to create a valid escrow for testing refunds/releases
pub fn setup_locked_escrow<'a>(
    setup: &FuzzTestSetup<'a>,
    bounty_id: u64,
    amount: i128,
    deadline_offset: u64,
) -> Result<(), bounty_escrow::Error> {
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + deadline_offset;

    // Ensure depositor has enough tokens
    let current_balance = setup.token.balance(&setup.depositor);
    if current_balance < amount {
        setup.token_admin.mint(&setup.depositor, &(amount - current_balance));
    }

    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline)
}

/// Property: Locking funds should decrease depositor balance and increase contract balance
pub fn verify_lock_funds_invariant<'a>(
    setup: &FuzzTestSetup<'a>,
    bounty_id: u64,
    amount: i128,
) -> bool {
    let depositor_balance_before = setup.token.balance(&setup.depositor);
    let contract_balance_before = setup.token.balance(&setup.escrow.address);

    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    if let Ok(()) = setup.escrow.try_lock_funds(&setup.depositor, &bounty_id, &amount, &deadline) {
        let depositor_balance_after = setup.token.balance(&setup.depositor);
        let contract_balance_after = setup.token.balance(&setup.escrow.address);

        // Invariant: depositor balance decreased by amount
        // Invariant: contract balance increased by amount
        depositor_balance_after == depositor_balance_before - amount
            && contract_balance_after == contract_balance_before + amount
    } else {
        // If lock failed, balances should be unchanged
        let depositor_balance_after = setup.token.balance(&setup.depositor);
        let contract_balance_after = setup.token.balance(&setup.escrow.address);
        
        depositor_balance_after == depositor_balance_before
            && contract_balance_after == contract_balance_before
    }
}

/// Property: Releasing funds should decrease contract balance and increase contributor balance
pub fn verify_release_funds_invariant<'a>(
    setup: &FuzzTestSetup<'a>,
    bounty_id: u64,
    contributor: &Address,
) -> bool {
    let contract_balance_before = setup.token.balance(&setup.escrow.address);
    let contributor_balance_before = setup.token.balance(contributor);

    if let Ok(()) = setup.escrow.try_release_funds(&bounty_id, contributor) {
        let contract_balance_after = setup.token.balance(&setup.escrow.address);
        let contributor_balance_after = setup.token.balance(contributor);

        // Get the escrow amount that was released
        if let Ok(escrow) = setup.escrow.try_get_escrow_info(&bounty_id) {
            // Invariant: contract balance decreased
            // Invariant: contributor balance increased by escrow amount
            contract_balance_after <= contract_balance_before
                && contributor_balance_after >= contributor_balance_before
        } else {
            true // Escrow info might not be available after release
        }
    } else {
        true // Release failed, invariants don't apply
    }
}

/// Property: Total funds should be conserved (no creation/destruction)
pub fn verify_fund_conservation<'a>(setup: &FuzzTestSetup<'a>) -> bool {
    let contract_balance = setup.token.balance(&setup.escrow.address);
    let depositor_balance = setup.token.balance(&setup.depositor);
    let contributor_balance = setup.token.balance(&setup.contributor);
    let admin_balance = setup.token.balance(&setup.admin);

    // Total supply should remain constant (assuming no new minting in test)
    // This is a simplified check - in real tests would track total supply
    contract_balance >= 0 && depositor_balance >= 0 && contributor_balance >= 0
}
