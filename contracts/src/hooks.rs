//! Hook infrastructure for insurance and reserve pool integration
//!
//! This module provides a standardized hook system that allows external contracts
//! (insurance pools, reserve pools, custom handlers) to react to critical escrow events
//! without being part of the core escrow logic.
//!
//! ## Key Features
//!
//! - **Composable**: Hooks are optional and don't block core operations
//! - **Best-Effort**: Hook failures never affect core escrow flow
//! - **Standardized**: Well-defined event types enable multiple integrations
//! - **Auditable**: All hook calls are emitted as events
//! - **Configurable**: Admin can enable/disable hooks per contract
//!
//! ## Usage Example
//!
//! ```ignore
//! // Set up hook
//! contract.set_hook_address(&env, &admin, &Some(insurance_pool_address));
//!
//! // Later, when dispute opens:
//! hooks::call_dispute_opened_hook(
//!     &env,
//!     bounty_id,
//!     &disputer,
//!     amount,
//!     &reason,
//!     deadline,
//! );
//! ```

use soroban_sdk::{
    contracttype, symbol_short, vec, Address, Env, Error, IntoVal, String, Symbol, Val,
};

/// Hook event types
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum HookEventType {
    DisputeOpened = 0,    // Dispute initiated on a bounty
    LargeRelease = 1,     // Single or batch release exceeds threshold
    Refund = 2,           // Funds refunded to depositor
    DisputeResolved = 3,  // Dispute resolved (approved/denied)
}

/// Refund reason enumeration
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RefundReason {
    Expired = 0,     // Claim deadline passed
    Cancelled = 1,   // Cancelled by authorized user
    Disputed = 2,    // Dispute outcome resulted in refund
    ErrorRecovery = 3, // Error recovery mechanism
}

/// Hook call information
#[contracttype]
#[derive(Clone, Debug)]
pub struct HookCall {
    pub event_type: HookEventType,
    pub bounty_id: u64,
    pub amount: i128,
    pub timestamp: u64,
}

/// Dispute opened hook event
#[contracttype]
#[derive(Clone, Debug)]
pub struct DisputeOpenedHook {
    pub version: u32,
    pub bounty_id: u64,
    pub disputer: Address,
    pub amount: i128,
    pub reason: String,       // Dispute reason
    pub deadline: u64,        // Dispute resolution deadline
    pub timestamp: u64,
}

/// Large release hook event
#[contracttype]
#[derive(Clone, Debug)]
pub struct LargeReleaseHook {
    pub version: u32,
    pub bounty_id: u64,
    pub recipient: Address,
    pub amount: i128,
    pub release_count: u32,   // Number of recipients in batch
    pub threshold: i128,      // Threshold that triggered hook
    pub timestamp: u64,
}

/// Refund hook event
#[contracttype]
#[derive(Clone, Debug)]
pub struct RefundHook {
    pub version: u32,
    pub bounty_id: u64,
    pub refund_to: Address,
    pub amount: i128,
    pub reason: RefundReason,
    pub timestamp: u64,
}

/// Dispute resolved hook event
#[contracttype]
#[derive(Clone, Debug)]
pub struct DisputeResolvedHook {
    pub version: u32,
    pub bounty_id: u64,
    pub outcome: String,      // Approved, Denied, Partial
    pub amount_released: i128,
    pub amount_refunded: i128,
    pub timestamp: u64,
}

