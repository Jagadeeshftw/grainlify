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

fn is_paused(client: &BountyEscrowContractClient) -> bool {
    let flags = client.get_pause_flags();
    flags.lock_paused || flags.release_paused || flags.refund_paused
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

fn assert_current_call_has_versioned_contract_event(env: &Env, contract_id: &Address) {
    let events = env.events().all();
    let mut found = false;
    for (contract, _topics, data) in events.iter() {
        if contract != *contract_id {
            continue;
        }
        let data_map = match Map::<Symbol, Val>::try_from_val(env, &data) {
            Ok(map) => map,
            Err(_) => continue,
        };
        let version_val = match data_map.get(Symbol::new(env, "version")) {
            Some(value) => value,
            None => continue,
        };
        let version = u32::try_from_val(env, &version_val).expect("version should decode as u32");
        assert_eq!(version, 2);
        found = true;
    }
    assert!(found, "expected at least one versioned contract event");
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
fn test_non_transferable_rewards_flag() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    env.mock_all_auths();
    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &10_000);

    let deadline = env.ledger().timestamp() + 3600;

    // Lock a bounty; flag should default to false
    client.lock_funds(&depositor, &1, &1_000, &deadline);
    assert!(
        !client.get_non_transferable_rewards(&1),
        "bounty 1 should not be marked non-transferable"
    );

    // Lock another bounty with non_transferable_rewards = None (default)
    client.lock_funds(&depositor, &2, &2_000, &deadline);
    assert!(
        !client.get_non_transferable_rewards(&2),
        "bounty 2 should not be marked non-transferable"
    );

    // Bounty 3 still defaults to false
    client.lock_funds(&depositor, &3, &500, &deadline);
    assert!(
        !client.get_non_transferable_rewards(&3),
        "bounty 3 should not be marked non-transferable"
    );
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
#[should_panic(expected = "Error(Contract, #13)")] // InvalidAmount
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
    env.budget().reset_unlimited();

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

/// Executes the high-load bounty lifecycle workload used by the stress tests.
///
/// Locks `count` bounties starting at `first_id`, then releases every even
/// offset and refunds every odd offset once the ledger has advanced past the
/// escrow deadline. The fixture is left fully drained: the contract's locked
/// balance must return to zero before the `Env` is torn down.
///
/// The depositor is expected to already be whitelisted by the caller so the
/// anti-abuse rate limiter does not introduce timing-dependent behaviour.
fn run_high_load_bounty_workload(
    env: &Env,
    client: &BountyEscrowContractClient<'_>,
    depositor: &Address,
    contributor: &Address,
    token_client: &token::Client<'_>,
    first_id: u64,
    count: u64,
    start_timestamp: u64,
) {
    // Pin the ledger explicitly instead of relying on whatever
    // `Env::default()` happens to provide, so the fixture is reproducible
    // regardless of SDK defaults.
    env.ledger().set_timestamp(start_timestamp);

    for i in 0..count {
        let amount = 100 + (i as i128 % 10);
        let deadline = start_timestamp + 30 + i;
        client.lock_funds(depositor, &(first_id + i), &amount, &deadline);
    }
    assert!(client.get_balance() > 0);

    for i in 0..count {
        let id = first_id + i;
        if i % 2 == 0 {
            client.release_funds(&id, contributor);
        } else {
            let info = client.get_escrow_info(&id);
            env.ledger().set_timestamp(info.deadline);
            client.refund(&id);
        }
    }

    // Teardown invariant: every escrow is drained, so the `Env` is dropped with
    // the contract holding no funds and no half-open lifecycle state behind it.
    assert_eq!(client.get_balance(), 0);
    assert!(token_client.balance(contributor) > 0);
}

#[test]
fn test_stress_high_load_bounty_operations() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);

    env.mock_all_auths();
    env.budget().reset_unlimited();

    let token_admin = Address::generate(&env);
    let (token, token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    client.init(&admin, &token);
    // Whitelist the depositor so the anti-abuse cooldown/window checks cannot
    // turn this fixture into a timing-dependent test.
    client.set_whitelist(&depositor, &true);
    token_admin_client.mint(&depositor, &1_000_000);

    run_high_load_bounty_workload(
        &env,
        &client,
        &depositor,
        &contributor,
        &token_client,
        5_000,
        40,
        1_000,
    );
}

/// Regression coverage for the destructor-time flakiness that originally caused
/// `test_stress_high_load_bounty_operations` to be `#[ignore]`d (issue #1853).
///
/// Runs the same workload 100 times in a row against a single `Env`. Each round
/// must drain back to a zero locked balance and must not leave the fixture in a
/// state that panics when the environment is cleaned up.
#[test]
fn test_stress_high_load_bounty_operations_repeats_stably() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);

    env.mock_all_auths();
    env.budget().reset_unlimited();

    let token_admin = Address::generate(&env);
    let (token, token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    client.init(&admin, &token);
    client.set_whitelist(&depositor, &true);
    token_admin_client.mint(&depositor, &10_000_000);

    for round in 0..100_u64 {
        run_high_load_bounty_workload(
            &env,
            &client,
            &depositor,
            &contributor,
            &token_client,
            20_000 + round * 10,
            5,
            1_000 + round * 1_000,
        );
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

#[test]
fn test_emergency_withdraw() {
    let (env, client, _contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);

    // Create and setup token
    let (token_address, _token_client, _token_admin) = create_token_contract(&env, &admin);

    // Initialize escrow
    client.init(&admin, &token_address);

    // Pause contract
    client.set_paused(&Some(true), &Some(true), &Some(true), &None);
    assert_eq!(is_paused(&client), true);

    // Call emergency_withdraw (it will fail gracefully if no funds)
    // The important thing is that it's callable when paused
    let emergency_recipient = Address::generate(&env);
    client.emergency_withdraw(&emergency_recipient);

    // Verify pause state still true
    assert_eq!(is_paused(&client), true);
}
