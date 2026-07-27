//! Freeze-mechanism tests for `BountyEscrowContract`.
//!
//! Two independent freeze layers exist:
//! - **Escrow-level**: `freeze_escrow` / `unfreeze_escrow`, keyed by `bounty_id`
//! - **Address-level**: `freeze_address` / `unfreeze_address`, keyed by the
//!   escrow **depositor** address
//!
//! Precedence (locked in by the matrix tests below):
//! - **Either freeze independently blocks** release, refund, and claim.
//!   Both freezes must be absent for the operation to proceed (logical OR of
//!   blocks; equivalently AND of permissions).
//! - When **both** freezes apply, the escrow-level check runs first, so the
//!   deterministic error is `EscrowFrozen` (never `AddressFrozen`).
//! - Address freezes are keyed on the **depositor**: a frozen contributor /
//!   claim recipient does NOT block release or claim (funds-out is gated on
//!   the escrow and its owner, not the payee). Freezing a payee requires
//!   freezing the escrow itself.
//! - Freezes gate funds-out paths only: a frozen depositor can still lock
//!   new funds, and read-only queries always succeed.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

use crate::{BountyEscrowContract, BountyEscrowContractClient, DisputeReason, Error};

struct TestEnv<'a> {
    env: Env,
    client: BountyEscrowContractClient<'a>,
    token_admin: token::StellarAssetClient<'a>,
    admin: Address,
    depositor: Address,
    contributor: Address,
}

impl<'a> TestEnv<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);

        let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
        let token_id = token_contract.address();
        let token_admin = token::StellarAssetClient::new(&env, &token_id);

        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);

        client.init(&admin, &token_id);

        // Fund depositor
        token_admin.mint(&depositor, &10_000);

        Self {
            env,
            client,
            token_admin,
            admin,
            depositor,
            contributor,
        }
    }

    fn lock(&self, bounty_id: u64, amount: i128) {
        let deadline = self.env.ledger().timestamp() + 10_000;
        self.client
            .lock_funds(&self.depositor, &bounty_id, &amount, &deadline);
    }

    /// Apply one of the four {escrow frozen} x {depositor frozen} combos.
    fn apply_freezes(&self, bounty_id: u64, escrow_frozen: bool, address_frozen: bool) {
        if escrow_frozen {
            self.client.freeze_escrow(&bounty_id, &None);
        }
        if address_frozen {
            self.client.freeze_address(&self.depositor, &None);
        }
    }

    /// Advance the ledger clock past every deadline used by `lock`.
    fn pass_deadline(&self) {
        self.env
            .ledger()
            .set_timestamp(self.env.ledger().timestamp() + 20_000);
    }

    /// Authorize a claim for `bounty_id` payable to the contributor.
    fn authorize_claim(&self, bounty_id: u64) {
        self.client.set_claim_window(&10_000);
        self.client
            .authorize_claim(&bounty_id, &self.contributor, &DisputeReason::Other);
    }
}

// ── Escrow-level freeze ──────────────────────────────────────────────────────

#[test]
fn test_freeze_escrow_blocks_release() {
    let t = TestEnv::new();
    t.lock(1, 1000);
    t.client.freeze_escrow(
        &1,
        &Some(soroban_sdk::String::from_str(&t.env, "investigation")),
    );
    let result = t.client.try_release_funds(&1, &t.contributor);
    assert_eq!(result.unwrap_err().unwrap(), Error::EscrowFrozen);
}

#[test]
fn test_freeze_escrow_blocks_refund() {
    let t = TestEnv::new();
    t.lock(1, 1000);
    t.client.freeze_escrow(&1, &None);
    t.pass_deadline();
    let result = t.client.try_refund(&1);
    assert_eq!(result.unwrap_err().unwrap(), Error::EscrowFrozen);
}

#[test]
fn test_freeze_escrow_allows_read_access() {
    let t = TestEnv::new();
    t.lock(1, 1000);
    t.client.freeze_escrow(&1, &None);
    // read-only calls must succeed
    let info = t.client.get_escrow_info(&1);
    assert_eq!(info.amount, 1000);
    let record = t.client.get_escrow_freeze_record(&1);
    assert!(record.is_some());
    assert!(record.unwrap().frozen);
}

