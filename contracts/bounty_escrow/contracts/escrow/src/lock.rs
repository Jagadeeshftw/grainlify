//! Fund locking: single lock, anonymous lock, dry-run, archival, batching, and multisig approvals.



use soroban_sdk::{symbol_short, token, Address, BytesN, Env, Vec};
use crate::{
    anti_abuse, events, gas_budget, multitoken_invariants, reentrancy_guard,
    Escrow, EscrowMetadata, EscrowStatus, AnonymousEscrow, DataKey, Error,
    LockFundsItem, MultisigConfig, RefundMode, RefundRecord, SimulationResult,
    ESCROW_LIVE_TTL, ESCROW_ARCHIVAL_TTL, CLAIM_LIVE_TTL, CLAIM_ARCHIVAL_TTL,
    COMMITMENT_LIVE_TTL, COMMITMENT_ARCHIVAL_TTL, INDEX_LIVE_TTL, INDEX_ARCHIVAL_TTL,
    ARCHIVAL_MARKER_TTL, TTL_RENEWAL_DIVISOR, MAX_BATCH_SIZE,
    events::{emit_batch_funds_locked, emit_funds_locked, emit_funds_locked_anon,
             BatchFundsLocked, FundsLocked, FundsLockedAnon, EscrowPublished,
             CriticalOperationOutcome, EVENT_VERSION_V2},
    ReleaseApproval,
    FeeConfig,
    Capability,
};

// ─────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────


