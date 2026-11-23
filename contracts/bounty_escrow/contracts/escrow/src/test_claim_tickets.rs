//! Claim-ticket lifecycle tests: issue → redeem / revoke, with auth, expiry,
//! single-use and reservation (no double-spend) checks.

use crate::*;
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token, Address, Env, Symbol, TryFromVal,
};

const BOUNTY: u64 = 1;
const LOCKED: i128 = 1_000;

struct Setup<'a> {
    env: Env,
    admin: Address,
    depositor: Address,
    beneficiary: Address,
    token: token::Client<'a>,
    escrow: BountyEscrowContractClient<'a>,
    deadline: u64,
}

impl<'a> Setup<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(1_000);

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let beneficiary = Address::generate(&env);

        let sac = env.register_stellar_asset_contract_v2(admin.clone());
        let token = token::Client::new(&env, &sac.address());
        token::StellarAssetClient::new(&env, &sac.address()).mint(&depositor, &10_000);

        let escrow = BountyEscrowContractClient::new(
            &env,
            &env.register_contract(None, BountyEscrowContract),
        );
        escrow.init(&admin, &token.address);

        let deadline = env.ledger().timestamp() + 10_000;
        escrow.lock_funds(&depositor, &BOUNTY, &LOCKED, &deadline);

        Self {
            env,
            admin,
            depositor,
            beneficiary,
            token,
            escrow,
            deadline,
        }
    }

    fn now(&self) -> u64 {
        self.env.ledger().timestamp()
    }

    fn issue(&self, amount: i128) -> u64 {
        self.escrow.issue_claim_ticket(
            &self.admin,
            &BOUNTY,
            &self.beneficiary,
            &amount,
            &(self.now() + 500),
        )
    }

    fn topic_count(&self, name: &str) -> usize {
        let want = Symbol::new(&self.env, name);
        self.env
            .events()
            .all()
            .iter()
            .filter(|(_, topics, _)| {
                topics
                    .get(0)
                    .and_then(|t| Symbol::try_from_val(&self.env, &t).ok())
                    .is_some_and(|s| s == want)
            })
            .count()
    }
}

// ─── Issuance ────────────────────────────────────────────────────────────────

#[test]
fn test_issue_ticket_stores_ticket_and_reserves_amount() {
    let s = Setup::new();
    let expires_at = s.now() + 500;
    let id = s
        .escrow
        .issue_claim_ticket(&s.admin, &BOUNTY, &s.beneficiary, &400, &expires_at);
    assert_eq!(id, 1);
    assert_eq!(s.topic_count("ticket_i"), 1);

    let t = s.escrow.get_claim_ticket(&id);
    assert_eq!(
        t,
        ClaimTicket {
            ticket_id: 1,
            bounty_id: BOUNTY,
            beneficiary: s.beneficiary.clone(),
            amount: 400,
            expires_at,
            used: false,
            issued_at: s.now(),
            revoked: false,
        }
    );
    assert_eq!(s.escrow.get_ticket_reserved_amount(&BOUNTY), 400);
    // Issuance moves no funds.
    let e = s.escrow.get_escrow_info(&BOUNTY);
    assert_eq!(e.remaining_amount, LOCKED);
    assert_eq!(e.status, EscrowStatus::Locked);
}

#[test]
fn test_ticket_ids_are_monotonic_and_indexed_per_beneficiary() {
    let s = Setup::new();
    let other = Address::generate(&s.env);
    assert_eq!(s.issue(100), 1);
    assert_eq!(
        s.escrow
            .issue_claim_ticket(&s.admin, &BOUNTY, &other, &100, &(s.now() + 10)),
        2
    );
    assert_eq!(s.issue(100), 3);

    let mine = s.escrow.get_beneficiary_tickets(&s.beneficiary, &0, &10);
    assert_eq!(mine.len(), 2);
    assert_eq!(mine.get(0), Some(1));
    assert_eq!(mine.get(1), Some(3));
    let page = s.escrow.get_beneficiary_tickets(&s.beneficiary, &1, &10);
    assert_eq!(page.len(), 1);
    assert_eq!(page.get(0), Some(3));
    assert_eq!(s.escrow.get_beneficiary_tickets(&other, &0, &10).len(), 1);
    assert_eq!(s.escrow.get_ticket_reserved_amount(&BOUNTY), 300);
}

#[test]
fn test_depositor_can_issue_ticket() {
    let s = Setup::new();
    let id = s.escrow.issue_claim_ticket(
        &s.depositor,
        &BOUNTY,
        &s.beneficiary,
        &250,
        &(s.now() + 100),
    );
    assert_eq!(s.escrow.get_claim_ticket(&id).amount, 250);
}

