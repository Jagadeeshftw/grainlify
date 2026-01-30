#[cfg(test)]
mod multisig_tests {
    use crate::{ProgramEscrowContract, ProgramEscrowContractClient, MultisigConfig};
    use soroban_sdk::{testutils::Address as _, token, vec, Address, Env, String};

    fn create_token<'a>(env: &Env, admin: &Address) -> token::Client<'a> {
        let addr = env.register_stellar_asset_contract(admin.clone());
        token::Client::new(env, &addr)
    }

    fn setup_contract(env: &Env) -> (ProgramEscrowContractClient, Address, token::Client, String, Address) {
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        let token = create_token(env, &admin);
        let program_id = String::from_str(env, "test_program");
        
        client.initialize_program(&program_id, &admin, &token.address);
        
        // Mint tokens TO THE CONTRACT and lock funds
        token::StellarAssetClient::new(env, &token.address).mint(&contract_id, &100000);
        client.lock_program_funds(&program_id, &50000);
        
        (client, admin, token, program_id, contract_id)
    }

    // ========================================================================
    // Configuration Tests
    // ========================================================================

    #[test]
    fn test_configure_multisig() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, _, program_id, _) = setup_contract(&env);

        let signer1 = Address::generate(&env);
        let signer2 = Address::generate(&env);
        let signer3 = Address::generate(&env);
        let signers = vec![&env, signer1.clone(), signer2.clone(), signer3.clone()];

        client.configure_multisig(&program_id, &1000i128, &signers, &2, &true);

        let config = client.get_multisig_config();
        assert!(config.is_some());
        let config = config.unwrap();
        assert_eq!(config.threshold_amount, 1000i128);
        assert_eq!(config.required_approvals, 2);
        assert_eq!(config.enabled, true);
        assert_eq!(config.signers.len(), 3);
    }

    #[test]
    fn test_configure_multisig_disabled() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _, program_id, _) = setup_contract(&env);

        let signer1 = Address::generate(&env);
        let signers = vec![&env, signer1.clone()];

        client.configure_multisig(&program_id, &1000i128, &signers, &1, &false);

        let config = client.get_multisig_config().unwrap();
        assert_eq!(config.enabled, false);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #9)")]
    fn test_configure_multisig_invalid_threshold() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _, program_id, _) = setup_contract(&env);

        let signers = vec![&env, Address::generate(&env)];

        // required_approvals > signers.len() should fail
        client.configure_multisig(&program_id, &1000i128, &signers, &5, &true);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #9)")]
    fn test_configure_multisig_zero_required() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _, program_id, _) = setup_contract(&env);

        let signers = vec![&env, Address::generate(&env)];

        // required_approvals = 0 should fail
        client.configure_multisig(&program_id, &1000i128, &signers, &0, &true);
    }

    // ========================================================================
    // Initiate Payout Tests
    // ========================================================================

    #[test]
    fn test_initiate_payout() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _, program_id, _) = setup_contract(&env);

        let signers = vec![&env, Address::generate(&env), Address::generate(&env)];
        client.configure_multisig(&program_id, &100i128, &signers, &2, &true);

        // Initiate payout above threshold
        let recipient = Address::generate(&env);
        let result = client.initiate_payout(&program_id, &recipient, &500);
        assert!(result.is_some());
        let payout_id = result.unwrap();

        // Approval should exist
        let approval = client.get_payout_approval(&payout_id);
        assert!(approval.is_some());
        let approval = approval.unwrap();
        assert_eq!(approval.amount, 500);
    }

    #[test]
    fn test_initiate_payout_below_threshold() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _, program_id, _) = setup_contract(&env);

        let signers = vec![&env, Address::generate(&env)];
        client.configure_multisig(&program_id, &1000i128, &signers, &1, &true);

        // Initiate payout below threshold - should return None
        let recipient = Address::generate(&env);
        let result = client.initiate_payout(&program_id, &recipient, &500);
        assert!(result.is_none());
    }

    #[test]
    fn test_initiate_payout_without_multisig() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _, program_id, _) = setup_contract(&env);

        // No multisig configured - should return None
        let recipient = Address::generate(&env);
        let result = client.initiate_payout(&program_id, &recipient, &500);
        assert!(result.is_none());
    }

    // ========================================================================
    // Approval Flow Tests
    // ========================================================================

    #[test]
    fn test_multisig_approval_flow() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _, program_id, _) = setup_contract(&env);

        let signer1 = Address::generate(&env);
        let signer2 = Address::generate(&env);
        let signers = vec![&env, signer1.clone(), signer2.clone()];

        // Configure 2-of-2 multisig with threshold of 100
        client.configure_multisig(&program_id, &100i128, &signers, &2, &true);

        // Initiate payout above threshold
        let recipient = Address::generate(&env);
        let payout_id = client.initiate_payout(&program_id, &recipient, &500).unwrap();

        // First signer approves
        let executed = client.approve_payout_as(&payout_id, &signer1);
        assert_eq!(executed, false); // Not enough approvals yet

        // Second signer approves
        let executed = client.approve_payout_as(&payout_id, &signer2);
        assert_eq!(executed, true); // Now it executes

        // Approval should be cleared
        let approval = client.get_payout_approval(&payout_id);
        assert!(approval.is_none());
    }

    // ========================================================================
    // Approval Error Tests
    // ========================================================================

    #[test]
    #[should_panic(expected = "Error(Contract, #5)")]
    fn test_approve_not_authorized_signer() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _, program_id, _) = setup_contract(&env);

        let signer1 = Address::generate(&env);
        let signers = vec![&env, signer1.clone()];

        client.configure_multisig(&program_id, &100i128, &signers, &1, &true);

        let recipient = Address::generate(&env);
        let payout_id = client.initiate_payout(&program_id, &recipient, &500).unwrap();

        // Non-signer tries to approve
        let non_signer = Address::generate(&env);
        client.approve_payout_as(&payout_id, &non_signer);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #6)")]
    fn test_approve_already_approved() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _, program_id, _) = setup_contract(&env);

        let signer1 = Address::generate(&env);
        let signer2 = Address::generate(&env);
        let signers = vec![&env, signer1.clone(), signer2.clone()];

        client.configure_multisig(&program_id, &100i128, &signers, &2, &true);

        let recipient = Address::generate(&env);
        let payout_id = client.initiate_payout(&program_id, &recipient, &500).unwrap();

        // First approval
        client.approve_payout_as(&payout_id, &signer1);

        // Same signer tries again
        client.approve_payout_as(&payout_id, &signer1);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #8)")]
    fn test_approve_no_pending_approval() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _, program_id, _) = setup_contract(&env);

        let signer1 = Address::generate(&env);
        let signers = vec![&env, signer1.clone()];

        client.configure_multisig(&program_id, &100i128, &signers, &1, &true);

        // Try to approve non-existent approval
        client.approve_payout_as(&999, &signer1);
    }

    // ========================================================================
    // Cancel Approval Tests
    // ========================================================================

    #[test]
    fn test_cancel_payout_approval() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _, program_id, _) = setup_contract(&env);

        let signer1 = Address::generate(&env);
        let signers = vec![&env, signer1.clone()];

        client.configure_multisig(&program_id, &100i128, &signers, &1, &true);

        let recipient = Address::generate(&env);
        let payout_id = client.initiate_payout(&program_id, &recipient, &500).unwrap();

        // Approval exists
        assert!(client.get_payout_approval(&payout_id).is_some());

        // Cancel
        client.cancel_payout_approval(&program_id, &payout_id);

        // Approval gone
        assert!(client.get_payout_approval(&payout_id).is_none());
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #8)")]
    fn test_cancel_nonexistent_approval() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _, program_id, _) = setup_contract(&env);

        client.cancel_payout_approval(&program_id, &999);
    }

    // ========================================================================
    // Edge Cases
    // ========================================================================

    #[test]
    fn test_1_of_3_multisig() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _, program_id, _) = setup_contract(&env);

        let signer1 = Address::generate(&env);
        let signer2 = Address::generate(&env);
        let signer3 = Address::generate(&env);
        let signers = vec![&env, signer1.clone(), signer2.clone(), signer3.clone()];

        // 1-of-3 multisig
        client.configure_multisig(&program_id, &100i128, &signers, &1, &true);

        let recipient = Address::generate(&env);
        let payout_id = client.initiate_payout(&program_id, &recipient, &500).unwrap();

        // Single approval should execute
        let executed = client.approve_payout_as(&payout_id, &signer2);
        assert_eq!(executed, true);
    }

    #[test]
    fn test_full_payout_after_multisig() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _, program_id, _) = setup_contract(&env);

        let signer1 = Address::generate(&env);
        let signers = vec![&env, signer1.clone()];

        client.configure_multisig(&program_id, &100i128, &signers, &1, &true);

        let recipient = Address::generate(&env);

        // Initiate and approve
        let payout_id = client.initiate_payout(&program_id, &recipient, &500).unwrap();
        let executed = client.approve_payout_as(&payout_id, &signer1);
        assert_eq!(executed, true);

        // Approval should be cleared after execution
        let approval = client.get_payout_approval(&payout_id);
        assert!(approval.is_none());
    }
}
