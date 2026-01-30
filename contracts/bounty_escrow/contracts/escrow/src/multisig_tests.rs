#[cfg(test)]
mod multisig_tests {
    use crate::{BountyEscrowContract, BountyEscrowContractClient, MultisigConfig};
    use soroban_sdk::{testutils::Address as _, token, vec, Address, Env};

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
    // Configuration Tests
    // ========================================================================

    #[test]
    fn test_configure_multisig() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, _) = setup_contract(&env);

        let signer1 = Address::generate(&env);
        let signer2 = Address::generate(&env);
        let signer3 = Address::generate(&env);
        let signers = vec![&env, signer1.clone(), signer2.clone(), signer3.clone()];

        client.configure_multisig(&1000_0000000i128, &signers, &2, &true);

        let config = client.get_multisig_config();
        assert!(config.is_some());
        let config = config.unwrap();
        assert_eq!(config.threshold_amount, 1000_0000000i128);
        assert_eq!(config.required_approvals, 2);
        assert_eq!(config.enabled, true);
        assert_eq!(config.signers.len(), 3);
    }

    #[test]
    fn test_configure_multisig_disabled() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _) = setup_contract(&env);

        let signer1 = Address::generate(&env);
        let signers = vec![&env, signer1.clone()];

        client.configure_multisig(&1000i128, &signers, &1, &false);

        let config = client.get_multisig_config().unwrap();
        assert_eq!(config.enabled, false);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #26)")]
    fn test_configure_multisig_invalid_threshold() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _) = setup_contract(&env);

        let signers = vec![&env, Address::generate(&env)];

        // required_approvals > signers.len() should fail
        client.configure_multisig(&1000i128, &signers, &5, &true);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #26)")]
    fn test_configure_multisig_zero_required() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _) = setup_contract(&env);

        let signers = vec![&env, Address::generate(&env)];

        // required_approvals = 0 should fail
        client.configure_multisig(&1000i128, &signers, &0, &true);
    }

    // ========================================================================
    // Release Below Threshold Tests
    // ========================================================================

    #[test]
    fn test_release_below_threshold_single_key() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token) = setup_contract(&env);

        // Configure multisig with threshold of 1000
        let signers = vec![&env, Address::generate(&env), Address::generate(&env)];
        client.configure_multisig(&1000i128, &signers, &2, &true);

        // Lock 500 (below threshold)
        token::StellarAssetClient::new(&env, &token.address).mint(&admin, &10000);
        client.lock_funds(&admin, &1, &500, &9999999999);

        // Single-key release should work (below threshold)
        let contributor = Address::generate(&env);
        let result = client.release_funds(&1, &contributor);
        // Should succeed without multisig
    }

    #[test]
    fn test_release_without_multisig_configured() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token) = setup_contract(&env);

        // No multisig configured
        token::StellarAssetClient::new(&env, &token.address).mint(&admin, &10000);
        client.lock_funds(&admin, &1, &5000, &9999999999);

        // Single-key release should work
        let contributor = Address::generate(&env);
        client.release_funds(&1, &contributor);
    }

    #[test]
    fn test_release_with_multisig_disabled() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token) = setup_contract(&env);

        // Configure but disable multisig
        let signers = vec![&env, Address::generate(&env)];
        client.configure_multisig(&100i128, &signers, &1, &false);

        // Lock above threshold
        token::StellarAssetClient::new(&env, &token.address).mint(&admin, &10000);
        client.lock_funds(&admin, &1, &5000, &9999999999);

        // Should work with single key since multisig disabled
        let contributor = Address::generate(&env);
        client.release_funds(&1, &contributor);
    }

    // ========================================================================
    // Release Above Threshold Tests
    // ========================================================================

    #[test]
    #[should_panic(expected = "Error(Contract, #21)")]
    fn test_release_above_threshold_requires_multisig() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token) = setup_contract(&env);

        // Configure multisig with threshold of 100
        let signers = vec![&env, Address::generate(&env), Address::generate(&env)];
        client.configure_multisig(&100i128, &signers, &2, &true);

        // Lock 500 (above threshold)
        token::StellarAssetClient::new(&env, &token.address).mint(&admin, &10000);
        client.lock_funds(&admin, &1, &500, &9999999999);

        // Direct release should fail with MultisigRequired error
        let contributor = Address::generate(&env);
        client.release_funds(&1, &contributor);
    }

    #[test]
    fn test_initiate_release() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token) = setup_contract(&env);

        let signers = vec![&env, Address::generate(&env), Address::generate(&env)];
        client.configure_multisig(&100i128, &signers, &2, &true);

        token::StellarAssetClient::new(&env, &token.address).mint(&admin, &10000);
        client.lock_funds(&admin, &1, &500, &9999999999);

        // Initiate release creates pending approval
        let contributor = Address::generate(&env);
        let needs_multisig = client.initiate_release(&1, &contributor);
        assert_eq!(needs_multisig, true);

        // Approval should exist
        let approval = client.get_release_approval(&1);
        assert!(approval.is_some());
    }

    #[test]
    fn test_initiate_release_below_threshold() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token) = setup_contract(&env);

        let signers = vec![&env, Address::generate(&env)];
        client.configure_multisig(&1000i128, &signers, &1, &true);

        token::StellarAssetClient::new(&env, &token.address).mint(&admin, &10000);
        client.lock_funds(&admin, &1, &500, &9999999999);

        // Initiate release returns false (no multisig needed)
        let contributor = Address::generate(&env);
        let needs_multisig = client.initiate_release(&1, &contributor);
        assert_eq!(needs_multisig, false);

        // No approval created
        let approval = client.get_release_approval(&1);
        assert!(approval.is_none());
    }

    #[test]
    fn test_multisig_approval_flow() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token) = setup_contract(&env);

        let signer1 = Address::generate(&env);
        let signer2 = Address::generate(&env);
        let signers = vec![&env, signer1.clone(), signer2.clone()];

        // Configure 2-of-2 multisig with threshold of 100
        client.configure_multisig(&100i128, &signers, &2, &true);

        // Lock 500 (above threshold)
        token::StellarAssetClient::new(&env, &token.address).mint(&admin, &10000);
        client.lock_funds(&admin, &1, &500, &9999999999);

        // Initiate release - creates pending approval
        let contributor = Address::generate(&env);
        let needs_multisig = client.initiate_release(&1, &contributor);
        assert_eq!(needs_multisig, true);

        // Verify pending approval exists
        let approval = client.get_release_approval(&1);
        assert!(approval.is_some());
        let approval = approval.unwrap();
        assert_eq!(approval.bounty_id, 1);
        assert_eq!(approval.amount, 500);

        // First signer approves
        let executed = client.approve_release_as(&1, &signer1);
        assert_eq!(executed, false); // Not enough approvals yet

        // Second signer approves
        let executed = client.approve_release_as(&1, &signer2);
        assert_eq!(executed, true); // Now it executes

        // Approval should be cleared
        let approval = client.get_release_approval(&1);
        assert!(approval.is_none());
    }

    // ========================================================================
    // Approval Error Tests
    // ========================================================================

    #[test]
    #[should_panic(expected = "Error(Contract, #22)")]
    fn test_approve_not_authorized_signer() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token) = setup_contract(&env);

        let signer1 = Address::generate(&env);
        let signers = vec![&env, signer1.clone()];

        client.configure_multisig(&100i128, &signers, &1, &true);

        token::StellarAssetClient::new(&env, &token.address).mint(&admin, &10000);
        client.lock_funds(&admin, &1, &500, &9999999999);

        let contributor = Address::generate(&env);
        client.initiate_release(&1, &contributor);

        // Non-signer tries to approve
        let non_signer = Address::generate(&env);
        client.approve_release_as(&1, &non_signer);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #23)")]
    fn test_approve_already_approved() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token) = setup_contract(&env);

        let signer1 = Address::generate(&env);
        let signer2 = Address::generate(&env);
        let signers = vec![&env, signer1.clone(), signer2.clone()];

        client.configure_multisig(&100i128, &signers, &2, &true);

        token::StellarAssetClient::new(&env, &token.address).mint(&admin, &10000);
        client.lock_funds(&admin, &1, &500, &9999999999);

        let contributor = Address::generate(&env);
        client.initiate_release(&1, &contributor);

        // First approval
        client.approve_release_as(&1, &signer1);

        // Same signer tries again
        client.approve_release_as(&1, &signer1);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #25)")]
    fn test_approve_no_pending_approval() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _) = setup_contract(&env);

        let signer1 = Address::generate(&env);
        let signers = vec![&env, signer1.clone()];

        client.configure_multisig(&100i128, &signers, &1, &true);

        // Try to approve non-existent approval
        client.approve_release_as(&999, &signer1);
    }

    // ========================================================================
    // Cancel Approval Tests
    // ========================================================================

    #[test]
    fn test_cancel_release_approval() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token) = setup_contract(&env);

        let signer1 = Address::generate(&env);
        let signers = vec![&env, signer1.clone()];

        client.configure_multisig(&100i128, &signers, &1, &true);

        token::StellarAssetClient::new(&env, &token.address).mint(&admin, &10000);
        client.lock_funds(&admin, &1, &500, &9999999999);

        let contributor = Address::generate(&env);
        client.initiate_release(&1, &contributor);

        // Approval exists
        assert!(client.get_release_approval(&1).is_some());

        // Admin cancels
        client.cancel_release_approval(&1);

        // Approval gone
        assert!(client.get_release_approval(&1).is_none());
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #25)")]
    fn test_cancel_nonexistent_approval() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _) = setup_contract(&env);

        client.cancel_release_approval(&999);
    }

    // ========================================================================
    // Edge Cases
    // ========================================================================

    #[test]
    fn test_threshold_boundary_exact() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token) = setup_contract(&env);

        let signers = vec![&env, Address::generate(&env)];
        client.configure_multisig(&1000i128, &signers, &1, &true);

        token::StellarAssetClient::new(&env, &token.address).mint(&admin, &10000);

        // Lock exactly at threshold - should NOT require multisig
        // (only amounts ABOVE threshold require it)
        client.lock_funds(&admin, &1, &1000, &9999999999);

        let contributor = Address::generate(&env);
        client.release_funds(&1, &contributor);
    }

    #[test]
    fn test_1_of_3_multisig() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token) = setup_contract(&env);

        let signer1 = Address::generate(&env);
        let signer2 = Address::generate(&env);
        let signer3 = Address::generate(&env);
        let signers = vec![&env, signer1.clone(), signer2.clone(), signer3.clone()];

        // 1-of-3 multisig
        client.configure_multisig(&100i128, &signers, &1, &true);

        token::StellarAssetClient::new(&env, &token.address).mint(&admin, &10000);
        client.lock_funds(&admin, &1, &500, &9999999999);

        let contributor = Address::generate(&env);
        client.initiate_release(&1, &contributor);

        // Single approval should execute
        let executed = client.approve_release_as(&1, &signer2);
        assert_eq!(executed, true);
    }

    #[test]
    fn test_full_release_after_multisig() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, token) = setup_contract(&env);

        let signer1 = Address::generate(&env);
        let signers = vec![&env, signer1.clone()];

        client.configure_multisig(&100i128, &signers, &1, &true);

        token::StellarAssetClient::new(&env, &token.address).mint(&admin, &10000);
        client.lock_funds(&admin, &1, &500, &9999999999);

        let contributor = Address::generate(&env);
        
        // Initiate and approve
        client.initiate_release(&1, &contributor);
        let executed = client.approve_release_as(&1, &signer1);
        assert_eq!(executed, true);

        // Approval should be cleared after execution
        let approval = client.get_release_approval(&1);
        assert!(approval.is_none());
    }
}
