//! Refund workflows: admin approval, full/partial, capability-based, anonymous, oracle, auto-refund, and eligibility view.



use soroban_sdk::{symbol_short, token, Address, BytesN, Env};
use crate::{
    events, multitoken_invariants, reentrancy_guard,
    Capability, CapabilityAction, DataKey, Error, Escrow, EscrowStatus,
    RefundApproval, RefundEligibilityCode, RefundEligibilityView, RefundMode, RefundRecord,
    SimulationResult,
    REFUND_ELIGIBILITY_SCHEMA_VERSION_V1,
    events::{emit_funds_refunded, emit_refund_approval_consumed, emit_refund_approval_set,
             CriticalOperationOutcome, FundsRefunded, RefundApprovalConsumed, RefundApprovalSet,
             RefundTriggerType, EVENT_VERSION_V2},
    AnonymousEscrow,
    ClaimRecord,
};

// ─────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────


pub(crate) fn compute_refund_eligibility(env: &Env, bounty_id: u64) -> RefundEligibilityView {
    let now = env.ledger().timestamp();

    if crate::pause_freeze::check_paused(env, symbol_short!("refund")) {
        return RefundEligibilityView {
            eligible: false,
            code: RefundEligibilityCode::IneligibleRefundPaused,
            bounty_id,
            amount: 0,
            recipient: None,
            now,
            deadline: 0,
            approval_present: false,
        };
    }

    if env
        .storage()
        .persistent()
        .has(&DataKey::EscrowAnon(bounty_id))
    {
        let anon: AnonymousEscrow = env
            .storage()
            .persistent()
            .get(&DataKey::EscrowAnon(bounty_id))
            .unwrap();
        return RefundEligibilityView {
            eligible: false,
            code: RefundEligibilityCode::IneligibleAnonRequiresResolution,
            bounty_id,
            amount: 0,
            recipient: None,
            now,
            deadline: anon.deadline,
            approval_present: false,
        };
    }

    if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
        return RefundEligibilityView {
            eligible: false,
            code: RefundEligibilityCode::IneligibleBountyNotFound,
            bounty_id,
            amount: 0,
            recipient: None,
            now,
            deadline: 0,
            approval_present: false,
        };
    }

    let escrow: Escrow = env
        .storage()
        .persistent()
        .get(&DataKey::Escrow(bounty_id))
        .unwrap();

    if crate::pause_freeze::ensure_escrow_not_frozen(env, bounty_id).is_err() {
        return RefundEligibilityView {
            eligible: false,
            code: RefundEligibilityCode::IneligibleEscrowFrozen,
            bounty_id,
            amount: 0,
            recipient: None,
            now,
            deadline: escrow.deadline,
            approval_present: false,
        };
    }
    if crate::pause_freeze::ensure_address_not_frozen(env, &escrow.depositor).is_err() {
        return RefundEligibilityView {
            eligible: false,
            code: RefundEligibilityCode::IneligibleAddressFrozen,
            bounty_id,
            amount: 0,
            recipient: None,
            now,
            deadline: escrow.deadline,
            approval_present: false,
        };
    }
    if escrow.status != EscrowStatus::Locked && escrow.status != EscrowStatus::PartiallyRefunded
    {
        return RefundEligibilityView {
            eligible: false,
            code: RefundEligibilityCode::IneligibleInvalidStatus,
            bounty_id,
            amount: 0,
            recipient: None,
            now,
            deadline: escrow.deadline,
            approval_present: false,
        };
    }

    if env
        .storage()
        .persistent()
        .has(&DataKey::PendingClaim(bounty_id))
    {
        let claim: ClaimRecord = env
            .storage()
            .persistent()
            .get(&DataKey::PendingClaim(bounty_id))
            .unwrap();
        if !claim.claimed {
            return RefundEligibilityView {
                eligible: false,
                code: RefundEligibilityCode::IneligibleClaimPending,
                bounty_id,
                amount: 0,
                recipient: None,
                now,
                deadline: escrow.deadline,
                approval_present: false,
            };
        }
    }

    let approval: Option<RefundApproval> = env
        .storage()
        .persistent()
        .get(&DataKey::RefundApproval(bounty_id));
    if let Some(app) = approval {
        if app.amount <= 0 || app.amount > escrow.remaining_amount {
            return RefundEligibilityView {
                eligible: false,
                code: RefundEligibilityCode::IneligibleInvalidApproval,
                bounty_id,
                amount: 0,
                recipient: None,
                now,
                deadline: escrow.deadline,
                approval_present: true,
            };
        }
        return RefundEligibilityView {
            eligible: true,
            code: RefundEligibilityCode::EligibleAdminApproval,
            bounty_id,
            amount: app.amount,
            recipient: Some(app.recipient),
            now,
            deadline: escrow.deadline,
            approval_present: true,
        };
    }

    if now >= escrow.deadline {
        return RefundEligibilityView {
            eligible: true,
            code: RefundEligibilityCode::EligibleDeadlinePassed,
            bounty_id,
            amount: escrow.remaining_amount,
            recipient: Some(escrow.depositor),
            now,
            deadline: escrow.deadline,
            approval_present: false,
        };
    }

    RefundEligibilityView {
        eligible: false,
        code: RefundEligibilityCode::IneligibleDeadlineNotPassed,
        bounty_id,
        amount: 0,
        recipient: None,
        now,
        deadline: escrow.deadline,
        approval_present: false,
    }
}


