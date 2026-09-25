//! Fee configuration, per-bounty routing overrides, and treasury distribution.



use soroban_sdk::{token, symbol_short, Address, Env, Vec};
use crate::{
    events, rbac,
    DataKey, Error, FeeConfig, PerBountyFeeRouting, TokenFeeConfig, TreasuryDestination,
    EscrowStatus,
    BASIS_POINTS, MAX_FEE_RATE,
    events::{EVENT_VERSION_V2},
};

// ─────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────


/// Calculate fee amount based on rate (in basis points), using **ceiling division**.
///
/// Ceiling division ensures that a non-zero fee rate always produces at least
/// 1 stroop of fee, regardless of how small the individual amount is.  This
/// closes the principal-drain vector where an attacker breaks a large deposit
/// into dust amounts that each round down to a zero fee.
///
/// Formula: ceil(amount * fee_rate / BASIS_POINTS)
///        = (amount * fee_rate + BASIS_POINTS - 1) / BASIS_POINTS
///
/// # Panics
/// Returns 0 on arithmetic overflow rather than panicking.
pub(crate) fn calculate_fee(amount: i128, fee_rate: i128) -> i128 {
    if fee_rate == 0 || amount == 0 {
        return 0;
    }
    // Ceiling integer division: (a + b - 1) / b
    let numerator = amount
        .checked_mul(fee_rate)
        .and_then(|x| x.checked_add(BASIS_POINTS - 1))
        .unwrap_or(0);
    if numerator == 0 {
        return 0;
    }
    numerator / BASIS_POINTS
}


/// Total fee on `amount`: ceiling percentage plus optional fixed, capped at `amount`.
pub(crate) fn combined_fee_amount(amount: i128, rate_bps: i128, fixed: i128, fee_enabled: bool) -> i128 {
    if !fee_enabled || amount <= 0 {
        return 0;
    }
    if fixed < 0 {
        return 0;
    }
    let pct = calculate_fee(amount, rate_bps);
    let sum = pct.saturating_add(fixed);
    sum.min(amount).max(0)
}




/// Get fee configuration (internal helper)
pub(crate) fn get_fee_config_internal(env: &Env) -> FeeConfig {
    env.storage()
        .instance()
        .get(&DataKey::FeeConfig)
        .unwrap_or_else(|| FeeConfig {
            lock_fee_rate: 0,
            release_fee_rate: 0,
            lock_fixed_fee: 0,
            release_fixed_fee: 0,
            fee_recipient: env.storage().instance().get(&DataKey::Admin).unwrap(),
            fee_enabled: false,
            treasury_destinations: Vec::new(env),
            distribution_enabled: false,
        })
}


/// Validates treasury destinations before enabling multi-region routing.
pub(crate) fn validate_treasury_destinations(
    _env: &Env,
    destinations: &Vec<TreasuryDestination>,
    distribution_enabled: bool,
) -> Result<(), Error> {
    if !distribution_enabled {
        return Ok(());
    }

    if destinations.is_empty() {
        return Err(Error::InvalidAmount);
    }

    let mut total_weight: u64 = 0;
    for destination in destinations.iter() {
        if destination.weight == 0 {
            return Err(Error::InvalidAmount);
        }

        if destination.region.is_empty() || destination.region.len() > 50 {
            return Err(Error::InvalidAmount);
        }

        total_weight = total_weight
            .checked_add(destination.weight as u64)
            .ok_or(Error::InvalidAmount)?;
    }

    if total_weight == 0 {
        return Err(Error::InvalidAmount);
    }

    Ok(())
}


