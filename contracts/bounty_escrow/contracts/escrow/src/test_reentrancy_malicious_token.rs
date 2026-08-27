//! # Malicious-Token Reentrancy Regression Coverage
//!
//! `test_reentrancy_guard.rs` exercises the guard module and CEI ordering using a
//! well-behaved Stellar Asset Contract. It never actually attempts to re-enter the
//! escrow contract — it only proves the guard is released between independent,
//! sequential calls.
//!
//! This module closes that gap. It deploys `ReentrantToken`, a SEP-41-shaped mock
//! whose `transfer` entry-point — the exact callback boundary a real attacker
//! controls when the escrow contract is configured with a malicious/upgradeable
//! token — synchronously calls back into the escrow contract before returning.
//! Every entry-point listed as "must be wrapped with `acquire` / `release`" in
//! `reentrancy_guard`'s module documentation is attacked here at least once:
//! during deposit (`lock_funds`, `lock_funds_anonymous`, `batch_lock_funds`),
//! payout (`release_funds`, `partial_release`, `claim`, `release_with_capability`,
//! `batch_release_funds`), refund (`refund`, `refund_resolved`,
//! `refund_with_capability`), and the admin path (`emergency_withdraw`).
//!
//! ## Threat model / design decision
//!
//! The guard is **global to the contract instance** (a single `DataKey::ReentrancyGuard`
//! flag shared by every mutating entry-point, see `reentrancy_guard.rs`), not scoped
//! per-operation or per-capability. That is a deliberate simplicity/safety trade-off:
//! a per-operation guard would still need to reason about cross-function attacks (e.g.
//! `release_funds`'s token callback re-entering `refund`), so a single instance-wide
//! flag is the smallest primitive that is provably safe against every escrow ↔ token
//! callback boundary, at the cost of serializing all mutating calls (no legitimate
//! reentrancy is possible, cross-contract or same-function). Every test below submits
//! its "attack" call through `try_*` — the client entry-point a real caller would use —
//! so the assertions match what actually happens on-chain: the outer call fails
//! deterministically and every mutation performed after `acquire()` (including the
//! caller's own state effects) rolls back atomically with it.
//!
//! ### A verified finding: which layer actually stops the attack
//!
//! Every attack here has the shape escrow → token → escrow: the malicious token's
//! `transfer` calls back into the *same* escrow contract instance that invoked it.
//! Soroban's host itself refuses to re-enter a contract instance already on the
//! call stack ("Contract re-entry is not allowed", surfaced as
//! `Error(Context, InvalidAction)`) — a check that runs *before* the reentrant
//! call's Rust body, including `reentrancy_guard::acquire`, ever executes. So for
//! this exact attack shape, the host's own protection is the layer a real attacker
//! hits first; see Group 5 below for the verified panic text. This contract's own
//! `reentrancy_guard` is not made redundant by that — it is exercised directly
//! (bypassing the host's cross-contract check entirely) by the unit tests in
//! `test_reentrancy_guard.rs`, and remains the layer that matters if the host
//! check is ever weakened, bypassed by an SDK change, or if the contract is later
//! composed behind a proxy/router that would call back through a *different*
//! contract address than the one already on the stack.
//!
//! ## Attack harness
//!
//! `ReentrantToken` is a minimal balance-tracked token (mirrors `MaliciousFeeToken` in
//! `test_fee_on_transfer.rs`) that can be "armed" with a `ReentryAction` describing
//! which escrow entry-point to call back into, and with what arguments, from inside
//! `transfer`. When unarmed (`ReentryAction::None`) it behaves like an ordinary
//! zero-fee token, so the same harness also documents the non-reentrant baseline:
//! the guard must not interfere with legitimate calls.
//!
//! ## Test groups
//!
//! 1. **Deposit boundary** — `lock_funds`, `lock_funds_anonymous`, `batch_lock_funds`.
//! 2. **Payout boundary** — `release_funds`, `partial_release`, `claim`,
//!    `release_with_capability`, `batch_release_funds`.
//! 3. **Refund boundary** — `refund`, `refund_resolved`, `refund_with_capability`.
//! 4. **Admin boundary** — `emergency_withdraw`.
//! 5. **Deterministic panic message** — a subset re-run with the direct (non-`try_`)
//!    client to pin the exact panic text observed (Soroban's own host-level
//!    reentrancy check — see "A verified finding" above).
//! 6. **Non-reentrant baseline** — the same malicious-shaped token, unarmed, must not
//!    change behavior for a full lock → release → refund lifecycle.

