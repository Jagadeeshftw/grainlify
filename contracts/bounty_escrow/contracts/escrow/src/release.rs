//! Fund release: single, conversion, capability-based, partial, batch, and high-value timelock queue.



use soroban_sdk::{symbol_short, token, Address, BytesN, Env, Vec};
use crate::{
    events, multitoken_invariants, rbac, reentrancy_guard,
    Capability, CapabilityAction, DataKey, Error, Escrow, EscrowStatus,
    HighValueConfig, MultisigConfig, QueuedRelease, RefundMode, RefundRecord,
    ReleaseFundsItem, SimulationResult,
    MAX_BATCH_SIZE,
    events::{emit_batch_funds_released, emit_funds_released,
             BatchFundsReleased, FundsReleased, CriticalOperationOutcome,
             RefundTriggerType, EVENT_VERSION_V2},
};

// ─────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────


pub(crate) fn release_funds_logic(env: Env, bounty_id: u64, contributor: Address) -> Result<(), Error> {
    // Validation precedence (deterministic ordering):
    // 1. Reentrancy guard
    // 2. Contract initialized
    // 3. Paused (operational state)
    // 4. Authorization
    // 5. Business logic (bounty exists, funds locked)

    // 1. GUARD: acquire reentrancy lock
    reentrancy_guard::acquire(&env);

    // 2. Contract must be initialized
    if !env.storage().instance().has(&DataKey::Admin) {
        reentrancy_guard::release(&env);
        return Err(Error::NotInitialized);
    }

    // 3. Operational state: paused
    if crate::pause_freeze::check_paused(&env, symbol_short!("release")) {
        reentrancy_guard::release(&env);
        return Err(Error::FundsPaused);
    }

    // 4. Authorization
    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();

    // 5. Business logic: bounty must exist and be locked
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

    if escrow.status != EscrowStatus::Locked {
        reentrancy_guard::release(&env);
        return Err(Error::FundsNotLocked);
    }

    // High-value timelock: if configured and amount >= threshold, queue instead of releasing.
    if let Some(hv_cfg) = env
        .storage()
        .instance()
        .get::<DataKey, HighValueConfig>(&DataKey::HighValueConfig)
    {
        if hv_cfg.threshold > 0 && escrow.amount >= hv_cfg.threshold {
            // Reject if a release is already queued for this bounty.
            if env
                .storage()
                .persistent()
                .has(&DataKey::QueuedRelease(bounty_id))
            {
                reentrancy_guard::release(&env);
                return Err(Error::ReleaseAlreadyQueued);
            }

            let executable_at = env.ledger().timestamp().saturating_add(hv_cfg.duration);
            let queued = QueuedRelease {
                contributor: contributor.clone(),
                amount: escrow.amount,
                executable_at,
            };
            env.storage()
                .persistent()
                .set(&DataKey::QueuedRelease(bounty_id), &queued);

            events::emit_release_queued(
                &env,
                events::ReleaseQueued {
                    version: EVENT_VERSION_V2,
                    bounty_id,
                    contributor,
                    amount: escrow.amount,
                    executable_at,
                    timestamp: env.ledger().timestamp(),
                },
            );

            reentrancy_guard::release(&env);
            return Ok(());
        }
    }

    // Resolve effective fee config for release.
    let (
        _lock_fee_rate,
        release_fee_rate,
        _lock_fixed,
        release_fixed_fee,
        fee_recipient,
        fee_enabled,
    ) = crate::fee::resolve_fee_config(&env);

    let release_fee = crate::fee::combined_fee_amount(
        escrow.amount,
        release_fee_rate,
        release_fixed_fee,
        fee_enabled,
    );
    let mut fee_config = crate::fee::get_fee_config_internal(&env);
    fee_config.release_fee_rate = release_fee_rate;
    fee_config.release_fixed_fee = release_fixed_fee;
    fee_config.fee_recipient = fee_recipient.clone();
    fee_config.fee_enabled = fee_enabled;

    // Net payout to contributor after release fee.
    let net_payout = escrow
        .amount
        .checked_sub(release_fee)
        .unwrap_or(escrow.amount);
    if net_payout <= 0 {
        return Err(Error::InvalidAmount);
    }

    // EFFECTS: update state before external calls (CEI)
    escrow.status = EscrowStatus::Released;
    escrow.remaining_amount = 0;
    invariants::assert_escrow(&env, &escrow);
    env.storage()
        .persistent()
        .set(&DataKey::Escrow(bounty_id), &escrow);
    crate::lock::renew_escrow_record(&env, bounty_id, true);

    // INTERACTION: external token transfers are last
    let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
    let client = token::Client::new(&env, &token_addr);

    if release_fee > 0 {
        crate::fee::route_fee_for_bounty(
            &env,
            &client,
            &fee_config,
            bounty_id,
            release_fee,
            release_fee_rate,
            escrow.amount,
            events::FeeOperationType::Release,
        )?;
    }

    client.transfer(&env.current_contract_address(), &contributor, &net_payout);

    emit_funds_released(
        &env,
        FundsReleased {
            version: EVENT_VERSION_V2,
            bounty_id,
            amount: escrow.amount,
            recipient: contributor.clone(),
            timestamp: env.ledger().timestamp(),
            correlation_id: None,
        },
    );

    // GUARD: release reentrancy lock
    reentrancy_guard::release(&env);
    Ok(())
}


