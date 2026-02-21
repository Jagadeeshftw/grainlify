#![cfg(test)]

/// # Program Status & Lifecycle Transition Tests
///
/// This module tests the implicit lifecycle of the Program Escrow contract,
/// covering all state transitions and asserting which operations are allowed
/// or forbidden in each state.
///
/// ## Lifecycle States
///
/// ```text
/// Uninitialized  ──init_program()──►  Initialized
///                                         │
///                                   lock_program_funds()
///                                         │
///                                         ▼
///                                       Active  ◄──── lock_program_funds() (top-up)
///                                         │
///                              ┌──────────┼──────────┐
///                        set_paused()  payouts()  set_paused()
///                              │                      │
///                              ▼                      │
///                            Paused ──set_paused()──► Active (resume)
///                              │
///                         (forbidden ops)
///                                         │
///                              all funds paid out
///                                         │
///                                         ▼
///                                       Drained  (remaining_balance == 0)
///                                         │
///                              lock_program_funds()  (re-activate)
///                                         │
///                                         ▼
///                                       Active
/// ```

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, vec, Address, Env, String,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Register the contract and return a client plus the contract address.
fn make_client(env: &Env) -> (ProgramEscrowContractClient<'static>, Address) {
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);
    (client, contract_id)
}

/// Create a real SAC token, mint `amount` to the contract address, and return
/// the token client and token contract id.
fn fund_contract(
    env: &Env,
    contract_id: &Address,
    amount: i128,
) -> (token::Client<'static>, Address) {
    let token_admin = Address::generate(env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = token_contract.address();
    let token_client = token::Client::new(env, &token_id);
    let token_sac = token::StellarAssetClient::new(env, &token_id);
    if amount > 0 {
        token_sac.mint(contract_id, &amount);
    }
    (token_client, token_id)
}

/// Full setup: contract, admin (authorized payout key), token, program
/// initialized and funded.
fn setup_active_program(
    env: &Env,
    amount: i128,
) -> (
    ProgramEscrowContractClient<'static>,
    Address,
    Address,
    token::Client<'static>,
) {
    env.mock_all_auths();
    let (client, contract_id) = make_client(env);
    let (token_client, token_id) = fund_contract(env, &contract_id, amount);
    let admin = Address::generate(env);
    let program_id = String::from_str(env, "hack-2026");
    client.init_program(&program_id, &admin, &token_id);
    if amount > 0 {
        client.lock_program_funds(&amount);
    }
    (client, admin, contract_id, token_client)
}

// ---------------------------------------------------------------------------
// STATE: Uninitialized
// Any operation before init_program must be rejected.
// ---------------------------------------------------------------------------

#[test]
#[should_panic(expected = "Program not initialized")]
fn test_uninitialized_lock_funds_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _cid) = make_client(&env);
    client.lock_program_funds(&1_000);
}

#[test]
#[should_panic(expected = "Program not initialized")]
fn test_uninitialized_single_payout_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _cid) = make_client(&env);
    let recipient = Address::generate(&env);
    client.single_payout(&recipient, &100);
}

#[test]
#[should_panic(expected = "Program not initialized")]
fn test_uninitialized_batch_payout_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _cid) = make_client(&env);
    let r = Address::generate(&env);
    client.batch_payout(&vec![&env, r], &vec![&env, 100i128]);
}

#[test]
#[should_panic(expected = "Program not initialized")]
fn test_uninitialized_get_info_rejected() {
    let env = Env::default();
    let (client, _cid) = make_client(&env);
    client.get_program_info();
}

#[test]
#[should_panic(expected = "Program not initialized")]
fn test_uninitialized_get_balance_rejected() {
    let env = Env::default();
    let (client, _cid) = make_client(&env);
    client.get_remaining_balance();
}

#[test]
#[should_panic(expected = "Program not initialized")]
fn test_uninitialized_create_schedule_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _cid) = make_client(&env);
    let r = Address::generate(&env);
    client.create_program_release_schedule(&r, &100, &1000);
}

#[test]
#[should_panic(expected = "Program not initialized")]
fn test_uninitialized_trigger_releases_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _cid) = make_client(&env);
    client.trigger_program_releases();
}

// ---------------------------------------------------------------------------
// STATE: Initialized (program exists, no funds locked yet)
// ---------------------------------------------------------------------------

