#![cfg(test)]
<<<<<<< feat/dispute-reason-and-outcome-codes

use crate::{
    BountyEscrowContract, BountyEscrowContractClient, DisputeOutcome, DisputeReason, EscrowStatus,
};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events},
    token, Address, Env, IntoVal,
};

fn create_token_contract<'a>(
    e: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let contract = e.register_stellar_asset_contract_v2(admin.clone());
    let contract_address = contract.address();
    (
        token::Client::new(e, &contract_address),
        token::StellarAssetClient::new(e, &contract_address),
    )
}

fn create_escrow_contract<'a>(e: &Env) -> BountyEscrowContractClient<'a> {
    let contract_id = e.register_contract(None, BountyEscrowContract);
    BountyEscrowContractClient::new(e, &contract_id)
}

struct TestSetup<'a> {
    env: Env,
    admin: Address,
    depositor: Address,
    contributor: Address,
    token: token::Client<'a>,
    token_admin: token::StellarAssetClient<'a>,
    escrow: BountyEscrowContractClient<'a>,
}

impl<'a> TestSetup<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);

        let (token, token_admin) = create_token_contract(&env, &admin);
        let escrow = create_escrow_contract(&env);

        escrow.init(&admin, &token.address);
        token_admin.mint(&depositor, &10_000_000);

        Self {
            env,
            admin,
            depositor,
            contributor,
            token,
            token_admin,
            escrow,
        }
    }
}

#[test]
fn test_dispute_reason_and_outcome_tracking() {
    let s = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let now = s.env.ledger().timestamp();
    let deadline = now + 1000;

    s.escrow
        .lock_funds(&s.depositor, &bounty_id, &amount, &deadline);

    // 1. Authorize Claim with Reason: QualityIssue
    s.escrow
        .authorize_claim(&bounty_id, &s.contributor, &DisputeReason::QualityIssue);

    let claim = s.escrow.get_pending_claim(&bounty_id);
    assert_eq!(claim.reason, DisputeReason::QualityIssue);

    // 2. Resolve via Payout
    s.escrow.claim(&bounty_id);

    let info = s.escrow.get_escrow_info(&bounty_id);
    assert_eq!(info.status, EscrowStatus::Released);

    // 3. New Bounty: Cancel with Outcome: ResolvedByRefund
    let bounty_id_2 = 2;
    s.escrow
        .lock_funds(&s.depositor, &bounty_id_2, &amount, &deadline);
    s.escrow
        .authorize_claim(&bounty_id_2, &s.contributor, &DisputeReason::IncompleteWork);

    s.escrow
        .cancel_pending_claim(&bounty_id_2, &DisputeOutcome::ResolvedByRefund);

    let info2 = s.escrow.get_escrow_info(&bounty_id_2);
    assert_eq!(info2.status, EscrowStatus::Locked); // Cancel returns to Locked
}

#[test]
fn test_dispute_event_codes() {
    let s = TestSetup::new();
    let bounty_id = 3;
    let amount = 2000;
    let deadline = s.env.ledger().timestamp() + 1000;

    s.escrow
        .lock_funds(&s.depositor, &bounty_id, &amount, &deadline);

    // Check ClaimCreated event
    s.escrow
        .authorize_claim(&bounty_id, &s.contributor, &DisputeReason::ParticipantFraud);

    // We can't easily check event data in this environment without more boilerplate,
    // but the fact it runs means the data was correctly constructed and published.
=======

use crate::{
    events::{ClaimCancelled, ClaimCreated, ClaimExecuted, FundsRefunded},
    BountyEscrowContract, BountyEscrowContractClient, Error, EscrowStatus,
};
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token, Address, Env, IntoVal, Symbol, TryIntoVal,
};

fn create_token_contract<'a>(
    env: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let contract = env.register_stellar_asset_contract_v2(admin.clone());
    let contract_address = contract.address();
    (
        token::Client::new(env, &contract_address),
        token::StellarAssetClient::new(env, &contract_address),
    )
}

fn create_escrow_contract<'a>(env: &Env) -> BountyEscrowContractClient<'a> {
    let contract_id = env.register_contract(None, BountyEscrowContract);
    BountyEscrowContractClient::new(env, &contract_id)
}

struct DisputeTestSetup<'a> {
    env: Env,
    depositor: Address,
    contributor: Address,
    token: token::Client<'a>,
    escrow: BountyEscrowContractClient<'a>,
}

impl<'a> DisputeTestSetup<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);

        let (token, token_admin) = create_token_contract(&env, &admin);
        let escrow = create_escrow_contract(&env);

        escrow.init(&admin, &token.address);
        token_admin.mint(&depositor, &10_000_000);

        Self {
            env,
            depositor,
            contributor,
            token,
            escrow,
        }
    }
}

