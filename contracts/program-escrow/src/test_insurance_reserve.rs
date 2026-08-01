//! # Insurance Reserve Tests
//!
//! Tests for the insurance-reserve basis-point carve-out feature added to
//! `FeeConfig` (issue #1478).
//!
//! ## What is tested
//!
//! 1. **FeeConfig field** – `insurance_reserve_bps` is initialised to 0, survives
//!    round-trips through `update_fee_config`, and is validated against `MAX_FEE_RATE`.
//! 2. **Split invariant** – `reserve_share + recipient_share == total_fee` for every
//!    fee path (lock, single payout, batch payout).
//! 3. **Accumulation** – multiple operations accumulate in the reserve without
//!    leakage or double-counting.
//! 4. **Query** – `get_insurance_reserve_balance` returns the correct balance.
//! 5. **Withdraw** – `withdraw_insurance_reserve` requires admin auth, decrements
//!    the reserve, transfers tokens, emits an audit event, and rejects
//!    over-withdrawals.
//! 6. **Zero-bps passthrough** – when `insurance_reserve_bps == 0` the full fee
//!    goes to `fee_recipient` and the reserve stays at 0.

#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token, Address, Env, IntoVal, String, Symbol, TryFromVal, Val,
};

// ─────────────────────────────────────────────────────────────────────────────
// Test helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Spin up a contract, mint `initial_balance` tokens into it, and return
/// everything callers need.
struct TestEnv<'a> {
    env: Env,
    client: ProgramEscrowContractClient<'a>,
    admin: Address,
    fee_recipient: Address,
    token: token::Client<'a>,
    token_admin: token::StellarAssetClient<'a>,
    program_id: String,
}

impl<'a> TestEnv<'a> {
    fn new(initial_balance: i128) -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let fee_recipient = Address::generate(&env);
        let token_admin_addr = Address::generate(&env);

        let sac = env.register_stellar_asset_contract_v2(token_admin_addr.clone());
        let token_id = sac.address();
        let token = token::Client::new(&env, &token_id);
        let token_admin = token::StellarAssetClient::new(&env, &token_id);

        let program_id = String::from_str(&env, "test-prog");

        // Initialise contract + program
        client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
        client.publish_program(&program_id, &admin);

        // Mint tokens and lock them into the program
        if initial_balance > 0 {
            token_admin.mint(&client.address, &initial_balance);
            client.lock_program_funds(&initial_balance);
        }

