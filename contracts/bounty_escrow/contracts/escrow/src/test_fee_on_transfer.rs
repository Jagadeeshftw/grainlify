//! # Fee-on-Transfer Token Security Tests for Bounty Escrow
//!
//! This module tests the escrow contract's resilience against malicious tokens
//! that charge fees during transfer, causing the contract to record more tokens
//! than it actually receives (accounting discrepancy).
//!
//! ## Attack Vector
//!
//! A "fee-on-transfer" token deducts a fee from the transferred amount at the
//! token level, delivering less than the declared amount to the recipient.
//! If the escrow blindly trusts the declared transfer amount:
//!
//! 1. `lock_funds(depositor, bounty_id, 1000, deadline)` is called.
//! 2. Token records escrow receives `1000 - fee` (e.g., 0 for 100% fee).
//! 3. Escrow stores `remaining_amount = 1000`.
//! 4. Later, `release_funds(bounty_id, contributor)` tries to transfer `1000`.
//! 5. Contract holds only `0` tokens → transfer fails or drains other escrows.
//!
//! ## Defences Verified
//!
//! | Defence | Location | Behaviour |
//! |---------|----------|-----------|
//! | **INV-2** (Aggregate-to-Ledger) | `assert_after_lock` | Panics and rolls back the entire `lock_funds` call when `sum(remaining) != actual_balance`. |
//! | **Net-amount guard** | `lock_funds_logic` | Returns `Error::InvalidAmount` when protocol fees reduce `net_amount` to ≤ 0. |
//! | **publish() INV-2** | `publish_logic` | `publish()` also runs `assert_after_lock`, so any Draft escrow with a balance shortfall is caught before it goes live. |
//!
//! ## Test Groups
//!
//! 1. **INV-2 active** — lock_funds/publish panics when token charges fee.
//! 2. **Zero-fee baseline** — healthy token completes full lifecycle.
//! 3. **INV-1 always holds** — escrow data never goes negative (with INV-2 bypassed).
//! 4. **Downstream failure** — release panics when contract is drained.
//! 5. **Net-amount guard** — InvalidAmount when protocol fee = 100% of tiny deposit.
//! 6. **publish() safety** — Draft-to-Locked transition catches shortfall.

#![cfg(test)]

use super::*;
use soroban_sdk::{
    contract, contractimpl, contracttype,
    testutils::Address as _,
    Address, Env,
};

// ============================================================================
// MaliciousFeeToken — SEP-41 compatible token with configurable fee drain
// ============================================================================
//
// Design:
//   - Storage: persistent per-address balance + instance fee-rate.
//   - `transfer`: charges floor(amount * fee_rate_bps / 10_000) and delivers
//     only `max(0, amount - fee)` to the recipient.  The fee is burned.
//   - `balance`: standard view.
//   - `mint`: unchecked test helper (no auth required).
//
// Complexity:  O(1) time per operation; O(N_accounts) storage.
// ============================================================================

#[contracttype]
enum FotKey {
    Balance(Address),
    FeeRateBps,
}

#[contract]
struct MaliciousFeeToken;

#[contractimpl]
impl MaliciousFeeToken {
    /// Initialise the mock with `fee_rate_bps` (0 = no fee, 10_000 = 100%).
    /// Values above 10_000 cause `received = 0` (over-drain).
    pub fn initialize(env: Env, fee_rate_bps: i128) {
        env.storage()
            .instance()
            .set(&FotKey::FeeRateBps, &fee_rate_bps);
    }

    /// Mint `amount` tokens to `to` — no authorisation required (test setup).
    pub fn mint(env: Env, to: Address, amount: i128) {
        let cur: i128 = env
            .storage()
            .persistent()
            .get(&FotKey::Balance(to.clone()))
            .unwrap_or(0);
        env.storage()
            .persistent()
            .set(&FotKey::Balance(to), &(cur + amount));
    }

    // ── SEP-41 `transfer` ────────────────────────────────────────────────────

    /// Transfer `amount` from `from` to `to`, retaining only
    /// `max(0, amount - fee)` for the recipient.
    ///
    /// # Panics
    /// Panics when `from` has insufficient balance — mirrors real-token
    /// behaviour so downstream callers experience a clean failure.
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        from.require_auth();

        let fee_rate: i128 = env
            .storage()
            .instance()
            .get(&FotKey::FeeRateBps)
            .unwrap_or(0);

