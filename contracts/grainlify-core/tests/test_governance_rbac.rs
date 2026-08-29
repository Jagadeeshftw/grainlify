#![cfg(test)]

use soroban_sdk::{symbol_short, Address, BytesN, Env, Symbol};
use grainlify_core::governance::{
    self, GovernanceContract, GovernanceConfig, ProposalStatus, VotingScheme, Role, Error, VoteType,
};

fn default_config(env: &Env) -> GovernanceConfig {
    GovernanceConfig {
        voting_period: 100,
        execution_delay: 50,
        quorum_percentage: 5000,
        approval_threshold: 6000,
        min_proposal_stake: 0,
        voting_scheme: VotingScheme::OnePersonOneVote,
        governance_token: Address::generate(env),
    }
}

fn setup_governance(env: &Env) -> (Address, Address, Address, Address, Address) {
    env.mock_all_auths();
    let admin = Address::generate(env);
    let emergency = Address::generate(env);
    let upgrade = Address::generate(env);
    let config_role = Address::generate(env);
    let other = Address::generate(env);

    let cfg = default_config(env);
    GovernanceContract::init_governance_state(env.clone(), admin.clone(), cfg).unwrap();

    GovernanceContract::set_emergency_role(env.clone(), admin.clone(), emergency.clone()).unwrap();
    GovernanceContract::set_upgrade_role(env.clone(), admin.clone(), upgrade.clone()).unwrap();
    GovernanceContract::set_config_role(env.clone(), admin.clone(), config_role.clone()).unwrap();

    (admin, emergency, upgrade, config_role, other)
}

// ============================================================================
// TEST CATEGORY 1: Authorized vs. Unauthorized — every mutating entrypoint
// ============================================================================

#[test]
fn test_set_emergency_role_authorized_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let new_holder = Address::generate(&env);
    let cfg = default_config(&env);
    GovernanceContract::init_governance_state(env.clone(), admin.clone(), cfg).unwrap();

    let result = GovernanceContract::set_emergency_role(env.clone(), admin.clone(), new_holder.clone());
    assert!(result.is_ok());
    assert_eq!(governance::get_role_holder(&env, Role::Emergency), Some(new_holder));
}

#[test]
fn test_set_emergency_role_unauthorized_fails() {
    let env = Env::default();
    let (admin, _emerg, _upg, _cfg, other) = setup_governance(&env);
    let new_holder = Address::generate(&env);

    let result = GovernanceContract::set_emergency_role(env.clone(), other.clone(), new_holder.clone());
    assert_eq!(result, Err(Error::NotAuthorizedForRole));

    let original = governance::get_role_holder(&env, Role::Emergency).unwrap();
    assert_ne!(original, new_holder);
}

#[test]
fn test_set_upgrade_role_authorized_succeeds() {
    let env = Env::default();
    let (admin, _e, _u, _c, _o) = setup_governance(&env);
    let new_holder = Address::generate(&env);

    let result = GovernanceContract::set_upgrade_role(env.clone(), admin.clone(), new_holder.clone());
    assert!(result.is_ok());
    assert_eq!(governance::get_role_holder(&env, Role::Upgrade), Some(new_holder));
}

#[test]
fn test_set_upgrade_role_unauthorized_fails() {
    let env = Env::default();
    let (_admin, _e, _u, _c, other) = setup_governance(&env);
    let new_holder = Address::generate(&env);

    let result = GovernanceContract::set_upgrade_role(env.clone(), other.clone(), new_holder.clone());
    assert_eq!(result, Err(Error::NotAuthorizedForRole));
}

#[test]
fn test_set_config_role_authorized_succeeds() {
    let env = Env::default();
    let (admin, _e, _u, _c, _o) = setup_governance(&env);
    let new_holder = Address::generate(&env);

    let result = GovernanceContract::set_config_role(env.clone(), admin.clone(), new_holder.clone());
    assert!(result.is_ok());
    assert_eq!(governance::get_role_holder(&env, Role::Config), Some(new_holder));
}

#[test]
fn test_set_config_role_unauthorized_fails() {
    let env = Env::default();
    let (_admin, _e, _u, _c, other) = setup_governance(&env);
    let new_holder = Address::generate(&env);

    let result = GovernanceContract::set_config_role(env.clone(), other.clone(), new_holder.clone());
    assert_eq!(result, Err(Error::NotAuthorizedForRole));
}

