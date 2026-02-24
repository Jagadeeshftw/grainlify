#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

fn create_token_contract<'a>(
    env: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let contract_address = env.register_stellar_asset_contract(admin.clone());
    (
        token::Client::new(env, &contract_address),
        token::StellarAssetClient::new(env, &contract_address),
    )
}

fn create_escrow_contract<'a>(env: &Env) -> BountyEscrowContractClient<'a> {
    let contract_id = env.register_contract(None, BountyEscrowContract);
    BountyEscrowContractClient::new(env, &contract_id)
}

struct TestSetup<'a> {
    env: Env,
    depositor: Address,
    token: token::Client<'a>,
    escrow: BountyEscrowContractClient<'a>,
}

impl<'a> TestSetup<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);

        let (token, token_admin) = create_token_contract(&env, &admin);
        let escrow = create_escrow_contract(&env);
        escrow.init(&admin, &token.address);
        token_admin.mint(&depositor, &1_000_000);

        Self {
            env,
            depositor,
            token,
            escrow,
        }
    }
}

#[test]
fn test_release_race_first_recipient_wins_order_ab() {
    let setup = TestSetup::new();
    let bounty_id = 9101_u64;
    let amount = 80_000_i128;
    let deadline = setup.env.ledger().timestamp() + 1_000;
    let recipient_a = Address::generate(&setup.env);
    let recipient_b = Address::generate(&setup.env);

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.escrow.release_funds(&bounty_id, &recipient_a);
    let second_release = setup.escrow.try_release_funds(&bounty_id, &recipient_b);

    assert_eq!(second_release, Err(Ok(Error::FundsNotLocked)));
    assert_eq!(setup.token.balance(&recipient_a), amount);
    assert_eq!(setup.token.balance(&recipient_b), 0);

    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
}

#[test]
fn test_release_race_first_recipient_wins_order_ba() {
    let setup = TestSetup::new();
    let bounty_id = 9102_u64;
    let amount = 80_000_i128;
    let deadline = setup.env.ledger().timestamp() + 1_000;
    let recipient_a = Address::generate(&setup.env);
    let recipient_b = Address::generate(&setup.env);

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.escrow.release_funds(&bounty_id, &recipient_b);
    let second_release = setup.escrow.try_release_funds(&bounty_id, &recipient_a);

    assert_eq!(second_release, Err(Ok(Error::FundsNotLocked)));
    assert_eq!(setup.token.balance(&recipient_b), amount);
    assert_eq!(setup.token.balance(&recipient_a), 0);

    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
}

#[test]
fn test_authorize_claim_race_last_authorization_wins() {
    let setup = TestSetup::new();
    let bounty_id = 9103_u64;
    let amount = 90_000_i128;
    let deadline = setup.env.ledger().timestamp() + 2_000;
    let claimant_a = Address::generate(&setup.env);
    let claimant_b = Address::generate(&setup.env);

    setup.escrow.set_claim_window(&500);
    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.escrow.authorize_claim(&bounty_id, &claimant_a);
    setup.escrow.authorize_claim(&bounty_id, &claimant_b);

    let pending = setup.escrow.get_pending_claim(&bounty_id);
    assert_eq!(pending.recipient, claimant_b);
    assert_eq!(pending.amount, amount);

    setup.escrow.claim(&bounty_id);

    assert_eq!(setup.token.balance(&claimant_a), 0);
    assert_eq!(setup.token.balance(&claimant_b), amount);
    assert_eq!(setup.token.balance(&setup.escrow.address), 0);

    let second_claim = setup.escrow.try_claim(&bounty_id);
    assert_eq!(second_claim, Err(Ok(Error::FundsNotLocked)));
}
