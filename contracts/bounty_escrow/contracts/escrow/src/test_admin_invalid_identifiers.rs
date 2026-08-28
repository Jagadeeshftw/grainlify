//! Negative tests for zero and malformed contract identifiers across all admin APIs.
//!
//! Covers every admin entrypoint that accepts a `bounty_id` parameter:
//! - `release_funds`, `approve_refund`, `partial_release`
//! - `freeze_escrow`, `unfreeze_escrow`
//! - `get_escrow_freeze_record` (read-only query)
//!
//! Each test verifies:
//! 1. The correct error variant is returned for invalid identifiers.
//! 2. No partial state is created (no freeze records created for valid escrows).

#![cfg(test)]

use crate::{BountyEscrowContract, BountyEscrowContractClient, Error, RefundMode};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{token, Address, Env};

/// Helper: deploy an initialized escrow contract and mint tokens to a depositor.
fn setup() -> (Env, BountyEscrowContractClient<'static>, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token = token_id.address();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let sac = token::StellarAssetClient::new(&env, &token);
    sac.mint(&depositor, &1_000_000);

    client.init(&admin, &token);

    (env, client, admin, depositor, contributor)
}

/// Lock a valid escrow and return its bounty_id for subsequent use in negative tests.
fn lock_valid_escrow(
    client: &BountyEscrowContractClient<'static>,
    depositor: &Address,
) -> u64 {
    let bounty_id = 42;
    let amount = 5_000;
    let deadline = client.env.ledger().timestamp() + 86_400;
    client.lock_funds(depositor, &bounty_id, &amount, &deadline);
    bounty_id
}

// ===========================================================================
// release_funds — invalid identifiers
// ===========================================================================

#[test]
fn test_release_funds_zero_id_returns_bounty_not_found() {
    let (_env, client, _admin, _depositor, contributor) = setup();
    let result = client.try_release_funds(&0u64, &contributor);
    assert!(result.is_err(), "release_funds(0) should fail");
    assert_eq!(
        result.unwrap_err().unwrap(),
        Error::BountyNotFound,
        "zero bounty_id should return BountyNotFound"
    );
}

#[test]
fn test_release_funds_max_id_returns_bounty_not_found() {
    let (_env, client, _admin, _depositor, contributor) = setup();
    let result = client.try_release_funds(&u64::MAX, &contributor);
    assert!(result.is_err(), "release_funds(MAX) should fail");
    assert_eq!(
        result.unwrap_err().unwrap(),
        Error::BountyNotFound,
        "u64::MAX bounty_id should return BountyNotFound"
    );
}

#[test]
fn test_release_funds_nonexistent_id_returns_bounty_not_found() {
    let (_env, client, _admin, _depositor, contributor) = setup();
    let result = client.try_release_funds(&999_999u64, &contributor);
    assert!(result.is_err(), "release_funds(nonexistent) should fail");
    assert_eq!(
        result.unwrap_err().unwrap(),
        Error::BountyNotFound,
        "non-existent bounty_id should return BountyNotFound"
    );
}

#[test]
fn test_release_funds_zero_id_no_freeze_record_created() {
    let (_env, client, _admin, depositor, contributor) = setup();
    let valid_id = lock_valid_escrow(&client, &depositor);

    // Confirm no freeze record on valid escrow before the call
    assert!(
        client.get_escrow_freeze_record(&valid_id).is_none(),
        "valid escrow should not be frozen before release_funds(0)"
    );

    let result = client.try_release_funds(&0u64, &contributor);
    assert!(result.is_err());

    // Confirm no freeze record was spuriously created
    assert!(
        client.get_escrow_freeze_record(&valid_id).is_none(),
        "valid escrow must not be frozen after failed release_funds(0)"
    );
    // Confirm bounty 0 also has no freeze record
    assert!(
        client.get_escrow_freeze_record(&0u64).is_none(),
        "bounty 0 must not have a freeze record"
    );
}

// ===========================================================================
// approve_refund — invalid identifiers
// ===========================================================================

#[test]
fn test_approve_refund_zero_id_returns_bounty_not_found() {
    let (_env, client, _admin, _depositor, contributor) = setup();
    let result =
        client.try_approve_refund(&0u64, &100i128, &contributor, &RefundMode::Full);
    assert!(result.is_err(), "approve_refund(0) should fail");
    assert_eq!(
        result.unwrap_err().unwrap(),
        Error::BountyNotFound,
        "zero bounty_id should return BountyNotFound"
    );
}

#[test]
fn test_approve_refund_max_id_returns_bounty_not_found() {
    let (_env, client, _admin, _depositor, contributor) = setup();
    let result = client.try_approve_refund(
        &u64::MAX,
        &100i128,
        &contributor,
        &RefundMode::Full,
    );
    assert!(result.is_err(), "approve_refund(MAX) should fail");
    assert_eq!(
        result.unwrap_err().unwrap(),
        Error::BountyNotFound,
        "u64::MAX bounty_id should return BountyNotFound"
    );
}

