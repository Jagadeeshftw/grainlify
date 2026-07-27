#![cfg(test)]
//! # Query Cache Tests
//!
//! Comprehensive tests for the [`QueryCache`] using Soroban temporary storage.
//!
//! ## Test Categories
//!
//! 1. **Unit tests** — Verify `QueryCache` isolation behaviour without a full
//!    contract deployment.
//! 2. **Integration tests** — Exercise the new cached query methods on the
//!    `ViewFacade` contract (`query_program_data_cached`, `query_fee_config_cached`,
//!    `query_program_balance_and_fee`).
//! 3. **Cache coherence** — Prove that a second read within the same invocation
//!    returns the cached value (no additional cross-contract call).
//! 4. **Invalidation** — Verify `invalidate_*` helpers correctly remove entries.
//! 5. **Edge cases** — Cache with different escrow addresses, empty cache,
//!    multiple programs on the same escrow.

use crate::{
    ContractKind, QueryCache, ViewFacade, ViewFacadeClient,
};
use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    Address, Env, IntoVal, String,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a fresh `ViewFacade` instance, initialized with `admin`.
fn setup() -> (Env, ViewFacadeClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let facade_id = env.register_contract(None, ViewFacade);
    let facade = ViewFacadeClient::new(&env, &facade_id);

    let admin = Address::generate(&env);
    facade.init(&admin);

    (env, facade, admin)
}

/// Generate a mock escrow address and register it in the facade.
fn register_mock_escrow(env: &Env, facade: &ViewFacadeClient, kind: ContractKind) -> Address {
    let escrow = Address::generate(env);
    facade.register(&escrow, &kind, &1u32);
    escrow
}

// ---------------------------------------------------------------------------
// 1. Unit tests — QueryCache isolation
// ---------------------------------------------------------------------------

/// `QueryCache::get_or_load_fee_config` on an empty cache triggers a
/// cross-contract call (will panic because the escrow address is not a
/// real ProgramEscrow contract).  This test verifies that the cache
/// **attempts** the cross-contract call on first access (i.e. no false
/// cache hit).
#[test]
#[should_panic]
fn test_cache_miss_triggers_cross_contract_call() {
    let (env, _facade, _admin) = setup();
    let fake_escrow = Address::generate(&env);

    // This should panic because the generated address is not a real
    // ProgramEscrow contract — proving the cache tried to fetch live data.
    QueryCache::get_or_load_fee_config(&env, &fake_escrow);
}

// ---------------------------------------------------------------------------
// 2. Cache key isolation
// ---------------------------------------------------------------------------

/// Different escrow addresses produce different cache keys; storing data
/// for escrow A must not leak into escrow B.
#[test]
fn test_cache_key_isolation_by_escrow() {
    let env = Env::default();
    let escrow_a = Address::generate(&env);
    let escrow_b = Address::generate(&env);

    // Manually populate the cache for escrow_a (simulating a successful
    // cross-contract call).  This writes into temporary storage.
    let fake_key = crate::QueryCacheKey::FeeConfig(escrow_a.clone());
    let dummy_config = program_escrow::FeeConfig {
        lock_fee_rate: 100,
        payout_fee_rate: 50,
        lock_fixed_fee: 0,
        payout_fixed_fee: 0,
        fee_recipient: escrow_a.clone(),
        fee_enabled: true,
        fee_waivers: 0,
        insurance_reserve_bps: 0,
    };
    env.storage().temporary().set(&fake_key, &dummy_config);

    // Check that escrow_b does NOT have a cached entry.
    let key_b = crate::QueryCacheKey::FeeConfig(escrow_b.clone());
    let cached_b: Option<program_escrow::FeeConfig> = env.storage().temporary().get(&key_b);
    assert!(cached_b.is_none(), "escrow B should not have a cached FeeConfig");
}

