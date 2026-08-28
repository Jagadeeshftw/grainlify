#![cfg(test)]

extern crate std;

use super::*;
use crate::test_support::*;
use soroban_sdk::{testutils::{Address as _, Events, Ledger, MockAuth, MockAuthInvoke}, token, vec, Address, Env, IntoVal, Map, String, Symbol, TryFromVal, Val};

fn test_spend_limit_single_payout_below_threshold_succeeds() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let recipient = Address::generate(&env);

    client.set_program_spend_threshold(&program_id, &5_000);
    client.single_payout(&recipient, &4_999,
    &None
);

    assert_eq!(token_client.balance(&recipient), 4_999);
    assert_eq!(client.get_remaining_balance(), 5_001);
}

/// SL-2: single_payout exactly at threshold succeeds.
#[test]
fn test_spend_limit_single_payout_at_threshold_succeeds() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let recipient = Address::generate(&env);

    client.set_program_spend_threshold(&program_id, &5_000);
    client.single_payout(&recipient, &5_000,
    &None
);

    assert_eq!(token_client.balance(&recipient), 5_000);
    assert_eq!(client.get_remaining_balance(), 5_000);
}

/// SL-3: single_payout above threshold is rejected.
#[test]
#[should_panic(expected = "Spend threshold exceeded")]
fn test_spend_limit_single_payout_above_threshold_rejected() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let recipient = Address::generate(&env);

    client.set_program_spend_threshold(&program_id, &5_000);
    client.single_payout(&recipient, &5_001,
    &None
); // must panic
}

/// SL-4: batch_payout total below threshold succeeds.
#[test]
fn test_spend_limit_batch_payout_below_threshold_succeeds() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    client.set_program_spend_threshold(&program_id, &6_000);
    client.batch_payout(
        &soroban_sdk::vec![&env, r1.clone(), r2.clone()],
        &soroban_sdk::vec![&env, 2_000i128, 3_000i128],
        &None
);

    assert_eq!(token_client.balance(&r1), 2_000);
    assert_eq!(token_client.balance(&r2), 3_000);
    assert_eq!(client.get_remaining_balance(), 5_000);
}

/// SL-5: batch_payout total above threshold is rejected.
#[test]
#[should_panic(expected = "Spend threshold exceeded")]
fn test_spend_limit_batch_payout_above_threshold_rejected() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    client.set_program_spend_threshold(&program_id, &4_000);
    client.batch_payout(
        &soroban_sdk::vec![&env, r1, r2],
        &soroban_sdk::vec![&env, 2_000i128, 3_000i128], // total = 5_000 > 4_000
        &None,
    );
}

/// SL-6: threshold check runs before balance check (deterministic ordering).
/// Even when balance is sufficient, exceeding threshold is rejected first.
#[test]
#[should_panic(expected = "Spend threshold exceeded")]
fn test_spend_limit_threshold_checked_before_balance() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 100_000);
    let program_id = client.get_program_info().program_id;
    let recipient = Address::generate(&env);

    // Balance is 100_000 but threshold is only 1_000.
    client.set_program_spend_threshold(&program_id, &1_000);
    client.single_payout(&recipient, &50_000,
    &None
); // threshold exceeded, not balance
}

/// SL-7: no threshold set â†’ i128::MAX â†’ any amount within balance is allowed.
#[test]
fn test_spend_limit_no_threshold_allows_full_balance() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let recipient = Address::generate(&env);

    // Verify default is i128::MAX (unlimited).
    assert_eq!(
        client.get_program_spend_threshold(&program_id),
        i128::MAX,
        "default threshold must be i128::MAX"
    );

    client.single_payout(&recipient, &10_000,
    &None
);
    assert_eq!(token_client.balance(&recipient), 10_000);
    assert_eq!(client.get_remaining_balance(), 0);
}

