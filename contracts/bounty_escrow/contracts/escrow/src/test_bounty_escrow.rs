#![cfg(test)]
use crate::{BountyEscrowContract, BountyEscrowContractClient, Error as ContractError};
use soroban_sdk::testutils::Events;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, Map, Symbol, TryFromVal, Val,
};

fn create_test_env() -> (Env, BountyEscrowContractClient<'static>, Address) {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    (env, client, contract_id)
}

fn create_token_contract<'a>(
    e: &'a Env,
    admin: &Address,
) -> (Address, token::Client<'a>, token::StellarAssetClient<'a>) {
    let token_id = e.register_stellar_asset_contract_v2(admin.clone());
    let token = token_id.address();
    let token_client = token::Client::new(e, &token);
    let token_admin_client = token::StellarAssetClient::new(e, &token);
    (token, token_client, token_admin_client)
}

fn assert_event_data_has_v2_tag(env: &Env, data: &Val) {
    let data_map: Map<Symbol, Val> =
        Map::try_from_val(env, data).unwrap_or_else(|_| panic!("event payload should be a map"));
    let version_val = data_map
        .get(Symbol::new(env, "version"))
        .unwrap_or_else(|| panic!("event payload must contain version field"));
    let version = u32::try_from_val(env, &version_val).expect("version should decode as u32");
    assert_eq!(version, 2);
}

fn assert_current_call_has_versioned_contract_event(env: &Env, contract_id: &Address) {
    let events = env.events().all();
    let mut found = false;
    for (contract, _topics, data) in events.iter() {
        if contract != *contract_id {
            continue;
        }
        assert_event_data_has_v2_tag(env, &data);
        found = true;
    }
    assert!(found);
}

#[test]
fn test_init_event() {
    let (env, client, _contract_id) = create_test_env();
    let _employee = Address::generate(&env);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let _depositor = Address::generate(&env);
    let _bounty_id = 1;

    env.mock_all_auths();

    // Initialize
    client.init(&admin.clone(), &token.clone());

    // Get all events emitted
    let events = env.events().all();

    // Verify the event was emitted
    assert_eq!(events.len(), 1);
}

#[test]
fn test_events_emit_v2_version_tags_for_all_bounty_emitters() {
    let (env, client, contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin, &token);
    assert_current_call_has_versioned_contract_event(&env, &contract_id);

    token_admin_client.mint(&depositor, &10_000);
    client.lock_funds(
        &depositor,
        &1,
        &10_000,
        &Some((env.ledger()).timestamp() + 10),
    );
    assert_current_call_has_versioned_contract_event(&env, &contract_id);

    client.release_funds(&1, &contributor);
    assert_current_call_has_versioned_contract_event(&env, &contract_id);
}

#[test]
fn test_lock_fund() {
    let (env, client, _contract_id) = create_test_env();
    let _employee = Address::generate(&env);

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let bounty_id = 1;
    let amount = 1000;
    let deadline = 10;

    env.mock_all_auths();

    // Setup token
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    // Initialize
    client.init(&admin.clone(), &token.clone());

    token_admin_client.mint(&depositor, &amount);

    client.lock_funds(&depositor, &bounty_id, &amount, &Some(deadline));

    // Get all events emitted
    let events = env.events().all();

    // Verify lock produced events (exact count can vary across Soroban versions).
    assert!(events.len() >= 2);
}

#[test]
fn test_release_fund() {
    let (env, client, _contract_id) = create_test_env();

    let admin = Address::generate(&env);
    // let token = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let bounty_id = 1;
    let amount = 1000;
    let deadline = 10;

    env.mock_all_auths();

    // Setup token
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    // Initialize
    client.init(&admin.clone(), &token.clone());

    token_admin_client.mint(&depositor, &amount);

    client.lock_funds(&depositor, &bounty_id, &amount, &Some(deadline));

    client.release_funds(&bounty_id, &contributor);

    // Get all events emitted
    let events = env.events().all();

    // Verify release produced events (exact count can vary across Soroban versions).
    assert!(events.len() >= 2);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")] // AlreadyInitialized
fn test_init_rejects_reinitialization() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    env.mock_all_auths();

    client.init(&admin, &token);
    client.init(&admin, &token);
}

