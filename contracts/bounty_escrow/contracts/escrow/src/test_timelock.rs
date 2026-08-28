use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env,
};

fn advance_ledger_time(env: &Env, seconds: u64) {
    env.ledger().set_timestamp(env.ledger().timestamp() + seconds);
}

#[test]
fn test_configure_timelock() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);

    let delay = 7_200u64;
    client.configure_timelock(&delay, &true);

    let config = client.get_timelock_config();
    assert_eq!(config.delay, delay);
    assert!(config.is_enabled);

    client.configure_timelock(&86_400, &false);
    let config = client.get_timelock_config();
    assert_eq!(config.delay, 86_400);
    assert!(!config.is_enabled);
}

#[test]
#[should_panic(expected = "Error(Contract, #57)")]
fn test_configure_timelock_below_minimum() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);
    client.configure_timelock(&3_599, &true);
}

#[test]
#[should_panic(expected = "Error(Contract, #58)")]
fn test_configure_timelock_above_maximum() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);
    client.configure_timelock(&2_592_001, &true);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")]
fn test_configure_timelock_unauthorized() {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let non_admin = Address::generate(&env);

    client.init(&admin, &token);
    env.mock_auths(&[&non_admin]);
    client.configure_timelock(&7_200, &true);
}

#[test]
fn test_propose_admin_action_immediate_execution_when_disabled() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);
    client.configure_timelock(&86_400, &false);

    let action_id = client.propose_admin_action(
        &ActionType::ChangeAdmin,
        &ActionPayload::ChangeAdmin(new_admin.clone()),
    );

    assert_eq!(action_id, 0);
    assert_eq!(client.get_admin(), new_admin);
    assert_eq!(client.get_pending_actions().len(), 0);
}

#[test]
fn test_propose_admin_action_creates_pending_when_enabled() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let delay = 7_200u64;

    client.init(&admin, &token);
    client.configure_timelock(&delay, &true);

    let current_timestamp = env.ledger().timestamp();
    let action_id = client.propose_admin_action(
        &ActionType::ChangeAdmin,
        &ActionPayload::ChangeAdmin(new_admin.clone()),
    );

    assert!(action_id > 0);
    let action = client.get_action(&action_id);
    assert_eq!(action.action_id, action_id);
    assert_eq!(action.action_type, ActionType::ChangeAdmin);
    assert_eq!(action.proposed_by, admin);
    assert_eq!(action.proposed_at, current_timestamp);
    assert_eq!(action.execute_after, current_timestamp + delay);
    assert_eq!(action.status, ActionStatus::Pending);
    assert_eq!(client.get_admin(), admin);
}

#[test]
#[should_panic(expected = "Error(Contract, #51)")]
fn test_execute_before_delay_reverts() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);
    client.configure_timelock(&7_200, &true);

    let action_id = client.propose_admin_action(
        &ActionType::ChangeAdmin,
        &ActionPayload::ChangeAdmin(new_admin),
    );
    client.execute_after_delay(&action_id);
}

#[test]
fn test_execute_at_exact_delay_succeeds() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let delay = 7_200u64;

    client.init(&admin, &token);
    client.configure_timelock(&delay, &true);

    let start_timestamp = env.ledger().timestamp();
    let action_id = client.propose_admin_action(
        &ActionType::ChangeAdmin,
        &ActionPayload::ChangeAdmin(new_admin.clone()),
    );

    advance_ledger_time(&env, delay);
    client.execute_after_delay(&action_id);

    assert_eq!(client.get_admin(), new_admin);
    assert_eq!(client.get_action(&action_id).status, ActionStatus::Executed);
}

#[test]
fn test_execute_after_delay_succeeds() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let delay = 7_200u64;

    client.init(&admin, &token);
    client.configure_timelock(&delay, &true);

    let start_timestamp = env.ledger().timestamp();
    let action_id = client.propose_admin_action(
        &ActionType::ChangeAdmin,
        &ActionPayload::ChangeAdmin(new_admin.clone()),
    );

    env.ledger().set_timestamp(start_timestamp + delay + 100);
    client.execute_after_delay(&action_id);

    assert_eq!(client.get_admin(), new_admin);
    assert_eq!(client.get_action(&action_id).status, ActionStatus::Executed);
}

#[test]
#[should_panic(expected = "Error(Contract, #54)")]
fn test_execute_already_executed_reverts() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let delay = 7_200u64;

    client.init(&admin, &token);
    client.configure_timelock(&delay, &true);

    let start_timestamp = env.ledger().timestamp();
    let action_id = client.propose_admin_action(
        &ActionType::ChangeAdmin,
        &ActionPayload::ChangeAdmin(new_admin),
    );

    env.ledger().set_timestamp(start_timestamp + delay);
    client.execute_after_delay(&action_id);
    client.execute_after_delay(&action_id);
}