/// After init_program the program is queryable and balance is 0.
#[test]
fn test_initialized_state_balance_is_zero() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _cid) = make_client(&env);
    let token_id = Address::generate(&env);
    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "hack-2026");
    client.init_program(&program_id, &admin, &token_id);

    let info = client.get_program_info();
    assert_eq!(info.total_funds, 0);
    assert_eq!(info.remaining_balance, 0);
    assert_eq!(info.payout_history.len(), 0);
    assert_eq!(client.get_remaining_balance(), 0);
}

/// Re-initializing the same program must be rejected (single-init guard).
#[test]
#[should_panic(expected = "Program already initialized")]
fn test_initialized_double_init_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _cid) = make_client(&env);
    let token_id = Address::generate(&env);
    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "hack-2026");
    client.init_program(&program_id, &admin, &token_id);
    // Second call must panic
    client.init_program(&program_id, &admin, &token_id);
}

/// Payout from a zero-balance (Initialized) program must be rejected.
#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_initialized_single_payout_zero_balance_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _cid) = make_client(&env);
    let token_id = Address::generate(&env);
    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "hack-2026");
    client.init_program(&program_id, &admin, &token_id);
    let r = Address::generate(&env);
    client.single_payout(&r, &100);
}

/// Batch payout from a zero-balance (Initialized) program must be rejected.
#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_initialized_batch_payout_zero_balance_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _cid) = make_client(&env);
    let token_id = Address::generate(&env);
    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "hack-2026");
    client.init_program(&program_id, &admin, &token_id);
    let r = Address::generate(&env);
    client.batch_payout(&vec![&env, r], &vec![&env, 100i128]);
}

/// Locking funds transitions the contract from Initialized to Active.
#[test]
fn test_initialized_to_active_via_lock_funds() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, contract_id) = make_client(&env);
    let (_, token_id) = fund_contract(&env, &contract_id, 50_000);
    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "hack-2026");
    client.init_program(&program_id, &admin, &token_id);

    // Before lock: Initialized — balance is 0
    assert_eq!(client.get_remaining_balance(), 0);

    // Transition: Initialized → Active
    let data = client.lock_program_funds(&50_000);
    assert_eq!(data.total_funds, 50_000);
    assert_eq!(data.remaining_balance, 50_000);

    // After lock: Active — balance reflects locked amount
    assert_eq!(client.get_remaining_balance(), 50_000);
}

// ---------------------------------------------------------------------------
// STATE: Active (funds locked, payouts can happen)
// ---------------------------------------------------------------------------

/// In Active state, single_payout succeeds and reduces remaining balance.
#[test]
fn test_active_single_payout_allowed() {
    let env = Env::default();
    let (client, _admin, _cid, token_client) = setup_active_program(&env, 100_000);
    let recipient = Address::generate(&env);

    let data = client.single_payout(&recipient, &40_000);
    assert_eq!(data.remaining_balance, 60_000);
    assert_eq!(token_client.balance(&recipient), 40_000);
}

/// In Active state, batch_payout succeeds and reduces remaining balance.
#[test]
fn test_active_batch_payout_allowed() {
    let env = Env::default();
    let (client, _admin, _cid, token_client) = setup_active_program(&env, 100_000);
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    let data = client.batch_payout(
        &vec![&env, r1.clone(), r2.clone()],
        &vec![&env, 30_000i128, 20_000i128],
    );
    assert_eq!(data.remaining_balance, 50_000);
    assert_eq!(token_client.balance(&r1), 30_000);
    assert_eq!(token_client.balance(&r2), 20_000);
}

/// Multiple lock calls accumulate funds (top-up stays in Active state).
#[test]
fn test_active_top_up_lock_increases_balance() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, contract_id) = make_client(&env);
    let (_, token_id) = fund_contract(&env, &contract_id, 200_000);
    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "hack-2026");
    client.init_program(&program_id, &admin, &token_id);

    client.lock_program_funds(&80_000);
    assert_eq!(client.get_remaining_balance(), 80_000);

    client.lock_program_funds(&70_000);
    assert_eq!(client.get_remaining_balance(), 150_000);

    let info = client.get_program_info();
    assert_eq!(info.total_funds, 150_000);
}

/// In Active state, negative lock amounts are rejected.
#[test]
#[should_panic(expected = "Amount must be greater than zero")]
fn test_active_negative_lock_amount_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _cid) = make_client(&env);
    let token_id = Address::generate(&env);
    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "hack-2026");
    client.init_program(&program_id, &admin, &token_id);
    client.lock_program_funds(&-1);
}

