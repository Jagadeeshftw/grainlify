#![cfg(test)]

//! # ABI Compatibility Suite
//!
//! Pins the public contract surface so any breaking change causes an immediate
//! failure rather than a silent regression.
//!
//! ## Breaking changes
//! - Renumbering an `Error` variant
//! - Removing or renaming a public entrypoint
//! - Changing parameter types/order on a stable entrypoint
//! - Changing fields/types of a public `#[contracttype]` struct
//!
//! ## Non-breaking (additive)
//! - New `Error` variants at unused discriminants
//! - New entrypoints
//!
//! ## Migration notes
//! No breaking changes since initial release.

use crate::{
    BountyEscrowContract, BountyEscrowContractClient, CapabilityAction, DisputeOutcome,
    DisputeReason, Error, EscrowStatus, FeeConfig, RefundMode,
};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

fn setup(env: &Env) -> (Address, Address, token::Client<'_>, BountyEscrowContractClient<'_>) {
    let admin = Address::generate(env);
    let depositor = Address::generate(env);
    let token_addr = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let token = token::Client::new(env, &token_addr);
    let token_admin = token::StellarAssetClient::new(env, &token_addr);
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(env, &contract_id);
    env.mock_all_auths();
    client.init(&admin, &token_addr);
    token_admin.mint(&depositor, &100_000);
    (admin, depositor, token, client)
}

// --- Error discriminant stability ---

/// Pins every `Error` discriminant to its wire value.
/// Changing any of these is a BREAKING change requiring a major version bump.
#[test]
fn test_all_error_codes_stable() {
    assert_eq!(Error::AlreadyInitialized as u32, 1);
    assert_eq!(Error::NotInitialized as u32, 2);
    // 3, 4, 5 intentionally unassigned (BountyExists/BountyNotFound/FundsNotLocked moved to 55-57)
    assert_eq!(Error::DeadlineNotPassed as u32, 6);
    assert_eq!(Error::Unauthorized as u32, 7);
    assert_eq!(Error::InvalidFeeRate as u32, 8);
    assert_eq!(Error::FeeRecipientNotSet as u32, 9);
    assert_eq!(Error::InvalidBatchSize as u32, 10);
    assert_eq!(Error::BatchSizeMismatch as u32, 11);
    assert_eq!(Error::DuplicateBountyId as u32, 12);
    assert_eq!(Error::InvalidAmount as u32, 13);
    assert_eq!(Error::InvalidDeadline as u32, 14);
    // 15 intentionally unassigned
    assert_eq!(Error::InsufficientFunds as u32, 16);
    assert_eq!(Error::RefundNotApproved as u32, 17);
    assert_eq!(Error::FundsPaused as u32, 18);
    assert_eq!(Error::AmountBelowMinimum as u32, 19);
    assert_eq!(Error::AmountAboveMaximum as u32, 20);
    assert_eq!(Error::NotPaused as u32, 21);
    assert_eq!(Error::ClaimPending as u32, 22);
    assert_eq!(Error::TicketNotFound as u32, 23);
    assert_eq!(Error::TicketAlreadyUsed as u32, 24);
    assert_eq!(Error::TicketExpired as u32, 25);
    assert_eq!(Error::CapabilityNotFound as u32, 26);
    assert_eq!(Error::CapabilityExpired as u32, 27);
    assert_eq!(Error::CapabilityRevoked as u32, 28);
    assert_eq!(Error::CapabilityActionMismatch as u32, 29);
    assert_eq!(Error::CapabilityAmountExceeded as u32, 30);
    assert_eq!(Error::CapabilityUsesExhausted as u32, 31);
    assert_eq!(Error::CapabilityExceedsAuthority as u32, 32);
    assert_eq!(Error::InvalidAssetId as u32, 33);
    assert_eq!(Error::ContractDeprecated as u32, 34);
    assert_eq!(Error::ParticipantBlocked as u32, 35);
    assert_eq!(Error::ParticipantNotAllowed as u32, 36);
    // 37, 38 intentionally unassigned
    assert_eq!(Error::AnonRefundRequiresResolution as u32, 39);
    assert_eq!(Error::AnonymousResolverNotSet as u32, 40);
    assert_eq!(Error::NotAnonymousEscrow as u32, 41);
    // 42 intentionally unassigned
    assert_eq!(Error::UpgradeSafetyCheckFailed as u32, 43);
    assert_eq!(Error::GasBudgetExceeded as u32, 44);
    assert_eq!(Error::EscrowFrozen as u32, 45);
    assert_eq!(Error::AddressFrozen as u32, 46);
    assert_eq!(Error::AdminRotationAlreadyPending as u32, 47);
    assert_eq!(Error::AdminRotationNotPending as u32, 48);
    assert_eq!(Error::AdminRotationTimelockActive as u32, 49);
    assert_eq!(Error::InvalidAdminRotationTimelock as u32, 50);
    assert_eq!(Error::InvalidAdminRotationTarget as u32, 51);
    assert_eq!(Error::InvalidBatchSizeCap as u32, 52);
    assert_eq!(Error::TimelockNotElapsed as u32, 53);
    assert_eq!(Error::ReleaseAlreadyQueued as u32, 54);
    assert_eq!(Error::BountyExists as u32, 55);
    assert_eq!(Error::BountyNotFound as u32, 56);
    assert_eq!(Error::FundsNotLocked as u32, 57);
    assert_eq!(Error::RouterNotConfigured as u32, 58);
    assert_eq!(Error::SlippageExceeded as u32, 59);
    assert_eq!(Error::FeeRoutingLocked as u32, 60);
}