pub(crate) fn dry_run_refund_impl(env: &Env, bounty_id: u64) -> Result<(i128, EscrowStatus, i128), Error> {
    if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
        return Err(Error::BountyNotFound);
    }
    let escrow: Escrow = env
        .storage()
        .persistent()
        .get(&DataKey::Escrow(bounty_id))
        .unwrap();
    let eligibility = compute_refund_eligibility(env, bounty_id);
    if !eligibility.eligible {
        return Err(match eligibility.code {
            RefundEligibilityCode::IneligibleRefundPaused => Error::FundsPaused,
            RefundEligibilityCode::IneligibleBountyNotFound => Error::BountyNotFound,
            RefundEligibilityCode::IneligibleAnonRequiresResolution => {
                Error::AnonRefundRequiresResolution
            }
            RefundEligibilityCode::IneligibleEscrowFrozen => Error::EscrowFrozen,
            RefundEligibilityCode::IneligibleAddressFrozen => Error::AddressFrozen,
            RefundEligibilityCode::IneligibleInvalidStatus => Error::FundsNotLocked,
            RefundEligibilityCode::IneligibleClaimPending => Error::ClaimPending,
            RefundEligibilityCode::IneligibleDeadlineNotPassed => Error::DeadlineNotPassed,
            RefundEligibilityCode::IneligibleInvalidApproval => Error::InvalidAmount,
            RefundEligibilityCode::EligibleDeadlinePassed
            | RefundEligibilityCode::EligibleAdminApproval => Error::InvalidAmount,
        });
    }
    let refund_amount = eligibility.amount;
    let remaining_after = escrow
        .remaining_amount
        .checked_sub(refund_amount)
        .unwrap_or(0);
    let resulting_status = if remaining_after == 0 {
        EscrowStatus::Refunded
    } else {
        EscrowStatus::PartiallyRefunded
    };
    Ok((refund_amount, resulting_status, remaining_after))
}

// ─────────────────────────────────────────────────────────────────
// Public entry points (dispatcher targets)
// ─────────────────────────────────────────────────────────────────


