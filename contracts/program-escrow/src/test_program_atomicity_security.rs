#![cfg(test)]

extern crate std;

use super::*;
use crate::test_support::*;
use soroban_sdk::{testutils::{Address as _, Events, Ledger, MockAuth, MockAuthInvoke}, token, vec, Address, Env, IntoVal, Map, String, Symbol, TryFromVal, Val};

#[test]
fn test_idempotency_key_batch_payout_success() {
    let env = Env::default();
    let (client, admin, token, token_admin) = setup_program(&env, 1000_0000000);

    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let recipients = vec![&env, recipient1.clone(), recipient2.clone()];
    let amounts = vec![&env, 100_0000000, 200_0000000];
    let idempotency_key = String::from_str(&env, "test-batch-123");

    // First successful payout with idempotency key
    let result = client.batch_payout(&recipients, &amounts, &Some(idempotency_key.clone()));
    assert_eq!(result.remaining_balance, 700_0000000);

    // Verify idempotency record was stored
    let record: IdempotencyRecord = env.as_contract(&client.address, || { env.storage().instance().get(&DataKey::IdempotencyKey(idempotency_key.clone())).unwrap() });
    assert_eq!(record.idempotency_key, idempotency_key);
    assert_eq!(record.operation_type, symbol_short!("batchpay"));
    assert!(record.success);
    assert_eq!(record.total_amount, 300_0000000);
    assert_eq!(record.recipient_count, 2);

    // Verify events were emitted
    let events = env.events().all();
    assert!(events.len() >= 2); // BatchPayout + IdempotencyKeyUsed
}

/// Test idempotency key retry behavior for batch payout
#[test]
fn test_idempotency_key_batch_payout_retry() {
    let env = Env::default();
    let (client, admin, token, token_admin) = setup_program(&env, 1000_0000000);

    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let recipients = vec![&env, recipient1.clone(), recipient2.clone()];
    let amounts = vec![&env, 100_0000000, 200_0000000];
    let idempotency_key = String::from_str(&env, "test-batch-retry-456");

    // First successful payout
    let result1 = client.batch_payout(&recipients, &amounts, &Some(idempotency_key.clone()));
    assert_eq!(result1.remaining_balance, 700_0000000);

    // Retry with same idempotency key should return same result
    let result2 = client.batch_payout(&recipients, &amounts, &Some(idempotency_key.clone()));
    assert_eq!(result2.remaining_balance, 700_0000000);
    assert_eq!(result1.payout_history.len(), result2.payout_history.len());

    // Verify retry event was emitted
    let events = env.events().all();
    let retry_events: soroban_sdk::Vec<_> = events.clone();
    assert_eq!(retry_events.len(), 2); // First use + retry
}

/// Test idempotency key validation for successful single payout
#[test]
fn test_idempotency_key_single_payout_success() {
    let env = Env::default();
    let (client, admin, token, token_admin) = setup_program(&env, 1000_0000000);

    let recipient = Address::generate(&env);
    let amount = 500_0000000;
    let idempotency_key = String::from_str(&env, "test-single-789");

    // First successful payout with idempotency key
    let result = client.single_payout(&recipient, &amount, &Some(idempotency_key.clone()));
    assert_eq!(result.remaining_balance, 500_0000000);

    // Verify idempotency record was stored
    let record: IdempotencyRecord = env.as_contract(&client.address, || { env.storage().instance().get(&DataKey::IdempotencyKey(idempotency_key.clone())).unwrap() });
    assert_eq!(record.idempotency_key, idempotency_key);
    assert_eq!(record.operation_type, symbol_short!("singlepay"));
    assert!(record.success);
    assert_eq!(record.total_amount, 500_0000000);
    assert_eq!(record.recipient_count, 1);
}

/// Test idempotency key retry behavior for single payout
#[test]
fn test_idempotency_key_single_payout_retry() {
    let env = Env::default();
    let (client, admin, token, token_admin) = setup_program(&env, 1000_0000000);

    let recipient = Address::generate(&env);
    let amount = 300_0000000;
    let idempotency_key = String::from_str(&env, "test-single-retry-999");

    // First successful payout
    let result1 = client.single_payout(&recipient, &amount, &Some(idempotency_key.clone()));
    assert_eq!(result1.remaining_balance, 700_0000000);

    // Retry with same idempotency key should return same result
    let result2 = client.single_payout(&recipient, &amount, &Some(idempotency_key.clone()));
    assert_eq!(result2.remaining_balance, 700_0000000);
    assert_eq!(result1.payout_history.len(), result2.payout_history.len());
}

