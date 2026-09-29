#![cfg(test)]

use grainlify_core::governance::{
    self, Error, GovernanceConfig, GovernanceContract, ProposalStatus, Role, VoteType, VotingScheme,
};
use soroban_sdk::{symbol_short, Address, BytesN, Env, Symbol};
use grainlify_core::governance::{Error, GovernanceConfig, Role, VoteType, VotingScheme};
use soroban_sdk::testutils::{Address as _, Events as _, Ledger as _};
use soroban_sdk::{symbol_short, Address, BytesN, Env, FromVal as _, Symbol};

mod gov_support;

/// Generates a fresh address that is guaranteed not to be the all-zero contract
/// address.
///
/// `Address::generate` yields the all-zero address as its first value on a fresh
/// `Env`, and the governance contract rejects that address as a role holder, so
/// the first (zero) value is skipped.
fn actor(env: &Env) -> Address {
    let candidate = Address::generate(env);
    if candidate == zero_address(env) {
        Address::generate(env)
    } else {
        candidate
    }
}

fn default_config(env: &Env) -> GovernanceConfig {
    GovernanceConfig {
        voting_period: 100,
        execution_delay: 50,
        quorum_percentage: 5000,
        approval_threshold: 6000,
        min_proposal_stake: 0,
        voting_scheme: VotingScheme::OnePersonOneVote,
        governance_token: actor(env),
    }
}

fn setup_governance(env: &Env) -> (Address, Address, Address, Address, Address) {
    env.mock_all_auths();
    let admin = actor(env);
    let emergency = actor(env);
    let upgrade = actor(env);
    let config_role = actor(env);
    let other = actor(env);

    let cfg = default_config(env);
    gov_support::init_governance_state(env.clone(), admin.clone(), cfg).unwrap();

    gov_support::set_emergency_role(env.clone(), admin.clone(), emergency.clone()).unwrap();
    gov_support::set_upgrade_role(env.clone(), admin.clone(), upgrade.clone()).unwrap();
    gov_support::set_config_role(env.clone(), admin.clone(), config_role.clone()).unwrap();

    (admin, emergency, upgrade, config_role, other)
}

// ============================================================================
// TEST CATEGORY 1: Authorized vs. Unauthorized — every mutating entrypoint
// ============================================================================

