//! Generic delay-then-execute queue for sensitive admin actions.
//!
//! Direct admin setters remain available while the timelock is disabled so
//! existing deployments keep working. Once `configure_timelock(..., true)` is
//! called, those setters reject direct calls and changes must be proposed,
//! observed publicly, and executed after the configured delay.

use crate::{
    admin, events, fee, pause_freeze, release, DataKey, Error, TreasuryDestination,
    DEFAULT_TIMELOCK_DELAY, MAX_TIMELOCK_DELAY, MIN_TIMELOCK_DELAY,
};
use soroban_sdk::{contracttype, Address, Env, String, Vec};

/// Sensitive operations that can be queued behind the timelock.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionType {
    ChangeAdmin,
    ChangeFeeRecipient,
    UpdateFeeConfig,
    SetTreasuryDistributions,
    SetRouter,
    EnableKillSwitch,
    DisableKillSwitch,
    SetMaintenanceMode,
    SetPaused,
}

/// Fee fields accepted by [`ActionType::UpdateFeeConfig`].
///
/// `None` leaves the corresponding field unchanged, matching the semantics of
/// the direct `update_fee_config` entrypoint.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfigUpdate {
    pub lock_fee_rate: Option<i128>,
    pub release_fee_rate: Option<i128>,
    pub lock_fixed_fee: Option<i128>,
    pub release_fixed_fee: Option<i128>,
    pub fee_recipient: Option<Address>,
    pub fee_enabled: Option<bool>,
}

/// Payload for a queued action. The variant must match the proposed
/// [`ActionType`] or `propose_admin_action` returns [`Error::InvalidPayload`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActionPayload {
    ChangeAdmin(Address),
    ChangeFeeRecipient(Address),
    UpdateFeeConfig(FeeConfigUpdate),
    SetTreasuryDistributions(Vec<TreasuryDestination>, bool),
    SetRouter(Address),
    EnableKillSwitch,
    DisableKillSwitch,
    SetMaintenanceMode(bool, Option<String>),
    SetPaused(Option<bool>, Option<bool>, Option<bool>),
}

/// Lifecycle of a queued action.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionStatus {
    Pending,
    Executed,
    Cancelled,
}

/// Admin-action timelock settings.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimelockConfig {
    pub delay: u64,
    pub is_enabled: bool,
}

/// Publicly queryable record for a proposed admin action.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingAction {
    pub action_id: u64,
    pub action_type: ActionType,
    pub payload: ActionPayload,
    pub proposed_by: Address,
    pub proposed_at: u64,
    pub execute_after: u64,
    pub status: ActionStatus,
}

fn require_admin(env: &Env) -> Result<Address, Error> {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(Error::NotInitialized)
}

/// Rejects direct admin calls while the generic timelock is enabled.
pub(crate) fn ensure_direct_admin_call_allowed(env: &Env) -> Result<(), Error> {
    if get_config(env).is_enabled {
        return Err(Error::TimelockEnabled);
    }
    Ok(())
}

/// Configure the delay and activation flag. Admin-only; never timelocked.
pub fn configure_timelock(env: Env, delay: u64, is_enabled: bool) -> Result<(), Error> {
    let admin = require_admin(&env)?;
    admin.require_auth();
    validate_delay(delay)?;

    env.storage().instance().set(
        &DataKey::TimelockConfig,
        &TimelockConfig { delay, is_enabled },
    );

    events::emit_timelock_configured(
        &env,
        events::TimelockConfigured {
            version: events::EVENT_VERSION_V2,
            delay,
            is_enabled,
            configured_by: admin,
            timestamp: env.ledger().timestamp(),
            correlation_id: None,
        },
    );
    Ok(())
}

/// Returns the current configuration, defaulting to disabled with the
/// documented 24-hour delay for contracts upgraded before this feature.
pub fn get_config(env: &Env) -> TimelockConfig {
    env.storage()
        .instance()
        .get(&DataKey::TimelockConfig)
        .unwrap_or(TimelockConfig {
            delay: DEFAULT_TIMELOCK_DELAY,
            is_enabled: false,
        })
}