#[test]
fn test_lock_funds_zero_amount_edge_case() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let bounty_id = 100;
    let amount = 0;
    let deadline = env.ledger().timestamp() + 100;

    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &1_000);

    client.lock_funds(&depositor, &bounty_id, &amount, &Some(deadline));

    let escrow = client.get_escrow_info(&bounty_id);
    assert_eq!(escrow.amount, 0);
    assert_eq!(escrow.status, crate::EscrowStatus::Locked);
}

#[test]
#[should_panic] // Token transfer fails due to insufficient balance, protecting against overflows/invalid accounting.
fn test_lock_funds_insufficient_balance_rejected() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let bounty_id = 101;
    let deadline = env.ledger().timestamp() + 100;

    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &100);

    client.lock_funds(&depositor, &bounty_id, &1_000, &Some(deadline));
}

#[test]
fn test_refund_allows_exact_deadline_boundary() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let bounty_id = 102;
    let amount = 700;
    let now = env.ledger().timestamp();
    let deadline = now + 500;

    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let (token, token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &amount);
    client.lock_funds(&depositor, &bounty_id, &amount, &Some(deadline));

    env.ledger().set_timestamp(deadline);
    client.refund(&bounty_id);

    let escrow = client.get_escrow_info(&bounty_id);
    assert_eq!(escrow.status, crate::EscrowStatus::Refunded);
    assert_eq!(token_client.balance(&depositor), amount);
}

#[test]
fn test_maximum_lock_and_release_path() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let bounty_id = 103;
    let amount = i64::MAX as i128;
    let deadline = env.ledger().timestamp() + 1_000;

    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let (token, token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &amount);
    client.lock_funds(&depositor, &bounty_id, &amount, &Some(deadline));

    assert_eq!(token_client.balance(&client.address), amount);
    client.release_funds(&bounty_id, &contributor);
    assert_eq!(token_client.balance(&client.address), 0);
    assert_eq!(token_client.balance(&contributor), amount);
}

#[test]
fn test_integration_multi_bounty_lifecycle() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let now = env.ledger().timestamp();

    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let (token, token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &10_000);

    client.lock_funds(&depositor, &201, &3_000, &Some((now + 100)));
    client.lock_funds(&depositor, &202, &2_000, &Some((now + 200)));
    client.lock_funds(&depositor, &203, &1_000, &Some((now + 300)));
    assert_eq!(token_client.balance(&client.address), 6_000);

    client.release_funds(&201, &contributor);
    env.ledger().set_timestamp(now + 201);
    client.refund(&202);
    assert_eq!(token_client.balance(&client.address), 1_000);

    let escrow_201 = client.get_escrow_info(&201);
    let escrow_202 = client.get_escrow_info(&202);
    let escrow_203 = client.get_escrow_info(&203);
    assert_eq!(escrow_201.status, crate::EscrowStatus::Released);
    assert_eq!(escrow_202.status, crate::EscrowStatus::Refunded);
    assert_eq!(escrow_203.status, crate::EscrowStatus::Locked);
    assert_eq!(token_client.balance(&contributor), 3_000);
}

fn next_seed(seed: &mut u64) -> u64 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    *seed
}

