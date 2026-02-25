#![cfg(test)]

use crate::{
    BountyEscrowContract, BountyEscrowContractClient, CapabilityAction, Error, EscrowStatus,
};
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token, Address, Env, Symbol, TryFromVal,
};

struct CapabilitySetup {
    env: Env,
    client: BountyEscrowContractClient<'static>,
    token_client: token::Client<'static>,
    admin: Address,
    depositor: Address,
    contributor: Address,
    delegate: Address,
    recipient: Address,
}

impl CapabilitySetup {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);
        let delegate = Address::generate(&env);
        let recipient = Address::generate(&env);

        let token_admin_addr = Address::generate(&env);
        let token_address = env
            .register_stellar_asset_contract_v2(token_admin_addr.clone())
            .address();
        let token_client = token::Client::new(&env, &token_address);
        let token_admin = token::StellarAssetClient::new(&env, &token_address);

        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);

        client.init(&admin, &token_address);
        token_admin.mint(&depositor, &100_000);

        Self {
            env,
            client,
            token_client,
            admin,
            depositor,
            contributor,
            delegate,
            recipient,
        }
    }

    fn lock(&self, bounty_id: u64, amount: i128) {
        let deadline = self.env.ledger().timestamp() + 10_000;
        self.client
            .lock_funds(&self.depositor, &bounty_id, &amount, &deadline);
    }
}

fn has_event_topic(env: &Env, topic_name: &str) -> bool {
    let expected = Symbol::new(env, topic_name);
    let events = env.events().all();
    for (_contract, topics, _data) in events.iter() {
        if topics.len() == 0 {
            continue;
        }
        let first = topics.get(0).unwrap();
        if let Ok(sym) = Symbol::try_from_val(env, &first) {
            if sym == expected {
                return true;
            }
        }
    }
    false
}

#[test]
fn test_issue_and_use_release_capability() {
    let setup = CapabilitySetup::new();
    setup.lock(1, 1_000);

    let expiry = setup.env.ledger().timestamp() + 300;
    let capability_id = setup.client.issue_capability(
        &setup.admin,
        &setup.delegate,
        &CapabilityAction::Release,
        &1,
        &600,
        &expiry,
        &2,
    );

    let issued = setup.client.get_capability(&capability_id);
    assert_eq!(issued.holder, setup.delegate);
    assert_eq!(issued.amount_limit, 600);
    assert_eq!(issued.remaining_amount, 600);
    assert_eq!(issued.remaining_uses, 2);
    assert!(has_event_topic(&setup.env, "cap_new"));

    setup.client.release_with_capability(
        &1,
        &setup.contributor,
        &400,
        &setup.delegate,
        &capability_id,
    );
    assert!(has_event_topic(&setup.env, "cap_use"));

    let escrow = setup.client.get_escrow_info(&1);
    assert_eq!(escrow.status, EscrowStatus::Locked);
    assert_eq!(escrow.remaining_amount, 600);
    assert_eq!(setup.token_client.balance(&setup.contributor), 400);

    let after_use = setup.client.get_capability(&capability_id);
    assert_eq!(after_use.remaining_amount, 200);
    assert_eq!(after_use.remaining_uses, 1);

    let too_large = setup.client.try_release_with_capability(
        &1,
        &setup.contributor,
        &300,
        &setup.delegate,
        &capability_id,
    );
    assert_eq!(
        too_large.unwrap_err().unwrap(),
        Error::CapabilityAmountExceeded
    );
}

#[test]
fn test_claim_with_capability() {
    let setup = CapabilitySetup::new();
    setup.lock(2, 2_000);

    setup.client.set_claim_window(&600);
    setup.client.authorize_claim(&2, &setup.recipient);

    let expiry = setup.env.ledger().timestamp() + 120;
    let capability_id = setup.client.issue_capability(
        &setup.recipient,
        &setup.delegate,
        &CapabilityAction::Claim,
        &2,
        &2_000,
        &expiry,
        &1,
    );

    setup
        .client
        .claim_with_capability(&2, &setup.delegate, &capability_id);

    let escrow = setup.client.get_escrow_info(&2);
    assert_eq!(escrow.status, EscrowStatus::Released);
    assert_eq!(setup.token_client.balance(&setup.recipient), 2_000);

    let used = setup.client.get_capability(&capability_id);
    assert_eq!(used.remaining_uses, 0);
    assert_eq!(used.remaining_amount, 0);
}

#[test]
fn test_capability_expiry_and_revocation() {
    let setup = CapabilitySetup::new();
    setup.lock(3, 1_000);

    let expiry = setup.env.ledger().timestamp() + 5;
    let expired_id = setup.client.issue_capability(
        &setup.admin,
        &setup.delegate,
        &CapabilityAction::Refund,
        &3,
        &500,
        &expiry,
        &1,
    );

    setup.env.ledger().set_timestamp(expiry + 1);
    let expired = setup
        .client
        .try_refund_with_capability(&3, &100, &setup.delegate, &expired_id);
    assert_eq!(expired.unwrap_err().unwrap(), Error::CapabilityExpired);

    let active_expiry = setup.env.ledger().timestamp() + 200;
    let active_id = setup.client.issue_capability(
        &setup.admin,
        &setup.delegate,
        &CapabilityAction::Refund,
        &3,
        &500,
        &active_expiry,
        &1,
    );
    setup.client.revoke_capability(&setup.admin, &active_id);
    assert!(has_event_topic(&setup.env, "cap_rev"));

    let revoked = setup
        .client
        .try_refund_with_capability(&3, &100, &setup.delegate, &active_id);
    assert_eq!(revoked.unwrap_err().unwrap(), Error::CapabilityRevoked);
}

#[test]
fn test_capability_cannot_exceed_owner_authority() {
    let setup = CapabilitySetup::new();
    setup.lock(4, 700);

    let expiry = setup.env.ledger().timestamp() + 100;
    let non_admin_issue = setup.client.try_issue_capability(
        &setup.depositor,
        &setup.delegate,
        &CapabilityAction::Release,
        &4,
        &300,
        &expiry,
        &1,
    );
    assert_eq!(non_admin_issue.unwrap_err().unwrap(), Error::Unauthorized);

    let over_limit_issue = setup.client.try_issue_capability(
        &setup.admin,
        &setup.delegate,
        &CapabilityAction::Release,
        &4,
        &701,
        &expiry,
        &1,
    );
    assert_eq!(
        over_limit_issue.unwrap_err().unwrap(),
        Error::CapabilityExceedsAuthority
    );

    setup.client.set_claim_window(&300);
    setup.client.authorize_claim(&4, &setup.recipient);
    let wrong_claim_owner = setup.client.try_issue_capability(
        &setup.admin,
        &setup.delegate,
        &CapabilityAction::Claim,
        &4,
        &200,
        &expiry,
        &1,
    );
    assert_eq!(wrong_claim_owner.unwrap_err().unwrap(), Error::Unauthorized);
}
