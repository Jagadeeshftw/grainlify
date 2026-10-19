use crate::{BountyEscrowContract, BountyEscrowContractClient};
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn setup() -> (Env, Address, BountyEscrowContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    client.init(&admin, &token);

    (env, admin, client)
}

#[test]
fn test_metadata_storage_and_query() {
    let (env, admin, client) = setup();

    let bounty_id = 1u64;
    let repo_id = 12345u64;
    let issue_id = 67890u64;
    let b_type = String::from_str(&env, "bounty");

    // 2. Set Metadata (requires admin auth)
    client.update_metadata(&admin, &bounty_id, &repo_id, &issue_id, &b_type, &None);

    // 3. Verify retrieval
    let fetched = client.get_metadata(&bounty_id);
    assert_eq!(fetched.repo_id, repo_id);
    assert_eq!(fetched.issue_id, issue_id);
    assert_eq!(fetched.bounty_type, b_type);
    assert_eq!(fetched.notification_prefs, 0);
}



#[test]
#[should_panic(expected = "Auth")]
fn test_update_metadata_unauthorized_caller_panics_with_auth_error() {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    client.init(&admin, &token);

    let bounty_id = 1u64;
    let repo_id = 12345u64;
    let issue_id = 67890u64;
    let bounty_type = String::from_str(&env, "bounty");
    let non_admin = Address::generate(&env);

    // Set empty auths so that require_auth will fail with Auth error
    env.set_auths(&[]);
    client.update_metadata(&non_admin, &bounty_id, &repo_id, &issue_id, &bounty_type, &None);
}

#[test]
fn test_update_metadata_unauthorized_does_not_mutate_state() {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    client.init(&admin, &token);

    let bounty_id = 1u64;
    let repo_id = 12345u64;
    let issue_id = 67890u64;
    let bounty_type = String::from_str(&env, "bounty");
    let non_admin = Address::generate(&env);

    // Get initial metadata state (should be default)
    let initial_meta = client.get_metadata(&bounty_id);
    assert_eq!(initial_meta.repo_id, 0);
    assert_eq!(initial_meta.issue_id, 0);

    // First, verify authorized caller succeeds with mock_all_auths
    env.mock_all_auths();
    client.update_metadata(&admin, &bounty_id, &repo_id, &issue_id, &bounty_type, &None);
    let meta_after_success = client.get_metadata(&bounty_id);
    assert_eq!(meta_after_success.repo_id, repo_id);
    assert_eq!(meta_after_success.issue_id, issue_id);
    assert_eq!(meta_after_success.bounty_type, bounty_type);

    // Now try with non-admin - should fail without mutating state
    // Set empty auths so that require_auth will fail
    env.set_auths(&[]);
    let result = client.try_update_metadata(
        &non_admin,
        &bounty_id,
        &repo_id,
        &issue_id,
        &bounty_type,
        &None,
    );
    // The call should fail (auth error)
    assert!(result.is_err());

    // Verify state was not mutated by the failed call
    let meta_after_failed = client.get_metadata(&bounty_id);
    assert_eq!(meta_after_failed.repo_id, repo_id);
    assert_eq!(meta_after_failed.issue_id, issue_id);
    assert_eq!(meta_after_failed.bounty_type, bounty_type);
}