#[test]
fn test_cancel_pending_action() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);
    client.configure_timelock(&7_200, &true);

    let action_id = client.propose_admin_action(
        &ActionType::ChangeAdmin,
        &ActionPayload::ChangeAdmin(new_admin),
    );

    client.cancel_admin_action(&action_id);
    let action = client.get_action(&action_id);
    assert_eq!(action.status, ActionStatus::Cancelled);
    assert_eq!(client.get_admin(), admin);
}

#[test]
#[should_panic(expected = "Error(Contract, #55)")]
fn test_execute_cancelled_action_reverts() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);
    client.configure_timelock(&7_200, &true);

    let action_id = client.propose_admin_action(
        &ActionType::ChangeAdmin,
        &ActionPayload::ChangeAdmin(new_admin),
    );

    client.cancel_admin_action(&action_id);
    client.execute_after_delay(&action_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #54)")]
fn test_cancel_executed_action_reverts() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);
    client.configure_timelock(&7_200, &true);

    let start_timestamp = env.ledger().timestamp();
    let action_id = client.propose_admin_action(
        &ActionType::ChangeAdmin,
        &ActionPayload::ChangeAdmin(new_admin),
    );

    env.ledger().set_timestamp(start_timestamp + 7_200);
    client.execute_after_delay(&action_id);
    client.cancel_admin_action(&action_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")]
fn test_only_admin_can_cancel() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let non_admin = Address::generate(&env);

    client.init(&admin, &token);
    client.configure_timelock(&7_200, &true);

    let action_id = client.propose_admin_action(
        &ActionType::ChangeAdmin,
        &ActionPayload::ChangeAdmin(new_admin),
    );

    env.mock_auths(&[&non_admin]);
    client.cancel_admin_action(&action_id);
}

#[test]
fn test_non_admin_can_execute_after_delay() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let executor = Address::generate(&env);

    client.init(&admin, &token);
    client.configure_timelock(&7_200, &true);

    let start_timestamp = env.ledger().timestamp();
    let action_id = client.propose_admin_action(
        &ActionType::ChangeAdmin,
        &ActionPayload::ChangeAdmin(new_admin.clone()),
    );

    env.ledger().set_timestamp(start_timestamp + 7_200);
    env.mock_auths(&[&executor]);
    client.execute_after_delay(&action_id);

    assert_eq!(client.get_admin(), new_admin);
}

#[test]
#[should_panic(expected = "Error(Contract, #52)")]
fn test_direct_admin_call_blocked_when_enabled() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &token);
    client.configure_timelock(&7_200, &true);
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
    client.configure_timelock(&7_200, &false);
    client.set_maintenance_mode(&true, &None);
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
    client.configure_timelock(&7_200, &true);

    let action_id1 = client.propose_admin_action(
        &ActionType::ChangeAdmin,
        &ActionPayload::ChangeAdmin(new_admin1),
    );
    advance_ledger_time(&env, 100);
    let action_id2 = client.propose_admin_action(
        &ActionType::ChangeAdmin,
        &ActionPayload::ChangeAdmin(new_admin2),
    );

    let pending = client.get_pending_actions();
    assert_eq!(pending.len(), 2);
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
    client.configure_timelock(&7_200, &true);

    let start_timestamp = env.ledger().timestamp();
    let action_id = client.propose_admin_action(
        &ActionType::ChangeFeeRecipient,
        &ActionPayload::ChangeFeeRecipient(new_recipient.clone()),
    );

    env.ledger().set_timestamp(start_timestamp + 7_200);
    client.execute_after_delay(&action_id);

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
    client.configure_timelock(&7_200, &true);

    let start_timestamp = env.ledger().timestamp();
    let action_id = client.propose_admin_action(
        &ActionType::EnableKillSwitch,
        &ActionPayload::EnableKillSwitch,
    );

    env.ledger().set_timestamp(start_timestamp + 7_200);
    client.execute_after_delay(&action_id);
    assert!(client.get_deprecation_status().deprecated);
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
    client.configure_timelock(&7_200, &true);

    let start_timestamp = env.ledger().timestamp();
    let action_id = client.propose_admin_action(
        &ActionType::SetPaused,
        &ActionPayload::SetPaused(Some(true), Some(false), Some(false)),
    );

    env.ledger().set_timestamp(start_timestamp + 7_200);
    client.execute_after_delay(&action_id);

    let pause_flags = client.get_pause_flags();
    assert!(pause_flags.lock_paused);
    assert!(!pause_flags.release_paused);
    assert!(!pause_flags.refund_paused);
}

#[test]
#[should_panic(expected = "Error(Contract, #56)")]
fn test_invalid_payload_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token);
    client.configure_timelock(&7_200, &true);

    client.propose_admin_action(
        &ActionType::ChangeAdmin,
        &ActionPayload::ChangeFeeRecipient(new_admin),
    );
}