#[test]
fn test_approve_refund_nonexistent_id_returns_bounty_not_found() {
    let (_env, client, _admin, _depositor, contributor) = setup();
    let result =
        client.try_approve_refund(&777u64, &100i128, &contributor, &RefundMode::Full);
    assert!(result.is_err(), "approve_refund(nonexistent) should fail");
    assert_eq!(
        result.unwrap_err().unwrap(),
        Error::BountyNotFound,
        "non-existent bounty_id should return BountyNotFound"
    );
}

#[test]
fn test_approve_refund_zero_id_no_freeze_record_created() {
    let (_env, client, _admin, depositor, contributor) = setup();
    let valid_id = lock_valid_escrow(&client, &depositor);

    let result =
        client.try_approve_refund(&0u64, &100i128, &contributor, &RefundMode::Full);
    assert!(result.is_err());

    // Confirm no freeze record on valid escrow or bounty 0
    assert!(
        client.get_escrow_freeze_record(&valid_id).is_none(),
        "valid escrow must not be frozen after failed approve_refund(0)"
    );
    assert!(
        client.get_escrow_freeze_record(&0u64).is_none(),
        "bounty 0 must not have a freeze record"
    );
}

// ===========================================================================
// partial_release — invalid identifiers
// ===========================================================================

#[test]
fn test_partial_release_zero_id_returns_bounty_not_found() {
    let (_env, client, _admin, _depositor, contributor) = setup();
    let result = client.try_partial_release(&0u64, &contributor, &100i128);
    assert!(result.is_err(), "partial_release(0) should fail");
    assert_eq!(
        result.unwrap_err().unwrap(),
        Error::BountyNotFound,
        "zero bounty_id should return BountyNotFound"
    );
}

#[test]
fn test_partial_release_max_id_returns_bounty_not_found() {
    let (_env, client, _admin, _depositor, contributor) = setup();
    let result = client.try_partial_release(&u64::MAX, &contributor, &100i128);
    assert!(result.is_err(), "partial_release(MAX) should fail");
    assert_eq!(
        result.unwrap_err().unwrap(),
        Error::BountyNotFound,
        "u64::MAX bounty_id should return BountyNotFound"
    );
}

#[test]
fn test_partial_release_nonexistent_id_returns_bounty_not_found() {
    let (_env, client, _admin, _depositor, contributor) = setup();
    let result = client.try_partial_release(&123_456u64, &contributor, &100i128);
    assert!(
        result.is_err(),
        "partial_release(nonexistent) should fail"
    );
    assert_eq!(
        result.unwrap_err().unwrap(),
        Error::BountyNotFound,
        "non-existent bounty_id should return BountyNotFound"
    );
}

#[test]
fn test_partial_release_zero_id_no_freeze_record_created() {
    let (_env, client, _admin, depositor, contributor) = setup();
    let valid_id = lock_valid_escrow(&client, &depositor);

    let result = client.try_partial_release(&0u64, &contributor, &100i128);
    assert!(result.is_err());

    assert!(
        client.get_escrow_freeze_record(&valid_id).is_none(),
        "valid escrow must not be frozen after failed partial_release(0)"
    );
    assert!(
        client.get_escrow_freeze_record(&0u64).is_none(),
        "bounty 0 must not have a freeze record"
    );
}

// ===========================================================================
// freeze_escrow — invalid identifiers
// ===========================================================================

#[test]
fn test_freeze_escrow_zero_id_returns_bounty_not_found() {
    let (_env, client, _admin, _depositor, _contributor) = setup();
    let result = client.try_freeze_escrow(&0u64, &None);
    assert!(result.is_err(), "freeze_escrow(0) should fail");
    assert_eq!(
        result.unwrap_err().unwrap(),
        Error::BountyNotFound,
        "zero bounty_id should return BountyNotFound"
    );
}

#[test]
fn test_freeze_escrow_max_id_returns_bounty_not_found() {
    let (_env, client, _admin, _depositor, _contributor) = setup();
    let result = client.try_freeze_escrow(&u64::MAX, &None);
    assert!(result.is_err(), "freeze_escrow(MAX) should fail");
    assert_eq!(
        result.unwrap_err().unwrap(),
        Error::BountyNotFound,
        "u64::MAX bounty_id should return BountyNotFound"
    );
}