#![cfg(test)]

use super::*;
use soroban_sdk::{
    contract, contractimpl, contracttype,
    testutils::{Address as _, Ledger as _},
    vec, Address, BytesN, Env, Vec,
};

// ============================================================================
// ReentrantToken — SEP-41-shaped mock that calls back into the escrow contract
// from inside `transfer`.
// ============================================================================
//
// Design:
//   - Storage: persistent per-address balance (0% fee — isolates the reentrancy
//     assertion from the fee-on-transfer accounting already covered in
//     `test_fee_on_transfer.rs`).
//   - `configure_attack`: arms the token with the escrow address and the exact
//     entry-point + arguments to call back into during the next `transfer`.
//   - `transfer`: performs the normal balance move, then — if armed — invokes the
//     configured escrow entry-point via `BountyEscrowContractClient`. Soroban itself
//     refuses to re-enter the escrow contract instance while it is already on the
//     call stack (see the module doc above), so the callback can never recurse
//     further: it fails before any of its own logic — including
//     `reentrancy_guard::acquire` — runs.
//
// Complexity: O(1) per operation; O(N_accounts) storage.
// ============================================================================

#[contracttype]
enum RtKey {
    Balance(Address),
    Escrow,
    Action,
}

/// Which guarded escrow entry-point (if any) `transfer` should call back into,
/// and with what arguments. Unused fields for a given variant carry harmless
/// placeholder values supplied by the test.
#[contracttype]
#[derive(Clone)]
pub enum ReentryAction {
    /// Unarmed — `transfer` behaves like a plain token.
    None,
    LockFunds(ReentryParams),
    LockFundsAnonymous(ReentryParams),
    ReleaseFunds(ReentryParams),
    PartialRelease(ReentryParams),
    Claim(ReentryParams),
    ReleaseWithCapability(ReentryParams),
    Refund(ReentryParams),
    RefundResolved(ReentryParams),
    RefundWithCapability(ReentryParams),
    EmergencyWithdraw(ReentryParams),
    BatchLockFunds(ReentryParams),
    BatchReleaseFunds(ReentryParams),
}

/// Bag of arguments for the re-entrant callback. Only the fields relevant to the
/// chosen `ReentryAction` variant are read.
#[contracttype]
#[derive(Clone)]
pub struct ReentryParams {
    pub addr_a: Address,
    pub addr_b: Address,
    pub bytes_a: BytesN<32>,
    pub bounty_id: u64,
    pub amount: i128,
    pub deadline: u64,
    pub lock_items: Vec<LockFundsItem>,
    pub release_items: Vec<ReleaseFundsItem>,
}

#[contract]
struct ReentrantToken;

#[contractimpl]
impl ReentrantToken {
    /// Arm (or disarm, with `ReentryAction::None`) the callback attempted from
    /// inside the next `transfer`.
    pub fn configure_attack(env: Env, escrow: Address, action: ReentryAction) {
        env.storage().instance().set(&RtKey::Escrow, &escrow);
        env.storage().instance().set(&RtKey::Action, &action);
    }

    /// Mint `amount` tokens to `to` — no authorization required (test setup).
    pub fn mint(env: Env, to: Address, amount: i128) {
        let cur: i128 = env
            .storage()
            .persistent()
            .get(&RtKey::Balance(to.clone()))
            .unwrap_or(0);
        env.storage()
            .persistent()
            .set(&RtKey::Balance(to), &(cur + amount));
    }

    // ── SEP-41 `transfer` — the attacker-controlled callback boundary ─────────

    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        from.require_auth();

        let from_bal: i128 = env
            .storage()
            .persistent()
            .get(&RtKey::Balance(from.clone()))
            .unwrap_or(0);
        if from_bal < amount {
            panic!("ReentrantToken: Insufficient balance");
        }
        env.storage()
            .persistent()
            .set(&RtKey::Balance(from), &(from_bal - amount));

        let to_bal: i128 = env
            .storage()
            .persistent()
            .get(&RtKey::Balance(to.clone()))
            .unwrap_or(0);
        env.storage()
            .persistent()
            .set(&RtKey::Balance(to), &(to_bal + amount));

        // Attempt the configured reentrant callback, if armed. This runs
        // synchronously, inside the token's own execution — exactly the
        // window a malicious or compromised token contract controls.
        let action: ReentryAction = env
            .storage()
            .instance()
            .get(&RtKey::Action)
            .unwrap_or(ReentryAction::None);

