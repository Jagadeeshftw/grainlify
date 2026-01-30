#[cfg(test)]
mod pause_tests {
    use crate::{ProgramEscrowContract, ProgramEscrowContractClient, PauseConfig};
    use soroban_sdk::{testutils::Address as _, token, Address, Env, String};

    fn create_token<'a>(env: &Env, admin: &Address) -> token::Client<'a> {
        let addr = env.register_stellar_asset_contract(admin.clone());
        token::Client::new(env, &addr)
    }

    fn setup_contract(env: &Env) -> (ProgramEscrowContractClient, Address, token::Client, String) {
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        let token = create_token(env, &admin);
        let prog_id = String::from_str(env, "TestProgram");
        client.initialize_program(&prog_id, &admin, &token.address);
        (client, admin, token, prog_id)
    }

    // ========================================================================
    // Global Pause Tests (Backward Compatibility)
    // ========================================================================

    #[test]
    fn test_pause() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        client.pause();
        assert!(client.is_paused());
    }

    #[test]
    fn test_unpause() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        client.pause();
        client.unpause();
        assert!(!client.is_paused());
    }

    #[test]
    #[should_panic(expected = "Lock operations are paused")]
    fn test_lock_blocked_when_globally_paused() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token, prog_id) = setup_contract(&env);

        token::StellarAssetClient::new(&env, &token.address).mint(&admin, &10000);

        client.pause();
        client.lock_program_funds(&prog_id, &1000);
    }

    #[test]
    fn test_emergency_withdraw() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _, prog_id) = setup_contract(&env);
        let recipient = Address::generate(&env);

        client.pause();
        client.emergency_withdraw(&prog_id, &recipient);
    }

    #[test]
    fn test_pause_state_persists() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        client.pause();
        assert!(client.is_paused());
        assert!(client.is_paused());
    }

    // ========================================================================
    // Granular Pause Configuration Tests
    // ========================================================================

    #[test]
    fn test_get_pause_config_default() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let config = client.get_pause_config();
        assert_eq!(config.lock_paused, false);
        assert_eq!(config.payout_paused, false);
        assert_eq!(config.schedule_paused, false);
    }

    #[test]
    fn test_set_pause_lock() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        client.set_pause_lock(&true);
        assert!(client.is_lock_paused());
        assert!(!client.is_payout_paused());
        assert!(!client.is_schedule_paused());

        let config = client.get_pause_config();
        assert_eq!(config.lock_paused, true);
        assert_eq!(config.payout_paused, false);
        assert_eq!(config.schedule_paused, false);
    }

    #[test]
    fn test_set_pause_payout() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        client.set_pause_payout(&true);
        assert!(!client.is_lock_paused());
        assert!(client.is_payout_paused());
        assert!(!client.is_schedule_paused());

        let config = client.get_pause_config();
        assert_eq!(config.lock_paused, false);
        assert_eq!(config.payout_paused, true);
        assert_eq!(config.schedule_paused, false);
    }

    #[test]
    fn test_set_pause_schedule() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        client.set_pause_schedule(&true);
        assert!(!client.is_lock_paused());
        assert!(!client.is_payout_paused());
        assert!(client.is_schedule_paused());

        let config = client.get_pause_config();
        assert_eq!(config.lock_paused, false);
        assert_eq!(config.payout_paused, false);
        assert_eq!(config.schedule_paused, true);
    }

    #[test]
    fn test_granular_pause_toggle() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        // Pause lock
        client.set_pause_lock(&true);
        assert!(client.is_lock_paused());

        // Unpause lock
        client.set_pause_lock(&false);
        assert!(!client.is_lock_paused());
    }

    #[test]
    fn test_is_paused_requires_all_paused() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        // Only lock paused - is_paused should be false
        client.set_pause_lock(&true);
        assert!(!client.is_paused());

        // Lock and payout paused - is_paused should still be false
        client.set_pause_payout(&true);
        assert!(!client.is_paused());

        // All three paused - is_paused should be true
        client.set_pause_schedule(&true);
        assert!(client.is_paused());
    }

    #[test]
    fn test_global_pause_sets_all_flags() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        client.pause();

        let config = client.get_pause_config();
        assert_eq!(config.lock_paused, true);
        assert_eq!(config.payout_paused, true);
        assert_eq!(config.schedule_paused, true);
    }

    #[test]
    fn test_global_unpause_clears_all_flags() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        client.pause();
        client.unpause();

        let config = client.get_pause_config();
        assert_eq!(config.lock_paused, false);
        assert_eq!(config.payout_paused, false);
        assert_eq!(config.schedule_paused, false);
    }

    // ========================================================================
    // Granular Pause Operation Tests - Lock Paused Only
    // ========================================================================

    #[test]
    #[should_panic(expected = "Lock operations are paused")]
    fn test_lock_blocked_when_lock_paused() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token, prog_id) = setup_contract(&env);

        token::StellarAssetClient::new(&env, &token.address).mint(&admin, &10000);

        client.set_pause_lock(&true);
        client.lock_program_funds(&prog_id, &1000);
    }

    #[test]
    fn test_payout_works_when_only_lock_paused() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let token = create_token(&env, &admin);
        let prog_id = String::from_str(&env, "TestProgram");
        client.initialize_program(&prog_id, &admin, &token.address);

        // Setup: mint tokens to the contract and lock funds
        token::StellarAssetClient::new(&env, &token.address).mint(&contract_id, &10000);
        client.lock_program_funds(&prog_id, &5000);

        // Pause only lock operations
        client.set_pause_lock(&true);

        // Payout should still work
        let recipient = Address::generate(&env);
        client.single_payout(&prog_id, &recipient, &100);
    }

    // ========================================================================
    // Granular Pause Operation Tests - Payout Paused Only
    // ========================================================================

    #[test]
    fn test_lock_works_when_only_payout_paused() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token, prog_id) = setup_contract(&env);

        token::StellarAssetClient::new(&env, &token.address).mint(&admin, &10000);

        // Pause only payout operations
        client.set_pause_payout(&true);

        // Lock should still work
        client.lock_program_funds(&prog_id, &1000);
    }

    #[test]
    #[should_panic(expected = "Payout operations are paused")]
    fn test_single_payout_blocked_when_payout_paused() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token, prog_id) = setup_contract(&env);

        // Setup: lock funds first
        token::StellarAssetClient::new(&env, &token.address).mint(&admin, &10000);
        client.lock_program_funds(&prog_id, &5000);

        // Pause payout operations
        client.set_pause_payout(&true);

        // Payout should fail
        let recipient = Address::generate(&env);
        client.single_payout(&prog_id, &recipient, &100);
    }

    #[test]
    #[should_panic(expected = "Payout operations are paused")]
    fn test_batch_payout_blocked_when_payout_paused() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token, prog_id) = setup_contract(&env);

        // Setup: lock funds first
        token::StellarAssetClient::new(&env, &token.address).mint(&admin, &10000);
        client.lock_program_funds(&prog_id, &5000);

        // Pause payout operations
        client.set_pause_payout(&true);

        // Batch payout should fail
        let recipients = soroban_sdk::vec![&env, Address::generate(&env)];
        let amounts = soroban_sdk::vec![&env, 100i128];
        client.batch_payout(&prog_id, &recipients, &amounts);
    }

    // ========================================================================
    // Granular Pause Operation Tests - Schedule Paused Only
    // ========================================================================

    #[test]
    fn test_lock_works_when_only_schedule_paused() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token, prog_id) = setup_contract(&env);

        token::StellarAssetClient::new(&env, &token.address).mint(&admin, &10000);

        // Pause only schedule operations
        client.set_pause_schedule(&true);

        // Lock should still work
        client.lock_program_funds(&prog_id, &1000);
    }

    #[test]
    fn test_payout_works_when_only_schedule_paused() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let token = create_token(&env, &admin);
        let prog_id = String::from_str(&env, "TestProgram");
        client.initialize_program(&prog_id, &admin, &token.address);

        // Setup: mint tokens to the contract and lock funds
        token::StellarAssetClient::new(&env, &token.address).mint(&contract_id, &10000);
        client.lock_program_funds(&prog_id, &5000);

        // Pause only schedule operations
        client.set_pause_schedule(&true);

        // Payout should still work
        let recipient = Address::generate(&env);
        client.single_payout(&prog_id, &recipient, &100);
    }

    #[test]
    #[should_panic(expected = "Schedule operations are paused")]
    fn test_create_schedule_blocked_when_schedule_paused() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token, prog_id) = setup_contract(&env);

        // Setup: lock funds first
        token::StellarAssetClient::new(&env, &token.address).mint(&admin, &10000);
        client.lock_program_funds(&prog_id, &5000);

        // Pause schedule operations
        client.set_pause_schedule(&true);

        // Create schedule should fail
        let recipient = Address::generate(&env);
        client.create_program_release_schedule(
            &prog_id,
            &1000,
            &9999999999u64,
            &recipient,
        );
    }

    // ========================================================================
    // Granular Pause Operation Tests - Multiple Paused
    // ========================================================================

    #[test]
    #[should_panic(expected = "Lock operations are paused")]
    fn test_lock_blocked_when_lock_and_payout_paused() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token, prog_id) = setup_contract(&env);

        token::StellarAssetClient::new(&env, &token.address).mint(&admin, &10000);

        client.set_pause_lock(&true);
        client.set_pause_payout(&true);

        client.lock_program_funds(&prog_id, &1000);
    }

    #[test]
    fn test_schedule_works_when_lock_and_payout_paused() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token, prog_id) = setup_contract(&env);

        // Setup: lock funds first
        token::StellarAssetClient::new(&env, &token.address).mint(&admin, &10000);
        client.lock_program_funds(&prog_id, &5000);

        // Pause lock and payout
        client.set_pause_lock(&true);
        client.set_pause_payout(&true);

        // Schedule should still work
        let recipient = Address::generate(&env);
        client.create_program_release_schedule(
            &prog_id,
            &1000,
            &9999999999u64,
            &recipient,
        );
    }

    // ========================================================================
    // Pause After Partial Unpause Tests
    // ========================================================================

    #[test]
    fn test_partial_unpause_after_global_pause() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        // Global pause
        client.pause();
        assert!(client.is_paused());

        // Partial unpause - only unlock schedule
        client.set_pause_schedule(&false);

        // is_paused should be false now (not all operations are paused)
        assert!(!client.is_paused());

        // But lock and payout should still be paused
        assert!(client.is_lock_paused());
        assert!(client.is_payout_paused());
        assert!(!client.is_schedule_paused());
    }

    #[test]
    fn test_idempotent_pause_operations() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        // Set pause lock twice - should be idempotent
        client.set_pause_lock(&true);
        client.set_pause_lock(&true);
        assert!(client.is_lock_paused());

        // Unset pause lock twice - should be idempotent
        client.set_pause_lock(&false);
        client.set_pause_lock(&false);
        assert!(!client.is_lock_paused());
    }
}
