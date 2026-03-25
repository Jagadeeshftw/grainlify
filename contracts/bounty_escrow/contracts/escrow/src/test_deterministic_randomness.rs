#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, token, Address, BytesN, Env, Vec as SdkVec};

struct Setup<'a> {
    env: Env,
    client: BountyEscrowContractClient<'a>,
    admin: Address,
    depositor: Address,
    token_id: Address,
}

impl<'a> Setup<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token_id = env
            .register_stellar_asset_contract_v2(token_admin.clone())
            .address();
        client.init(&admin, &token_id);

        Self {
            env,
            client,
            admin,
            depositor,
            token_id,
        }
    }
}

fn get_claim_ticket(env: &Env, contract: &Address, ticket_id: u64) -> ClaimTicket {
    env.as_contract(contract, || {
        env.storage()
            .persistent()
            .get::<DataKey, ClaimTicket>(&DataKey::ClaimTicket(ticket_id))
            .expect("claim ticket should exist")
    })
}

#[test]
fn test_deterministic_winner_is_stable_for_same_inputs() {
    let s = Setup::new();
    let _ = &s.admin;
    let mut candidates = SdkVec::new(&s.env);
    candidates.push_back(Address::generate(&s.env));
    candidates.push_back(Address::generate(&s.env));
    candidates.push_back(Address::generate(&s.env));
    let seed = BytesN::from_array(&s.env, &[7u8; 32]);
    let expires_at = s.env.ledger().timestamp() + 500;

    let w1 = s
        .client
        .derive_claim_ticket_winner(&42, &candidates, &1000, &expires_at, &seed);
    let w2 = s
        .client
        .derive_claim_ticket_winner(&42, &candidates, &1000, &expires_at, &seed);

    assert_eq!(w1, w2);
}

#[test]
fn test_deterministic_winner_is_order_independent() {
    let s = Setup::new();
    let a = Address::generate(&s.env);
    let b = Address::generate(&s.env);
    let c = Address::generate(&s.env);
    let seed = BytesN::from_array(&s.env, &[9u8; 32]);
    let expires_at = s.env.ledger().timestamp() + 600;

    let mut candidates_1 = SdkVec::new(&s.env);
    candidates_1.push_back(a.clone());
    candidates_1.push_back(b.clone());
    candidates_1.push_back(c.clone());
    let mut candidates_2 = SdkVec::new(&s.env);
    candidates_2.push_back(c);
    candidates_2.push_back(a);
    candidates_2.push_back(b);

    let w1 = s
        .client
        .derive_claim_ticket_winner(&77, &candidates_1, &2500, &expires_at, &seed);
    let w2 = s
        .client
        .derive_claim_ticket_winner(&77, &candidates_2, &2500, &expires_at, &seed);

    assert_eq!(w1, w2);
}

#[test]
fn test_issue_claim_ticket_deterministic_issues_for_derived_winner() {
    let s = Setup::new();
    let token_admin_client = token::StellarAssetClient::new(&s.env, &s.token_id);

    let bounty_id = 1u64;
    let lock_amount = 50_000i128;
    let deadline = s.env.ledger().timestamp() + 1_000;
    token_admin_client.mint(&s.depositor, &lock_amount);
    s.client
        .lock_funds(&s.depositor, &bounty_id, &lock_amount, &deadline);

    let mut candidates = SdkVec::new(&s.env);
    candidates.push_back(Address::generate(&s.env));
    candidates.push_back(Address::generate(&s.env));
    candidates.push_back(Address::generate(&s.env));
    let seed = BytesN::from_array(&s.env, &[3u8; 32]);
    let expires_at = s.env.ledger().timestamp() + 500;
    let claim_amount = 10_000i128;

    let derived_winner = s.client.derive_claim_ticket_winner(
        &bounty_id,
        &candidates,
        &claim_amount,
        &expires_at,
        &seed,
    );

    let ticket_id = s.client.issue_claim_ticket_deterministic(
        &bounty_id,
        &candidates,
        &claim_amount,
        &expires_at,
        &seed,
    );
    let ticket = get_claim_ticket(&s.env, &s.client.address, ticket_id);

    assert_eq!(ticket.beneficiary, derived_winner);
    assert_eq!(ticket.amount, claim_amount);
    assert_eq!(ticket.bounty_id, bounty_id);
    // The contract doesn't expose get_claim_ticket, so we just verify the issue call succeeds
    // and returns a ticket ID. The winner logic is already verified in other tests.
    assert!(ticket_id > 0);
}