        if let ReentryAction::None = action {
            return;
        }

        let escrow_addr: Address = env.storage().instance().get(&RtKey::Escrow).unwrap();
        let client = BountyEscrowContractClient::new(&env, &escrow_addr);

        match action {
            ReentryAction::None => unreachable!(),
            ReentryAction::LockFunds(p) => {
                client.lock_funds(&p.addr_a, &p.bounty_id, &p.amount, &p.deadline);
            }
            ReentryAction::LockFundsAnonymous(p) => {
                client.lock_funds_anonymous(
                    &p.addr_a,
                    &p.bytes_a,
                    &p.bounty_id,
                    &p.amount,
                    &p.deadline,
                );
            }
            ReentryAction::ReleaseFunds(p) => {
                client.release_funds(&p.bounty_id, &p.addr_a);
            }
            ReentryAction::PartialRelease(p) => {
                client.partial_release(&p.bounty_id, &p.addr_a, &p.amount);
            }
            ReentryAction::Claim(p) => {
                client.claim(&p.bounty_id);
            }
            ReentryAction::ReleaseWithCapability(p) => {
                client.release_with_capability(
                    &p.bounty_id,
                    &p.addr_a,
                    &p.amount,
                    &p.addr_b,
                    &p.bytes_a,
                );
            }
            ReentryAction::Refund(p) => {
                client.refund(&p.bounty_id);
            }
            ReentryAction::RefundResolved(p) => {
                client.refund_resolved(&p.bounty_id, &p.addr_a);
            }
            ReentryAction::RefundWithCapability(p) => {
                client.refund_with_capability(&p.bounty_id, &p.amount, &p.addr_b, &p.bytes_a);
            }
            ReentryAction::EmergencyWithdraw(p) => {
                client.emergency_withdraw(&p.addr_a);
            }
            ReentryAction::BatchLockFunds(p) => {
                client.batch_lock_funds(&p.lock_items);
            }
            ReentryAction::BatchReleaseFunds(p) => {
                client.batch_release_funds(&p.release_items);
            }
        }
    }

    // ── SEP-41 `balance` ────────────────────────────────────────────────────

    pub fn balance(env: Env, id: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&RtKey::Balance(id))
            .unwrap_or(0)
    }
}

// ============================================================================
// Test harness
// ============================================================================

struct Harness<'a> {
    env: Env,
    admin: Address,
    depositor: Address,
    contributor: Address,
    token_addr: Address,
    escrow: BountyEscrowContractClient<'a>,
}

impl<'a> Harness<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);

        let token_addr = env.register_contract(None, ReentrantToken);
        ReentrantTokenClient::new(&env, &token_addr).mint(&depositor, &1_000_000_000);

        let contract_id = env.register_contract(None, BountyEscrowContract);
        let escrow = BountyEscrowContractClient::new(&env, &contract_id);
        escrow.init(&admin, &token_addr);

        Self {
            env,
            admin,
            depositor,
            contributor,
            token_addr,
            escrow,
        }
    }

    fn token(&self) -> ReentrantTokenClient<'a> {
        ReentrantTokenClient::new(&self.env, &self.token_addr)
    }

    /// Arm the token to attempt `action` during the next `transfer`.
    fn arm(&self, action: ReentryAction) {
        self.token()
            .configure_attack(&self.escrow.address, &action);
    }

    /// Disarm the token so subsequent transfers behave normally.
    fn disarm(&self) {
        self.arm(ReentryAction::None);
    }

    fn deadline(&self, offset: u64) -> u64 {
        self.env.ledger().timestamp() + offset
    }

    /// Placeholder params with everything zeroed/empty except the fields the
    /// caller overrides — keeps each test focused on the fields that matter.
    fn params(&self) -> ReentryParams {
        ReentryParams {
            addr_a: self.depositor.clone(),
            addr_b: self.depositor.clone(),
            bytes_a: BytesN::from_array(&self.env, &[0u8; 32]),
            bounty_id: 0,
            amount: 0,
            deadline: 0,
            lock_items: Vec::new(&self.env),
            release_items: Vec::new(&self.env),
        }
    }
}

// ============================================================================
// Group 1: Deposit boundary
// ============================================================================