/// Routes a fee either to the configured fee recipient or across weighted treasury routes.
///
/// # Invariant
/// After routing, `distributed_total == amount` must hold. This is enforced
/// by assigning any rounding remainder to the final destination and verified
/// by emitting a [`events::FeeRoutingInvariantChecked`] audit event.
///
/// # Panics
/// Panics if `distributed_total != amount` after routing (invariant violation).
pub(crate) fn route_fee(
    env: &Env,
    client: &token::Client,
    config: &FeeConfig,
    bounty_id: u64,
    amount: i128,
    fee_rate: i128,
    operation_type: events::FeeOperationType,
) -> Result<(), Error> {
    if amount <= 0 {
        return Ok(());
    }

    let fee_fixed = match operation_type {
        events::FeeOperationType::Lock => config.lock_fixed_fee,
        events::FeeOperationType::Release => config.release_fixed_fee,
    };

    if !config.distribution_enabled || config.treasury_destinations.is_empty() {
        client.transfer(
            &env.current_contract_address(),
            &config.fee_recipient,
            &amount,
        );
        events::emit_fee_collected(
            env,
            events::FeeCollected {
                version: EVENT_VERSION_V2,
                operation_type: operation_type.clone(),
                amount,
                fee_rate,
                fee_fixed,
                recipient: config.fee_recipient.clone(),
                timestamp: env.ledger().timestamp(),
            },
        );
        // Single-recipient: invariant trivially holds (distributed == amount).
        events::emit_fee_routing_invariant_checked(
            env,
            events::FeeRoutingInvariantChecked {
                version: EVENT_VERSION_V2,
                bounty_id,
                operation_type,
                gross_amount: amount,
                fee_amount: amount,
                distributed_total: amount,
                weight_total: 1,
                destination_count: 1,
                invariant_ok: true,
                timestamp: env.ledger().timestamp(),
            },
        );
        return Ok(());
    }

    let mut total_weight: u64 = 0;
    for destination in config.treasury_destinations.iter() {
        total_weight = total_weight
            .checked_add(destination.weight as u64)
            .ok_or(Error::InvalidAmount)?;
    }
    if total_weight == 0 {
        return Err(Error::InvalidAmount);
    }

    let mut distributed = 0i128;
    let destination_count = config.treasury_destinations.len() as usize;

    for (index, destination) in config.treasury_destinations.iter().enumerate() {
        let share = if index + 1 == destination_count {
            // Last destination absorbs any rounding remainder, ensuring
            // distributed_total == amount (fee routing invariant).
            amount
                .checked_sub(distributed)
                .ok_or(Error::InvalidAmount)?
        } else {
            amount
                .checked_mul(destination.weight as i128)
                .and_then(|v| v.checked_div(total_weight as i128))
                .ok_or(Error::InvalidAmount)?
        };

        distributed = distributed.checked_add(share).ok_or(Error::InvalidAmount)?;
        if share <= 0 {
            continue;
        }

        client.transfer(
            &env.current_contract_address(),
            &destination.address,
            &share,
        );
        events::emit_fee_collected(
            env,
            events::FeeCollected {
                version: EVENT_VERSION_V2,
                operation_type: operation_type.clone(),
                amount: share,
                fee_rate,
                fee_fixed,
                recipient: destination.address,
                timestamp: env.ledger().timestamp(),
            },
        );
    }

    // Enforce the fee routing invariant: every stroop must be accounted for.
    // This is a hard invariant — a violation indicates a logic or overflow bug.
    let invariant_ok = distributed == amount;
    events::emit_fee_routing_invariant_checked(
        env,
        events::FeeRoutingInvariantChecked {
            version: EVENT_VERSION_V2,
            bounty_id,
            operation_type,
            gross_amount: amount,
            fee_amount: amount,
            distributed_total: distributed,
            weight_total: total_weight,
            destination_count: destination_count as u32,
            invariant_ok,
            timestamp: env.ledger().timestamp(),
        },
    );
    if !invariant_ok {
        panic!("Fee routing invariant violated: distributed != fee_amount");
    }

    Ok(())
}


/// Internal: whether per-bounty fee routing is immutable for `bounty_id`.
///
/// Routing locks as soon as depositor funds are committed: a regular
/// escrow is mutable only while in `Draft` status; an anonymous escrow is
/// created directly in `Locked` status and is therefore always locked.
/// Callers must have already verified that the bounty exists.
pub(crate) fn fee_routing_is_locked(env: &Env, bounty_id: u64) -> bool {
    if let Some(escrow) = env
        .storage()
        .persistent()
        .get::<DataKey, Escrow>(&DataKey::Escrow(bounty_id))
    {
        escrow.status != EscrowStatus::Draft
    } else {
        // Anonymous escrows never pass through Draft.
        env.storage()
            .persistent()
            .has(&DataKey::EscrowAnon(bounty_id))
    }
}