/// Backward-compatible refund-eligibility tuple view.
/// Returns `(can_refund, deadline_passed, remaining_amount, approval)`.
pub fn get_refund_eligibility(
    env: Env,
    bounty_id: u64,
) -> (bool, bool, i128, Option<RefundApproval>) {
    let view = compute_refund_eligibility(&env, bounty_id);
    let approval: Option<RefundApproval> = env
        .storage()
        .persistent()
        .get(&DataKey::RefundApproval(bounty_id));
    let deadline_passed = view.deadline > 0 && view.now >= view.deadline;
    (
        view.eligible,
        deadline_passed,
        view.amount,
        if view.approval_present {
            approval
        } else {
            None
        },
    )
}


/// New typed refund-eligibility view with explicit semantics.
/// Implements issue #1040: Add refund eligibility view with clear semantics.
pub fn get_refund_eligibility_view(env: Env, bounty_id: u64) -> RefundEligibilityView {
    compute_refund_eligibility(&env, bounty_id)
}


/// Return the refund-eligibility view storage schema version written during `init`.
///
/// A value of `0` identifies a legacy deployment that was initialized before the
/// explicit refund-eligibility schema marker existed.
pub fn get_refund_schema_version(env: Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::RefundEligibilitySchemaVersion)
        .unwrap_or(0u32)
}


/// Approve a refund before deadline (admin only).
/// This allows early refunds with admin approval.
pub fn approve_refund(
    env: Env,
    bounty_id: u64,
    amount: i128,
    recipient: Address,
    mode: RefundMode,
) -> Result<(), Error> {
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }

    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();

    if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
        return Err(Error::BountyNotFound);
    }

    let escrow: Escrow = env
        .storage()
        .persistent()
        .get(&DataKey::Escrow(bounty_id))
        .unwrap();

    if escrow.status != EscrowStatus::Locked && escrow.status != EscrowStatus::PartiallyRefunded
    {
        return Err(Error::FundsNotLocked);
    }

    if amount <= 0 || amount > escrow.remaining_amount {
        return Err(Error::InvalidAmount);
    }

    let approval = RefundApproval {
        bounty_id,
        amount,
        recipient: recipient.clone(),
        mode: mode.clone(),
        approved_by: admin.clone(),
        approved_at: env.ledger().timestamp(),
    };

    env.storage()
        .persistent()
        .set(&DataKey::RefundApproval(bounty_id), &approval);

    emit_refund_approval_set(
        &env,
        RefundApprovalSet {
            version: EVENT_VERSION_V2,
            bounty_id,
            amount,
            recipient,
            mode,
            approved_by: admin,
            approved_at: env.ledger().timestamp(),
        },
    );

    // INV-2: Verify aggregate balance matches token balance after partial release
    multitoken_invariants::assert_after_disbursement(&env);
    Ok(())
}