/// Payout exceeding balance must be rejected (Active state guard).
#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_active_payout_exceeds_balance_rejected() {
    let env = Env::default();
    let (client, _admin, _cid, _token) = setup_active_program(&env, 50_000);
    let r = Address::generate(&env);
    client.single_payout(&r, &50_001); // 1 unit over balance
}

/// Batch payout total exceeding balance must be rejected.
#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_active_batch_exceeds_balance_rejected() {
    let env = Env::default();
    let (client, _admin, _cid, _token) = setup_active_program(&env, 50_000);
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    // 30_000 + 30_000 = 60_000 > 50_000
    client.batch_payout(
        &vec![&env, r1, r2],
        &vec![&env, 30_000i128, 30_000i128],
    );
}

/// Zero-amount single payout must be rejected.
#[test]
#[should_panic(expected = "Amount must be greater than zero")]
fn test_active_zero_single_payout_rejected() {
    let env = Env::default();
    let (client, _admin, _cid, _token) = setup_active_program(&env, 50_000);
    let r = Address::generate(&env);
    client.single_payout(&r, &0);
}

/// Zero-amount entry in a batch must be rejected.
#[test]
#[should_panic(expected = "All amounts must be greater than zero")]
fn test_active_zero_amount_in_batch_rejected() {
    let env = Env::default();
    let (client, _admin, _cid, _token) = setup_active_program(&env, 50_000);
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    client.batch_payout(
        &vec![&env, r1, r2],
        &vec![&env, 100i128, 0i128],
    );
}

/// Mismatched recipients/amounts vectors must be rejected.
#[test]
#[should_panic(expected = "Recipients and amounts vectors must have the same length")]
fn test_active_batch_mismatched_lengths_rejected() {
    let env = Env::default();
    let (client, _admin, _cid, _token) = setup_active_program(&env, 50_000);
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    client.batch_payout(&vec![&env, r1, r2], &vec![&env, 100i128]);
}

/// Empty batch must be rejected.
#[test]
#[should_panic(expected = "Cannot process empty batch")]
fn test_active_empty_batch_rejected() {
    let env = Env::default();
    let (client, _admin, _cid, _token) = setup_active_program(&env, 50_000);
    client.batch_payout(&vec![&env], &vec![&env]);
}

/// Payout history grows correctly in Active state after multiple operations.
#[test]
fn test_active_payout_history_grows() {
    let env = Env::default();
    let (client, _admin, _cid, _token) = setup_active_program(&env, 100_000);
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    let r3 = Address::generate(&env);

    client.single_payout(&r1, &10_000);
    client.batch_payout(&vec![&env, r2.clone(), r3.clone()], &vec![&env, 15_000i128, 5_000i128]);

    let info = client.get_program_info();
    assert_eq!(info.payout_history.len(), 3);
    assert_eq!(info.remaining_balance, 70_000);
}

// ---------------------------------------------------------------------------
// STATE: Paused
// Pause flags block specific operations; other ops remain unaffected.
// ---------------------------------------------------------------------------

/// Pausing lock prevents lock_program_funds.
#[test]
#[should_panic(expected = "Funds Paused")]
fn test_paused_lock_operation_blocked() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, contract_id) = make_client(&env);
    let (_, token_id) = fund_contract(&env, &contract_id, 100_000);
    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "hack-2026");
    client.init_program(&program_id, &admin, &token_id);
    client.initialize_contract(&admin);
    client.set_paused(&Some(true), &None, &None);

    client.lock_program_funds(&10_000);
}

/// Pausing release prevents single_payout.
#[test]
#[should_panic(expected = "Funds Paused")]
fn test_paused_single_payout_blocked() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, contract_id) = make_client(&env);
    let (_, token_id) = fund_contract(&env, &contract_id, 100_000);
    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "hack-2026");
    client.init_program(&program_id, &admin, &token_id);
    client.lock_program_funds(&100_000);
    client.initialize_contract(&admin);
    client.set_paused(&None, &Some(true), &None);

    let r = Address::generate(&env);
    client.single_payout(&r, &1_000);
}

/// Pausing release prevents batch_payout.
#[test]
#[should_panic(expected = "Funds Paused")]
fn test_paused_batch_payout_blocked() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, contract_id) = make_client(&env);
    let (_, token_id) = fund_contract(&env, &contract_id, 100_000);
    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "hack-2026");
    client.init_program(&program_id, &admin, &token_id);
    client.lock_program_funds(&100_000);
    client.initialize_contract(&admin);
    client.set_paused(&None, &Some(true), &None);

    let r = Address::generate(&env);
    client.batch_payout(&vec![&env, r], &vec![&env, 1_000i128]);
}