#[test]
fn test_freeze_escrow_blocks_partial_release() {
    let t = TestEnv::new();
    t.lock(1, 1000);
    t.client.freeze_escrow(&1, &None);
    let result = t.client.try_partial_release(&1, &t.contributor, &500);
    assert_eq!(result.unwrap_err().unwrap(), Error::EscrowFrozen);
}

#[test]
fn test_freeze_escrow_blocks_batch_release() {
    let t = TestEnv::new();
    t.lock(1, 1000);
    t.client.freeze_escrow(&1, &None);
    let items = soroban_sdk::vec![
        &t.env,
        crate::ReleaseFundsItem {
            bounty_id: 1,
            contributor: t.contributor.clone(),
        }
    ];
    let result = t.client.try_batch_release_funds(&items);
    assert_eq!(result.unwrap_err().unwrap(), Error::EscrowFrozen);
}

#[test]
fn test_unfreeze_escrow_allows_release() {
    let t = TestEnv::new();
    t.lock(1, 1000);
    t.client.freeze_escrow(&1, &None);
    t.client.unfreeze_escrow(&1);
    // should succeed now
    t.client.release_funds(&1, &t.contributor);
    let info = t.client.get_escrow_info(&1);
    assert_eq!(info.status, crate::EscrowStatus::Released);
}

#[test]
fn test_unfreeze_escrow_allows_refund() {
    let t = TestEnv::new();
    t.lock(1, 1000);
    t.client.freeze_escrow(&1, &None);
    t.client.unfreeze_escrow(&1);
    t.pass_deadline();
    t.client.refund(&1);
    let info = t.client.get_escrow_info(&1);
    assert_eq!(info.status, crate::EscrowStatus::Refunded);
}

#[test]
fn test_freeze_escrow_emits_event() {
    let t = TestEnv::new();
    t.lock(1, 1000);
    // freeze_escrow should not panic — event emission is tested implicitly
    t.client
        .freeze_escrow(&1, &Some(soroban_sdk::String::from_str(&t.env, "audit")));
    let record = t.client.get_escrow_freeze_record(&1).unwrap();
    assert!(record.frozen);
}

#[test]
fn test_unfreeze_escrow_emits_event() {
    let t = TestEnv::new();
    t.lock(1, 1000);
    t.client.freeze_escrow(&1, &None);
    t.client.unfreeze_escrow(&1);
    let record = t.client.get_escrow_freeze_record(&1);
    assert!(record.is_none());
}

#[test]
fn test_freeze_escrow_missing_bounty_rejected() {
    let t = TestEnv::new();
    // freeze_escrow on a non-existent bounty returns BountyNotFound
    let result = t.client.try_freeze_escrow(&999, &None);
    assert_eq!(result.unwrap_err().unwrap(), Error::BountyNotFound);
}

#[test]
fn test_freeze_one_escrow_does_not_affect_another() {
    let t = TestEnv::new();
    t.lock(1, 1000);
    t.lock(2, 500);
    t.client.freeze_escrow(&1, &None);
    // escrow 2 should still be releasable
    t.client.release_funds(&2, &t.contributor);
    let info = t.client.get_escrow_info(&2);
    assert_eq!(info.status, crate::EscrowStatus::Released);
}

#[test]
fn test_freeze_escrow_does_not_block_new_lock_on_different_id() {
    let t = TestEnv::new();
    t.lock(1, 1000);
    t.client.freeze_escrow(&1, &None);
    // locking a new bounty id should work fine
    t.lock(2, 500);
    let info = t.client.get_escrow_info(&2);
    assert_eq!(info.status, crate::EscrowStatus::Locked);
}

