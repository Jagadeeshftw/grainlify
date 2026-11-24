//! Mock hook contract for integration testing
//!
//! This contract provides a simple hook implementation for testing the hook system
//! without requiring a full insurance or reserve pool implementation.

use crate::hooks::{HookCall, HookEventType};
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, Env, String, Symbol, Vec,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum MockHookError {
    ConfiguredFailure = 1,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct MockHookStats {
    pub calls_received: u32,
    pub last_event: HookEventType,
    pub last_bounty_id: u64,
    pub last_amount: i128,
    pub last_timestamp: u64,
    pub should_fail: bool,
    pub fail_message: String,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct HookCallRecord {
    pub event_type: HookEventType,
    pub bounty_id: u64,
    pub amount: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MockHookKey {
    Stats,
    CallHistory,
    CallCount,
}

#[contract]
pub struct MockHook;

#[contractimpl]
impl MockHook {
    /// Handle hook call from escrow contract
    pub fn handle_hook(env: Env, call: HookCall) -> Result<(), MockHookError> {
        let key = Symbol::new(&env, "mock_stats");
        let mut stats: MockHookStats = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(MockHookStats {
                calls_received: 0,
                last_event: call.event_type,
                last_bounty_id: 0,
                last_amount: 0,
                last_timestamp: 0,
                should_fail: false,
                fail_message: String::from_str(&env, "Mock hook failure"),
            });

        // Update statistics
        stats.calls_received += 1;
        stats.last_event = call.event_type;
        stats.last_bounty_id = call.bounty_id;
        stats.last_amount = call.amount;
        stats.last_timestamp = call.timestamp;

        env.storage().persistent().set(&key, &stats);

        // Record call in history
        let history_key = Symbol::new(&env, "call_history");
        let mut history: Vec<HookCallRecord> = env
            .storage()
            .persistent()
            .get(&history_key)
            .unwrap_or(Vec::new(&env));

        history.push_back(HookCallRecord {
            event_type: call.event_type,
            bounty_id: call.bounty_id,
            amount: call.amount,
            timestamp: call.timestamp,
        });

        env.storage().persistent().set(&history_key, &history);

        // Return error if configured to fail
        if stats.should_fail {
            Err(MockHookError::ConfiguredFailure)
        } else {
            Ok(())
        }
    }

    /// Get current statistics
    pub fn get_stats(env: Env) -> MockHookStats {
        let key = Symbol::new(&env, "mock_stats");
        env.storage()
            .persistent()
            .get(&key)
            .unwrap_or(MockHookStats {
                calls_received: 0,
                last_event: HookEventType::DisputeOpened,
                last_bounty_id: 0,
                last_amount: 0,
                last_timestamp: 0,
                should_fail: false,
                fail_message: String::from_str(&env, ""),
            })
    }

    /// Get call history
    pub fn get_call_history(env: Env) -> Vec<HookCallRecord> {
        let history_key = Symbol::new(&env, "call_history");
        env.storage()
            .persistent()
            .get(&history_key)
            .unwrap_or(Vec::new(&env))
    }

    /// Configure mock to fail on next call
    pub fn set_fail(env: Env, should_fail: bool, message: String) {
        let key = Symbol::new(&env, "mock_stats");
        let mut stats = Self::get_stats(env.clone());
        stats.should_fail = should_fail;
        stats.fail_message = message;
        env.storage().persistent().set(&key, &stats);
    }

    /// Reset statistics and history
    pub fn reset(env: Env) {
        env.storage()
            .persistent()
            .remove(&Symbol::new(&env, "mock_stats"));
        env.storage()
            .persistent()
            .remove(&Symbol::new(&env, "call_history"));
    }

    /// Get number of calls received
    pub fn get_call_count(env: Env) -> u32 {
        Self::get_stats(env).calls_received
    }

    /// Check if last call matched expected values
    pub fn last_call_matches(
        env: Env,
        event_type: HookEventType,
        bounty_id: u64,
        amount: i128,
    ) -> bool {
        let stats = Self::get_stats(env);
        stats.last_event == event_type
            && stats.last_bounty_id == bounty_id
            && stats.last_amount == amount
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_hook_records_call() {
        let env = Env::default();
        let contract_id = env.register_contract(None, MockHook);
        let client = MockHookClient::new(&env, &contract_id);

        let call = HookCall {
            event_type: HookEventType::DisputeOpened,
            bounty_id: 123,
            amount: 1_000_000,
            timestamp: 1000,
        };

        client.handle_hook(&call);

        let stats = client.get_stats();
        assert_eq!(stats.calls_received, 1);
        assert_eq!(stats.last_event, HookEventType::DisputeOpened);
        assert_eq!(stats.last_bounty_id, 123);
        assert_eq!(stats.last_amount, 1_000_000);
    }

    #[test]
    fn test_mock_hook_failure() {
        let env = Env::default();
        let contract_id = env.register_contract(None, MockHook);
        let client = MockHookClient::new(&env, &contract_id);

        client.set_fail(&true, &String::from_str(&env, "Test failure"));

        let call = HookCall {
            event_type: HookEventType::DisputeOpened,
            bounty_id: 123,
            amount: 1_000_000,
            timestamp: 1000,
        };

        let result = client.try_handle_hook(&call);
        assert_eq!(result, Err(Ok(MockHookError::ConfiguredFailure)));
    }

    #[test]
    fn test_mock_hook_reset() {
        let env = Env::default();
        let contract_id = env.register_contract(None, MockHook);
        let client = MockHookClient::new(&env, &contract_id);

        let call = HookCall {
            event_type: HookEventType::DisputeOpened,
            bounty_id: 123,
            amount: 1_000_000,
            timestamp: 1000,
        };

        client.handle_hook(&call);
        assert_eq!(client.get_call_count(), 1);

        client.reset();
        assert_eq!(client.get_call_count(), 0);
    }
}
