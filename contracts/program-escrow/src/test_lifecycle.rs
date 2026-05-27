#[cfg(test)]
mod test_lifecycle {
    use soroban_sdk::{
        testutils::{Address as _, Ledger, LedgerInfo},
        token, Address, Env, IntoVal, String,
    };
    use crate::{ProgramEscrowContract, ProgramEscrowContractClient, ProgramStatus, Error};

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn setup() -> (Env, Address, ProgramEscrowContractClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);
        (env, admin, client)
    }

    fn make_token(env: &Env, admin: &Address) -> Address {
        let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
        token_contract.address()
    }

    fn pid(env: &Env, s: &str) -> String {
        String::from_str(env, s)
    }

    // -----------------------------------------------------------------------
    // Initialization
    // -----------------------------------------------------------------------

    #[test]
    fn test_initialize_sets_admin() {
        let (env, admin, client) = setup();
        assert_eq!(client.get_admin(), Some(admin));
    }

    #[test]
    fn test_double_initialize_fails() {
        let (env, admin, client) = setup();
        let result = client.try_initialize(&admin);
        assert_eq!(result, Err(Ok(Error::AlreadyInitialized)));
    }

    // -----------------------------------------------------------------------
    // create_program
    // -----------------------------------------------------------------------

    #[test]
    fn test_create_program_starts_as_draft() {
        let (env, admin, client) = setup();
        let token = make_token(&env, &admin);
        client.create_program(&pid(&env, "p-001"), &pid(&env, "Test Program"), &token);
        let p = client.get_program(&pid(&env, "p-001")).unwrap();
        assert_eq!(p.status, ProgramStatus::Draft);
        assert_eq!(p.published_at, None);
        assert_eq!(p.balance, 0);
    }

    #[test]
    fn test_create_duplicate_program_fails() {
        let (env, admin, client) = setup();
        let token = make_token(&env, &admin);
        client.create_program(&pid(&env, "p-001"), &pid(&env, "Test"), &token);
        let result = client.try_create_program(&pid(&env, "p-001"), &pid(&env, "Dup"), &token);
        assert_eq!(result, Err(Ok(Error::AlreadyExists)));
    }

    // -----------------------------------------------------------------------
    // publish_program
    // -----------------------------------------------------------------------

    #[test]
    fn test_publish_transitions_draft_to_active() {
        let (env, admin, client) = setup();
        let token = make_token(&env, &admin);
        client.create_program(&pid(&env, "p-001"), &pid(&env, "Test"), &token);
        client.publish_program(&pid(&env, "p-001"));
        let p = client.get_program(&pid(&env, "p-001")).unwrap();
        assert_eq!(p.status, ProgramStatus::Active);
        assert!(p.published_at.is_some());
    }

    #[test]
    fn test_publish_active_program_fails() {
        let (env, admin, client) = setup();
        let token = make_token(&env, &admin);
        client.create_program(&pid(&env, "p-001"), &pid(&env, "Test"), &token);
        client.publish_program(&pid(&env, "p-001"));
        let result = client.try_publish_program(&pid(&env, "p-001"));
        assert_eq!(result, Err(Ok(Error::InvalidStatus)));
    }

    #[test]
    fn test_publish_nonexistent_program_fails() {
        let (_, _, client) = setup();
        let result = client.try_publish_program(&client.get_admin().unwrap().to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_publish_completed_program_fails() {
        let (env, admin, client) = setup();
        let token = make_token(&env, &admin);
        client.create_program(&pid(&env, "p-001"), &pid(&env, "Test"), &token);
        client.publish_program(&pid(&env, "p-001"));
        client.complete_program(&pid(&env, "p-001"));
        let result = client.try_publish_program(&pid(&env, "p-001"));
        assert_eq!(result, Err(Ok(Error::InvalidStatus)));
    }

    // -----------------------------------------------------------------------
    // deposit_funds
    // -----------------------------------------------------------------------

    #[test]
    fn test_deposit_into_draft_fails() {
        let (env, admin, client) = setup();
        let token = make_token(&env, &admin);
        client.create_program(&pid(&env, "p-001"), &pid(&env, "Test"), &token);
        let depositor = Address::generate(&env);
        let result = client.try_deposit_funds(&pid(&env, "p-001"), &depositor, &1000);
        assert_eq!(result, Err(Ok(Error::InvalidStatus)));
    }

    #[test]
    fn test_deposit_zero_fails() {
        let (env, admin, client) = setup();
        let token = make_token(&env, &admin);
        client.create_program(&pid(&env, "p-001"), &pid(&env, "Test"), &token);
        client.publish_program(&pid(&env, "p-001"));
        let depositor = Address::generate(&env);
        let result = client.try_deposit_funds(&pid(&env, "p-001"), &depositor, &0);
        assert_eq!(result, Err(Ok(Error::InvalidAmount)));
    }

    // -----------------------------------------------------------------------
    // complete_program
    // -----------------------------------------------------------------------

    #[test]
    fn test_complete_active_program() {
        let (env, admin, client) = setup();
        let token = make_token(&env, &admin);
        client.create_program(&pid(&env, "p-001"), &pid(&env, "Test"), &token);
        client.publish_program(&pid(&env, "p-001"));
        client.complete_program(&pid(&env, "p-001"));
        let p = client.get_program(&pid(&env, "p-001")).unwrap();
        assert_eq!(p.status, ProgramStatus::Completed);
        assert_eq!(p.balance, 0);
    }

    #[test]
    fn test_complete_draft_fails() {
        let (env, admin, client) = setup();
        let token = make_token(&env, &admin);
        client.create_program(&pid(&env, "p-001"), &pid(&env, "Test"), &token);
        let result = client.try_complete_program(&pid(&env, "p-001"));
        assert_eq!(result, Err(Ok(Error::InvalidStatus)));
    }

    #[test]
    fn test_complete_twice_fails() {
        let (env, admin, client) = setup();
        let token = make_token(&env, &admin);
        client.create_program(&pid(&env, "p-001"), &pid(&env, "Test"), &token);
        client.publish_program(&pid(&env, "p-001"));
        client.complete_program(&pid(&env, "p-001"));
        let result = client.try_complete_program(&pid(&env, "p-001"));
        assert_eq!(result, Err(Ok(Error::InvalidStatus)));
    }

    // -----------------------------------------------------------------------
    // cancel_program
    // -----------------------------------------------------------------------

    #[test]
    fn test_cancel_draft_program() {
        let (env, admin, client) = setup();
        let token = make_token(&env, &admin);
        let refund = Address::generate(&env);
        client.create_program(&pid(&env, "p-001"), &pid(&env, "Test"), &token);
        client.cancel_program(&pid(&env, "p-001"), &refund);
        let p = client.get_program(&pid(&env, "p-001")).unwrap();
        assert_eq!(p.status, ProgramStatus::Cancelled);
    }

    #[test]
    fn test_cancel_active_program() {
        let (env, admin, client) = setup();
        let token = make_token(&env, &admin);
        let refund = Address::generate(&env);
        client.create_program(&pid(&env, "p-001"), &pid(&env, "Test"), &token);
        client.publish_program(&pid(&env, "p-001"));
        client.cancel_program(&pid(&env, "p-001"), &refund);
        let p = client.get_program(&pid(&env, "p-001")).unwrap();
        assert_eq!(p.status, ProgramStatus::Cancelled);
    }

    #[test]
    fn test_cancel_completed_fails() {
        let (env, admin, client) = setup();
        let token = make_token(&env, &admin);
        let refund = Address::generate(&env);
        client.create_program(&pid(&env, "p-001"), &pid(&env, "Test"), &token);
        client.publish_program(&pid(&env, "p-001"));
        client.complete_program(&pid(&env, "p-001"));
        let result = client.try_cancel_program(&pid(&env, "p-001"), &refund);
        assert_eq!(result, Err(Ok(Error::InvalidStatus)));
    }

    #[test]
    fn test_cancel_twice_fails() {
        let (env, admin, client) = setup();
        let token = make_token(&env, &admin);
        let refund = Address::generate(&env);
        client.create_program(&pid(&env, "p-001"), &pid(&env, "Test"), &token);
        client.cancel_program(&pid(&env, "p-001"), &refund);
        let result = client.try_cancel_program(&pid(&env, "p-001"), &refund);
        assert_eq!(result, Err(Ok(Error::InvalidStatus)));
    }

    // -----------------------------------------------------------------------
    // Full lifecycle
    // -----------------------------------------------------------------------

    #[test]
    fn test_full_lifecycle_draft_to_completed() {
        let (env, admin, client) = setup();
        let token = make_token(&env, &admin);

        client.create_program(&pid(&env, "p-001"), &pid(&env, "Hackathon"), &token);
        assert_eq!(client.get_program(&pid(&env, "p-001")).unwrap().status, ProgramStatus::Draft);

        client.publish_program(&pid(&env, "p-001"));
        let p = client.get_program(&pid(&env, "p-001")).unwrap();
        assert_eq!(p.status, ProgramStatus::Active);
        assert!(p.published_at.is_some());

        client.complete_program(&pid(&env, "p-001"));
        assert_eq!(client.get_program(&pid(&env, "p-001")).unwrap().status, ProgramStatus::Completed);
    }

    #[test]
    fn test_published_at_is_set_only_on_publish() {
        let (env, admin, client) = setup();
        let token = make_token(&env, &admin);
        client.create_program(&pid(&env, "p-001"), &pid(&env, "Test"), &token);
        assert_eq!(client.get_program(&pid(&env, "p-001")).unwrap().published_at, None);
        client.publish_program(&pid(&env, "p-001"));
        assert!(client.get_program(&pid(&env, "p-001")).unwrap().published_at.is_some());
    }
}

// ---------------------------------------------------------------------------
// STATE: Draft - Refund Operations Blocked
// Refund operations should fail when program is in Draft status.
// ---------------------------------------------------------------------------

/// cancel_claim should fail when program is in Draft status.
#[test]
#[should_panic(expected = "107")]
fn test_cancel_claim_fails_on_draft_program() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, contract_id) = make_client(&env);
    let (_token_client, token_id) = fund_contract(&env, &contract_id, 50_000);
    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "draft-prog");
    
    // Initialize program but do NOT publish - stays in Draft status
    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    
    // Attempt to cancel a claim on Draft program should fail with ProgramNotActive (107)
    client.cancel_claim(&program_id, 1, &admin);
}