/// Internal: shared validation, storage, and audit-event emission for the
/// pre-lock and post-lock fee routing paths.
///
/// `allow_post_lock` is `true` only for the audited
/// `set_fee_routing_with_reason` path, which must supply `reason`.
pub(crate) fn set_fee_routing_internal(
    env: Env,
    bounty_id: u64,
    treasury_recipient: Address,
    treasury_bps: i128,
    partner_recipient: Option<Address>,
    partner_bps: i128,
    allow_post_lock: bool,
    reason: Option<soroban_sdk::String>,
) -> Result<(), Error> {
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }
    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();

    // Bounty must exist (regular or anonymous).
    if !env.storage().persistent().has(&DataKey::Escrow(bounty_id))
        && !env
            .storage()
            .persistent()
            .has(&DataKey::EscrowAnon(bounty_id))
    {
        return Err(Error::BountyNotFound);
    }

    // Immutability guard: once funds are committed (Locked or any later
    // status), the non-audited path may not change where fees land.
    if !allow_post_lock && fee_routing_is_locked(&env, bounty_id) {
        return Err(Error::FeeRoutingLocked);
    }

    // Validate share invariants.
    if !(0..=BASIS_POINTS).contains(&treasury_bps) {
        return Err(Error::InvalidAmount);
    }
    if !(0..=BASIS_POINTS).contains(&partner_bps) {
        return Err(Error::InvalidAmount);
    }
    match &partner_recipient {
        None => {
            // No partner: treasury must take 100 % and partner_bps must be 0.
            if treasury_bps != BASIS_POINTS || partner_bps != 0 {
                return Err(Error::InvalidAmount);
            }
        }
        Some(_) => {
            // Partner present: shares must sum to exactly 100 %.
            if treasury_bps.checked_add(partner_bps).unwrap_or(-1) != BASIS_POINTS {
                return Err(Error::InvalidAmount);
            }
        }
    }

    // Capture the outgoing routing for the audit event before overwriting.
    let previous: Option<PerBountyFeeRouting> = env
        .storage()
        .persistent()
        .get(&DataKey::PerBountyFeeRouting(bounty_id));

    let routing = PerBountyFeeRouting {
        treasury_recipient: treasury_recipient.clone(),
        treasury_bps,
        partner_recipient: partner_recipient.clone(),
        partner_bps,
    };

    env.storage()
        .persistent()
        .set(&DataKey::PerBountyFeeRouting(bounty_id), &routing);

    events::emit_fee_routing_updated(
        &env,
        events::FeeRoutingUpdated {
            version: EVENT_VERSION_V2,
            bounty_id,
            treasury_recipient: treasury_recipient.clone(),
            treasury_bps,
            partner_recipient: partner_recipient.clone(),
            partner_bps,
            timestamp: env.ledger().timestamp(),
        },
    );

    events::emit_fee_routing_changed(
        &env,
        events::FeeRoutingChanged {
            version: EVENT_VERSION_V2,
            bounty_id,
            old_treasury_recipient: previous.as_ref().map(|p| p.treasury_recipient.clone()),
            old_partner_recipient: previous.as_ref().and_then(|p| p.partner_recipient.clone()),
            new_treasury_recipient: treasury_recipient,
            new_partner_recipient: partner_recipient,
            changed_by: admin,
            post_lock_override: allow_post_lock,
            reason,
            timestamp: env.ledger().timestamp(),
        },
    );

    Ok(())
}


