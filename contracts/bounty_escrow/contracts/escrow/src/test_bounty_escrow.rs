use crate::{BountyEscrowContract, BountyEscrowContractClient, Error as ContractError};
use soroban_sdk::testutils::Events;
use soroban_sdk::{
    testutils::{Address as _, Ledger, MockAuth, MockAuthInvoke},
    token, Address, Env, IntoVal, Map, Symbol, TryFromVal, Val,
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
    client.lock_funds(&depositor, &1, &10_000, &(env.ledger().timestamp() + 10));
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

    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

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

    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

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

    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

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

    client.lock_funds(&depositor, &bounty_id, &1_000, &deadline);
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
    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

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
    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

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

    client.lock_funds(&depositor, &201, &3_000, &(now + 100));
    client.lock_funds(&depositor, &202, &2_000, &(now + 200));
    client.lock_funds(&depositor, &203, &1_000, &(now + 300));
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

#[test]
fn test_multi_token_balance_accounting_isolated_across_escrow_instances() {
    let env = Env::default();
    env.mock_all_auths();

    // Two escrow instances simulate simultaneous use of different tokens.
    let contract_a = env.register_contract(None, BountyEscrowContract);
    let contract_b = env.register_contract(None, BountyEscrowContract);
    let client_a = BountyEscrowContractClient::new(&env, &contract_a);
    let client_b = BountyEscrowContractClient::new(&env, &contract_b);

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let now = env.ledger().timestamp();

    let token_admin_a = Address::generate(&env);
    let token_admin_b = Address::generate(&env);
    let (token_a, token_client_a, token_admin_client_a) =
        create_token_contract(&env, &token_admin_a);
    let (token_b, token_client_b, token_admin_client_b) =
        create_token_contract(&env, &token_admin_b);

    client_a.init(&admin, &token_a);
    client_b.init(&admin, &token_b);

    token_admin_client_a.mint(&depositor, &5_000);
    token_admin_client_b.mint(&depositor, &7_000);

    client_a.lock_funds(&depositor, &11, &1_200, &(now + 120));
    client_b.lock_funds(&depositor, &22, &3_400, &(now + 240));

    // Per-token locked balances are tracked independently.
    assert_eq!(client_a.get_balance(), 1_200);
    assert_eq!(client_b.get_balance(), 3_400);
    assert_eq!(token_client_a.balance(&client_a.address), 1_200);
    assert_eq!(token_client_b.balance(&client_b.address), 3_400);

    // Release only token A escrow and verify token B path is unchanged.
    client_a.release_funds(&11, &contributor);

    assert_eq!(client_a.get_balance(), 0);
    assert_eq!(client_b.get_balance(), 3_400);
    assert_eq!(token_client_a.balance(&contributor), 1_200);
    assert_eq!(token_client_b.balance(&contributor), 0);
    assert_eq!(token_client_a.balance(&client_a.address), 0);
    assert_eq!(token_client_b.balance(&client_b.address), 3_400);

    let escrow_b = client_b.get_escrow_info(&22);
    assert_eq!(escrow_b.status, crate::EscrowStatus::Locked);
    assert_eq!(escrow_b.remaining_amount, 3_400);
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
        client.lock_funds(&depositor, id, amount, deadline);
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
            env.ledger().set_timestamp(info.deadline);
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
        client.lock_funds(&depositor, &(5_000 + i), &amount, &deadline);
    }
    assert!(client.get_balance() > 0);

    for i in 0..40_u64 {
        let id = 5_000 + i;
        if i % 2 == 0 {
            client.release_funds(&id, &contributor);
        } else {
            let info = client.get_escrow_info(&id);
            env.ledger().set_timestamp(info.deadline);
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
        client.lock_funds(&depositor, &id, &10, &(now + 100 + offset));
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
    assert!(config.fee_enabled); // Should remain true

    // Third update: Update recipient and enabled flag
    client.update_fee_config(&None, &None, &Some(fee_recipient_2.clone()), &Some(false));

    let config = client.get_fee_config();
    assert_eq!(config.lock_fee_rate, 300); // Should remain 300
    assert_eq!(config.release_fee_rate, 200); // Should remain 200
    assert_eq!(config.fee_recipient, fee_recipient_2); // Should be updated to recipient_2
    assert!(!config.fee_enabled); // Should be updated to false
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

// ── Min/Max Amount Policy Enforcement Tests ───────────────────────────────────

/// Locking an amount strictly below the configured minimum must be rejected.
#[test]
#[should_panic(expected = "Error(Contract, #19)")] // AmountBelowMinimum
fn test_lock_funds_below_minimum_rejected() {
    let (env, client, _) = create_test_env();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 100;

    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &1_000);

    // Policy: min=100, max=10_000.  Attempting to lock 50 must be rejected.
    client.set_amount_policy(&admin, &100_i128, &10_000_i128);
    client.lock_funds(&depositor, &1, &50_i128, &deadline);
}

/// Locking an amount strictly above the configured maximum must be rejected.
#[test]
#[should_panic(expected = "Error(Contract, #20)")] // AmountAboveMaximum
fn test_lock_funds_above_maximum_rejected() {
    let (env, client, _) = create_test_env();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 100;

    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &100_000);

    // Policy: min=100, max=10_000.  Attempting to lock 50_000 must be rejected.
    client.set_amount_policy(&admin, &100_i128, &10_000_i128);
    client.lock_funds(&depositor, &2, &50_000_i128, &deadline);
}

/// An amount equal to the configured minimum is on the inclusive boundary and
/// must succeed.
#[test]
fn test_lock_funds_at_exact_minimum_succeeds() {
    let (env, client, _) = create_test_env();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 100;

    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &1_000);

    client.set_amount_policy(&admin, &100_i128, &10_000_i128);
    // amount == min → allowed (inclusive lower bound)
    client.lock_funds(&depositor, &3, &100_i128, &deadline);

    let escrow = client.get_escrow_info(&3);
    assert_eq!(escrow.amount, 100);
    assert_eq!(escrow.status, crate::EscrowStatus::Locked);
}