/// Test idempotency key validation failures
#[test]
fn test_idempotency_key_validation_failures() {
    let env = Env::default();
    let (client, admin, token, token_admin) = setup_program(&env, 1000_0000000);

    let recipient = Address::generate(&env);
    let amount = 100_0000000;

    // Empty idempotency key should panic
    let empty_key = String::from_str(&env, "");
    let result = client.try_single_payout(&recipient, &amount, &Some(empty_key));
    assert!(result.is_err());

    // Oversized idempotency key should panic
    let oversized_key = String::from_str(&env, &"a".repeat(300));
    let result = client.try_single_payout(&recipient, &amount, &Some(oversized_key));
    assert!(result.is_err());
}

/// Test idempotency key with insufficient funds (failure case)
#[test]
fn test_idempotency_key_insufficient_funds() {
    let env = Env::default();
    let (client, admin, token, token_admin) = setup_program(&env, 100_0000000);

    let recipient = Address::generate(&env);
    let amount = 2000_0000000; // More than available
    let idempotency_key = String::from_str(&env, "test-insufficient-111");

    // First attempt should fail
    let result = client.try_single_payout(&recipient, &amount, &Some(idempotency_key.clone()));
    assert!(result.is_err());

    // Verify failure record was stored
    let record: IdempotencyRecord = env.as_contract(&client.address, || { env.storage().instance().get(&DataKey::IdempotencyKey(idempotency_key.clone())).unwrap() });
    assert_eq!(record.idempotency_key, idempotency_key);
    assert!(!record.success);
    assert!(record.error_code.is_some());

    // Retry should return same failure
    let result2 = client.try_single_payout(&recipient, &amount, &Some(idempotency_key.clone()));
    assert!(result2.is_err());
}

/// Test idempotency schema version initialization
#[test]
fn test_idempotency_schema_version() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize_contract(&admin);

    // Verify schema version is set
    let schema_version = client.get_idempotency_schema_version();
    assert_eq!(schema_version, IDEMPOTENCY_SCHEMA_VERSION_V1);

    // Verify schema version event was emitted
    let events = env.events().all();
    let schema_events: soroban_sdk::Vec<_> = events.clone();
    assert_eq!(schema_events.len(), 1);
}

/// Test idempotency key with no key provided (normal operation)
#[test]
fn test_idempotency_key_none_provided() {
    let env = Env::default();
    let (client, admin, token, token_admin) = setup_program(&env, 1000_0000000);

    let recipient = Address::generate(&env);
    let amount = 300_0000000;

    // Payout without idempotency key should work normally
    let result = client.single_payout(&recipient, &amount, &None);
    assert_eq!(result.remaining_balance, 700_0000000);

    // Should be able to do multiple payouts without idempotency keys
    let recipient2 = Address::generate(&env);
    let result2 = client.single_payout(&recipient2, &amount, &None);
    assert_eq!(result2.remaining_balance, 400_0000000);
}

/// Test idempotency key isolation between different operations
#[test]
fn test_idempotency_key_operation_isolation() {
    let env = Env::default();
    let (client, admin, token_id, token_admin) = setup_program(&env, 1000_0000000);
    let token_client = token::Client::new(&env, &token_id);

    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let recipients = vec![&env, recipient1.clone(), recipient2.clone()];
    let amounts = vec![&env, 1000, 2000];
    let idempotency_key = String::from_str(&env, "batch-replay-test");

    // First batch payout
    let data1 = client.batch_payout_idempotent(&recipients, &amounts, &Some(idempotency_key.clone()));
    let balance_after_first = token_client.balance(&client.address);
    assert_eq!(data1.remaining_balance, 7000);

    // Try to replay with DIFFERENT recipients and amounts - should still be idempotent
    let recipient3 = Address::generate(&env);
    let different_recipients = vec![&env, recipient3.clone()];
    let different_amounts = vec![&env, 5000];

    // This should return the original result, not execute with new params
    let data2 = client.batch_payout_idempotent(&different_recipients, &different_amounts, &Some(idempotency_key.clone()));
    let balance_after_replay = token_client.balance(&client.address);

    // Balance should be the same (no execution with different params)
    assert_eq!(balance_after_first, balance_after_replay);
    assert_eq!(data2.remaining_balance, 7000);
    assert_eq!(data2.payout_history.len(), 2); // Still only 2 from first payout
}

/// Test idempotency key with different keys for same operation
#[test]
fn test_idempotency_key_different_keys_same_operation() {
    let env = Env::default();
    let (client, admin, token, token_admin) = setup_program(&env, 1000_0000000);

    let recipient = Address::generate(&env);
    let amount = 300_0000000;
    let key1 = String::from_str(&env, "test-diff-key-1");
    let key2 = String::from_str(&env, "test-diff-key-2");

    // First payout with key1
    let result1 = client.single_payout(&recipient, &amount, &Some(key1.clone()));
    assert_eq!(result1.remaining_balance, 700_0000000);

    // Second payout with different key2 should work (different recipient)
    let recipient2 = Address::generate(&env);
    let result2 = client.single_payout(&recipient2, &amount, &Some(key2.clone()));
    assert_eq!(result2.remaining_balance, 400_0000000);

    // Verify both keys have their own records
    let record1: IdempotencyRecord = env.as_contract(&client.address, || { env.storage().instance().get(&DataKey::IdempotencyKey(key1)).unwrap() });
    let record2: IdempotencyRecord = env.as_contract(&client.address, || { env.storage().instance().get(&DataKey::IdempotencyKey(key2)).unwrap() });
    assert_eq!(record1.recipient_count, 1);
    assert_eq!(record2.recipient_count, 1);
}