#[test]
fn test_set_security_council_authorized_succeeds() {
    let env = Env::default();
    let (admin, _e, _u, _c, _o) = setup_governance(&env);
    let council = Address::generate(&env);

    let result = GovernanceContract::set_security_council(env.clone(), admin.clone(), council.clone());
    assert!(result.is_ok());
    assert_eq!(GovernanceContract::get_security_council(env.clone()).unwrap(), council);
}

#[test]
fn test_set_security_council_unauthorized_fails() {
    let env = Env::default();
    let (_admin, _e, _u, _c, other) = setup_governance(&env);
    let council = Address::generate(&env);

    let result = GovernanceContract::set_security_council(env.clone(), other.clone(), council.clone());
    assert_eq!(result, Err(Error::NotAuthorizedForRole));
}

#[test]
fn test_rotate_admin_authorized_succeeds() {
    let env = Env::default();
    let (admin, _e, _u, _c, _o) = setup_governance(&env);
    let new_admin = Address::generate(&env);

    let result = GovernanceContract::rotate_admin(env.clone(), admin.clone(), new_admin.clone());
    assert!(result.is_ok());

    let pending = GovernanceContract::get_pending_admin_rotation(env.clone()).unwrap();
    assert_eq!(pending.proposed_admin, new_admin);
}

#[test]
fn test_rotate_admin_unauthorized_fails() {
    let env = Env::default();
    let (_admin, _e, _u, _c, other) = setup_governance(&env);
    let new_admin = Address::generate(&env);

    let result = GovernanceContract::rotate_admin(env.clone(), other.clone(), new_admin.clone());
    assert_eq!(result, Err(Error::NotAuthorizedForRole));
    assert!(GovernanceContract::get_pending_admin_rotation(env.clone()).is_none());
}

#[test]
fn test_emergency_pause_authorized_succeeds() {
    let env = Env::default();
    let (_admin, emergency, _u, _c, _o) = setup_governance(&env);

    let result = GovernanceContract::emergency_pause(env.clone(), emergency.clone());
    assert!(result.is_ok());
    assert!(governance::is_emergency_paused(&env));
}

#[test]
fn test_emergency_pause_unauthorized_fails() {
    let env = Env::default();
    let (_admin, _e, _u, _c, other) = setup_governance(&env);

    let result = GovernanceContract::emergency_pause(env.clone(), other.clone());
    assert_eq!(result, Err(Error::NotAuthorizedForRole));
    assert!(!governance::is_emergency_paused(&env));
}

#[test]
fn test_emergency_unpause_authorized_succeeds() {
    let env = Env::default();
    let (_admin, emergency, _u, _c, _o) = setup_governance(&env);

    GovernanceContract::emergency_pause(env.clone(), emergency.clone()).unwrap();
    assert!(governance::is_emergency_paused(&env));

    let result = GovernanceContract::emergency_unpause(env.clone(), emergency.clone());
    assert!(result.is_ok());
    assert!(!governance::is_emergency_paused(&env));
}

#[test]
fn test_emergency_unpause_unauthorized_fails() {
    let env = Env::default();
    let (_admin, emergency, _u, _c, other) = setup_governance(&env);

    GovernanceContract::emergency_pause(env.clone(), emergency.clone()).unwrap();
    assert!(governance::is_emergency_paused(&env));

    let result = GovernanceContract::emergency_unpause(env.clone(), other.clone());
    assert_eq!(result, Err(Error::NotAuthorizedForRole));
    assert!(governance::is_emergency_paused(&env));
}

#[test]
fn test_update_config_authorized_succeeds() {
    let env = Env::default();
    let (_admin, _e, _u, config_role, _o) = setup_governance(&env);

    let result = GovernanceContract::update_governance_config(
        env.clone(),
        config_role.clone(),
        200,
        100,
        6000,
        7000,
        0,
    );
    assert!(result.is_ok());

    let cfg = GovernanceContract::get_config(env.clone()).unwrap();
    assert_eq!(cfg.voting_period, 200);
    assert_eq!(cfg.execution_delay, 100);
    assert_eq!(cfg.quorum_percentage, 6000);
    assert_eq!(cfg.approval_threshold, 7000);
}