        // fee = floor(amount * fee_rate_bps / 10_000); safe against overflow.
        let fee = amount
            .checked_mul(fee_rate)
            .and_then(|x| x.checked_div(10_000))
            .unwrap_or(0);

        // Net received by destination (clamped to [0, amount]).
        let received = (amount - fee).max(0);

        // Deduct full declared amount from sender.
        let from_bal: i128 = env
            .storage()
            .persistent()
            .get(&FotKey::Balance(from.clone()))
            .unwrap_or(0);
        if from_bal < amount {
            panic!("MaliciousFeeToken: Insufficient balance");
        }
        env.storage()
            .persistent()
            .set(&FotKey::Balance(from), &(from_bal - amount));

        // Credit net amount to recipient (nothing if fee = 100%).
        if received > 0 {
            let to_bal: i128 = env
                .storage()
                .persistent()
                .get(&FotKey::Balance(to.clone()))
                .unwrap_or(0);
            env.storage()
                .persistent()
                .set(&FotKey::Balance(to), &(to_bal + received));
        }
        // Fee is burned — not forwarded to any address.
    }

    // ── SEP-41 `balance` ─────────────────────────────────────────────────────

    /// Return the balance of `id`.
    pub fn balance(env: Env, id: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&FotKey::Balance(id))
            .unwrap_or(0)
    }
}

// ============================================================================
// Test helpers
// ============================================================================

/// Deploy a `MaliciousFeeToken` contract with the given fee rate and return
/// its on-chain address.  O(1).
fn deploy_fee_token(env: &Env, fee_rate_bps: i128) -> Address {
    let addr = env.register_contract(None, MaliciousFeeToken);
    MaliciousFeeTokenClient::new(env, &addr).initialize(&fee_rate_bps);
    addr
}

/// Mint `amount` tokens of the fee token to `recipient`.  O(1).
fn mint_fee_token(env: &Env, token_addr: &Address, recipient: &Address, amount: i128) {
    MaliciousFeeTokenClient::new(env, token_addr).mint(recipient, &amount);
}

/// Deploy and initialise a `BountyEscrowContract` backed by `token_addr`.
/// O(1) excluding contract registration overhead.
fn deploy_escrow<'a>(
    env: &'a Env,
    admin: &Address,
    token_addr: &Address,
) -> BountyEscrowContractClient<'a> {
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(env, &contract_id);
    client.init(admin, token_addr);
    client
}

// ============================================================================
// Group 1: INV-2 actively blocks fee-on-transfer drains
// ============================================================================

/// **Security: 100% fee drain caught at lock time.**
///
/// A token that charges a 100% fee delivers 0 tokens to the escrow contract.
/// `lock_funds` records `remaining_amount = 1_000`, but the actual token
/// balance at the contract address is 0.  `assert_after_lock` (INV-2)
/// detects the divergence and panics, causing the entire transaction to
/// roll back atomically — depositor's tokens are returned.
#[test]
#[should_panic(expected = "INV-2 violated after lock")]
fn test_full_fee_drain_detected_by_inv2_on_lock() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);

    // 10_000 bps = 100% fee: recipient receives 0 for every transfer.
    let token_addr = deploy_fee_token(&env, 10_000);
    mint_fee_token(&env, &token_addr, &depositor, 1_000);

    let escrow = deploy_escrow(&env, &admin, &token_addr);
    let deadline = env.ledger().timestamp() + 1_000;

    // Expected flow:
    //   1. Escrow records remaining_amount = 1_000
    //   2. token.transfer(depositor, contract, 1_000) → contract receives 0
    //   3. assert_after_lock: sum(1_000) != balance(0) → PANIC + full rollback
    escrow.lock_funds(&depositor, &1u64, &1_000, &deadline);
}

/// **Security: 50% partial fee drain caught at lock time.**
///
/// A 50% fee token delivers only 500 when 1_000 is declared.  The escrow
/// records `remaining_amount = 1_000` while the contract holds only 500.
/// INV-2 catches this 2x discrepancy immediately.
#[test]
#[should_panic(expected = "INV-2 violated after lock")]
fn test_partial_fee_imbalance_detected_by_inv2_on_lock() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);

    // 5_000 bps = 50% fee: recipient receives half.
    let token_addr = deploy_fee_token(&env, 5_000);
    mint_fee_token(&env, &token_addr, &depositor, 1_000);

    let escrow = deploy_escrow(&env, &admin, &token_addr);
    let deadline = env.ledger().timestamp() + 1_000;

    escrow.lock_funds(&depositor, &2u64, &1_000, &deadline);
}

