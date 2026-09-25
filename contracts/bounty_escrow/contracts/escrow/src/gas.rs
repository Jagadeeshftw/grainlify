//! Gas budget configuration and advisory status queries.



use soroban_sdk::Env;
use crate::{gas_budget, rbac, DataKey, Error};

// ─────────────────────────────────────────────────────────────────
// Public entry points (dispatcher targets)
// ─────────────────────────────────────────────────────────────────


/// Configure per-operation gas budget caps for the contract instance.
///
/// All seven fields are written atomically. Callers that only want to
/// configure a subset should pass `OperationBudget::uncapped()` for the
/// operations they do not need to bound.
///
/// ## Authorisation
/// Requires the contract admin. The admin's signature is verified via
/// `require_auth` and the call aborts if authorisation fails.
///
/// ## Arguments
/// - `lock` — budget for [`crate::lock::lock_funds`] and `lock_funds_anonymous`.
/// - `release` — budget for [`crate::release::release_funds`].
/// - `refund` — budget for [`crate::refund::refund`] and admin-refund paths.
/// - `partial_release` — budget for [`crate::release::partial_release`].
/// - `batch_lock` — aggregate budget for [`crate::lock::batch_lock_funds`].
/// - `batch_release` — aggregate budget for [`crate::release::batch_release_funds`].
/// - `enforce` — when `true`, breaches in a testutils build cause the
///   transaction to revert with [`Error::GasBudgetExceeded`]. In
///   production WASM builds this flag is persisted but has **no runtime
///   effect** because `env.budget()` is unavailable to on-chain contracts.
///
/// # Errors
/// * [`Error::NotInitialized`] — `init` has not been called.
pub fn set_gas_budget(
    env: Env,
    lock: gas_budget::OperationBudget,
    release: gas_budget::OperationBudget,
    refund: gas_budget::OperationBudget,
    partial_release: gas_budget::OperationBudget,
    batch_lock: gas_budget::OperationBudget,
    batch_release: gas_budget::OperationBudget,
    enforce: bool,
) -> Result<(), Error> {
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(Error::NotInitialized)?;
    admin.require_auth();

    let config = gas_budget::GasBudgetConfig {
        lock,
        release,
        refund,
        partial_release,
        batch_lock,
        batch_release,
        enforce,
    };
    gas_budget::set_config(&env, config);
    Ok(())
}


/// Return the current per-operation gas budget configuration.
///
/// Returns the fully uncapped default if no configuration has been set.
///
/// ## ⚠ Production note
///
/// The returned caps are **advisory-only** on the live network. CPU and
/// memory measurement requires `env.budget()`, which is only available
/// under the `testutils` feature. Call
/// [`get_gas_budget_advisory_status`](get_gas_budget_advisory_status)
/// to obtain an explicit flag indicating whether caps are enforced at
/// runtime in the current build.
pub fn get_gas_budget(env: Env) -> gas_budget::GasBudgetConfig {
    gas_budget::get_config(&env)
}


/// Return the advisory enforcement status for the current gas budget config.
///
/// This is the **canonical query** for operators, dashboards, and auditors
/// to determine whether configured gas caps are being enforced at runtime.
///
/// ## Return value
///
/// Returns a [`gas_budget::GasBudgetAdvisoryStatus`] that includes:
///
/// - `caps_enforced_in_production` — always `false` in production WASM.
/// - `caps_configured` — `true` when any non-zero cap is set.
/// - `enforce_flag_set` — reflects `GasBudgetConfig::enforce`.
/// - `config` — full snapshot of current caps for reference.
///
/// ## Advisory event
///
/// When `caps_configured` is `true`, a `"gas_adv"` event is emitted into
/// the on-chain event stream. This event is observable by indexers and
/// monitoring systems without decoding contract storage, and explicitly
/// carries `caps_enforced_in_production = false` to flag the gap.
///
/// ## Security note
///
/// `caps_enforced_in_production` is a compile-time constant (`false`).
/// It is structurally impossible for a production WASM build to return
/// `true` — the `env.budget()` API is unconditionally absent outside
/// `testutils`. Auditors can use this function as definitive confirmation
/// that the deployment is operating in advisory-only mode.
///
/// See `docs/security/gas-budget-production-gap.md` for the full operator
/// guide and `docs/security/external-audit-checklist.md` for the auditor
/// checklist entry.
pub fn get_gas_budget_advisory_status(env: Env) -> gas_budget::GasBudgetAdvisoryStatus {
    let status = gas_budget::advisory_status(&env);
    gas_budget::emit_advisory_notice_if_needed(&env, &status);
    status
}