#[test]
fn test_update_config_unauthorized_fails() {
    let env = Env::default();
    let (_admin, _e, _u, _c, other) = setup_governance(&env);

    let original = GovernanceContract::get_config(env.clone()).unwrap();
    let result = GovernanceContract::update_governance_config(
        env.clone(),
        other.clone(),
        9999,
        9999,
        9999,
        9999,
        0,
    );
    assert_eq!(result, Err(Error::NotAuthorizedForRole));

    let after = GovernanceContract::get_config(env.clone()).unwrap();
    assert_eq!(after.voting_period, original.voting_period);
    assert_eq!(after.execution_delay, original.execution_delay);
}

#[test]
fn test_update_config_blocked_by_emergency_pause() {
    let env = Env::default();
    let (_admin, emergency, _u, config_role, _o) = setup_governance(&env);

    GovernanceContract::emergency_pause(env.clone(), emergency.clone()).unwrap();

    let result = GovernanceContract::update_governance_config(
        env.clone(),
        config_role.clone(),
        200,
        100,
        6000,
        7000,
        0,
    );
    assert_eq!(result, Err(Error::EmergencyPaused));
}

#[test]
fn test_execute_proposal_authorized_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let upgrade_role = Address::generate(&env);
    let proposer = Address::generate(&env);
    let dummy_hash = BytesN::from_array(&env, &[0u8; 32]);

    let cfg = default_config(&env);
    GovernanceContract::init_governance_state(env.clone(), admin.clone(), cfg).unwrap();
    GovernanceContract::set_upgrade_role(env.clone(), admin.clone(), upgrade_role.clone()).unwrap();

    let proposal_id = GovernanceContract::create_proposal(
        env.clone(),
        proposer.clone(),
        dummy_hash.clone(),
        symbol_short!("test"),
    ).unwrap();

    env.ledger().set_timestamp(101);
    GovernanceContract::finalize_proposal(env.clone(), proposal_id).unwrap();
    env.ledger().set_timestamp(151);

    let result = GovernanceContract::execute_proposal(env.clone(), upgrade_role.clone(), proposal_id);
    assert!(result.is_ok());
}

#[test]
fn test_execute_proposal_unauthorized_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let upgrade_role = Address::generate(&env);
    let other = Address::generate(&env);
    let proposer = Address::generate(&env);
    let dummy_hash = BytesN::from_array(&env, &[0u8; 32]);

    let cfg = default_config(&env);
    GovernanceContract::init_governance_state(env.clone(), admin.clone(), cfg).unwrap();
    GovernanceContract::set_upgrade_role(env.clone(), admin.clone(), upgrade_role.clone()).unwrap();

    let proposal_id = GovernanceContract::create_proposal(
        env.clone(),
        proposer.clone(),
        dummy_hash.clone(),
        symbol_short!("test"),
    ).unwrap();

    env.ledger().set_timestamp(101);
    GovernanceContract::finalize_proposal(env.clone(), proposal_id).unwrap();
    env.ledger().set_timestamp(151);

    let result = GovernanceContract::execute_proposal(env.clone(), other.clone(), proposal_id);
    assert_eq!(result, Err(Error::NotAuthorizedForRole));
}

#[test]
fn test_create_proposal_blocked_by_emergency_pause() {
    let env = Env::default();
    let (_admin, emergency, _u, _c, _o) = setup_governance(&env);
    let proposer = Address::generate(&env);
    let dummy_hash = BytesN::from_array(&env, &[0u8; 32]);

    GovernanceContract::emergency_pause(env.clone(), emergency.clone()).unwrap();

    let result = GovernanceContract::create_proposal(
        env.clone(),
        proposer.clone(),
        dummy_hash.clone(),
        symbol_short!("test"),
    );
    assert_eq!(result, Err(Error::EmergencyPaused));
}