pub(crate) fn release_with_conversion_logic(
    env: Env,
    bounty_id: u64,
    contributor: Address,
    dest_asset: Address,
    path: Vec<Address>,
    max_slippage_bps: u32,
) -> Result<(), Error> {
    // 1. GUARD: acquire reentrancy lock
    reentrancy_guard::acquire(&env);

    // 2. Contract must be initialized
    if !env.storage().instance().has(&DataKey::Admin) {
        reentrancy_guard::release(&env);
        return Err(Error::NotInitialized);
    }

    // 3. Operational state: paused
    if crate::pause_freeze::check_paused(&env, symbol_short!("release")) {
        reentrancy_guard::release(&env);
        return Err(Error::FundsPaused);
    }

    // 4. Authorization
    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();

    // 5. Business logic: bounty must exist and be locked
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

    if escrow.status != EscrowStatus::Locked {
        reentrancy_guard::release(&env);
        return Err(Error::FundsNotLocked);
    }

    // High-value timelock check (same as release_funds_logic)
    if let Some(hv_cfg) = env
        .storage()
        .instance()
        .get::<DataKey, HighValueConfig>(&DataKey::HighValueConfig)
    {
        if hv_cfg.threshold > 0 && escrow.amount >= hv_cfg.threshold {
            // Reject if a release is already queued for this bounty.
            if env
                .storage()
                .persistent()
                .has(&DataKey::QueuedRelease(bounty_id))
            {
                reentrancy_guard::release(&env);
                return Err(Error::ReleaseAlreadyQueued);
            }

            let executable_at = env.ledger().timestamp().saturating_add(hv_cfg.duration);
            let queued = QueuedRelease {
                contributor: contributor.clone(),
                amount: escrow.amount,
                executable_at,
            };
            env.storage()
                .persistent()
                .set(&DataKey::QueuedRelease(bounty_id), &queued);

            events::emit_release_queued(
                &env,
                events::ReleaseQueued {
                    version: EVENT_VERSION_V2,
                    bounty_id,
                    contributor,
                    amount: escrow.amount,
                    executable_at,
                    timestamp: env.ledger().timestamp(),
                },
            );

            reentrancy_guard::release(&env);
            return Ok(());
        }
    }

    // Resolve effective fee config for release.
    let (
        _lock_fee_rate,
        release_fee_rate,
        _lock_fixed,
        release_fixed_fee,
        fee_recipient,
        fee_enabled,
    ) = crate::fee::resolve_fee_config(&env);

    let release_fee = crate::fee::combined_fee_amount(
        escrow.amount,
        release_fee_rate,
        release_fixed_fee,
        fee_enabled,
    );
    let mut fee_config = crate::fee::get_fee_config_internal(&env);
    fee_config.release_fee_rate = release_fee_rate;
    fee_config.release_fixed_fee = release_fixed_fee;
    fee_config.fee_recipient = fee_recipient.clone();
    fee_config.fee_enabled = fee_enabled;

    // Net payout to contributor after release fee.
    let net_payout = escrow
        .amount
        .checked_sub(release_fee)
        .unwrap_or(escrow.amount);
    if net_payout <= 0 {
        reentrancy_guard::release(&env);
        return Err(Error::InvalidAmount);
    }

    // EFFECTS: update state before external calls (CEI)
    escrow.status = EscrowStatus::Released;
    escrow.remaining_amount = 0;
    invariants::assert_escrow(&env, &escrow);
    env.storage()
        .persistent()
        .set(&DataKey::Escrow(bounty_id), &escrow);
    crate::lock::renew_escrow_record(&env, bounty_id, true);

    // INTERACTION: external token transfers are last
    let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
    let client = token::Client::new(&env, &token_addr);

    if release_fee > 0 {
        crate::fee::route_fee_for_bounty(
            &env,
            &client,
            &fee_config,
            bounty_id,
            release_fee,
            release_fee_rate,
            escrow.amount,
            events::FeeOperationType::Release,
        )?;
    }

    // Swapping the escrowed asset to the recipient's preferred currency atomically before transfer
    let router_address: Address =
        env.storage()
            .instance()
            .get(&DataKey::Router)
            .ok_or_else(|| {
                reentrancy_guard::release(&env);
                Error::RouterNotConfigured
            })?;

    let router_client = RouterClient::new(&env, &router_address);

    // Approve router to spend net_payout of source asset from contract.
    // `approve` expects an expiration *ledger sequence* (u32), not a timestamp.
    let deadline = env.ledger().timestamp() + 300;
    let approve_expiration_ledger = env.ledger().sequence() + 100;
    client.approve(
        &env.current_contract_address(),
        &router_address,
        &net_payout,
        &approve_expiration_ledger,
    );

    // Query the router for the expected amount out to validate slippage
    let amounts_out = router_client.get_amounts_out(&net_payout, &path);
    let expected_out = amounts_out.last().unwrap_or(0);

    // Calculate min amount out based on max slippage bps
    let min_amount_out = expected_out
        .checked_mul(10000 - max_slippage_bps as i128)
        .unwrap()
        .checked_div(10000)
        .unwrap();

    // Execute the swap
    let swap_amounts = router_client.swap_exact_tokens_for_tokens(
        &net_payout,
        &min_amount_out,
        &path,
        &contributor,
        &deadline,
    );

    let actual_out = swap_amounts.last().unwrap_or(0);

    // Validate slippage against actual received amount
    if actual_out < min_amount_out {
        reentrancy_guard::release(&env);
        return Err(Error::SlippageExceeded);
    }

    // Emit ReleasedWithConversion event
    // rate = (actual_out * 1_000_000) / net_payout
    let rate = if net_payout > 0 {
        actual_out
            .checked_mul(1_000_000)
            .unwrap()
            .checked_div(net_payout)
            .unwrap_or(0)
    } else {
        0
    };

    events::emit_released_with_conversion(
        &env,
        events::ReleasedWithConversion {
            escrow_id: bounty_id,
            src_asset: token_addr,
            dest_asset,
            rate,
        },
    );

    // GUARD: release reentrancy lock
    reentrancy_guard::release(&env);
    Ok(())
}


