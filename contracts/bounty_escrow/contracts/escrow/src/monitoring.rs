//! Contract operation monitoring, performance tracking, and health metrics.
use soroban_sdk::{contracttype, symbol_short, Address, Env, String, Symbol};

// Storage keys
#[allow(dead_code)]
const OPERATION_COUNT: &str = "op_count";
#[allow(dead_code)]
const USER_COUNT: &str = "usr_count";
#[allow(dead_code)]
const ERROR_COUNT: &str = "err_count";

// Event: Operation metric
#[contracttype]
#[derive(Clone, Debug)]
pub struct OperationMetric {
    pub operation: Symbol,
    pub caller: Address,
    pub timestamp: u64,
    pub success: bool,
}

// Event: Performance metric
#[contracttype]
#[derive(Clone, Debug)]
pub struct PerformanceMetric {
    pub function: Symbol,
    pub duration: u64,
    pub timestamp: u64,
}

// Data: Health status
#[contracttype]
#[derive(Clone, Debug)]
pub struct HealthStatus {
    pub is_healthy: bool,
    pub last_operation: u64,
    pub total_operations: u64,
    pub contract_version: String,
}
// Data: Analytics
#[contracttype]
#[derive(Clone, Debug)]
pub struct Analytics {
    pub operation_count: u64,
    pub unique_users: u64,
    pub error_count: u64,
    pub error_rate: u32,
}

// Data: State snapshot
#[contracttype]
#[derive(Clone, Debug)]
pub struct StateSnapshot {
    pub timestamp: u64,
    pub total_operations: u64,
    pub total_users: u64,
    pub total_errors: u64,
}

// Data: Performance stats
#[contracttype]
#[derive(Clone, Debug)]
pub struct PerformanceStats {
    pub function_name: Symbol,
    pub call_count: u64,
    pub total_time: u64,
    pub avg_time: u64,
    pub last_called: u64,
}

// Track operation
#[allow(dead_code)]
pub fn track_operation(env: &Env, operation: Symbol, caller: Address, success: bool) {
    let key = Symbol::new(env, OPERATION_COUNT);
    let count: u64 = env.storage().persistent().get(&key).unwrap_or(0);
    env.storage().persistent().set(&key, &(count + 1));

    if !success {
        let err_key = Symbol::new(env, ERROR_COUNT);
        let err_count: u64 = env.storage().persistent().get(&err_key).unwrap_or(0);
        env.storage().persistent().set(&err_key, &(err_count + 1));
    }

    env.events().publish(
        (symbol_short!("metric"), symbol_short!("op")),
        OperationMetric {
            operation,
            caller,
            timestamp: env.ledger().timestamp(),
            success,
        },
    );
}

// Track performance
#[allow(dead_code)]
pub fn emit_performance(env: &Env, function: Symbol, duration: u64) {
    let count_key = (Symbol::new(env, "perf_cnt"), function.clone());
    let time_key = (Symbol::new(env, "perf_time"), function.clone());

    let count: u64 = env.storage().persistent().get(&count_key).unwrap_or(0);
    let total: u64 = env.storage().persistent().get(&time_key).unwrap_or(0);

    env.storage().persistent().set(&count_key, &(count + 1));
    env.storage()
        .persistent()
        .set(&time_key, &(total + duration));

    env.events().publish(
        (symbol_short!("metric"), symbol_short!("perf")),
        PerformanceMetric {
            function,
            duration,
            timestamp: env.ledger().timestamp(),
        },
    );
}

// Health check
#[allow(dead_code)]
pub fn health_check(env: &Env) -> HealthStatus {
    let key = Symbol::new(env, OPERATION_COUNT);
    let ops: u64 = env.storage().persistent().get(&key).unwrap_or(0);

    HealthStatus {
        is_healthy: true,
        last_operation: env.ledger().timestamp(),
        total_operations: ops,
        contract_version: String::from_str(env, "1.0.0"),
    }
}

// Get analytics
#[allow(dead_code)]
pub fn get_analytics(env: &Env) -> Analytics {
    let op_key = Symbol::new(env, OPERATION_COUNT);
    let usr_key = Symbol::new(env, USER_COUNT);
    let err_key = Symbol::new(env, ERROR_COUNT);

    let ops: u64 = env.storage().persistent().get(&op_key).unwrap_or(0);
    let users: u64 = env.storage().persistent().get(&usr_key).unwrap_or(0);
    let errors: u64 = env.storage().persistent().get(&err_key).unwrap_or(0);

    let error_rate = if ops > 0 {
        ((errors as u128 * 10000) / ops as u128) as u32
    } else {
        0
    };

    Analytics {
        operation_count: ops,
        unique_users: users,
        error_count: errors,
        error_rate,
    }
}

// Get state snapshot
#[allow(dead_code)]
pub fn get_state_snapshot(env: &Env) -> StateSnapshot {
    let op_key = Symbol::new(env, OPERATION_COUNT);
    let usr_key = Symbol::new(env, USER_COUNT);
    let err_key = Symbol::new(env, ERROR_COUNT);

    StateSnapshot {
        timestamp: env.ledger().timestamp(),
        total_operations: env.storage().persistent().get(&op_key).unwrap_or(0),
        total_users: env.storage().persistent().get(&usr_key).unwrap_or(0),
        total_errors: env.storage().persistent().get(&err_key).unwrap_or(0),
    }
}

// Get performance stats
#[allow(dead_code)]
pub fn get_performance_stats(env: &Env, function_name: Symbol) -> PerformanceStats {
    let count_key = (Symbol::new(env, "perf_cnt"), function_name.clone());
    let time_key = (Symbol::new(env, "perf_time"), function_name.clone());
    let last_key = (Symbol::new(env, "perf_last"), function_name.clone());

    let count: u64 = env.storage().persistent().get(&count_key).unwrap_or(0);
    let total: u64 = env.storage().persistent().get(&time_key).unwrap_or(0);
    let last: u64 = env.storage().persistent().get(&last_key).unwrap_or(0);

    let avg = total.checked_div(count).unwrap_or(0);

    PerformanceStats {
        function_name,
        call_count: count,
        total_time: total,
        avg_time: avg,
        last_called: last,
    }
}