// --- Enum variant stability ---

/// `EscrowStatus` variants must remain distinct and named consistently.
#[test]
fn test_escrow_status_variants_stable() {
    assert_ne!(EscrowStatus::Locked, EscrowStatus::Released);
    assert_ne!(EscrowStatus::Locked, EscrowStatus::Refunded);
    assert_ne!(EscrowStatus::Locked, EscrowStatus::PartiallyRefunded);
    assert_ne!(EscrowStatus::Released, EscrowStatus::Refunded);
    assert_ne!(EscrowStatus::Released, EscrowStatus::PartiallyRefunded);
    assert_ne!(EscrowStatus::Refunded, EscrowStatus::PartiallyRefunded);
}

/// `RefundMode` variants must remain distinct.
#[test]
fn test_refund_mode_variants_stable() {
    assert_ne!(RefundMode::Full, RefundMode::Partial);
}

/// `DisputeReason` discriminants are part of the on-chain event schema.
#[test]
fn test_dispute_reason_discriminants_stable() {
    assert_eq!(DisputeReason::Expired as u32, 1);
    assert_eq!(DisputeReason::UnsatisfactoryWork as u32, 2);
    assert_eq!(DisputeReason::Fraud as u32, 3);
    assert_eq!(DisputeReason::QualityIssue as u32, 4);
    assert_eq!(DisputeReason::Other as u32, 5);
}

/// `DisputeOutcome` discriminants are part of the on-chain event schema.
#[test]
fn test_dispute_outcome_discriminants_stable() {
    assert_eq!(DisputeOutcome::ResolvedInFavorOfContributor as u32, 1);
    assert_eq!(DisputeOutcome::ResolvedInFavorOfDepositor as u32, 2);
    assert_eq!(DisputeOutcome::CancelledByAdmin as u32, 3);
    assert_eq!(DisputeOutcome::Refunded as u32, 4);
}

/// `CapabilityAction` variants must remain distinct.
#[test]
fn test_capability_action_variants_stable() {
    assert_ne!(CapabilityAction::Claim, CapabilityAction::Release);
    assert_ne!(CapabilityAction::Claim, CapabilityAction::Refund);
    assert_ne!(CapabilityAction::Release, CapabilityAction::Refund);
}

// --- Struct field stability ---

/// `FeeConfig` fields must remain accessible with the same types.
#[test]
fn test_fee_config_fields_stable() {
    let env = Env::default();
    let (_, _, _, client) = setup(&env);
    let cfg: FeeConfig = client.get_fee_config();
    let _: i128 = cfg.lock_fee_rate;
    let _: i128 = cfg.release_fee_rate;
    let _: Address = cfg.fee_recipient;
    let _: bool = cfg.fee_enabled;
}

/// `Escrow` struct fields must remain accessible with the same types.
#[test]
fn test_escrow_struct_fields_stable() {
    let env = Env::default();
    let (_, depositor, _, client) = setup(&env);
    client.lock_funds(&depositor, &1, &1000, &9999);
    let e = client.get_escrow_info(&1);
    let _: Address = e.depositor;
    let _: i128 = e.amount;
    let _: i128 = e.remaining_amount;
    let _: EscrowStatus = e.status;
    let _: u64 = e.deadline;
    let _: u32 = e.refund_history.len();
}