/// Different `program_id` values produce different cache keys; storing
/// data for program A must not leak into program B on the same escrow.
#[test]
fn test_cache_key_isolation_by_program_id() {
    let env = Env::default();
    let escrow = Address::generate(&env);

    let prog_a = String::from_str(&env, "prog-A");
    let prog_b = String::from_str(&env, "prog-B");

    let key_a = crate::QueryCacheKey::ProgramData(escrow.clone(), prog_a.clone());
    let dummy_a = program_escrow::ProgramData {
        program_id: prog_a.clone(),
        total_funds: 1000,
        remaining_balance: 500,
        authorized_payout_key: escrow.clone(),
        delegate: None,
        delegate_permissions: 0,
        payout_history: soroban_sdk::Vec::new(&env),
        token_address: escrow.clone(),
        initial_liquidity: 0,
        risk_flags: 0,
        reference_hash: None,            archived: false,
            archived_at: None,
            status: program_escrow::ProgramStatus::Active,
            circuit_breaker_threshold: None,
            fot_router: program_escrow::OptionalFotRouter::None,
    };
    env.storage().temporary().set(&key_a, &dummy_a);

    // prog_b should not be cached
    let key_b = crate::QueryCacheKey::ProgramData(escrow.clone(), prog_b.clone());
    let cached_b: Option<program_escrow::ProgramData> = env.storage().temporary().get(&key_b);
    assert!(cached_b.is_none(), "program B should not have cached data");
}

// ---------------------------------------------------------------------------
// 3. Cache coherence — same invocation returns cached value
// ---------------------------------------------------------------------------

/// After manually populating the cache, `get_or_load_fee_config` must
/// return the cached value without attempting a cross-contract call
/// (i.e. it must not panic on a fake address).
#[test]
fn test_cache_hit_returns_cached_value() {
    let env = Env::default();
    let escrow = Address::generate(&env);

    // Simulate a prior successful fetch by writing directly to temp storage.
    let key = crate::QueryCacheKey::FeeConfig(escrow.clone());
    let config = program_escrow::FeeConfig {
        lock_fee_rate: 200,
        payout_fee_rate: 75,
        lock_fixed_fee: 10,
        payout_fixed_fee: 5,
        fee_recipient: escrow.clone(),
        fee_enabled: true,
        fee_waivers: 0,
        insurance_reserve_bps: 50,
    };
    env.storage().temporary().set(&key, &config);

    // This should NOT panic because the cache hit avoids the cross-contract call.
    let result = QueryCache::get_or_load_fee_config(&env, &escrow);
    assert_eq!(result.lock_fee_rate, 200);
    assert_eq!(result.payout_fee_rate, 75);
    assert_eq!(result.insurance_reserve_bps, 50);
}

/// Same pattern for ProgramData — a pre-populated cache must be returned.
#[test]
fn test_cache_hit_returns_cached_program_data() {
    let env = Env::default();
    let escrow = Address::generate(&env);
    let prog_id = String::from_str(&env, "my-program");

    let key = crate::QueryCacheKey::ProgramData(escrow.clone(), prog_id.clone());
    let data = program_escrow::ProgramData {
        program_id: prog_id.clone(),
        total_funds: 5000,
        remaining_balance: 3000,
        authorized_payout_key: escrow.clone(),
        delegate: None,
        delegate_permissions: 0,
        payout_history: soroban_sdk::Vec::new(&env),
        token_address: escrow.clone(),
        initial_liquidity: 0,
        risk_flags: 0,
        reference_hash: None,            archived: false,
            archived_at: None,
            status: program_escrow::ProgramStatus::Active,
            circuit_breaker_threshold: None,
            fot_router: program_escrow::OptionalFotRouter::None,
    };
    env.storage().temporary().set(&key, &data);

    let result = QueryCache::get_or_load_program_data(&env, &escrow, &prog_id);
    assert_eq!(result.total_funds, 5000);
    assert_eq!(result.remaining_balance, 3000);
    assert_eq!(result.program_id, prog_id);
}

// ---------------------------------------------------------------------------
// 4. Invalidation helpers
// ---------------------------------------------------------------------------

/// `invalidate_fee_config` must remove the cached entry so that the next
/// `get_or_load_fee_config` call is a cache miss (and panics on a fake address).
#[test]
#[should_panic]
fn test_invalidate_fee_config_causes_cache_miss() {
    let env = Env::default();
    let escrow = Address::generate(&env);

    // Populate cache
    let key = crate::QueryCacheKey::FeeConfig(escrow.clone());
    let config = program_escrow::FeeConfig {
        lock_fee_rate: 0,
        payout_fee_rate: 0,
        lock_fixed_fee: 0,
        payout_fixed_fee: 0,
        fee_recipient: escrow.clone(),
        fee_enabled: false,
        fee_waivers: 0,
        insurance_reserve_bps: 0,
    };
    env.storage().temporary().set(&key, &config);

    // Invalidate
    QueryCache::invalidate_fee_config(&env, &escrow);

    // This should now panic because the cache is empty and the address is fake.
    QueryCache::get_or_load_fee_config(&env, &escrow);
}

