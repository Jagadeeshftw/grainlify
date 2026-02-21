#![cfg(test)]

//! # Granular Pause Per-Operation Tests — Program Escrow
//!
//! Tests every combination of pause flags (lock, release, refund) to confirm
//! that each flag blocks only its intended operation and leaves all other
//! operations unaffected.
//!
//! ## Pause Flag Matrix
//!
//! | lock_paused | release_paused | refund_paused | lock_funds | single_payout | batch_payout |
//! |-------------|----------------|---------------|------------|---------------|--------------|
//! | false       | false          | false         | ✓          | ✓             | ✓            |
//! | true        | false          | false         | ✗          | ✓             | ✓            |
//! | false       | true           | false         | ✓          | ✗             | ✗            |
//! | false       | false          | true          | ✓          | ✓             | ✓            |
//! | true        | true           | false         | ✗          | ✗             | ✗            |
//! | true        | false          | true          | ✗          | ✓             | ✓            |
//! | false       | true           | true          | ✓          | ✗             | ✗            |
//! | true        | true           | true          | ✗          | ✗             | ✗            |

use super::*;
use soroban_sdk::{
    testutils::Address as _,
    token, vec, Address, Env, String,
};

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/// Set up a contract + admin, register a token, init the program, and mint
/// `initial_balance` tokens into the contract address.
///
/// Returns `(client, token_client)`.
fn setup(
    env: &Env,
    initial_balance: i128,
) -> (ProgramEscrowContractClient<'static>, token::Client<'static>) {
    env.mock_all_auths();

    // Register escrow contract
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);

    // Register token (SAC)
    let token_admin = Address::generate(env);
    let token_addr = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_client = token::Client::new(env, &token_addr);
    let token_sac = token::StellarAssetClient::new(env, &token_addr);

    // Initialize the escrow admin (required for set_paused)
    let admin = Address::generate(env);
    client.initialize_contract(&admin);

    // Initialize program
    let payout_key = Address::generate(env);
    let program_id = String::from_str(env, "test-prog");
    client.init_program(&program_id, &payout_key, &token_addr);

    // Fund the contract with tokens and lock them
    if initial_balance > 0 {
        token_sac.mint(&contract_id, &initial_balance);
        client.lock_program_funds(&initial_balance);
    }

    (client, token_client)
}

// ---------------------------------------------------------------------------
// § 1  Default state — all flags false
// ---------------------------------------------------------------------------

#[test]
fn test_default_all_flags_false() {
    let env = Env::default();
    let (client, _token) = setup(&env, 0);

    let flags = client.get_pause_flags();
    assert!(!flags.lock_paused, "lock_paused should default to false");
    assert!(!flags.release_paused, "release_paused should default to false");
    assert!(!flags.refund_paused, "refund_paused should default to false");
}

// ---------------------------------------------------------------------------
// § 2  Individual flag set / unset (no enforcement yet)
// ---------------------------------------------------------------------------

#[test]
fn test_set_lock_paused_only() {
    let env = Env::default();
    let (client, _token) = setup(&env, 0);

    client.set_paused(&Some(true), &None, &None);
    let flags = client.get_pause_flags();
    assert!(flags.lock_paused);
    assert!(!flags.release_paused);
    assert!(!flags.refund_paused);
}

#[test]
fn test_set_release_paused_only() {
    let env = Env::default();
    let (client, _token) = setup(&env, 0);

    client.set_paused(&None, &Some(true), &None);
    let flags = client.get_pause_flags();
    assert!(!flags.lock_paused);
    assert!(flags.release_paused);
    assert!(!flags.refund_paused);
}

#[test]
fn test_set_refund_paused_only() {
    let env = Env::default();
    let (client, _token) = setup(&env, 0);

    client.set_paused(&None, &None, &Some(true));
    let flags = client.get_pause_flags();
    assert!(!flags.lock_paused);
    assert!(!flags.release_paused);
    assert!(flags.refund_paused);
}

