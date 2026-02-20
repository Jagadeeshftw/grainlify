#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::{Address as _, Ledger}, Address, Env, vec};
use soroban_sdk::token::Client as TokenClient;
use soroban_sdk::token::StellarAssetClient as TokenAdminClient;

fn create_token_contract<'a>(e: &Env, admin: &Address) -> (TokenClient<'a>, TokenAdminClient<'a>) {
    let contract_address = e.register_stellar_asset_contract(admin.clone());
    (
        TokenClient::new(e, &contract_address),
        TokenAdminClient::new(e, &contract_address),
    )
}

fn create_escrow_contract<'a>(e: &Env) -> ProgramEscrowContractClient<'a> {
    let contract_id = e.register_contract(None, ProgramEscrowContract);
    ProgramEscrowContractClient::new(e, &contract_id)
}

#[test]
fn test_program_granular_pause_lock() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let manager = Address::generate(&env); // authorized payout key
    let token_admin = Address::generate(&env);
    
    let (token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    let escrow_client = create_escrow_contract(&env);
    
    // Initialize contract admin
    escrow_client.initialize_contract(&admin);
    
    // Check default flags
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.lock_paused, false);
    
    // Initialize program
    let program_id = String::from_str(&env, "prog1");
    escrow_client.initialize_program(&program_id, &manager, &token_client.address);
    
    // Setup funds
    let escrow_address = escrow_client.address;
    token_admin_client.mint(&escrow_address, &1000); // simulate transfer
    
    // Verify lock works unpaused
    escrow_client.lock_program_funds(&program_id, &100);
    
    // Pause lock
    escrow_client.set_paused(
        &Some(true),
        &None,
        &None
    );
    
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.lock_paused, true);
    
    // Try to lock (should fail)
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        escrow_client.lock_program_funds(&program_id, &100);
    }));
    assert!(res.is_err()); // Should be panic "Funds Paused"
    
    // Unpause
    escrow_client.set_paused(
        &Some(false),
        &None,
        &None
    );
    
    // Lock works again
    escrow_client.lock_program_funds(&program_id, &100);
}

#[test]
fn test_program_granular_pause_payout() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let manager = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token_admin = Address::generate(&env);
    
    let (token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    let escrow_client = create_escrow_contract(&env);
    
    escrow_client.initialize_contract(&admin);
    
    let program_id = String::from_str(&env, "prog1");
    escrow_client.initialize_program(&program_id, &manager, &token_client.address);
    
    let escrow_address = escrow_client.address;
    token_admin_client.mint(&escrow_address, &1000);
    escrow_client.lock_program_funds(&program_id, &1000);
    
    // Pause release
    escrow_client.set_paused(
        &None,
        &Some(true),
        &None
    );
    
    // Single payout should fail
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        escrow_client.single_payout(&program_id, &recipient, &100);
    }));
    assert!(res.is_err());
    
    // Batch payout should fail
    let recipients = vec![&env, recipient.clone()];
    let amounts = vec![&env, 100];
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        escrow_client.batch_payout(&program_id, &recipients, &amounts);
    }));
    assert!(res.is_err());
    
    // Unpause
    escrow_client.set_paused(
        &None,
        &Some(false),
        &None
    );
    
    // Single payout works
    escrow_client.single_payout(&program_id, &recipient, &100);
}

#[test]
#[should_panic(expected = "Already initialized")]
fn test_double_initialize_contract() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let escrow_client = create_escrow_contract(&env);
    
    escrow_client.initialize_contract(&admin);
    escrow_client.initialize_contract(&admin);
}

#[test]
#[should_panic(expected = "Not initialized")]
fn test_set_paused_uninitialized() {
    let env = Env::default();
    let escrow_client = create_escrow_contract(&env);
    
    escrow_client.set_paused(&Some(true), &None, &None);
}
