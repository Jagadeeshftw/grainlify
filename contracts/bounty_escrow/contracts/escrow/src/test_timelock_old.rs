#[cfg(test)]
mod test_timelock {
    use super::*;
    use soroban_sdk::{
        Address,
        Env,
    };

    #[test]
    fn test_configure_timelock() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token = Address::generate(&env);

        // Initialize contract
        client.init(&admin, &token);

        // Test configuring timelock with valid delay
        let delay = 7200; // 2 hours
        client.configure_timelock(&delay, &true);

        let config = client.get_timelock_config();
        assert_eq!(config.delay, delay);
        assert_eq!(config.is_enabled, true);

        // Test configuring timelock with disabled
        client.configure_timelock(&86400, &false);
        let config = client.get_timelock_config();
        assert_eq!(config.delay, 86400);
        assert_eq!(config.is_enabled, false);
    }

    // Other timelock tests temporarily disabled due to mock_auths API changes
    // These can be re-enabled once the test API is stabilized

    #[test]
    #[should_panic(expected = "Error(Contract, #49)")] // TimelockEnabled
    fn test_direct_admin_call_blocked_when_enabled() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token = Address::generate(&env);

        client.init(&admin, &token);

        // Enable timelock
        client.configure_timelock(&7200, &true);

        // Try direct admin call - should be blocked
        client.set_maintenance_mode(&true, &None);
    }

    #[test]
    fn test_direct_admin_call_works_when_disabled() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token = Address::generate(&env);

        client.init(&admin, &token);

        // Ensure timelock is disabled
        client.configure_timelock(&7200, &false);

        // Direct admin call should work
        client.set_maintenance_mode(&true, &None);

        // Verify maintenance mode is set
        assert!(client.is_maintenance_mode());
    }

    #[test]
    fn test_get_pending_actions_ordered_by_time() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token = Address::generate(&env);
        let new_admin1 = Address::generate(&env);
        let new_admin2 = Address::generate(&env);

        client.init(&admin, &token);

        // Enable timelock
        client.configure_timelock(&7200, &true);

        // Propose first action
        let action_id1 = client.propose_admin_action(
            &ActionType::ChangeAdmin,
            &ActionPayload::ChangeAdmin(new_admin1),
        );

        // Advance time a bit
        env.ledger().set_timestamp(env.ledger().timestamp() + 100);

        // Propose second action
        let action_id2 = client.propose_admin_action(
            &ActionType::ChangeAdmin,
            &ActionPayload::ChangeAdmin(new_admin2),
        );

        // Get pending actions
        let pending = client.get_pending_actions();
        assert_eq!(pending.len(), 2);

        // Should be ordered by proposed_at (earliest first)
        assert_eq!(pending.get(0).unwrap().action_id, action_id1);
        assert_eq!(pending.get(1).unwrap().action_id, action_id2);
        assert!(pending.get(0).unwrap().proposed_at <= pending.get(1).unwrap().proposed_at);
    }

    #[test]
    fn test_change_fee_recipient_via_timelock() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token = Address::generate(&env);
        let new_recipient = Address::generate(&env);

        client.init(&admin, &token);

        // Enable timelock
        client.configure_timelock(&7200, &true);

        let start_timestamp = env.ledger().timestamp();

        // Propose fee recipient change
        let action_id = client.propose_admin_action(
            &ActionType::ChangeFeeRecipient,
            &ActionPayload::ChangeFeeRecipient(new_recipient.clone()),
        );

        // Advance time and execute
        env.ledger().set_timestamp(start_timestamp + 7200);
        client.execute_after_delay(&action_id);

        // Verify fee recipient changed
        let fee_config = client.get_fee_config();
        assert_eq!(fee_config.fee_recipient, new_recipient);
    }

    #[test]
    fn test_enable_kill_switch_via_timelock() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token = Address::generate(&env);

        client.init(&admin, &token);

        // Enable timelock
        client.configure_timelock(&7200, &true);

        let start_timestamp = env.ledger().timestamp();

        // Propose kill switch enable
        let action_id = client.propose_admin_action(
            &ActionType::EnableKillSwitch,
            &ActionPayload::EnableKillSwitch,
        );

        // Advance time and execute
        env.ledger().set_timestamp(start_timestamp + 7200);
        client.execute_after_delay(&action_id);

        // Verify deprecation state
        let deprecation_status = client.get_deprecation_status();
        assert!(deprecation_status.deprecated);
    }

    #[test]
    fn test_set_paused_via_timelock() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token = Address::generate(&env);

        client.init(&admin, &token);

        // Enable timelock
        client.configure_timelock(&7200, &true);

        let start_timestamp = env.ledger().timestamp();

        // Propose pause state change
        let action_id = client.propose_admin_action(
            &ActionType::SetPaused,
            &ActionPayload::SetPaused(Some(true), Some(false), Some(false)),
        );

        // Advance time and execute
        env.ledger().set_timestamp(start_timestamp + 7200);
        client.execute_after_delay(&action_id);

        // Verify pause flags
        let pause_flags = client.get_pause_flags();
        assert!(pause_flags.lock_paused);
        assert!(!pause_flags.release_paused);
        assert!(!pause_flags.refund_paused);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #53)")] // InvalidPayload
    fn test_invalid_payload_rejected() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token = Address::generate(&env);

        client.init(&admin, &token);

        // Enable timelock
        client.configure_timelock(&7200, &true);

        let start_timestamp = env.ledger().timestamp();

        // Propose invalid action
        let action_id = client.propose_admin_action(
            &ActionType::ChangeAdmin,
            &ActionPayload::EnableKillSwitch,
        );

        // Advance time and execute
        env.ledger().set_timestamp(start_timestamp + 7200);
        client.execute_after_delay(&action_id);
    }
}
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    // Enable timelock
    client.configure_timelock(&7200, &true);

    // Try direct admin call - should be blocked
    client.set_maintenance_mode(&true, &None);
}