/// Refunds remaining funds when refund conditions are met.
///
/// # Authorization
/// Refund execution requires authenticated authorization from the contract admin
/// and the escrow depositor.
///
/// # Eligibility
/// Refund is allowed when either:
/// 1. The deadline has passed (standard full refund to depositor), or
/// 2. An admin approval exists (early, partial, or custom-recipient refund).
///
/// # Transition Guards
/// This function enforces the following state transition guards:
///
/// ## Pre-conditions (checked in order):
/// 1. **Reentrancy Guard**: Acquires reentrancy lock to prevent concurrent execution
/// 2. **Operational State**: Contract must not be paused for refund operations
/// 3. **Escrow Existence**: Bounty must exist in storage
/// 4. **Freeze Check**: Escrow and depositor must not be frozen
/// 5. **Authorization**: Both admin and depositor must authorize the transaction
/// 6. **Status Guard**: Escrow status must be `Locked` or `PartiallyRefunded`
/// 7. **Claim Guard**: No pending claim exists (or claim is already executed)
/// 8. **Deadline/Approval Guard**: Deadline has passed OR admin approval exists
///
/// ## State Transition:
/// - **From**: `Locked` or `PartiallyRefunded`
/// - **To**: `Refunded` (if full refund) or `PartiallyRefunded` (if partial)
/// - **Effect**: Decrements `remaining_amount` by refund amount
///
/// ## Post-conditions:
/// - External token transfer to refund recipient (after state update)
/// - Refund record added to history
/// - Approval removed (if applicable)
/// - Event emission
///
/// ## Contention Safety:
/// - If status is `Released` or `Refunded`, returns `Error::FundsNotLocked`
/// - Reentrancy guard prevents concurrent execution of any protected function
/// - CEI pattern ensures state is updated before external calls
/// - No double-spend: once refunded, release fails with `Error::FundsNotLocked`
///
/// # Errors
/// Returns `Error::NotInitialized` if admin is not set.
pub fn refund(env: Env, bounty_id: u64) -> Result<(), Error> {
    // GUARD: acquire reentrancy lock
    reentrancy_guard::acquire(&env);

    if crate::pause_freeze::check_paused(&env, symbol_short!("refund")) {
        reentrancy_guard::release(&env);
        return Err(Error::FundsPaused);
    }
    // Snapshot resource meters for gas cap enforcement (test / testutils only).
    #[cfg(any(test, feature = "testutils"))]
    let gas_snapshot = gas_budget::capture(&env);

    if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
        reentrancy_guard::release(&env);
        return Err(Error::BountyNotFound);
    }

    let mut escrow: Escrow = env
        .storage()
        .persistent()
        .get(&DataKey::Escrow(bounty_id))
        .unwrap();

    crate::pause_freeze::ensure_escrow_not_frozen(&env, bounty_id)?;
    crate::pause_freeze::ensure_address_not_frozen(&env, &escrow.depositor)?;

    // Require authenticated approval from both admin and depositor.
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(Error::NotInitialized)?;
    admin.require_auth();
    escrow.depositor.require_auth();

    if escrow.status != EscrowStatus::Locked && escrow.status != EscrowStatus::PartiallyRefunded
    {
        reentrancy_guard::release(&env);
        return Err(Error::FundsNotLocked);
    }

    // Block refund if there is a pending claim (Issue #391 fix)
    if env
        .storage()
        .persistent()
        .has(&DataKey::PendingClaim(bounty_id))
    {
        let claim: ClaimRecord = env
            .storage()
            .persistent()
            .get(&DataKey::PendingClaim(bounty_id))
            .unwrap();
        if !claim.claimed {
            reentrancy_guard::release(&env);
            return Err(Error::ClaimPending);
        }
    }

    let now = env.ledger().timestamp();
    let approval_key = DataKey::RefundApproval(bounty_id);
    let approval: Option<RefundApproval> = env.storage().persistent().get(&approval_key);

    // Refund is allowed if:
    // 1. Deadline has passed (returns full amount to depositor)
    // 2. An administrative approval exists (can be early, partial, and to custom recipient)
    if now < escrow.deadline && approval.is_none() {
        reentrancy_guard::release(&env);
        return Err(Error::DeadlineNotPassed);
    }

    let (refund_amount, refund_to, is_full) = if let Some(app) = approval.clone() {
        let full = app.mode == RefundMode::Full || app.amount >= escrow.remaining_amount;
        (app.amount, app.recipient, full)
    } else {
        // Standard refund after deadline
        (escrow.remaining_amount, escrow.depositor.clone(), true)
    };

    if refund_amount <= 0 || refund_amount > escrow.remaining_amount {
        reentrancy_guard::release(&env);
        return Err(Error::InvalidAmount);
    }

    // EFFECTS: update state before external call (CEI)
    invariants::assert_escrow(&env, &escrow);
    // Update escrow state: subtract the amount exactly refunded
    escrow.remaining_amount = escrow.remaining_amount.checked_sub(refund_amount).unwrap();
    if is_full || escrow.remaining_amount == 0 {
        escrow.status = EscrowStatus::Refunded;
    } else {
        escrow.status = EscrowStatus::PartiallyRefunded;
    }

    // Add to refund history
    escrow.refund_history.push_back(RefundRecord {
        amount: refund_amount,
        recipient: refund_to.clone(),
        timestamp: now,
        mode: if is_full {
            RefundMode::Full
        } else {
            RefundMode::Partial
        },
    });

    // Save updated escrow
    env.storage()
        .persistent()
        .set(&DataKey::Escrow(bounty_id), &escrow);
    crate::lock::renew_escrow_record(
        &env,
        bounty_id,
        escrow.status == EscrowStatus::Refunded,
    );

    // Remove approval after successful execution
    if approval.is_some() {
        env.storage().persistent().remove(&approval_key);
        emit_refund_approval_consumed(
            &env,
            RefundApprovalConsumed {
                version: EVENT_VERSION_V2,
                bounty_id,
                refunded_amount: refund_amount,
                refunded_to: refund_to.clone(),
                consumed_at: now,
            },
        );
    }

    // INTERACTION: external token transfer is last
    let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
    let client = token::Client::new(&env, &token_addr);
    client.transfer(&env.current_contract_address(), &refund_to, &refund_amount);

    emit_funds_refunded(
        &env,
        FundsRefunded {
            version: EVENT_VERSION_V2,
            bounty_id,
            amount: refund_amount,
            refund_to: refund_to.clone(),
            timestamp: now,
            trigger_type: if approval.is_some() {
                RefundTriggerType::AdminApproval
            } else {
                RefundTriggerType::DeadlineExpired
            },
            correlation_id: None,
        },
    );
    crate::capability::record_receipt(
        &env,
        CriticalOperationOutcome::Refunded,
        bounty_id,
        refund_amount,
        refund_to.clone(),
    );

    // INV-2: Verify aggregate balance matches token balance after refund
    multitoken_invariants::assert_after_disbursement(&env);

    #[cfg(any(test, feature = "testutils"))]
    {
        let gas_cfg = gas_budget::get_config(&env);
        gas_budget::check(
            &env,
            symbol_short!("refund"),
            &gas_cfg.refund,
            &gas_snapshot,
            gas_cfg.enforce,
        )?;
    }

    // GUARD: release reentrancy lock
    reentrancy_guard::release(&env);
    Ok(())
}