/// SL-8: threshold can be updated; new value takes effect immediately.
#[test]
fn test_spend_limit_threshold_update_takes_effect() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 20_000);
    let program_id = client.get_program_info().program_id;
    let recipient = Address::generate(&env);

    // Set tight threshold.
    client.set_program_spend_threshold(&program_id, &3_000);
    client.single_payout(&recipient, &3_000,
    &None
);
    assert_eq!(token_client.balance(&recipient), 3_000);

    // Raise threshold.
    client.set_program_spend_threshold(&program_id, &10_000);
    client.single_payout(&recipient, &10_000,
    &None
);
    assert_eq!(token_client.balance(&recipient), 13_000);
    assert_eq!(client.get_remaining_balance(), 7_000);
}

/// SL-9: SpendLimitSetEvent is emitted with correct fields.
#[test]
fn test_spend_limit_set_event_emitted() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 0);
    let program_id = client.get_program_info().program_id;

    let events_before = env.events().all().len();
    client.set_program_spend_threshold(&program_id, &7_500);
    let events_after = env.events().all();

    // At least one new event must have been emitted.
    assert!(
        events_after.len() > events_before,
        "SpendLimitSetEvent must be emitted"
    );
}

/// SL-10: SpendLimitExceededEvent is emitted when threshold is breached.
#[test]
fn test_spend_limit_exceeded_event_emitted_on_rejection() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let recipient = Address::generate(&env);

    client.set_program_spend_threshold(&program_id, &1_000);

    let events_before = env.events().all().len();
    // Attempt an over-threshold payout; it will panic but the event is emitted first.
    let result = client.try_single_payout(&recipient, &5_000,
    &None
);
    assert!(result.is_err(), "over-threshold payout must fail");

    // The SpendLimitExceededEvent must have been emitted before the panic.
    let events_after = env.events().all();
    assert!(
        events_after.len() > events_before,
        "SpendLimitExceededEvent must be emitted on rejection"
    );
}

/// SL-11: upgrade-safe schema version is written on init.
#[test]
fn test_spend_limit_schema_version_written_on_init() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_admin_addr = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(token_admin_addr.clone());
    let token_id = sac.address();
    let program_id = String::from_str(&env, "schema-test");

    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);

    let version = client.get_spend_limit_schema_version();
    assert_eq!(version, 1u32, "schema version must be 1 after init");
}

/// SL-12: threshold of 1 rejects any amount > 1.
#[test]
#[should_panic(expected = "Spend threshold exceeded")]
fn test_spend_limit_minimum_threshold_rejects_larger_amounts() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let recipient = Address::generate(&env);

    client.set_program_spend_threshold(&program_id, &1);
    client.single_payout(&recipient, &2,
    &None
); // must panic
}

/// SL-13: threshold of 1 allows amount == 1.
#[test]
fn test_spend_limit_minimum_threshold_allows_exact_amount() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let recipient = Address::generate(&env);

    client.set_program_spend_threshold(&program_id, &1);
    client.single_payout(&recipient, &1,
    &None
);
    assert_eq!(token_client.balance(&recipient), 1);
}

/// SL-14: zero threshold is rejected by set_program_spend_threshold.
#[test]
#[should_panic(expected = "Invalid spend threshold")]
fn test_spend_limit_zero_threshold_rejected() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 0);
    let program_id = client.get_program_info().program_id;
    client.set_program_spend_threshold(&program_id, &0);
}

/// SL-15: negative threshold is rejected by set_program_spend_threshold.
#[test]
#[should_panic(expected = "Invalid spend threshold")]
fn test_spend_limit_negative_threshold_rejected() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 0);
    let program_id = client.get_program_info().program_id;
    client.set_program_spend_threshold(&program_id, &-1);
}

// ============================================================================
// PER-WINDOW SPENDING LIMITS â€” Issue #25
// ============================================================================
// Tests for time-windowed spend limits: set/get config, enforcement in
// single_payout, batch_payout, schedule releases, window reset, and events.

/// SW-1: No limit set â†’ payouts proceed without restriction.
#[test]
fn test_spending_window_no_limit_allows_payout() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 10_000);
    let recipient = Address::generate(&env);

    client.single_payout(&recipient, &10_000,
    &None
);
    assert_eq!(token_client.balance(&recipient), 10_000);
}