#[test]
fn test_unset_lock_paused() {
    let env = Env::default();
    let (client, _token) = setup(&env, 0);

    client.set_paused(&Some(true), &None, &None);
    client.set_paused(&Some(false), &None, &None);
    let flags = client.get_pause_flags();
    assert!(!flags.lock_paused);
}

#[test]
fn test_unset_release_paused() {
    let env = Env::default();
    let (client, _token) = setup(&env, 0);

    client.set_paused(&None, &Some(true), &None);
    client.set_paused(&None, &Some(false), &None);
    let flags = client.get_pause_flags();
    assert!(!flags.release_paused);
}

// ---------------------------------------------------------------------------
// § 3  None arguments leave other flags unchanged
// ---------------------------------------------------------------------------

#[test]
fn test_partial_update_preserves_other_flags() {
    let env = Env::default();
    let (client, _token) = setup(&env, 0);

    // Pause all three
    client.set_paused(&Some(true), &Some(true), &Some(true));

    // Only unpause release; lock and refund must remain paused
    client.set_paused(&None, &Some(false), &None);
    let flags = client.get_pause_flags();
    assert!(flags.lock_paused, "lock_paused should remain true");
    assert!(!flags.release_paused, "release_paused should be false after unset");
    assert!(flags.refund_paused, "refund_paused should remain true");
}

// ---------------------------------------------------------------------------
// § 4  lock_paused = true  ─►  lock_program_funds blocked
// ---------------------------------------------------------------------------

#[test]
#[should_panic(expected = "Funds Paused")]
fn test_lock_blocked_when_lock_paused() {
    let env = Env::default();
    let (client, _token) = setup(&env, 0);

    client.set_paused(&Some(true), &None, &None);
    client.lock_program_funds(&500);
}

/// lock_paused does NOT block single_payout
#[test]
fn test_release_allowed_when_only_lock_paused() {
    let env = Env::default();
    let (client, _token) = setup(&env, 1_000);

    client.set_paused(&Some(true), &None, &None);

    let recipient = Address::generate(&env);
    // Should succeed — release_paused is false
    let data = client.single_payout(&recipient, &200);
    assert_eq!(data.remaining_balance, 800);
}

/// lock_paused does NOT block batch_payout
#[test]
fn test_batch_allowed_when_only_lock_paused() {
    let env = Env::default();
    let (client, _token) = setup(&env, 1_000);

    client.set_paused(&Some(true), &None, &None);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    let data = client.batch_payout(
        &vec![&env, r1, r2],
        &vec![&env, 100i128, 200i128],
    );
    assert_eq!(data.remaining_balance, 700);
}

// ---------------------------------------------------------------------------
// § 5  release_paused = true  ─►  single_payout and batch_payout blocked
// ---------------------------------------------------------------------------

#[test]
#[should_panic(expected = "Funds Paused")]
fn test_single_payout_blocked_when_release_paused() {
    let env = Env::default();
    let (client, _token) = setup(&env, 1_000);

    client.set_paused(&None, &Some(true), &None);
    let recipient = Address::generate(&env);
    client.single_payout(&recipient, &100);
}

#[test]
#[should_panic(expected = "Funds Paused")]
fn test_batch_payout_blocked_when_release_paused() {
    let env = Env::default();
    let (client, _token) = setup(&env, 1_000);

    client.set_paused(&None, &Some(true), &None);
    let r1 = Address::generate(&env);
    client.batch_payout(&vec![&env, r1], &vec![&env, 100i128]);
}

/// release_paused does NOT block lock_program_funds
#[test]
fn test_lock_allowed_when_only_release_paused() {
    let env = Env::default();
    let (client, _token) = setup(&env, 0);

    client.set_paused(&None, &Some(true), &None);

    // Should succeed — lock_paused is false
    let data = client.lock_program_funds(&300);
    assert_eq!(data.remaining_balance, 300);
}

// ---------------------------------------------------------------------------
// § 6  refund_paused = true  ─►  program-escrow has no refund op,
//       so lock and release must still work normally
// ---------------------------------------------------------------------------