// --- Entrypoint behavioural contracts ---

/// `init` called twice returns `AlreadyInitialized`.
#[test]
fn test_init_idempotent_guard() {
    let env = Env::default();
    let (admin, _, _, client) = setup(&env);
    let token2 = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    assert_eq!(
        client.try_init(&admin, &token2).unwrap_err().unwrap(),
        Error::AlreadyInitialized
    );
}

/// `lock_funds` with a duplicate bounty_id returns `BountyExists`.
#[test]
fn test_lock_funds_duplicate_id_returns_bounty_exists() {
    let env = Env::default();
    let (_, depositor, _, client) = setup(&env);
    client.lock_funds(&depositor, &1, &500, &9999);
    assert_eq!(
        client
            .try_lock_funds(&depositor, &1, &500, &9999)
            .unwrap_err()
            .unwrap(),
        Error::BountyExists
    );
}

/// `release_funds` on a missing bounty returns `BountyNotFound`.
#[test]
fn test_release_funds_missing_bounty() {
    let env = Env::default();
    let (_, _, _, client) = setup(&env);
    let contributor = Address::generate(&env);
    assert_eq!(
        client
            .try_release_funds(&9999, &contributor)
            .unwrap_err()
            .unwrap(),
        Error::BountyNotFound
    );
}

/// `release_funds` on an already-released escrow returns `FundsNotLocked`.
#[test]
fn test_release_funds_double_release_returns_funds_not_locked() {
    let env = Env::default();
    let (_, depositor, _, client) = setup(&env);
    let contributor = Address::generate(&env);
    client.lock_funds(&depositor, &1, &1000, &9999);
    client.release_funds(&1, &contributor);
    assert_eq!(
        client
            .try_release_funds(&1, &contributor)
            .unwrap_err()
            .unwrap(),
        Error::FundsNotLocked
    );
}

/// `refund` before deadline without approval returns `DeadlineNotPassed`.
#[test]
fn test_refund_before_deadline_no_approval() {
    let env = Env::default();
    let (_, depositor, _, client) = setup(&env);
    let deadline = env.ledger().timestamp() + 1000;
    client.lock_funds(&depositor, &1, &1000, &deadline);
    assert_eq!(
        client.try_refund(&1).unwrap_err().unwrap(),
        Error::DeadlineNotPassed
    );
}

/// `refund` after deadline succeeds and status becomes `Refunded`.
#[test]
fn test_refund_after_deadline_status_is_refunded() {
    let env = Env::default();
    let (_, depositor, token, client) = setup(&env);
    let deadline = env.ledger().timestamp() + 100;
    client.lock_funds(&depositor, &1, &1000, &deadline);
    env.ledger().set_timestamp(deadline + 1);
    client.refund(&1);
    assert_eq!(client.get_escrow_info(&1).status, EscrowStatus::Refunded);
    assert_eq!(token.balance(&depositor), 100_000);
}

/// Admin-approved early refund succeeds before deadline.
#[test]
fn test_approve_refund_before_deadline_succeeds() {
    let env = Env::default();
    let (_, depositor, token, client) = setup(&env);
    let deadline = env.ledger().timestamp() + 9999;
    client.lock_funds(&depositor, &1, &2000, &deadline);
    client.approve_refund(&1, &2000, &depositor, &RefundMode::Full);
    client.refund(&1);
    assert_eq!(client.get_escrow_info(&1).status, EscrowStatus::Refunded);
    assert_eq!(token.balance(&depositor), 100_000);
}

/// `get_refund_eligibility` returns correct flags across the lifecycle.
#[test]
fn test_get_refund_eligibility_flags_lifecycle() {
    let env = Env::default();
    let (_, depositor, _, client) = setup(&env);
    let deadline = env.ledger().timestamp() + 500;
    client.lock_funds(&depositor, &1, &1000, &deadline);

    let (can, passed, remaining, approval) = client.get_refund_eligibility(&1);
    assert!(!can);
    assert!(!passed);
    // `amount` is 0 whenever the escrow is not refund-eligible (deadline not
    // passed, no admin approval); it only reports `remaining_amount` once
    // `eligible` is true.
    assert_eq!(remaining, 0);
    assert!(approval.is_none());

    env.ledger().set_timestamp(deadline + 1);
    let (can, passed, remaining, _) = client.get_refund_eligibility(&1);
    assert!(can);
    assert!(passed);
    assert_eq!(remaining, 1000);

    client.refund(&1);
    let (can, _, _, _) = client.get_refund_eligibility(&1);
    assert!(!can);
}