pub(crate) fn lock_funds_logic(
    env: Env,
    depositor: Address,
    bounty_id: u64,
    amount: i128,
    deadline: u64,
) -> Result<(), Error> {
    // Validation precedence (deterministic ordering):
    // 1. Reentrancy guard
    // 2. Contract initialized
    // 3. Paused / deprecated (operational state)
    // 4. Participant filter + rate limiting
    // 5. Authorization
    // 6. Input validation (amount policy)
    // 7. Business logic (bounty uniqueness)

    // 1. GUARD: acquire reentrancy lock
    reentrancy_guard::acquire(&env);
    // Snapshot resource meters for gas cap enforcement (test / testutils only).
    #[cfg(any(test, feature = "testutils"))]
    let gas_snapshot = gas_budget::capture(&env);

    // 2. Contract must be initialized before any other check
    if !env.storage().instance().has(&DataKey::Admin) {
        reentrancy_guard::release(&env);
        return Err(Error::NotInitialized);
    }
    soroban_sdk::log!(&env, "admin ok");

    // 3. Operational state: paused / deprecated
    if crate::pause_freeze::check_paused(&env, symbol_short!("lock")) {
        reentrancy_guard::release(&env);
        return Err(Error::FundsPaused);
    }
    if crate::participant_filter::get_deprecation_state(&env).deprecated {
        reentrancy_guard::release(&env);
        return Err(Error::ContractDeprecated);
    }
    soroban_sdk::log!(&env, "check paused ok");

    // 4. Participant filtering and rate limiting
    crate::participant_filter::check_participant_filter(&env, depositor.clone())?;
    soroban_sdk::log!(&env, "start lock_funds");
    anti_abuse::check_rate_limit(&env, depositor.clone());
    soroban_sdk::log!(&env, "rate limit ok");

    let _start = env.ledger().timestamp();
    let _caller = depositor.clone();

    // 5. Authorization
    depositor.require_auth();
    soroban_sdk::log!(&env, "auth ok");

    // 6. Input validation: amount policy
    // Enforce min/max amount policy if one has been configured (Issue #62).
    if let Some((min_amount, max_amount)) = env
        .storage()
        .instance()
        .get::<DataKey, (i128, i128)>(&DataKey::AmountPolicy)
    {
        if amount < min_amount {
            reentrancy_guard::release(&env);
            return Err(Error::AmountBelowMinimum);
        }
        if amount > max_amount {
            reentrancy_guard::release(&env);
            return Err(Error::AmountAboveMaximum);
        }
    }
    soroban_sdk::log!(&env, "amount policy ok");

    // 7. Business logic: bounty must not already exist
    if env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
        reentrancy_guard::release(&env);
        return Err(Error::BountyExists);
    }
    soroban_sdk::log!(&env, "bounty exists ok");

    let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
    let client = token::Client::new(&env, &token_addr);
    soroban_sdk::log!(&env, "token client ok");

    // Resolve effective fee config (per-token takes precedence over global).
    let (
        lock_fee_rate,
        _release_fee_rate,
        lock_fixed_fee,
        _release_fixed,
        fee_recipient,
        fee_enabled,
    ) = crate::fee::resolve_fee_config(&env);
    let fee_config = FeeConfig {
        lock_fee_rate,
        release_fee_rate: 0,
        lock_fixed_fee,
        release_fixed_fee: 0,
        fee_recipient: fee_recipient.clone(),
        fee_enabled,
        treasury_destinations: Vec::new(&env),
        distribution_enabled: false,
    };

    // Deduct lock fee from the escrowed principal (percentage + fixed, capped at deposit).
    let fee_amount =
        crate::fee::combined_fee_amount(amount, lock_fee_rate, lock_fixed_fee, fee_enabled);

    // Net amount stored in escrow after fee.
    // Fee must never exceed the deposit; guard against misconfiguration.
    let net_amount = amount.checked_sub(fee_amount).unwrap_or(amount);
    if net_amount <= 0 {
        reentrancy_guard::release(&env);
        return Err(Error::InvalidAmount);
    }

    let escrow = Escrow {
        depositor: depositor.clone(),
        amount: net_amount,
        status: EscrowStatus::Locked,
        deadline,
        refund_history: vec![&env],
        remaining_amount: net_amount,
        archived: false,
        archived_at: None,
    };
    invariants::assert_escrow(&env, &escrow);

    // EFFECTS: Update state and indexes before interactions
    env.storage()
        .persistent()
        .set(&DataKey::Escrow(bounty_id), &escrow);
    renew_escrow_record(&env, bounty_id, false);

    // Update indexes
    let mut index: Vec<u64> = env
        .storage()
        .persistent()
        .get(&DataKey::EscrowIndex)
        .unwrap_or(Vec::new(&env));
    index.push_back(bounty_id);
    env.storage()
        .persistent()
        .set(&DataKey::EscrowIndex, &index);
    renew_escrow_index(&env, false);

    let mut depositor_index: Vec<u64> = env
        .storage()
        .persistent()
        .get(&DataKey::DepositorIndex(depositor.clone()))
        .unwrap_or(Vec::new(&env));
    depositor_index.push_back(bounty_id);
    env.storage().persistent().set(
        &DataKey::DepositorIndex(depositor.clone()),
        &depositor_index,
    );
    renew_depositor_index(&env, &depositor, false);

    // INTERACTION: all external token transfers happen after state is finalized (CEI)
    // Transfer full gross amount from depositor to contract.
    client.transfer(&depositor, &env.current_contract_address(), &amount);
    soroban_sdk::log!(&env, "transfer ok");

    // Transfer fee to recipient immediately (separate transfer so it is
    // visible as a distinct on-chain operation).
    if fee_amount > 0 {
        crate::fee::route_fee_for_bounty(
            &env,
            &client,
            &fee_config,
            bounty_id,
            fee_amount,
            lock_fee_rate,
            amount,
            events::FeeOperationType::Lock,
        )?;
    }
    soroban_sdk::log!(&env, "fee ok");

    let mut depositor_index: Vec<u64> = env
        .storage()
        .persistent()
        .get(&DataKey::DepositorIndex(depositor.clone()))
        .unwrap_or(Vec::new(&env));
    depositor_index.push_back(bounty_id);
    env.storage().persistent().set(
        &DataKey::DepositorIndex(depositor.clone()),
        &depositor_index,
    );
    renew_depositor_index(&env, &depositor, false);

    // Emit value allows for off-chain indexing
    emit_funds_locked(
        &env,
        FundsLocked {
            version: EVENT_VERSION_V2,
            bounty_id,
            amount,
            depositor: depositor.clone(),
            deadline,
            correlation_id: None,
        },
    );

    // INV-2: Verify aggregate balance matches token balance after lock
    multitoken_invariants::assert_after_lock(&env);

    // Gas budget cap enforcement (test / testutils only; see `gas_budget` module docs).
    #[cfg(any(test, feature = "testutils"))]
    {
        let gas_cfg = gas_budget::get_config(&env);
        gas_budget::check(
            &env,
            symbol_short!("lock"),
            &gas_cfg.lock,
            &gas_snapshot,
            gas_cfg.enforce,
        )?;
    }

    // GUARD: release reentrancy lock
    reentrancy_guard::release(&env);
    Ok(())
}


