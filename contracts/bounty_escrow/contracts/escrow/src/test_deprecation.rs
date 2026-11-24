//! Tests for the controlled kill-switch (deprecation) and migration-target flow.
//!
//! # Coverage goals
//! - `set_deprecated` is admin-only; non-admin callers are rejected.
//! - After deprecation, `lock_funds`, `batch_lock_funds`, and `lock_funds_anonymous`
//!   all return `Error::ContractDeprecated` (code 34).
//! - Release, partial-release, and refund remain available after deprecation.
//! - `get_deprecation_status` is always readable regardless of state.
//! - Migration target is stored and returned correctly.
//! - Migration target can be updated or cleared after the initial set.
//! - Deprecation can be toggled off, restoring new-lock capability.
//! - Multiple state transitions are handled correctly.
//! - The `DeprecationStateChanged` event is emitted on every state change.

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger as _},
    token, vec, Address, BytesN, Env, Vec,
};

// ============================================================================
// Helpers
// ============================================================================

fn create_token<'a>(
    e: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let contract = e.register_stellar_asset_contract_v2(admin.clone());
    let addr = contract.address();
    (
        token::Client::new(e, &addr),
        token::StellarAssetClient::new(e, &addr),
    )
}

/// Spin up a fully-initialised escrow contract with a funded depositor.
///
/// Returns `(client, admin, depositor, token_client, token_admin_client)`.
fn setup<'a>(
    env: &'a Env,
) -> (
    BountyEscrowContractClient<'a>,
    Address,
    Address,
    token::Client<'a>,
    token::StellarAssetClient<'a>,
) {
    env.mock_all_auths();
    let admin = Address::generate(env);
    let depositor = Address::generate(env);
    let (token, token_admin) = create_token(env, &admin);
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let escrow = BountyEscrowContractClient::new(env, &contract_id);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &10_000_000);
    (escrow, admin, depositor, token, token_admin)
}

// ============================================================================
// Default state
// ============================================================================

/// A freshly-initialised contract must not be deprecated and must have no
/// migration target.
#[test]
fn test_default_not_deprecated() {
    let env = Env::default();
    let (escrow, _admin, _depositor, _token, _) = setup(&env);

    let status = escrow.get_deprecation_status();

    assert!(
        !status.deprecated,
        "contract should not be deprecated by default"
    );
    assert!(
        status.migration_target.is_none(),
        "migration target should be None by default"
    );
}

// ============================================================================
// set_deprecated — happy paths
// ============================================================================

/// Admin can deprecate the contract without a migration target.
#[test]
fn test_set_deprecated_no_migration_target() {
    let env = Env::default();
    let (escrow, _admin, _depositor, _token, _) = setup(&env);

    escrow.set_deprecated(&true, &None);

    let status = escrow.get_deprecation_status();
    assert!(status.deprecated);
    assert!(status.migration_target.is_none());
}

/// Admin can deprecate the contract and set a migration target in one call.
#[test]
fn test_set_deprecated_with_migration_target() {
    let env = Env::default();
    let (escrow, _admin, _depositor, _token, _) = setup(&env);
    let new_contract = Address::generate(&env);

    escrow.set_deprecated(&true, &Some(new_contract.clone()));

    let status = escrow.get_deprecation_status();
    assert!(status.deprecated);
    assert_eq!(
        status.migration_target,
        Some(new_contract),
        "migration target must match the address passed to set_deprecated"
    );
}

/// Admin can update the migration target after the initial deprecation call.
#[test]
fn test_update_migration_target_after_deprecation() {
    let env = Env::default();
    let (escrow, _admin, _depositor, _token, _) = setup(&env);
    let first_target = Address::generate(&env);
    let second_target = Address::generate(&env);

    escrow.set_deprecated(&true, &Some(first_target));
    escrow.set_deprecated(&true, &Some(second_target.clone()));

    let status = escrow.get_deprecation_status();
    assert_eq!(
        status.migration_target,
        Some(second_target),
        "migration target should reflect the most recent call"
    );
}

/// Admin can clear the migration target by passing None while keeping deprecated=true.
#[test]
fn test_clear_migration_target() {
    let env = Env::default();
    let (escrow, _admin, _depositor, _token, _) = setup(&env);
    let target = Address::generate(&env);

    escrow.set_deprecated(&true, &Some(target));
    escrow.set_deprecated(&true, &None);

    let status = escrow.get_deprecation_status();
    assert!(status.deprecated);
    assert!(
        status.migration_target.is_none(),
        "migration target should be cleared after passing None"
    );
}