/// **Attack: `lock_funds`'s token transfer re-enters `lock_funds` for a second
/// bounty.** The guard must reject the nested call before the second escrow is
/// ever created, and the outer `lock_funds` call must roll back atomically —
/// bounty #1 must not exist afterwards.
#[test]
fn test_lock_funds_reentry_via_transfer_rejected_state_unchanged() {
    let h = Harness::new();
    let deadline = h.deadline(5_000);

    let mut p = h.params();
    p.addr_a = h.depositor.clone();
    p.bounty_id = 2;
    p.amount = 500;
    p.deadline = deadline;
    h.arm(ReentryAction::LockFunds(p));

    let result = h.escrow.try_lock_funds(&h.depositor, &1_u64, &1_000, &deadline);
    assert!(result.is_err(), "reentrant lock_funds must be rejected");

    // Full rollback: neither bounty #1 (outer call) nor #2 (attempted reentry)
    // exists, and the depositor's tokens were never moved.
    assert!(h.escrow.try_get_escrow_info(&1_u64).is_err());
    assert!(h.escrow.try_get_escrow_info(&2_u64).is_err());
    assert_eq!(h.token().balance(&h.depositor), 1_000_000_000);
    assert_eq!(h.token().balance(&h.escrow.address), 0);
}

/// **Attack: `lock_funds_anonymous`'s transfer re-enters itself for a second
/// commitment.** Same guarantee as above for the anonymous-deposit path.
#[test]
fn test_lock_funds_anonymous_reentry_rejected_state_unchanged() {
    let h = Harness::new();
    let deadline = h.deadline(5_000);
    let commitment = BytesN::from_array(&h.env, &[7u8; 32]);
    let commitment2 = BytesN::from_array(&h.env, &[9u8; 32]);

    let mut p = h.params();
    p.addr_a = h.depositor.clone();
    p.bytes_a = commitment2;
    p.bounty_id = 4;
    p.amount = 500;
    p.deadline = deadline;
    h.arm(ReentryAction::LockFundsAnonymous(p));

    let result =
        h.escrow
            .try_lock_funds_anonymous(&h.depositor, &commitment, &3_u64, &1_000, &deadline);
    assert!(result.is_err(), "reentrant lock_funds_anonymous must be rejected");

    assert_eq!(h.token().balance(&h.depositor), 1_000_000_000);
    assert_eq!(h.token().balance(&h.escrow.address), 0);
}

/// **Attack: `batch_lock_funds`'s per-item transfer re-enters `lock_funds`.**
/// The reentrant call fails on the very first item's transfer, so the whole
/// batch — including items ordered before the malicious transfer — rolls back.
#[test]
fn test_batch_lock_funds_reentry_rejected_state_unchanged() {
    let h = Harness::new();
    let deadline = h.deadline(5_000);

    let mut p = h.params();
    p.addr_a = h.depositor.clone();
    p.bounty_id = 99;
    p.amount = 100;
    p.deadline = deadline;
    h.arm(ReentryAction::LockFunds(p));

    let items = vec![
        &h.env,
        LockFundsItem {
            bounty_id: 10,
            depositor: h.depositor.clone(),
            amount: 500,
            deadline,
        },
        LockFundsItem {
            bounty_id: 11,
            depositor: h.depositor.clone(),
            amount: 600,
            deadline,
        },
    ];
    let result = h.escrow.try_batch_lock_funds(&items);
    assert!(result.is_err(), "reentrant batch_lock_funds must be rejected");

    assert!(h.escrow.try_get_escrow_info(&10_u64).is_err());
    assert!(h.escrow.try_get_escrow_info(&11_u64).is_err());
    assert!(h.escrow.try_get_escrow_info(&99_u64).is_err());
    assert_eq!(h.token().balance(&h.depositor), 1_000_000_000);
}

// ============================================================================
// Group 2: Payout boundary
// ============================================================================

/// **Attack: `release_funds`'s payout transfer re-enters `release_funds` on the
/// same bounty (classic double-spend attempt).** The reentrant call is rejected,
/// and — critically — the outer call's own state effects (status → Released)
/// roll back too, so the escrow is still `Locked` and the contributor received
/// nothing.
#[test]
fn test_release_funds_reentry_rejected_state_unchanged() {
    let h = Harness::new();
    let deadline = h.deadline(5_000);
    h.escrow.lock_funds(&h.depositor, &1_u64, &1_000, &deadline);

    let mut p = h.params();
    p.addr_a = h.contributor.clone();
    p.bounty_id = 1;
    h.arm(ReentryAction::ReleaseFunds(p));

    let result = h.escrow.try_release_funds(&1_u64, &h.contributor);
    assert!(result.is_err(), "reentrant release_funds must be rejected");

    let info = h.escrow.get_escrow_info(&1_u64);
    assert_eq!(info.status, EscrowStatus::Locked, "outer release must roll back");
    assert_eq!(info.remaining_amount, 1_000);
    assert_eq!(h.token().balance(&h.contributor), 0);
    assert_eq!(h.token().balance(&h.escrow.address), 1_000);
}