#[test]
fn test_cast_vote_blocked_by_emergency_pause() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let emergency = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);
    let dummy_hash = BytesN::from_array(&env, &[0u8; 32]);

    let cfg = default_config(&env);
    GovernanceContract::init_governance_state(env.clone(), admin.clone(), cfg).unwrap();
    GovernanceContract::set_emergency_role(env.clone(), admin.clone(), emergency.clone()).unwrap();

    let proposal_id = GovernanceContract::create_proposal(
        env.clone(),
        proposer.clone(),
        dummy_hash.clone(),
        symbol_short!("test"),
    ).unwrap();

    GovernanceContract::emergency_pause(env.clone(), emergency.clone()).unwrap();

    let result = GovernanceContract::cast_vote(
        env.clone(),
        voter.clone(),
        proposal_id,
        VoteType::For,
    );
    assert_eq!(result, Err(Error::EmergencyPaused));
}

#[test]
fn test_finalize_proposal_blocked_by_emergency_pause() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let emergency = Address::generate(&env);
    let proposer = Address::generate(&env);
    let dummy_hash = BytesN::from_array(&env, &[0u8; 32]);

    let cfg = default_config(&env);
    GovernanceContract::init_governance_state(env.clone(), admin.clone(), cfg).unwrap();
    GovernanceContract::set_emergency_role(env.clone(), admin.clone(), emergency.clone()).unwrap();

    let proposal_id = GovernanceContract::create_proposal(
        env.clone(),
        proposer.clone(),
        dummy_hash.clone(),
        symbol_short!("test"),
    ).unwrap();

    env.ledger().set_timestamp(101);
    GovernanceContract::emergency_pause(env.clone(), emergency.clone()).unwrap();

    let result = GovernanceContract::finalize_proposal(env.clone(), proposal_id);
    assert_eq!(result, Err(Error::EmergencyPaused));
}

#[test]
fn test_execute_proposal_blocked_by_emergency_pause() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let emergency = Address::generate(&env);
    let upgrade_role = Address::generate(&env);
    let proposer = Address::generate(&env);
    let dummy_hash = BytesN::from_array(&env, &[0u8; 32]);

    let cfg = default_config(&env);
    GovernanceContract::init_governance_state(env.clone(), admin.clone(), cfg).unwrap();
    GovernanceContract::set_emergency_role(env.clone(), admin.clone(), emergency.clone()).unwrap();
    GovernanceContract::set_upgrade_role(env.clone(), admin.clone(), upgrade_role.clone()).unwrap();

    let proposal_id = GovernanceContract::create_proposal(
        env.clone(),
        proposer.clone(),
        dummy_hash.clone(),
        symbol_short!("test"),
    ).unwrap();

    env.ledger().set_timestamp(101);
    GovernanceContract::finalize_proposal(env.clone(), proposal_id).unwrap();
    env.ledger().set_timestamp(151);
    GovernanceContract::emergency_pause(env.clone(), emergency.clone()).unwrap();

    let result = GovernanceContract::execute_proposal(env.clone(), upgrade_role.clone(), proposal_id);
    assert_eq!(result, Err(Error::EmergencyPaused));
}

// ============================================================================
// TEST CATEGORY 2: Rotated Admin — old admin fails, new admin succeeds
// ============================================================================

#[test]
fn test_rotated_admin_old_fails_new_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let old_admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let cfg = default_config(&env);

    GovernanceContract::init_governance_state(env.clone(), old_admin.clone(), cfg).unwrap();
    assert_eq!(governance::get_role_holder(&env, Role::Admin), Some(old_admin.clone()));

    GovernanceContract::rotate_admin(env.clone(), old_admin.clone(), new_admin.clone()).unwrap();
    GovernanceContract::confirm_admin_rotation(env.clone(), new_admin.clone()).unwrap();

    assert_eq!(governance::get_role_holder(&env, Role::Admin), Some(new_admin.clone()));

    let some_new_holder = Address::generate(&env);
    let old_result = GovernanceContract::set_emergency_role(env.clone(), old_admin.clone(), some_new_holder.clone());
    assert_eq!(old_result, Err(Error::NotAuthorizedForRole));

    let another_holder = Address::generate(&env);
    let new_result = GovernanceContract::set_emergency_role(env.clone(), new_admin.clone(), another_holder.clone());
    assert!(new_result.is_ok());
    assert_eq!(governance::get_role_holder(&env, Role::Emergency), Some(another_holder));

    let old_rotate_result = GovernanceContract::rotate_admin(env.clone(), old_admin.clone(), Address::generate(&env));
    assert_eq!(old_rotate_result, Err(Error::NotAuthorizedForRole));

    let even_newer = Address::generate(&env);
    let new_rotate_result = GovernanceContract::rotate_admin(env.clone(), new_admin.clone(), even_newer.clone());
    assert!(new_rotate_result.is_ok());
}