/// Unpausing restores operations — Active state is fully resumed.
#[test]
fn test_paused_to_active_resume_via_unpause() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, contract_id) = make_client(&env);
    let (token_client, token_id) = fund_contract(&env, &contract_id, 100_000);
    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "hack-2026");
    client.init_program(&program_id, &admin, &token_id);
    client.lock_program_funds(&100_000);
    client.initialize_contract(&admin);

    // Transition: Active → Paused
    client.set_paused(&None, &Some(true), &None);
    assert!(client.get_pause_flags().release_paused);

    // Transition: Paused → Active
    client.set_paused(&None, &Some(false), &None);
    assert!(!client.get_pause_flags().release_paused);

    // Payout is allowed again
    let r = Address::generate(&env);
    let data = client.single_payout(&r, &10_000);
    assert_eq!(data.remaining_balance, 90_000);
    assert_eq!(token_client.balance(&r), 10_000);
}

/// Pausing lock does NOT affect release (payout) operations.
#[test]
fn test_paused_lock_does_not_block_release() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, contract_id) = make_client(&env);
    let (token_client, token_id) = fund_contract(&env, &contract_id, 100_000);
    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "hack-2026");
    client.init_program(&program_id, &admin, &token_id);
    client.lock_program_funds(&100_000);
    client.initialize_contract(&admin);

    // Only lock is paused; release must still succeed
    client.set_paused(&Some(true), &None, &None);
    assert!(client.get_pause_flags().lock_paused);
    assert!(!client.get_pause_flags().release_paused);

    let r = Address::generate(&env);
    let data = client.single_payout(&r, &5_000);
    assert_eq!(data.remaining_balance, 95_000);
    assert_eq!(token_client.balance(&r), 5_000);
}

/// Pausing release does NOT affect lock (funding) operations.
#[test]
fn test_paused_release_does_not_block_lock() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, contract_id) = make_client(&env);
    // Mint enough for two lock operations
    let (_, token_id) = fund_contract(&env, &contract_id, 200_000);
    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "hack-2026");
    client.init_program(&program_id, &admin, &token_id);
    client.lock_program_funds(&100_000);
    client.initialize_contract(&admin);

    // Only release is paused; lock must still succeed
    client.set_paused(&None, &Some(true), &None);
    assert!(!client.get_pause_flags().lock_paused);
    assert!(client.get_pause_flags().release_paused);

    let data = client.lock_program_funds(&50_000);
    assert_eq!(data.total_funds, 150_000);
    assert_eq!(data.remaining_balance, 150_000);
}

/// All flags paused simultaneously — info/balance queries still work.
#[test]
fn test_fully_paused_query_still_works() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, contract_id) = make_client(&env);
    let (_, token_id) = fund_contract(&env, &contract_id, 100_000);
    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "hack-2026");
    client.init_program(&program_id, &admin, &token_id);
    client.lock_program_funds(&100_000);
    client.initialize_contract(&admin);
    client.set_paused(&Some(true), &Some(true), &Some(true));

    let flags = client.get_pause_flags();
    assert!(flags.lock_paused);
    assert!(flags.release_paused);
    assert!(flags.refund_paused);

    // State queries are not affected by pause
    let info = client.get_program_info();
    assert_eq!(info.remaining_balance, 100_000);
    assert_eq!(client.get_remaining_balance(), 100_000);
}

/// Default pause flags are all false (contract starts unpaused).
#[test]
fn test_default_pause_flags_all_false() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _cid) = make_client(&env);
    let admin = Address::generate(&env);
    client.initialize_contract(&admin);

    let flags = client.get_pause_flags();
    assert!(!flags.lock_paused);
    assert!(!flags.release_paused);
    assert!(!flags.refund_paused);
}

// ---------------------------------------------------------------------------
// STATE: Drained (remaining_balance == 0 after all payouts)
// ---------------------------------------------------------------------------

/// After a full single payout the program enters Drained state.
#[test]
fn test_drained_after_full_single_payout() {
    let env = Env::default();
    let (client, _admin, _cid, token_client) = setup_active_program(&env, 50_000);
    let r = Address::generate(&env);

    let data = client.single_payout(&r, &50_000);
    assert_eq!(data.remaining_balance, 0);
    assert_eq!(token_client.balance(&r), 50_000);
    assert_eq!(client.get_remaining_balance(), 0);
}