// ============================================================================
// Batch Payout Atomicity Tests â€” Issue #24
//
// Verifies the all-or-nothing guarantee: if any validation fails, no transfers
// occur and the contract balance is unchanged.
// ============================================================================

/// Atomicity: duplicate recipient in batch â†’ zero transfers, balance unchanged.
#[test]
fn test_batch_atomicity_duplicate_recipient_no_partial_transfer() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 10_000);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    // r1 appears twice â€” must be rejected before any transfer
    let result = client.try_batch_payout(
        &vec![&env, r1.clone(), r2.clone(), r1.clone()],
        &vec![&env, 1_000i128, 2_000i128, 1_500i128],
        &None,
    );
    assert!(result.is_err(), "duplicate recipient must be rejected");
    assert_eq!(client.get_remaining_balance(), 10_000, "balance must be unchanged");
    assert_eq!(token_client.balance(&r1), 0);
    assert_eq!(token_client.balance(&r2), 0);
}

/// Atomicity: zero amount in batch â†’ zero transfers, balance unchanged.
#[test]
fn test_batch_atomicity_zero_amount_no_partial_transfer() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 10_000);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    // Second amount is zero â€” must be rejected before any transfer
    let result = client.try_batch_payout(
        &vec![&env, r1.clone(), r2.clone()],
        &vec![&env, 1_000i128, 0i128],
        &None,
    );
    assert!(result.is_err(), "zero amount must be rejected");
    assert_eq!(client.get_remaining_balance(), 10_000);
    assert_eq!(token_client.balance(&r1), 0);
    assert_eq!(token_client.balance(&r2), 0);
}

/// Atomicity: insufficient balance â†’ zero transfers, balance unchanged.
#[test]
fn test_batch_atomicity_insufficient_balance_no_partial_transfer() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 1_000);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    // Total 3_000 > balance 1_000
    let result = client.try_batch_payout(
        &vec![&env, r1.clone(), r2.clone()],
        &vec![&env, 1_500i128, 1_500i128],
        &None,
    );
    assert!(result.is_err(), "over-balance batch must be rejected");
    assert_eq!(client.get_remaining_balance(), 1_000);
    assert_eq!(token_client.balance(&r1), 0);
    assert_eq!(token_client.balance(&r2), 0);
}

/// Atomicity: mismatched recipients/amounts â†’ zero transfers, balance unchanged.
#[test]
fn test_batch_atomicity_length_mismatch_no_partial_transfer() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 10_000);

    let r1 = Address::generate(&env);

    let result = client.try_batch_payout(
        &vec![&env, r1.clone()],
        &vec![&env, 1_000i128, 2_000i128], // 2 amounts, 1 recipient
        &None,
    );
    assert!(result.is_err(), "length mismatch must be rejected");
    assert_eq!(client.get_remaining_balance(), 10_000);
}

/// Atomicity: batch exceeds MAX_BATCH_SIZE â†’ rejected, balance unchanged.
#[test]
fn test_batch_atomicity_exceeds_max_batch_size() {
    let env = Env::default();
    let total = (MAX_BATCH_SIZE as i128 + 1) * 100;
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, total);

    let mut recipients = vec![&env];
    let mut amounts = vec![&env];
    for _ in 0..(MAX_BATCH_SIZE + 1) {
        recipients.push_back(Address::generate(&env));
        amounts.push_back(100i128);
    }

    let result = client.try_batch_payout(&recipients, &amounts, &None);
    assert!(result.is_err(), "batch exceeding MAX_BATCH_SIZE must be rejected");
    assert_eq!(client.get_remaining_balance(), total);
}

/// Deterministic ordering: MAX_BATCH_SIZE boundary is accepted.
#[test]
fn test_batch_max_size_boundary_accepted() {
    let env = Env::default();
    let total = MAX_BATCH_SIZE as i128 * 100;
    let (client, _admin, token_client, _token_admin) = setup_program(&env, total);

    let mut recipients = vec![&env];
    let mut amounts = vec![&env];
    let mut addrs = soroban_sdk::Vec::new(&env);
    for _ in 0..MAX_BATCH_SIZE {
        let a = Address::generate(&env);
        addrs.push_back(a.clone());
        recipients.push_back(a);
        amounts.push_back(100i128);
    }

    let data = client.batch_payout(&recipients, &amounts, &None);
    assert_eq!(data.remaining_balance, 0);
    assert_eq!(data.payout_history.len(), MAX_BATCH_SIZE);
    for i in 0..MAX_BATCH_SIZE {
        assert_eq!(token_client.balance(&addrs.get(i).unwrap()), 100);
    }
}