pub(crate) fn dry_run_lock_impl(
    env: &Env,
    depositor: Address,
    bounty_id: u64,
    amount: i128,
    _deadline: u64,
) -> Result<(i128,), Error> {
    // 1. Contract must be initialized
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }
    // 2. Operational state: paused / deprecated
    if crate::pause_freeze::check_paused(env, symbol_short!("lock")) {
        return Err(Error::FundsPaused);
    }
    if crate::participant_filter::get_deprecation_state(env).deprecated {
        return Err(Error::ContractDeprecated);
    }
    // 3. Participant filtering (read-only)
    crate::participant_filter::check_participant_filter(env, depositor.clone())?;
    // 4. Amount policy
    if let Some((min_amount, max_amount)) = env
        .storage()
        .instance()
        .get::<DataKey, (i128, i128)>(&DataKey::AmountPolicy)
    {
        if amount < min_amount {
            return Err(Error::AmountBelowMinimum);
        }
        if amount > max_amount {
            return Err(Error::AmountAboveMaximum);
        }
    }
    // 5. Bounty must not already exist
    if env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
        return Err(Error::BountyExists);
    }
    // 6. Amount validation
    if amount <= 0 {
        return Err(Error::InvalidAmount);
    }
    let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
    let client = token::Client::new(env, &token_addr);
    // 7. Sufficient balance (read-only)
    let balance = client.balance(&depositor);
    if balance < amount {
        return Err(Error::InsufficientFunds);
    }
    // 8. Fee computation (pure)
    let (
        lock_fee_rate,
        _release_fee_rate,
        lock_fixed_fee,
        _release_fixed,
        _fee_recipient,
        fee_enabled,
    ) = crate::fee::resolve_fee_config(env);
    let fee_amount =
        crate::fee::combined_fee_amount(amount, lock_fee_rate, lock_fixed_fee, fee_enabled);
    let net_amount = amount.checked_sub(fee_amount).unwrap_or(amount);
    if net_amount <= 0 {
        return Err(Error::InvalidAmount);
    }
    Ok((net_amount,))
}


pub(crate) fn publish_logic(env: Env, bounty_id: u64, publisher: Address) -> Result<(), Error> {
    // Validation precedence:
    // 1. Reentrancy guard
    // 2. Authorization (admin only)
    // 3. Escrow exists and is in Draft status

    // 1. Acquire reentrancy guard
    reentrancy_guard::acquire(&env);

    // 2. Admin authorization
    publisher.require_auth();

    // 3. Get escrow and verify it's in Draft status
    let mut escrow: Escrow = env
        .storage()
        .persistent()
        .get(&DataKey::Escrow(bounty_id))
        .ok_or(Error::BountyNotFound)?;

    if escrow.status != EscrowStatus::Draft {
        reentrancy_guard::release(&env);
        return Err(Error::FundsNotLocked);
    }

    // Transition from Draft to Locked
    escrow.status = EscrowStatus::Locked;
    env.storage()
        .persistent()
        .set(&DataKey::Escrow(bounty_id), &escrow);
    renew_escrow_record(&env, bounty_id, false);

    // Emit EscrowPublished event
    events::emit_escrow_published(
        &env,
        EscrowPublished {
            version: EVENT_VERSION_V2,
            bounty_id,
            published_by: publisher,
            timestamp: env.ledger().timestamp(),
        },
    );

    multitoken_invariants::assert_after_lock(&env);
    reentrancy_guard::release(&env);
    Ok(())
}

pub(crate) fn renew_tracked_record(
    env: &Env,
    key: &DataKey,
    marker: &DataKey,
    live_ttl: u32,
    archival_ttl: u32,
    archival: bool,
) {
    if !env.storage().persistent().has(key) {
        return;
    }

    let extension_ttl = if archival { archival_ttl } else { live_ttl };
    let renewal_threshold = extension_ttl / TTL_RENEWAL_DIVISOR;
    let current_ledger = env.ledger().sequence();
    let previous: Option<u32> = env.storage().persistent().get(marker);

    if previous
        .map(|live_until| {
            live_until.saturating_sub(current_ledger) <= renewal_threshold
        })
        .unwrap_or(true)
    {
        env.storage()
            .persistent()
            .extend_ttl(key, renewal_threshold, extension_ttl);
        env.storage()
            .persistent()
            .set(marker, &current_ledger.saturating_add(extension_ttl));
    }

    env.storage()
        .persistent()
        .extend_ttl(
            marker,
            ARCHIVAL_MARKER_TTL / TTL_RENEWAL_DIVISOR,
            ARCHIVAL_MARKER_TTL,
        );
}


