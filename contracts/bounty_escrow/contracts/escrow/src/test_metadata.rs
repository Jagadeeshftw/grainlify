use crate::{BountyEscrowContract, BountyEscrowContractClient, NOTIFY_ON_LOCK, NOTIFY_ON_RELEASE};
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token, Address, Env, IntoVal, String, Symbol, TryIntoVal,
};

fn create_token_contract<'a>(env: &Env, admin: &Address) -> token::Client<'a> {
    let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
    let token_address = token_contract.address();
    token::Client::new(env, &token_address)
}

#[test]
fn test_metadata_storage_and_query() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    // 1. Initialize with your specific init(admin, token)
    client.init(&admin, &token);

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
fn test_notification_preferences_set_and_event() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = create_token_contract(&env, &token_admin);
    client.init(&admin, &token.address);

    let depositor = Address::generate(&env);
    let token_admin_client = soroban_sdk::token::StellarAssetClient::new(&env, &token.address);
    token_admin_client.mint(&depositor, &1_000i128);

    let bounty_id = 77u64;
    let amount = 1_000i128;
    env.ledger().with_mut(|li| {
        li.timestamp = 500;
    });
    let deadline = env.ledger().timestamp() + 600;
    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

    let prefs = NOTIFY_ON_LOCK | NOTIFY_ON_RELEASE;
    client.set_notification_preferences(&depositor, &bounty_id, &prefs);

    let fetched = client.get_metadata(&bounty_id);
    assert_eq!(fetched.notification_prefs, prefs);

    let events = env.events().all();
    let emitted = events.iter().last().unwrap();
    let topics = emitted.1;
    let topic_0: Symbol = topics.get(0).unwrap().into_val(&env);
    assert_eq!(topic_0, Symbol::new(&env, "npref"));

    let data: crate::events::NotificationPreferencesUpdated = emitted.2.try_into_val(&env).unwrap();
    assert_eq!(data.bounty_id, bounty_id);
    assert_eq!(data.previous_prefs, 0);
    assert_eq!(data.new_prefs, prefs);
    assert_eq!(data.created, true);
    assert_eq!(data.timestamp, 500);
}

#[test]
#[should_panic(expected = "bounty_type exceeds maximum length of 50 characters")]
fn test_metadata_rejects_oversized_bounty_type() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    client.init(&admin, &token);

    let bounty_id = 2u64;
    let repo_id = 111u64;
    let issue_id = 222u64;
    let long_tag = "a".repeat(51);
    let bounty_type = String::from_str(&env, &long_tag);

    client.update_metadata(&admin, &bounty_id, &repo_id, &issue_id, &bounty_type, &None);
}

#[test]
#[should_panic(expected = "bounty_type cannot be empty")]
fn test_metadata_rejects_empty_bounty_type() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    client.init(&admin, &token);

    let bounty_id = 3u64;
    let repo_id = 333u64;
    let issue_id = 444u64;
    let bounty_type = String::from_str(&env, "");

    client.update_metadata(&admin, &bounty_id, &repo_id, &issue_id, &bounty_type, &None);
}
