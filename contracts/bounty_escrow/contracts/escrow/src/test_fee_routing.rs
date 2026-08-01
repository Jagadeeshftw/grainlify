//! Per-bounty fee routing tests for `BountyEscrowContract`.
//!
//! Covers the fee-routing immutability guard and audit trail
//! (see `docs/security/fee-routing-immutability.md`):
//! - `set_fee_routing` succeeds while the bounty is in `Draft` status
//! - `set_fee_routing` is rejected with `FeeRoutingLocked` once the bounty is
//!   `Locked` (or any later status, including anonymous escrows which are
//!   created directly in `Locked`)
//! - `set_fee_routing_with_reason` is the audited post-lock override: it
//!   requires a non-empty reason and emits `FeeRoutingChanged` with the
//!   previous and new destinations
//! - Share invariants are enforced identically on both paths
//! - Routing set pre-lock actually governs where release fees land

#![cfg(test)]

use crate::{
    events, BountyEscrowContract, BountyEscrowContractClient, DataKey, Error, Escrow, EscrowStatus,
    PerBountyFeeRouting,
};
use soroban_sdk::{
    testutils::{Address as _, Events},
    token, vec, Address, Env, String, Symbol, TryFromVal, Val, Vec,
};

// ── helpers ──────────────────────────────────────────────────────────────────

struct Suite {
    env: Env,
    client: BountyEscrowContractClient<'static>,
    contract_id: Address,
    admin: Address,
    depositor: Address,
    contributor: Address,
    treasury: Address,
    partner: Address,
    token_id: Address,
    token_admin: token::StellarAssetClient<'static>,
}

impl Suite {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);
        let treasury = Address::generate(&env);
        let partner = Address::generate(&env);

        let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
        let token_id = token_contract.address();
        let token_admin = token::StellarAssetClient::new(&env, &token_id);

        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);
        client.init(&admin, &token_id);

        Self {
            env,
            client,
            contract_id,
            admin,
            depositor,
            contributor,
            treasury,
            partner,
            token_id,
            token_admin,
        }
    }

    /// Lock a funded escrow (created directly in `Locked` status).
    fn lock(&self, bounty_id: u64, amount: i128) {
        self.token_admin.mint(&self.depositor, &amount);
        let deadline = self.env.ledger().timestamp() + 10_000;
        self.client
            .lock_funds(&self.depositor, &bounty_id, &amount, &deadline);
    }

    /// Create a funded escrow in `Draft` status (the pre-lock state used by
    /// the recurring-lock flow) by writing the record directly, since drafts
    /// are only produced by that flow in production. Mirrors that flow:
    /// tokens are already held by the contract and the bounty is indexed, so
    /// the INV-2 aggregate-to-ledger invariant holds after `publish`.
    fn create_draft(&self, bounty_id: u64, amount: i128) {
        let escrow = Escrow {
            depositor: self.depositor.clone(),
            amount,
            remaining_amount: amount,
            status: EscrowStatus::Draft,
            deadline: self.env.ledger().timestamp() + 10_000,
            refund_history: vec![&self.env],
            archived: false,
            archived_at: None,
        };
        self.token_admin.mint(&self.contract_id, &amount);
        self.env.as_contract(&self.contract_id, || {
            self.env
                .storage()
                .persistent()
                .set(&DataKey::Escrow(bounty_id), &escrow);
            let mut index: Vec<u64> = self
                .env
                .storage()
                .persistent()
                .get(&DataKey::EscrowIndex)
                .unwrap_or(Vec::new(&self.env));
            index.push_back(bounty_id);
            self.env
                .storage()
                .persistent()
                .set(&DataKey::EscrowIndex, &index);
        });
    }

    /// Set routing 100% to `self.treasury` via the plain (pre-lock) path.
    fn set_routing_to_treasury(&self, bounty_id: u64) {
        self.client
            .set_fee_routing(&bounty_id, &self.treasury, &10_000, &None, &0);
    }

    /// Decode the most recent `FeeRoutingChanged` event, identified by its
    /// `"fee_rchg"` topic.
    fn last_routing_changed_event(&self) -> events::FeeRoutingChanged {
        let marker = Symbol::new(&self.env, "fee_rchg");
        let mut found = None;
        for (contract, topics, data) in self.env.events().all().iter() {
            if contract != self.contract_id {
                continue;
            }
            let first: Val = topics.get(0).unwrap();
            if Symbol::try_from_val(&self.env, &first) != Ok(marker.clone()) {
                continue;
            }
            found = Some(
                events::FeeRoutingChanged::try_from_val(&self.env, &data)
                    .expect("fee_rchg event data must decode as FeeRoutingChanged"),
            );
        }
        found.expect("no FeeRoutingChanged event emitted")
    }
}