#[test]
fn test_rotated_admin_cannot_confirm_twice() {
    let env = Env::default();
    env.mock_all_auths();
    let old_admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let cfg = default_config(&env);

    GovernanceContract::init_governance_state(env.clone(), old_admin.clone(), cfg).unwrap();
    GovernanceContract::rotate_admin(env.clone(), old_admin.clone(), new_admin.clone()).unwrap();
    GovernanceContract::confirm_admin_rotation(env.clone(), new_admin.clone()).unwrap();

    let result = GovernanceContract::confirm_admin_rotation(env.clone(), new_admin.clone());
    assert_eq!(result, Err(Error::NoPendingAdminRotation));
}

#[test]
fn test_rotated_admin_wrong_address_cannot_confirm() {
    let env = Env::default();
    env.mock_all_auths();
    let old_admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let imposter = Address::generate(&env);
    let cfg = default_config(&env);

    GovernanceContract::init_governance_state(env.clone(), old_admin.clone(), cfg).unwrap();
    GovernanceContract::rotate_admin(env.clone(), old_admin.clone(), new_admin.clone()).unwrap();

    let result = GovernanceContract::confirm_admin_rotation(env.clone(), imposter.clone());
    assert_eq!(result, Err(Error::NoPendingAdminRotation));

    assert_eq!(governance::get_role_holder(&env, Role::Admin), Some(old_admin.clone()));
}

// ============================================================================
// TEST CATEGORY 3: Expired Capability
// ============================================================================

#[test]
fn test_expired_pending_admin_rotation_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let old_admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let cfg = default_config(&env);

    GovernanceContract::init_governance_state(env.clone(), old_admin.clone(), cfg).unwrap();

    let start_ts = env.ledger().timestamp();
    GovernanceContract::rotate_admin(env.clone(), old_admin.clone(), new_admin.clone()).unwrap();

    let pending = GovernanceContract::get_pending_admin_rotation(env.clone()).unwrap();
    assert!(pending.expires_at > start_ts);

    env.ledger().set_timestamp(pending.expires_at.saturating_add(1));

    let result = GovernanceContract::confirm_admin_rotation(env.clone(), new_admin.clone());
    assert_eq!(result, Err(Error::PendingAdminExpired));

    assert_eq!(governance::get_role_holder(&env, Role::Admin), Some(old_admin.clone()));

    assert!(GovernanceContract::get_pending_admin_rotation(env.clone()).is_none());
}

#[test]
fn test_expired_admin_rotation_cleans_storage() {
    let env = Env::default();
    env.mock_all_auths();
    let old_admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let cfg = default_config(&env);

    GovernanceContract::init_governance_state(env.clone(), old_admin.clone(), cfg).unwrap();
    GovernanceContract::rotate_admin(env.clone(), old_admin.clone(), new_admin.clone()).unwrap();
    assert!(GovernanceContract::get_pending_admin_rotation(env.clone()).is_some());

    let pending = GovernanceContract::get_pending_admin_rotation(env.clone()).unwrap();
    env.ledger().set_timestamp(pending.expires_at + 1000);

    let _ = GovernanceContract::confirm_admin_rotation(env.clone(), new_admin.clone());
    assert!(GovernanceContract::get_pending_admin_rotation(env.clone()).is_none());
}

// ============================================================================
// TEST CATEGORY 4: Zero-Address / Invalid-Address Guards
// ============================================================================

fn zero_address(env: &Env) -> Address {
    use soroban_sdk::IntoVal;
    let bytes = [0u8; 32];
    Address::from_val(env, &bytes.into_val(env))
}