/// refund_paused does NOT block lock_program_funds
#[test]
fn test_lock_allowed_when_only_refund_paused() {
    let env = Env::default();
    let (client, _token) = setup(&env, 0);

    client.set_paused(&None, &None, &Some(true));
    let data = client.lock_program_funds(&400);
    assert_eq!(data.remaining_balance, 400);
}

/// refund_paused does NOT block single_payout
#[test]
fn test_single_payout_allowed_when_only_refund_paused() {
    let env = Env::default();
    let (client, _token) = setup(&env, 1_000);

    client.set_paused(&None, &None, &Some(true));
    let recipient = Address::generate(&env);
    let data = client.single_payout(&recipient, &300);
    assert_eq!(data.remaining_balance, 700);
}

/// refund_paused does NOT block batch_payout
#[test]
fn test_batch_allowed_when_only_refund_paused() {
    let env = Env::default();
    let (client, _token) = setup(&env, 1_000);

    client.set_paused(&None, &None, &Some(true));
    let r1 = Address::generate(&env);
    let data = client.batch_payout(&vec![&env, r1], &vec![&env, 100i128]);
    assert_eq!(data.remaining_balance, 900);
}

// ---------------------------------------------------------------------------
// § 7  Combination: lock + release paused
// ---------------------------------------------------------------------------

#[test]
#[should_panic(expected = "Funds Paused")]
fn test_lock_blocked_when_lock_and_release_paused() {
    let env = Env::default();
    let (client, _token) = setup(&env, 0);

    client.set_paused(&Some(true), &Some(true), &None);
    client.lock_program_funds(&100);
}

#[test]
#[should_panic(expected = "Funds Paused")]
fn test_single_payout_blocked_when_lock_and_release_paused() {
    let env = Env::default();
    let (client, _token) = setup(&env, 500);

    client.set_paused(&Some(true), &Some(true), &None);
    let recipient = Address::generate(&env);
    client.single_payout(&recipient, &100);
}

#[test]
#[should_panic(expected = "Funds Paused")]
fn test_batch_payout_blocked_when_lock_and_release_paused() {
    let env = Env::default();
    let (client, _token) = setup(&env, 500);

    client.set_paused(&Some(true), &Some(true), &None);
    let r1 = Address::generate(&env);
    client.batch_payout(&vec![&env, r1], &vec![&env, 100i128]);
}

// ---------------------------------------------------------------------------
// § 8  Combination: lock + refund paused  (release still allowed)
// ---------------------------------------------------------------------------

#[test]
#[should_panic(expected = "Funds Paused")]
fn test_lock_blocked_when_lock_and_refund_paused() {
    let env = Env::default();
    let (client, _token) = setup(&env, 0);

    client.set_paused(&Some(true), &None, &Some(true));
    client.lock_program_funds(&100);
}

#[test]
fn test_single_payout_allowed_when_lock_and_refund_paused() {
    let env = Env::default();
    let (client, _token) = setup(&env, 500);

    client.set_paused(&Some(true), &None, &Some(true));
    let recipient = Address::generate(&env);
    let data = client.single_payout(&recipient, &100);
    assert_eq!(data.remaining_balance, 400);
}

#[test]
fn test_batch_allowed_when_lock_and_refund_paused() {
    let env = Env::default();
    let (client, _token) = setup(&env, 500);

    client.set_paused(&Some(true), &None, &Some(true));
    let r1 = Address::generate(&env);
    let data = client.batch_payout(&vec![&env, r1], &vec![&env, 200i128]);
    assert_eq!(data.remaining_balance, 300);
}

// ---------------------------------------------------------------------------
// § 9  Combination: release + refund paused  (lock still allowed)
// ---------------------------------------------------------------------------

#[test]
fn test_lock_allowed_when_release_and_refund_paused() {
    let env = Env::default();
    let (client, _token) = setup(&env, 0);

    client.set_paused(&None, &Some(true), &Some(true));
    let data = client.lock_program_funds(&600);
    assert_eq!(data.remaining_balance, 600);
}

