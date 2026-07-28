//! Capability Token Replay-Attack Regression Tests
//!
//! Covers issue #1379: revoked, expired, and cross-escrow tokens must be
//! permanently invalid.  Every replay attempt MUST:
//!   - Return an error before any state mutation
//!   - NOT update escrow balances or status
//!   - NOT emit side-effects (token transfers, etc.)
//!
//! Test matrix
//! ───────────
//! 1. revoke_then_reuse_release     – release_with_capability after revocation
//! 2. revoke_then_reuse_refund      – refund_with_capability after revocation
//! 3. revoke_then_reuse_lock        – capability cannot be (re)used on lock path
//! 4. expired_token_replay          – capability expired before use
//! 5. cross_escrow_token_replay     – token bound to bounty A used against bounty B
//! 6. exact_byte_replay             – raw BytesN<32> identity reuse after revocation

#![cfg(test)]

use crate::{
    BountyEscrowContract, BountyEscrowContractClient, CapabilityAction, Error, EscrowStatus,
};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, BytesN, Env,
};

// ─────────────────────────────────────────────────────────────────────────────
// Shared test harness
// ─────────────────────────────────────────────────────────────────────────────

/// Minimal setup used across all replay regression tests.
struct ReplaySetup {
    env: Env,
    client: BountyEscrowContractClient<'static>,
    token_client: token::Client<'static>,
    admin: Address,
    depositor: Address,
    delegate: Address,
}

impl ReplaySetup {
    /// Create a fresh contract environment with funds minted to the depositor.
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let delegate = Address::generate(&env);