/// Simulate refund operation without state changes or token transfers.
///
/// Returns a `SimulationResult` indicating whether the operation would succeed and the
/// resulting escrow state. Does not require authorization; safe for off-chain preview.
///
/// # Arguments
/// * `bounty_id` - Bounty identifier
///
/// # Security
/// This function performs only read operations. No storage writes, token transfers,
/// or events are emitted.
pub fn dry_run_refund(env: Env, bounty_id: u64) -> SimulationResult {
    fn err_result(e: Error, default_status: EscrowStatus) -> SimulationResult {
        SimulationResult {
            success: false,
            error_code: e as u32,
            amount: 0,
            resulting_status: default_status,
            remaining_amount: 0,
        }
    }
    match dry_run_refund_impl(&env, bounty_id) {
        Ok((refund_amount, resulting_status, remaining_amount)) => SimulationResult {
            success: true,
            error_code: 0,
            amount: refund_amount,
            resulting_status,
            remaining_amount,
        },
        Err(e) => err_result(e, EscrowStatus::Refunded),
    }
}


/// Sets or clears the anonymous resolver address.
/// Only the admin can call this. The resolver is the trusted entity that
/// resolves anonymous escrow refunds via `refund_resolved`.
pub fn set_anonymous_resolver(env: Env, resolver: Option<Address>) -> Result<(), Error> {
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }
    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();

    match resolver {
        Some(addr) => env
            .storage()
            .instance()
            .set(&DataKey::AnonymousResolver, &addr),
        None => env.storage().instance().remove(&DataKey::AnonymousResolver),
    }
    // INV-2: Verify aggregate balance matches token balance after capability refund
    multitoken_invariants::assert_after_disbursement(&env);
    Ok(())
}