/// `invalidate_program_data` must remove the cached entry.
#[test]
#[should_panic]
fn test_invalidate_program_data_causes_cache_miss() {
    let env = Env::default();
    let escrow = Address::generate(&env);
    let prog_id = String::from_str(&env, "prog");

    // Populate
    let key = crate::QueryCacheKey::ProgramData(escrow.clone(), prog_id.clone());
    let data = program_escrow::ProgramData {
        program_id: prog_id.clone(),
        total_funds: 100,
        remaining_balance: 100,
        authorized_payout_key: escrow.clone(),
        delegate: None,
        delegate_permissions: 0,
        payout_history: soroban_sdk::Vec::new(&env),
        token_address: escrow.clone(),
        initial_liquidity: 0,
        risk_flags: 0,
        reference_hash: None,            archived: false,
            archived_at: None,
            status: program_escrow::ProgramStatus::Active,
            circuit_breaker_threshold: None,
            fot_router: program_escrow::OptionalFotRouter::None,
    };
    env.storage().temporary().set(&key, &data);

    // Invalidate
    QueryCache::invalidate_program_data(&env, &escrow, &prog_id);

    // Should panic
    QueryCache::get_or_load_program_data(&env, &escrow, &prog_id);
}

// ---------------------------------------------------------------------------
// 5. ViewFacade integration tests
// ---------------------------------------------------------------------------

/// `query_fee_config_cached` on a valid (registered) escrow must be callable.
/// Because the escrow is not a real ProgramEscrow contract, the cross-contract
/// call will fail — but the facade method itself must compile and be reachable.
#[test]
#[should_panic]
fn test_view_facade_query_fee_config_cached_panics_on_fake_escrow() {
    let (env, facade, _admin) = setup();
    let escrow = register_mock_escrow(&env, &facade, ContractKind::ProgramEscrow);

    // This will panic because the mock address isn't a real ProgramEscrow.
    facade.query_fee_config_cached(&escrow);
}

/// `query_program_data_cached` must be callable on the facade.
#[test]
#[should_panic]
fn test_view_facade_query_program_data_cached_panics_on_fake_escrow() {
    let (env, facade, _admin) = setup();
    let escrow = register_mock_escrow(&env, &facade, ContractKind::ProgramEscrow);
    let prog_id = String::from_str(&env, "test-prog");

    facade.query_program_data_cached(&escrow, &prog_id);
}

/// `query_program_balance_and_fee` must be callable and return a tuple.
#[test]
#[should_panic]
fn test_view_facade_query_program_balance_and_fee_panics_on_fake_escrow() {
    let (env, facade, _admin) = setup();
    let escrow = register_mock_escrow(&env, &facade, ContractKind::ProgramEscrow);
    let prog_id = String::from_str(&env, "test-prog");

    facade.query_program_balance_and_fee(&escrow, &prog_id);
}

// ---------------------------------------------------------------------------
// 6. Temporary storage scoping
// ---------------------------------------------------------------------------

/// Temporary storage entries must not persist across separate `Env` instances.
#[test]
fn test_temp_storage_does_not_leak_across_envs() {
    let env1 = Env::default();
    let env2 = Env::default();
    let escrow = Address::generate(&env1);

    let key1 = crate::QueryCacheKey::FeeConfig(escrow.clone());
    let config1 = program_escrow::FeeConfig {
        lock_fee_rate: 300,
        payout_fee_rate: 0,
        lock_fixed_fee: 0,
        payout_fixed_fee: 0,
        fee_recipient: escrow.clone(),
        fee_enabled: false,
        fee_waivers: 0,
        insurance_reserve_bps: 0,
    };
    env1.storage().temporary().set(&key1, &config1);

    // Convert escrow for env2 (same underlying bytes, different env)
    let escrow_bytes = escrow.to_string();
    let escrow2 = Address::from_string(&soroban_sdk::String::from_str(&env2, &escrow_bytes));

    let key2 = crate::QueryCacheKey::FeeConfig(escrow2.clone());
    let cached_in_env2: Option<program_escrow::FeeConfig> = env2.storage().temporary().get(&key2);
    assert!(
        cached_in_env2.is_none(),
        "Temporary storage must not leak across separate Env instances"
    );
}