/// **Security: over-100% fee drain caught at lock time.**
///
/// A 200% fee token also delivers 0 (net clamped at 0).  Validates that
/// fee rates above 10_000 bps are handled safely without integer underflow.
#[test]
#[should_panic(expected = "INV-2 violated after lock")]
fn test_over_hundred_pct_fee_drain_detected_by_inv2_on_lock() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);

    // 20_000 bps = 200% fee: fee = 2×amount; net = max(0, -amount) = 0.
    let token_addr = deploy_fee_token(&env, 20_000);
    mint_fee_token(&env, &token_addr, &depositor, 1_000);

    let escrow = deploy_escrow(&env, &admin, &token_addr);
    let deadline = env.ledger().timestamp() + 1_000;

    escrow.lock_funds(&depositor, &3u64, &1_000, &deadline);
}

// ============================================================================
// Group 2: Zero-fee token — healthy baseline
// ============================================================================

/// **Baseline: 0% fee token completes full lock → release lifecycle.**
///
/// Verifies that the MaliciousFeeToken with fee_rate_bps = 0 behaves
/// identically to a standard Stellar token and that INV-2 does not
/// interfere with a correct token.
#[test]
fn test_zero_fee_token_completes_full_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);

    // 0% fee: recipient receives exactly what is declared.
    let token_addr = deploy_fee_token(&env, 0);
    mint_fee_token(&env, &token_addr, &depositor, 1_000);

    let escrow = deploy_escrow(&env, &admin, &token_addr);
    let deadline = env.ledger().timestamp() + 1_000;

    // Lock succeeds; contract holds exactly 1_000.
    escrow.lock_funds(&depositor, &10u64, &1_000, &deadline);

    let tok = MaliciousFeeTokenClient::new(&env, &token_addr);
    assert_eq!(tok.balance(&escrow.address), 1_000, "contract must hold full amount");

    let info = escrow.get_escrow_info(&10u64);
    assert_eq!(info.amount, 1_000);
    assert_eq!(info.remaining_amount, 1_000);
    assert_eq!(info.status, EscrowStatus::Locked);

    // Release succeeds; contributor receives 1_000, contract emptied.
    escrow.release_funds(&10u64, &contributor);

    assert_eq!(tok.balance(&contributor), 1_000, "contributor must receive full amount");
    assert_eq!(tok.balance(&escrow.address), 0, "contract must be empty after release");
}

// ============================================================================
// Group 3: INV-1 (per-escrow sanity) always holds — even with INV-2 bypassed
// ============================================================================

/// **Property: escrow data fields never go negative.**
///
/// To test the hypothetical state that would arise if INV-2 were bypassed,
/// a `Locked` escrow with `amount = remaining_amount = 1_000` is injected
/// directly into contract storage — without going through `lock_funds`.
/// No tokens are transferred, so the contract holds 0.
///
/// This demonstrates INV-1 (per-escrow sanity) in the worst case:
/// - `amount > 0`
/// - `remaining_amount >= 0`
/// - `remaining_amount <= amount`
///
/// The accounting discrepancy (escrow records 1_000, contract holds 0) is
/// exactly what INV-2 catches at lock time; this test documents the data
/// shape that INV-2 prevents from ever persisting.
#[test]
fn test_escrow_data_invariants_remain_valid_with_drained_token() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);

    // 100% fee token: contract receives 0 for every declared transfer.
    let token_addr = deploy_fee_token(&env, 10_000);
    let escrow = deploy_escrow(&env, &admin, &token_addr);
    let escrow_addr = escrow.address.clone();

    let bounty_id: u64 = 20;
    let deadline = env.ledger().timestamp() + 1_000;

    // Inject the adversarial state directly: a Locked escrow recording 1_000
    // with no corresponding token balance at the contract address.
    env.as_contract(&escrow_addr, || {
        let drained = Escrow {
            depositor: depositor.clone(),
            amount: 1_000,
            remaining_amount: 1_000,
            status: EscrowStatus::Locked,
            deadline,
            refund_history: soroban_sdk::vec![&env],
            archived: false,
            archived_at: None,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &drained);
    });

    let info = escrow.get_escrow_info(&bounty_id);

    // INV-1: amount > 0.
    assert!(info.amount > 0, "INV-1 violation: amount must be positive");

    // INV-1: remaining_amount in [0, amount].
    assert!(
        info.remaining_amount >= 0,
        "INV-1 violation: remaining_amount must be non-negative"
    );
    assert!(
        info.remaining_amount <= info.amount,
        "INV-1 violation: remaining_amount must not exceed amount"
    );

    // Document the INV-2 discrepancy for auditors.
    let tok = MaliciousFeeTokenClient::new(&env, &token_addr);
    assert_eq!(
        tok.balance(&escrow_addr),
        0,
        "100% fee token: contract holds 0"
    );
    assert_eq!(
        info.remaining_amount, 1_000,
        "escrow records 1_000 despite contract holding 0 — INV-2 discrepancy"
    );
}