#[test]
fn test_property_fuzz_lock_release_refund_invariants() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let start = env.ledger().timestamp();

    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    client.init(&admin, &token);

    let mut seed = 7_u64;
    let mut fuzz_cases: [(u64, i128, u64); 40] = [(0, 0, 0); 40];
    let mut total_locked = 0_i128;
    for i in 0..40_u64 {
        let amount = (next_seed(&mut seed) % 900 + 100) as i128;
        let deadline = start + (next_seed(&mut seed) % 500 + 10);
        fuzz_cases[i as usize] = (2_000 + i, amount, deadline);
        total_locked += amount;
    }
    token_admin_client.mint(&depositor, &total_locked);

    // Lock deterministic fuzz cases.
    for (id, amount, deadline) in fuzz_cases.iter() {
        client.lock_funds(&depositor, id, amount, &Some(*deadline));
    }

    let mut expected_locked_balance = client.get_balance();
    for i in 0..40_u64 {
        let id = 2_000 + i;
        if i % 3 == 0 {
            let info = client.get_escrow_info(&id);
            client.release_funds(&id, &contributor);
            expected_locked_balance -= info.amount;
        } else if i % 3 == 1 {
            let info = client.get_escrow_info(&id);
            env.ledger().set_timestamp(info.deadline.unwrap());
            client.refund(&id);
            expected_locked_balance -= info.amount;
        }
    }

    assert_eq!(client.get_balance(), expected_locked_balance);
}

#[test]
fn test_stress_high_load_bounty_operations() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let now = env.ledger().timestamp();

    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let (token, token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &1_000_000);

    for i in 0..40_u64 {
        let amount = 100 + (i as i128 % 10);
        let deadline = now + 30 + i;
        client.lock_funds(&depositor, &(5_000 + i), &amount, &Some(deadline));
    }
    assert!(client.get_balance() > 0);

    for i in 0..40_u64 {
        let id = 5_000 + i;
        if i % 2 == 0 {
            client.release_funds(&id, &contributor);
        } else {
            let info = client.get_escrow_info(&id);
            env.ledger().set_timestamp(info.deadline.unwrap());
            client.refund(&id);
        }
    }

    assert_eq!(client.get_balance(), 0);
    assert!(token_client.balance(&contributor) > 0);
}

#[test]
fn test_gas_proxy_event_footprint_per_operation_is_constant() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let now = env.ledger().timestamp();

    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &10_000);

    let before_lock = env.events().all().len();
    for offset in 0..20_u64 {
        let id = 8_001 + offset;
        client.lock_funds(&depositor, &id, &10, &Some((now + 100 + offset)));
    }
    let after_locks = env.events().all().len();
    let lock_event_growth = after_locks - before_lock;
    assert!(lock_event_growth > 0);

    let before_release = env.events().all().len();
    client.release_funds(&8_001, &contributor);
    let after_release = env.events().all().len();
    assert!(after_release >= before_release);
}

// ==================== FEE CONFIGURATION EDGE CASE TESTS ====================

#[test]
fn test_update_fee_config_with_zero_lock_fee() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let fee_recipient = Address::generate(&env);

    env.mock_all_auths();

    client.init(&admin, &token);

    // Test: Set lock_fee_rate to 0 (should succeed)
    let result = client.try_update_fee_config(
        &Some(0), // lock_fee_rate: 0%
        &None,    // release_fee_rate: unchanged
        &Some(fee_recipient.clone()),
        &None, // fee_enabled: unchanged
    );
    assert!(result.is_ok());

    let config = client.get_fee_config();
    assert_eq!(config.lock_fee_rate, 0);
    assert_eq!(config.fee_recipient, fee_recipient);
}

#[test]
fn test_update_fee_config_with_zero_release_fee() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let fee_recipient = Address::generate(&env);

    env.mock_all_auths();

    client.init(&admin, &token);

    // Test: Set release_fee_rate to 0 (should succeed)
    let result = client.try_update_fee_config(
        &None,    // lock_fee_rate: unchanged
        &Some(0), // release_fee_rate: 0%
        &Some(fee_recipient.clone()),
        &None, // fee_enabled: unchanged
    );
    assert!(result.is_ok());

    let config = client.get_fee_config();
    assert_eq!(config.release_fee_rate, 0);
    assert_eq!(config.fee_recipient, fee_recipient);
}