pub(crate) fn renew_escrow_record(env: &Env, bounty_id: u64, archival: bool) {
    let regular = DataKey::Escrow(bounty_id);
    let anonymous = DataKey::EscrowAnon(bounty_id);
    let marker = DataKey::EscrowTtl(bounty_id);
    if env.storage().persistent().has(&regular) {
        let escrow: Option<Escrow> = env.storage().persistent().get(&regular);
        renew_tracked_record(
            env,
            &regular,
            &marker,
            ESCROW_LIVE_TTL,
            ESCROW_ARCHIVAL_TTL,
            archival,
        );
        if archival {
            renew_escrow_index(env, true);
            if let Some(escrow) = escrow {
                renew_depositor_index(env, &escrow.depositor, true);
            }
        }
    } else if env.storage().persistent().has(&anonymous) {
        renew_tracked_record(
            env,
            &anonymous,
            &marker,
            ESCROW_LIVE_TTL,
            ESCROW_ARCHIVAL_TTL,
            archival,
        );
        if archival {
            renew_escrow_index(env, true);
        }
    }
}


pub(crate) fn renew_claim_record(env: &Env, bounty_id: u64, archival: bool) {
    renew_tracked_record(
        env,
        &DataKey::PendingClaim(bounty_id),
        &DataKey::ClaimTtl(bounty_id),
        CLAIM_LIVE_TTL,
        CLAIM_ARCHIVAL_TTL,
        archival,
    );
}


pub(crate) fn renew_capability_record(env: &Env, capability_id: &BytesN<32>, archival: bool) {
    renew_tracked_record(
        env,
        &DataKey::Capability(capability_id.clone()),
        &DataKey::CapabilityTtl(capability_id.clone()),
        COMMITMENT_LIVE_TTL,
        COMMITMENT_ARCHIVAL_TTL,
        archival,
    );
}


pub(crate) fn renew_escrow_index(env: &Env, archival: bool) {
    renew_tracked_record(
        env,
        &DataKey::EscrowIndex,
        &DataKey::EscrowIndexTtl,
        INDEX_LIVE_TTL,
        INDEX_ARCHIVAL_TTL,
        archival,
    );
}


pub(crate) fn renew_depositor_index(env: &Env, depositor: &Address, archival: bool) {
    renew_tracked_record(
        env,
        &DataKey::DepositorIndex(depositor.clone()),
        &DataKey::DepositorIndexTtl(depositor.clone()),
        INDEX_LIVE_TTL,
        INDEX_ARCHIVAL_TTL,
        archival,
    );
}

// ─────────────────────────────────────────────────────────────────
// Public entry points (dispatcher targets)
// ─────────────────────────────────────────────────────────────────


/// Locks funds for a bounty and records escrow state.
///
/// # Security
/// - Validation order is deterministic to avoid ambiguous failure behavior under contention.
/// - Reentrancy guard is acquired before validation and released on completion.
///
/// # Errors
/// Returns `Error` variants for initialization, policy, authorization, and duplicate-bounty
/// failures.
pub fn lock_funds(
    env: Env,
    depositor: Address,
    bounty_id: u64,
    amount: i128,
    deadline: u64,
) -> Result<(), Error> {
    let res =
        lock_funds_logic(env.clone(), depositor.clone(), bounty_id, amount, deadline);
    monitoring::track_operation(&env, symbol_short!("lock"), depositor, res.is_ok());
    res
}


/// Simulate lock operation without state changes or token transfers.
///
/// Returns a `SimulationResult` indicating whether the operation would succeed and the
/// resulting escrow state. Does not require authorization; safe for off-chain preview.
///
/// # Arguments
/// * `depositor` - Address that would lock funds
/// * `bounty_id` - Bounty identifier
/// * `amount` - Amount to lock
/// * `deadline` - Deadline timestamp
///
/// # Security
/// This function performs only read operations. No storage writes, token transfers,
/// or events are emitted.
pub fn archive_escrow(env: Env, bounty_id: u64) -> Result<(), Error> {
    let admin = rbac::require_admin(&env);
    admin.require_auth();

    let mut escrow = env
        .storage()
        .persistent()
        .get::<DataKey, Escrow>(&DataKey::Escrow(bounty_id))
        .ok_or(Error::BountyNotFound)?;

    escrow.archived = true;
    escrow.archived_at = Some(env.ledger().timestamp());

    env.storage()
        .persistent()
        .set(&DataKey::Escrow(bounty_id), &escrow);

    // Also check anon escrow
    if let Some(mut anon) = env
        .storage()
        .persistent()
        .get::<DataKey, AnonymousEscrow>(&DataKey::EscrowAnon(bounty_id))
    {
        anon.archived = true;
        anon.archived_at = Some(env.ledger().timestamp());
        env.storage()
            .persistent()
            .set(&DataKey::EscrowAnon(bounty_id), &anon);
    }
    renew_escrow_record(&env, bounty_id, true);

    events::emit_archived(&env, bounty_id, env.ledger().timestamp());
    Ok(())
}