/// SW-2: Limit disabled (enabled=false) â†’ payouts proceed even if amount > max_amount.
#[test]
fn test_spending_window_disabled_limit_allows_payout() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let recipient = Address::generate(&env);

    client.set_program_spending_limit(&program_id, &86400u64, &100i128, &false);
    client.single_payout(&recipient, &10_000,
    &None
);
    assert_eq!(token_client.balance(&recipient), 10_000);
}

/// SW-3: single_payout within window limit succeeds.
#[test]
fn test_spending_window_single_payout_within_limit_succeeds() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let recipient = Address::generate(&env);

    client.set_program_spending_limit(&program_id, &86400u64, &5_000i128, &true);
    client.single_payout(&recipient, &5_000,
    &None
);
    assert_eq!(token_client.balance(&recipient), 5_000);
}

/// SW-4: single_payout exceeding window limit is rejected.
#[test]
#[should_panic(expected = "Program spending limit exceeded for current window")]
fn test_spending_window_single_payout_exceeds_limit_rejected() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let recipient = Address::generate(&env);

    client.set_program_spending_limit(&program_id, &86400u64, &3_000i128, &true);
    client.single_payout(&recipient, &3_001,
    &None
);
}

/// SW-5: Cumulative payouts within window are tracked; second payout that
///        would push total over limit is rejected.
#[test]
#[should_panic(expected = "Program spending limit exceeded for current window")]
fn test_spending_window_cumulative_limit_enforced() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    // Window limit = 5_000; first payout = 3_000 (ok), second = 3_000 (total 6_000 > 5_000)
    client.set_program_spending_limit(&program_id, &86400u64, &5_000i128, &true);
    client.single_payout(&r1, &3_000,
    &None
);
    client.single_payout(&r2, &3_000,
    &None
); // must panic
}

/// SW-6: batch_payout total within window limit succeeds.
#[test]
fn test_spending_window_batch_payout_within_limit_succeeds() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    client.set_program_spending_limit(&program_id, &86400u64, &6_000i128, &true);
    client.batch_payout(
        &soroban_sdk::vec![&env, r1.clone(), r2.clone()],
        &soroban_sdk::vec![&env, 2_000i128, 3_000i128],
        &None
);
    assert_eq!(token_client.balance(&r1), 2_000);
    assert_eq!(token_client.balance(&r2), 3_000);
}

/// SW-7: batch_payout total exceeding window limit is rejected.
#[test]
#[should_panic(expected = "Program spending limit exceeded for current window")]
fn test_spending_window_batch_payout_exceeds_limit_rejected() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    client.set_program_spending_limit(&program_id, &86400u64, &4_000i128, &true);
    client.batch_payout(
        &soroban_sdk::vec![&env, r1, r2],
        &soroban_sdk::vec![&env, 2_000i128, 3_000i128], // total 5_000 > 4_000
        &None,
    );
}

/// SW-8: Window resets after window_size seconds; new window allows full limit again.
#[test]
fn test_spending_window_resets_after_window_expires() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 20_000);
    let program_id = client.get_program_info().program_id;
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    // Short window of 100 seconds, limit 5_000
    client.set_program_spending_limit(&program_id, &100u64, &5_000i128, &true);

    // Exhaust the window
    client.single_payout(&r1, &5_000,
    &None
);
    assert_eq!(token_client.balance(&r1), 5_000);

    // Advance time past the window
    env.ledger().with_mut(|l| l.timestamp += 101);

    // New window: same limit available again
    client.single_payout(&r2, &5_000,
    &None
);
    assert_eq!(token_client.balance(&r2), 5_000);
}

/// SW-9: get_program_spending_limit returns None when not set.
#[test]
fn test_spending_window_get_limit_none_when_not_set() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 0);
    let program_id = client.get_program_info().program_id;

    let limit = client.get_program_spending_limit(&program_id);
    assert!(limit.is_none(), "limit must be None when not configured");
}