// ── pre-lock: plain path works ───────────────────────────────────────────────

/// Routing may be configured normally while the bounty is still Draft.
#[test]
fn test_set_routing_on_draft_succeeds() {
    let s = Suite::new();
    s.create_draft(1, 1_000);

    s.set_routing_to_treasury(1);

    let routing = s.client.get_fee_routing(&1).unwrap();
    assert_eq!(routing.treasury_recipient, s.treasury);
    assert_eq!(routing.treasury_bps, 10_000);
    assert_eq!(routing.partner_recipient, None);
    assert_eq!(routing.partner_bps, 0);
}

/// Routing with a partner split may be configured on a Draft bounty.
#[test]
fn test_set_routing_with_partner_on_draft_succeeds() {
    let s = Suite::new();
    s.create_draft(1, 1_000);

    s.client
        .set_fee_routing(&1, &s.treasury, &7_000, &Some(s.partner.clone()), &3_000);

    let routing = s.client.get_fee_routing(&1).unwrap();
    assert_eq!(routing.treasury_bps, 7_000);
    assert_eq!(routing.partner_recipient, Some(s.partner.clone()));
    assert_eq!(routing.partner_bps, 3_000);
}

/// Pre-lock changes may be revised any number of times while still Draft.
#[test]
fn test_routing_can_be_revised_while_draft() {
    let s = Suite::new();
    s.create_draft(1, 1_000);

    s.set_routing_to_treasury(1);
    let other = Address::generate(&s.env);
    s.client.set_fee_routing(&1, &other, &10_000, &None, &0);

    assert_eq!(
        s.client.get_fee_routing(&1).unwrap().treasury_recipient,
        other
    );
}

/// Routing configured while Draft survives the Draft -> Locked transition and
/// governs where the release fee actually lands.
#[test]
fn test_draft_routing_survives_publish_and_routes_release_fee() {
    let s = Suite::new();

    // 1% release fee, globally enabled; default recipient is the admin so a
    // successful per-bounty override is observable.
    s.client.update_fee_config(
        &Some(0i128),
        &Some(100i128),
        &None,
        &None,
        &Some(s.admin.clone()),
        &Some(true),
    );

    // Draft with funds pre-deposited to the contract so release can pay out.
    s.create_draft(1, 100_000);

    s.set_routing_to_treasury(1);
    s.client.publish(&1);

    s.client.release_funds(&1, &s.contributor);

    let tc = token::TokenClient::new(&s.env, &s.token_id);
    // release fee = 1% of 100_000 = 1_000, routed to the per-bounty treasury.
    assert_eq!(tc.balance(&s.treasury), 1_000);
    assert_eq!(tc.balance(&s.contributor), 99_000);
}

// ── post-lock: plain path is rejected ────────────────────────────────────────

/// The core guard: once Locked, the plain path must fail with FeeRoutingLocked.
#[test]
fn test_set_routing_after_lock_rejected() {
    let s = Suite::new();
    s.lock(1, 1_000);

    let result = s
        .client
        .try_set_fee_routing(&1, &s.treasury, &10_000, &None, &0);
    assert_eq!(result.unwrap_err().unwrap(), Error::FeeRoutingLocked);
    assert!(
        s.client.get_fee_routing(&1).is_none(),
        "rejected call must not write any routing"
    );
}

/// Publishing a Draft locks routing: the plain path stops working exactly at
/// the Draft -> Locked transition.
#[test]
fn test_publish_locks_routing() {
    let s = Suite::new();
    s.create_draft(1, 1_000);

    // Works while Draft…
    s.set_routing_to_treasury(1);

    // …and stops working the moment the bounty is published (Locked).
    s.client.publish(&1);
    let other = Address::generate(&s.env);
    let result = s.client.try_set_fee_routing(&1, &other, &10_000, &None, &0);
    assert_eq!(result.unwrap_err().unwrap(), Error::FeeRoutingLocked);

    // The pre-lock routing is untouched.
    assert_eq!(
        s.client.get_fee_routing(&1).unwrap().treasury_recipient,
        s.treasury
    );
}