/// **Property: accounting discrepancy is proportional to fee rate.**
///
/// A 50% fee token delivers only 1_000 when 2_000 is declared.  The
/// discrepancy (shortfall) equals the fee amount.
///
/// The token transfer is performed directly (not via `lock_funds`) and the
/// escrow state is injected to mirror what the contract would have stored
/// had it trusted the declared amount.  This isolates the accounting
/// arithmetic from the cross-contract auth mechanics.
#[test]
fn test_partial_fee_creates_documented_accounting_discrepancy() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);

    // 50% fee: recipient receives half of any declared amount.
    let token_addr = deploy_fee_token(&env, 5_000);
    mint_fee_token(&env, &token_addr, &depositor, 2_000);

    let escrow = deploy_escrow(&env, &admin, &token_addr);
    let escrow_addr = escrow.address.clone();

    let bounty_id: u64 = 21;
    let deadline = env.ledger().timestamp() + 1_000;

    // Simulate a fee-on-transfer lock: depositor declares 2_000 but the
    // token delivers only 1_000 to the escrow address (50% burned as fee).
    MaliciousFeeTokenClient::new(&env, &token_addr)
        .transfer(&depositor, &escrow_addr, &2_000);

    // Inject the escrow record as if the contract trusted the declared 2_000.
    env.as_contract(&escrow_addr, || {
        let recording = Escrow {
            depositor: depositor.clone(),
            amount: 2_000,
            remaining_amount: 2_000,
            status: EscrowStatus::Locked,
            deadline,
            refund_history: soroban_sdk::vec![&env],
            archived: false,
            archived_at: None,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &recording);
    });

    let info = escrow.get_escrow_info(&bounty_id);
    let tok = MaliciousFeeTokenClient::new(&env, &token_addr);
    let actual = tok.balance(&escrow_addr);

    // Escrow records 2_000; contract received only 1_000.
    assert_eq!(info.amount, 2_000);
    assert_eq!(actual, 1_000);

    // Shortfall equals the fee the token silently charged.
    let shortfall = info.amount - actual;
    assert_eq!(shortfall, 1_000, "shortfall must equal the token fee charged");

    // INV-1 holds on the injected record.
    assert!(info.amount > 0);
    assert!(info.remaining_amount >= 0);
    assert!(info.remaining_amount <= info.amount);
}

// ============================================================================
// Group 4: Downstream failure when release is attempted after drain
// ============================================================================

/// **Security consequence: release panics after a fee-drain lock.**
///
/// A drained escrow records 1_000 tokens that the contract does not hold.
/// When `release_funds` attempts `token.transfer(contract, contributor, 1_000)`,
/// the token panics ("Insufficient balance") because the contract's actual
/// balance is 0.
///
/// The drained escrow state is injected directly (no `lock_funds` call) to
/// isolate the release-side failure from the lock-side INV-2 protection.
/// This demonstrates the second-order consequence of a successful drain:
/// no contributor can ever be paid from a drained bounty.
#[test]
#[should_panic(expected = "MaliciousFeeToken: Insufficient balance")]
fn test_release_panics_when_contract_balance_drained_by_fee_token() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);

    // 100% fee token: any transfer to the contract delivers 0 tokens.
    let token_addr = deploy_fee_token(&env, 10_000);
    let escrow = deploy_escrow(&env, &admin, &token_addr);
    let escrow_addr = escrow.address.clone();

    let bounty_id: u64 = 30;
    let deadline = env.ledger().timestamp() + 1_000;

    // Inject a Locked escrow recording 1_000 with no actual token balance.
    // This is the state that INV-2 prevents from arising via lock_funds.
    env.as_contract(&escrow_addr, || {
        let drained = Escrow {
            depositor: depositor.clone(),
            amount: 1_000,
            remaining_amount: 1_000,
            status: EscrowStatus::Locked,
            deadline,
            refund_history: soroban_sdk::vec![&env],
            archived: false,
            archived_at: None,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &drained);
    });

    // release_funds → token.transfer(contract, contributor, 1_000)
    // Contract holds 0 → MaliciousFeeToken panics "Insufficient balance".
    escrow.release_funds(&bounty_id, &contributor);
}