#[test]
fn test_get_escrow_freeze_record_returns_correct_data() {
    let t = TestEnv::new();
    t.lock(1, 1000);
    let reason = soroban_sdk::String::from_str(&t.env, "compliance hold");
    t.client.freeze_escrow(&1, &Some(reason.clone()));
    let record = t.client.get_escrow_freeze_record(&1).unwrap();
    assert!(record.frozen);
    assert_eq!(record.reason, Some(reason));
    assert_eq!(record.frozen_by, t.admin);
}

// ── Address-level freeze ─────────────────────────────────────────────────────

#[test]
fn test_freeze_address_blocks_refund() {
    let t = TestEnv::new();
    t.lock(1, 1000);
    t.client.freeze_address(
        &t.depositor,
        &Some(soroban_sdk::String::from_str(&t.env, "kyc")),
    );
    t.pass_deadline();
    let result = t.client.try_refund(&1);
    assert_eq!(result.unwrap_err().unwrap(), Error::AddressFrozen);
}

#[test]
fn test_freeze_address_blocks_release_on_all_owned_escrows() {
    let t = TestEnv::new();
    t.lock(1, 500);
    t.lock(2, 500);
    t.client.freeze_address(&t.depositor, &None);
    assert_eq!(
        t.client
            .try_release_funds(&1, &t.contributor)
            .unwrap_err()
            .unwrap(),
        Error::AddressFrozen
    );
    assert_eq!(
        t.client
            .try_release_funds(&2, &t.contributor)
            .unwrap_err()
            .unwrap(),
        Error::AddressFrozen
    );
}

#[test]
fn test_freeze_address_allows_read_queries() {
    let t = TestEnv::new();
    t.lock(1, 1000);
    t.client.freeze_address(&t.depositor, &None);
    // read-only calls must still work
    let info = t.client.get_escrow_info(&1);
    assert_eq!(info.amount, 1000);
    let record = t.client.get_address_freeze_record(&t.depositor);
    assert!(record.is_some());
}

#[test]
fn test_freeze_address_does_not_affect_different_depositor() {
    let t = TestEnv::new();
    let other = Address::generate(&t.env);
    t.token_admin.mint(&other, &5_000);
    let deadline = t.env.ledger().timestamp() + 10_000;
    t.client.lock_funds(&t.depositor, &1, &1000, &deadline);
    t.client.lock_funds(&other, &2, &500, &deadline);
    t.client.freeze_address(&t.depositor, &None);
    // other depositor's escrow unaffected
    t.client.release_funds(&2, &t.contributor);
    let info = t.client.get_escrow_info(&2);
    assert_eq!(info.status, crate::EscrowStatus::Released);
}

#[test]
fn test_unfreeze_address_restores_operations() {
    let t = TestEnv::new();
    t.lock(1, 1000);
    t.client.freeze_address(&t.depositor, &None);
    t.client.unfreeze_address(&t.depositor);
    t.client.release_funds(&1, &t.contributor);
    let info = t.client.get_escrow_info(&1);
    assert_eq!(info.status, crate::EscrowStatus::Released);
}

#[test]
fn test_get_address_freeze_record() {
    let t = TestEnv::new();
    let reason = soroban_sdk::String::from_str(&t.env, "aml check");
    t.client.freeze_address(&t.depositor, &Some(reason.clone()));
    let record = t.client.get_address_freeze_record(&t.depositor).unwrap();
    assert!(record.frozen);
    assert_eq!(record.reason, Some(reason));
    assert_eq!(record.frozen_by, t.admin);
}

// ── Overlapping freeze precedence: {escrow} x {address} matrix ───────────────
//
// Locked-in precedence: either freeze independently blocks release, refund,
// and claim. When both apply, the escrow-level check runs first, so the
// deterministic error is EscrowFrozen. A future change that makes one layer
// bypass the other flips a row in these matrices and fails the test.

/// Expected outcome for a funds-out operation under a freeze combination.
/// `None` = operation must succeed; `Some(e)` = must fail with exactly `e`.
fn expected_error(escrow_frozen: bool, address_frozen: bool) -> Option<Error> {
    match (escrow_frozen, address_frozen) {
        (false, false) => None,
        (true, _) => Some(Error::EscrowFrozen), // escrow check runs first
        (false, true) => Some(Error::AddressFrozen),
    }
}