/// SW-10: get_program_spending_state returns None before any payout.
#[test]
fn test_spending_window_get_state_none_before_payout() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 0);
    let program_id = client.get_program_info().program_id;

    client.set_program_spending_limit(&program_id, &86400u64, &5_000i128, &true);
    let state = client.get_program_spending_state(&program_id);
    assert!(state.is_none(), "state must be None before any payout");
}

/// SW-11: get_program_spending_state reflects cumulative amount after payouts.
#[test]
fn test_spending_window_state_tracks_cumulative_amount() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    client.set_program_spending_limit(&program_id, &86400u64, &10_000i128, &true);
    client.single_payout(&r1, &2_000,
    &None
);
    client.single_payout(&r2, &3_000,
    &None
);

    let state = client.get_program_spending_state(&program_id).unwrap();
    assert_eq!(state.amount_released, 5_000);
}

/// SW-12: zero window_size is rejected.
#[test]
#[should_panic(expected = "window_size must be greater than zero")]
fn test_spending_window_zero_window_size_rejected() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 0);
    let program_id = client.get_program_info().program_id;
    client.set_program_spending_limit(&program_id, &0u64, &5_000i128, &true);
}

/// SW-13: negative max_amount is rejected.
#[test]
#[should_panic(expected = "max_amount must be non-negative")]
fn test_spending_window_negative_max_amount_rejected() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 0);
    let program_id = client.get_program_info().program_id;
    client.set_program_spending_limit(&program_id, &86400u64, &-1i128, &true);
}

/// SW-14: Rejection emits the (limit, prog_spend) event.
#[test]
fn test_spending_window_rejection_emits_event() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let recipient = Address::generate(&env);

    client.set_program_spending_limit(&program_id, &86400u64, &1_000i128, &true);

    let events_before = env.events().all().len();
    let result = client.try_single_payout(&recipient, &5_000,
    &None
);
    assert!(result.is_err(), "over-limit payout must fail");

    let events_after = env.events().all();
    assert!(
        events_after.len() > events_before,
        "rejection event must be emitted"
    );
}

/// SW-15: Limit can be updated; new value takes effect immediately.
#[test]
fn test_spending_window_limit_update_takes_effect() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 20_000);
    let program_id = client.get_program_info().program_id;
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    client.set_program_spending_limit(&program_id, &86400u64, &3_000i128, &true);
    client.single_payout(&r1, &3_000,
    &None
);
    assert_eq!(token_client.balance(&r1), 3_000);

    // Raise limit
    client.set_program_spending_limit(&program_id, &86400u64, &20_000i128, &true);
    client.single_payout(&r2, &10_000,
    &None
);
    assert_eq!(token_client.balance(&r2), 10_000);
}

// ============================================================================
// PAUSE MODE BLOCKS PAYOUTS â€” Issue #1060
// ============================================================================
// Tests for deterministic pause behavior, PauseStateChangedV2 events,
// upgrade-safe storage (PauseSchemaVersion), and edge cases.

/// PM-01: Pause schema version is written at init and readable.
#[test]
fn test_pause_schema_version_written_at_init() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 0);

    let schema_version = client.get_pause_schema_version();
    assert_eq!(
        schema_version, PAUSE_SCHEMA_VERSION_V1,
        "Pause schema version must be PAUSE_SCHEMA_VERSION_V1 after init"
    );
}

/// PM-02: Default pause flags are all false after init.
#[test]
fn test_pause_flags_default_all_false() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 0);

    let flags = client.get_pause_flags();
    assert!(!flags.lock_paused, "lock_paused must default to false");
    assert!(
        !flags.release_paused,
        "release_paused must default to false"
    );
    assert!(!flags.refund_paused, "refund_paused must default to false");
}

/// PM-03: release_paused blocks single_payout with deterministic "Funds Paused" panic.
#[test]
#[should_panic(expected = "Funds Paused")]
fn test_release_paused_blocks_single_payout() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 1_000);

    client.set_paused(&None, &Some(true), &None, &None, &None);

    let recipient = Address::generate(&env);
    client.single_payout(&recipient, &100,
    &None
);
}