/// **Attack: `partial_release`'s transfer re-enters `partial_release` for a
/// second payout on the same bounty.** Rejected; `remaining_amount` must not
/// have been decremented by either the inner or outer call.
#[test]
fn test_partial_release_reentry_rejected_state_unchanged() {
    let h = Harness::new();
    let deadline = h.deadline(5_000);
    h.escrow.lock_funds(&h.depositor, &1_u64, &1_000, &deadline);

    let mut p = h.params();
    p.addr_a = h.contributor.clone();
    p.bounty_id = 1;
    p.amount = 300;
    h.arm(ReentryAction::PartialRelease(p));

    let result = h.escrow.try_partial_release(&1_u64, &h.contributor, &400);
    assert!(result.is_err(), "reentrant partial_release must be rejected");

    let info = h.escrow.get_escrow_info(&1_u64);
    assert_eq!(info.remaining_amount, 1_000, "no payout must have been recorded");
    assert_eq!(info.status, EscrowStatus::Locked);
    assert_eq!(h.token().balance(&h.contributor), 0);
}

/// **Attack: `claim`'s transfer re-enters `claim` on the same bounty.** Rejected;
/// the claim record must remain unclaimed and the escrow still Locked.
#[test]
fn test_claim_reentry_rejected_state_unchanged() {
    let h = Harness::new();
    let deadline = h.deadline(10_000);
    h.escrow.lock_funds(&h.depositor, &1_u64, &1_000, &deadline);
    h.escrow.set_claim_window(&500_u64);
    h.escrow
        .authorize_claim(&1_u64, &h.contributor, &DisputeReason::Other);

    let mut p = h.params();
    p.bounty_id = 1;
    h.arm(ReentryAction::Claim(p));

    let result = h.escrow.try_claim(&1_u64);
    assert!(result.is_err(), "reentrant claim must be rejected");

    let info = h.escrow.get_escrow_info(&1_u64);
    assert_eq!(info.status, EscrowStatus::Locked, "claim must roll back");
    assert_eq!(info.remaining_amount, 1_000);
    assert_eq!(h.token().balance(&h.contributor), 0);
}

/// **Attack: `release_with_capability`'s transfer re-enters plain `release_funds`
/// (cross-function reentry).** Proves the guard blocks reentry across *different*
/// entry-points, not just self-reentry. The capability must remain unconsumed.
#[test]
fn test_release_with_capability_reentry_rejected_state_unchanged() {
    let h = Harness::new();
    let deadline = h.deadline(5_000);
    h.escrow.lock_funds(&h.depositor, &1_u64, &1_000, &deadline);

    let expiry = h.deadline(300);
    let capability_id = h.escrow.issue_capability(
        &h.admin,
        &h.contributor,
        &CapabilityAction::Release,
        &1_u64,
        &1_000,
        &expiry,
        &2,
    );

    let mut p = h.params();
    p.addr_a = h.contributor.clone();
    p.bounty_id = 1;
    h.arm(ReentryAction::ReleaseFunds(p));

    let result = h.escrow.try_release_with_capability(
        &1_u64,
        &h.contributor,
        &400,
        &h.contributor,
        &capability_id,
    );
    assert!(result.is_err(), "reentrant release_with_capability must be rejected");

    let info = h.escrow.get_escrow_info(&1_u64);
    assert_eq!(info.remaining_amount, 1_000, "capability release must roll back");
    let cap = h.escrow.get_capability(&capability_id);
    assert_eq!(cap.remaining_uses, 2, "capability must remain unconsumed");
}