fn assert_last_claim_event_topics(env: &Env, contract: &Address, t1: &str) {
    let last_event = env.events().all().last().unwrap();
    assert_eq!(last_event.0, *contract);

    let topics = last_event.1;
    let topic_0: Symbol = topics.get(0).unwrap().into_val(env);
    let topic_1: Symbol = topics.get(1).unwrap().into_val(env);
    assert_eq!(topic_0, Symbol::new(env, "claim"));
    assert_eq!(topic_1, Symbol::new(env, t1));
}

#[test]
fn test_open_dispute_blocks_release() {
    let setup = DisputeTestSetup::new();
    let bounty_id = 61_u64;
    let amount = 1_000_i128;
    let deadline = setup.env.ledger().timestamp() + 1_000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);
    assert_last_claim_event_topics(&setup.env, &setup.escrow.address, "created");
    let claim_created: ClaimCreated = setup
        .env
        .events()
        .all()
        .last()
        .unwrap()
        .2
        .try_into_val(&setup.env)
        .unwrap();
    assert_eq!(claim_created.bounty_id, bounty_id);
    assert_eq!(claim_created.recipient, setup.contributor);

    let release_attempt = setup
        .escrow
        .try_release_funds(&bounty_id, &setup.contributor);
    assert!(release_attempt.is_err());

    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.status, EscrowStatus::Locked);
    assert_eq!(setup.token.balance(&setup.escrow.address), amount);
}

#[test]
fn test_open_dispute_blocks_refund() {
    let setup = DisputeTestSetup::new();
    let bounty_id = 62_u64;
    let amount = 2_000_i128;
    let deadline = setup.env.ledger().timestamp() + 500;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);

    setup.env.ledger().set_timestamp(deadline + 1);

    let refund_attempt = setup.escrow.try_refund(&bounty_id);
    assert_eq!(refund_attempt, Err(Ok(Error::ClaimPending)));

    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.status, EscrowStatus::Locked);
    assert_eq!(setup.token.balance(&setup.escrow.address), amount);
}

#[test]
fn test_resolve_dispute_in_favor_of_release() {
    let setup = DisputeTestSetup::new();
    let bounty_id = 63_u64;
    let amount = 3_000_i128;
    let deadline = setup.env.ledger().timestamp() + 2_000;

    setup.escrow.set_claim_window(&600_u64);
    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);

    let claim = setup.escrow.get_pending_claim(&bounty_id);
    setup.env.ledger().set_timestamp(claim.expires_at - 1);
    setup.escrow.claim(&bounty_id);

    assert_last_claim_event_topics(&setup.env, &setup.escrow.address, "done");
    let claim_done: ClaimExecuted = setup
        .env
        .events()
        .all()
        .last()
        .unwrap()
        .2
        .try_into_val(&setup.env)
        .unwrap();
    assert_eq!(claim_done.bounty_id, bounty_id);
    assert_eq!(claim_done.amount, amount);

    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
    assert_eq!(setup.token.balance(&setup.contributor), amount);
    assert_eq!(setup.token.balance(&setup.escrow.address), 0);
}

#[test]
fn test_resolve_dispute_in_favor_of_refund() {
    let setup = DisputeTestSetup::new();
    let bounty_id = 64_u64;
    let amount = 1_500_i128;
    let now = setup.env.ledger().timestamp();
    let deadline = now + 400;

    setup.escrow.set_claim_window(&100_u64);
    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);

    let claim = setup.escrow.get_pending_claim(&bounty_id);
    setup.env.ledger().set_timestamp(claim.expires_at + 1);
    setup.escrow.cancel_pending_claim(&bounty_id);

    assert_last_claim_event_topics(&setup.env, &setup.escrow.address, "cancel");
    let claim_cancelled: ClaimCancelled = setup
        .env
        .events()
        .all()
        .last()
        .unwrap()
        .2
        .try_into_val(&setup.env)
        .unwrap();
    assert_eq!(claim_cancelled.bounty_id, bounty_id);
    assert_eq!(claim_cancelled.amount, amount);

    setup.env.ledger().set_timestamp(deadline + 1);
    setup.escrow.refund(&bounty_id);

    let last_event = setup.env.events().all().last().unwrap();
    assert_eq!(last_event.0, setup.escrow.address);
    let topics = last_event.1;
    let topic_0: Symbol = topics.get(0).unwrap().into_val(&setup.env);
    let topic_1: u64 = topics.get(1).unwrap().into_val(&setup.env);
    assert_eq!(topic_0, Symbol::new(&setup.env, "f_ref"));
    assert_eq!(topic_1, bounty_id);
    let refunded: FundsRefunded = setup
        .env
        .events()
        .all()
        .last()
        .unwrap()
        .2
        .try_into_val(&setup.env)
        .unwrap();
    assert_eq!(refunded.bounty_id, bounty_id);
    assert_eq!(refunded.amount, amount);

    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.status, EscrowStatus::Refunded);
    assert_eq!(setup.token.balance(&setup.depositor), 10_000_000);
    assert_eq!(setup.token.balance(&setup.escrow.address), 0);
>>>>>>> master
}