/// Propose an admin action. Returns `0` when the timelock is disabled and the
/// action was applied immediately; otherwise returns the new positive action id.
pub fn propose_admin_action(
    env: Env,
    action_type: ActionType,
    payload: ActionPayload,
) -> Result<u64, Error> {
    let admin = require_admin(&env)?;
    admin.require_auth();
    validate_payload(&action_type, &payload)?;

    let config = get_config(&env);
    if !config.is_enabled {
        apply_action(&env, &payload)?;
        return Ok(0);
    }

    let action_id = env
        .storage()
        .instance()
        .get::<DataKey, u64>(&DataKey::ActionCounter)
        .unwrap_or(0)
        .checked_add(1)
        .ok_or(Error::ActionNotFound)?;

    let proposed_at = env.ledger().timestamp();
    let execute_after = proposed_at.saturating_add(config.delay);
    let action = PendingAction {
        action_id,
        action_type,
        payload,
        proposed_by: admin.clone(),
        proposed_at,
        execute_after,
        status: ActionStatus::Pending,
    };

    env.storage()
        .persistent()
        .set(&DataKey::PendingAction(action_id), &action);
    env.storage()
        .instance()
        .set(&DataKey::ActionCounter, &action_id);

    events::emit_admin_action_proposed(
        &env,
        events::AdminActionProposed {
            version: events::EVENT_VERSION_V2,
            action_type: action.action_type,
            execute_after,
            proposed_by: admin,
            timestamp: proposed_at,
            correlation_id: None,
        },
    );
    Ok(action_id)
}

/// Execute a pending action once `execute_after` has been reached.
///
/// Execution is permissionless; the action's authorization was captured when
/// it was proposed. The exact boundary is inclusive: `now == execute_after`
/// succeeds, `now == execute_after - 1` returns [`Error::AdminActionTimelockActive`].
pub fn execute_after_delay(env: Env, action_id: u64) -> Result<(), Error> {
    let mut action = get_action(&env, action_id)?;
    match action.status {
        ActionStatus::Executed => return Err(Error::ActionAlreadyExecuted),
        ActionStatus::Cancelled => return Err(Error::ActionAlreadyCancelled),
        ActionStatus::Pending => {}
    }

    let now = env.ledger().timestamp();
    if now < action.execute_after {
        return Err(Error::AdminActionTimelockActive);
    }

    apply_action(&env, &action.payload)?;
    action.status = ActionStatus::Executed;
    env.storage()
        .persistent()
        .set(&DataKey::PendingAction(action_id), &action);

    events::emit_admin_action_executed(
        &env,
        events::AdminActionExecuted {
            version: events::EVENT_VERSION_V2,
            action_type: action.action_type,
            executed_by: env.current_contract_address(),
            executed_at: now,
            correlation_id: None,
        },
    );
    Ok(())
}

/// Cancel a pending action. Admin-only; cancelling an already-cancelled action
/// is a no-op so indexers can safely retry.
pub fn cancel_admin_action(env: Env, action_id: u64) -> Result<(), Error> {
    let admin = require_admin(&env)?;
    admin.require_auth();

    let mut action = get_action(&env, action_id)?;
    match action.status {
        ActionStatus::Executed => return Err(Error::ActionAlreadyExecuted),
        ActionStatus::Cancelled => return Ok(()),
        ActionStatus::Pending => {}
    }

    action.status = ActionStatus::Cancelled;
    env.storage()
        .persistent()
        .set(&DataKey::PendingAction(action_id), &action);

    events::emit_admin_action_cancelled(
        &env,
        events::AdminActionCancelled {
            version: events::EVENT_VERSION_V2,
            action_type: action.action_type,
            cancelled_by: admin,
            cancelled_at: env.ledger().timestamp(),
            correlation_id: None,
        },
    );
    Ok(())
}