#[test]
#[should_panic(expected = "Funds Paused")]
fn test_single_payout_blocked_when_release_and_refund_paused() {
    let env = Env::default();
    let (client, _token) = setup(&env, 600);

    client.set_paused(&None, &Some(true), &Some(true));
    let recipient = Address::generate(&env);
    client.single_payout(&recipient, &100);
}

#[test]
#[should_panic(expected = "Funds Paused")]
fn test_batch_blocked_when_release_and_refund_paused() {
    let env = Env::default();
    let (client, _token) = setup(&env, 600);

    client.set_paused(&None, &Some(true), &Some(true));
    let r1 = Address::generate(&env);
    client.batch_payout(&vec![&env, r1], &vec![&env, 100i128]);
}

// ---------------------------------------------------------------------------
// § 10  All flags paused
// ---------------------------------------------------------------------------

#[test]
#[should_panic(expected = "Funds Paused")]
fn test_lock_blocked_when_all_paused() {
    let env = Env::default();
    let (client, _token) = setup(&env, 0);

    client.set_paused(&Some(true), &Some(true), &Some(true));
    client.lock_program_funds(&100);
}

#[test]
#[should_panic(expected = "Funds Paused")]
fn test_single_payout_blocked_when_all_paused() {
    let env = Env::default();
    let (client, _token) = setup(&env, 500);

    client.set_paused(&Some(true), &Some(true), &Some(true));
    let recipient = Address::generate(&env);
    client.single_payout(&recipient, &100);
}

#[test]
#[should_panic(expected = "Funds Paused")]
fn test_batch_payout_blocked_when_all_paused() {
    let env = Env::default();
    let (client, _token) = setup(&env, 500);

    client.set_paused(&Some(true), &Some(true), &Some(true));
    let r1 = Address::generate(&env);
    client.batch_payout(&vec![&env, r1], &vec![&env, 100i128]);
}

// ---------------------------------------------------------------------------
// § 11  Resume after pause — operations restored
// ---------------------------------------------------------------------------

#[test]
fn test_lock_restored_after_unpause() {
    let env = Env::default();
    let (client, _token) = setup(&env, 0);

    client.set_paused(&Some(true), &None, &None);
    // Confirm it's blocked
    assert!(client.try_lock_program_funds(&200).is_err());

    client.set_paused(&Some(false), &None, &None);
    // Now it should succeed
    let data = client.lock_program_funds(&200);
    assert_eq!(data.remaining_balance, 200);
}

#[test]
fn test_single_payout_restored_after_unpause() {
    let env = Env::default();
    let (client, _token) = setup(&env, 1_000);

    client.set_paused(&None, &Some(true), &None);
    let recipient = Address::generate(&env);
    assert!(client.try_single_payout(&recipient, &100).is_err());

    client.set_paused(&None, &Some(false), &None);
    let data = client.single_payout(&recipient, &100);
    assert_eq!(data.remaining_balance, 900);
}

#[test]
fn test_batch_payout_restored_after_unpause() {
    let env = Env::default();
    let (client, _token) = setup(&env, 1_000);

    client.set_paused(&None, &Some(true), &None);
    let r1 = Address::generate(&env);
    assert!(
        client
            .try_batch_payout(&vec![&env, r1.clone()], &vec![&env, 100i128])
            .is_err()
    );

    client.set_paused(&None, &Some(false), &None);
    let data = client.batch_payout(&vec![&env, r1], &vec![&env, 100i128]);
    assert_eq!(data.remaining_balance, 900);
}

// ---------------------------------------------------------------------------
// § 12  get_program_info / get_remaining_balance unaffected by any flag
// ---------------------------------------------------------------------------

#[test]
fn test_query_functions_unaffected_when_all_paused() {
    let env = Env::default();
    let (client, _token) = setup(&env, 500);

    client.set_paused(&Some(true), &Some(true), &Some(true));

    // Read-only queries must still succeed
    let info = client.get_program_info();
    assert_eq!(info.remaining_balance, 500);

    let balance = client.get_remaining_balance();
    assert_eq!(balance, 500);
}