/// Get all archived escrow IDs.
pub fn get_archived_escrows(env: Env) -> Vec<u64> {
    let index: Vec<u64> = env
        .storage()
        .persistent()
        .get(&DataKey::EscrowIndex)
        .unwrap_or(Vec::new(&env));
    if !index.is_empty() {
        renew_escrow_index(&env, false);
    }
    let mut archived = Vec::new(&env);
    for id in index.iter() {
        if let Some(escrow) = env
            .storage()
            .persistent()
            .get::<DataKey, Escrow>(&DataKey::Escrow(id))
        {
            let terminal = escrow.archived
                || matches!(escrow.status, EscrowStatus::Released | EscrowStatus::Refunded);
            renew_escrow_record(&env, id, terminal);
            if escrow.archived {
                archived.push_back(id);
            }
        } else if let Some(anon) = env
            .storage()
            .persistent()
            .get::<DataKey, AnonymousEscrow>(&DataKey::EscrowAnon(id))
        {
            let terminal = anon.archived
                || matches!(anon.status, EscrowStatus::Released | EscrowStatus::Refunded);
            renew_escrow_record(&env, id, terminal);
            if anon.archived {
                archived.push_back(id);
            }
        }
    }
    archived
}


/// Simulation of a lock operation.
pub fn dry_run_lock(
    env: Env,
    depositor: Address,
    bounty_id: u64,
    amount: i128,
    deadline: u64,
) -> SimulationResult {
    fn err_result(e: Error) -> SimulationResult {
        SimulationResult {
            success: false,
            error_code: e as u32,
            amount: 0,
            resulting_status: EscrowStatus::Locked,
            remaining_amount: 0,
        }
    }
    match dry_run_lock_impl(&env, depositor, bounty_id, amount, deadline) {
        Ok((net_amount,)) => SimulationResult {
            success: true,
            error_code: 0,
            amount: net_amount,
            resulting_status: EscrowStatus::Locked,
            remaining_amount: net_amount,
        },
        Err(e) => err_result(e),
    }
}


/// Returns whether the given bounty escrow is marked as using non-transferable (soulbound)
/// reward tokens. When true, the token is expected to disallow further transfers after claim.
pub fn get_non_transferable_rewards(env: Env, bounty_id: u64) -> Result<bool, Error> {
    if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
        return Err(Error::BountyNotFound);
    }
    Ok(env
        .storage()
        .persistent()
        .get(&DataKey::NonTransferableRewards(bounty_id))
        .unwrap_or(false))
}


