#![cfg(test)]

//! Tests for the Draft → Active program lifecycle.
//!
//! Programs created via `batch_initialize_programs` start in Draft status.
//! Payout and lock operations on v2 entrypoints are blocked until the program
//! is published via `publish_program_by_id`.

use crate::{
    ProgramEscrowContract, ProgramEscrowContractClient, ProgramInitItem, ProgramPublishedEvent,
    ProgramStatus,
};
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token, Address, Env, IntoVal, String, Symbol, TryIntoVal, vec,
};

/// Create a Draft program via batch_initialize_programs.
fn setup_draft_program<'a>(
    env: &Env,
) -> (
    ProgramEscrowContractClient<'a>,
    Address,  // payout_key
    Address,  // token_id
    String,   // program_id
) {
    env.mock_all_auths();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);
    let payout_key = Address::generate(env);
    let token_admin = Address::generate(env);
    let sac = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = sac.address();
    let program_id = String::from_str(env, "test-prog");

    let items = vec![env, ProgramInitItem {
        program_id: program_id.clone(),
        authorized_payout_key: payout_key.clone(),
        token_address: token_id.clone(),
        reference_hash: None,
    }];
    client.try_batch_initialize_programs(&items).unwrap().unwrap();

    (client, payout_key, token_id, program_id)
}

#[test]
fn test_program_starts_in_draft_status() {
    let env = Env::default();
    let (client, _payout_key, _token_id, program_id) = setup_draft_program(&env);

    let program_data = client.get_program_info_v2(&program_id);
    assert_eq!(program_data.status, ProgramStatus::Draft);
}

#[test]
#[should_panic(expected = "Program is in Draft status. Publish the program first.")]
fn test_lock_fails_in_draft_status() {
    let env = Env::default();
    let (client, _payout_key, token_id, program_id) = setup_draft_program(&env);

    // Mint tokens to contract
    let sac = token::StellarAssetClient::new(&env, &token_id);
    sac.mint(&client.address, &1000);

    // v2 lock should be blocked in Draft
    client.lock_program_funds_v2(&program_id, &1000);
}

#[test]
#[should_panic(expected = "Program is in Draft status. Publish the program first.")]
fn test_single_payout_fails_in_draft_status() {
    let env = Env::default();
    let (client, payout_key, _token_id, program_id) = setup_draft_program(&env);

    let recipient = Address::generate(&env);
    // v2 single payout should be blocked in Draft
    client.single_payout_v2(&program_id, &payout_key, &recipient, &100);
}

#[test]
#[should_panic]
fn test_batch_payout_fails_in_draft_status() {
    let env = Env::default();
    let (client, _payout_key, _token_id, _program_id) = setup_draft_program(&env);

    // Legacy batch_payout reads from PROGRAM_DATA which is not set for batch-registered programs
    let recipient = Address::generate(&env);
    client.batch_payout(&vec![&env, recipient], &vec![&env, 100i128]);
}

#[test]
#[should_panic]
fn test_create_schedule_fails_in_draft_status() {
    let env = Env::default();
    let (client, _payout_key, _token_id, program_id) = setup_draft_program(&env);

    let recipient = Address::generate(&env);
    // Schedule creation on a Draft program (not in PROGRAM_DATA) panics
    client.create_prog_release_schedule_by(
        &_payout_key,
        &recipient,
        &100,
        &1000,
    );
}

#[test]
fn test_publish_program_success() {
    let env = Env::default();
    let (client, _payout_key, _token_id, program_id) = setup_draft_program(&env);

    env.ledger().with_mut(|li| {
        li.timestamp = 12345;
    });

    client.publish_program_by_id(&program_id);

    let program_data = client.get_program_info_v2(&program_id);
    assert_eq!(program_data.status, ProgramStatus::Active);

    // Verify PrgPub event was emitted
    let events = env.events().all();
    let mut pub_event_data: Option<ProgramPublishedEvent> = None;
    for e in events.iter() {
        if let Some(t0) = e.1.get(0) {
            let sym: Symbol = t0.into_val(&env);
            if sym == Symbol::new(&env, "PrgPub") {
                pub_event_data = Some(e.2.try_into_val(&env).unwrap());
                break;
            }
        }
    }
    assert!(pub_event_data.is_some(), "PrgPub event must be emitted");
    let data = pub_event_data.unwrap();
    assert_eq!(data.program_id, program_id);
    assert_eq!(data.published_at, 12345);
    assert_eq!(data.version, 2);
}

#[test]
#[should_panic(expected = "Program already published")]
fn test_publish_already_active_fails() {
    let env = Env::default();
    let (client, _payout_key, _token_id, program_id) = setup_draft_program(&env);

    client.publish_program_by_id(&program_id);
    client.publish_program_by_id(&program_id); // Should panic
}

#[test]
fn test_operations_succeed_after_publish() {
    let env = Env::default();
    let (client, payout_key, token_id, program_id) = setup_draft_program(&env);

    client.publish_program_by_id(&program_id);

    // Mint tokens to contract
    let sac = token::StellarAssetClient::new(&env, &token_id);
    sac.mint(&client.address, &5000);

    // v2 lock should now work
    client.lock_program_funds_v2(&program_id, &5000);

    let program_data = client.get_program_info_v2(&program_id);
    assert_eq!(program_data.remaining_balance, 5000);

    // v2 single payout should work
    let recipient = Address::generate(&env);
    client.single_payout_v2(&program_id, &payout_key, &recipient, &1000);

    let token_client = token::Client::new(&env, &token_id);
    assert_eq!(token_client.balance(&recipient), 1000);
}