/// Internal: route a fee using per-bounty routing when available, falling back to
/// the global `route_fee` path.
///
/// Emits [`events::FeeRouted`] when per-bounty routing is active so indexers can
/// reconstruct the exact split without inspecting storage.
pub(crate) fn route_fee_for_bounty(
    env: &Env,
    client: &token::Client,
    config: &FeeConfig,
    bounty_id: u64,
    fee_amount: i128,
    fee_rate: i128,
    gross_amount: i128,
    operation_type: events::FeeOperationType,
) -> Result<(), Error> {
    if fee_amount <= 0 {
        return Ok(());
    }

    // Check for a per-bounty routing override.
    let maybe_routing: Option<PerBountyFeeRouting> = env
        .storage()
        .persistent()
        .get(&DataKey::PerBountyFeeRouting(bounty_id));

    match maybe_routing {
        None => {
            // No per-bounty override — use the global route_fee path.
            route_fee(
                env,
                client,
                config,
                bounty_id,
                fee_amount,
                fee_rate,
                operation_type,
            )
        }
        Some(routing) => {
            // Per-bounty routing: split fee between treasury and optional partner.
            // Invariant: treasury_share + partner_share == fee_amount (last leg absorbs remainder).
            let treasury_share = if routing.partner_recipient.is_some() {
                fee_amount
                    .checked_mul(routing.treasury_bps)
                    .and_then(|v| v.checked_div(BASIS_POINTS))
                    .ok_or(Error::InvalidAmount)?
            } else {
                fee_amount
            };

            let partner_share = fee_amount
                .checked_sub(treasury_share)
                .ok_or(Error::InvalidAmount)?;

            // Transfer treasury share.
            if treasury_share > 0 {
                client.transfer(
                    &env.current_contract_address(),
                    &routing.treasury_recipient,
                    &treasury_share,
                );
            }

            // Transfer partner share (if any).
            if partner_share > 0 {
                if let Some(ref partner) = routing.partner_recipient {
                    client.transfer(&env.current_contract_address(), partner, &partner_share);
                }
            }

            // Verify invariant: distributed == fee_amount.
            let distributed = treasury_share
                .checked_add(partner_share)
                .ok_or(Error::InvalidAmount)?;
            let invariant_ok = distributed == fee_amount;

            // Emit FeeRouted audit event.
            events::emit_fee_routed(
                env,
                events::FeeRouted {
                    version: EVENT_VERSION_V2,
                    bounty_id,
                    operation_type: operation_type.clone(),
                    gross_amount,
                    total_fee: fee_amount,
                    fee_rate,
                    treasury_recipient: routing.treasury_recipient.clone(),
                    treasury_fee: treasury_share,
                    partner_recipient: routing.partner_recipient.clone(),
                    partner_fee: partner_share,
                    timestamp: env.ledger().timestamp(),
                },
            );

            // Emit invariant-checked audit event.
            events::emit_fee_routing_invariant_checked(
                env,
                events::FeeRoutingInvariantChecked {
                    version: EVENT_VERSION_V2,
                    bounty_id,
                    operation_type,
                    gross_amount,
                    fee_amount,
                    distributed_total: distributed,
                    weight_total: BASIS_POINTS as u64,
                    destination_count: if routing.partner_recipient.is_some() {
                        2
                    } else {
                        1
                    },
                    invariant_ok,
                    timestamp: env.ledger().timestamp(),
                },
            );

            if !invariant_ok {
                panic!("Fee routing invariant violated: distributed != fee_amount");
            }

            Ok(())
        }
    }
}


