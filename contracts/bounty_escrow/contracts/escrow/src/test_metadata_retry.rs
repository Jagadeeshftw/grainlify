use crate::{
    BountyEscrowContract, BountyEscrowContractClient, EscrowMetadata, NOTIFY_ON_RELEASE,
    RISK_FLAG_UNDER_REVIEW,
};
use soroban_sdk::{
    testutils::{Address as _, Events as _},
    token, Address, Bytes, Env, String,
};

fn assert_metadata_retry(with_reference_hash: bool) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let token = token::Client::new(&env, &token_address);
    token::StellarAssetClient::new(&env, &token_address).mint(&depositor, &2_000);

    let contract = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract);
    client.init(&admin, &token_address);
    let bounty_id = 1994;
    client.lock_funds(&depositor, &bounty_id, &1_000, &10_000);
    client.set_escrow_risk_flags(&bounty_id, &RISK_FLAG_UNDER_REVIEW);
    client.set_notification_preferences(&bounty_id, &NOTIFY_ON_RELEASE);

    let expected = EscrowMetadata {
        repo_id: 123,
        issue_id: 456,
        bounty_type: String::from_str(&env, "bug-fix"),
        risk_flags: RISK_FLAG_UNDER_REVIEW,
        notification_prefs: NOTIFY_ON_RELEASE,
        reference_hash: with_reference_hash.then(|| Bytes::from_slice(&env, &[7; 32])),
    };
    let escrow_before = client.get_escrow_info(&bounty_id);
    assert_ne!(client.get_metadata(&bounty_id), expected);
    assert_eq!(token.balance(&depositor), 1_000);
    assert_eq!(token.balance(&contract), 1_000);
    assert_eq!(token.balance(&admin), 0);

    for submission in 1..=2 {
        let events_before = env.events().all();
        let result = client.update_metadata(
            &admin,
            &bounty_id,
            &expected.repo_id,
            &expected.issue_id,
            &expected.bounty_type,
            &expected.reference_hash,
        );

        // The current entrypoint emits no events, including on a retry.
        assert_eq!(env.events().all(), events_before, "submission {submission}");
        assert_eq!(result, expected, "submission {submission}");
        assert_eq!(client.get_metadata(&bounty_id), expected);
        assert_eq!(client.get_escrow_info(&bounty_id), escrow_before);
        assert_eq!(token.balance(&depositor), 1_000, "submission {submission}");
        assert_eq!(token.balance(&contract), 1_000, "submission {submission}");
        assert_eq!(token.balance(&admin), 0, "submission {submission}");
    }
}

#[test]
fn update_metadata_twice_without_reference_hash_is_idempotent() {
    assert_metadata_retry(false);
}

#[test]
fn update_metadata_twice_with_reference_hash_is_idempotent() {
    assert_metadata_retry(true);
}