/// After a full batch payout the program enters Drained state.
#[test]
fn test_drained_after_full_batch_payout() {
    let env = Env::default();
    let (client, _admin, _cid, token_client) = setup_active_program(&env, 90_000);
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    let r3 = Address::generate(&env);

    let data = client.batch_payout(
        &vec![&env, r1.clone(), r2.clone(), r3.clone()],
        &vec![&env, 40_000i128, 30_000i128, 20_000i128],
    );
    assert_eq!(data.remaining_balance, 0);
    assert_eq!(token_client.balance(&r1), 40_000);
    assert_eq!(token_client.balance(&r2), 30_000);
    assert_eq!(token_client.balance(&r3), 20_000);
}

/// Further payouts from Drained state must be rejected.
#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_drained_further_payout_rejected() {
    let env = Env::default();
    let (client, _admin, _cid, _token) = setup_active_program(&env, 50_000);
    let r = Address::generate(&env);
    client.single_payout(&r, &50_000); // drains to 0
    client.single_payout(&r, &1);     // must panic
}

/// Re-locking funds after drain transitions back to Active (Drained → Active).
#[test]
fn test_drained_to_active_via_top_up() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, contract_id) = make_client(&env);
    // Mint enough for both initial lock and top-up
    let (token_client, token_id) = fund_contract(&env, &contract_id, 200_000);
    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "hack-2026");
    client.init_program(&program_id, &admin, &token_id);
    client.lock_program_funds(&100_000);

    // Drain
    let r = Address::generate(&env);
    client.single_payout(&r, &100_000);
    assert_eq!(client.get_remaining_balance(), 0);

    // Re-activate: Drained → Active
    let data = client.lock_program_funds(&80_000);
    assert_eq!(data.remaining_balance, 80_000);
    assert_eq!(data.total_funds, 180_000); // cumulative total

    // Payouts work again
    let r2 = Address::generate(&env);
    let data2 = client.single_payout(&r2, &30_000);
    assert_eq!(data2.remaining_balance, 50_000);
    assert_eq!(token_client.balance(&r2), 30_000);
}

/// Payout history is preserved and grows across all lifecycle transitions.
#[test]
fn test_payout_history_preserved_across_states() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, contract_id) = make_client(&env);
    let (_, token_id) = fund_contract(&env, &contract_id, 300_000);
    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "hack-2026");
    client.init_program(&program_id, &admin, &token_id);

    // Active: first batch of payouts
    client.lock_program_funds(&200_000);
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    client.single_payout(&r1, &100_000);
    client.single_payout(&r2, &100_000);

    // Now Drained
    assert_eq!(client.get_remaining_balance(), 0);
    let info = client.get_program_info();
    assert_eq!(info.payout_history.len(), 2);

    // Re-activate and pay out more
    client.lock_program_funds(&100_000);
    let r3 = Address::generate(&env);
    client.single_payout(&r3, &50_000);

    // All three payouts must be in history
    let info2 = client.get_program_info();
    assert_eq!(info2.payout_history.len(), 3);
    assert_eq!(info2.payout_history.get(0).unwrap().recipient, r1);
    assert_eq!(info2.payout_history.get(1).unwrap().recipient, r2);
    assert_eq!(info2.payout_history.get(2).unwrap().recipient, r3);
}

// ---------------------------------------------------------------------------
// RELEASE SCHEDULE: Lifecycle integration
// ---------------------------------------------------------------------------

/// Release schedules created before the timestamp are not triggered.
#[test]
fn test_schedule_before_timestamp_not_triggered() {
    let env = Env::default();
    let (client, _admin, _cid, token_client) = setup_active_program(&env, 100_000);
    let recipient = Address::generate(&env);

    let now = env.ledger().timestamp();
    client.create_program_release_schedule(&recipient, &30_000, &(now + 500));

    // Trigger at t < release_timestamp — should release 0 schedules
    env.ledger().set_timestamp(now + 499);
    let count = client.trigger_program_releases();
    assert_eq!(count, 0);
    assert_eq!(token_client.balance(&recipient), 0);
}

/// Release schedules are triggered at exactly the release_timestamp boundary.
#[test]
fn test_schedule_triggered_at_exact_timestamp() {
    let env = Env::default();
    let (client, _admin, _cid, token_client) = setup_active_program(&env, 100_000);
    let recipient = Address::generate(&env);

    let now = env.ledger().timestamp();
    client.create_program_release_schedule(&recipient, &25_000, &(now + 200));

    env.ledger().set_timestamp(now + 200);
    let count = client.trigger_program_releases();
    assert_eq!(count, 1);
    assert_eq!(token_client.balance(&recipient), 25_000);
    assert_eq!(client.get_remaining_balance(), 75_000);
}