/// emergency_withdraw should fail when program is in Draft status.
#[test]
#[should_panic(expected = "107")]
fn test_emergency_withdraw_fails_on_draft_program() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, contract_id) = make_client(&env);
    let (_token_client, token_id) = fund_contract(&env, &contract_id, 50_000);
    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "draft-prog");
    
    // Initialize program but do NOT publish - stays in Draft status
    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    
    // Set lock_paused to true (required for emergency_withdraw)
    client.set_pause_flags(&Some(true), &Some(false), &Some(false), &None);
    
    // Attempt emergency withdraw on Draft program should fail with ProgramNotActive (107)
    let target = Address::generate(&env);
    client.emergency_withdraw(&target);
}

/// cancel_claim should succeed when program is in Active status.
#[test]
fn test_cancel_claim_succeeds_on_active_program() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, contract_id) = make_client(&env);
    let (_token_client, token_id) = fund_contract(&env, &contract_id, 50_000);
    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "active-prog");
    
    // Initialize and publish program - transitions to Active status
    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    client.publish_program(&program_id);
    client.lock_program_funds(&50_000);
    
    // Create a claim first
    let recipient = Address::generate(&env);
    client.create_claim(&program_id, &recipient, &10_000, &86400);
    
    // Cancel claim should succeed on Active program
    client.cancel_claim(&program_id, 1, &admin);
}

/// emergency_withdraw should succeed when program is in Active status.
#[test]
fn test_emergency_withdraw_succeeds_on_active_program() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, contract_id) = make_client(&env);
    let (token_client, token_id) = fund_contract(&env, &contract_id, 50_000);
    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "active-prog");
    
    // Initialize and publish program - transitions to Active status
    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    client.publish_program(&program_id);
    client.lock_program_funds(&50_000);
    
    // Set lock_paused to true (required for emergency_withdraw)
    client.set_pause_flags(&Some(true), &Some(false), &Some(false), &None);
    
    // Emergency withdraw should succeed on Active program
    let target = Address::generate(&env);
    let initial_balance = token_client.balance(&target);
    client.emergency_withdraw(&target);
    let final_balance = token_client.balance(&target);
    
    // Target should have received the funds
    assert_eq!(final_balance - initial_balance, 50_000);
}