/// Pending actions in proposal (and therefore timestamp) order.
pub fn get_pending_actions(env: &Env) -> Vec<PendingAction> {
    let counter = env
        .storage()
        .instance()
        .get::<DataKey, u64>(&DataKey::ActionCounter)
        .unwrap_or(0);
    let mut pending = Vec::new(env);
    for action_id in 1..=counter {
        if let Some(action) = env
            .storage()
            .persistent()
            .get::<DataKey, PendingAction>(&DataKey::PendingAction(action_id))
        {
            if action.status == ActionStatus::Pending {
                pending.push_back(action);
            }
        }
    }
    pending
}

/// Fetch a queued action by id, including executed and cancelled records.
pub fn get_action(env: &Env, action_id: u64) -> Result<PendingAction, Error> {
    env.storage()
        .persistent()
        .get(&DataKey::PendingAction(action_id))
        .ok_or(Error::ActionNotFound)
}

fn validate_delay(delay: u64) -> Result<(), Error> {
    if delay < MIN_TIMELOCK_DELAY {
        return Err(Error::DelayBelowMinimum);
    }
    if delay > MAX_TIMELOCK_DELAY {
        return Err(Error::DelayAboveMaximum);
    }
    Ok(())
}

fn validate_payload(action_type: &ActionType, payload: &ActionPayload) -> Result<(), Error> {
    let matches = matches!(
        (action_type, payload),
        (ActionType::ChangeAdmin, ActionPayload::ChangeAdmin(_))
            | (
                ActionType::ChangeFeeRecipient,
                ActionPayload::ChangeFeeRecipient(_)
            )
            | (
                ActionType::UpdateFeeConfig,
                ActionPayload::UpdateFeeConfig(_)
            )
            | (
                ActionType::SetTreasuryDistributions,
                ActionPayload::SetTreasuryDistributions(_, _)
            )
            | (ActionType::SetRouter, ActionPayload::SetRouter(_))
            | (
                ActionType::EnableKillSwitch,
                ActionPayload::EnableKillSwitch
            )
            | (
                ActionType::DisableKillSwitch,
                ActionPayload::DisableKillSwitch
            )
            | (
                ActionType::SetMaintenanceMode,
                ActionPayload::SetMaintenanceMode(_, _)
            )
            | (ActionType::SetPaused, ActionPayload::SetPaused(_, _, _))
    );
    if matches {
        Ok(())
    } else {
        Err(Error::InvalidPayload)
    }
}

fn apply_action(env: &Env, payload: &ActionPayload) -> Result<(), Error> {
    match payload {
        ActionPayload::ChangeAdmin(new_admin) => admin::set_admin_internal(env, new_admin),
        ActionPayload::ChangeFeeRecipient(recipient) => {
            fee::set_fee_recipient_internal(env, recipient)
        }
        ActionPayload::UpdateFeeConfig(update) => fee::update_fee_config_internal(
            env,
            update.lock_fee_rate,
            update.release_fee_rate,
            update.lock_fixed_fee,
            update.release_fixed_fee,
            update.fee_recipient.clone(),
            update.fee_enabled,
        ),
        ActionPayload::SetTreasuryDistributions(destinations, enabled) => {
            fee::set_treasury_distributions_internal(env, destinations.clone(), *enabled)
        }
        ActionPayload::SetRouter(router) => {
            release::set_router_internal(env, router);
            Ok(())
        }
        ActionPayload::EnableKillSwitch => pause_freeze::set_deprecated_internal(env, true, None),
        ActionPayload::DisableKillSwitch => pause_freeze::set_deprecated_internal(env, false, None),
        ActionPayload::SetMaintenanceMode(enabled, reason) => {
            pause_freeze::set_maintenance_mode_internal(env, *enabled, reason)
        }
        ActionPayload::SetPaused(lock, release_paused, refund) => {
            pause_freeze::set_paused_internal(env, *lock, *release_paused, *refund, &None)
        }
    }
}