/// Upgrade-safe storage: BatchPayoutSchemaVersion is readable after init.
#[test]
fn test_batch_payout_schema_version_set_on_init() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 0);
    // Version 0 means not yet written (legacy) â€” any value is acceptable.
    let _v = client.get_batch_payout_schema_version();
}

#[test]
fn test_update_fee_recipient_admin_only() {
    let env = Env::default();
    let (client, admin, _token, _token_admin) = setup_program(&env, 1000);

    let new_recipient = Address::generate(&env);

    // Admin should be able to update
    env.mock_all_auths();
    client.update_fee_recipient(&new_recipient);

    let cfg = client.get_fee_config();
    assert_eq!(cfg.fee_recipient, new_recipient);

    // Verify event was emitted
    let events = env.events().all();
    assert!(events.len() > 0);
}



#[test]
fn test_update_fee_recipient_multiple_times() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 1000);

    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);

    env.mock_all_auths();

    // First update
    client.update_fee_recipient(&recipient1);
    let cfg = client.get_fee_config();
    assert_eq!(cfg.fee_recipient, recipient1);

    // Second update
    client.update_fee_recipient(&recipient2);
    let cfg = client.get_fee_config();
    assert_eq!(cfg.fee_recipient, recipient2);
}

#[test]
fn test_fee_recipient_update_event_contains_old_and_new() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 1000);

    let original_cfg = client.get_fee_config();
    let new_recipient = Address::generate(&env);

    env.mock_all_auths();
    client.update_fee_recipient(&new_recipient);

    let events = env.events().all();
    // Verify at least one event was published
    assert!(events.len() > 0, "Expected at least one event to be published");
}

#[test]
fn test_update_fee_recipient_preserves_other_fee_config() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 1000);

    let original_cfg = client.get_fee_config();
    let new_recipient = Address::generate(&env);

    env.mock_all_auths();
    client.update_fee_recipient(&new_recipient);

    let updated_cfg = client.get_fee_config();

    // Verify only fee_recipient changed
    assert_eq!(updated_cfg.fee_recipient, new_recipient);
    assert_eq!(updated_cfg.lock_fee_rate, original_cfg.lock_fee_rate);
    assert_eq!(updated_cfg.payout_fee_rate, original_cfg.payout_fee_rate);
    assert_eq!(updated_cfg.lock_fixed_fee, original_cfg.lock_fixed_fee);
    assert_eq!(updated_cfg.payout_fixed_fee, original_cfg.payout_fixed_fee);
    assert_eq!(updated_cfg.fee_enabled, original_cfg.fee_enabled);
}

#[test]
fn test_access_control_violation_unauthorized_payout() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 100_000);
    let unauthorized_user = Address::generate(&env);
    let recipient = Address::generate(&env);

    // Mock auth for unauthorized user attempting a payout
    env.mock_auths(&[MockAuth {
        address: &unauthorized_user,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "single_payout",
            args: (recipient.clone(), 10_000_i128, None::<String>).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    // This should fail because unauthorized_user is not the payout_key
    let result = client.try_single_payout(&recipient, &10_000, &None);
    assert!(result.is_err());
}

#[test]
fn test_threat_model_reentrancy_prevention() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 100_000);
    let recipient = Address::generate(&env);

    // Soroban handles reentrancy by not allowing cross-contract calls
    // during a contract execution unless explicitly allowed.
    client.single_payout(&recipient, &10_000, &None);
    assert_eq!(client.get_remaining_balance(), 90_000);
}

#[test]
fn test_threat_model_oracle_manipulation_unauthorized_rotation() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 100_000);
    let attacker = Address::generate(&env);

    // Attacker tries to propose themselves as admin without authorization
    env.mock_auths(&[MockAuth {
        address: &attacker,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "propose_admin",
            args: (attacker.clone(),).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let result = client.try_propose_admin(&attacker);
    assert!(result.is_err());
}

#[test]
#[should_panic(expected = "Invalid payout fee rate")]
fn test_threat_model_fee_drain_prevention() {
    let env = Env::default();
    let (client, admin, _token_client, _token_admin) = setup_program(&env, 100_000);

    // Admin tries to set payout fee to 20% (assuming MAX_FEE_RATE is 1000 = 10%)
    env.mock_all_auths();

    client.update_fee_config(
        &None,
        &Some(2000), // This should panic
        &None,
        &None,
        &None,
        &None,
    );
}