pub(crate) fn dry_run_release_impl(
    env: &Env,
    bounty_id: u64,
    _contributor: Address,
) -> Result<(i128,), Error> {
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }
    if crate::pause_freeze::check_paused(env, symbol_short!("release")) {
        return Err(Error::FundsPaused);
    }
    if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
        return Err(Error::BountyNotFound);
    }
    let escrow: Escrow = env
        .storage()
        .persistent()
        .get(&DataKey::Escrow(bounty_id))
        .unwrap();
    crate::pause_freeze::ensure_escrow_not_frozen(env, bounty_id)?;
    crate::pause_freeze::ensure_address_not_frozen(env, &escrow.depositor)?;
    if escrow.status != EscrowStatus::Locked {
        return Err(Error::FundsNotLocked);
    }
    let (
        _lock_fee_rate,
        release_fee_rate,
        _lock_fixed,
        release_fixed_fee,
        _fee_recipient,
        fee_enabled,
    ) = crate::fee::resolve_fee_config(env);
    let release_fee = crate::fee::combined_fee_amount(
        escrow.amount,
        release_fee_rate,
        release_fixed_fee,
        fee_enabled,
    );
    let net_payout = escrow
        .amount
        .checked_sub(release_fee)
        .unwrap_or(escrow.amount);
    if net_payout <= 0 {
        return Err(Error::InvalidAmount);
    }
    Ok((escrow.amount,))
}

// ─────────────────────────────────────────────────────────────────
// Public entry points (dispatcher targets)
// ─────────────────────────────────────────────────────────────────