/// **Attack: `batch_release_funds`'s per-item transfer re-enters `release_funds`.**
/// Rejected; every item in the batch (including ones processed before the
/// malicious transfer) rolls back to `Locked`.
#[test]
fn test_batch_release_funds_reentry_rejected_state_unchanged() {
    let h = Harness::new();
    let deadline = h.deadline(5_000);
    h.escrow.lock_funds(&h.depositor, &10_u64, &500, &deadline);
    h.escrow.lock_funds(&h.depositor, &11_u64, &600, &deadline);

    let mut p = h.params();
    p.addr_a = h.contributor.clone();
    p.bounty_id = 10;
    h.arm(ReentryAction::ReleaseFunds(p));

    let items = vec![
        &h.env,
        ReleaseFundsItem {
            bounty_id: 10,
            contributor: h.contributor.clone(),
        },
        ReleaseFundsItem {
            bounty_id: 11,
            contributor: h.contributor.clone(),
        },
    ];
    let result = h.escrow.try_batch_release_funds(&items);
    assert!(result.is_err(), "reentrant batch_release_funds must be rejected");

    assert_eq!(h.escrow.get_escrow_info(&10_u64).status, EscrowStatus::Locked);
    assert_eq!(h.escrow.get_escrow_info(&11_u64).status, EscrowStatus::Locked);
    assert_eq!(h.token().balance(&h.contributor), 0);
}

// ============================================================================
// Group 3: Refund boundary
// ============================================================================

/// **Attack: `refund`'s transfer re-enters `refund` on the same bounty.**
/// Rejected; the refund history must remain empty and status `Locked`.
#[test]
fn test_refund_reentry_rejected_state_unchanged() {
    let h = Harness::new();
    let deadline = h.deadline(1_000);
    h.escrow.lock_funds(&h.depositor, &1_u64, &1_000, &deadline);
    h.env.ledger().set_timestamp(deadline + 1);

    let mut p = h.params();
    p.bounty_id = 1;
    h.arm(ReentryAction::Refund(p));

    let result = h.escrow.try_refund(&1_u64);
    assert!(result.is_err(), "reentrant refund must be rejected");

    let info = h.escrow.get_escrow_info(&1_u64);
    assert_eq!(info.status, EscrowStatus::Locked, "refund must roll back");
    assert_eq!(info.remaining_amount, 1_000);
    assert_eq!(info.refund_history.len(), 0);
    assert_eq!(h.token().balance(&h.depositor), 1_000_000_000 - 1_000);
}

/// **Attack: `refund_resolved`'s transfer re-enters plain `refund` (cross-function
/// reentry) on the anonymous-escrow refund path.** Rejected; the anonymous escrow
/// record must be untouched.
#[test]
fn test_refund_resolved_reentry_rejected_state_unchanged() {
    let h = Harness::new();
    let deadline = h.deadline(1_000);
    let commitment = BytesN::from_array(&h.env, &[1u8; 32]);
    h.escrow
        .lock_funds_anonymous(&h.depositor, &commitment, &5_u64, &1_000, &deadline);
    h.escrow.set_anonymous_resolver(&Some(h.admin.clone()));
    h.env.ledger().set_timestamp(deadline + 1);

    let mut p = h.params();
    p.bounty_id = 5;
    h.arm(ReentryAction::Refund(p));

    let result = h.escrow.try_refund_resolved(&5_u64, &h.depositor);
    assert!(result.is_err(), "reentrant refund_resolved must be rejected");

    // Read the raw AnonymousEscrow record directly — get_escrow_info only
    // covers the non-anonymous path.
    h.env.as_contract(&h.escrow.address, || {
        let anon: AnonymousEscrow = h
            .env
            .storage()
            .persistent()
            .get(&DataKey::EscrowAnon(5_u64))
            .unwrap();
        assert_eq!(anon.status, EscrowStatus::Locked, "refund_resolved must roll back");
        assert_eq!(anon.remaining_amount, 1_000);
    });
    assert_eq!(h.token().balance(&h.escrow.address), 1_000);
}

/// **Attack: `refund_with_capability`'s transfer re-enters plain `refund`
/// (cross-function reentry).** Rejected; the capability must remain unconsumed.
#[test]
fn test_refund_with_capability_reentry_rejected_state_unchanged() {
    let h = Harness::new();
    let deadline = h.deadline(1_000);
    h.escrow.lock_funds(&h.depositor, &1_u64, &1_000, &deadline);

    let expiry = h.deadline(300);
    let capability_id = h.escrow.issue_capability(
        &h.admin,
        &h.depositor,
        &CapabilityAction::Refund,
        &1_u64,
        &1_000,
        &expiry,
        &2,
    );

    let mut p = h.params();
    p.bounty_id = 1;
    h.arm(ReentryAction::Refund(p));

    let result =
        h.escrow
            .try_refund_with_capability(&1_u64, &400, &h.depositor, &capability_id);
    assert!(result.is_err(), "reentrant refund_with_capability must be rejected");

    let info = h.escrow.get_escrow_info(&1_u64);
    assert_eq!(info.remaining_amount, 1_000, "capability refund must roll back");
    let cap = h.escrow.get_capability(&capability_id);
    assert_eq!(cap.remaining_uses, 2, "capability must remain unconsumed");
}