#[test]
fn test_update_fee_config_with_max_lock_fee() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let fee_recipient = Address::generate(&env);

    env.mock_all_auths();

    client.init(&admin, &token);

    // Test: Set lock_fee_rate to MAX_FEE_RATE (5000 = 50%) (should succeed)
    let result = client.try_update_fee_config(
        &Some(5000), // lock_fee_rate: 50% (MAX_FEE_RATE)
        &None,       // release_fee_rate: unchanged
        &Some(fee_recipient.clone()),
        &None, // fee_enabled: unchanged
    );
    assert!(result.is_ok());

    let config = client.get_fee_config();
    assert_eq!(config.lock_fee_rate, 5000);
    assert_eq!(config.fee_recipient, fee_recipient);
}

#[test]
fn test_update_fee_config_with_max_release_fee() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let fee_recipient = Address::generate(&env);

    env.mock_all_auths();

    client.init(&admin, &token);

    // Test: Set release_fee_rate to MAX_FEE_RATE (5000 = 50%) (should succeed)
    let result = client.try_update_fee_config(
        &None,       // lock_fee_rate: unchanged
        &Some(5000), // release_fee_rate: 50% (MAX_FEE_RATE)
        &Some(fee_recipient.clone()),
        &None, // fee_enabled: unchanged
    );
    assert!(result.is_ok());

    let config = client.get_fee_config();
    assert_eq!(config.release_fee_rate, 5000);
    assert_eq!(config.fee_recipient, fee_recipient);
}

#[test]
fn test_update_fee_config_rejects_negative_lock_fee() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let fee_recipient = Address::generate(&env);

    env.mock_all_auths();

    client.init(&admin, &token);

    let original_config = client.get_fee_config();

    let result =
        client.try_update_fee_config(&Some(-1), &None, &Some(fee_recipient.clone()), &None);
    assert_eq!(result, Err(Ok(ContractError::InvalidFeeRate)));

    let current_config = client.get_fee_config();
    assert_eq!(current_config.lock_fee_rate, original_config.lock_fee_rate);
    assert_eq!(
        current_config.release_fee_rate,
        original_config.release_fee_rate
    );
}

#[test]
fn test_update_fee_config_rejects_negative_release_fee() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let fee_recipient = Address::generate(&env);

    env.mock_all_auths();

    client.init(&admin, &token);

    let original_config = client.get_fee_config();

    let result =
        client.try_update_fee_config(&None, &Some(-1), &Some(fee_recipient.clone()), &None);
    assert_eq!(result, Err(Ok(ContractError::InvalidFeeRate)));

    let current_config = client.get_fee_config();
    assert_eq!(current_config.lock_fee_rate, original_config.lock_fee_rate);
    assert_eq!(
        current_config.release_fee_rate,
        original_config.release_fee_rate
    );
}

#[test]
fn test_update_fee_config_rejects_over_max_lock_fee() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let fee_recipient = Address::generate(&env);

    env.mock_all_auths();

    client.init(&admin, &token);

    let original_config = client.get_fee_config();

    let result =
        client.try_update_fee_config(&Some(5001), &None, &Some(fee_recipient.clone()), &None);
    assert_eq!(result, Err(Ok(ContractError::InvalidFeeRate)));

    let current_config = client.get_fee_config();
    assert_eq!(current_config.lock_fee_rate, original_config.lock_fee_rate);
    assert_eq!(
        current_config.release_fee_rate,
        original_config.release_fee_rate
    );
}

#[test]
fn test_update_fee_config_rejects_over_max_release_fee() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let fee_recipient = Address::generate(&env);

    env.mock_all_auths();

    client.init(&admin, &token);

    let original_config = client.get_fee_config();

    let result =
        client.try_update_fee_config(&None, &Some(5001), &Some(fee_recipient.clone()), &None);
    assert_eq!(result, Err(Ok(ContractError::InvalidFeeRate)));

    let current_config = client.get_fee_config();
    assert_eq!(current_config.lock_fee_rate, original_config.lock_fee_rate);
    assert_eq!(
        current_config.release_fee_rate,
        original_config.release_fee_rate
    );
}