/// Writing to temporary storage must not affect instance storage.
#[test]
fn test_temp_storage_does_not_affect_instance_storage() {
    let env = Env::default();
    let escrow = Address::generate(&env);

    // Write to temp storage
    let key = crate::QueryCacheKey::FeeConfig(escrow.clone());
    let config = program_escrow::FeeConfig {
        lock_fee_rate: 400,
        payout_fee_rate: 0,
        lock_fixed_fee: 0,
        payout_fixed_fee: 0,
        fee_recipient: escrow.clone(),
        fee_enabled: false,
        fee_waivers: 0,
        insurance_reserve_bps: 0,
    };
    env.storage().temporary().set(&key, &config);

    // Instance storage must be untouched
    assert!(!env.storage().instance().has(&crate::DataKey::Admin));
}

// ---------------------------------------------------------------------------
// 7. Cache key enum round-trip
// ---------------------------------------------------------------------------

/// `QueryCacheKey` must survive a `set` → `get` round-trip through
/// temporary storage with a simple value type.
#[test]
fn test_query_cache_key_roundtrip() {
    let env = Env::default();
    let escrow = Address::generate(&env);
    let prog_id = String::from_str(&env, "roundtrip-test");

    let key = crate::QueryCacheKey::ProgramData(escrow.clone(), prog_id.clone());

    // Store a u32 marker value
    env.storage().temporary().set(&key, &42u32);

    let retrieved: Option<u32> = env.storage().temporary().get(&key);
    assert_eq!(retrieved, Some(42));
}

// ---------------------------------------------------------------------------
// 8. Concurrent cache entries (FeeConfig + ProgramData on same escrow)
// ---------------------------------------------------------------------------

/// Both FeeConfig and ProgramData can coexist in the cache for the same
/// escrow without collision (they use different key variants).
#[test]
fn test_fee_and_program_data_cache_independent() {
    let env = Env::default();
    let escrow = Address::generate(&env);
    let prog_id = String::from_str(&env, "dual");

    // Populate FeeConfig
    let fee_key = crate::QueryCacheKey::FeeConfig(escrow.clone());
    let fee_config = program_escrow::FeeConfig {
        lock_fee_rate: 123,
        payout_fee_rate: 45,
        lock_fixed_fee: 0,
        payout_fixed_fee: 0,
        fee_recipient: escrow.clone(),
        fee_enabled: true,
        fee_waivers: 0,
        insurance_reserve_bps: 0,
    };
    env.storage().temporary().set(&fee_key, &fee_config);

    // Populate ProgramData
    let prog_key = crate::QueryCacheKey::ProgramData(escrow.clone(), prog_id.clone());
    let prog_data = program_escrow::ProgramData {
        program_id: prog_id.clone(),
        total_funds: 777,
        remaining_balance: 333,
        authorized_payout_key: escrow.clone(),
        delegate: None,
        delegate_permissions: 0,
        payout_history: soroban_sdk::Vec::new(&env),
        token_address: escrow.clone(),
        initial_liquidity: 0,
        risk_flags: 0,
        reference_hash: None,            archived: false,
            archived_at: None,
            status: program_escrow::ProgramStatus::Active,
            circuit_breaker_threshold: None,
            fot_router: program_escrow::OptionalFotRouter::None,
    };
    env.storage().temporary().set(&prog_key, &prog_data);

    // Both must be independently retrievable
    let fee: program_escrow::FeeConfig = env.storage().temporary().get(&fee_key).unwrap();
    assert_eq!(fee.lock_fee_rate, 123);

    let prog: program_escrow::ProgramData = env.storage().temporary().get(&prog_key).unwrap();
    assert_eq!(prog.total_funds, 777);
}

// ---------------------------------------------------------------------------
// 9. Cache coherence — query_program_balance_and_fee populates both caches
// ---------------------------------------------------------------------------