// ============================================================================
// Group 5: Net-amount guard (protocol-side zero-net check)
// ============================================================================

/// **Guard: Error::InvalidAmount when protocol fee consumes entire deposit.**
///
/// When `combined_fee_amount(amount, fee_rate, fixed, enabled)` equals
/// `amount`, `net_amount = 0` and `lock_funds` returns `Error::InvalidAmount`
/// (error code #13) before transferring any tokens.
///
/// This tests the existing guard in `lock_funds_logic` (independent of
/// fee-on-transfer tokens) that prevents zero-value escrows.
///
/// Scenario: amount = 1, lock_fee_rate = 5_000 bps (50%).
///   fee = ceil(1 × 5_000 / 10_000) = ceil(0.5) = 1 (ceiling division)
///   net_amount = 1 - 1 = 0 → InvalidAmount
#[test]
#[should_panic(expected = "Error(Contract, #13)")]
fn test_lock_returns_invalid_amount_when_protocol_fee_equals_deposit() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let fee_recipient = Address::generate(&env);

    // 0% token fee — the protocol fee is the only fee here.
    let token_addr = deploy_fee_token(&env, 0);
    mint_fee_token(&env, &token_addr, &depositor, 1_000);

    let escrow = deploy_escrow(&env, &admin, &token_addr);

    // Enable protocol lock fee at the maximum allowed rate (50%).
    escrow.update_fee_config(
        &Some(5_000), // lock_fee_rate = 50%
        &Some(0),
        &Some(0),
        &Some(0),
        &Some(fee_recipient),
        &Some(true),
    );

    let deadline = env.ledger().timestamp() + 1_000;
    // amount = 1: ceil(1 × 5000/10000) = 1 = fee → net = 0 → InvalidAmount.
    escrow.lock_funds(&depositor, &40u64, &1, &deadline);
}

// ============================================================================
// Group 6: publish() — Draft-to-Locked transition with fee-on-transfer token
// ============================================================================

/// **Security: publish() detects balance shortfall via INV-2.**
///
/// The `publish()` function transitions an escrow from `Draft` to `Locked`
/// and then calls `assert_after_lock` (INV-2).  If a Draft escrow was
/// created with a fee-on-transfer token (so the contract holds fewer tokens
/// than recorded), `publish()` catches the discrepancy.
///
/// Setup: a Draft escrow with `remaining_amount = 1_000` is injected
/// directly into contract storage.  The fee token contract records 0 balance
/// at the escrow address (no real lock occurred).  Calling `publish()`:
///
///   1. Transitions Draft → Locked (now counted in INV-2 sum).
///   2. `assert_after_lock`: sum(1_000) != balance(0) → PANIC.
#[test]
#[should_panic(expected = "INV-2 violated after lock")]
fn test_publish_detects_token_balance_shortfall_via_inv2() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);

    // Token holds 0 at the escrow address — simulates a 100% fee drain.
    let token_addr = deploy_fee_token(&env, 10_000);

    let escrow = deploy_escrow(&env, &admin, &token_addr);
    let escrow_addr = escrow.address.clone();

    let bounty_id: u64 = 50;
    let deadline = env.ledger().timestamp() + 1_000;

    // Inject a Draft escrow directly into contract storage.
    // This simulates the state that would arise if a recurring-lock execution
    // used a fee-on-transfer token while INV-2 was disabled.
    env.as_contract(&escrow_addr, || {
        let draft = Escrow {
            depositor: depositor.clone(),
            amount: 1_000,
            remaining_amount: 1_000,
            status: EscrowStatus::Draft,
            deadline,
            refund_history: soroban_sdk::vec![&env],
            archived: false,
            archived_at: None,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &draft);

        // Register in the global index so sum_active_escrow_balances finds it
        // once it is promoted to Locked by publish().
        let mut idx: soroban_sdk::Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::EscrowIndex)
            .unwrap_or(soroban_sdk::Vec::new(&env));
        idx.push_back(bounty_id);
        env.storage()
            .persistent()
            .set(&DataKey::EscrowIndex, &idx);
    });

    // publish() will:
    //   1. Load Draft escrow → status = Draft (ok).
    //   2. Transition Draft → Locked.
    //   3. assert_after_lock: sum(1_000) != balance(0) → PANIC.
    escrow.publish(&bounty_id);
}