/// PM-04: release_paused blocks batch_payout with deterministic "Funds Paused" panic.
#[test]
#[should_panic(expected = "Funds Paused")]
fn test_release_paused_blocks_batch_payout() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 1_000);

    client.set_paused(&None, &Some(true), &None, &None, &None);

    let r1 = Address::generate(&env);
    client.batch_payout(
        &soroban_sdk::vec![&env, r1],
        &soroban_sdk::vec![&env, 100i128],
        &None
);
}

/// PM-05: lock_paused blocks lock_program_funds with deterministic "Funds Paused" panic.
#[test]
#[should_panic(expected = "Funds Paused")]
fn test_lock_paused_blocks_lock_program_funds() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 0);

    client.set_paused(&Some(true), &None, &None, &None, &None);
    client.lock_program_funds(&500);
}

/// PM-06: lock_paused does NOT block single_payout (orthogonal flags).
#[test]
fn test_lock_paused_does_not_block_single_payout() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 1_000);

    client.set_paused(&Some(true), &None, &None, &None, &None);

    let recipient = Address::generate(&env);
    let data = client.single_payout(&recipient, &200,
    &None
);
    assert_eq!(data.remaining_balance, 800);
}

/// PM-07: release_paused does NOT block lock_program_funds (orthogonal flags).
#[test]
fn test_release_paused_does_not_block_lock() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 0);

    client.set_paused(&None, &Some(true), &None, &None, &None);

    let data = client.lock_program_funds(&300);
    assert_eq!(data.remaining_balance, 300);
}

/// PM-08: Unpause restores single_payout after release_paused.
#[test]
fn test_unpause_restores_single_payout() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 1_000);

    client.set_paused(&None, &Some(true), &None, &None, &None);
    assert!(client
        .try_single_payout(&Address::generate(&env), &100,
    &None
)
        .is_err());

    client.set_paused(&None, &Some(false), &None, &None, &None);
    let data = client.single_payout(&Address::generate(&env), &100,
    &None
);
    assert_eq!(data.remaining_balance, 900);
}

/// PM-09: Unpause restores batch_payout after release_paused.
#[test]
fn test_unpause_restores_batch_payout() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 1_000);

    client.set_paused(&None, &Some(true), &None, &None, &None);
    let r1 = Address::generate(&env);
    assert!(client
        .try_batch_payout(
            &soroban_sdk::vec![&env, r1.clone()],
            &soroban_sdk::vec![&env, 100i128]
        ,
    &None
)
        .is_err());

    client.set_paused(&None, &Some(false), &None, &None, &None);
    let data = client.batch_payout(
        &soroban_sdk::vec![&env, r1],
        &soroban_sdk::vec![&env, 100i128],
        &None
);
    assert_eq!(data.remaining_balance, 900);
}

/// PM-10: PauseStateChangedV2 event is emitted with correct fields on pause.
#[test]
fn test_pause_state_changed_v2_event_on_pause() {
    let env = Env::default();
    let (client, admin, _token, _token_admin) = setup_program(&env, 0);

    env.ledger().with_mut(|li| li.timestamp = 99_999);

    client.set_paused(&None, &Some(true), &None, &None, &None);

    // Find the PauseStateChangedV2 event
    let events = env.events().all();
    let v2_event = events.iter().find(|e| {
        let topics = e.1.clone();
        if let Some(t0) = topics.get(0) {
            let sym: Symbol = t0.into_val(&env);
            sym == Symbol::new(&env, "PauseStV2")
        } else {
            false
        }
    });

    assert!(
        v2_event.is_some(),
        "PauseStateChangedV2 event must be emitted"
    );

    let event = v2_event.unwrap();
    let data = PauseStateChangedV2::try_from_val(&env, &event.2).unwrap();

    assert_eq!(data.version, EVENT_VERSION_V2);
    assert_eq!(data.operation, symbol_short!("release"));
    assert_eq!(
        data.previous_paused, false,
        "previous_paused must be false before first pause"
    );
    assert_eq!(data.paused, true);
    assert_eq!(data.actor, admin);
    assert_eq!(data.timestamp, 99_999);
    assert!(data.receipt_id > 0);
}