/// `get_balance` tracks locked and released amounts.
#[test]
fn test_get_balance_tracks_escrow_state() {
    let env = Env::default();
    let (_, depositor, _, client) = setup(&env);
    let contributor = Address::generate(&env);
    assert_eq!(client.get_balance(), 0);
    client.lock_funds(&depositor, &1, &3000, &9999);
    assert_eq!(client.get_balance(), 3000);
    client.release_funds(&1, &contributor);
    assert_eq!(client.get_balance(), 0);
}

/// `update_fee_config` with all-`None` is a no-op.
#[test]
fn test_update_fee_config_none_is_noop() {
    let env = Env::default();
    let (_, _, _, client) = setup(&env);
    client.update_fee_config(&None, &None, &None, &None, &None, &None);
    let cfg = client.get_fee_config();
    assert_eq!(cfg.lock_fee_rate, 0);
    assert_eq!(cfg.release_fee_rate, 0);
    assert!(!cfg.fee_enabled);
}

/// `set_paused` / `get_pause_flags` round-trip is stable.
#[test]
fn test_pause_flags_round_trip() {
    let env = Env::default();
    let (_, _, _, client) = setup(&env);
    client.set_paused(&Some(true), &None, &None, &None);
    let flags = client.get_pause_flags();
    assert!(flags.lock_paused);
    assert!(!flags.release_paused);
    assert!(!flags.refund_paused);
    client.set_paused(&Some(false), &None, &None, &None);
    assert!(!client.get_pause_flags().lock_paused);
}

/// `lock_funds` while lock-paused returns `FundsPaused`.
#[test]
fn test_lock_while_paused_returns_funds_paused() {
    let env = Env::default();
    let (_, depositor, _, client) = setup(&env);
    client.set_paused(&Some(true), &None, &None, &None);
    assert_eq!(
        client
            .try_lock_funds(&depositor, &1, &1000, &9999)
            .unwrap_err()
            .unwrap(),
        Error::FundsPaused
    );
}

/// `release_funds` while release-paused returns `FundsPaused`.
#[test]
fn test_release_while_paused_returns_funds_paused() {
    let env = Env::default();
    let (_, depositor, _, client) = setup(&env);
    let contributor = Address::generate(&env);
    client.lock_funds(&depositor, &1, &1000, &9999);
    client.set_paused(&None, &Some(true), &None, &None);
    assert_eq!(
        client
            .try_release_funds(&1, &contributor)
            .unwrap_err()
            .unwrap(),
        Error::FundsPaused
    );
}

/// `set_deprecated` blocks new locks but existing escrows can still settle.
#[test]
fn test_deprecated_blocks_new_locks_existing_settles() {
    let env = Env::default();
    let (_, depositor, _, client) = setup(&env);
    let contributor = Address::generate(&env);
    client.lock_funds(&depositor, &1, &1000, &9999);
    client.set_deprecated(&true, &None);
    assert_eq!(
        client
            .try_lock_funds(&depositor, &2, &1000, &9999)
            .unwrap_err()
            .unwrap(),
        Error::ContractDeprecated
    );
    client.release_funds(&1, &contributor);
    assert_eq!(client.get_escrow_info(&1).status, EscrowStatus::Released);
}

/// `partial_release` decrements `remaining_amount`; transitions to `Released` when drained.
#[test]
fn test_partial_release_remaining_and_status() {
    let env = Env::default();
    let (_, depositor, _, client) = setup(&env);
    let contributor = Address::generate(&env);
    client.lock_funds(&depositor, &1, &1000, &9999);
    client.partial_release(&1, &contributor, &600);
    let e = client.get_escrow_info(&1);
    assert_eq!(e.remaining_amount, 400);
    assert_eq!(e.status, EscrowStatus::Locked);
    client.partial_release(&1, &contributor, &400);
    let e = client.get_escrow_info(&1);
    assert_eq!(e.remaining_amount, 0);
    assert_eq!(e.status, EscrowStatus::Released);
}