#[test]
fn test_direct_admin_call_works_when_disabled() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    // Ensure timelock is disabled
    client.configure_timelock(&7200, &false);

    // Direct admin call should work
    client.set_maintenance_mode(&true, &None);

    // Verify maintenance mode is set
    assert!(client.is_maintenance_mode());
}

#[test]
fn test_get_pending_actions_ordered_by_time() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin1 = Address::generate(&env);
    let new_admin2 = Address::generate(&env);

    client.init(&admin, &token);

    // Enable timelock
    client.configure_timelock(&7200, &true);

    // Propose first action
    let action_id1 = client.propose_admin_action(
        &ActionType::ChangeAdmin,
        &ActionPayload::ChangeAdmin(new_admin1),
    );

    // Advance time a bit
    env.ledger().set_timestamp(env.ledger().timestamp() + 100);

    // Propose second action
    let action_id2 = client.propose_admin_action(
        &ActionType::ChangeAdmin,
        &ActionPayload::ChangeAdmin(new_admin2),
    );

    // Get pending actions
    let pending = client.get_pending_actions();
    assert_eq!(pending.len(), 2);

    // Should be ordered by proposed_at (earliest first)
    assert_eq!(pending.get(0).unwrap().action_id, action_id1);
    assert_eq!(pending.get(1).unwrap().action_id, action_id2);
    assert!(pending.get(0).unwrap().proposed_at <= pending.get(1).unwrap().proposed_at);
}

#[test]
fn test_change_fee_recipient_via_timelock() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_recipient = Address::generate(&env);

    client.init(&admin, &token);

    // Enable timelock
    client.configure_timelock(&7200, &true);

    let start_timestamp = env.ledger().timestamp();

    // Propose fee recipient change
    let action_id = client.propose_admin_action(
        &ActionType::ChangeFeeRecipient,
        &ActionPayload::ChangeFeeRecipient(new_recipient.clone()),
    );

    // Advance time and execute
    env.ledger().set_timestamp(start_timestamp + 7200);
    client.execute_after_delay(&action_id);

    // Verify fee recipient changed
    let fee_config = client.get_fee_config();
    assert_eq!(fee_config.fee_recipient, new_recipient);
}

#[test]
fn test_enable_kill_switch_via_timelock() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    // Enable timelock
    client.configure_timelock(&7200, &true);

    let start_timestamp = env.ledger().timestamp();

    // Propose kill switch enable
    let action_id = client.propose_admin_action(
        &ActionType::EnableKillSwitch,
        &ActionPayload::EnableKillSwitch,
    );

    // Advance time and execute
    env.ledger().set_timestamp(start_timestamp + 7200);
    client.execute_after_delay(&action_id);

    // Verify deprecation state
    let deprecation_status = client.get_deprecation_status();
    assert!(deprecation_status.deprecated);
}

#[test]
fn test_set_paused_via_timelock() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    // Enable timelock
    client.configure_timelock(&7200, &true);

    let start_timestamp = env.ledger().timestamp();

    // Propose pause state change
    let action_id = client.propose_admin_action(
        &ActionType::SetPaused,
        &ActionPayload::SetPaused(Some(true), Some(false), Some(false)),
    );

    // Advance time and execute
    env.ledger().set_timestamp(start_timestamp + 7200);
    client.execute_after_delay(&action_id);

    // Verify pause flags
    let pause_flags = client.get_pause_flags();
    assert!(pause_flags.lock_paused);
    assert!(!pause_flags.release_paused);
    assert!(!pause_flags.refund_paused);
}

#[test]
#[should_panic(expected = "Error(Contract, #53)")] // InvalidPayload
fn test_invalid_payload_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);

    // Enable timelock
    client.configure_timelock(&7200, &true);

    // Try to propose with mismatched payload
    client.propose_admin_action(
        &ActionType::ChangeAdmin,
        &ActionPayload::ChangeFeeRecipient(new_admin), // Wrong payload type
    );
}