#[test]
fn test_set_emergency_role_authorized_succeeds() {
    let env = Env::default();
    gov_support::reset();
    env.mock_all_auths();
    let admin = actor(&env);
    let new_holder = actor(&env);
    let cfg = default_config(&env);
    gov_support::init_governance_state(env.clone(), admin.clone(), cfg).unwrap();

    let result =
        GovernanceContract::set_emergency_role(env.clone(), admin.clone(), new_holder.clone());
    assert!(result.is_ok());
    assert_eq!(
        governance::get_role_holder(&env, Role::Emergency),
    let result = gov_support::set_emergency_role(env.clone(), admin.clone(), new_holder.clone());
    assert!(result.is_ok());
    assert_eq!(
        gov_support::get_role_holder(&env, Role::Emergency),
        Some(new_holder)
    );
}

#[test]
fn test_set_emergency_role_unauthorized_fails() {
    let env = Env::default();
    gov_support::reset();
    let (_admin, _emerg, _upg, _cfg, other) = setup_governance(&env);
    let new_holder = actor(&env);

    let result =
        GovernanceContract::set_emergency_role(env.clone(), other.clone(), new_holder.clone());
    let result = gov_support::set_emergency_role(env.clone(), other.clone(), new_holder.clone());
    assert_eq!(result, Err(Error::NotAuthorizedForRole));

    let original = gov_support::get_role_holder(&env, Role::Emergency).unwrap();
    assert_ne!(original, new_holder);
}

#[test]
fn test_set_upgrade_role_authorized_succeeds() {
    let env = Env::default();
    gov_support::reset();
    let (admin, _e, _u, _c, _o) = setup_governance(&env);
    let new_holder = actor(&env);

    let result =
        GovernanceContract::set_upgrade_role(env.clone(), admin.clone(), new_holder.clone());
    assert!(result.is_ok());
    assert_eq!(
        governance::get_role_holder(&env, Role::Upgrade),
    let result = gov_support::set_upgrade_role(env.clone(), admin.clone(), new_holder.clone());
    assert!(result.is_ok());
    assert_eq!(
        gov_support::get_role_holder(&env, Role::Upgrade),
        Some(new_holder)
    );
}

#[test]
fn test_set_upgrade_role_unauthorized_fails() {
    let env = Env::default();
    gov_support::reset();
    let (_admin, _e, _u, _c, other) = setup_governance(&env);
    let new_holder = actor(&env);

    let result =
        GovernanceContract::set_upgrade_role(env.clone(), other.clone(), new_holder.clone());
    let result = gov_support::set_upgrade_role(env.clone(), other.clone(), new_holder.clone());
    assert_eq!(result, Err(Error::NotAuthorizedForRole));
}

#[test]
fn test_set_config_role_authorized_succeeds() {
    let env = Env::default();
    gov_support::reset();
    let (admin, _e, _u, _c, _o) = setup_governance(&env);
    let new_holder = actor(&env);

    let result =
        GovernanceContract::set_config_role(env.clone(), admin.clone(), new_holder.clone());
    assert!(result.is_ok());
    assert_eq!(
        governance::get_role_holder(&env, Role::Config),
    let result = gov_support::set_config_role(env.clone(), admin.clone(), new_holder.clone());
    assert!(result.is_ok());
    assert_eq!(
        gov_support::get_role_holder(&env, Role::Config),
        Some(new_holder)
    );
}

#[test]
fn test_set_config_role_unauthorized_fails() {
    let env = Env::default();
    gov_support::reset();
    let (_admin, _e, _u, _c, other) = setup_governance(&env);
    let new_holder = actor(&env);

    let result =
        GovernanceContract::set_config_role(env.clone(), other.clone(), new_holder.clone());
    let result = gov_support::set_config_role(env.clone(), other.clone(), new_holder.clone());
    assert_eq!(result, Err(Error::NotAuthorizedForRole));
}

#[test]
fn test_set_security_council_authorized_succeeds() {
    let env = Env::default();
    gov_support::reset();
    let (admin, _e, _u, _c, _o) = setup_governance(&env);
    let council = actor(&env);

    let result =
        GovernanceContract::set_security_council(env.clone(), admin.clone(), council.clone());
    assert!(result.is_ok());
    assert_eq!(
        GovernanceContract::get_security_council(env.clone()).unwrap(),
    let result = gov_support::set_security_council(env.clone(), admin.clone(), council.clone());
    assert!(result.is_ok());
    assert_eq!(
        gov_support::get_security_council(env.clone()).unwrap(),
        council
    );
}

#[test]
fn test_set_security_council_unauthorized_fails() {
    let env = Env::default();
    gov_support::reset();
    let (_admin, _e, _u, _c, other) = setup_governance(&env);
    let council = actor(&env);

    let result =
        GovernanceContract::set_security_council(env.clone(), other.clone(), council.clone());
    let result = gov_support::set_security_council(env.clone(), other.clone(), council.clone());
    assert_eq!(result, Err(Error::NotAuthorizedForRole));
}

#[test]
fn test_rotate_admin_authorized_succeeds() {
    let env = Env::default();
    gov_support::reset();
    let (admin, _e, _u, _c, _o) = setup_governance(&env);
    let new_admin = actor(&env);

    let result = gov_support::rotate_admin(env.clone(), admin.clone(), new_admin.clone());
    assert!(result.is_ok());

    let pending = gov_support::get_pending_admin_rotation(env.clone()).unwrap();
    assert_eq!(pending.proposed_admin, new_admin);
}

#[test]
fn test_rotate_admin_unauthorized_fails() {
    let env = Env::default();
    gov_support::reset();
    let (_admin, _e, _u, _c, other) = setup_governance(&env);
    let new_admin = actor(&env);

    let result = gov_support::rotate_admin(env.clone(), other.clone(), new_admin.clone());
    assert_eq!(result, Err(Error::NotAuthorizedForRole));
    assert!(gov_support::get_pending_admin_rotation(env.clone()).is_none());
}

#[test]
fn test_emergency_pause_authorized_succeeds() {
    let env = Env::default();
    gov_support::reset();
    let (_admin, emergency, _u, _c, _o) = setup_governance(&env);

    let result = gov_support::emergency_pause(env.clone(), emergency.clone());
    assert!(result.is_ok());
    assert!(gov_support::is_emergency_paused(&env));
}

#[test]
fn test_emergency_pause_unauthorized_fails() {
    let env = Env::default();
    gov_support::reset();
    let (_admin, _e, _u, _c, other) = setup_governance(&env);

    let result = gov_support::emergency_pause(env.clone(), other.clone());
    assert_eq!(result, Err(Error::NotAuthorizedForRole));
    assert!(!gov_support::is_emergency_paused(&env));
}

#[test]
fn test_emergency_unpause_authorized_succeeds() {
    let env = Env::default();
    gov_support::reset();
    let (_admin, emergency, _u, _c, _o) = setup_governance(&env);

    gov_support::emergency_pause(env.clone(), emergency.clone()).unwrap();
    assert!(gov_support::is_emergency_paused(&env));

    let result = gov_support::emergency_unpause(env.clone(), emergency.clone());
    assert!(result.is_ok());
    assert!(!gov_support::is_emergency_paused(&env));
}

#[test]
fn test_emergency_unpause_unauthorized_fails() {
    let env = Env::default();
    gov_support::reset();
    let (_admin, emergency, _u, _c, other) = setup_governance(&env);

    gov_support::emergency_pause(env.clone(), emergency.clone()).unwrap();
    assert!(gov_support::is_emergency_paused(&env));

    let result = gov_support::emergency_unpause(env.clone(), other.clone());
    assert_eq!(result, Err(Error::NotAuthorizedForRole));
    assert!(gov_support::is_emergency_paused(&env));
}

#[test]
fn test_update_config_authorized_succeeds() {
    let env = Env::default();
    gov_support::reset();
    let (_admin, _e, _u, config_role, _o) = setup_governance(&env);

    let result = gov_support::update_governance_config(
        env.clone(),
        config_role.clone(),
        200,
        100,
        6000,
        7000,
        0,
    );
    assert!(result.is_ok());

    let cfg = gov_support::get_config(env.clone()).unwrap();
    assert_eq!(cfg.voting_period, 200);
    assert_eq!(cfg.execution_delay, 100);
    assert_eq!(cfg.quorum_percentage, 6000);
    assert_eq!(cfg.approval_threshold, 7000);
}

#[test]
fn test_update_config_unauthorized_fails() {
    let env = Env::default();
    gov_support::reset();
    let (_admin, _e, _u, _c, other) = setup_governance(&env);

    let original = gov_support::get_config(env.clone()).unwrap();
    let result = gov_support::update_governance_config(
        env.clone(),
        other.clone(),
        9999,
        9999,
        9999,
        9999,
        0,
    );
    assert_eq!(result, Err(Error::NotAuthorizedForRole));

    let after = gov_support::get_config(env.clone()).unwrap();
    assert_eq!(after.voting_period, original.voting_period);
    assert_eq!(after.execution_delay, original.execution_delay);
}

#[test]
fn test_update_config_blocked_by_emergency_pause() {
    let env = Env::default();
    gov_support::reset();
    let (_admin, emergency, _u, config_role, _o) = setup_governance(&env);

    gov_support::emergency_pause(env.clone(), emergency.clone()).unwrap();

    let result = gov_support::update_governance_config(
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
    gov_support::reset();
    env.mock_all_auths();
    let admin = actor(&env);
    let upgrade_role = actor(&env);
    let proposer = actor(&env);
    let dummy_hash = BytesN::from_array(&env, &[0u8; 32]);

    let mut cfg = default_config(&env);
    // A single "For" vote meets a 1% quorum, so the proposal reaches Approved.
    cfg.quorum_percentage = 100;
    gov_support::init_governance_state(env.clone(), admin.clone(), cfg).unwrap();
    gov_support::set_upgrade_role(env.clone(), admin.clone(), upgrade_role.clone()).unwrap();

    let proposal_id = gov_support::create_proposal(
        env.clone(),
        proposer.clone(),
        dummy_hash.clone(),
        symbol_short!("test"),
    )
    .unwrap();
    gov_support::cast_vote(env.clone(), proposer.clone(), proposal_id, VoteType::For).unwrap();

    env.ledger().set_timestamp(101);
    gov_support::finalize_proposal(env.clone(), proposal_id).unwrap();
    env.ledger().set_timestamp(151);

    let result =
        GovernanceContract::execute_proposal(env.clone(), upgrade_role.clone(), proposal_id);
    let result = gov_support::execute_proposal(env.clone(), upgrade_role.clone(), proposal_id);
    assert!(result.is_ok());
}

#[test]
fn test_execute_proposal_unauthorized_fails() {
    let env = Env::default();
    gov_support::reset();
    env.mock_all_auths();
    let admin = actor(&env);
    let upgrade_role = actor(&env);
    let other = actor(&env);
    let proposer = actor(&env);
    let dummy_hash = BytesN::from_array(&env, &[0u8; 32]);

    let cfg = default_config(&env);
    gov_support::init_governance_state(env.clone(), admin.clone(), cfg).unwrap();
    gov_support::set_upgrade_role(env.clone(), admin.clone(), upgrade_role.clone()).unwrap();

    let proposal_id = gov_support::create_proposal(
        env.clone(),
        proposer.clone(),
        dummy_hash.clone(),
        symbol_short!("test"),
    )
    .unwrap();

    env.ledger().set_timestamp(101);
    gov_support::finalize_proposal(env.clone(), proposal_id).unwrap();
    env.ledger().set_timestamp(151);

    let result = gov_support::execute_proposal(env.clone(), other.clone(), proposal_id);
    assert_eq!(result, Err(Error::NotAuthorizedForRole));
}

#[test]
fn test_create_proposal_blocked_by_emergency_pause() {
    let env = Env::default();
    gov_support::reset();
    let (_admin, emergency, _u, _c, _o) = setup_governance(&env);
    let proposer = actor(&env);
    let dummy_hash = BytesN::from_array(&env, &[0u8; 32]);

    gov_support::emergency_pause(env.clone(), emergency.clone()).unwrap();

    let result = gov_support::create_proposal(
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
    gov_support::reset();
    env.mock_all_auths();
    let admin = actor(&env);
    let emergency = actor(&env);
    let proposer = actor(&env);
    let voter = actor(&env);
    let dummy_hash = BytesN::from_array(&env, &[0u8; 32]);

    let cfg = default_config(&env);
    gov_support::init_governance_state(env.clone(), admin.clone(), cfg).unwrap();
    gov_support::set_emergency_role(env.clone(), admin.clone(), emergency.clone()).unwrap();

    let proposal_id = gov_support::create_proposal(
        env.clone(),
        proposer.clone(),
        dummy_hash.clone(),
        symbol_short!("test"),
    )
    .unwrap();

    gov_support::emergency_pause(env.clone(), emergency.clone()).unwrap();

    let result =
        GovernanceContract::cast_vote(env.clone(), voter.clone(), proposal_id, VoteType::For);
    let result = gov_support::cast_vote(env.clone(), voter.clone(), proposal_id, VoteType::For);
    assert_eq!(result, Err(Error::EmergencyPaused));
}

#[test]
fn test_finalize_proposal_blocked_by_emergency_pause() {
    let env = Env::default();
    gov_support::reset();
    env.mock_all_auths();
    let admin = actor(&env);
    let emergency = actor(&env);
    let proposer = actor(&env);
    let dummy_hash = BytesN::from_array(&env, &[0u8; 32]);

    let cfg = default_config(&env);
    gov_support::init_governance_state(env.clone(), admin.clone(), cfg).unwrap();
    gov_support::set_emergency_role(env.clone(), admin.clone(), emergency.clone()).unwrap();

    let proposal_id = gov_support::create_proposal(
        env.clone(),
        proposer.clone(),
        dummy_hash.clone(),
        symbol_short!("test"),
    )
    .unwrap();

    env.ledger().set_timestamp(101);
    gov_support::emergency_pause(env.clone(), emergency.clone()).unwrap();

    let result = gov_support::finalize_proposal(env.clone(), proposal_id);
    assert_eq!(result, Err(Error::EmergencyPaused));
}

#[test]
fn test_execute_proposal_blocked_by_emergency_pause() {
    let env = Env::default();
    gov_support::reset();
    env.mock_all_auths();
    let admin = actor(&env);
    let emergency = actor(&env);
    let upgrade_role = actor(&env);
    let proposer = actor(&env);
    let dummy_hash = BytesN::from_array(&env, &[0u8; 32]);

    let cfg = default_config(&env);
    gov_support::init_governance_state(env.clone(), admin.clone(), cfg).unwrap();
    gov_support::set_emergency_role(env.clone(), admin.clone(), emergency.clone()).unwrap();
    gov_support::set_upgrade_role(env.clone(), admin.clone(), upgrade_role.clone()).unwrap();

    let proposal_id = gov_support::create_proposal(
        env.clone(),
        proposer.clone(),
        dummy_hash.clone(),
        symbol_short!("test"),
    )
    .unwrap();

    env.ledger().set_timestamp(101);
    gov_support::finalize_proposal(env.clone(), proposal_id).unwrap();
    env.ledger().set_timestamp(151);
    gov_support::emergency_pause(env.clone(), emergency.clone()).unwrap();

    let result =
        GovernanceContract::execute_proposal(env.clone(), upgrade_role.clone(), proposal_id);
    let result = gov_support::execute_proposal(env.clone(), upgrade_role.clone(), proposal_id);
    assert_eq!(result, Err(Error::EmergencyPaused));
}

// ============================================================================
// TEST CATEGORY 2: Rotated Admin — old admin fails, new admin succeeds
// ============================================================================

#[test]
fn test_rotated_admin_old_fails_new_succeeds() {
    let env = Env::default();
    gov_support::reset();
    env.mock_all_auths();
    let old_admin = actor(&env);
    let new_admin = actor(&env);
    let cfg = default_config(&env);

    GovernanceContract::init_governance_state(env.clone(), old_admin.clone(), cfg).unwrap();
    assert_eq!(
        governance::get_role_holder(&env, Role::Admin),
    gov_support::init_governance_state(env.clone(), old_admin.clone(), cfg).unwrap();
    assert_eq!(
        gov_support::get_role_holder(&env, Role::Admin),
        Some(old_admin.clone())
    );

    gov_support::rotate_admin(env.clone(), old_admin.clone(), new_admin.clone()).unwrap();
    gov_support::confirm_admin_rotation(env.clone(), new_admin.clone()).unwrap();

    assert_eq!(
        governance::get_role_holder(&env, Role::Admin),
        Some(new_admin.clone())
    );

    let some_new_holder = Address::generate(&env);
    let old_result = GovernanceContract::set_emergency_role(
        env.clone(),
        old_admin.clone(),
        some_new_holder.clone(),
    );
    assert_eq!(old_result, Err(Error::NotAuthorizedForRole));

    let another_holder = Address::generate(&env);
    let new_result = GovernanceContract::set_emergency_role(
        env.clone(),
        new_admin.clone(),
        another_holder.clone(),
    );
    assert!(new_result.is_ok());
    assert_eq!(
        governance::get_role_holder(&env, Role::Emergency),
        Some(another_holder)
    );

    let old_rotate_result =
        GovernanceContract::rotate_admin(env.clone(), old_admin.clone(), Address::generate(&env));
    assert_eq!(old_rotate_result, Err(Error::NotAuthorizedForRole));

    let even_newer = Address::generate(&env);
    let new_rotate_result =
        GovernanceContract::rotate_admin(env.clone(), new_admin.clone(), even_newer.clone());
        gov_support::get_role_holder(&env, Role::Admin),
        Some(new_admin.clone())
    );

    let some_new_holder = actor(&env);
    let old_result =
        gov_support::set_emergency_role(env.clone(), old_admin.clone(), some_new_holder.clone());
    assert_eq!(old_result, Err(Error::NotAuthorizedForRole));

    let another_holder = actor(&env);
    let new_result =
        gov_support::set_emergency_role(env.clone(), new_admin.clone(), another_holder.clone());
    assert!(new_result.is_ok());
    assert_eq!(
        gov_support::get_role_holder(&env, Role::Emergency),
        Some(another_holder)
    );

    let old_rotate_result = gov_support::rotate_admin(env.clone(), old_admin.clone(), actor(&env));
    assert_eq!(old_rotate_result, Err(Error::NotAuthorizedForRole));

    let even_newer = actor(&env);
    let new_rotate_result =
        gov_support::rotate_admin(env.clone(), new_admin.clone(), even_newer.clone());
    assert!(new_rotate_result.is_ok());
}

#[test]
fn test_rotated_admin_cannot_confirm_twice() {
    let env = Env::default();
    gov_support::reset();
    env.mock_all_auths();
    let old_admin = actor(&env);
    let new_admin = actor(&env);
    let cfg = default_config(&env);

    gov_support::init_governance_state(env.clone(), old_admin.clone(), cfg).unwrap();
    gov_support::rotate_admin(env.clone(), old_admin.clone(), new_admin.clone()).unwrap();
    gov_support::confirm_admin_rotation(env.clone(), new_admin.clone()).unwrap();

    let result = gov_support::confirm_admin_rotation(env.clone(), new_admin.clone());
    assert_eq!(result, Err(Error::NoPendingAdminRotation));
}

#[test]
fn test_rotated_admin_wrong_address_cannot_confirm() {
    let env = Env::default();
    gov_support::reset();
    env.mock_all_auths();
    let old_admin = actor(&env);
    let new_admin = actor(&env);
    let imposter = actor(&env);
    let cfg = default_config(&env);

    gov_support::init_governance_state(env.clone(), old_admin.clone(), cfg).unwrap();
    gov_support::rotate_admin(env.clone(), old_admin.clone(), new_admin.clone()).unwrap();

    let result = gov_support::confirm_admin_rotation(env.clone(), imposter.clone());
    assert_eq!(result, Err(Error::NoPendingAdminRotation));

    assert_eq!(
        governance::get_role_holder(&env, Role::Admin),
        gov_support::get_role_holder(&env, Role::Admin),
        Some(old_admin.clone())
    );
}

// ============================================================================
// TEST CATEGORY 3: Expired Capability
// ============================================================================

#[test]
fn test_expired_pending_admin_rotation_fails() {
    let env = Env::default();
    gov_support::reset();
    env.mock_all_auths();
    let old_admin = actor(&env);
    let new_admin = actor(&env);
    let cfg = default_config(&env);

    gov_support::init_governance_state(env.clone(), old_admin.clone(), cfg).unwrap();

    let start_ts = env.ledger().timestamp();
    gov_support::rotate_admin(env.clone(), old_admin.clone(), new_admin.clone()).unwrap();

    let pending = gov_support::get_pending_admin_rotation(env.clone()).unwrap();
    assert!(pending.expires_at > start_ts);

    env.ledger()
        .set_timestamp(pending.expires_at.saturating_add(1));

    let result = gov_support::confirm_admin_rotation(env.clone(), new_admin.clone());
    assert_eq!(result, Err(Error::PendingAdminExpired));

    assert_eq!(
        governance::get_role_holder(&env, Role::Admin),
        gov_support::get_role_holder(&env, Role::Admin),
        Some(old_admin.clone())
    );

    assert!(gov_support::get_pending_admin_rotation(env.clone()).is_none());
}

#[test]
fn test_expired_admin_rotation_cleans_storage() {
    let env = Env::default();
    gov_support::reset();
    env.mock_all_auths();
    let old_admin = actor(&env);
    let new_admin = actor(&env);
    let cfg = default_config(&env);

    gov_support::init_governance_state(env.clone(), old_admin.clone(), cfg).unwrap();
    gov_support::rotate_admin(env.clone(), old_admin.clone(), new_admin.clone()).unwrap();
    assert!(gov_support::get_pending_admin_rotation(env.clone()).is_some());

    let pending = gov_support::get_pending_admin_rotation(env.clone()).unwrap();
    env.ledger().set_timestamp(pending.expires_at + 1000);

    let _ = gov_support::confirm_admin_rotation(env.clone(), new_admin.clone());
    assert!(gov_support::get_pending_admin_rotation(env.clone()).is_none());
}

// ============================================================================
// TEST CATEGORY 4: Zero-Address / Invalid-Address Guards
// ============================================================================

fn zero_address(env: &Env) -> Address {
    // StrKey for the all-zero contract id — the canonical "zero address".
    use soroban_sdk::String;
    Address::from_string(&String::from_str(
        env,
        "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM",
    ))
}

#[test]
fn test_init_governance_zero_admin_fails() {
    let env = Env::default();
    gov_support::reset();
    env.mock_all_auths();
    let zero = zero_address(&env);
    let cfg = default_config(&env);

    let result = gov_support::init_governance_state(env.clone(), zero, cfg);
    assert_eq!(result, Err(Error::InvalidRoleHolder));
    assert!(gov_support::get_role_holder(&env, Role::Admin).is_none());
}

#[test]
fn test_rotate_admin_zero_new_admin_fails() {
    let env = Env::default();
    gov_support::reset();
    let (admin, _e, _u, _c, _o) = setup_governance(&env);
    let zero = zero_address(&env);

    let result = gov_support::rotate_admin(env.clone(), admin.clone(), zero);
    assert_eq!(result, Err(Error::InvalidRoleHolder));
    assert!(gov_support::get_pending_admin_rotation(env.clone()).is_none());
}

#[test]
fn test_set_emergency_role_zero_address_fails() {
    let env = Env::default();
    gov_support::reset();
    let (admin, _e, _u, _c, _o) = setup_governance(&env);
    let zero = zero_address(&env);
    let original = gov_support::get_role_holder(&env, Role::Emergency).unwrap();

    let result = gov_support::set_emergency_role(env.clone(), admin.clone(), zero);
    assert_eq!(result, Err(Error::InvalidRoleHolder));
    assert_eq!(
        governance::get_role_holder(&env, Role::Emergency),
        gov_support::get_role_holder(&env, Role::Emergency),
        Some(original)
    );
}

#[test]
fn test_set_upgrade_role_zero_address_fails() {
    let env = Env::default();
    gov_support::reset();
    let (admin, _e, _u, _c, _o) = setup_governance(&env);
    let zero = zero_address(&env);

    let result = gov_support::set_upgrade_role(env.clone(), admin.clone(), zero);
    assert_eq!(result, Err(Error::InvalidRoleHolder));
}

#[test]
fn test_set_config_role_zero_address_fails() {
    let env = Env::default();
    gov_support::reset();
    let (admin, _e, _u, _c, _o) = setup_governance(&env);
    let zero = zero_address(&env);

    let result = gov_support::set_config_role(env.clone(), admin.clone(), zero);
    assert_eq!(result, Err(Error::InvalidRoleHolder));
}

#[test]
fn test_set_security_council_zero_address_fails() {
    let env = Env::default();
    gov_support::reset();
    let (admin, _e, _u, _c, _o) = setup_governance(&env);
    let zero = zero_address(&env);

    let result = gov_support::set_security_council(env.clone(), admin.clone(), zero);
    assert_eq!(result, Err(Error::InvalidRoleHolder));
}

#[test]
fn test_set_emergency_role_contract_self_address_fails() {
    let env = Env::default();
    gov_support::reset();
    let (admin, _e, _u, _c, _o) = setup_governance(&env);
    let self_addr = gov_support::current_contract_address(&env);
    let original = gov_support::get_role_holder(&env, Role::Emergency).unwrap();

    let result =
        GovernanceContract::set_emergency_role(env.clone(), admin.clone(), self_addr.clone());
    assert_eq!(result, Err(Error::InvalidRoleHolder));
    assert_eq!(
        governance::get_role_holder(&env, Role::Emergency),
    let result = gov_support::set_emergency_role(env.clone(), admin.clone(), self_addr.clone());
    assert_eq!(result, Err(Error::InvalidRoleHolder));
    assert_eq!(
        gov_support::get_role_holder(&env, Role::Emergency),
        Some(original)
    );
}

#[test]
fn test_rotate_admin_contract_self_address_fails() {
    let env = Env::default();
    gov_support::reset();
    let (admin, _e, _u, _c, _o) = setup_governance(&env);
    let self_addr = gov_support::current_contract_address(&env);

    let result = gov_support::rotate_admin(env.clone(), admin.clone(), self_addr);
    assert_eq!(result, Err(Error::InvalidRoleHolder));
    assert!(gov_support::get_pending_admin_rotation(env.clone()).is_none());
}

// ============================================================================
// TEST CATEGORY 5: Config role — cross-role boundary violations
// ============================================================================

#[test]
fn test_config_role_cannot_set_emergency_role() {
    let env = Env::default();
    gov_support::reset();
    let (_admin, _e, _u, config_role, _o) = setup_governance(&env);
    let new_holder = actor(&env);

    let result =
        GovernanceContract::set_emergency_role(env.clone(), config_role.clone(), new_holder);
    let result = gov_support::set_emergency_role(env.clone(), config_role.clone(), new_holder);
    assert_eq!(result, Err(Error::NotAuthorizedForRole));
}

#[test]
fn test_emergency_role_cannot_update_config() {
    let env = Env::default();
    gov_support::reset();
    let (_admin, emergency, _u, _c, _o) = setup_governance(&env);

    let result = gov_support::update_governance_config(
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
    gov_support::reset();
    let (_admin, _e, upgrade_role, _c, _o) = setup_governance(&env);

    let result = gov_support::emergency_pause(env.clone(), upgrade_role.clone());
    assert_eq!(result, Err(Error::NotAuthorizedForRole));
    assert!(!gov_support::is_emergency_paused(&env));
}

// ============================================================================
// TEST CATEGORY 6: Event auditing — role change events emitted
// ============================================================================

#[test]
fn test_role_change_emits_event_emergency() {
    let env = Env::default();
    gov_support::reset();
    let (admin, _e, _u, _c, _o) = setup_governance(&env);
    let new_holder = actor(&env);

    gov_support::set_emergency_role(env.clone(), admin.clone(), new_holder.clone()).unwrap();

    let events = env.events().all();
    assert!(!events.is_empty());
    let last_event = events.last().unwrap();
    let topics = &last_event.1;
    let first_topic: Symbol = Symbol::from_val(&env, &topics.get(0).unwrap());
    assert_eq!(first_topic, symbol_short!("emg_role"));
}

#[test]
fn test_admin_rotation_emits_events() {
    let env = Env::default();
    gov_support::reset();
    env.mock_all_auths();
    let old_admin = actor(&env);
    let new_admin = actor(&env);
    let cfg = default_config(&env);

    gov_support::init_governance_state(env.clone(), old_admin.clone(), cfg).unwrap();
    let _event_count_before = env.events().all().len();

    gov_support::rotate_admin(env.clone(), old_admin.clone(), new_admin.clone()).unwrap();
    let events_after_propose = env.events().all();
    let propose_event = events_after_propose.last().unwrap();
    let propose_topic: Symbol = Symbol::from_val(&env, &propose_event.1.get(0).unwrap());
    assert_eq!(propose_topic, symbol_short!("adm_rot_p"));

    gov_support::confirm_admin_rotation(env.clone(), new_admin.clone()).unwrap();
    let events_after_confirm = env.events().all();
    let confirm_event = events_after_confirm.last().unwrap();
    let confirm_topic: Symbol = Symbol::from_val(&env, &confirm_event.1.get(0).unwrap());
    assert_eq!(confirm_topic, symbol_short!("adm_rot_c"));
}

#[test]
fn test_pause_unpause_emit_events() {
    let env = Env::default();
    gov_support::reset();
    let (_admin, emergency, _u, _c, _o) = setup_governance(&env);

    let _before = env.events().all().len();
    gov_support::emergency_pause(env.clone(), emergency.clone()).unwrap();
    let pause_event = env.events().all().last().unwrap();
    let pause_topic: Symbol = Symbol::from_val(&env, &pause_event.1.get(0).unwrap());
    assert_eq!(pause_topic, symbol_short!("emg_pause"));

    gov_support::emergency_unpause(env.clone(), emergency.clone()).unwrap();
    let unpause_event = env.events().all().last().unwrap();
    let unpause_topic: Symbol = Symbol::from_val(&env, &unpause_event.1.get(0).unwrap());
    assert_eq!(unpause_topic, symbol_short!("emg_unp"));
}

#[test]
fn test_config_update_emits_event() {
    let env = Env::default();
    gov_support::reset();
    let (_admin, _e, _u, config_role, _o) = setup_governance(&env);

    gov_support::update_governance_config(
        env.clone(),
        config_role.clone(),
        200,
        100,
        5500,
        6500,
        0,
    )
    .unwrap();

    let last_event = env.events().all().last().unwrap();
    let topic: Symbol = Symbol::from_val(&env, &last_event.1.get(0).unwrap());
    assert_eq!(topic, symbol_short!("gov_cfgup"));
}

// ============================================================================
// TEST CATEGORY 7: Double init protection
// ============================================================================

#[test]
fn test_cannot_init_governance_twice() {
    let env = Env::default();
    gov_support::reset();
    env.mock_all_auths();
    let admin = actor(&env);
    let cfg1 = default_config(&env);
    gov_support::init_governance_state(env.clone(), admin.clone(), cfg1).unwrap();

    let admin2 = actor(&env);
    let cfg2 = default_config(&env);
    let result = gov_support::init_governance_state(env.clone(), admin2, cfg2);
    assert_eq!(result, Err(Error::AlreadyInitialized));

    assert_eq!(gov_support::get_role_holder(&env, Role::Admin), Some(admin));
}