/// Internal: resolve the effective fee config for the escrow token.
///
/// # Precedence (global kill-switch first)
///
/// 1. **Global `FeeConfig.fee_enabled`** is the master kill-switch.
///    When `false`, no fees are collected for *any* token, regardless of
///    any per-token `TokenFeeConfig` override. This lets an admin halt all
///    fee collection in a single operation without needing to clear every
///    per-token config.
///
/// 2. **`TokenFeeConfig(token)`** — when present, its rate/fixed/recipient
///    fields override the global `FeeConfig` for that specific token.
///    However, its `fee_enabled` is **AND-ed** with the global
///    `fee_enabled`: the per-token flag can only *further restrict* fee
///    collection (i.e. disable it for that token), never re-enable it
///    when the global kill-switch is off.
///
/// 3. **Global `FeeConfig` fallback** — used when no per-token override
///    exists.
///
/// # Returns
/// `(lock_fee_rate, release_fee_rate, lock_fixed_fee, release_fixed_fee, fee_recipient, fee_enabled)`
pub(crate) fn resolve_fee_config(env: &Env) -> (i128, i128, i128, i128, Address, bool) {
    let global = get_fee_config_internal(env);
    let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
    if let Some(tok_cfg) = env
        .storage()
        .instance()
        .get::<DataKey, TokenFeeConfig>(&DataKey::TokenFeeConfig(token_addr))
    {
        (
            tok_cfg.lock_fee_rate,
            tok_cfg.release_fee_rate,
            tok_cfg.lock_fixed_fee,
            tok_cfg.release_fixed_fee,
            tok_cfg.fee_recipient,
            // Global kill-switch AND per-token flag: the global can only
            // disable fees; the per-token flag can only further restrict.
            global.fee_enabled && tok_cfg.fee_enabled,
        )
    } else {
        (
            global.lock_fee_rate,
            global.release_fee_rate,
            global.lock_fixed_fee,
            global.release_fixed_fee,
            global.fee_recipient,
            global.fee_enabled,
        )
    }
}

// ─────────────────────────────────────────────────────────────────
// Public entry points (dispatcher targets)
// ─────────────────────────────────────────────────────────────────


/// Update fee configuration (admin only)
pub fn update_fee_config(
    env: Env,
    lock_fee_rate: Option<i128>,
    release_fee_rate: Option<i128>,
    lock_fixed_fee: Option<i128>,
    release_fixed_fee: Option<i128>,
    fee_recipient: Option<Address>,
    fee_enabled: Option<bool>,
) -> Result<(), Error> {
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }

    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();

    let mut fee_config = get_fee_config_internal(&env);

    if let Some(rate) = lock_fee_rate {
        if !(0..=MAX_FEE_RATE).contains(&rate) {
            return Err(Error::InvalidFeeRate);
        }
        fee_config.lock_fee_rate = rate;
    }

    if let Some(rate) = release_fee_rate {
        if !(0..=MAX_FEE_RATE).contains(&rate) {
            return Err(Error::InvalidFeeRate);
        }
        fee_config.release_fee_rate = rate;
    }

    if let Some(fixed) = lock_fixed_fee {
        if fixed < 0 {
            return Err(Error::InvalidAmount);
        }
        fee_config.lock_fixed_fee = fixed;
    }

    if let Some(fixed) = release_fixed_fee {
        if fixed < 0 {
            return Err(Error::InvalidAmount);
        }
        fee_config.release_fixed_fee = fixed;
    }

    if let Some(recipient) = fee_recipient {
        fee_config.fee_recipient = recipient;
    }

    if let Some(enabled) = fee_enabled {
        fee_config.fee_enabled = enabled;
    }

    env.storage()
        .instance()
        .set(&DataKey::FeeConfig, &fee_config);

    events::emit_fee_config_updated(
        &env,
        events::FeeConfigUpdated {
            version: EVENT_VERSION_V2,
            lock_fee_rate: fee_config.lock_fee_rate,
            release_fee_rate: fee_config.release_fee_rate,
            lock_fixed_fee: fee_config.lock_fixed_fee,
            release_fixed_fee: fee_config.release_fixed_fee,
            fee_recipient: fee_config.fee_recipient.clone(),
            fee_enabled: fee_config.fee_enabled,
            timestamp: env.ledger().timestamp(),
        },
    );

    Ok(())
}


/// Configures weighted treasury destinations for multi-region fee routing.
///
/// When enabled, collected lock and release fees are routed proportionally
/// across `destinations` instead of sending the full amount to
/// `fee_recipient`. Disabled routing preserves the configured destinations
/// but falls back to the single-recipient path until re-enabled.
pub fn set_treasury_distributions(
    env: Env,
    destinations: Vec<TreasuryDestination>,
    distribution_enabled: bool,
) -> Result<(), Error> {
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }

    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();

    validate_treasury_destinations(&env, &destinations, distribution_enabled)?;

    let mut fee_config = get_fee_config_internal(&env);
    fee_config.treasury_destinations = destinations;
    fee_config.distribution_enabled = distribution_enabled;

    env.storage()
        .instance()
        .set(&DataKey::FeeConfig, &fee_config);

    Ok(())
}