/// A released schedule cannot be re-triggered (idempotency guard).
#[test]
fn test_schedule_not_released_twice() {
    let env = Env::default();
    let (client, _admin, _cid, token_client) = setup_active_program(&env, 100_000);
    let recipient = Address::generate(&env);

    let now = env.ledger().timestamp();
    client.create_program_release_schedule(&recipient, &20_000, &(now + 100));

    env.ledger().set_timestamp(now + 100);
    let count1 = client.trigger_program_releases();
    assert_eq!(count1, 1);

    // Second trigger must release nothing — schedule already marked released
    let count2 = client.trigger_program_releases();
    assert_eq!(count2, 0);
    assert_eq!(token_client.balance(&recipient), 20_000); // unchanged
}

/// Multiple schedules due at the same timestamp are all released in one call.
#[test]
fn test_multiple_schedules_same_timestamp_all_released() {
    let env = Env::default();
    let (client, _admin, _cid, token_client) = setup_active_program(&env, 100_000);
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    let r3 = Address::generate(&env);

    let now = env.ledger().timestamp();
    client.create_program_release_schedule(&r1, &10_000, &(now + 50));
    client.create_program_release_schedule(&r2, &15_000, &(now + 50));
    client.create_program_release_schedule(&r3, &20_000, &(now + 50));

    env.ledger().set_timestamp(now + 50);
    let count = client.trigger_program_releases();
    assert_eq!(count, 3);
    assert_eq!(token_client.balance(&r1), 10_000);
    assert_eq!(token_client.balance(&r2), 15_000);
    assert_eq!(token_client.balance(&r3), 20_000);
    assert_eq!(client.get_remaining_balance(), 55_000);
}

// ---------------------------------------------------------------------------
// COMPLETE LIFECYCLE INTEGRATION
// ---------------------------------------------------------------------------

/// Full end-to-end: Uninitialized → Initialized → Active → Paused
///                  → Active (resumed) → Drained → Active (top-up) → Drained.
#[test]
fn test_complete_lifecycle_all_transitions() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, contract_id) = make_client(&env);
    let (token_client, token_id) = fund_contract(&env, &contract_id, 500_000);
    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "hack-2026");

    // Uninitialized → Initialized
    let data = client.init_program(&program_id, &admin, &token_id);
    assert_eq!(data.total_funds, 0);
    assert_eq!(data.remaining_balance, 0);

    // Initialized → Active
    let data = client.lock_program_funds(&300_000);
    assert_eq!(data.total_funds, 300_000);
    assert_eq!(data.remaining_balance, 300_000);

    // Active: perform payouts
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    client.single_payout(&r1, &50_000);
    client.batch_payout(&vec![&env, r2.clone()], &vec![&env, 50_000i128]);
    assert_eq!(client.get_remaining_balance(), 200_000);

    // Active → Paused
    client.initialize_contract(&admin);
    client.set_paused(&None, &Some(true), &None);
    assert!(client.get_pause_flags().release_paused);

    // Paused → Active (resume)
    client.set_paused(&None, &Some(false), &None);
    assert!(!client.get_pause_flags().release_paused);

    // Active: drain the rest
    let r3 = Address::generate(&env);
    client.single_payout(&r3, &200_000);
    assert_eq!(client.get_remaining_balance(), 0);

    // Drained → Active (top-up)
    let data = client.lock_program_funds(&100_000);
    assert_eq!(data.remaining_balance, 100_000);

    // Active: final payout — drains again
    let r4 = Address::generate(&env);
    client.single_payout(&r4, &100_000);
    assert_eq!(client.get_remaining_balance(), 0);

    // Verify complete payout history
    let info = client.get_program_info();
    // r1 (single), r2 (batch), r3 (single drain), r4 (final)
    assert_eq!(info.payout_history.len(), 4);
    assert_eq!(info.total_funds, 400_000); // 300_000 + 100_000 top-up

    // Final token balances
    assert_eq!(token_client.balance(&r1), 50_000);
    assert_eq!(token_client.balance(&r2), 50_000);
    assert_eq!(token_client.balance(&r3), 200_000);
    assert_eq!(token_client.balance(&r4), 100_000);
    assert_eq!(token_client.balance(&contract_id), 0);
}