#[test]
fn test_freeze_escrow_nonexistent_id_returns_bounty_not_found() {
    let (_env, client, _admin, _depositor, _contributor) = setup();
    let result = client.try_freeze_escrow(&555u64, &None);
    assert!(result.is_err(), "freeze_escrow(nonexistent) should fail");
    assert_eq!(
        result.unwrap_err().unwrap(),
        Error::BountyNotFound,
        "non-existent bounty_id should return BountyNotFound"
    );
}

#[test]
fn test_freeze_escrow_zero_id_no_state_change() {
    let (_env, client, _admin, depositor, _contributor) = setup();
    let valid_id = lock_valid_escrow(&client, &depositor);

    // Verify valid escrow is NOT frozen before the call
    assert!(
        client.get_escrow_freeze_record(&valid_id).is_none(),
        "valid escrow should not be frozen before freeze_escrow(0)"
    );

    let result = client.try_freeze_escrow(&0u64, &None);
    assert!(result.is_err());

    // Verify valid escrow is still NOT frozen after the failed call
    assert!(
        client.get_escrow_freeze_record(&valid_id).is_none(),
        "valid escrow must not be affected by failed freeze_escrow(0)"
    );
}

// ===========================================================================
// unfreeze_escrow — invalid identifiers
// ===========================================================================

#[test]
fn test_unfreeze_escrow_zero_id_returns_bounty_not_found() {
    let (_env, client, _admin, _depositor, _contributor) = setup();
    let result = client.try_unfreeze_escrow(&0u64);
    assert!(result.is_err(), "unfreeze_escrow(0) should fail");
    assert_eq!(
        result.unwrap_err().unwrap(),
        Error::BountyNotFound,
        "zero bounty_id should return BountyNotFound"
    );
}

#[test]
fn test_unfreeze_escrow_max_id_returns_bounty_not_found() {
    let (_env, client, _admin, _depositor, _contributor) = setup();
    let result = client.try_unfreeze_escrow(&u64::MAX);
    assert!(result.is_err(), "unfreeze_escrow(MAX) should fail");
    assert_eq!(
        result.unwrap_err().unwrap(),
        Error::BountyNotFound,
        "u64::MAX bounty_id should return BountyNotFound"
    );
}

#[test]
fn test_unfreeze_escrow_nonexistent_id_returns_bounty_not_found() {
    let (_env, client, _admin, _depositor, _contributor) = setup();
    let result = client.try_unfreeze_escrow(&888u64);
    assert!(result.is_err(), "unfreeze_escrow(nonexistent) should fail");
    assert_eq!(
        result.unwrap_err().unwrap(),
        Error::BountyNotFound,
        "non-existent bounty_id should return BountyNotFound"
    );
}

// ===========================================================================
// Cross-cutting: all invalid IDs yield the same error variant (stability)
// ===========================================================================

#[test]
fn test_all_admin_apis_return_same_error_for_zero_id() {
    let (_env, client, _admin, depositor, contributor) = setup();
    let _valid_id = lock_valid_escrow(&client, &depositor);
    let zero = 0u64;

    // release_funds
    let r = client.try_release_funds(&zero, &contributor);
    assert_eq!(r.unwrap_err().unwrap(), Error::BountyNotFound);

    // approve_refund
    let r = client.try_approve_refund(&zero, &100, &contributor, &RefundMode::Full);
    assert_eq!(r.unwrap_err().unwrap(), Error::BountyNotFound);

    // partial_release
    let r = client.try_partial_release(&zero, &contributor, &100);
    assert_eq!(r.unwrap_err().unwrap(), Error::BountyNotFound);

    // freeze_escrow
    let r = client.try_freeze_escrow(&zero, &None);
    assert_eq!(r.unwrap_err().unwrap(), Error::BountyNotFound);

    // unfreeze_escrow
    let r = client.try_unfreeze_escrow(&zero);
    assert_eq!(r.unwrap_err().unwrap(), Error::BountyNotFound);
}

#[test]
fn test_valid_escrow_unaffected_by_invalid_id_attempts() {
    let (_env, client, _admin, depositor, contributor) = setup();
    let valid_id = lock_valid_escrow(&client, &depositor);

    // Confirm valid escrow is not frozen
    assert!(
        client.get_escrow_freeze_record(&valid_id).is_none(),
        "valid escrow should not be frozen initially"
    );

    // Attempt a bunch of invalid operations
    let _ = client.try_release_funds(&0u64, &contributor);
    let _ = client.try_release_funds(&u64::MAX, &contributor);
    let _ = client.try_freeze_escrow(&0u64, &None);
    let _ = client.try_partial_release(&0u64, &contributor, &100);

    // Valid escrow must still not be frozen
    assert!(
        client.get_escrow_freeze_record(&valid_id).is_none(),
        "valid escrow must not be frozen after invalid attempts"
    );

    // Now successfully release the valid escrow
    client.release_funds(&valid_id, &contributor);
}