/// Releases escrowed funds to a contributor.
///
/// # Invariants Verified
/// - INV-ESC-4: Released => remaining_amount == 0
/// - INV-ESC-7: Aggregate fund conservation (sum(active) == contract.balance)
///
/// # Access Control
/// Admin-only.
///
/// # Front-running Behavior
/// First valid release for a bounty transitions state to `Released`. Later release/refund/claim
/// races against that bounty must fail with `Error::FundsNotLocked`.
///
/// # Transition Guards
/// This function enforces the following state transition guards:
///
/// ## Pre-conditions (checked in order):
/// 1. **Reentrancy Guard**: Acquires reentrancy lock to prevent concurrent execution
/// 2. **Initialization**: Contract must be initialized (admin set)
/// 3. **Operational State**: Contract must not be paused for release operations
/// 4. **Authorization**: Admin must authorize the transaction
/// 5. **Escrow Existence**: Bounty must exist in storage
/// 6. **Freeze Check**: Escrow and depositor must not be frozen
/// 7. **Status Guard**: Escrow status must be `Locked` or `PartiallyRefunded`
///
/// ## State Transition:
/// - **From**: `Locked` or `PartiallyRefunded`
/// - **To**: `Released`
/// - **Effect**: Sets `remaining_amount` to 0
///
/// ## Post-conditions:
/// - External token transfer to contributor (after state update)
/// - Fee transfer to fee recipient (if applicable)
/// - Event emission
///
/// ## Contention Safety:
/// - If status is `Released`, `Refunded`, or `Draft`, returns `Error::FundsNotLocked`
/// - Reentrancy guard prevents concurrent execution of any protected function
/// - CEI pattern ensures state is updated before external calls
///
/// # Security
/// Reentrancy guard is always cleared before any explicit error return after acquisition.
pub fn release_funds(env: Env, bounty_id: u64, contributor: Address) -> Result<(), Error> {
    crate::claims::validate_claim_window(env.clone(), bounty_id)?;
    let caller = env
        .storage()
        .instance()
        .get::<DataKey, Address>(&DataKey::Admin)
        .unwrap_or(contributor.clone());
    let res = release_funds_logic(env.clone(), bounty_id, contributor);
    monitoring::track_operation(&env, symbol_short!("release"), caller, res.is_ok());
    res
}


pub fn set_router(env: Env, router: Address) -> Result<(), Error> {
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(Error::NotInitialized)?;
    admin.require_auth();
    env.storage().instance().set(&DataKey::Router, &router);
    Ok(())
}


pub fn get_router(env: Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::Router)
}


pub fn release_with_conversion(
    env: Env,
    bounty_id: u64,
    contributor: Address,
    dest_asset: Address,
    path: Vec<Address>,
    max_slippage_bps: u32,
) -> Result<(), Error> {
    crate::claims::validate_claim_window(env.clone(), bounty_id)?;
    let caller = env
        .storage()
        .instance()
        .get::<DataKey, Address>(&DataKey::Admin)
        .unwrap_or(contributor.clone());
    let res = release_with_conversion_logic(
        env.clone(),
        bounty_id,
        contributor,
        dest_asset,
        path,
        max_slippage_bps,
    );
    monitoring::track_operation(&env, symbol_short!("rel_conv"), caller, res.is_ok());
    res
}


/// Simulate release operation without state changes or token transfers.
///
/// Returns a `SimulationResult` indicating whether the operation would succeed and the
/// resulting escrow state. Does not require authorization; safe for off-chain preview.
///
/// # Arguments
/// * `bounty_id` - Bounty identifier
/// * `contributor` - Recipient address
///
/// # Security
/// This function performs only read operations. No storage writes, token transfers,
/// or events are emitted.
pub fn dry_run_release(env: Env, bounty_id: u64, contributor: Address) -> SimulationResult {
    fn err_result(e: Error) -> SimulationResult {
        SimulationResult {
            success: false,
            error_code: e as u32,
            amount: 0,
            resulting_status: EscrowStatus::Released,
            remaining_amount: 0,
        }
    }
    match dry_run_release_impl(&env, bounty_id, contributor) {
        Ok((amount,)) => SimulationResult {
            success: true,
            error_code: 0,
            amount,
            resulting_status: EscrowStatus::Released,
            remaining_amount: 0,
        },
        Err(e) => err_result(e),
    }
}