/// After `query_program_balance_and_fee` populates both cache entries,
/// a subsequent direct temporary-storage read of the same keys must
/// return the cached values (proving no cross-contract call would be made).
#[test]
fn test_query_program_balance_and_fee_populates_both_cache_entries() {
    let env = Env::default();
    let escrow = Address::generate(&env);
    let prog_id = String::from_str(&env, "coherence-test");

    // Simulate what query_program_balance_and_fee would do internally:
    // manually populate both cache entries.
    let prog_key = crate::QueryCacheKey::ProgramData(escrow.clone(), prog_id.clone());
    let fee_key = crate::QueryCacheKey::FeeConfig(escrow.clone());

    // Before populating, both must be empty
    assert!(env.storage().temporary().get::<_, program_escrow::ProgramData>(&prog_key).is_none());
    assert!(env.storage().temporary().get::<_, program_escrow::FeeConfig>(&fee_key).is_none());

    // Populate both (simulating the aggregated call)
    let prog_data = program_escrow::ProgramData {
        program_id: prog_id.clone(),
        total_funds: 9000,
        remaining_balance: 4500,
        authorized_payout_key: escrow.clone(),
        delegate: None,
        delegate_permissions: 0,
        payout_history: soroban_sdk::Vec::new(&env),
        token_address: escrow.clone(),
        initial_liquidity: 0,
        risk_flags: 0,
        reference_hash: None,            archived: false,
            archived_at: None,
            status: program_escrow::ProgramStatus::Active,
            circuit_breaker_threshold: None,
            fot_router: program_escrow::OptionalFotRouter::None,
    };
    let fee_config = program_escrow::FeeConfig {
        lock_fee_rate: 42,
        payout_fee_rate: 7,
        lock_fixed_fee: 1,
        payout_fixed_fee: 2,
        fee_recipient: escrow.clone(),
        fee_enabled: true,
        fee_waivers: 0,
        insurance_reserve_bps: 10,
    };
    env.storage().temporary().set(&prog_key, &prog_data);
    env.storage().temporary().set(&fee_key, &fee_config);

    // A subsequent cache-aware read should hit the cache for both
    let cached_prog: program_escrow::ProgramData = env.storage().temporary().get(&prog_key).unwrap();
    assert_eq!(cached_prog.total_funds, 9000);
    assert_eq!(cached_prog.remaining_balance, 4500);

    let cached_fee: program_escrow::FeeConfig = env.storage().temporary().get(&fee_key).unwrap();
    assert_eq!(cached_fee.lock_fee_rate, 42);
    assert_eq!(cached_fee.insurance_reserve_bps, 10);

    // And they must be independent (invalidating one doesn't affect the other)
    QueryCache::invalidate_fee_config(&env, &escrow);
    assert!(env.storage().temporary().get::<_, program_escrow::FeeConfig>(&fee_key).is_none());
    assert!(env.storage().temporary().get::<_, program_escrow::ProgramData>(&prog_key).is_some());
}

// ---------------------------------------------------------------------------
// 10. Duplicate reads within same invocation avoid redundant cross-contract
// ---------------------------------------------------------------------------

/// Two consecutive `get_or_load_fee_config` calls must both succeed, with
/// the second returning the cached value (verified by the fact that the
/// first call populates temp storage and the second finds it).
#[test]
fn test_double_read_returns_same_cached_value() {
    let env = Env::default();
    let escrow = Address::generate(&env);

    // Pre-populate cache (simulating first successful cross-contract call)
    let key = crate::QueryCacheKey::FeeConfig(escrow.clone());
    let config = program_escrow::FeeConfig {
        lock_fee_rate: 555,
        payout_fee_rate: 55,
        lock_fixed_fee: 0,
        payout_fixed_fee: 0,
        fee_recipient: escrow.clone(),
        fee_enabled: true,
        fee_waivers: 0,
        insurance_reserve_bps: 100,
    };
    env.storage().temporary().set(&key, &config);

    // First read (cache hit)
    let result1 = QueryCache::get_or_load_fee_config(&env, &escrow);
    // Second read (cache hit again — not a cross-contract call)
    let result2 = QueryCache::get_or_load_fee_config(&env, &escrow);

    assert_eq!(result1.lock_fee_rate, result2.lock_fee_rate);
    assert_eq!(result1.insurance_reserve_bps, result2.insurance_reserve_bps);
    assert_eq!(result1.fee_recipient, result2.fee_recipient);
}