#[test]
fn test_init_governance_zero_admin_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let zero = zero_address(&env);
    let cfg = default_config(&env);

    let result = GovernanceContract::init_governance_state(env.clone(), zero, cfg);
    assert_eq!(result, Err(Error::InvalidRoleHolder));
    assert!(!env.storage().instance().has(&governance::ROLE_ADMIN));
}

#[test]
fn test_rotate_admin_zero_new_admin_fails() {
    let env = Env::default();
    let (admin, _e, _u, _c, _o) = setup_governance(&env);
    let zero = zero_address(&env);

    let result = GovernanceContract::rotate_admin(env.clone(), admin.clone(), zero);
    assert_eq!(result, Err(Error::InvalidRoleHolder));
    assert!(GovernanceContract::get_pending_admin_rotation(env.clone()).is_none());
}

#[test]
fn test_set_emergency_role_zero_address_fails() {
    let env = Env::default();
    let (admin, _e, _u, _c, _o) = setup_governance(&env);
    let zero = zero_address(&env);
    let original = governance::get_role_holder(&env, Role::Emergency).unwrap();

    let result = GovernanceContract::set_emergency_role(env.clone(), admin.clone(), zero);
    assert_eq!(result, Err(Error::InvalidRoleHolder));
    assert_eq!(governance::get_role_holder(&env, Role::Emergency), Some(original));
}

#[test]
fn test_set_upgrade_role_zero_address_fails() {
    let env = Env::default();
    let (admin, _e, _u, _c, _o) = setup_governance(&env);
    let zero = zero_address(&env);

    let result = GovernanceContract::set_upgrade_role(env.clone(), admin.clone(), zero);
    assert_eq!(result, Err(Error::InvalidRoleHolder));
}

#[test]
fn test_set_config_role_zero_address_fails() {
    let env = Env::default();
    let (admin, _e, _u, _c, _o) = setup_governance(&env);
    let zero = zero_address(&env);

    let result = GovernanceContract::set_config_role(env.clone(), admin.clone(), zero);
    assert_eq!(result, Err(Error::InvalidRoleHolder));
}

#[test]
fn test_set_security_council_zero_address_fails() {
    let env = Env::default();
    let (admin, _e, _u, _c, _o) = setup_governance(&env);
    let zero = zero_address(&env);

    let result = GovernanceContract::set_security_council(env.clone(), admin.clone(), zero);
    assert_eq!(result, Err(Error::InvalidRoleHolder));
}

#[test]
fn test_set_emergency_role_contract_self_address_fails() {
    let env = Env::default();
    let (admin, _e, _u, _c, _o) = setup_governance(&env);
    let self_addr = env.current_contract_address();
    let original = governance::get_role_holder(&env, Role::Emergency).unwrap();

    let result = GovernanceContract::set_emergency_role(env.clone(), admin.clone(), self_addr.clone());
    assert_eq!(result, Err(Error::InvalidRoleHolder));
    assert_eq!(governance::get_role_holder(&env, Role::Emergency), Some(original));
}

#[test]
fn test_rotate_admin_contract_self_address_fails() {
    let env = Env::default();
    let (admin, _e, _u, _c, _o) = setup_governance(&env);
    let self_addr = env.current_contract_address();

    let result = GovernanceContract::rotate_admin(env.clone(), admin.clone(), self_addr);
    assert_eq!(result, Err(Error::InvalidRoleHolder));
    assert!(GovernanceContract::get_pending_admin_rotation(env.clone()).is_none());
}

// ============================================================================
// TEST CATEGORY 5: Config role — cross-role boundary violations
// ============================================================================

#[test]
fn test_config_role_cannot_set_emergency_role() {
    let env = Env::default();
    let (_admin, _e, _u, config_role, _o) = setup_governance(&env);
    let new_holder = Address::generate(&env);

    let result = GovernanceContract::set_emergency_role(env.clone(), config_role.clone(), new_holder);
    assert_eq!(result, Err(Error::NotAuthorizedForRole));
}

#[test]
fn test_emergency_role_cannot_update_config() {
    let env = Env::default();
    let (_admin, emergency, _u, _c, _o) = setup_governance(&env);

    let result = GovernanceContract::update_governance_config(
        env.clone(),
        emergency.clone(),
        999,
        999,
        5000,
        6000,
        0,
    );
    assert_eq!(result, Err(Error::NotAuthorizedForRole));
}