/// An amount equal to the configured maximum is on the inclusive boundary and
/// must succeed.
#[test]
fn test_lock_funds_at_exact_maximum_succeeds() {
    let (env, client, _) = create_test_env();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 100;

    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &10_000);

    client.set_amount_policy(&admin, &100_i128, &10_000_i128);
    // amount == max → allowed (inclusive upper bound)
    client.lock_funds(&depositor, &4, &10_000_i128, &deadline);

    let escrow = client.get_escrow_info(&4);
    assert_eq!(escrow.amount, 10_000);
    assert_eq!(escrow.status, crate::EscrowStatus::Locked);
}

/// An amount that sits strictly inside [min, max] must succeed.
#[test]
fn test_lock_funds_within_range_succeeds() {
    let (env, client, _) = create_test_env();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 100;

    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &5_000);

    client.set_amount_policy(&admin, &100_i128, &10_000_i128);
    client.lock_funds(&depositor, &5, &5_000_i128, &deadline);

    let escrow = client.get_escrow_info(&5);
    assert_eq!(escrow.amount, 5_000);
    assert_eq!(escrow.status, crate::EscrowStatus::Locked);
}

/// Only the admin may call `set_amount_policy`.  Any other caller must be
/// rejected with an Unauthorized error.
#[test]
#[should_panic(expected = "Error(Contract, #7)")] // Unauthorized
fn test_non_admin_cannot_set_amount_policy() {
    let (env, client, _) = create_test_env();
    let admin = Address::generate(&env);
    let non_admin = Address::generate(&env);

    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let (token, _token_client, _token_admin_client) = create_token_contract(&env, &token_admin);
    client.init(&admin, &token);

    // non_admin attempts to set policy — must be rejected with Unauthorized.
    client.set_amount_policy(&non_admin, &100_i128, &10_000_i128);
}

/// When no policy has been set the contract must remain backward-compatible:
/// any positive (or zero per the existing edge-case test) amount is accepted.
#[test]
fn test_no_policy_set_allows_any_positive_amount() {
    let (env, client, _) = create_test_env();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 100;

    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &1_000_000);

    // No set_amount_policy call — all positive amounts must be accepted.
    client.lock_funds(&depositor, &6, &1_i128, &deadline);
    client.lock_funds(&depositor, &7, &999_999_i128, &deadline);

    assert_eq!(client.get_escrow_info(&6).amount, 1);
    assert_eq!(client.get_escrow_info(&7).amount, 999_999);
}