/// Refund an anonymous escrow to a resolved recipient.
/// Only the configured anonymous resolver can call this; they resolve the depositor
/// commitment off-chain and pass the recipient address (signed instruction pattern).
pub fn refund_resolved(env: Env, bounty_id: u64, recipient: Address) -> Result<(), Error> {
    if crate::pause_freeze::check_paused(&env, symbol_short!("refund")) {
        return Err(Error::FundsPaused);
    }

    let resolver: Address = env
        .storage()
        .instance()
        .get(&DataKey::AnonymousResolver)
        .ok_or(Error::AnonymousResolverNotSet)?;
    resolver.require_auth();

    if !env
        .storage()
        .persistent()
        .has(&DataKey::EscrowAnon(bounty_id))
    {
        return Err(Error::NotAnonymousEscrow);
    }

    reentrancy_guard::acquire(&env);

    let mut anon: AnonymousEscrow = env
        .storage()
        .persistent()
        .get(&DataKey::EscrowAnon(bounty_id))
        .unwrap();

    crate::pause_freeze::ensure_escrow_not_frozen(&env, bounty_id)?;

    if anon.status != EscrowStatus::Locked && anon.status != EscrowStatus::PartiallyRefunded {
        reentrancy_guard::release(&env);
        return Err(Error::FundsNotLocked);
    }

    // GUARD 1: Block refund if there is a pending claim (Issue #391 fix)
    if env
        .storage()
        .persistent()
        .has(&DataKey::PendingClaim(bounty_id))
    {
        let claim: ClaimRecord = env
            .storage()
            .persistent()
            .get(&DataKey::PendingClaim(bounty_id))
            .unwrap();
        if !claim.claimed {
            reentrancy_guard::release(&env);
            return Err(Error::ClaimPending);
        }
    }

    let now = env.ledger().timestamp();
    let approval_key = DataKey::RefundApproval(bounty_id);
    let approval: Option<RefundApproval> = env.storage().persistent().get(&approval_key);

    // Refund is allowed if:
    // 1. Deadline has passed (returns full amount to depositor)
    // 2. An administrative approval exists (can be early, partial, and to custom recipient)
    if now < anon.deadline && approval.is_none() {
        reentrancy_guard::release(&env);
        return Err(Error::DeadlineNotPassed);
    }

    let (refund_amount, refund_to, is_full) = if let Some(app) = approval.clone() {
        let full = app.mode == RefundMode::Full || app.amount >= anon.remaining_amount;
        (app.amount, app.recipient, full)
    } else {
        // Standard refund after deadline
        (anon.remaining_amount, recipient.clone(), true)
    };

    if refund_amount <= 0 || refund_amount > anon.remaining_amount {
        reentrancy_guard::release(&env);
        return Err(Error::InvalidAmount);
    }

    // EFFECTS: update escrow state before external call (CEI)
    // Update escrow state: subtract the amount exactly refunded
    anon.remaining_amount -= refund_amount;
    if is_full || anon.remaining_amount == 0 {
        anon.status = EscrowStatus::Refunded;
    } else {
        anon.status = EscrowStatus::PartiallyRefunded;
    }

    // Add to refund history
    anon.refund_history.push_back(RefundRecord {
        amount: refund_amount,
        recipient: refund_to.clone(),
        timestamp: now,
        mode: if is_full {
            RefundMode::Full
        } else {
            RefundMode::Partial
        },
    });

    // Save updated escrow
    env.storage()
        .persistent()
        .set(&DataKey::EscrowAnon(bounty_id), &anon);
    crate::lock::renew_escrow_record(
        &env,
        bounty_id,
        anon.status == EscrowStatus::Refunded,
    );

    // Remove approval after successful execution
    if approval.is_some() {
        env.storage().persistent().remove(&approval_key);
    }

    // INTERACTION: external token transfer after state finalized
    let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
    let client = token::Client::new(&env, &token_addr);
    client.transfer(&env.current_contract_address(), &refund_to, &refund_amount);

    emit_funds_refunded(
        &env,
        FundsRefunded {
            version: EVENT_VERSION_V2,
            bounty_id,
            amount: refund_amount,
            refund_to: refund_to.clone(),
            timestamp: now,
            trigger_type: if approval.is_some() {
                RefundTriggerType::AdminApproval
            } else {
                RefundTriggerType::DeadlineExpired
            },
            correlation_id: None,
        },
    );

    // GUARD: release reentrancy lock
    reentrancy_guard::release(&env);
    multitoken_invariants::assert_after_disbursement(&env);
    Ok(())
}