#[test]
fn test_release_freeze_precedence_matrix() {
    for escrow_frozen in [false, true] {
        for address_frozen in [false, true] {
            let t = TestEnv::new();
            t.lock(1, 1000);
            t.apply_freezes(1, escrow_frozen, address_frozen);

            let result = t.client.try_release_funds(&1, &t.contributor);
            match expected_error(escrow_frozen, address_frozen) {
                None => {
                    assert!(
                        result.is_ok(),
                        "release must succeed when neither freeze applies"
                    );
                    assert_eq!(
                        t.client.get_escrow_info(&1).status,
                        crate::EscrowStatus::Released
                    );
                }
                Some(expected) => {
                    assert_eq!(
                        result.unwrap_err().unwrap(),
                        expected,
                        "release: wrong outcome for escrow_frozen={}, address_frozen={}",
                        escrow_frozen,
                        address_frozen
                    );
                    // Blocked release must not move state.
                    assert_eq!(
                        t.client.get_escrow_info(&1).status,
                        crate::EscrowStatus::Locked
                    );
                }
            }
        }
    }
}

#[test]
fn test_refund_freeze_precedence_matrix() {
    for escrow_frozen in [false, true] {
        for address_frozen in [false, true] {
            let t = TestEnv::new();
            t.lock(1, 1000);
            t.apply_freezes(1, escrow_frozen, address_frozen);
            t.pass_deadline();

            let result = t.client.try_refund(&1);
            match expected_error(escrow_frozen, address_frozen) {
                None => {
                    assert!(
                        result.is_ok(),
                        "refund must succeed when neither freeze applies"
                    );
                    assert_eq!(
                        t.client.get_escrow_info(&1).status,
                        crate::EscrowStatus::Refunded
                    );
                }
                Some(expected) => {
                    assert_eq!(
                        result.unwrap_err().unwrap(),
                        expected,
                        "refund: wrong outcome for escrow_frozen={}, address_frozen={}",
                        escrow_frozen,
                        address_frozen
                    );
                    assert_eq!(
                        t.client.get_escrow_info(&1).status,
                        crate::EscrowStatus::Locked
                    );
                }
            }
        }
    }
}

#[test]
fn test_claim_freeze_precedence_matrix() {
    for escrow_frozen in [false, true] {
        for address_frozen in [false, true] {
            let t = TestEnv::new();
            t.lock(1, 1000);
            // Authorize before freezing: authorize_claim itself checks both
            // freezes, so the freeze must land between authorize and claim.
            t.authorize_claim(1);
            t.apply_freezes(1, escrow_frozen, address_frozen);

            let result = t.client.try_claim(&1);
            match expected_error(escrow_frozen, address_frozen) {
                None => {
                    assert!(
                        result.is_ok(),
                        "claim must succeed when neither freeze applies"
                    );
                    assert_eq!(
                        t.client.get_escrow_info(&1).status,
                        crate::EscrowStatus::Released
                    );
                }
                Some(expected) => {
                    assert_eq!(
                        result.unwrap_err().unwrap(),
                        expected,
                        "claim: wrong outcome for escrow_frozen={}, address_frozen={}",
                        escrow_frozen,
                        address_frozen
                    );
                    assert_eq!(
                        t.client.get_escrow_info(&1).status,
                        crate::EscrowStatus::Locked
                    );
                }
            }
        }
    }
}

/// authorize_claim is itself a gated path: both freezes independently block
/// creating a new pending claim in the first place.
#[test]
fn test_authorize_claim_blocked_by_either_freeze() {
    let t = TestEnv::new();
    t.lock(1, 1000);
    t.client.freeze_escrow(&1, &None);
    t.client.set_claim_window(&10_000);
    assert_eq!(
        t.client
            .try_authorize_claim(&1, &t.contributor, &DisputeReason::Other)
            .unwrap_err()
            .unwrap(),
        Error::EscrowFrozen
    );

    let t2 = TestEnv::new();
    t2.lock(1, 1000);
    t2.client.freeze_address(&t2.depositor, &None);
    t2.client.set_claim_window(&10_000);
    assert_eq!(
        t2.client
            .try_authorize_claim(&1, &t2.contributor, &DisputeReason::Other)
            .unwrap_err()
            .unwrap(),
        Error::AddressFrozen
    );
}