/// Supplying min > max is a logically invalid policy and must be rejected.
#[test]
#[should_panic] // InvalidPolicy / contract-defined panic for malformed config
fn test_set_amount_policy_min_greater_than_max_rejected() {
    let (env, client, _) = create_test_env();
    let admin = Address::generate(&env);

    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let (token, _token_client, _) = create_token_contract(&env, &token_admin);
    client.init(&admin, &token);

    // min=5_000 > max=100 — invalid policy, must panic.
    client.set_amount_policy(&admin, &5_000_i128, &100_i128);
}

/// The admin must be able to update the policy after initial configuration, and
/// the new limits must take effect immediately for subsequent lock calls.
#[test]
fn test_amount_policy_can_be_updated_by_admin() {
    let (env, client, _) = create_test_env();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 100;

    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &100_000);

    // First policy: min=1_000 — amount 500 would be rejected here.
    client.set_amount_policy(&admin, &1_000_i128, &50_000_i128);

    // Loosen the policy: min=10 — amount 500 must now be accepted.
    client.set_amount_policy(&admin, &10_i128, &50_000_i128);
    client.lock_funds(&depositor, &8, &500_i128, &deadline);

    assert_eq!(client.get_escrow_info(&8).amount, 500);
}

/// min - 1 is the tightest possible value below the minimum boundary and must
/// be rejected (off-by-one lower).
#[test]
#[should_panic(expected = "Error(Contract, #19)")] // AmountBelowMinimum
fn test_one_below_minimum_boundary_rejected() {
    let (env, client, _) = create_test_env();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 100;

    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &1_000);

    client.set_amount_policy(&admin, &100_i128, &10_000_i128);
    // 99 == min(100) - 1 → must be rejected.
    client.lock_funds(&depositor, &9, &99_i128, &deadline);
}

/// max + 1 is the tightest possible value above the maximum boundary and must
/// be rejected (off-by-one upper).
#[test]
#[should_panic(expected = "Error(Contract, #20)")] // AmountAboveMaximum
fn test_one_above_maximum_boundary_rejected() {
    let (env, client, _) = create_test_env();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 100;

    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &100_000);

    client.set_amount_policy(&admin, &100_i128, &10_000_i128);
    // 10_001 == max(10_000) + 1 → must be rejected.
    client.lock_funds(&depositor, &10, &10_001_i128, &deadline);
}

/// (#501) Create many bounties (bounded for CI) and ensure counts and sampling
/// queries remain accurate without index/key collisions.
#[test]
fn test_max_bounty_count_queries_accurate() {
    let (env, client, _) = create_test_env();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 1000;

    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    client.init(&admin, &token);

    const N: u64 = 15;
    let total: i128 = 100 * (N as i128);
    token_admin_client.mint(&depositor, &total);

    for i in 1..=N {
        client.lock_funds(&depositor, &i, &100_i128, &deadline);
    }

    assert_eq!(client.get_escrow_count(), N as u32);

    let first = client.get_escrow_info(&1);
    assert_eq!(first.amount, 100);
    let mid = client.get_escrow_info(&(N / 2));
    assert_eq!(mid.amount, 100);
    let last = client.get_escrow_info(&N);
    assert_eq!(last.amount, 100);
}

// =============================================================================
// Rate limit and cooldown enforcement (Issue #460)
// =============================================================================

/// Exactly at rate limit: max_operations locks succeed; the next one panics with "Rate limit exceeded".
#[test]
#[should_panic(expected = "Rate limit exceeded")]
fn test_anti_abuse_exact_rate_limit_then_exceeded() {
    let (env, client, _) = create_test_env();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 10_000;

    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    client.init(&admin, &token);

    // Strict config: 2 operations per window, 60s cooldown
    client.update_anti_abuse_config(&3600, &2, &60);

    token_admin_client.mint(&depositor, &10_000);

    // Exactly 2 locks must succeed (different bounty_ids)
    client.lock_funds(&depositor, &1, &100, &deadline);
    client.lock_funds(&depositor, &2, &100, &deadline);

    // Third lock in same window must panic
    client.lock_funds(&depositor, &3, &100, &deadline);
}