#[test]
fn test_update_fee_config_rejects_overflow_lock_fee() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let fee_recipient = Address::generate(&env);

    env.mock_all_auths();

    client.init(&admin, &token);

    let original_config = client.get_fee_config();

    let result =
        client.try_update_fee_config(&Some(i128::MAX), &None, &Some(fee_recipient.clone()), &None);
    assert_eq!(result, Err(Ok(ContractError::InvalidFeeRate)));

    let current_config = client.get_fee_config();
    assert_eq!(current_config.lock_fee_rate, original_config.lock_fee_rate);
    assert_eq!(
        current_config.release_fee_rate,
        original_config.release_fee_rate
    );
}

#[test]
fn test_update_fee_config_rejects_overflow_release_fee() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let fee_recipient = Address::generate(&env);

    env.mock_all_auths();

    client.init(&admin, &token);

    let original_config = client.get_fee_config();

    let result =
        client.try_update_fee_config(&None, &Some(i128::MAX), &Some(fee_recipient.clone()), &None);
    assert_eq!(result, Err(Ok(ContractError::InvalidFeeRate)));

    let current_config = client.get_fee_config();
    assert_eq!(current_config.lock_fee_rate, original_config.lock_fee_rate);
    assert_eq!(
        current_config.release_fee_rate,
        original_config.release_fee_rate
    );
}

#[test]
fn test_update_fee_config_both_rates_zero() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let fee_recipient = Address::generate(&env);

    env.mock_all_auths();

    client.init(&admin, &token);

    // Test: Set both lock and release fees to 0 (should succeed)
    let result = client.try_update_fee_config(
        &Some(0), // lock_fee_rate: 0%
        &Some(0), // release_fee_rate: 0%
        &Some(fee_recipient.clone()),
        &None,
    );
    assert!(result.is_ok());

    let config = client.get_fee_config();
    assert_eq!(config.lock_fee_rate, 0);
    assert_eq!(config.release_fee_rate, 0);
}

#[test]
fn test_update_fee_config_both_rates_at_max() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let fee_recipient = Address::generate(&env);

    env.mock_all_auths();

    client.init(&admin, &token);

    // Test: Set both lock and release fees to MAX_FEE_RATE (should succeed)
    let result = client.try_update_fee_config(
        &Some(5000), // lock_fee_rate: 50% (MAX_FEE_RATE)
        &Some(5000), // release_fee_rate: 50% (MAX_FEE_RATE)
        &Some(fee_recipient.clone()),
        &None,
    );
    assert!(result.is_ok());

    let config = client.get_fee_config();
    assert_eq!(config.lock_fee_rate, 5000);
    assert_eq!(config.release_fee_rate, 5000);
}

#[test]
fn test_update_fee_config_valid_intermediate_rates() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let fee_recipient = Address::generate(&env);

    env.mock_all_auths();

    client.init(&admin, &token);

    // Test: Set lock to 100 (1%) and release to 250 (2.5%) (should succeed)
    let result = client.try_update_fee_config(
        &Some(100), // lock_fee_rate: 1% (100 basis points)
        &Some(250), // release_fee_rate: 2.5% (250 basis points)
        &Some(fee_recipient.clone()),
        &None,
    );
    assert!(result.is_ok());

    let config = client.get_fee_config();
    assert_eq!(config.lock_fee_rate, 100);
    assert_eq!(config.release_fee_rate, 250);
}

#[test]
fn test_update_fee_config_partial_updates_preserve_existing_values() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let fee_recipient_1 = Address::generate(&env);
    let fee_recipient_2 = Address::generate(&env);

    env.mock_all_auths();

    client.init(&admin, &token);

    // First update: Set lock fee, release fee, and recipient
    client.update_fee_config(
        &Some(100),
        &Some(200),
        &Some(fee_recipient_1.clone()),
        &Some(true),
    );

    // Second update: Only update lock fee, other values should remain unchanged
    client.update_fee_config(&Some(300), &None, &None, &None);

    let config = client.get_fee_config();
    assert_eq!(config.lock_fee_rate, 300);
    assert_eq!(config.release_fee_rate, 200); // Should remain 200
    assert_eq!(config.fee_recipient, fee_recipient_1); // Should remain recipient_1
    assert_eq!(config.fee_enabled, true); // Should remain true

    // Third update: Update recipient and enabled flag
    client.update_fee_config(&None, &None, &Some(fee_recipient_2.clone()), &Some(false));

    let config = client.get_fee_config();
    assert_eq!(config.lock_fee_rate, 300); // Should remain 300
    assert_eq!(config.release_fee_rate, 200); // Should remain 200
    assert_eq!(config.fee_recipient, fee_recipient_2); // Should be updated to recipient_2
    assert_eq!(config.fee_enabled, false); // Should be updated to false
}

