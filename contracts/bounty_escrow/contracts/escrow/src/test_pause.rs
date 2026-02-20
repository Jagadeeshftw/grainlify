#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::{Address as _, Ledger}, Address, Env};
use soroban_sdk::token::Client as TokenClient;
use soroban_sdk::token::StellarAssetClient as TokenAdminClient;

fn create_token_contract<'a>(e: &Env, admin: &Address) -> (TokenClient<'a>, TokenAdminClient<'a>) {
    let contract_address = e.register_stellar_asset_contract(admin.clone());
    (
        TokenClient::new(e, &contract_address),
        TokenAdminClient::new(e, &contract_address),
    )
}

fn create_escrow_contract<'a>(e: &Env) -> EscrowClient<'a> {
    let contract_id = e.register_contract(None, EscrowContract);
    EscrowClient::new(e, &contract_id)
}

#[test]
fn test_granular_pause_lock() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    
    let (token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    let escrow_client = create_escrow_contract(&env);
    
    // Initialize
    escrow_client.initialize(&admin, &token_client.address);
    
    // Check default state (unpaused)
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.lock_paused, false);
    assert_eq!(flags.release_paused, false);
    assert_eq!(flags.refund_paused, false);
    
    // Setup funds
    token_admin_client.mint(&depositor, &1000);
    
    // Verify lock works unpaused
    let bounty_id_1 = 1;
    let deadline = env.ledger().timestamp() + 1000;
    
    escrow_client.lock_funds(
        &depositor,
        &bounty_id_1,
        &100,
        &deadline
    );
    
    // Pause lock
    escrow_client.set_paused(
        &Some(true),
        &None,
        &None
    );
    
    // Verify lock paused
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.lock_paused, true);
    
    // Try to lock (should fail)
    let bounty_id_2 = 2;
    let res = escrow_client.try_lock_funds(
        &depositor,
        &bounty_id_2,
        &100,
        &deadline
    );
    assert_eq!(res, Err(Ok(Error::FundsPaused)));
    
    // Verify other operations still work (if capable)
    // Release should fail because funds not locked, but not because paused logic
    // We can test release pause separately.
    
    // Unpause lock
    escrow_client.set_paused(
        &Some(false),
        &None,
        &None
    );
    
    // Verify lock works again
    escrow_client.lock_funds(
        &depositor,
        &bounty_id_2,
        &100,
        &deadline
    );
}

#[test]
fn test_granular_pause_release() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    
    let (token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    let escrow_client = create_escrow_contract(&env);
    
    escrow_client.initialize(&admin, &token_client.address);
    token_admin_client.mint(&depositor, &1000);
    
    let bounty_id = 1;
    let deadline = env.ledger().timestamp() + 1000;
    
    escrow_client.lock_funds(
        &depositor,
        &bounty_id,
        &100,
        &deadline
    );
    
    // Pause release
    escrow_client.set_paused(
        &None,
        &Some(true),
        &None
    );
    
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.release_paused, true);
    
    // Try to release
    let res = escrow_client.try_release_funds(&bounty_id, &contributor);
    assert_eq!(res, Err(Ok(Error::FundsPaused)));
    
    // Unpause
    escrow_client.set_paused(
        &None,
        &Some(false),
        &None
    );
    
    // Release should work
    escrow_client.release_funds(&bounty_id, &contributor);
}

#[test]
fn test_granular_pause_refund() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    
    let (token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    let escrow_client = create_escrow_contract(&env);
    
    escrow_client.initialize(&admin, &token_client.address);
    token_admin_client.mint(&depositor, &1000);
    
    let bounty_id = 1;
    // Set deadline in past so we can refund immediately
    let deadline = env.ledger().timestamp(); // now
    
    escrow_client.lock_funds(
        &depositor,
        &bounty_id,
        &100,
        &deadline
    );
    
    // Advance time to pass deadline
    env.ledger().set_timestamp(deadline + 1);
    
    // Pause refund
    escrow_client.set_paused(
        &None,
        &None,
        &Some(true)
    );
    
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.refund_paused, true);
    
    // Try to refund
    let res = escrow_client.try_refund(
        &bounty_id,
        &None,
        &None,
        &RefundMode::Full
    );
    assert_eq!(res, Err(Ok(Error::FundsPaused)));
    
    // Unpause
    escrow_client.set_paused(
        &None,
        &None,
        &Some(false)
    );
    
    // Refund should work
    escrow_client.refund(
        &bounty_id,
        &None,
        &None,
        &RefundMode::Full
    );
}

#[test]
fn test_mixed_pause_states() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let (token_client, _) = create_token_contract(&env, &admin);
    let escrow_client = create_escrow_contract(&env);
    
    escrow_client.initialize(&admin, &token_client.address);
    
    // Pause lock and release, but not refund
    escrow_client.set_paused(
        &Some(true),
        &Some(true),
        &Some(false)
    );
    
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.lock_paused, true);
    assert_eq!(flags.release_paused, true);
    assert_eq!(flags.refund_paused, false);
    
    // Update only release to unpaused
    escrow_client.set_paused(
        &None,
        &Some(false),
        &None
    );
    
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.lock_paused, true);
    assert_eq!(flags.release_paused, false);
    assert_eq!(flags.refund_paused, false);
}