#[test]
fn test_upgrade_role_cannot_pause() {
    let env = Env::default();
    let (_admin, _e, upgrade_role, _c, _o) = setup_governance(&env);

    let result = GovernanceContract::emergency_pause(env.clone(), upgrade_role.clone());
    assert_eq!(result, Err(Error::NotAuthorizedForRole));
    assert!(!governance::is_emergency_paused(&env));
}

// ============================================================================
// TEST CATEGORY 6: Event auditing — role change events emitted
// ============================================================================

#[test]
fn test_role_change_emits_event_emergency() {
    let env = Env::default();
    let (admin, _e, _u, _c, _o) = setup_governance(&env);
    let new_holder = Address::generate(&env);

    GovernanceContract::set_emergency_role(env.clone(), admin.clone(), new_holder.clone()).unwrap();

    let events = env.events().all();
    assert!(events.len() > 0);
    let last_event = events.last().unwrap();
    let topics = last_event.topics();
    let first_topic: Symbol = topics.get(0).unwrap().into_val(&env);
    assert_eq!(first_topic, symbol_short!("emg_role"));
}

#[test]
fn test_admin_rotation_emits_events() {
    let env = Env::default();
    env.mock_all_auths();
    let old_admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let cfg = default_config(&env);

    GovernanceContract::init_governance_state(env.clone(), old_admin.clone(), cfg).unwrap();
    let event_count_before = env.events().all().len();

    GovernanceContract::rotate_admin(env.clone(), old_admin.clone(), new_admin.clone()).unwrap();
    let events_after_propose = env.events().all();
    let propose_event = events_after_propose.last().unwrap();
    let propose_topic: Symbol = propose_event.topics().get(0).unwrap().into_val(&env);
    assert_eq!(propose_topic, symbol_short!("adm_rot_p"));

    GovernanceContract::confirm_admin_rotation(env.clone(), new_admin.clone()).unwrap();
    let events_after_confirm = env.events().all();
    let confirm_event = events_after_confirm.last().unwrap();
    let confirm_topic: Symbol = confirm_event.topics().get(0).unwrap().into_val(&env);
    assert_eq!(confirm_topic, symbol_short!("adm_rot_c"));
}

#[test]
fn test_pause_unpause_emit_events() {
    let env = Env::default();
    let (_admin, emergency, _u, _c, _o) = setup_governance(&env);

    let before = env.events().all().len();
    GovernanceContract::emergency_pause(env.clone(), emergency.clone()).unwrap();
    let pause_event = env.events().all().last().unwrap();
    let pause_topic: Symbol = pause_event.topics().get(0).unwrap().into_val(&env);
    assert_eq!(pause_topic, symbol_short!("emg_pause"));

    GovernanceContract::emergency_unpause(env.clone(), emergency.clone()).unwrap();
    let unpause_event = env.events().all().last().unwrap();
    let unpause_topic: Symbol = unpause_event.topics().get(0).unwrap().into_val(&env);
    assert_eq!(unpause_topic, symbol_short!("emg_unp"));
}

#[test]
fn test_config_update_emits_event() {
    let env = Env::default();
    let (_admin, _e, _u, config_role, _o) = setup_governance(&env);

    GovernanceContract::update_governance_config(
        env.clone(),
        config_role.clone(),
        200,
        100,
        5500,
        6500,
        0,
    ).unwrap();

    let last_event = env.events().all().last().unwrap();
    let topic: Symbol = last_event.topics().get(0).unwrap().into_val(&env);
    assert_eq!(topic, symbol_short!("gov_cfgup"));
}

// ============================================================================
// TEST CATEGORY 7: Double init protection
// ============================================================================

#[test]
fn test_cannot_init_governance_twice() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let cfg1 = default_config(&env);
    GovernanceContract::init_governance_state(env.clone(), admin.clone(), cfg1).unwrap();

    let admin2 = Address::generate(&env);
    let cfg2 = default_config(&env);
    let result = GovernanceContract::init_governance_state(env.clone(), admin2, cfg2);
    assert_eq!(result, Err(Error::AlreadyInitialized));

    assert_eq!(governance::get_role_holder(&env, Role::Admin), Some(admin));
}