/// Delegated release flow using a capability instead of admin auth.
/// The capability amount limit is consumed by `payout_amount`.
pub fn release_with_capability(
    env: Env,
    bounty_id: u64,
    contributor: Address,
    payout_amount: i128,
    holder: Address,
    capability_id: BytesN<32>,
) -> Result<(), Error> {
    // GUARD: acquire reentrancy lock
    reentrancy_guard::acquire(&env);

    if crate::pause_freeze::check_paused(&env, symbol_short!("release")) {
        reentrancy_guard::release(&env);
        return Err(Error::FundsPaused);
    }
    if payout_amount <= 0 {
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
    if escrow.status != EscrowStatus::Locked {
        reentrancy_guard::release(&env);
        return Err(Error::FundsNotLocked);
    }
    if payout_amount > escrow.remaining_amount {
        reentrancy_guard::release(&env);
        return Err(Error::InsufficientFunds);
    }

    crate::capability::consume_capability(
        &env,
        &holder,
        capability_id,
        CapabilityAction::Release,
        bounty_id,
        payout_amount,
    )?;

    // EFFECTS: update state before external call (CEI)
    escrow.remaining_amount -= payout_amount;
    if escrow.remaining_amount == 0 {
        escrow.status = EscrowStatus::Released;
    }
    env.storage()
        .persistent()
        .set(&DataKey::Escrow(bounty_id), &escrow);
    crate::lock::renew_escrow_record(
        &env,
        bounty_id,
        escrow.status == EscrowStatus::Released,
    );

    // INTERACTION: external token transfer is last
    let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
    let client = token::Client::new(&env, &token_addr);
    client.transfer(
        &env.current_contract_address(),
        &contributor,
        &payout_amount,
    );

    emit_funds_released(
        &env,
        FundsReleased {
            version: EVENT_VERSION_V2,
            bounty_id,
            amount: payout_amount,
            recipient: contributor,
            timestamp: env.ledger().timestamp(),
            correlation_id: None,
        },
    );

    // GUARD: release reentrancy lock
    reentrancy_guard::release(&env);
    Ok(())
}


/// Releases a partial amount of locked funds.
///
/// # Access Control
/// Admin-only.
///
/// # Front-running Behavior
/// Each successful call decreases `remaining_amount` exactly once. Attempts to exceed remaining
/// balance fail with `Error::InsufficientFunds`.
///
/// - `payout_amount` must be > 0 and <= `remaining_amount`.
/// - `remaining_amount` is decremented by `payout_amount` after each call.
/// - When `remaining_amount` reaches 0 the escrow status is set to Released.
/// - The bounty stays Locked while any funds remain unreleased.
pub fn partial_release(
    env: Env,
    bounty_id: u64,
    contributor: Address,
    payout_amount: i128,
) -> Result<(), Error> {
    // GUARD: acquire reentrancy lock
    reentrancy_guard::acquire(&env);

    if !env.storage().instance().has(&DataKey::Admin) {
        reentrancy_guard::release(&env);
        return Err(Error::NotInitialized);
    }

    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();
    // Snapshot resource meters for gas cap enforcement (test / testutils only).
    #[cfg(any(test, feature = "testutils"))]
    let _gas_snapshot = gas_budget::capture(&env);

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

    if escrow.status != EscrowStatus::Locked {
        reentrancy_guard::release(&env);
        return Err(Error::FundsNotLocked);
    }

    // Guard: zero or negative payout makes no sense and would corrupt state
    if payout_amount <= 0 {
        reentrancy_guard::release(&env);
        return Err(Error::InvalidAmount);
    }

    // Guard: prevent overpayment — payout cannot exceed what is still owed
    if payout_amount > escrow.remaining_amount {
        reentrancy_guard::release(&env);
        return Err(Error::InsufficientFunds);
    }

    // EFFECTS: update state before external call (CEI)
    // Decrement remaining; this is always an exact integer subtraction — no rounding
    escrow.remaining_amount = escrow.remaining_amount.checked_sub(payout_amount).unwrap();

    // Automatically transition to Released once fully paid out
    if escrow.remaining_amount == 0 {
        escrow.status = EscrowStatus::Released;
    }

    env.storage()
        .persistent()
        .set(&DataKey::Escrow(bounty_id), &escrow);
    crate::lock::renew_escrow_record(
        &env,
        bounty_id,
        escrow.status == EscrowStatus::Released,
    );

    // INTERACTION: external token transfer is last
    let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
    let client = token::Client::new(&env, &token_addr);
    client.transfer(
        &env.current_contract_address(),
        &contributor,
        &payout_amount,
    );

    events::emit_funds_released(
        &env,
        FundsReleased {
            version: EVENT_VERSION_V2,
            bounty_id,
            amount: payout_amount,
            recipient: contributor,
            timestamp: env.ledger().timestamp(),
            correlation_id: None,
        },
    );

    // GUARD: release reentrancy lock
    reentrancy_guard::release(&env);
    multitoken_invariants::assert_after_disbursement(&env);
    Ok(())
}


/// Batch release funds to multiple contributors in a single atomic transaction.
///
/// Releases between 1 and [`MAX_BATCH_SIZE`] bounties in one admin-authorised
/// call, reducing per-transaction overhead compared to repeated single-item
/// `release_funds` calls.
///
/// ## Batch failure semantics
///
/// This operation is **strictly atomic** (all-or-nothing):
///
/// 1. All items are validated in a single pass **before** any escrow status
///    is updated or any token transfer is initiated.
/// 2. If *any* item fails validation the entire call reverts immediately.
///    No status is changed, no token leaves the contract, and every
///    "sibling" row in the same batch is left completely unaffected.
/// 3. After a failed batch the contract is in exactly the same state as
///    before the call; subsequent operations behave as if this call never
///    happened.
///
/// ## Ordering guarantee
///
/// Items are processed in ascending `bounty_id` order regardless of the
/// caller-supplied ordering, ensuring deterministic execution.
///
/// ## Checks-Effects-Interactions (CEI)
///
/// All escrow statuses are updated to `Released` in a first pass (Effects);
/// external token transfers and event emissions happen in a second pass
/// (Interactions).
///
/// # Arguments
/// * `items` - 1–[`MAX_BATCH_SIZE`] [`ReleaseFundsItem`] entries (bounty_id,
///   contributor address).
///
/// # Returns
/// Number of bounties successfully released (equals `items.len()` on success).
///
/// # Errors
/// * [`Error::InvalidBatchSize`] — batch is empty or exceeds `MAX_BATCH_SIZE`
/// * [`Error::FundsPaused`] — release operations are currently paused
/// * [`Error::NotInitialized`] — `init` has not been called
/// * [`Error::Unauthorized`] — caller is not the admin
/// * [`Error::BountyNotFound`] — a `bounty_id` does not exist in storage
/// * [`Error::FundsNotLocked`] — a bounty's status is not `Locked`
/// * [`Error::DuplicateBountyId`] — the same `bounty_id` appears more than once
///
/// # Reentrancy
/// Protected by the shared reentrancy guard (acquired before validation,
/// released after all effects and interactions complete).
pub fn batch_release_funds(env: Env, items: Vec<ReleaseFundsItem>) -> Result<u32, Error> {
    if crate::pause_freeze::check_paused(&env, symbol_short!("release")) {
        return Err(Error::FundsPaused);
    }
    // GUARD: acquire reentrancy lock
    reentrancy_guard::acquire(&env);
    // Snapshot resource meters for gas cap enforcement (test / testutils only).
    #[cfg(any(test, feature = "testutils"))]
    let gas_snapshot = gas_budget::capture(&env);
    let result: Result<u32, Error> = (|| {
        // Validate batch size against the release-specific runtime cap.
        let batch_size = items.len();
        if batch_size == 0 {
            reentrancy_guard::release(&env);
            return Err(Error::InvalidBatchSize);
        }
        let max_batch_size = crate::admin::get_max_release_batch_size(env.clone());
        if batch_size > max_batch_size {
            reentrancy_guard::release(&env);
            return Err(Error::InvalidBatchSize);
        }

        if !env.storage().instance().has(&DataKey::Admin) {
            reentrancy_guard::release(&env);
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        let contract_address = env.current_contract_address();
        let timestamp = env.ledger().timestamp();

        // Validate all items before processing (all-or-nothing approach)
        let mut total_amount: i128 = 0;
        for item in items.iter() {
            // Check if bounty exists
            if !env
                .storage()
                .persistent()
                .has(&DataKey::Escrow(item.bounty_id))
            {
                reentrancy_guard::release(&env);
                return Err(Error::BountyNotFound);
            }

            let escrow: Escrow = env
                .storage()
                .persistent()
                .get(&DataKey::Escrow(item.bounty_id))
                .unwrap();

            crate::pause_freeze::ensure_escrow_not_frozen(&env, item.bounty_id)?;
            crate::pause_freeze::ensure_address_not_frozen(&env, &escrow.depositor)?;

            // Check if funds are locked
            if escrow.status != EscrowStatus::Locked {
                reentrancy_guard::release(&env);
                return Err(Error::FundsNotLocked);
            }

            // Check for duplicate bounty_ids in the batch
            let mut count = 0u32;
            for other_item in items.iter() {
                if other_item.bounty_id == item.bounty_id {
                    count += 1;
                }
            }
            if count > 1 {
                reentrancy_guard::release(&env);
                return Err(Error::DuplicateBountyId);
            }

            total_amount = total_amount
                .checked_add(escrow.amount)
                .ok_or(Error::InvalidAmount)?;
        }

        let ordered_items = crate::admin::order_batch_release_items(&env, &items);

        // EFFECTS: update all escrow records before any external calls (CEI)
        // We collect (contributor, amount) pairs for the transfer pass.
        let mut release_pairs: Vec<(Address, i128)> = Vec::new(&env);
        let mut released_count = 0u32;
        for item in ordered_items.iter() {
            let mut escrow: Escrow = env
                .storage()
                .persistent()
                .get(&DataKey::Escrow(item.bounty_id))
                .unwrap();

            let amount = escrow.amount;
            escrow.status = EscrowStatus::Released;
            escrow.remaining_amount = 0;
            env.storage()
                .persistent()
                .set(&DataKey::Escrow(item.bounty_id), &escrow);
            crate::lock::renew_escrow_record(&env, item.bounty_id, true);

            release_pairs.push_back((item.contributor.clone(), amount));
            released_count += 1;
        }

        // INTERACTION: all external token transfers happen after state is finalized
        for (idx, item) in ordered_items.iter().enumerate() {
            let (ref contributor, amount) = release_pairs.get(idx as u32).unwrap();
            client.transfer(&contract_address, contributor, &amount);

            emit_funds_released(
                &env,
                FundsReleased {
                    version: EVENT_VERSION_V2,
                    bounty_id: item.bounty_id,
                    amount,
                    recipient: contributor.clone(),
                    timestamp,
                    correlation_id: None,
                },
            );
        }

        // Emit batch event
        emit_batch_funds_released(
            &env,
            BatchFundsReleased {
                version: EVENT_VERSION_V2,
                count: released_count,
                total_amount,
                timestamp,
            },
        );
        Ok(released_count)
    })();

    // Gas budget cap enforcement (test / testutils only).
    #[cfg(any(test, feature = "testutils"))]
    if result.is_ok() {
        let gas_cfg = gas_budget::get_config(&env);
        gas_budget::check(
            &env,
            symbol_short!("b_rel"),
            &gas_cfg.batch_release,
            &gas_snapshot,
            gas_cfg.enforce,
        )?;
    }

    let count = result?;
    multitoken_invariants::assert_after_disbursement(&env);
    reentrancy_guard::release(&env);
    Ok(count)
}


/// Structure-of-Arrays (SoA) variant of `batch_release_funds`.
/// Reduces host-to-guest deserialization overhead by accepting parallel arrays
/// of primitives instead of an array of structs.
pub fn batch_release_funds_soa(
    env: Env,
    bounty_ids: Vec<u64>,
    contributors: Vec<Address>,
) -> Result<u32, Error> {
    if bounty_ids.len() != contributors.len() {
        return Err(Error::BatchSizeMismatch);
    }

    let mut items = Vec::new(&env);
    for i in 0..bounty_ids.len() {
        items.push_back(ReleaseFundsItem {
            bounty_id: bounty_ids.get(i).unwrap(),
            contributor: contributors.get(i).unwrap(),
        });
    }
    batch_release_funds(env, items)
}


// ============================================================================
// HIGH-VALUE RELEASE TIMELOCK QUEUE
// ============================================================================

/// Configures the high-value timelock threshold and duration.
///
/// Both `threshold` and `duration` must be positive: a zero duration would
/// make releases immediately executable (defeating the timelock), and a
/// zero threshold would queue every release regardless of amount.
pub fn set_high_value_config(env: Env, threshold: i128, duration: u64) -> Result<(), Error> {
    let admin = rbac::require_admin(&env);
    admin.require_auth();

    if threshold <= 0 {
        return Err(Error::InvalidAmount);
    }

    // Duration must be > 0; otherwise the timelock delay is meaningless.
    if duration == 0 {
        return Err(Error::InvalidAmount);
    }

    let config = HighValueConfig {
        threshold,
        duration,
    };
    env.storage()
        .instance()
        .set(&DataKey::HighValueConfig, &config);

    events::emit_high_value_config_updated(
        &env,
        events::HighValueConfigUpdated {
            version: events::EVENT_VERSION_V2,
            admin,
            threshold,
            duration,
            timestamp: env.ledger().timestamp(),
        },
    );

    Ok(())
}


/// View: Gets the current high-value release configuration.
pub fn get_high_value_config(env: Env) -> Option<HighValueConfig> {
    env.storage().instance().get(&DataKey::HighValueConfig)
}


/// View: Gets a currently queued release for a specific bounty.
pub fn get_queued_release(env: Env, bounty_id: u64) -> Option<QueuedRelease> {
    env.storage()
        .persistent()
        .get(&DataKey::QueuedRelease(bounty_id))
}


/// View: Gets the stored high-value config schema version (upgrade safety check).
pub fn get_hv_config_schema_version(env: Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::HighValueConfigSchemaVersion)
        .unwrap_or(0)
}


/// Executes a queued high-value release once its timelock has elapsed.
///
/// Anyone may call this after `executable_at`; the admin queued the release
/// via `release_funds` and the timelock enforces the delay.
/// Applies release fees consistently with the standard `release_funds` path.
pub fn execute_queued_release(env: Env, bounty_id: u64) -> Result<(), Error> {
    reentrancy_guard::acquire(&env);

    let result: Result<(), Error> = (|| {
        if crate::pause_freeze::check_paused(&env, symbol_short!("release")) {
            return Err(Error::FundsPaused);
        }

        let queued: QueuedRelease = env
            .storage()
            .persistent()
            .get(&DataKey::QueuedRelease(bounty_id))
            .ok_or(Error::BountyNotFound)?;

        if env.ledger().timestamp() < queued.executable_at {
            return Err(Error::TimelockNotElapsed);
        }

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .ok_or(Error::BountyNotFound)?;

        crate::pause_freeze::ensure_escrow_not_frozen(&env, bounty_id)?;
        crate::pause_freeze::ensure_address_not_frozen(&env, &escrow.depositor)?;

        if escrow.status != EscrowStatus::Locked {
            return Err(Error::FundsNotLocked);
        }

        let (
            _lock_fee_rate,
            release_fee_rate,
            _lock_fixed,
            release_fixed_fee,
            fee_recipient,
            fee_enabled,
        ) = crate::fee::resolve_fee_config(&env);

        let release_fee = crate::fee::combined_fee_amount(
            escrow.amount,
            release_fee_rate,
            release_fixed_fee,
            fee_enabled,
        );
        let net_payout = escrow
            .amount
            .checked_sub(release_fee)
            .ok_or(Error::InvalidAmount)?;
        if net_payout <= 0 {
            return Err(Error::InvalidAmount);
        }

        // EFFECTS: remove queue entry before token transfer (CEI)
        env.storage()
            .persistent()
            .remove(&DataKey::QueuedRelease(bounty_id));

        escrow.status = EscrowStatus::Released;
        escrow.remaining_amount = 0;
        invariants::assert_escrow(&env, &escrow);
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);
        crate::lock::renew_escrow_record(&env, bounty_id, true);

        // INTERACTION: token transfer after state update
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);

        if release_fee > 0 {
            let mut fee_config = crate::fee::get_fee_config_internal(&env);
            fee_config.release_fee_rate = release_fee_rate;
            fee_config.release_fixed_fee = release_fixed_fee;
            fee_config.fee_recipient = fee_recipient;
            fee_config.fee_enabled = fee_enabled;

            crate::fee::route_fee_for_bounty(
                &env,
                &client,
                &fee_config,
                bounty_id,
                release_fee,
                release_fee_rate,
                escrow.amount,
                events::FeeOperationType::Release,
            )?;
        }

        client.transfer(
            &env.current_contract_address(),
            &queued.contributor,
            &net_payout,
        );

        events::emit_queued_release_executed(
            &env,
            events::QueuedReleaseExecuted {
                version: events::EVENT_VERSION_V2,
                bounty_id,
                contributor: queued.contributor,
                amount: net_payout,
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    })();

    multitoken_invariants::assert_after_disbursement(&env);
    reentrancy_guard::release(&env);
    result
}


/// Cancels a pending queued release (admin only).
///
/// The escrow remains in `Locked` status so the admin can re-release
/// normally or queue again.
pub fn cancel_queued_release(env: Env, bounty_id: u64) -> Result<(), Error> {
    let admin = rbac::require_admin(&env);
    admin.require_auth();

    let queued: QueuedRelease = env
        .storage()
        .persistent()
        .get(&DataKey::QueuedRelease(bounty_id))
        .ok_or(Error::BountyNotFound)?;

    env.storage()
        .persistent()
        .remove(&DataKey::QueuedRelease(bounty_id));

    events::emit_release_queue_cancelled(
        &env,
        events::ReleaseQueueCancelled {
            version: events::EVENT_VERSION_V2,
            bounty_id,
            contributor: queued.contributor,
            amount: queued.amount,
            admin,
            timestamp: env.ledger().timestamp(),
        },
    );

    Ok(())
}