/// Anonymous escrows are created directly in Locked status, so the plain path
/// is always rejected for them.
#[test]
fn test_set_routing_on_anonymous_escrow_rejected() {
    let s = Suite::new();
    s.token_admin.mint(&s.depositor, &1_000);
    let commitment = soroban_sdk::BytesN::from_array(&s.env, &[7u8; 32]);
    let deadline = s.env.ledger().timestamp() + 10_000;
    s.client
        .lock_funds_anonymous(&s.depositor, &commitment, &1, &1_000, &deadline);

    let result = s
        .client
        .try_set_fee_routing(&1, &s.treasury, &10_000, &None, &0);
    assert_eq!(result.unwrap_err().unwrap(), Error::FeeRoutingLocked);
}

/// Later statuses (Released) are equally immutable through the plain path.
#[test]
fn test_set_routing_after_release_rejected() {
    let s = Suite::new();
    s.lock(1, 1_000);
    s.client.release_funds(&1, &s.contributor);

    let result = s
        .client
        .try_set_fee_routing(&1, &s.treasury, &10_000, &None, &0);
    assert_eq!(result.unwrap_err().unwrap(), Error::FeeRoutingLocked);
}

/// A non-existent bounty is still BountyNotFound (checked before the guard).
#[test]
fn test_set_routing_missing_bounty_rejected() {
    let s = Suite::new();
    let result = s
        .client
        .try_set_fee_routing(&999, &s.treasury, &10_000, &None, &0);
    assert_eq!(result.unwrap_err().unwrap(), Error::BountyNotFound);
}

// ── post-lock: audited override path ─────────────────────────────────────────

/// The elevated path accepts a post-lock change when a reason is supplied.
#[test]
fn test_with_reason_succeeds_after_lock() {
    let s = Suite::new();
    s.lock(1, 1_000);

    let reason = String::from_str(&s.env, "treasury key rotation, ticket OPS-42");
    s.client
        .set_fee_routing_with_reason(&1, &s.treasury, &10_000, &None, &0, &reason);

    let routing = s.client.get_fee_routing(&1).unwrap();
    assert_eq!(routing.treasury_recipient, s.treasury);
}

/// An empty reason defeats the audit trail and must be rejected.
#[test]
fn test_with_reason_rejects_empty_reason() {
    let s = Suite::new();
    s.lock(1, 1_000);

    let empty = String::from_str(&s.env, "");
    let result =
        s.client
            .try_set_fee_routing_with_reason(&1, &s.treasury, &10_000, &None, &0, &empty);
    assert_eq!(result.unwrap_err().unwrap(), Error::InvalidAmount);
    assert!(s.client.get_fee_routing(&1).is_none());
}

/// The elevated path still requires the bounty to exist.
#[test]
fn test_with_reason_missing_bounty_rejected() {
    let s = Suite::new();
    let reason = String::from_str(&s.env, "why");
    let result = s.client.try_set_fee_routing_with_reason(
        &999,
        &s.treasury,
        &10_000,
        &None,
        &0,
        &reason,
    );
    assert_eq!(result.unwrap_err().unwrap(), Error::BountyNotFound);
}

/// The elevated path also works pre-lock (it is a superset of the plain path).
#[test]
fn test_with_reason_works_on_draft_too() {
    let s = Suite::new();
    s.create_draft(1, 1_000);
    let reason = String::from_str(&s.env, "initial config via audited path");
    s.client
        .set_fee_routing_with_reason(&1, &s.treasury, &10_000, &None, &0, &reason);
    assert!(s.client.get_fee_routing(&1).is_some());
}

// ── share invariants hold on both paths ──────────────────────────────────────