/// `dry_run_lock` does not create an escrow.
#[test]
fn test_dry_run_lock_is_read_only() {
    let env = Env::default();
    let (_, depositor, _, client) = setup(&env);
    let result = client.dry_run_lock(&depositor, &1, &1000, &9999);
    assert!(result.success);
    assert!(client.try_get_escrow_info(&1).is_err());
}

/// `dry_run_release` does not change escrow status.
#[test]
fn test_dry_run_release_is_read_only() {
    let env = Env::default();
    let (_, depositor, _, client) = setup(&env);
    let contributor = Address::generate(&env);
    client.lock_funds(&depositor, &1, &1000, &9999);
    let result = client.dry_run_release(&1, &contributor);
    assert!(result.success);
    assert_eq!(client.get_escrow_info(&1).status, EscrowStatus::Locked);
}

/// `get_refund_history` is empty before any refund and grows by one per call.
#[test]
fn test_refund_history_grows_per_refund() {
    let env = Env::default();
    let (_, depositor, _, client) = setup(&env);
    let deadline = env.ledger().timestamp() + 100;
    client.lock_funds(&depositor, &1, &1000, &deadline);
    assert_eq!(client.get_refund_history(&1).len(), 0);

    client.approve_refund(&1, &400, &depositor, &RefundMode::Partial);
    client.refund(&1);
    assert_eq!(client.get_refund_history(&1).len(), 1);

    env.ledger().set_timestamp(deadline + 1);
    client.refund(&1);
    assert_eq!(client.get_refund_history(&1).len(), 2);
}

/// `get_deprecation_status` default is not deprecated; toggles correctly.
#[test]
fn test_deprecation_status_stable() {
    let env = Env::default();
    let (_, _, _, client) = setup(&env);
    let s = client.get_deprecation_status();
    assert!(!s.deprecated);
    assert!(s.migration_target.is_none());
    client.set_deprecated(&true, &None);
    assert!(client.get_deprecation_status().deprecated);
}

// --- Additional entrypoint coverage ---

/// `batch_lock_funds` locks multiple escrows atomically.
#[test]
fn test_batch_lock_funds_stable() {
    use crate::LockFundsItem;
    use soroban_sdk::vec;

    let env = Env::default();
    let (_, depositor, _, client) = setup(&env);

    let items = vec![
        &env,
        LockFundsItem {
            bounty_id: 10,
            depositor: depositor.clone(),
            amount: 500,
            deadline: 9999,
        },
        LockFundsItem {
            bounty_id: 11,
            depositor: depositor.clone(),
            amount: 500,
            deadline: 9999,
        },
    ];
    let count = client.batch_lock_funds(&items);
    assert_eq!(count, 2);
    assert_eq!(client.get_escrow_info(&10).status, EscrowStatus::Locked);
    assert_eq!(client.get_escrow_info(&11).status, EscrowStatus::Locked);
}

/// `batch_release_funds` releases multiple escrows atomically.
#[test]
fn test_batch_release_funds_stable() {
    use crate::{LockFundsItem, ReleaseFundsItem};
    use soroban_sdk::vec;

    let env = Env::default();
    let (_, depositor, _, client) = setup(&env);
    let contributor = Address::generate(&env);

    let lock_items = vec![
        &env,
        LockFundsItem {
            bounty_id: 20,
            depositor: depositor.clone(),
            amount: 500,
            deadline: 9999,
        },
        LockFundsItem {
            bounty_id: 21,
            depositor: depositor.clone(),
            amount: 500,
            deadline: 9999,
        },
    ];
    client.batch_lock_funds(&lock_items);

    let release_items = vec![
        &env,
        ReleaseFundsItem {
            bounty_id: 20,
            contributor: contributor.clone(),
        },
        ReleaseFundsItem {
            bounty_id: 21,
            contributor: contributor.clone(),
        },
    ];
    let count = client.batch_release_funds(&release_items);
    assert_eq!(count, 2);
    assert_eq!(client.get_escrow_info(&20).status, EscrowStatus::Released);
    assert_eq!(client.get_escrow_info(&21).status, EscrowStatus::Released);
}