/// Admin can un-deprecate the contract (toggle off).
#[test]
fn test_unset_deprecated_restores_lock() {
    let env = Env::default();
    let (escrow, _admin, depositor, _token, _) = setup(&env);

    escrow.set_deprecated(&true, &None);
    escrow.set_deprecated(&false, &None);

    let status = escrow.get_deprecation_status();
    assert!(!status.deprecated);

    // New locks must succeed after un-deprecation.
    let deadline = env.ledger().timestamp() + 1_000;
    escrow.lock_funds(&depositor, &1u64, &1_000i128, &deadline);
    let info = escrow.get_escrow_info(&1u64);
    assert_eq!(info.status, EscrowStatus::Locked);
}

/// Multiple toggle cycles (on → off → on) are handled correctly.
#[test]
fn test_multiple_deprecation_toggles() {
    let env = Env::default();
    let (escrow, _admin, depositor, _token, _) = setup(&env);
    let deadline = env.ledger().timestamp() + 1_000;

    // First cycle: deprecate → un-deprecate → lock succeeds
    escrow.set_deprecated(&true, &None);
    escrow.set_deprecated(&false, &None);
    escrow.lock_funds(&depositor, &1u64, &500i128, &deadline);

    // Second cycle: deprecate again → lock blocked
    escrow.set_deprecated(&true, &None);
    let result = escrow.try_lock_funds(&depositor, &2u64, &500i128, &deadline);
    assert!(
        result.is_err(),
        "lock_funds must fail when deprecated again"
    );
}

// ============================================================================
// set_deprecated — access control
// ============================================================================

/// A non-admin caller must not be able to deprecate the contract.
///
/// `mock_all_auths` is intentionally NOT used here so that the auth check fires.
#[test]
#[should_panic]
fn test_non_admin_cannot_set_deprecated() {
    let env = Env::default();
    // Do NOT call env.mock_all_auths() — we want real auth enforcement.
    let admin = Address::generate(&env);
    let attacker = Address::generate(&env);
    let (token, _token_admin) = create_token(&env, &admin);
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let escrow = BountyEscrowContractClient::new(&env, &contract_id);

    // Init with mock_all_auths just for the init call.
    env.mock_all_auths();
    escrow.init(&admin, &token.address);

    // Now clear mocked auths and attempt set_deprecated as attacker.
    // The SDK will panic because attacker's auth is not satisfied.
    env.set_auths(&[]);
    // This call should panic — attacker is not the admin.
    escrow.set_deprecated(&true, &None);
    let _ = attacker; // suppress unused warning
}

// ============================================================================
// lock_funds blocked when deprecated
// ============================================================================

/// `lock_funds` must return `Error::ContractDeprecated` (code 34) when deprecated.
#[test]
fn test_lock_funds_blocked_when_deprecated() {
    let env = Env::default();
    let (escrow, _admin, depositor, _token, _) = setup(&env);

    escrow.set_deprecated(&true, &None);

    let deadline = env.ledger().timestamp() + 1_000;
    let res = escrow.try_lock_funds(&depositor, &1u64, &1_000i128, &deadline);
    assert_eq!(res, Err(Ok(Error::ContractDeprecated)));
}

/// `batch_lock_funds` must return `Error::ContractDeprecated` (code 34) when deprecated.
#[test]
fn test_batch_lock_funds_blocked_when_deprecated() {
    let env = Env::default();
    let (escrow, _admin, depositor, _token, _) = setup(&env);

    escrow.set_deprecated(&true, &None);

    let mut items = Vec::new(&env);
    items.push_back(LockFundsItem {
        bounty_id: 1,
        depositor: depositor.clone(),
        amount: 500,
        deadline: env.ledger().timestamp() + 1_000,
    });
    let res = escrow.try_batch_lock_funds(&items);
    assert_eq!(res, Err(Ok(Error::ContractDeprecated)));
}

/// `lock_funds_anonymous` must return `Error::ContractDeprecated` (code 34) when deprecated.
#[test]
fn test_lock_funds_anonymous_blocked_when_deprecated() {
    let env = Env::default();
    let (escrow, _admin, depositor, _token, _) = setup(&env);

    escrow.set_deprecated(&true, &None);

    let commitment = BytesN::from_array(&env, &[0u8; 32]);
    let deadline = env.ledger().timestamp() + 1_000;
    let res = escrow.try_lock_funds_anonymous(&depositor, &commitment, &1u64, &1_000i128, &deadline);
    assert_eq!(res, Err(Ok(Error::ContractDeprecated)));
}

// ============================================================================
// Existing escrows unaffected by deprecation
// ============================================================================

/// `release_funds` must succeed for an escrow that was locked before deprecation.
#[test]
fn test_release_funds_works_when_deprecated() {
    let env = Env::default();
    let (escrow, _admin, depositor, _token, _) = setup(&env);
    let contributor = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 1_000;

    escrow.lock_funds(&depositor, &1u64, &1_000i128, &deadline);
    escrow.set_deprecated(&true, &None);

    escrow.release_funds(&1u64, &contributor);

    let info = escrow.get_escrow_info(&1u64);
    assert_eq!(
        info.status,
        EscrowStatus::Released,
        "release must succeed even when contract is deprecated"
    );
}