#[test]
fn test_share_invariants_enforced_on_plain_path() {
    let s = Suite::new();
    s.create_draft(1, 1_000);

    // No partner: treasury must take 100%.
    let r = s
        .client
        .try_set_fee_routing(&1, &s.treasury, &9_000, &None, &0);
    assert_eq!(r.unwrap_err().unwrap(), Error::InvalidAmount);

    // Partner present: shares must sum to exactly 100%.
    let r = s
        .client
        .try_set_fee_routing(&1, &s.treasury, &9_000, &Some(s.partner.clone()), &500);
    assert_eq!(r.unwrap_err().unwrap(), Error::InvalidAmount);

    // Out-of-range share.
    let r = s
        .client
        .try_set_fee_routing(&1, &s.treasury, &10_001, &None, &0);
    assert_eq!(r.unwrap_err().unwrap(), Error::InvalidAmount);
}

#[test]
fn test_share_invariants_enforced_on_reason_path() {
    let s = Suite::new();
    s.lock(1, 1_000);
    let reason = String::from_str(&s.env, "attempted misconfig");

    let r = s
        .client
        .try_set_fee_routing_with_reason(&1, &s.treasury, &9_000, &None, &0, &reason);
    assert_eq!(r.unwrap_err().unwrap(), Error::InvalidAmount);

    let r = s.client.try_set_fee_routing_with_reason(
        &1,
        &s.treasury,
        &9_000,
        &Some(s.partner.clone()),
        &500,
        &reason,
    );
    assert_eq!(r.unwrap_err().unwrap(), Error::InvalidAmount);
}

// ── FeeRoutingChanged audit event ────────────────────────────────────────────

/// First configuration: old destinations are None, new ones populated,
/// pre-lock path flagged as non-override.
#[test]
fn test_audit_event_on_first_configuration() {
    let s = Suite::new();
    s.create_draft(1, 1_000);

    s.set_routing_to_treasury(1);

    let ev = s.last_routing_changed_event();
    assert_eq!(ev.bounty_id, 1);
    assert_eq!(ev.old_treasury_recipient, None);
    assert_eq!(ev.old_partner_recipient, None);
    assert_eq!(ev.new_treasury_recipient, s.treasury);
    assert_eq!(ev.new_partner_recipient, None);
    assert_eq!(ev.changed_by, s.admin);
    assert!(!ev.post_lock_override);
    assert_eq!(ev.reason, None);
}

/// Post-lock override: the event carries the PREVIOUS and NEW destinations,
/// the override flag, and the mandatory reason — the full audit trail.
#[test]
fn test_audit_event_on_post_lock_override() {
    let s = Suite::new();
    s.create_draft(1, 1_000);
    s.client
        .set_fee_routing(&1, &s.treasury, &7_000, &Some(s.partner.clone()), &3_000);
    s.client.publish(&1);

    let new_treasury = Address::generate(&s.env);
    let reason = String::from_str(&s.env, "partner agreement terminated");
    s.client
        .set_fee_routing_with_reason(&1, &new_treasury, &10_000, &None, &0, &reason);

    let ev = s.last_routing_changed_event();
    assert_eq!(ev.old_treasury_recipient, Some(s.treasury.clone()));
    assert_eq!(ev.old_partner_recipient, Some(s.partner.clone()));
    assert_eq!(ev.new_treasury_recipient, new_treasury);
    assert_eq!(ev.new_partner_recipient, None);
    assert_eq!(ev.changed_by, s.admin);
    assert!(ev.post_lock_override);
    assert_eq!(ev.reason, Some(reason));
}

// ── storage / view accuracy ──────────────────────────────────────────────────

#[test]
fn test_get_fee_routing_none_when_unset() {
    let s = Suite::new();
    s.lock(1, 1_000);
    assert!(s.client.get_fee_routing(&1).is_none());
}

/// The stored record equals what was submitted, field for field.
#[test]
fn test_get_fee_routing_roundtrip() {
    let s = Suite::new();
    s.create_draft(1, 1_000);
    s.client
        .set_fee_routing(&1, &s.treasury, &6_500, &Some(s.partner.clone()), &3_500);
    assert_eq!(
        s.client.get_fee_routing(&1).unwrap(),
        PerBountyFeeRouting {
            treasury_recipient: s.treasury.clone(),
            treasury_bps: 6_500,
            partner_recipient: Some(s.partner.clone()),
            partner_bps: 3_500,
        }
    );
}