/// Returns the current treasury routing configuration.
pub fn get_treasury_distributions(env: Env) -> (Vec<TreasuryDestination>, bool) {
    let fee_config = get_fee_config_internal(&env);
    (
        fee_config.treasury_destinations,
        fee_config.distribution_enabled,
    )
}


// ── Per-bounty fee routing ────────────────────────────────────────────────

/// Set a per-bounty fee routing override (admin only, **pre-lock only**).
///
/// When set, fees collected for `bounty_id` are split between
/// `treasury_recipient` and an optional `partner_recipient` according to
/// the supplied basis-point shares instead of using the global routing.
///
/// # Immutability guard
/// Routing can only be set through this path while the bounty is still in
/// `Draft` status. Once it transitions to `Locked` (or any later status)
/// — i.e. once depositors have committed funds under the routing they
/// observed — this call fails with [`Error::FeeRoutingLocked`]. Anonymous
/// escrows are created directly in `Locked` status, so they are always
/// post-lock here. To change routing after lock, use the audited
/// [`set_fee_routing_with_reason`] path, which requires a mandatory
/// reason and emits a `FeeRoutingChanged` event carrying the previous and
/// new destinations. See `docs/security/fee-routing-immutability.md`.
///
/// # Invariants enforced
/// - `treasury_bps + partner_bps == 10_000` (shares must sum to 100 %).
/// - `partner_bps == 0` when `partner_recipient` is `None`.
/// - Both shares must be in `[0, 10_000]`.
/// - The bounty must exist in persistent storage.
///
/// # Events
/// Emits `FeeRoutingUpdated` (legacy) and `FeeRoutingChanged` (audit,
/// with previous and new destinations) on every accepted change.
///
/// # Errors
/// * `NotInitialized`    – contract not yet initialised.
/// * `BountyNotFound`    – `bounty_id` does not exist.
/// * `FeeRoutingLocked`  – bounty already `Locked` or in a later status.
/// * `InvalidAmount`     – share invariant violated.
pub fn set_fee_routing(
    env: Env,
    bounty_id: u64,
    treasury_recipient: Address,
    treasury_bps: i128,
    partner_recipient: Option<Address>,
    partner_bps: i128,
) -> Result<(), Error> {
    set_fee_routing_internal(
        env,
        bounty_id,
        treasury_recipient,
        treasury_bps,
        partner_recipient,
        partner_bps,
        false,
        None,
    )
}


/// Change per-bounty fee routing **after** the bounty is locked
/// (admin only, audited override path).
///
/// This is the elevated counterpart to [`set_fee_routing`]: it
/// accepts routing changes regardless of escrow status, but demands a
/// non-empty `reason` string that is recorded on-chain in the
/// `FeeRoutingChanged` audit event together with the previous and new
/// destinations and the admin that made the change. Silent post-lock
/// re-routing is therefore impossible: every accepted change leaves an
/// indexable audit trail.
///
/// Share invariants are identical to [`set_fee_routing`].
///
/// # Errors
/// * `NotInitialized` – contract not yet initialised.
/// * `BountyNotFound` – `bounty_id` does not exist.
/// * `InvalidAmount`  – empty `reason`, or share invariant violated.
pub fn set_fee_routing_with_reason(
    env: Env,
    bounty_id: u64,
    treasury_recipient: Address,
    treasury_bps: i128,
    partner_recipient: Option<Address>,
    partner_bps: i128,
    reason: soroban_sdk::String,
) -> Result<(), Error> {
    // The audit trail is the entire point of this path: an empty reason
    // would defeat it, so reject it outright.
    if reason.is_empty() {
        return Err(Error::InvalidAmount);
    }
    set_fee_routing_internal(
        env,
        bounty_id,
        treasury_recipient,
        treasury_bps,
        partner_recipient,
        partner_bps,
        true,
        Some(reason),
    )
}