/// Hook call event (emitted regardless of hook success/failure)
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum HookCallStatus {
    Success = 0,
    Failed = 1,
    Panic = 2,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct HookCallEvent {
    pub bounty_id: u64,
    pub event_type: HookEventType,
    pub hook_address: Address,
    pub timestamp: u64,
    pub status: HookCallStatus,
    pub error_msg: Option<String>,
}

/// Storage key for hook address
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HookDataKey {
    HookAddress,
    LargeReleaseThreshold,
}

pub const HOOK_EVENT_VERSION: u32 = 1;

/// Get configured hook address
pub fn get_hook_address(env: &Env) -> Option<Address> {
    env.storage()
        .instance()
        .get(&HookDataKey::HookAddress)
}

/// Set hook address (for internal use by contracts)
pub fn set_hook_address_internal(env: &Env, hook_address: Option<Address>) {
    match hook_address {
        Some(addr) => {
            env.storage()
                .instance()
                .set(&HookDataKey::HookAddress, &addr);
            env.events().publish(
                (symbol_short!("hook_cfg"),),
                (symbol_short!("set"), addr),
            );
        }
        None => {
            env.storage().instance().remove(&HookDataKey::HookAddress);
            env.events()
                .publish((symbol_short!("hook_cfg"),), (symbol_short!("removed"),));
        }
    }
}

/// Get large release threshold (optional, for hook triggering)
pub fn get_large_release_threshold(env: &Env) -> Option<i128> {
    env.storage()
        .instance()
        .get(&HookDataKey::LargeReleaseThreshold)
}

/// Set large release threshold
pub fn set_large_release_threshold_internal(env: &Env, threshold: Option<i128>) {
    match threshold {
        Some(amount) => {
            env.storage()
                .instance()
                .set(&HookDataKey::LargeReleaseThreshold, &amount);
            env.events().publish(
                (symbol_short!("hook_cfg"),),
                (symbol_short!("threshold"), amount),
            );
        }
        None => {
            env.storage()
                .instance()
                .remove(&HookDataKey::LargeReleaseThreshold);
        }
    }
}

/// Call dispute opened hook (best-effort)
///
/// Emits a `DisputeOpenedHook` event and attempts to call the configured hook contract.
/// Hook failure or contract absence does not affect core flow.
pub fn call_dispute_opened_hook(
    env: &Env,
    bounty_id: u64,
    disputer: &Address,
    amount: i128,
    reason: &String,
    deadline: u64,
) {
    let hook_addr = match get_hook_address(env) {
        Some(addr) => addr,
        None => return, // No hook configured
    };

    let timestamp = env.ledger().timestamp();
    let event = DisputeOpenedHook {
        version: HOOK_EVENT_VERSION,
        bounty_id,
        disputer: disputer.clone(),
        amount,
        reason: reason.clone(),
        deadline,
        timestamp,
    };

    // Emit hook event for auditing
    env.events().publish(
        (symbol_short!("hook_evt"), symbol_short!("disp_open")),
        event.clone(),
    );

    // Attempt hook call (best-effort)
    _execute_hook_call(env, &hook_addr, HookEventType::DisputeOpened, bounty_id, amount);
}

/// Call large release hook (best-effort)
///
/// Triggered when a single or batch release exceeds the configured threshold.
/// Hook failure does not affect core flow.
pub fn call_large_release_hook(
    env: &Env,
    bounty_id: u64,
    recipient: &Address,
    amount: i128,
    release_count: u32,
) {
    // Check if this release qualifies as "large"
    let threshold = match get_large_release_threshold(env) {
        Some(t) => t,
        None => return, // No threshold configured
    };

    if amount < threshold {
        return; // Not a large release
    }

    let hook_addr = match get_hook_address(env) {
        Some(addr) => addr,
        None => return,
    };

    let timestamp = env.ledger().timestamp();
    let event = LargeReleaseHook {
        version: HOOK_EVENT_VERSION,
        bounty_id,
        recipient: recipient.clone(),
        amount,
        release_count,
        threshold,
        timestamp,
    };

    env.events().publish(
        (symbol_short!("hook_evt"), symbol_short!("lg_rel")),
        event.clone(),
    );

    _execute_hook_call(env, &hook_addr, HookEventType::LargeRelease, bounty_id, amount);
}

/// Call refund hook (best-effort)
pub fn call_refund_hook(
    env: &Env,
    bounty_id: u64,
    refund_to: &Address,
    amount: i128,
    reason: RefundReason,
) {
    let hook_addr = match get_hook_address(env) {
        Some(addr) => addr,
        None => return,
    };

    let timestamp = env.ledger().timestamp();
    let event = RefundHook {
        version: HOOK_EVENT_VERSION,
        bounty_id,
        refund_to: refund_to.clone(),
        amount,
        reason,
        timestamp,
    };

    env.events().publish(
        (symbol_short!("hook_evt"), symbol_short!("refund")),
        event.clone(),
    );

    _execute_hook_call(env, &hook_addr, HookEventType::Refund, bounty_id, amount);
}

/// Call dispute resolved hook (best-effort)
pub fn call_dispute_resolved_hook(
    env: &Env,
    bounty_id: u64,
    outcome: &String,
    amount_released: i128,
    amount_refunded: i128,
) {
    let hook_addr = match get_hook_address(env) {
        Some(addr) => addr,
        None => return,
    };

    let timestamp = env.ledger().timestamp();
    let event = DisputeResolvedHook {
        version: HOOK_EVENT_VERSION,
        bounty_id,
        outcome: outcome.clone(),
        amount_released,
        amount_refunded,
        timestamp,
    };

    env.events().publish(
        (symbol_short!("hook_evt"), symbol_short!("disp_res")),
        event.clone(),
    );

    _execute_hook_call(env, &hook_addr, HookEventType::DisputeResolved, bounty_id, 
                       amount_released + amount_refunded);
}

/// Internal helper - execute hook call with error handling
fn _execute_hook_call(
    env: &Env,
    hook_addr: &Address,
    event_type: HookEventType,
    bounty_id: u64,
    amount: i128,
) {
    let call = HookCall {
        event_type: event_type.clone(),
        bounty_id,
        amount,
        timestamp: env.ledger().timestamp(),
    };

    // Attempt to call the hook contract using try_invoke_contract so that
    // hook errors or panics/traps never revert the caller's transaction.
    let result = env.try_invoke_contract::<Val, Error>(
        hook_addr,
        &Symbol::new(env, "handle_hook"),
        vec![env, call.into_val(env)],
    );

    // Emit hook call result for auditing
    let (status, error_msg) = match result {
        Ok(Ok(_)) => (HookCallStatus::Success, None),
        Ok(Err(_)) => (
            HookCallStatus::Failed,
            Some(String::from_str(env, "Hook return conversion failed")),
        ),
        Err(Ok(err)) => {
            if err.is_type(soroban_sdk::xdr::ScErrorType::Contract) {
                (
                    HookCallStatus::Failed,
                    Some(String::from_str(env, "Hook returned error")),
                )
            } else {
                (
                    HookCallStatus::Panic,
                    Some(String::from_str(env, "Hook invocation panicked or aborted")),
                )
            }
        }
        Err(Err(_)) => (
            HookCallStatus::Panic,
            Some(String::from_str(env, "Hook invocation panicked or aborted")),
        ),
    };

    let call_event = HookCallEvent {
        bounty_id,
        event_type,
        hook_address: hook_addr.clone(),
        timestamp: env.ledger().timestamp(),
        status,
        error_msg,
    };

    env.events()
        .publish((symbol_short!("hook_call"),), call_event);
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{contract, contractimpl, testutils::Address as _};

    #[contract]
    struct DummyContract;

    #[contractimpl]
    impl DummyContract {}

    #[test]
    fn test_hook_address_storage() {
        let env = Env::default();
        let contract_id = env.register_contract(None, DummyContract);
        let addr = Address::generate(&env);

        env.as_contract(&contract_id, || {
            // Initially none
            assert_eq!(get_hook_address(&env), None);

            // Set
            set_hook_address_internal(&env, Some(addr.clone()));
            assert_eq!(get_hook_address(&env), Some(addr.clone()));

            // Remove
            set_hook_address_internal(&env, None);
            assert_eq!(get_hook_address(&env), None);
        });
    }

    #[test]
    fn test_threshold_storage() {
        let env = Env::default();
        let contract_id = env.register_contract(None, DummyContract);

        env.as_contract(&contract_id, || {
            // Initially none
            assert_eq!(get_large_release_threshold(&env), None);

            // Set
            set_large_release_threshold_internal(&env, Some(1_000_000));
            assert_eq!(get_large_release_threshold(&env), Some(1_000_000));

            // Remove
            set_large_release_threshold_internal(&env, None);
            assert_eq!(get_large_release_threshold(&env), None);
        });
    }
}
