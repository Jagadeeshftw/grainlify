#[cfg(test)]
mod pause_tests {
    use crate::{BountyEscrowContract, BountyEscrowContractClient, PauseConfig};
    use soroban_sdk::{testutils::Address as _, token, Address, Env};

    fn create_token<'a>(env: &Env, admin: &Address) -> token::Client<'a> {
        let addr = env.register_stellar_asset_contract(admin.clone());
        token::Client::new(env, &addr)
    }

    fn setup_contract(env: &Env) -> (BountyEscrowContractClient, Address, token::Client) {
        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        let token = create_token(env, &admin);
        client.init(&admin, &token.address);
        (client, admin, token)
    }

    // ========================================================================
    // Global Pause Tests (Backward Compatibility)
    // ========================================================================

    #[test]
    fn test_pause() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _) = setup_contract(&env);

        client.pause();
        assert!(client.is_paused());
    }

    #[test]
    fn test_unpause() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _) = setup_contract(&env);

        client.pause();
        client.unpause();
        assert!(!client.is_paused());
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #21)")]
    fn test_lock_blocked_when_globally_paused() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, _) = setup_contract(&env);

        client.pause();
        client.lock_funds(&admin, &1, &1000, &9999);
    }

    #[test]
    fn test_emergency_withdraw() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _) = setup_contract(&env);
        let recipient = Address::generate(&env);

        client.pause();
        client.emergency_withdraw(&recipient);
    }

    #[test]
    fn test_pause_state_persists() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _) = setup_contract(&env);

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
        let (client, _, _) = setup_contract(&env);

        let config = client.get_pause_config();
        assert_eq!(config.lock_paused, false);
        assert_eq!(config.release_paused, false);
        assert_eq!(config.refund_paused, false);
    }

    #[test]
    fn test_set_pause_lock() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _) = setup_contract(&env);

        client.set_pause_lock(&true);
        assert!(client.is_lock_paused());
        assert!(!client.is_release_paused());
        assert!(!client.is_refund_paused());

        let config = client.get_pause_config();
        assert_eq!(config.lock_paused, true);
        assert_eq!(config.release_paused, false);
        assert_eq!(config.refund_paused, false);
    }

    #[test]
    fn test_set_pause_release() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _) = setup_contract(&env);

        client.set_pause_release(&true);
        assert!(!client.is_lock_paused());
        assert!(client.is_release_paused());
        assert!(!client.is_refund_paused());

        let config = client.get_pause_config();
        assert_eq!(config.lock_paused, false);
        assert_eq!(config.release_paused, true);
        assert_eq!(config.refund_paused, false);
    }

    #[test]
    fn test_set_pause_refund() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _) = setup_contract(&env);

        client.set_pause_refund(&true);
        assert!(!client.is_lock_paused());
        assert!(!client.is_release_paused());
        assert!(client.is_refund_paused());

        let config = client.get_pause_config();
        assert_eq!(config.lock_paused, false);
        assert_eq!(config.release_paused, false);
        assert_eq!(config.refund_paused, true);
    }

    #[test]
    fn test_granular_pause_toggle() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _) = setup_contract(&env);

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
        let (client, _, _) = setup_contract(&env);

        // Only lock paused - is_paused should be false
        client.set_pause_lock(&true);
        assert!(!client.is_paused());

        // Lock and release paused - is_paused should still be false
        client.set_pause_release(&true);
        assert!(!client.is_paused());

        // All three paused - is_paused should be true
        client.set_pause_refund(&true);
        assert!(client.is_paused());
    }

    #[test]
    fn test_global_pause_sets_all_flags() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _) = setup_contract(&env);

        client.pause();

        let config = client.get_pause_config();
        assert_eq!(config.lock_paused, true);
        assert_eq!(config.release_paused, true);
        assert_eq!(config.refund_paused, true);
    }

    #[test]
    fn test_global_unpause_clears_all_flags() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _) = setup_contract(&env);

        client.pause();
        client.unpause();

        let config = client.get_pause_config();
        assert_eq!(config.lock_paused, false);
        assert_eq!(config.release_paused, false);
        assert_eq!(config.refund_paused, false);
    }

    // ========================================================================
    // Granular Pause Operation Tests - Lock Paused Only
    // ========================================================================

    #[test]
    #[should_panic(expected = "Error(Contract, #21)")]
    fn test_lock_blocked_when_lock_paused() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, _) = setup_contract(&env);

        client.set_pause_lock(&true);
        client.lock_funds(&admin, &1, &1000, &9999);
    }

    #[test]
    fn test_release_works_when_only_lock_paused() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token) = setup_contract(&env);

        // Setup: lock funds first
        token::StellarAssetClient::new(&env, &token.address).mint(&admin, &10000);
        client.lock_funds(&admin, &1, &1000, &9999999999);

        // Pause only lock operations
        client.set_pause_lock(&true);

        // Release should still work
        let contributor = Address::generate(&env);
        client.release_funds(&1, &contributor);
    }

    #[test]
    fn test_refund_works_when_only_lock_paused() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token) = setup_contract(&env);

        // Setup: lock funds
        token::StellarAssetClient::new(&env, &token.address).mint(&admin, &10000);
        client.lock_funds(&admin, &1, &1000, &9999999999);

        // Pause only lock operations
        client.set_pause_lock(&true);

        // Approve refund first (required for Custom mode before deadline)
        client.approve_refund(&1, &1000, &admin, &crate::RefundMode::Custom);

        // Refund should still work with Custom mode and approval
        client.refund(&1, &Some(1000), &Some(admin), &crate::RefundMode::Custom);
    }

    // ========================================================================
    // Granular Pause Operation Tests - Release Paused Only
    // ========================================================================

    #[test]
    fn test_lock_works_when_only_release_paused() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token) = setup_contract(&env);

        // Pause only release operations
        client.set_pause_release(&true);

        // Lock should still work
        token::StellarAssetClient::new(&env, &token.address).mint(&admin, &10000);
        client.lock_funds(&admin, &1, &1000, &9999);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #22)")]
    fn test_release_blocked_when_release_paused() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token) = setup_contract(&env);

        // Setup: lock funds first
        token::StellarAssetClient::new(&env, &token.address).mint(&admin, &10000);
        client.lock_funds(&admin, &1, &1000, &9999999999);

        // Pause release operations
        client.set_pause_release(&true);

        // Release should fail
        let contributor = Address::generate(&env);
        client.release_funds(&1, &contributor);
    }

    #[test]
    fn test_refund_works_when_only_release_paused() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token) = setup_contract(&env);

        // Setup: lock funds
        token::StellarAssetClient::new(&env, &token.address).mint(&admin, &10000);
        client.lock_funds(&admin, &1, &1000, &9999999999);

        // Pause only release operations
        client.set_pause_release(&true);

        // Approve refund first (required for Custom mode before deadline)
        client.approve_refund(&1, &1000, &admin, &crate::RefundMode::Custom);

        // Refund should still work with Custom mode and approval
        client.refund(&1, &Some(1000), &Some(admin), &crate::RefundMode::Custom);
    }

    // ========================================================================
    // Granular Pause Operation Tests - Refund Paused Only
    // ========================================================================

    #[test]
    fn test_lock_works_when_only_refund_paused() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token) = setup_contract(&env);

        // Pause only refund operations
        client.set_pause_refund(&true);

        // Lock should still work
        token::StellarAssetClient::new(&env, &token.address).mint(&admin, &10000);
        client.lock_funds(&admin, &1, &1000, &9999);
    }

    #[test]
    fn test_release_works_when_only_refund_paused() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token) = setup_contract(&env);

        // Setup: lock funds first
        token::StellarAssetClient::new(&env, &token.address).mint(&admin, &10000);
        client.lock_funds(&admin, &1, &1000, &9999999999);

        // Pause only refund operations
        client.set_pause_refund(&true);

        // Release should still work
        let contributor = Address::generate(&env);
        client.release_funds(&1, &contributor);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #23)")]
    fn test_refund_blocked_when_refund_paused() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token) = setup_contract(&env);

        // Setup: lock funds
        token::StellarAssetClient::new(&env, &token.address).mint(&admin, &10000);
        client.lock_funds(&admin, &1, &1000, &9999999999);

        // Approve refund first (so we test the pause check, not the approval check)
        client.approve_refund(&1, &1000, &admin, &crate::RefundMode::Custom);

        // Pause refund operations
        client.set_pause_refund(&true);

        // Refund should fail with RefundPaused error
        client.refund(&1, &Some(1000), &Some(admin), &crate::RefundMode::Custom);
    }

    // ========================================================================
    // Granular Pause Operation Tests - Multiple Paused
    // ========================================================================

    #[test]
    #[should_panic(expected = "Error(Contract, #21)")]
    fn test_lock_blocked_when_lock_and_release_paused() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, _) = setup_contract(&env);

        client.set_pause_lock(&true);
        client.set_pause_release(&true);

        client.lock_funds(&admin, &1, &1000, &9999);
    }

    #[test]
    fn test_refund_works_when_lock_and_release_paused() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token) = setup_contract(&env);

        // Setup: lock funds
        token::StellarAssetClient::new(&env, &token.address).mint(&admin, &10000);
        client.lock_funds(&admin, &1, &1000, &9999999999);

        // Pause lock and release
        client.set_pause_lock(&true);
        client.set_pause_release(&true);

        // Approve refund first (required for Custom mode before deadline)
        client.approve_refund(&1, &1000, &admin, &crate::RefundMode::Custom);

        // Refund should still work with Custom mode and approval
        client.refund(&1, &Some(1000), &Some(admin), &crate::RefundMode::Custom);
    }

    // ========================================================================
    // Batch Operation Pause Tests
    // ========================================================================

    #[test]
    #[should_panic(expected = "Error(Contract, #21)")]
    fn test_batch_lock_blocked_when_lock_paused() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token) = setup_contract(&env);

        token::StellarAssetClient::new(&env, &token.address).mint(&admin, &10000);

        client.set_pause_lock(&true);

        let items = soroban_sdk::vec![
            &env,
            crate::LockFundsItem {
                bounty_id: 1,
                depositor: admin.clone(),
                amount: 500,
                deadline: 9999,
            }
        ];
        client.batch_lock_funds(&items);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #22)")]
    fn test_batch_release_blocked_when_release_paused() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token) = setup_contract(&env);

        // Setup: lock funds first
        token::StellarAssetClient::new(&env, &token.address).mint(&admin, &10000);
        client.lock_funds(&admin, &1, &1000, &9999999999);

        client.set_pause_release(&true);

        let contributor = Address::generate(&env);
        let items = soroban_sdk::vec![
            &env,
            crate::ReleaseFundsItem {
                bounty_id: 1,
                contributor: contributor.clone(),
            }
        ];
        client.batch_release_funds(&items);
    }

    // ========================================================================
    // Pause After Partial Unpause Tests
    // ========================================================================

    #[test]
    fn test_partial_unpause_after_global_pause() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _) = setup_contract(&env);

        // Global pause
        client.pause();
        assert!(client.is_paused());

        // Partial unpause - only unlock refunds
        client.set_pause_refund(&false);

        // is_paused should be false now (not all operations are paused)
        assert!(!client.is_paused());

        // But lock and release should still be paused
        assert!(client.is_lock_paused());
        assert!(client.is_release_paused());
        assert!(!client.is_refund_paused());
    }

    #[test]
    fn test_idempotent_pause_operations() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _) = setup_contract(&env);

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