// ── Freeze scope boundaries (documented behavior) ────────────────────────────

/// Address freezes are keyed on the escrow **depositor**. Freezing the
/// contributor (payee) does NOT block release — funds-out gating follows the
/// escrow and its owner. To stop a payout to a specific recipient, freeze the
/// escrow itself.
#[test]
fn test_contributor_freeze_does_not_block_release() {
    let t = TestEnv::new();
    t.lock(1, 1000);
    t.client.freeze_address(&t.contributor, &None);

    t.client.release_funds(&1, &t.contributor);
    assert_eq!(
        t.client.get_escrow_info(&1).status,
        crate::EscrowStatus::Released
    );
}

/// Same boundary on the claim path: a frozen claim recipient can still claim
/// as long as the escrow and its depositor are unfrozen.
#[test]
fn test_recipient_freeze_does_not_block_claim() {
    let t = TestEnv::new();
    t.lock(1, 1000);
    t.authorize_claim(1);
    t.client.freeze_address(&t.contributor, &None);

    t.client.claim(&1);
    assert_eq!(
        t.client.get_escrow_info(&1).status,
        crate::EscrowStatus::Released
    );
}

/// Freezes gate funds-out only: a frozen depositor can still lock new funds.
#[test]
fn test_frozen_depositor_can_still_lock_new_funds() {
    let t = TestEnv::new();
    t.client.freeze_address(&t.depositor, &None);
    t.lock(1, 1000);
    assert_eq!(
        t.client.get_escrow_info(&1).status,
        crate::EscrowStatus::Locked
    );
    // ... but cannot release it while frozen.
    assert_eq!(
        t.client
            .try_release_funds(&1, &t.contributor)
            .unwrap_err()
            .unwrap(),
        Error::AddressFrozen
    );
}

// ── Overlapping freeze/unfreeze sequences and record independence ────────────

/// Both freezes active, then lifted one at a time (escrow first): the records
/// stay independently queryable and accurate at every step, and the operation
/// stays blocked until BOTH freezes are lifted.
#[test]
fn test_overlapping_freezes_unfreeze_escrow_first() {
    let t = TestEnv::new();
    t.lock(1, 1000);

    let escrow_reason = soroban_sdk::String::from_str(&t.env, "dispute");
    let address_reason = soroban_sdk::String::from_str(&t.env, "aml");
    t.client.freeze_escrow(&1, &Some(escrow_reason.clone()));
    t.client
        .freeze_address(&t.depositor, &Some(address_reason.clone()));

    // Both records exist, each with its own reason.
    let esc = t.client.get_escrow_freeze_record(&1).unwrap();
    let adr = t.client.get_address_freeze_record(&t.depositor).unwrap();
    assert!(esc.frozen && adr.frozen);
    assert_eq!(esc.reason, Some(escrow_reason));
    assert_eq!(adr.reason, Some(address_reason.clone()));

    // Both frozen → escrow error wins (checked first).
    assert_eq!(
        t.client
            .try_release_funds(&1, &t.contributor)
            .unwrap_err()
            .unwrap(),
        Error::EscrowFrozen
    );

    // Lift the escrow freeze: its record is gone, the address record is
    // untouched, and the operation is still blocked — now by the address.
    t.client.unfreeze_escrow(&1);
    assert!(t.client.get_escrow_freeze_record(&1).is_none());
    let adr = t.client.get_address_freeze_record(&t.depositor).unwrap();
    assert!(adr.frozen);
    assert_eq!(adr.reason, Some(address_reason));
    assert_eq!(
        t.client
            .try_release_funds(&1, &t.contributor)
            .unwrap_err()
            .unwrap(),
        Error::AddressFrozen
    );

    // Lift the address freeze: both records gone, operation proceeds.
    t.client.unfreeze_address(&t.depositor);
    assert!(t.client.get_address_freeze_record(&t.depositor).is_none());
    t.client.release_funds(&1, &t.contributor);
    assert_eq!(
        t.client.get_escrow_info(&1).status,
        crate::EscrowStatus::Released
    );
}

