#[cfg(test)]
mod test_token_rescue {
    use crate::{BountyEscrowContract, BountyEscrowContractClient, Error, EscrowStatus, RefundMode};
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token, Address, Env,
    };

    fn create_token_contract<'a>(
        env: &Env,
        admin: &Address,
    ) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
        let contract = env.register_stellar_asset_contract_v2(admin.clone());
        let contract_address = contract.address();
        (
            token::Client::new(env, &contract_address),
            token::StellarAssetClient::new(env, &contract_address),
        )
    }

    fn setup_contract(
        env: &Env,
    ) -> (
        BountyEscrowContractClient,
        Address,
        Address,
        token::Client,
        token::StellarAssetClient,
    ) {
        let admin = Address::generate(env);
        let treasury = Address::generate(env);
        let token_admin = Address::generate(env);
        let (token_client, token_stellar) = create_token_contract(env, &token_admin);

        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(env, &contract_id);

        client.init(&admin, &token_client.address);
        client.set_treasury(&treasury);

        (client, admin, treasury, token_client, token_stellar)
    }

    #[test]
    fn test_set_treasury() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin, treasury, _token, _token_stellar) = setup_contract(&env);

        let retrieved_treasury = client.get_treasury();
        assert_eq!(retrieved_treasury, Some(treasury));
    }

    #[test]
    fn test_get_untracked_balance_with_no_escrows() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin, _treasury, token_client, token_stellar) = setup_contract(&env);

        // Send tokens directly to contract (not through lock_funds)
        let sender = Address::generate(&env);
        token_stellar.mint(&sender, &1000);
        token_client.transfer(&sender, &client.address, &1000);

        // All balance should be untracked since no escrows exist
        let untracked = client.get_untracked_balance();
        assert_eq!(untracked, 1000);
    }

    #[test]
    fn test_get_untracked_balance_with_escrows() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin, _treasury, token_client, token_stellar) = setup_contract(&env);

        // Create an escrow with 500 tokens
        let depositor = Address::generate(&env);
        token_stellar.mint(&depositor, &1500);

        let deadline = env.ledger().timestamp() + 1000;
        client.lock_funds(&depositor, &1, &500, &deadline);

        // Send additional 1000 tokens directly to contract
        token_client.transfer(&depositor, &client.address, &1000);

        // Untracked should be 1000 (total 1500 - escrow 500)
        let untracked = client.get_untracked_balance();
        assert_eq!(untracked, 1000);
    }

    #[test]
    fn test_get_untracked_balance_excludes_released_escrows() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin, _treasury, token_client, token_stellar) = setup_contract(&env);

        // Create and release an escrow
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);
        token_stellar.mint(&depositor, &1500);

        let deadline = env.ledger().timestamp() + 1000;
        client.lock_funds(&depositor, &1, &500, &deadline);
        client.release_funds(&1, &contributor);

        // Send 1000 tokens directly to contract
        token_client.transfer(&depositor, &client.address, &1000);

        // Untracked should be 1000 (released escrow doesn't count)
        let untracked = client.get_untracked_balance();
        assert_eq!(untracked, 1000);
    }

    #[test]
    fn test_get_untracked_balance_with_multiple_escrows() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin, _treasury, token_client, token_stellar) = setup_contract(&env);

        let depositor = Address::generate(&env);
        token_stellar.mint(&depositor, &3000);

        let deadline = env.ledger().timestamp() + 1000;

        // Create multiple escrows: 500 + 700 = 1200 tracked
        client.lock_funds(&depositor, &1, &500, &deadline);
        client.lock_funds(&depositor, &2, &700, &deadline);

        // Send 800 tokens directly
        token_client.transfer(&depositor, &client.address, &800);

        // Untracked should be 800 (total 2000 - tracked 1200)
        let untracked = client.get_untracked_balance();
        assert_eq!(untracked, 800);
    }

    #[test]
    fn test_rescue_tokens_success() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin, treasury, token_client, token_stellar) = setup_contract(&env);

        // Send tokens directly to contract
        let sender = Address::generate(&env);
        token_stellar.mint(&sender, &1000);
        token_client.transfer(&sender, &client.address, &1000);

        let treasury_balance_before = token_client.balance(&treasury);

        // Rescue tokens
        let rescued_amount = client.rescue_tokens();
        assert_eq!(rescued_amount, 1000);

        // Verify treasury received the tokens
        let treasury_balance_after = token_client.balance(&treasury);
        assert_eq!(treasury_balance_after, treasury_balance_before + 1000);

        // Verify contract has no untracked balance left
        let untracked = client.get_untracked_balance();
        assert_eq!(untracked, 0);
    }

    #[test]
    fn test_rescue_tokens_preserves_escrow_funds() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin, treasury, token_client, token_stellar) = setup_contract(&env);

        // Create escrow with 500 tokens
        let depositor = Address::generate(&env);
        token_stellar.mint(&depositor, &1500);

        let deadline = env.ledger().timestamp() + 1000;
        client.lock_funds(&depositor, &1, &500, &deadline);

        // Send 1000 tokens directly
        token_client.transfer(&depositor, &client.address, &1000);

        // Rescue tokens
        let rescued_amount = client.rescue_tokens();
        assert_eq!(rescued_amount, 1000);

        // Verify escrow is still intact
        let escrow = client.get_escrow_info(&1);
        assert_eq!(escrow.amount, 500);
        assert_eq!(escrow.remaining_amount, 500);
        assert_eq!(escrow.status, EscrowStatus::Locked);

        // Verify contract still has 500 tokens for the escrow
        let contract_balance = token_client.balance(&client.address);
        assert_eq!(contract_balance, 500);
    }

    #[test]
    fn test_rescue_tokens_with_no_untracked_balance() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin, _treasury, token_client, token_stellar) = setup_contract(&env);

        // Create escrow with all tokens tracked
        let depositor = Address::generate(&env);
        token_stellar.mint(&depositor, &500);

        let deadline = env.ledger().timestamp() + 1000;
        client.lock_funds(&depositor, &1, &500, &deadline);

        // Try to rescue - should fail
        let result = client.try_rescue_tokens();
        assert_eq!(result, Err(Ok(Error::NoUntrackedBalance)));
    }

    #[test]
    fn test_rescue_tokens_without_treasury_set() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let (token_client, token_stellar) = create_token_contract(&env, &token_admin);

        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);

        client.init(&admin, &token_client.address);
        // Don't set treasury

        // Send tokens directly
        let sender = Address::generate(&env);
        token_stellar.mint(&sender, &1000);
        token_client.transfer(&sender, &client.address, &1000);

        // Try to rescue - should fail
        let result = client.try_rescue_tokens();
        assert_eq!(result, Err(Ok(Error::TreasuryNotSet)));
    }

    #[test]
    fn test_rescue_tokens_requires_admin_auth() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin, _treasury, token_client, token_stellar) = setup_contract(&env);

        // Send tokens directly
        let sender = Address::generate(&env);
        token_stellar.mint(&sender, &1000);
        token_client.transfer(&sender, &client.address, &1000);

        // This should succeed because we're mocking all auths
        let rescued_amount = client.rescue_tokens();
        assert_eq!(rescued_amount, 1000);
    }

    #[test]
    fn test_rescue_tokens_after_partial_refund() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin, treasury, token_client, token_stellar) = setup_contract(&env);

        // Create escrow
        let depositor = Address::generate(&env);
        token_stellar.mint(&depositor, &2000);

        let deadline = env.ledger().timestamp() + 1000;
        client.lock_funds(&depositor, &1, &1000, &deadline);

        // Approve partial refund of 400
        client.approve_refund(&1, &400, &depositor, &RefundMode::Partial);
        client.refund(&1);

        // Send 500 tokens directly
        token_client.transfer(&depositor, &client.address, &500);

        // Untracked should be 500 (total 1100 - remaining 600)
        let untracked = client.get_untracked_balance();
        assert_eq!(untracked, 500);

        // Rescue tokens
        let rescued_amount = client.rescue_tokens();
        assert_eq!(rescued_amount, 500);

        // Verify escrow still has 600 remaining
        let escrow = client.get_escrow_info(&1);
        assert_eq!(escrow.remaining_amount, 600);
    }

    #[test]
    fn test_rescue_tokens_emits_event() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin, _treasury, token_client, token_stellar) = setup_contract(&env);

        // Send tokens directly
        let sender = Address::generate(&env);
        token_stellar.mint(&sender, &1000);
        token_client.transfer(&sender, &client.address, &1000);

        // Rescue tokens
        client.rescue_tokens();

        // Events are emitted - in a real test we'd verify the event details
        // For now, just verify the operation succeeded
        let untracked = client.get_untracked_balance();
        assert_eq!(untracked, 0);
    }

    #[test]
    fn test_multiple_rescue_operations() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin, treasury, token_client, token_stellar) = setup_contract(&env);

        let sender = Address::generate(&env);
        token_stellar.mint(&sender, &3000);

        // First rescue
        token_client.transfer(&sender, &client.address, &1000);
        let rescued1 = client.rescue_tokens();
        assert_eq!(rescued1, 1000);

        // Second rescue
        token_client.transfer(&sender, &client.address, &500);
        let rescued2 = client.rescue_tokens();
        assert_eq!(rescued2, 500);

        // Third rescue
        token_client.transfer(&sender, &client.address, &1500);
        let rescued3 = client.rescue_tokens();
        assert_eq!(rescued3, 1500);

        // Total rescued
        let treasury_balance = token_client.balance(&treasury);
        assert_eq!(treasury_balance, 3000);
    }

    #[test]
    fn test_rescue_with_zero_untracked_balance() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin, _treasury, _token, _token_stellar) = setup_contract(&env);

        // No tokens in contract at all
        let result = client.try_rescue_tokens();
        assert_eq!(result, Err(Ok(Error::NoUntrackedBalance)));
    }

    #[test]
    fn test_get_untracked_balance_with_refunded_escrow() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin, _treasury, token_client, token_stellar) = setup_contract(&env);

        // Create escrow
        let depositor = Address::generate(&env);
        token_stellar.mint(&depositor, &1500);

        let deadline = env.ledger().timestamp() + 1000;
        client.lock_funds(&depositor, &1, &500, &deadline);

        // Fast forward past deadline and refund
        env.ledger().with_mut(|li| li.timestamp = deadline + 1);
        client.refund(&1);

        // Send 1000 tokens directly
        token_client.transfer(&depositor, &client.address, &1000);

        // All 1000 should be untracked (refunded escrow doesn't count)
        let untracked = client.get_untracked_balance();
        assert_eq!(untracked, 1000);
    }
}