/// Lock funds for a bounty in anonymous mode: only a 32-byte depositor commitment is stored.
/// The depositor must authorize and transfer; their address is used only for the transfer
/// in this call and is not stored on-chain. Refunds require the configured anonymous
/// resolver to call `refund_resolved(bounty_id, recipient)`.
pub fn lock_funds_anonymous(
    env: Env,
    depositor: Address,
    depositor_commitment: BytesN<32>,
    bounty_id: u64,
    amount: i128,
    deadline: u64,
) -> Result<(), Error> {
    // Validation precedence (deterministic ordering):
    // 1. Reentrancy guard
    // 2. Contract initialized
    // 3. Paused (operational state)
    // 4. Rate limiting
    // 5. Authorization
    // 6. Business logic (bounty uniqueness, amount policy)

    // 1. Reentrancy guard
    reentrancy_guard::acquire(&env);

    // 2. Contract must be initialized
    if !env.storage().instance().has(&DataKey::Admin) {
        reentrancy_guard::release(&env);
        return Err(Error::NotInitialized);
    }

    // 3. Operational state: paused
    if crate::pause_freeze::check_paused(&env, symbol_short!("lock")) {
        reentrancy_guard::release(&env);
        return Err(Error::FundsPaused);
    }

    // 4. Rate limiting
    anti_abuse::check_rate_limit(&env, depositor.clone());

    // 5. Authorization
    depositor.require_auth();

    if env.storage().persistent().has(&DataKey::Escrow(bounty_id))
        || env
            .storage()
            .persistent()
            .has(&DataKey::EscrowAnon(bounty_id))
    {
        reentrancy_guard::release(&env);
        return Err(Error::BountyExists);
    }

    if let Some((min_amount, max_amount)) = env
        .storage()
        .instance()
        .get::<DataKey, (i128, i128)>(&DataKey::AmountPolicy)
    {
        if amount < min_amount {
            reentrancy_guard::release(&env);
            return Err(Error::AmountBelowMinimum);
        }
        if amount > max_amount {
            reentrancy_guard::release(&env);
            return Err(Error::AmountAboveMaximum);
        }
    }

    let escrow_anon = AnonymousEscrow {
        depositor_commitment: depositor_commitment.clone(),
        amount,
        remaining_amount: amount,
        status: EscrowStatus::Locked,
        deadline,
        refund_history: vec![&env],
        archived: false,
        archived_at: None,
    };

    // EFFECTS: update state before interaction (CEI)
    env.storage()
        .persistent()
        .set(&DataKey::EscrowAnon(bounty_id), &escrow_anon);
    renew_escrow_record(&env, bounty_id, false);

    let mut index: Vec<u64> = env
        .storage()
        .persistent()
        .get(&DataKey::EscrowIndex)
        .unwrap_or(Vec::new(&env));
    index.push_back(bounty_id);
    env.storage()
        .persistent()
        .set(&DataKey::EscrowIndex, &index);
    renew_escrow_index(&env, false);

    // INTERACTION: external token transfer after state finalized
    let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
    let client = token::Client::new(&env, &token_addr);
    client.transfer(&depositor, &env.current_contract_address(), &amount);

    emit_funds_locked_anon(
        &env,
        FundsLockedAnon {
            version: EVENT_VERSION_V2,
            bounty_id,
            amount,
            depositor_commitment,
            deadline,
        },
    );

    multitoken_invariants::assert_after_lock(&env);
    reentrancy_guard::release(&env);
    Ok(())
}


/// Releases escrowed funds to a contributor.
///
/// # Access Control
/// Admin-only.
///
/// # Front-running Behavior
/// First valid release for a bounty transitions state to `Released`. Later release/refund/claim
/// races against that bounty must fail with `Error::FundsNotLocked`.
///
/// # Security
/// Reentrancy guard is always cleared before any explicit error return after acquisition.
pub fn publish(env: Env, bounty_id: u64) -> Result<(), Error> {
    let _caller = env
        .storage()
        .instance()
        .get::<DataKey, Address>(&DataKey::Admin)
        .expect("Admin not set");
    publish_logic(env, bounty_id, _caller)
}


/// Update multisig configuration (admin only)
pub fn update_multisig_config(
    env: Env,
    threshold_amount: i128,
    signers: Vec<Address>,
    required_signatures: u32,
) -> Result<(), Error> {
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }

    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();

    if required_signatures > signers.len() {
        return Err(Error::InvalidAmount);
    }

    let config = MultisigConfig {
        threshold_amount,
        signers,
        required_signatures,
    };

    env.storage()
        .instance()
        .set(&DataKey::MultisigConfig, &config);

    Ok(())
}


/// Get multisig configuration
pub fn get_multisig_config(env: Env) -> MultisigConfig {
    env.storage()
        .instance()
        .get(&DataKey::MultisigConfig)
        .unwrap_or(MultisigConfig {
            threshold_amount: i128::MAX,
            signers: vec![&env],
            required_signatures: 0,
        })
}


/// Approve release for large amount (requires multisig)
pub fn approve_large_release(
    env: Env,
    bounty_id: u64,
    contributor: Address,
    approver: Address,
) -> Result<(), Error> {
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }

    let multisig_config: MultisigConfig = get_multisig_config(env.clone());

    let mut is_signer = false;
    for signer in multisig_config.signers.iter() {
        if signer == approver {
            is_signer = true;
            break;
        }
    }

    if !is_signer {
        return Err(Error::Unauthorized);
    }

    approver.require_auth();

    let approval_key = DataKey::ReleaseApproval(bounty_id);
    let mut approval: ReleaseApproval = env
        .storage()
        .persistent()
        .get(&approval_key)
        .unwrap_or(ReleaseApproval {
            bounty_id,
            contributor: contributor.clone(),
            approvals: vec![&env],
        });

    for existing in approval.approvals.iter() {
        if existing == approver {
            return Ok(());
        }
    }

    approval.approvals.push_back(approver.clone());
    env.storage().persistent().set(&approval_key, &approval);

    events::emit_approval_added(
        &env,
        events::ApprovalAdded {
            version: EVENT_VERSION_V2,
            bounty_id,
            contributor: contributor.clone(),
            approver,
            timestamp: env.ledger().timestamp(),
        },
    );

    Ok(())
}