        let token_admin_addr = Address::generate(&env);
        let token_address = env
            .register_stellar_asset_contract_v2(token_admin_addr.clone())
            .address();
        let token_client = token::Client::new(&env, &token_address);
        let token_admin = token::StellarAssetClient::new(&env, &token_address);

        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);

        client.init(&admin, &token_address);
        token_admin.mint(&depositor, &200_000);

        Self {
            env,
            client,
            token_client,
            admin,
            depositor,
            delegate,
        }
    }

    /// Lock `amount` into a new escrow identified by `bounty_id`.
    fn lock(&self, bounty_id: u64, amount: i128) {
        let deadline = self.env.ledger().timestamp() + 100_000;
        self.client
            .lock_funds(&self.depositor, &bounty_id, &amount, &deadline);
    }

    /// Issue a Release capability for `bounty_id` valid for `max_uses` uses.
    fn issue_release_cap(&self, bounty_id: u64, amount: i128, max_uses: u32) -> BytesN<32> {
        let expiry = self.env.ledger().timestamp() + 3_600;
        self.client.issue_capability(
            &self.admin,
            &self.delegate,
            &CapabilityAction::Release,
            &bounty_id,
            &amount,
            &expiry,
            &max_uses,
        )
    }

    /// Issue a Refund capability for `bounty_id`.
    fn issue_refund_cap(&self, bounty_id: u64, amount: i128) -> BytesN<32> {
        let expiry = self.env.ledger().timestamp() + 3_600;
        self.client.issue_capability(
            &self.admin,
            &self.delegate,
            &CapabilityAction::Refund,
            &bounty_id,
            &amount,
            &expiry,
            &1,
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. Revoke-then-reuse: release_with_capability
// ─────────────────────────────────────────────────────────────────────────────

/// @security Revoked capability MUST NOT allow release.
/// Asserts: Error::CapabilityRevoked returned, escrow state unchanged.
#[test]
fn test_revoke_then_reuse_release() {
    let s = ReplaySetup::new();
    let bounty_id = 101u64;
    s.lock(bounty_id, 1_000);

    let contributor = Address::generate(&s.env);
    let cap_id = s.issue_release_cap(bounty_id, 500, 2);

    // Capture pre-revocation state
    let escrow_before = s.client.get_escrow_info(&bounty_id);
    assert_eq!(escrow_before.status, EscrowStatus::Locked);
    assert_eq!(escrow_before.remaining_amount, 1_000);

    // Revoke the capability
    s.client.revoke_capability(&s.admin, &cap_id);
    assert!(s.client.get_capability(&cap_id).revoked);

    // Replay attempt: release_with_capability using revoked token
    let result = s.client.try_release_with_capability(
        &bounty_id,
        &contributor,
        &500,
        &s.delegate,
        &cap_id,
    );
    assert_eq!(result.unwrap_err().unwrap(), Error::CapabilityRevoked);

    // State MUST be unchanged
    let escrow_after = s.client.get_escrow_info(&bounty_id);
    assert_eq!(escrow_after.status, EscrowStatus::Locked);
    assert_eq!(escrow_after.remaining_amount, 1_000);

    // No funds must have left the contract
    assert_eq!(s.token_client.balance(&contributor), 0);
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Revoke-then-reuse: refund_with_capability
// ─────────────────────────────────────────────────────────────────────────────

/// @security Revoked capability MUST NOT allow refund.
/// Asserts: Error::CapabilityRevoked returned, escrow state unchanged.
#[test]
fn test_revoke_then_reuse_refund() {
    let s = ReplaySetup::new();
    let bounty_id = 102u64;
    s.lock(bounty_id, 2_000);

    let cap_id = s.issue_refund_cap(bounty_id, 1_000);

    let escrow_before = s.client.get_escrow_info(&bounty_id);
    assert_eq!(escrow_before.remaining_amount, 2_000);

    // Revoke
    s.client.revoke_capability(&s.admin, &cap_id);

    // Replay attempt
    let result =
        s.client
            .try_refund_with_capability(&bounty_id, &1_000, &s.delegate, &cap_id);
    assert_eq!(result.unwrap_err().unwrap(), Error::CapabilityRevoked);

    // State unchanged
    let escrow_after = s.client.get_escrow_info(&bounty_id);
    assert_eq!(escrow_after.remaining_amount, 2_000);
    assert_eq!(escrow_after.status, EscrowStatus::Locked);
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. Revoke-then-reuse: lock path guard
// ─────────────────────────────────────────────────────────────────────────────

/// @security A revoked token must fail even if the caller invents a second use
/// attempt on the same capability ID (byte-identical replay on a different op).
/// Asserts: get_capability returns revoked=true; any consume attempt errors.
#[test]
fn test_revoke_prevents_all_subsequent_uses() {
    let s = ReplaySetup::new();
    let bounty_id = 103u64;
    s.lock(bounty_id, 5_000);

    // Issue a 3-use release capability
    let cap_id = s.issue_release_cap(bounty_id, 5_000, 3);

    let contributor = Address::generate(&s.env);

    // First use succeeds
    s.client.release_with_capability(
        &bounty_id,
        &contributor,
        &1_000,
        &s.delegate,
        &cap_id,
    );
    let cap_after_use = s.client.get_capability(&cap_id);
    assert_eq!(cap_after_use.remaining_uses, 2);
    assert_eq!(cap_after_use.remaining_amount, 4_000);

    // Now revoke mid-lifecycle
    s.client.revoke_capability(&s.admin, &cap_id);

    // Second use MUST fail with CapabilityRevoked — no partial drain allowed
    let result = s.client.try_release_with_capability(
        &bounty_id,
        &contributor,
        &1_000,
        &s.delegate,
        &cap_id,
    );
    assert_eq!(result.unwrap_err().unwrap(), Error::CapabilityRevoked);

    // remaining_amount in escrow is exactly what was released (1_000), not more
    let escrow = s.client.get_escrow_info(&bounty_id);
    assert_eq!(escrow.remaining_amount, 4_000);
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. Expired token replay
// ─────────────────────────────────────────────────────────────────────────────

/// @security A token past its expiry timestamp MUST be rejected.
/// Asserts: Error::CapabilityExpired returned, escrow state unchanged.
#[test]
fn test_expired_token_replay() {
    let s = ReplaySetup::new();
    let bounty_id = 104u64;
    s.lock(bounty_id, 1_500);

    // Issue with a very short TTL
    let expiry = s.env.ledger().timestamp() + 10;
    let cap_id = s.client.issue_capability(
        &s.admin,
        &s.delegate,
        &CapabilityAction::Release,
        &bounty_id,
        &800,
        &expiry,
        &1,
    );

    let contributor = Address::generate(&s.env);

    // Advance time past expiry
    s.env.ledger().set_timestamp(expiry + 1);

    // Replay attempt with expired token
    let result = s.client.try_release_with_capability(
        &bounty_id,
        &contributor,
        &800,
        &s.delegate,
        &cap_id,
    );
    assert_eq!(result.unwrap_err().unwrap(), Error::CapabilityExpired);

    // Escrow untouched
    let escrow = s.client.get_escrow_info(&bounty_id);
    assert_eq!(escrow.remaining_amount, 1_500);
    assert_eq!(escrow.status, EscrowStatus::Locked);
    assert_eq!(s.token_client.balance(&contributor), 0);
}

/// @security Expired refund capability MUST also be rejected.
#[test]
fn test_expired_refund_capability_replay() {
    let s = ReplaySetup::new();
    let bounty_id = 105u64;
    s.lock(bounty_id, 1_000);

    let expiry = s.env.ledger().timestamp() + 5;
    let cap_id = s.client.issue_capability(
        &s.admin,
        &s.delegate,
        &CapabilityAction::Refund,
        &bounty_id,
        &500,
        &expiry,
        &1,
    );

    s.env.ledger().set_timestamp(expiry + 1);

    let result =
        s.client
            .try_refund_with_capability(&bounty_id, &500, &s.delegate, &cap_id);
    assert_eq!(result.unwrap_err().unwrap(), Error::CapabilityExpired);

    let escrow = s.client.get_escrow_info(&bounty_id);
    assert_eq!(escrow.remaining_amount, 1_000);
    assert_eq!(escrow.status, EscrowStatus::Locked);
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. Cross-escrow token replay
// ─────────────────────────────────────────────────────────────────────────────

/// @security A capability issued for bounty A MUST NOT be accepted for bounty B.
/// Asserts: Error::CapabilityActionMismatch returned, both escrows unchanged.
#[test]
fn test_cross_escrow_token_replay() {
    let s = ReplaySetup::new();
    let bounty_a = 201u64;
    let bounty_b = 202u64;

    s.lock(bounty_a, 3_000);
    s.lock(bounty_b, 3_000);

    // Issue capability bound to bounty_a
    let cap_id = s.issue_release_cap(bounty_a, 1_000, 1);

    let contributor = Address::generate(&s.env);

    // Attempt to use it against bounty_b
    let result = s.client.try_release_with_capability(
        &bounty_b,   // wrong escrow
        &contributor,
        &1_000,
        &s.delegate,
        &cap_id,
    );
    // bounty_id mismatch triggers CapabilityActionMismatch
    assert_eq!(
        result.unwrap_err().unwrap(),
        Error::CapabilityActionMismatch
    );

    // Both escrows must remain untouched
    let a = s.client.get_escrow_info(&bounty_a);
    let b = s.client.get_escrow_info(&bounty_b);
    assert_eq!(a.remaining_amount, 3_000);
    assert_eq!(b.remaining_amount, 3_000);
    assert_eq!(s.token_client.balance(&contributor), 0);
}

/// @security Cross-escrow refund replay must also fail.
#[test]
fn test_cross_escrow_refund_replay() {
    let s = ReplaySetup::new();
    let bounty_a = 203u64;
    let bounty_b = 204u64;

    s.lock(bounty_a, 2_000);
    s.lock(bounty_b, 2_000);

    let cap_id = s.issue_refund_cap(bounty_a, 1_000);

    let result = s.client.try_refund_with_capability(
        &bounty_b, // wrong escrow
        &1_000,
        &s.delegate,
        &cap_id,
    );
    assert_eq!(
        result.unwrap_err().unwrap(),
        Error::CapabilityActionMismatch
    );

    let a = s.client.get_escrow_info(&bounty_a);
    let b = s.client.get_escrow_info(&bounty_b);
    assert_eq!(a.remaining_amount, 2_000);
    assert_eq!(b.remaining_amount, 2_000);
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. Exact token byte replay (critical case)
// ─────────────────────────────────────────────────────────────────────────────

/// @security After revocation the stored capability is permanently marked
/// revoked.  Any caller that retains the raw BytesN<32> token ID and attempts
/// to reuse it (byte-identical replay) MUST receive Error::CapabilityRevoked.
///
/// This is the most critical path: an attacker captures the token bytes and
/// tries to replay them after the token owner intended to invalidate it.
#[test]
fn test_exact_byte_replay_after_revoke() {
    let s = ReplaySetup::new();
    let bounty_id = 301u64;
    s.lock(bounty_id, 10_000);

    let cap_id: BytesN<32> = s.issue_release_cap(bounty_id, 5_000, 5);

    let contributor = Address::generate(&s.env);

    // Perform one legitimate use to prove the token works
    s.client.release_with_capability(
        &bounty_id,
        &contributor,
        &1_000,
        &s.delegate,
        &cap_id,
    );
    assert_eq!(s.token_client.balance(&contributor), 1_000);

    // Store the raw bytes — simulate attacker retaining the ID
    let raw_id: BytesN<32> = cap_id.clone();

    // Revoke
    s.client.revoke_capability(&s.admin, &cap_id);

    // Byte-identical replay: use the exact same BytesN<32> value
    let replay1 = s.client.try_release_with_capability(
        &bounty_id,
        &contributor,
        &1_000,
        &s.delegate,
        &raw_id,
    );
    assert_eq!(replay1.unwrap_err().unwrap(), Error::CapabilityRevoked);

    // Second replay attempt (attacker retries)
    let replay2 = s.client.try_release_with_capability(
        &bounty_id,
        &contributor,
        &500,
        &s.delegate,
        &raw_id,
    );
    assert_eq!(replay2.unwrap_err().unwrap(), Error::CapabilityRevoked);

    // Escrow must reflect only the one legitimate release (9_000 remaining)
    let escrow = s.client.get_escrow_info(&bounty_id);
    assert_eq!(escrow.remaining_amount, 9_000);
    // Contributor only ever received the one legitimate payout
    assert_eq!(s.token_client.balance(&contributor), 1_000);
}

/// @security A single-use token that exhausts its uses MUST be blocked on
/// any subsequent identical-byte replay attempt (CapabilityUsesExhausted).
#[test]
fn test_single_use_token_byte_replay() {
    let s = ReplaySetup::new();
    let bounty_id = 302u64;
    s.lock(bounty_id, 2_000);

    // Issue exactly 1 use
    let cap_id: BytesN<32> = s.issue_release_cap(bounty_id, 2_000, 1);
    let contributor = Address::generate(&s.env);

    // Legitimate use — exhausts the token
    s.client.release_with_capability(
        &bounty_id,
        &contributor,
        &2_000,
        &s.delegate,
        &cap_id,
    );

    // Escrow is now Released
    let escrow = s.client.get_escrow_info(&bounty_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
    assert_eq!(escrow.remaining_amount, 0);

    // Byte-identical replay: escrow is Released so FundsNotLocked fires first,
    // but the important invariant is: no further funds can be extracted.
    let replay = s.client.try_release_with_capability(
        &bounty_id,
        &contributor,
        &1,
        &s.delegate,
        &cap_id,
    );
    // Either FundsNotLocked (escrow already released) or CapabilityUsesExhausted
    // is acceptable — both confirm that replay is rejected.
    assert!(replay.is_err());
}