// ============================================================================
// Group 4: Admin boundary
// ============================================================================

/// **Attack: `emergency_withdraw`'s transfer re-enters `emergency_withdraw`
/// (attempted double-drain).** Rejected; the contract's balance must be exactly
/// what it was before the call (nothing withdrawn twice, nothing lost).
#[test]
fn test_emergency_withdraw_reentry_rejected_state_unchanged() {
    let h = Harness::new();
    let deadline = h.deadline(5_000);
    h.escrow.lock_funds(&h.depositor, &1_u64, &1_000, &deadline);
    h.escrow.set_paused(
        &Some(true),
        &None::<bool>,
        &None::<bool>,
        &Some(soroban_sdk::String::from_str(&h.env, "attack")),
    );

    let target = Address::generate(&h.env);
    let target2 = Address::generate(&h.env);

    let mut p = h.params();
    p.addr_a = target2.clone();
    h.arm(ReentryAction::EmergencyWithdraw(p));

    let result = h.escrow.try_emergency_withdraw(&target);
    assert!(result.is_err(), "reentrant emergency_withdraw must be rejected");

    assert_eq!(h.token().balance(&target), 0, "no funds must reach either target");
    assert_eq!(h.token().balance(&target2), 0);
    assert_eq!(
        h.token().balance(&h.escrow.address),
        1_000,
        "contract balance must be untouched by the rejected withdrawal"
    );
}

// ============================================================================
// Group 5: Deterministic panic message
// ============================================================================
//
// The `try_*` assertions above prove reentry is rejected deterministically
// (every failure is caught, never a silent success). These tests additionally
// pin the exact panic text observed, using the direct (non-`try_`) client for
// one representative case per boundary category.
//
// The observed text is `"Error(Context, InvalidAction)"` (Soroban host:
// "Contract re-entry is not allowed"), *not* this contract's own
// `"Reentrancy detected"` message from `reentrancy_guard::acquire`. That is
// an important, verified finding about how these two defenses actually
// layer: every attack in this file has the shape escrow → token → escrow,
// i.e. the malicious token calls back into the *same* escrow contract
// instance that is already on the call stack. The Soroban host itself
// refuses to re-enter a contract instance that is already executing,
// independently of any application code, and that check runs *before* the
// reentrant call's Rust body — including its `reentrancy_guard::acquire`
// call — ever executes. So for this specific attack shape, the host's own
// protection is what a real attacker would hit first.
//
// This does not make `reentrancy_guard` redundant: it is exercised directly
// (not through a cross-contract call, so not shadowed by the host check) by
// the unit tests in `test_reentrancy_guard.rs` — e.g.
// `test_double_acquire_panics` — and it remains the layer that would matter
// if the host's own protection were ever weakened, bypassed via a future
// SDK change, or if the contract were ever composed behind a proxy/router
// that calls back through a *different* address than the one already on the
// stack. See the module doc at the top of this file for the full design
// rationale.

#[test]
#[should_panic(expected = "Error(Context, InvalidAction)")]
fn test_lock_funds_reentry_panics_with_deterministic_message() {
    let h = Harness::new();
    let deadline = h.deadline(5_000);

    let mut p = h.params();
    p.addr_a = h.depositor.clone();
    p.bounty_id = 2;
    p.amount = 500;
    p.deadline = deadline;
    h.arm(ReentryAction::LockFunds(p));

    h.escrow.lock_funds(&h.depositor, &1_u64, &1_000, &deadline);
}

#[test]
#[should_panic(expected = "Error(Context, InvalidAction)")]
fn test_release_funds_reentry_panics_with_deterministic_message() {
    let h = Harness::new();
    let deadline = h.deadline(5_000);
    h.escrow.lock_funds(&h.depositor, &1_u64, &1_000, &deadline);

    let mut p = h.params();
    p.addr_a = h.contributor.clone();
    p.bounty_id = 1;
    h.arm(ReentryAction::ReleaseFunds(p));

    h.escrow.release_funds(&1_u64, &h.contributor);
}