        TestEnv {
            env,
            client,
            admin,
            fee_recipient,
            token,
            token_admin,
            program_id,
        }
    }

    /// Enable payout fees at `payout_rate_bps` with the given `reserve_bps`.
    fn enable_payout_fees(&self, payout_rate_bps: i128, reserve_bps: u32) {
        self.client.update_fee_config(
            &None,
            &Some(payout_rate_bps),
            &None,
            &None,
            &Some(self.fee_recipient.clone()),
            &Some(true),
            &Some(reserve_bps),
        );
    }

    /// Enable lock fees at `lock_rate_bps` with the given `reserve_bps`.
    fn enable_lock_fees(&self, lock_rate_bps: i128, reserve_bps: u32) {
        self.client.update_fee_config(
            &Some(lock_rate_bps),
            &None,
            &None,
            &None,
            &Some(self.fee_recipient.clone()),
            &Some(true),
            &Some(reserve_bps),
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. FeeConfig field initialisation and validation
// ─────────────────────────────────────────────────────────────────────────────

/// `insurance_reserve_bps` defaults to 0 after `init_program`.
#[test]
fn test_insurance_reserve_bps_defaults_to_zero() {
    let t = TestEnv::new(0);
    let cfg = t.client.get_fee_config();
    assert_eq!(cfg.insurance_reserve_bps, 0);
}

/// Setting `insurance_reserve_bps` persists through `get_fee_config`.
#[test]
fn test_update_fee_config_sets_insurance_reserve_bps() {
    let t = TestEnv::new(0);
    t.client.update_fee_config(
        &None,
        &None,
        &None,
        &None,
        &None,
        &None,
        &Some(500_u32),
    );
    let cfg = t.client.get_fee_config();
    assert_eq!(cfg.insurance_reserve_bps, 500);
}

/// `insurance_reserve_bps == MAX_FEE_RATE` (1 000) must be accepted.
#[test]
fn test_insurance_reserve_bps_at_max_fee_rate_accepted() {
    let t = TestEnv::new(0);
    let result = t.client.try_update_fee_config(
        &None,
        &None,
        &None,
        &None,
        &None,
        &None,
        &Some(MAX_FEE_RATE as u32),
    );
    assert!(result.is_ok());
    let cfg = t.client.get_fee_config();
    assert_eq!(cfg.insurance_reserve_bps, MAX_FEE_RATE as u32);
}

/// `insurance_reserve_bps > MAX_FEE_RATE` must be rejected with error 704.
#[test]
fn test_insurance_reserve_bps_above_max_rejected() {
    let t = TestEnv::new(0);
    let result = t.client.try_update_fee_config(
        &None,
        &None,
        &None,
        &None,
        &None,
        &None,
        &Some(MAX_FEE_RATE as u32 + 1),
    );
    assert!(result.is_err());
}

/// Updating other fee fields while `insurance_reserve_bps` is already set
/// does not reset it.
#[test]
fn test_insurance_reserve_bps_preserved_across_partial_updates() {
    let t = TestEnv::new(0);
    t.client.update_fee_config(
        &None,
        &None,
        &None,
        &None,
        &None,
        &None,
        &Some(200_u32),
    );
    // Now update only payout_fee_rate, leave insurance_reserve_bps as None
    t.client.update_fee_config(
        &None,
        &Some(100_i128),
        &None,
        &None,
        &None,
        &None,
        &None,
    );
    let cfg = t.client.get_fee_config();
    assert_eq!(cfg.insurance_reserve_bps, 200);
    assert_eq!(cfg.payout_fee_rate, 100);
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Split invariant: reserve_share + recipient_share == total_fee
// ─────────────────────────────────────────────────────────────────────────────

/// After a single payout with fees + carve-out:
///   token balances satisfy:  recipient + fee_recipient + reserve == gross
#[test]
fn test_single_payout_split_invariant_no_leakage() {
    let t = TestEnv::new(10_000);
    let recipient = Address::generate(&t.env);
    // 100 bps (1%) payout fee, 50% of that goes to reserve (5000 bps of fee)
    t.enable_payout_fees(100, 5_000);

    let gross = 1_000_i128;
    t.client.single_payout(&recipient, &gross, &None);

    let recipient_bal = t.token.balance(&recipient);
    let fee_recipient_bal = t.token.balance(&t.fee_recipient);
    let reserve_bal = t.client.get_insurance_reserve_balance();

    // recipient gets gross minus total fee; fee is split between recipient and reserve
    assert_eq!(
        recipient_bal + fee_recipient_bal + reserve_bal,
        gross,
        "total must equal gross — no value leakage"
    );
}

/// After a batch payout with fees + carve-out all token value is accounted for.
#[test]
fn test_batch_payout_split_invariant_no_leakage() {
    let t = TestEnv::new(10_000);
    // 100 bps fee, 3 000 bps of that into reserve
    t.enable_payout_fees(100, 3_000);

    let r1 = Address::generate(&t.env);
    let r2 = Address::generate(&t.env);
    let recipients = soroban_sdk::vec![&t.env, r1.clone(), r2.clone()];
    let gross1 = 1_000_i128;
    let gross2 = 2_000_i128;
    let amounts = soroban_sdk::vec![&t.env, gross1, gross2];

    t.client.batch_payout(&recipients, &amounts);

    let r1_bal = t.token.balance(&r1);
    let r2_bal = t.token.balance(&r2);
    let fee_bal = t.token.balance(&t.fee_recipient);
    let reserve_bal = t.client.get_insurance_reserve_balance();

    assert_eq!(
        r1_bal + r2_bal + fee_bal + reserve_bal,
        gross1 + gross2,
        "total across recipients + fee_recipient + reserve must equal sum of gross amounts"
    );
}

/// When `insurance_reserve_bps == 0` the full fee goes to `fee_recipient`.
/// Reserve stays at 0 — no silent accumulation.
#[test]
fn test_zero_reserve_bps_all_fee_to_recipient() {
    let t = TestEnv::new(10_000);
    // 1% payout fee, 0 reserve
    t.enable_payout_fees(100, 0);

    let recipient = Address::generate(&t.env);
    t.client.single_payout(&recipient, &1_000, &None);

    let reserve_bal = t.client.get_insurance_reserve_balance();
    assert_eq!(reserve_bal, 0, "reserve must be 0 when reserve_bps is 0");

    // fee_recipient must have received the full fee
    let fee = t.token.balance(&t.fee_recipient);
    assert!(fee > 0, "fee_recipient should have received the fee");
}

/// When fees are disabled no reserve accrues even if `insurance_reserve_bps > 0`.
#[test]
fn test_disabled_fees_no_reserve_accrual() {
    let t = TestEnv::new(10_000);
    t.client.update_fee_config(
        &None,
        &Some(100_i128),
        &None,
        &None,
        &Some(t.fee_recipient.clone()),
        &Some(false), // fees disabled
        &Some(500_u32),
    );

    let recipient = Address::generate(&t.env);
    t.client.single_payout(&recipient, &1_000, &None);

    assert_eq!(
        t.client.get_insurance_reserve_balance(),
        0,
        "reserve must not accrue when fee_enabled is false"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. Accumulation across multiple operations
// ─────────────────────────────────────────────────────────────────────────────

/// The reserve accumulates monotonically over successive payouts.
#[test]
fn test_reserve_accumulates_over_multiple_payouts() {
    let t = TestEnv::new(30_000);
    // 200 bps fee, 50% (5000 bps of fee) into reserve
    t.enable_payout_fees(200, 5_000);

    let recipient = Address::generate(&t.env);

    t.client.single_payout(&recipient, &1_000, &None);
    let r1 = t.client.get_insurance_reserve_balance();
    assert!(r1 > 0);

    t.client.single_payout(&recipient, &1_000, &None);
    let r2 = t.client.get_insurance_reserve_balance();
    assert!(r2 > r1, "reserve must grow after second payout");

    t.client.single_payout(&recipient, &1_000, &None);
    let r3 = t.client.get_insurance_reserve_balance();
    assert!(r3 > r2, "reserve must grow after third payout");
}

/// Recipient's balance does not include the reserve share — no double-pay.
#[test]
fn test_no_double_counting_recipient_vs_reserve() {
    let t = TestEnv::new(10_000);
    // 100 bps fee, 100% of fee into reserve
    t.enable_payout_fees(100, 10_000);

    let recipient = Address::generate(&t.env);
    let gross = 2_000_i128;
    t.client.single_payout(&recipient, &gross, &None);

    let recipient_bal = t.token.balance(&recipient);
    let reserve_bal = t.client.get_insurance_reserve_balance();

    // fee_recipient should get 0 since all fee is carved to reserve
    let fee_bal = t.token.balance(&t.fee_recipient);
    assert_eq!(fee_bal, 0, "all fee goes to reserve, not fee_recipient");
    assert_eq!(
        recipient_bal + reserve_bal,
        gross,
        "recipient + reserve == gross"
    );
    assert!(reserve_bal > 0, "reserve must have received the carve-out");
}

/// Lock fees also carve out to the reserve correctly.
#[test]
fn test_lock_fee_carveout_accrues_to_reserve() {
    let t = TestEnv::new(0); // start empty; we'll lock with fees enabled

    // Enable lock fees 100 bps, 50% reserve carve-out
    t.enable_lock_fees(100, 5_000);

    // Mint tokens to contract address then lock
    t.token_admin.mint(&t.client.address, &10_000);
    t.client.lock_program_funds(&10_000);

    let reserve = t.client.get_insurance_reserve_balance();
    // 100 bps of 10_000 = 100, ceil(100 * 5000 / 10000) = 50
    assert!(reserve > 0, "lock fee should carve out to reserve");
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. get_insurance_reserve_balance
// ─────────────────────────────────────────────────────────────────────────────

/// Balance starts at 0 on a fresh contract.
#[test]
fn test_get_insurance_reserve_balance_initial_zero() {
    let t = TestEnv::new(0);
    assert_eq!(t.client.get_insurance_reserve_balance(), 0);
}

/// Balance reflects exact reserve amount after known fee/split calculation.
#[test]
fn test_get_insurance_reserve_balance_exact_math() {
    let t = TestEnv::new(10_000);
    // 200 bps fee, 100% (10 000 bps) → all fee goes to reserve
    t.enable_payout_fees(200, 10_000);

    let recipient = Address::generate(&t.env);
    t.client.single_payout(&recipient, &1_000, &None);

    // total_fee = ceil(1000 * 200 / 10000) = ceil(20) = 20
    // reserve_share = ceil(20 * 10000 / 10000) = 20
    let reserve = t.client.get_insurance_reserve_balance();
    assert_eq!(reserve, 20, "reserve should be exactly 20");
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. withdraw_insurance_reserve
// ─────────────────────────────────────────────────────────────────────────────

/// Admin can withdraw exactly the reserve balance.
#[test]
fn test_withdraw_insurance_reserve_full_amount() {
    let t = TestEnv::new(10_000);
    t.enable_payout_fees(200, 10_000); // all fee → reserve

    let recipient = Address::generate(&t.env);
    t.client.single_payout(&recipient, &1_000, &None);

    let reserve = t.client.get_insurance_reserve_balance();
    assert!(reserve > 0);

    let target = Address::generate(&t.env);
    t.client.withdraw_insurance_reserve(&target, &reserve);

    assert_eq!(t.client.get_insurance_reserve_balance(), 0);
    assert_eq!(t.token.balance(&target), reserve);
}

/// Admin can do a partial withdrawal and the remainder stays in the reserve.
#[test]
fn test_withdraw_insurance_reserve_partial() {
    let t = TestEnv::new(10_000);
    t.enable_payout_fees(200, 10_000);

    let recipient = Address::generate(&t.env);
    t.client.single_payout(&recipient, &5_000, &None);

    let reserve = t.client.get_insurance_reserve_balance();
    assert!(reserve >= 2, "need at least 2 units for partial test");

    let partial = reserve / 2;
    let target = Address::generate(&t.env);
    t.client.withdraw_insurance_reserve(&target, &partial);

    assert_eq!(
        t.client.get_insurance_reserve_balance(),
        reserve - partial,
        "reserve should decrease by partial"
    );
    assert_eq!(t.token.balance(&target), partial);
}

/// Withdrawing more than the reserve balance panics.
#[test]
#[should_panic]
fn test_withdraw_insurance_reserve_exceeds_balance_panics() {
    let t = TestEnv::new(10_000);
    t.enable_payout_fees(200, 10_000);

    let recipient = Address::generate(&t.env);
    t.client.single_payout(&recipient, &1_000, &None);

    let reserve = t.client.get_insurance_reserve_balance();
    let target = Address::generate(&t.env);

    // Try to withdraw more than available
    t.client.withdraw_insurance_reserve(&target, &(reserve + 1));
}

/// Withdrawing amount = 0 is rejected.
#[test]
#[should_panic]
fn test_withdraw_insurance_reserve_zero_amount_panics() {
    let t = TestEnv::new(10_000);
    t.enable_payout_fees(200, 10_000);

    let recipient = Address::generate(&t.env);
    t.client.single_payout(&recipient, &1_000, &None);

    let target = Address::generate(&t.env);
    t.client.withdraw_insurance_reserve(&target, &0);
}

/// Withdrawing when reserve is empty panics.
#[test]
#[should_panic]
fn test_withdraw_insurance_reserve_empty_reserve_panics() {
    let t = TestEnv::new(10_000);
    // No reserve carve-out configured
    t.enable_payout_fees(100, 0);

    let recipient = Address::generate(&t.env);
    t.client.single_payout(&recipient, &1_000, &None);

    let target = Address::generate(&t.env);
    t.client.withdraw_insurance_reserve(&target, &1);
}

/// `withdraw_insurance_reserve` emits `InsuranceReserveWithdrawnEvent` with
/// correct fields.
#[test]
fn test_withdraw_insurance_reserve_emits_audit_event() {
    let t = TestEnv::new(10_000);
    t.enable_payout_fees(200, 10_000);

    let recipient = Address::generate(&t.env);
    t.client.single_payout(&recipient, &1_000, &None);

    let reserve = t.client.get_insurance_reserve_balance();
    let target = Address::generate(&t.env);

    let before_event_count = t.env.events().all().len();
    t.client.withdraw_insurance_reserve(&target, &reserve);

    let all_events = t.env.events().all();
    // Find the insurance reserve event
    let found = all_events
        .iter()
        .skip(before_event_count)
        .any(|(_, topics, data)| {
            if let Ok(topic_sym) = Symbol::try_from_val(&t.env, &topics.get(0).unwrap_or_default())
            {
                topic_sym == INSURANCE_RESERVE_WITHDRAWN
            } else {
                false
            }
        });

    assert!(found, "InsuranceReserveWithdrawnEvent must be emitted");
}

/// Non-admin cannot call `withdraw_insurance_reserve`.
#[test]
fn test_withdraw_insurance_reserve_non_admin_rejected() {
    let env = Env::default();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let attacker = Address::generate(&env);
    let token_admin_addr = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(token_admin_addr.clone());
    let token_id = sac.address();
    let token = token::Client::new(&env, &token_id);
    let token_admin = token::StellarAssetClient::new(&env, &token_id);

    // Setup with admin
    env.mock_all_auths();
    let program_id = String::from_str(&env, "test-prog");
    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    client.publish_program(&program_id, &admin);
    token_admin.mint(&client.address, &10_000);
    client.lock_program_funds(&10_000);

    // Enable fees with reserve carve-out
    client.update_fee_config(
        &None,
        &Some(200_i128),
        &None,
        &None,
        &Some(admin.clone()),
        &Some(true),
        &Some(10_000_u32),
    );

    let recipient = Address::generate(&env);
    client.single_payout(&recipient, &1_000, &None);

    let reserve = client.get_insurance_reserve_balance();
    assert!(reserve > 0);

    // Now try with attacker auth only
    env.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &attacker,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &client.address,
            fn_name: "withdraw_insurance_reserve",
            args: (attacker.clone(), reserve).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let result = client.try_withdraw_insurance_reserve(&attacker, &reserve);
    assert!(result.is_err(), "non-admin must be rejected");
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. Ceiling arithmetic correctness
// ─────────────────────────────────────────────────────────────────────────────

/// Ceiling division: reserve_share rounds up so recipient_share never exceeds
/// (total_fee - 1) when bps < BASIS_POINTS.
#[test]
fn test_ceiling_division_reserve_share() {
    let t = TestEnv::new(10_000);
    // 100 bps fee, 1 bps reserve → tiny reserve, mostly to recipient
    t.enable_payout_fees(100, 1);

    let recipient = Address::generate(&t.env);
    // gross = 10 000, fee = ceil(10000 * 100 / 10000) = 100
    // reserve = ceil(100 * 1 / 10000) = 1
    t.client.single_payout(&recipient, &10_000, &None);

    let reserve = t.client.get_insurance_reserve_balance();
    let fee_bal = t.token.balance(&t.fee_recipient);

    assert_eq!(reserve, 1, "reserve should be exactly 1 (ceiling of 0.01)");
    assert_eq!(fee_bal, 99, "fee_recipient gets total_fee - reserve_share");
}

/// When total_fee * bps is exactly divisible by BASIS_POINTS, ceiling ==
/// floor (no off-by-one).
#[test]
fn test_ceiling_division_exact_multiple() {
    let t = TestEnv::new(10_000);
    // 200 bps fee, 5000 bps of fee → exactly 50% to reserve
    t.enable_payout_fees(200, 5_000);

    let recipient = Address::generate(&t.env);
    // gross = 10_000, fee = 200, reserve = 200 * 5000 / 10000 = 100 exactly
    t.client.single_payout(&recipient, &10_000, &None);

    let reserve = t.client.get_insurance_reserve_balance();
    let fee_bal = t.token.balance(&t.fee_recipient);

    assert_eq!(reserve, 100, "reserve = 100 (exact division)");
    assert_eq!(fee_bal, 100, "fee_recipient = 100");
    assert_eq!(reserve + fee_bal, 200, "total fee preserved");
}

// ─────────────────────────────────────────────────────────────────────────────
// 7. Storage key isolation
// ─────────────────────────────────────────────────────────────────────────────

/// The insurance reserve is stored separately from `remaining_balance`.
/// Paying out does not reduce the reserve; locking does not inflate it without fees.
#[test]
fn test_reserve_storage_isolated_from_remaining_balance() {
    let t = TestEnv::new(10_000);
    t.enable_payout_fees(200, 10_000); // all fee to reserve

    let recipient = Address::generate(&t.env);
    t.client.single_payout(&recipient, &1_000, &None);

    let remaining = t.client.get_remaining_balance();
    let reserve = t.client.get_insurance_reserve_balance();

    // remaining_balance should reflect net payout reduction only
    // reserve should be the fee carve-out, not part of remaining_balance
    assert!(
        remaining + reserve < 10_000,
        "remaining + reserve < initial (fee went out of contract)"
    );
    assert!(reserve > 0);
    // They must be independent: reserve is not subtracted from remaining_balance
    assert_ne!(
        remaining,
        10_000 - 1_000,
        "remaining is gross minus fee, not just gross minus payout"
    );
}