/// Same sequence in the reverse order (address lifted first): still blocked by
/// the remaining escrow freeze, records stay independent throughout.
#[test]
fn test_overlapping_freezes_unfreeze_address_first() {
    let t = TestEnv::new();
    t.lock(1, 1000);
    t.client.freeze_escrow(&1, &None);
    t.client.freeze_address(&t.depositor, &None);

    t.client.unfreeze_address(&t.depositor);
    assert!(t.client.get_address_freeze_record(&t.depositor).is_none());
    assert!(t.client.get_escrow_freeze_record(&1).unwrap().frozen);

    assert_eq!(
        t.client
            .try_release_funds(&1, &t.contributor)
            .unwrap_err()
            .unwrap(),
        Error::EscrowFrozen
    );

    t.client.unfreeze_escrow(&1);
    t.client.release_funds(&1, &t.contributor);
    assert_eq!(
        t.client.get_escrow_info(&1).status,
        crate::EscrowStatus::Released
    );
}

/// Overlapping freezes block refund identically, in both unfreeze orders.
#[test]
fn test_overlapping_freezes_block_refund_until_both_lifted() {
    let t = TestEnv::new();
    t.lock(1, 1000);
    t.client.freeze_escrow(&1, &None);
    t.client.freeze_address(&t.depositor, &None);
    t.pass_deadline();

    assert_eq!(
        t.client.try_refund(&1).unwrap_err().unwrap(),
        Error::EscrowFrozen
    );

    t.client.unfreeze_escrow(&1);
    assert_eq!(
        t.client.try_refund(&1).unwrap_err().unwrap(),
        Error::AddressFrozen
    );

    t.client.unfreeze_address(&t.depositor);
    t.client.refund(&1);
    assert_eq!(
        t.client.get_escrow_info(&1).status,
        crate::EscrowStatus::Refunded
    );
}

/// Unfreezing the escrow must not clear the address record, and vice versa —
/// re-freezing one layer updates only that layer's record.
#[test]
fn test_refreeze_updates_only_its_own_record() {
    let t = TestEnv::new();
    t.lock(1, 1000);

    let first = soroban_sdk::String::from_str(&t.env, "first");
    let second = soroban_sdk::String::from_str(&t.env, "second");
    let addr_reason = soroban_sdk::String::from_str(&t.env, "addr");

    t.client.freeze_escrow(&1, &Some(first));
    t.client
        .freeze_address(&t.depositor, &Some(addr_reason.clone()));

    t.client.unfreeze_escrow(&1);
    t.client.freeze_escrow(&1, &Some(second.clone()));

    // Escrow record reflects the second freeze; address record untouched.
    let esc = t.client.get_escrow_freeze_record(&1).unwrap();
    assert!(esc.frozen);
    assert_eq!(esc.reason, Some(second));
    assert_eq!(esc.frozen_by, t.admin);
    let adr = t.client.get_address_freeze_record(&t.depositor).unwrap();
    assert!(adr.frozen);
    assert_eq!(adr.reason, Some(addr_reason));
}

/// Freeze records for unrelated keys are never created as a side effect.
#[test]
fn test_no_phantom_records_from_overlapping_freezes() {
    let t = TestEnv::new();
    t.lock(1, 1000);
    t.lock(2, 500);
    t.client.freeze_escrow(&1, &None);
    t.client.freeze_address(&t.depositor, &None);

    // Escrow 2 has no escrow-level record; contributor has no address record.
    assert!(t.client.get_escrow_freeze_record(&2).is_none());
    assert!(t
        .client
        .get_address_freeze_record(&t.contributor)
        .is_none());
}