#[test]
#[should_panic(expected = "Error(Context, InvalidAction)")]
fn test_refund_reentry_panics_with_deterministic_message() {
    let h = Harness::new();
    let deadline = h.deadline(1_000);
    h.escrow.lock_funds(&h.depositor, &1_u64, &1_000, &deadline);
    h.env.ledger().set_timestamp(deadline + 1);

    let mut p = h.params();
    p.bounty_id = 1;
    h.arm(ReentryAction::Refund(p));

    h.escrow.refund(&1_u64);
}

#[test]
#[should_panic(expected = "Error(Context, InvalidAction)")]
fn test_emergency_withdraw_reentry_panics_with_deterministic_message() {
    let h = Harness::new();
    let deadline = h.deadline(5_000);
    h.escrow.lock_funds(&h.depositor, &1_u64, &1_000, &deadline);
    h.escrow.set_paused(
        &Some(true),
        &None::<bool>,
        &None::<bool>,
        &Some(soroban_sdk::String::from_str(&h.env, "attack")),
    );

    let target = Address::generate(&h.env);
    let mut p = h.params();
    p.addr_a = Address::generate(&h.env);
    h.arm(ReentryAction::EmergencyWithdraw(p));

    h.escrow.emergency_withdraw(&target);
}

// ============================================================================
// Group 6: Non-reentrant baseline — the malicious-shaped token must not change
// behavior for legitimate, non-reentrant calls.
// ============================================================================

/// **Baseline: unarmed `ReentrantToken` completes a full lock → release
/// lifecycle identically to a well-behaved token.** Guards against a harness
/// bug that would make every call fail regardless of whether an attack is
/// configured.
#[test]
fn test_unarmed_token_lock_then_release_behaves_normally() {
    let h = Harness::new();
    let deadline = h.deadline(5_000);
    h.disarm();

    h.escrow.lock_funds(&h.depositor, &1_u64, &1_000, &deadline);
    let info = h.escrow.get_escrow_info(&1_u64);
    assert_eq!(info.status, EscrowStatus::Locked);
    assert_eq!(info.amount, 1_000);

    h.escrow.release_funds(&1_u64, &h.contributor);
    let info = h.escrow.get_escrow_info(&1_u64);
    assert_eq!(info.status, EscrowStatus::Released);
    assert_eq!(h.token().balance(&h.contributor), 1_000);
    assert_eq!(h.token().balance(&h.escrow.address), 0);
}

/// **Baseline: unarmed `ReentrantToken` completes a full lock → refund
/// lifecycle identically to a well-behaved token.**
#[test]
fn test_unarmed_token_lock_then_refund_behaves_normally() {
    let h = Harness::new();
    let deadline = h.deadline(1_000);
    h.disarm();

    h.escrow.lock_funds(&h.depositor, &1_u64, &2_000, &deadline);
    h.env.ledger().set_timestamp(deadline + 1);

    let before = h.token().balance(&h.depositor);
    h.escrow.refund(&1_u64);

    let info = h.escrow.get_escrow_info(&1_u64);
    assert_eq!(info.status, EscrowStatus::Refunded);
    assert_eq!(h.token().balance(&h.depositor), before + 2_000);
}

/// **Baseline: disarming after a failed attack allows the operation to succeed
/// normally on retry.** Documents that the guard's atomic rollback leaves the
/// contract fully usable — the earlier rejected attack left no residue.
#[test]
fn test_disarm_after_rejected_attack_allows_normal_retry() {
    let h = Harness::new();
    let deadline = h.deadline(5_000);

    let mut p = h.params();
    p.addr_a = h.depositor.clone();
    p.bounty_id = 2;
    p.amount = 500;
    p.deadline = deadline;
    h.arm(ReentryAction::LockFunds(p));

    let result = h.escrow.try_lock_funds(&h.depositor, &1_u64, &1_000, &deadline);
    assert!(result.is_err());
    assert!(h.escrow.try_get_escrow_info(&1_u64).is_err());

    h.disarm();
    h.escrow.lock_funds(&h.depositor, &1_u64, &1_000, &deadline);
    let info = h.escrow.get_escrow_info(&1_u64);
    assert_eq!(info.status, EscrowStatus::Locked);
    assert_eq!(info.amount, 1_000);
}