/// Batch lock funds for multiple bounties in a single atomic transaction.
///
/// Locks between 1 and [`MAX_BATCH_SIZE`] bounties in one call, reducing
/// per-transaction overhead compared to repeated single-item `lock_funds`
/// calls.
///
/// ## Batch failure semantics
///
/// This operation is **strictly atomic** (all-or-nothing):
///
/// 1. All items are validated in a single pass **before** any state is
///    mutated or any token transfer is initiated.
/// 2. If *any* item fails validation the entire call reverts immediately.
///    No escrow record is written, no token is transferred, and every
///    "sibling" row in the same batch is left completely unaffected.
/// 3. After a failed batch the contract is in exactly the same state as
///    before the call; subsequent operations behave as if this call never
///    happened.
///
/// ## Ordering guarantee
///
/// Items are processed in ascending `bounty_id` order regardless of the
/// caller-supplied ordering. This ensures deterministic execution and
/// eliminates ordering-based front-running attacks.
///
/// ## Checks-Effects-Interactions (CEI)
///
/// All escrow records and index updates are written in a first pass
/// (Effects); external token transfers and event emissions happen in a
/// second pass (Interactions). This ordering prevents reentrancy attacks.
///
/// # Arguments
/// * `items` - 1–[`MAX_BATCH_SIZE`] [`LockFundsItem`] entries (bounty_id,
///   depositor, amount, deadline).
///
/// # Returns
/// Number of bounties successfully locked (equals `items.len()` on success).
///
/// # Errors
/// * [`Error::InvalidBatchSize`] — batch is empty or exceeds `MAX_BATCH_SIZE`
/// * [`Error::ContractDeprecated`] — contract has been killed via `set_deprecated`
/// * [`Error::FundsPaused`] — lock operations are currently paused
/// * [`Error::NotInitialized`] — `init` has not been called
/// * [`Error::BountyExists`] — a `bounty_id` already exists in storage
/// * [`Error::DuplicateBountyId`] — the same `bounty_id` appears more than once
/// * [`Error::InvalidAmount`] — any item has `amount ≤ 0`
/// * [`Error::ParticipantBlocked`] / [`Error::ParticipantNotAllowed`] — participant filter
///
/// # Reentrancy
/// Protected by the shared reentrancy guard (acquired before validation,
/// released after all effects and interactions complete).
pub fn batch_lock_funds(env: Env, items: Vec<LockFundsItem>) -> Result<u32, Error> {
    if crate::pause_freeze::check_paused(&env, symbol_short!("lock")) {
        return Err(Error::FundsPaused);
    }

    // GUARD: acquire reentrancy lock
    reentrancy_guard::acquire(&env);
    // Snapshot resource meters for gas cap enforcement (test / testutils only).
    #[cfg(any(test, feature = "testutils"))]
    let gas_snapshot = gas_budget::capture(&env);
    let result: Result<u32, Error> = (|| {
        if crate::participant_filter::get_deprecation_state(&env).deprecated {
            reentrancy_guard::release(&env);
            return Err(Error::ContractDeprecated);
        }
        // Validate batch size
        let batch_size = items.len();
        if batch_size == 0 {
            reentrancy_guard::release(&env);
            return Err(Error::InvalidBatchSize);
        }
        let max_batch_size = crate::admin::get_max_batch_size(env.clone());
        if batch_size > max_batch_size {
            reentrancy_guard::release(&env);
            return Err(Error::InvalidBatchSize);
        }

        if !env.storage().instance().has(&DataKey::Admin) {
            reentrancy_guard::release(&env);
            return Err(Error::NotInitialized);
        }

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        let contract_address = env.current_contract_address();
        let timestamp = env.ledger().timestamp();

        // Validate all items before processing (all-or-nothing approach)
        for item in items.iter() {
            // Participant filtering (blocklist-only / allowlist-only / disabled)
            crate::participant_filter::check_participant_filter(&env, item.depositor.clone())?;

            // Check if bounty already exists
            if env
                .storage()
                .persistent()
                .has(&DataKey::Escrow(item.bounty_id))
            {
                reentrancy_guard::release(&env);
                return Err(Error::BountyExists);
            }

            // Validate amount
            if item.amount <= 0 {
                reentrancy_guard::release(&env);
                return Err(Error::InvalidAmount);
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
        }

        let ordered_items = crate::admin::order_batch_lock_items(&env, &items);

        // Collect unique depositors and require auth once for each
        // This prevents "frame is already authorized" errors when same depositor appears multiple times
        let mut seen_depositors: Vec<Address> = Vec::new(&env);
        for item in ordered_items.iter() {
            let mut found = false;
            for seen in seen_depositors.iter() {
                if seen.clone() == item.depositor {
                    found = true;
                    break;
                }
            }
            if !found {
                seen_depositors.push_back(item.depositor.clone());
                item.depositor.require_auth();
            }
        }

        // Process all items (atomic - all succeed or all fail)
        // First loop: write all state (escrow, indices). Second loop: transfers + events.
        let mut locked_count = 0u32;
        for item in ordered_items.iter() {
            let escrow = Escrow {
                depositor: item.depositor.clone(),
                amount: item.amount,
                status: EscrowStatus::Locked,
                deadline: item.deadline,
                refund_history: vec![&env],
                remaining_amount: item.amount,
                archived: false,
                archived_at: None,
            };

            env.storage()
                .persistent()
                .set(&DataKey::Escrow(item.bounty_id), &escrow);
            renew_escrow_record(&env, item.bounty_id, false);

            let mut index: Vec<u64> = env
                .storage()
                .persistent()
                .get(&DataKey::EscrowIndex)
                .unwrap_or(Vec::new(&env));
            index.push_back(item.bounty_id);
            env.storage()
                .persistent()
                .set(&DataKey::EscrowIndex, &index);
            renew_escrow_index(&env, false);

            let mut depositor_index: Vec<u64> = env
                .storage()
                .persistent()
                .get(&DataKey::DepositorIndex(item.depositor.clone()))
                .unwrap_or(Vec::new(&env));
            depositor_index.push_back(item.bounty_id);
            env.storage().persistent().set(
                &DataKey::DepositorIndex(item.depositor.clone()),
                &depositor_index,
            );
            renew_depositor_index(&env, &item.depositor, false);
        }

        // INTERACTION: all external token transfers happen after state is finalized
        for item in ordered_items.iter() {
            client.transfer(&item.depositor, &contract_address, &item.amount);

            emit_funds_locked(
                &env,
                FundsLocked {
                    version: EVENT_VERSION_V2,
                    bounty_id: item.bounty_id,
                    amount: item.amount,
                    depositor: item.depositor.clone(),
                    deadline: item.deadline,
                    correlation_id: None,
                },
            );

            locked_count += 1;
        }

        emit_batch_funds_locked(
            &env,
            BatchFundsLocked {
                version: EVENT_VERSION_V2,
                count: locked_count,
                total_amount: ordered_items
                    .iter()
                    .try_fold(0i128, |acc, i| acc.checked_add(i.amount))
                    .unwrap(),
                timestamp,
                correlation_id: None,
            },
        );
        Ok(locked_count)
    })();

    #[cfg(any(test, feature = "testutils"))]
    if result.is_ok() {
        let gas_cfg = gas_budget::get_config(&env);
        gas_budget::check(
            &env,
            symbol_short!("b_lock"),
            &gas_cfg.batch_lock,
            &gas_snapshot,
            gas_cfg.enforce,
        )?;
    }

    let locked_count = result?;
    multitoken_invariants::assert_after_lock(&env);
    reentrancy_guard::release(&env);
    Ok(locked_count)
}


/// Alias for batch_lock_funds to match the requested naming convention.
pub fn batch_lock(env: Env, items: Vec<LockFundsItem>) -> Result<u32, Error> {
    batch_lock_funds(env, items)
}


/// Structure-of-Arrays (SoA) variant of `batch_lock_funds`.
/// Reduces host-to-guest deserialization overhead by accepting parallel arrays
/// of primitives instead of an array of structs.
pub fn batch_lock_funds_soa(
    env: Env,
    bounty_ids: Vec<u64>,
    depositors: Vec<Address>,
    amounts: Vec<i128>,
    deadlines: Vec<u64>,
) -> Result<u32, Error> {
    if bounty_ids.len() != depositors.len()
        || bounty_ids.len() != amounts.len()
        || bounty_ids.len() != deadlines.len()
    {
        return Err(Error::BatchSizeMismatch);
    }

    let mut items = Vec::new(&env);
    for i in 0..bounty_ids.len() {
        items.push_back(LockFundsItem {
            bounty_id: bounty_ids.get(i).unwrap(),
            depositor: depositors.get(i).unwrap(),
            amount: amounts.get(i).unwrap(),
            deadline: deadlines.get(i).unwrap(),
        });
    }
    batch_lock_funds(env, items)
}