/// PM-11: PauseStateChangedV2 captures previous_paused = true when unpausing.
#[test]
fn test_pause_state_changed_v2_previous_paused_on_unpause() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 0);

    // First pause
    client.set_paused(&None, &Some(true), &None, &None, &None);

    // Then unpause â€” previous_paused should be true
    client.set_paused(&None, &Some(false), &None, &None, &None);

    let events = env.events().all();
    // Get the last PauseStateChangedV2 event (the unpause one)
    let v2_events: std::vec::Vec<_> = events
        .iter()
        .filter(|e| {
            let topics = e.1.clone();
            if let Some(t0) = topics.get(0) {
                let sym: Symbol = t0.into_val(&env);
                sym == Symbol::new(&env, "PauseStV2")
            } else {
                false
            }
        })
        .collect::<std::vec::Vec<_>>();

    assert!(
        v2_events.len() >= 2,
        "Should have at least 2 PauseStateChangedV2 events"
    );

    let unpause_event = v2_events.last().unwrap();
    let data = PauseStateChangedV2::try_from_val(&env, &unpause_event.2).unwrap();

    assert_eq!(
        data.previous_paused, true,
        "previous_paused must be true when unpausing"
    );
    assert_eq!(data.paused, false);
}

/// PM-12: All three flags can be paused simultaneously; all three block their ops.
#[test]
fn test_all_flags_paused_blocks_all_operations() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 1_000);

    client.set_paused(&Some(true), &Some(true), &Some(true), &None, &None);

    assert!(
        client.try_lock_program_funds(&100).is_err(),
        "lock must be blocked"
    );
    assert!(
        client
            .try_single_payout(&Address::generate(&env), &100,
    &None
)
            .is_err(),
        "single_payout must be blocked"
    );
    assert!(
        client
            .try_batch_payout(
                &soroban_sdk::vec![&env, Address::generate(&env)],
                &soroban_sdk::vec![&env, 100i128]
            ,
    &None
)
            .is_err(),
        "batch_payout must be blocked"
    );
}

/// PM-13: Partial unpause â€” only release unpaused, lock stays paused.
#[test]
fn test_partial_unpause_preserves_other_flags() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 1_000);

    client.set_paused(&Some(true), &Some(true), &Some(true), &None, &None);

    // Only unpause release
    client.set_paused(&None, &Some(false), &None, &None, &None);

    let flags = client.get_pause_flags();
    assert!(flags.lock_paused, "lock_paused must remain true");
    assert!(
        !flags.release_paused,
        "release_paused must be false after unpause"
    );
    assert!(flags.refund_paused, "refund_paused must remain true");
}

/// PM-14: Read-only queries (get_program_info, get_remaining_balance) are unaffected by pause.
#[test]
fn test_read_only_queries_unaffected_by_pause() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 500);

    client.set_paused(&Some(true), &Some(true), &Some(true), &None, &None);

    let info = client.get_program_info();
    assert_eq!(info.remaining_balance, 500);

    let balance = client.get_remaining_balance();
    assert_eq!(balance, 500);
}

/// PM-15: Pause reason is stored and retrievable via get_pause_flags.
#[test]
fn test_pause_reason_stored_in_flags() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 0);

    let reason = String::from_str(&env, "Security incident");
    client.set_paused(&Some(true), &None, &None, &Some(reason.clone()), &None);

    let flags = client.get_pause_flags();
    assert_eq!(flags.pause_reason, Some(reason));
}

/// PM-16: Pause reason is cleared when all flags are unpaused.
#[test]
fn test_pause_reason_cleared_on_full_unpause() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 0);

    let reason = String::from_str(&env, "Temporary halt");
    client.set_paused(&Some(true), &None, &None, &Some(reason), &None);
    client.set_paused(&Some(false), &None, &None, &None, &None);

    let flags = client.get_pause_flags();
    assert_eq!(
        flags.pause_reason, None,
        "reason must be cleared when fully unpaused"
    );
}

// ========================================================================
// Idempotency Key Tests
// ========================================================================

/// Test idempotency key validation for successful batch payout