/// `partial_release` must succeed for an escrow locked before deprecation.
#[test]
fn test_partial_release_works_when_deprecated() {
    let env = Env::default();
    let (escrow, _admin, depositor, _token, _) = setup(&env);
    let contributor = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 1_000;

    escrow.lock_funds(&depositor, &1u64, &2_000i128, &deadline);
    escrow.set_deprecated(&true, &None);

    // Partial release of half the amount — must not be blocked by deprecation.
    escrow.partial_release(&1u64, &contributor, &1_000i128);

    let info = escrow.get_escrow_info(&1u64);
    assert_eq!(
        info.remaining_amount, 1_000,
        "remaining amount should be halved after partial release"
    );
    assert_eq!(
        info.status,
        EscrowStatus::Locked,
        "escrow should still be Locked after partial release"
    );
}

/// `refund` must succeed for an escrow whose deadline has passed, even when deprecated.
#[test]
fn test_refund_works_when_deprecated() {
    let env = Env::default();
    // Set ledger time so we can use a past deadline.
    env.ledger().set_timestamp(10_000);
    let (escrow, _admin, depositor, _token, _) = setup(&env);

    let deadline = 9_999u64; // already in the past
    escrow.lock_funds(&depositor, &1u64, &1_000i128, &deadline);
    escrow.set_deprecated(&true, &None);

    escrow.refund(&1u64);

    let info = escrow.get_escrow_info(&1u64);
    assert_eq!(
        info.status,
        EscrowStatus::Refunded,
        "refund must succeed even when contract is deprecated"
    );
}

// ============================================================================
// View functions always available
// ============================================================================

/// `get_deprecation_status` must be callable regardless of deprecation state.
#[test]
fn test_get_deprecation_status_always_readable() {
    let env = Env::default();
    let (escrow, _admin, _depositor, _token, _) = setup(&env);

    // Before deprecation
    let before = escrow.get_deprecation_status();
    assert!(!before.deprecated);

    // After deprecation
    escrow.set_deprecated(&true, &None);
    let after = escrow.get_deprecation_status();
    assert!(after.deprecated);

    // After un-deprecation
    escrow.set_deprecated(&false, &None);
    let restored = escrow.get_deprecation_status();
    assert!(!restored.deprecated);
}

// ============================================================================
// Event emission
// ============================================================================

/// Every call to `set_deprecated` must emit a `DeprecationStateChanged` event.
#[test]
fn test_deprecation_event_emitted_on_set() {
    let env = Env::default();
    let (escrow, _admin, _depositor, _token, _) = setup(&env);

    let before = env.events().all().len();
    escrow.set_deprecated(&true, &None);
    let after_first = env.events().all().len();
    assert!(
        after_first > before,
        "a DeprecationStateChanged event must be emitted when deprecating"
    );

    escrow.set_deprecated(&false, &None);
    let after_second = env.events().all().len();
    assert!(
        after_second > after_first,
        "a DeprecationStateChanged event must be emitted when un-deprecating"
    );
}

/// Setting deprecation with a migration target must also emit an event.
#[test]
fn test_deprecation_event_emitted_with_migration_target() {
    let env = Env::default();
    let (escrow, _admin, _depositor, _token, _) = setup(&env);
    let target = Address::generate(&env);

    let before = env.events().all().len();
    escrow.set_deprecated(&true, &Some(target));
    let after = env.events().all().len();

    assert!(
        after > before,
        "event must be emitted when deprecating with a migration target"
    );
}

// ============================================================================
// Migration target query
// ============================================================================

/// `get_deprecation_status` must expose the migration target set by the admin.
#[test]
fn test_get_migration_target_returns_correct_address() {
    let env = Env::default();
    let (escrow, _admin, _depositor, _token, _) = setup(&env);
    let target = Address::generate(&env);

    escrow.set_deprecated(&true, &Some(target.clone()));

    let status = escrow.get_deprecation_status();
    assert_eq!(
        status.migration_target,
        Some(target),
        "get_deprecation_status must return the exact migration target address"
    );
}

/// When deprecated is false but a migration target was previously set, the
/// target should reflect whatever was last written (None in this case).
#[test]
fn test_migration_target_cleared_on_undeprecate_with_none() {
    let env = Env::default();
    let (escrow, _admin, _depositor, _token, _) = setup(&env);
    let target = Address::generate(&env);

    escrow.set_deprecated(&true, &Some(target));
    // Un-deprecate and explicitly clear the target.
    escrow.set_deprecated(&false, &None);

    let status = escrow.get_deprecation_status();
    assert!(!status.deprecated);
    assert!(
        status.migration_target.is_none(),
        "migration target must be None after un-deprecating with None"
    );
}