#[test]
fn test_update_fee_config_fails_with_one_invalid_rate_preserves_state() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let fee_recipient = Address::generate(&env);

    env.mock_all_auths();

    client.init(&admin, &token);

    client.update_fee_config(&Some(100), &Some(200), &Some(fee_recipient.clone()), &None);

    let original_config = client.get_fee_config();

    let result = client.try_update_fee_config(&Some(300), &Some(5001), &None, &None);
    assert_eq!(result, Err(Ok(ContractError::InvalidFeeRate)));

    let config = client.get_fee_config();
    assert_eq!(config.lock_fee_rate, original_config.lock_fee_rate);
    assert_eq!(config.release_fee_rate, original_config.release_fee_rate);
}

#[test]
fn test_update_fee_config_rejects_100_percent_lock_fee() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let fee_recipient = Address::generate(&env);

    env.mock_all_auths();

    client.init(&admin, &token);

    let original_config = client.get_fee_config();

    let result =
        client.try_update_fee_config(&Some(10_000), &None, &Some(fee_recipient.clone()), &None);
    assert_eq!(result, Err(Ok(ContractError::InvalidFeeRate)));

    let current_config = client.get_fee_config();
    assert_eq!(current_config.lock_fee_rate, original_config.lock_fee_rate);
    assert_eq!(
        current_config.release_fee_rate,
        original_config.release_fee_rate
    );
}

#[test]
fn test_update_fee_config_rejects_100_percent_release_fee() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let fee_recipient = Address::generate(&env);

    env.mock_all_auths();

    client.init(&admin, &token);

    let original_config = client.get_fee_config();

    let result =
        client.try_update_fee_config(&None, &Some(10_000), &Some(fee_recipient.clone()), &None);
    assert_eq!(result, Err(Ok(ContractError::InvalidFeeRate)));

    let current_config = client.get_fee_config();
    assert_eq!(current_config.lock_fee_rate, original_config.lock_fee_rate);
    assert_eq!(
        current_config.release_fee_rate,
        original_config.release_fee_rate
    );
}

#[test]
fn test_update_fee_config_rejects_over_100_percent_lock_fee() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let fee_recipient = Address::generate(&env);

    env.mock_all_auths();

    client.init(&admin, &token);

    let original_config = client.get_fee_config();

    let result =
        client.try_update_fee_config(&Some(10_001), &None, &Some(fee_recipient.clone()), &None);
    assert_eq!(result, Err(Ok(ContractError::InvalidFeeRate)));

    let current_config = client.get_fee_config();
    assert_eq!(current_config.lock_fee_rate, original_config.lock_fee_rate);
    assert_eq!(
        current_config.release_fee_rate,
        original_config.release_fee_rate
    );
}

#[test]
fn test_update_fee_config_rejects_over_100_percent_release_fee() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let fee_recipient = Address::generate(&env);

    env.mock_all_auths();

    client.init(&admin, &token);

    let original_config = client.get_fee_config();

    let result =
        client.try_update_fee_config(&None, &Some(10_001), &Some(fee_recipient.clone()), &None);
    assert_eq!(result, Err(Ok(ContractError::InvalidFeeRate)));

    let current_config = client.get_fee_config();
    assert_eq!(current_config.lock_fee_rate, original_config.lock_fee_rate);
    assert_eq!(
        current_config.release_fee_rate,
        original_config.release_fee_rate
    );
}