#[test]
fn test_stranger_cannot_issue_ticket() {
    let s = Setup::new();
    let stranger = Address::generate(&s.env);
    let res =
        s.escrow
            .try_issue_claim_ticket(&stranger, &BOUNTY, &s.beneficiary, &100, &(s.now() + 100));
    assert_eq!(res, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_issue_requires_issuer_auth() {
    let s = Setup::new();
    s.issue(100);
    let auths = s.env.auths();
    assert!(auths.iter().any(|(addr, _)| *addr == s.admin));
}

#[test]
fn test_issue_for_missing_bounty_fails() {
    let s = Setup::new();
    let res =
        s.escrow
            .try_issue_claim_ticket(&s.admin, &999, &s.beneficiary, &100, &(s.now() + 100));
    assert_eq!(res, Err(Ok(Error::BountyNotFound)));
}

#[test]
fn test_issue_rejects_non_positive_amount() {
    let s = Setup::new();
    for amount in [0i128, -5] {
        let res = s.escrow.try_issue_claim_ticket(
            &s.admin,
            &BOUNTY,
            &s.beneficiary,
            &amount,
            &(s.now() + 100),
        );
        assert_eq!(res, Err(Ok(Error::InvalidAmount)));
    }
}

#[test]
fn test_issue_rejects_amount_over_remaining() {
    let s = Setup::new();
    let res = s.escrow.try_issue_claim_ticket(
        &s.admin,
        &BOUNTY,
        &s.beneficiary,
        &(LOCKED + 1),
        &(s.now() + 100),
    );
    assert_eq!(res, Err(Ok(Error::InvalidAmount)));
}

#[test]
fn test_issue_rejects_amount_over_unreserved_remaining() {
    let s = Setup::new();
    s.issue(700);
    let res =
        s.escrow
            .try_issue_claim_ticket(&s.admin, &BOUNTY, &s.beneficiary, &301, &(s.now() + 100));
    assert_eq!(res, Err(Ok(Error::InvalidAmount)));
    // Exactly the unreserved remainder is fine.
    s.issue(300);
    assert_eq!(s.escrow.get_ticket_reserved_amount(&BOUNTY), LOCKED);
}

#[test]
fn test_issue_rejects_past_or_current_expiry() {
    let s = Setup::new();
    for expires_at in [s.now(), s.now() - 1] {
        let res =
            s.escrow
                .try_issue_claim_ticket(&s.admin, &BOUNTY, &s.beneficiary, &100, &expires_at);
        assert_eq!(res, Err(Ok(Error::InvalidDeadline)));
    }
}

#[test]
fn test_issue_rejects_released_escrow() {
    let s = Setup::new();
    s.escrow.release_funds(&BOUNTY, &s.beneficiary);
    let res =
        s.escrow
            .try_issue_claim_ticket(&s.admin, &BOUNTY, &s.beneficiary, &100, &(s.now() + 100));
    assert_eq!(res, Err(Ok(Error::FundsNotLocked)));
}

#[test]
fn test_get_missing_ticket_fails() {
    let s = Setup::new();
    assert_eq!(
        s.escrow.try_get_claim_ticket(&42),
        Err(Ok(Error::TicketNotFound))
    );
}

// ─── Redemption ──────────────────────────────────────────────────────────────

#[test]
fn test_redeem_full_amount_pays_beneficiary_and_releases_escrow() {
    let s = Setup::new();
    let id = s.issue(LOCKED);
    s.escrow.claim_with_ticket(&id, &s.beneficiary);

    assert!(s.env.auths().iter().any(|(addr, _)| *addr == s.beneficiary));
    assert_eq!(s.token.balance(&s.beneficiary), LOCKED);
    assert_eq!(s.token.balance(&s.escrow.address), 0);
    let e = s.escrow.get_escrow_info(&BOUNTY);
    assert_eq!(e.remaining_amount, 0);
    assert_eq!(e.status, EscrowStatus::Released);
    assert!(s.escrow.get_claim_ticket(&id).used);
    assert_eq!(s.escrow.get_ticket_reserved_amount(&BOUNTY), 0);
    assert_eq!(s.topic_count("ticket_c"), 1);
}

#[test]
fn test_redeem_partial_amount_keeps_escrow_locked() {
    let s = Setup::new();
    let id = s.issue(300);
    s.escrow.claim_with_ticket(&id, &s.beneficiary);

    assert_eq!(s.token.balance(&s.beneficiary), 300);
    let e = s.escrow.get_escrow_info(&BOUNTY);
    assert_eq!(e.remaining_amount, LOCKED - 300);
    assert_eq!(e.status, EscrowStatus::Locked);
    // Remaining funds can still be paid out through the normal release path.
    s.escrow
        .partial_release(&BOUNTY, &s.depositor, &(LOCKED - 300));
    assert_eq!(
        s.escrow.get_escrow_info(&BOUNTY).status,
        EscrowStatus::Released
    );
}

#[test]
fn test_multiple_tickets_drain_escrow_exactly() {
    let s = Setup::new();
    let a = s.issue(600);
    let b = s.issue(400);
    s.escrow.claim_with_ticket(&a, &s.beneficiary);
    s.escrow.claim_with_ticket(&b, &s.beneficiary);
    assert_eq!(s.token.balance(&s.beneficiary), LOCKED);
    assert_eq!(
        s.escrow.get_escrow_info(&BOUNTY).status,
        EscrowStatus::Released
    );
}

#[test]
fn test_double_redeem_rejected() {
    let s = Setup::new();
    let id = s.issue(200);
    s.escrow.claim_with_ticket(&id, &s.beneficiary);
    assert_eq!(
        s.escrow.try_claim_with_ticket(&id, &s.beneficiary),
        Err(Ok(Error::TicketAlreadyUsed))
    );
    assert_eq!(s.token.balance(&s.beneficiary), 200);
    assert_eq!(
        s.escrow.get_escrow_info(&BOUNTY).remaining_amount,
        LOCKED - 200
    );
}

#[test]
fn test_redeem_at_or_after_expiry_rejected() {
    let s = Setup::new();
    let id = s.issue(200);
    let expires_at = s.escrow.get_claim_ticket(&id).expires_at;

    s.env.ledger().set_timestamp(expires_at);
    assert_eq!(
        s.escrow.try_claim_with_ticket(&id, &s.beneficiary),
        Err(Ok(Error::TicketExpired))
    );
    s.env.ledger().set_timestamp(expires_at + 1_000);
    assert_eq!(
        s.escrow.try_claim_with_ticket(&id, &s.beneficiary),
        Err(Ok(Error::TicketExpired))
    );
    assert_eq!(s.token.balance(&s.beneficiary), 0);
}

#[test]
fn test_redeem_just_before_expiry_succeeds() {
    let s = Setup::new();
    let id = s.issue(200);
    let expires_at = s.escrow.get_claim_ticket(&id).expires_at;
    s.env.ledger().set_timestamp(expires_at - 1);
    s.escrow.claim_with_ticket(&id, &s.beneficiary);
    assert_eq!(s.token.balance(&s.beneficiary), 200);
}

#[test]
fn test_wrong_beneficiary_rejected() {
    let s = Setup::new();
    let id = s.issue(200);
    let thief = Address::generate(&s.env);
    assert_eq!(
        s.escrow.try_claim_with_ticket(&id, &thief),
        Err(Ok(Error::Unauthorized))
    );
    assert_eq!(s.token.balance(&thief), 0);
    assert!(!s.escrow.get_claim_ticket(&id).used);
}

#[test]
fn test_redeem_missing_ticket_rejected() {
    let s = Setup::new();
    assert_eq!(
        s.escrow.try_claim_with_ticket(&7, &s.beneficiary),
        Err(Ok(Error::TicketNotFound))
    );
}

#[test]
fn test_redeem_while_release_paused_rejected() {
    let s = Setup::new();
    let id = s.issue(200);
    s.escrow.set_paused(&None, &Some(true), &None, &None);
    assert_eq!(
        s.escrow.try_claim_with_ticket(&id, &s.beneficiary),
        Err(Ok(Error::FundsPaused))
    );
}

#[test]
fn test_redeem_releases_reentrancy_guard() {
    let s = Setup::new();
    let a = s.issue(100);
    let b = s.issue(100);
    s.escrow.claim_with_ticket(&a, &s.beneficiary);
    // A second guarded call succeeds only if the guard was released.
    s.escrow.claim_with_ticket(&b, &s.beneficiary);
    s.escrow.partial_release(&BOUNTY, &s.depositor, &100);
    assert_eq!(s.token.balance(&s.beneficiary), 200);
}

// ─── Reservation: no double-spend with other payout paths ────────────────────

#[test]
fn test_reservation_blocks_full_release() {
    let s = Setup::new();
    s.issue(1);
    let other = Address::generate(&s.env);
    assert_eq!(
        s.escrow.try_release_funds(&BOUNTY, &other),
        Err(Ok(Error::InsufficientFunds))
    );
}

#[test]
fn test_reservation_limits_partial_release() {
    let s = Setup::new();
    let id = s.issue(600);
    let other = Address::generate(&s.env);
    assert_eq!(
        s.escrow.try_partial_release(&BOUNTY, &other, &401),
        Err(Ok(Error::InsufficientFunds))
    );
    s.escrow.partial_release(&BOUNTY, &other, &400);
    // Ticket is still fully backed.
    s.escrow.claim_with_ticket(&id, &s.beneficiary);
    assert_eq!(s.token.balance(&s.beneficiary), 600);
    assert_eq!(s.token.balance(&other), 400);
    assert_eq!(
        s.escrow.get_escrow_info(&BOUNTY).status,
        EscrowStatus::Released
    );
}

#[test]
fn test_reservation_blocks_refund_until_revoked() {
    let s = Setup::new();
    let id = s.issue(500);
    s.env.ledger().set_timestamp(s.deadline + 1);
    assert_eq!(
        s.escrow.try_refund(&BOUNTY),
        Err(Ok(Error::InsufficientFunds))
    );
    s.escrow.revoke_claim_ticket(&s.depositor, &id);
    s.escrow.refund(&BOUNTY);
    assert_eq!(s.token.balance(&s.depositor), 10_000);
}

#[test]
fn test_reservation_limits_pending_claim() {
    let s = Setup::new();
    s.issue(600);
    let other = Address::generate(&s.env);
    s.escrow
        .authorize_claim(&BOUNTY, &other, &DisputeReason::Other);
    assert_eq!(
        s.escrow.try_claim(&BOUNTY),
        Err(Ok(Error::InsufficientFunds))
    );
}

// ─── Revocation ──────────────────────────────────────────────────────────────

#[test]
fn test_revoke_frees_reservation_and_blocks_redeem() {
    let s = Setup::new();
    let id = s.issue(400);
    s.escrow.revoke_claim_ticket(&s.admin, &id);

    assert!(s.escrow.get_claim_ticket(&id).revoked);
    assert_eq!(s.escrow.get_ticket_reserved_amount(&BOUNTY), 0);
    assert_eq!(s.topic_count("ticket_r"), 1);
    assert_eq!(
        s.escrow.try_claim_with_ticket(&id, &s.beneficiary),
        Err(Ok(Error::TicketInvalid))
    );
    // Full amount can be released again.
    s.escrow.release_funds(&BOUNTY, &s.depositor);
}

#[test]
fn test_revoke_expired_ticket_frees_reservation() {
    let s = Setup::new();
    let id = s.issue(400);
    s.env.ledger().set_timestamp(s.now() + 10_000);
    s.escrow.revoke_claim_ticket(&s.depositor, &id);
    assert_eq!(s.escrow.get_ticket_reserved_amount(&BOUNTY), 0);
}

#[test]
fn test_revoke_by_stranger_rejected() {
    let s = Setup::new();
    let id = s.issue(400);
    assert_eq!(
        s.escrow.try_revoke_claim_ticket(&s.beneficiary, &id),
        Err(Ok(Error::Unauthorized))
    );
}

#[test]
fn test_revoke_twice_rejected() {
    let s = Setup::new();
    let id = s.issue(400);
    s.escrow.revoke_claim_ticket(&s.admin, &id);
    assert_eq!(
        s.escrow.try_revoke_claim_ticket(&s.admin, &id),
        Err(Ok(Error::TicketInvalid))
    );
}

#[test]
fn test_revoke_used_ticket_rejected() {
    let s = Setup::new();
    let id = s.issue(400);
    s.escrow.claim_with_ticket(&id, &s.beneficiary);
    assert_eq!(
        s.escrow.try_revoke_claim_ticket(&s.admin, &id),
        Err(Ok(Error::TicketAlreadyUsed))
    );
}

#[test]
fn test_revoke_missing_ticket_rejected() {
    let s = Setup::new();
    assert_eq!(
        s.escrow.try_revoke_claim_ticket(&s.admin, &5),
        Err(Ok(Error::TicketNotFound))
    );
}

#[test]
fn test_issue_rejected_while_pending_claim_active() {
    let s = Setup::new();
    let other = Address::generate(&s.env);
    s.escrow
        .authorize_claim(&BOUNTY, &other, &DisputeReason::Other);
    let res = s.escrow.try_issue_claim_ticket(
        &s.depositor,
        &BOUNTY,
        &s.beneficiary,
        &100,
        &(s.now() + 100),
    );
    assert_eq!(res, Err(Ok(Error::ClaimPending)));
}

#[test]
fn test_full_beneficiary_index_does_not_block_issuance() {
    let s = Setup::new();
    s.env.cost_estimate().budget().reset_unlimited();
    for _ in 0..crate::claim_tickets::MAX_BENEFICIARY_INDEX {
        s.issue(1);
    }
    let id = s.issue(1);
    assert_eq!(id, crate::claim_tickets::MAX_BENEFICIARY_INDEX as u64 + 1);
    assert_eq!(s.escrow.get_claim_ticket(&id).amount, 1);
    let last_page = s.escrow.get_beneficiary_tickets(
        &s.beneficiary,
        &(crate::claim_tickets::MAX_BENEFICIARY_INDEX - 1),
        &10,
    );
    assert_eq!(last_page.len(), 1);
    s.escrow.claim_with_ticket(&id, &s.beneficiary);
}