/// `set_claim_window` + `authorize_claim` + `claim` happy path is stable.
#[test]
fn test_claim_flow_stable() {
    use crate::DisputeReason;

    let env = Env::default();
    let (_, depositor, token, client) = setup(&env);
    let contributor = Address::generate(&env);

    client.set_claim_window(&500);
    client.lock_funds(&depositor, &1, &1000, &9999);
    client.authorize_claim(&1, &contributor, &DisputeReason::QualityIssue);

    let claim = client.get_pending_claim(&1);
    assert_eq!(claim.recipient, contributor);
    assert!(!claim.claimed);

    env.ledger().set_timestamp(claim.expires_at - 1);
    client.claim(&1);

    assert_eq!(client.get_escrow_info(&1).status, EscrowStatus::Released);
    assert_eq!(token.balance(&contributor), 1000);
}

/// `set_filter_mode` + `set_blocklist_entry` blocks a participant correctly.
#[test]
fn test_participant_filter_blocklist_stable() {
    use crate::ParticipantFilterMode;

    let env = Env::default();
    let (_, depositor, _, client) = setup(&env);

    client.set_filter_mode(&ParticipantFilterMode::BlocklistOnly);
    client.set_blocklist_entry(&depositor, &true);

    let res = client.try_lock_funds(&depositor, &1, &500, &9999);
    assert_eq!(res.unwrap_err().unwrap(), Error::ParticipantBlocked);

    client.set_blocklist_entry(&depositor, &false);
    client.lock_funds(&depositor, &1, &500, &9999);
}

/// `set_token_fee_config` / `get_token_fee_config` round-trip is stable.
#[test]
fn test_token_fee_config_round_trip_stable() {
    let env = Env::default();
    let (admin, _, _, client) = setup(&env);
    let token_addr = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();

    client.set_token_fee_config(&token_addr, &100, &50, &0, &0, &admin, &true);

    let cfg = client.get_token_fee_config(&token_addr).unwrap();
    assert_eq!(cfg.lock_fee_rate, 100);
    assert_eq!(cfg.release_fee_rate, 50);
    assert!(cfg.fee_enabled);
}

/// `update_metadata` / `get_metadata` round-trip is stable.
#[test]
fn test_metadata_round_trip_stable() {
    use soroban_sdk::String;

    let env = Env::default();
    let (admin, depositor, _, client) = setup(&env);

    client.lock_funds(&depositor, &1, &500, &9999);
    client.update_metadata(&admin, &1, &42, &7, &String::from_str(&env, "bug"), &None);

    let meta = client.get_metadata(&1);
    assert_eq!(meta.repo_id, 42);
    assert_eq!(meta.issue_id, 7);
}

/// `set_anonymous_resolver` + `lock_funds_anonymous` + `refund_resolved` path.
#[test]
fn test_anonymous_refund_flow_stable() {
    use soroban_sdk::BytesN;

    let env = Env::default();
    let (_, depositor, token, client) = setup(&env);
    let resolver = Address::generate(&env);
    let recipient = Address::generate(&env);

    client.set_anonymous_resolver(&Some(resolver.clone()));

    let commitment: BytesN<32> = BytesN::from_array(&env, &[1u8; 32]);
    let deadline = env.ledger().timestamp() + 100;
    client.lock_funds_anonymous(&depositor, &commitment, &1, &1000, &deadline);

    env.ledger().set_timestamp(deadline + 1);
    client.refund_resolved(&1, &recipient);

    assert_eq!(token.balance(&recipient), 1000);
}

/// `refund_with_capability` delegates a bounded refund without admin rights.
#[test]
fn test_refund_with_capability_stable() {
    let env = Env::default();
    let (admin, depositor, token, client) = setup(&env);
    let holder = Address::generate(&env);

    let deadline = env.ledger().timestamp() + 9999;
    client.lock_funds(&depositor, &1, &1000, &deadline);

    let cap_expiry = env.ledger().timestamp() + 500;
    let cap_id = client.issue_capability(
        &admin,
        &holder,
        &crate::CapabilityAction::Refund,
        &1,
        &500,
        &cap_expiry,
        &1,
    );

    client.refund_with_capability(&1, &500, &holder, &cap_id);

    let e = client.get_escrow_info(&1);
    assert_eq!(e.remaining_amount, 500);
    assert_eq!(e.status, EscrowStatus::PartiallyRefunded);
    assert_eq!(token.balance(&depositor), 100_000 - 500);
}