/// Return the per-bounty fee routing override for `bounty_id`, if one has been set.
///
/// Returns `None` when no override exists; callers should fall back to the
/// global `FeeConfig` routing in that case.
pub fn get_fee_routing(env: Env, bounty_id: u64) -> Option<PerBountyFeeRouting> {
    env.storage()
        .persistent()
        .get(&DataKey::PerBountyFeeRouting(bounty_id))
}


/// Get the current **global** fee configuration (view function).
///
/// # Precedence
/// The global config is the fallback used when no per-token
/// `TokenFeeConfig` exists for the escrow token, and its `fee_enabled`
/// flag is the **master kill-switch**: when `false`, no fee is charged
/// for *any* token — even one with an active per-token override whose
/// own `fee_enabled` is `true`. The effective flag is
/// `global.fee_enabled AND token.fee_enabled` (see `resolve_fee_config`
/// and [`set_token_fee_config`]).
///
/// Note: this returns the stored global config as-is; it does not apply
/// any per-token override. Use `get_token_fee_config` to inspect a
/// token's override.
pub fn get_fee_config(env: Env) -> FeeConfig {
    get_fee_config_internal(&env)
}


/// Set a per-token fee configuration (admin only).
///
/// When a `TokenFeeConfig` is set for a given token address, its rate,
/// fixed-fee, and recipient fields take precedence over the global
/// `FeeConfig` for all escrows denominated in that token.  However, its
/// `fee_enabled` flag is **AND-ed** with the global kill-switch
/// (`FeeConfig.fee_enabled`): the per-token flag can only *further
/// restrict* fee collection — it can **never** re-enable fees when the
/// global kill-switch is `false`.
///
/// # Precedence (resolved in `resolve_fee_config`)
/// 1. Global `FeeConfig.fee_enabled` — master kill-switch.
/// 2. `TokenFeeConfig.token` — rate/fixed/recipient overrides, but
///    `fee_enabled` is AND-ed with the global flag.
/// 3. Global `FeeConfig` fallback — used when no per-token override exists.
///
/// # Arguments
/// * `token`            – the token contract address this config applies to
/// * `lock_fee_rate`    – fee rate on lock in basis points (0 – 5 000)
/// * `release_fee_rate` – fee rate on release in basis points (0 – 5 000)
/// * `lock_fixed_fee` / `release_fixed_fee` – flat fees in token units (≥ 0)
/// * `fee_recipient`    – address that receives fees for this token
/// * `fee_enabled`      – whether fee collection is active for this token
///
/// # Errors
/// * `NotInitialized`  – contract not yet initialised
/// * `InvalidFeeRate`  – any rate is outside `[0, MAX_FEE_RATE]`
pub fn set_token_fee_config(
    env: Env,
    token: Address,
    lock_fee_rate: i128,
    release_fee_rate: i128,
    lock_fixed_fee: i128,
    release_fixed_fee: i128,
    fee_recipient: Address,
    fee_enabled: bool,
) -> Result<(), Error> {
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }
    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();

    if !(0..=MAX_FEE_RATE).contains(&lock_fee_rate) {
        return Err(Error::InvalidFeeRate);
    }
    if !(0..=MAX_FEE_RATE).contains(&release_fee_rate) {
        return Err(Error::InvalidFeeRate);
    }
    if lock_fixed_fee < 0 || release_fixed_fee < 0 {
        return Err(Error::InvalidAmount);
    }

    let config = TokenFeeConfig {
        lock_fee_rate,
        release_fee_rate,
        lock_fixed_fee,
        release_fixed_fee,
        fee_recipient,
        fee_enabled,
    };

    env.storage()
        .instance()
        .set(&DataKey::TokenFeeConfig(token), &config);

    Ok(())
}


/// Get the per-token fee configuration for `token`, if one has been set.
///
/// Returns `None` when no token-specific config exists; callers should
/// fall back to the global `FeeConfig` in that case.
pub fn get_token_fee_config(env: Env, token: Address) -> Option<TokenFeeConfig> {
    env.storage()
        .instance()
        .get(&DataKey::TokenFeeConfig(token))
}