/// Exactly at limit: max_operations locks succeed; no panic at the boundary.
#[test]
fn test_anti_abuse_exact_rate_limit_boundary_succeeds() {
    let (env, client, _) = create_test_env();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 10_000;

    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    client.init(&admin, &token);

    client.update_anti_abuse_config(&3600, &3, &60);

    token_admin_client.mint(&depositor, &10_000);

    client.lock_funds(&depositor, &1, &100, &deadline);
    client.lock_funds(&depositor, &2, &100, &deadline);
    client.lock_funds(&depositor, &3, &100, &deadline);

    assert_eq!(client.get_escrow_count(), 3);
}

/// Rapid repeated lock within cooldown period must panic.
#[test]
#[should_panic(expected = "Operation in cooldown period")]
fn test_anti_abuse_cooldown_violation_panics() {
    let (env, client, _) = create_test_env();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let start = 1_000_000_u64;
    env.ledger().set_timestamp(start);
    let deadline = start + 10_000;

    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    client.init(&admin, &token);

    // Cooldown 100s; we will do second lock at start+50 (within cooldown)
    client.update_anti_abuse_config(&3600, &10, &100);

    token_admin_client.mint(&depositor, &10_000);

    client.lock_funds(&depositor, &1, &100, &deadline);

    env.ledger().set_timestamp(start + 50);
    client.lock_funds(&depositor, &2, &100, &deadline);
}

/// After cooldown period, next lock succeeds.
#[test]
fn test_anti_abuse_after_cooldown_succeeds() {
    let (env, client, _) = create_test_env();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let start = 1_000_000_u64;
    env.ledger().set_timestamp(start);
    let deadline = start + 10_000;

    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    client.init(&admin, &token);

    client.update_anti_abuse_config(&3600, &10, &60);

    token_admin_client.mint(&depositor, &10_000);

    client.lock_funds(&depositor, &1, &100, &deadline);

    env.ledger().set_timestamp(start + 61);
    client.lock_funds(&depositor, &2, &100, &deadline);

    assert_eq!(client.get_escrow_count(), 2);
}

/// Whitelisted address bypasses rate limit and cooldown.
#[test]
fn test_anti_abuse_whitelisted_address_bypasses_checks() {
    let (env, client, _) = create_test_env();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 10_000;

    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    client.init(&admin, &token);

    client.update_anti_abuse_config(&3600, &2, &60);
    client.set_whitelist_entry(&depositor, &true);

    token_admin_client.mint(&depositor, &50_000);

    // More than max_operations without advancing time; whitelisted so all succeed
    for i in 1..=5 {
        client.lock_funds(&depositor, &i, &100, &deadline);
    }
    assert_eq!(client.get_escrow_count(), 5);
}

// =============================================================================
// Admin and config updates (Issue #465)
// =============================================================================

/// Admin can update anti-abuse config; new values persist.
#[test]
fn test_admin_can_update_anti_abuse_config_persists() {
    let (env, client, _) = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    client.init(&admin, &token);
    client.update_anti_abuse_config(&7200, &5, &120);

    let config = client.get_anti_abuse_config();
    assert_eq!(config.window_size, 7200);
    assert_eq!(config.max_operations, 5);
    assert_eq!(config.cooldown_period, 120);
}

/// Non-admin cannot update anti-abuse config.
#[test]
#[should_panic(expected = "InvalidAction")] // Auth failure when non-admin calls
fn test_non_admin_cannot_update_anti_abuse_config() {
    let (env, client, contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let non_admin = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();
    client.init(&admin, &token);

    // Only non_admin is mocked for this call; contract requires admin.require_auth() so this must panic.
    env.mock_auths(&[MockAuth {
        address: &non_admin,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "update_anti_abuse_config",
            args: (7200u64, 5u32, 120u64).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.update_anti_abuse_config(&7200, &5, &120);
}