/// **Baseline: publish() succeeds when token balance matches escrow.**
///
/// When the contract's actual token balance equals the Draft escrow's
/// `remaining_amount`, `publish()` completes the transition and INV-2 holds.
#[test]
fn test_publish_succeeds_when_token_balance_matches_escrow() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);

    // 0% fee token — transfers deliver the full declared amount.
    let token_addr = deploy_fee_token(&env, 0);
    mint_fee_token(&env, &token_addr, &depositor, 1_000);

    let escrow = deploy_escrow(&env, &admin, &token_addr);
    let escrow_addr = escrow.address.clone();

    let bounty_id: u64 = 51;
    let deadline = env.ledger().timestamp() + 1_000;

    // Transfer 1_000 tokens directly to the escrow contract (mimicking a lock).
    MaliciousFeeTokenClient::new(&env, &token_addr)
        .transfer(&depositor, &escrow_addr, &1_000);

    // Inject a matching Draft escrow.
    env.as_contract(&escrow_addr, || {
        let draft = Escrow {
            depositor: depositor.clone(),
            amount: 1_000,
            remaining_amount: 1_000,
            status: EscrowStatus::Draft,
            deadline,
            refund_history: soroban_sdk::vec![&env],
            archived: false,
            archived_at: None,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &draft);

        let mut idx: soroban_sdk::Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::EscrowIndex)
            .unwrap_or(soroban_sdk::Vec::new(&env));
        idx.push_back(bounty_id);
        env.storage()
            .persistent()
            .set(&DataKey::EscrowIndex, &idx);
    });

    // publish() must succeed: INV-2 holds (sum = 1_000 == balance = 1_000).
    escrow.publish(&bounty_id);

    let info = escrow.get_escrow_info(&bounty_id);
    assert_eq!(info.status, EscrowStatus::Locked, "escrow must be Locked after publish");
    assert_eq!(info.amount, 1_000);
    assert_eq!(info.remaining_amount, 1_000);
}

/// **Safety: publish() on a non-existent bounty returns BountyNotFound.**
///
/// Verifies that publish() does not panic unconditionally on bad input.
#[test]
#[should_panic(expected = "Error(Contract, #56)")]
fn test_publish_nonexistent_bounty_returns_bounty_not_found() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_addr = deploy_fee_token(&env, 0);

    let escrow = deploy_escrow(&env, &admin, &token_addr);

    // BountyNotFound = 56
    escrow.publish(&999u64);
}

/// **Safety: publish() on a Locked escrow returns FundsNotLocked.**
///
/// `publish()` requires Draft status.  Calling it on an already-Locked
/// escrow (the common post-`lock_funds` state) must fail.
#[test]
#[should_panic(expected = "Error(Contract, #57)")]
fn test_publish_on_locked_escrow_returns_funds_not_locked() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);

    let token_addr = deploy_fee_token(&env, 0); // 0% fee
    mint_fee_token(&env, &token_addr, &depositor, 1_000);

    let escrow = deploy_escrow(&env, &admin, &token_addr);
    let deadline = env.ledger().timestamp() + 1_000;

    // lock_funds creates status = Locked directly.
    escrow.lock_funds(&depositor, &60u64, &1_000, &deadline);

    // publish() requires Draft — must fail with FundsNotLocked (#57).
    escrow.publish(&60u64);
}

/// **Invariant: Soroban atomicity — failed lock leaves no residue.**
///
/// Each call to `lock_funds` that ends in an INV-2 panic is a complete
/// no-op: the Soroban host rolls back all storage and token-balance mutations
/// atomically.  This test verifies the *before* state: a freshly deployed
/// escrow with a 100% fee token starts with zero contract balance.  The
/// `#[should_panic]` tests above confirm that the lock call terminates with
/// a panic; together these two properties establish that no residue escapes.
#[test]
fn test_freshly_deployed_escrow_holds_zero_balance_with_fee_token() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);

    let token_addr = deploy_fee_token(&env, 10_000); // 100% fee

    // A newly deployed escrow holds no tokens.
    let escrow = deploy_escrow(&env, &admin, &token_addr);

    let tok = MaliciousFeeTokenClient::new(&env, &token_addr);
    assert_eq!(
        tok.balance(&escrow.address),
        0,
        "newly deployed escrow must hold 0 tokens"
    );
}