/// Delegated refund path using a capability.
/// This can be used for short-lived, bounded delegated refunds without granting admin rights.
pub fn refund_with_capability(
    env: Env,
    bounty_id: u64,
    amount: i128,
    holder: Address,
    capability_id: BytesN<32>,
) -> Result<(), Error> {
    // GUARD: acquire reentrancy lock
    reentrancy_guard::acquire(&env);

    if crate::pause_freeze::check_paused(&env, symbol_short!("refund")) {
        reentrancy_guard::release(&env);
        return Err(Error::FundsPaused);
    }
    if amount <= 0 {
        reentrancy_guard::release(&env);
        return Err(Error::InvalidAmount);
    }
    if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
        reentrancy_guard::release(&env);
        return Err(Error::BountyNotFound);
    }

    let mut escrow: Escrow = env
        .storage()
        .persistent()
        .get(&DataKey::Escrow(bounty_id))
        .unwrap();

    crate::pause_freeze::ensure_escrow_not_frozen(&env, bounty_id)?;
    crate::pause_freeze::ensure_address_not_frozen(&env, &escrow.depositor)?;

    if escrow.status != EscrowStatus::Locked && escrow.status != EscrowStatus::PartiallyRefunded
    {
        reentrancy_guard::release(&env);
        return Err(Error::FundsNotLocked);
    }
    if amount > escrow.remaining_amount {
        reentrancy_guard::release(&env);
        return Err(Error::InvalidAmount);
    }

    if env
        .storage()
        .persistent()
        .has(&DataKey::PendingClaim(bounty_id))
    {
        let claim: ClaimRecord = env
            .storage()
            .persistent()
            .get(&DataKey::PendingClaim(bounty_id))
            .unwrap();
        if !claim.claimed {
            reentrancy_guard::release(&env);
            return Err(Error::ClaimPending);
        }
    }

    crate::capability::consume_capability(
        &env,
        &holder,
        capability_id,
        CapabilityAction::Refund,
        bounty_id,
        amount,
    )?;

    // EFFECTS: update state before external call (CEI)
    let now = env.ledger().timestamp();
    let refund_to = escrow.depositor.clone();
    escrow.remaining_amount = escrow.remaining_amount.checked_sub(amount).unwrap();
    if escrow.remaining_amount == 0 {
        escrow.status = EscrowStatus::Refunded;
    } else {
        escrow.status = EscrowStatus::PartiallyRefunded;
    }
    escrow.refund_history.push_back(RefundRecord {
        amount,
        recipient: refund_to.clone(),
        timestamp: now,
        mode: if escrow.remaining_amount == 0 {
            RefundMode::Full
        } else {
            RefundMode::Partial
        },
    });
    env.storage()
        .persistent()
        .set(&DataKey::Escrow(bounty_id), &escrow);
    crate::lock::renew_escrow_record(
        &env,
        bounty_id,
        escrow.status == EscrowStatus::Refunded,
    );

    let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
    let client = token::Client::new(&env, &token_addr);
    client.transfer(&env.current_contract_address(), &refund_to, &amount);

    emit_funds_refunded(
        &env,
        FundsRefunded {
            version: EVENT_VERSION_V2,
            bounty_id,
            amount,
            refund_to,
            timestamp: now,
            trigger_type: RefundTriggerType::AdminApproval,
            correlation_id: None,
        },
    );

    reentrancy_guard::release(&env);
    multitoken_invariants::assert_after_disbursement(&env);
    Ok(())
}

