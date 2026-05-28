#![allow(dead_code)]
// contracts/program-escrow/src/threshold_monitor.rs
//
// Threshold Monitor Module
//
// Implements automatic circuit breaker triggers based on configurable thresholds
// for failure rates and token outflow volumes. Monitors operations in sliding
// time windows and opens the circuit breaker when abnormal patterns are detected.

use soroban_sdk::{contracttype, symbol_short, Address, Env, Symbol};

// ─────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────

/// Configuration for threshold-based circuit breaking
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThresholdConfig {
    /// Maximum failures allowed per time window
    pub failure_rate_threshold: u32,
    /// Maximum outflow amount per time window
    pub outflow_volume_threshold: i128,
    /// Maximum amount for a single payout transaction
    pub max_single_payout: i128,
    /// Time window duration in seconds
    pub time_window_secs: u64,
    /// Minimum cooldown period before reopening (seconds)
    pub cooldown_period_secs: u64,
    /// Backoff multiplier for repeated breaches
    pub cooldown_multiplier: u32,
}

impl ThresholdConfig {
    /// Default configuration with conservative thresholds
    pub fn default() -> Self {
        ThresholdConfig {
            failure_rate_threshold: 10,
            outflow_volume_threshold: 5_000_000_0000000, // 5M tokens (7 decimals)
            max_single_payout: 500_000_0000000,          // 500K tokens
            time_window_secs: 600,                       // 10 minutes
            cooldown_period_secs: 300,                   // 5 minutes
            cooldown_multiplier: 2,
        }
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.failure_rate_threshold == 0 || self.failure_rate_threshold > 1000 {
            return Err("Failure threshold must be between 1 and 1000");
        }
        if self.outflow_volume_threshold <= 0 {
            return Err("Outflow threshold must be greater than zero");
        }

        if self.max_single_payout <= 0 {
            return Err("Max single payout must be greater than zero");
        }
        if self.time_window_secs < 10 || self.time_window_secs > 86400 {
            return Err("Time window must be between 10 and 86400 seconds");
        }
        if self.cooldown_period_secs < 60 || self.cooldown_period_secs > 3600 {
            return Err("Cooldown period must be between 60 and 3600 seconds");
        }
        Ok(())
    }
}

/// Current metrics for a time window
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowMetrics {
    /// Window start timestamp
    pub window_start: u64,
    /// Failures in current window
    pub failure_count: u32,
    /// Successes in current window
    pub success_count: u32,
    /// Total outflow in current window
    pub total_outflow: i128,
    /// Largest single outflow in window
    pub max_single_outflow: i128,
    /// Number of times thresholds breached
    pub breach_count: u32,
}

impl WindowMetrics {
    pub fn new(window_start: u64) -> Self {
        WindowMetrics {
            window_start,
            failure_count: 0,
            success_count: 0,
            total_outflow: 0,
            max_single_outflow: 0,
            breach_count: 0,
        }
    }
}

/// Threshold breach information
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThresholdBreach {
    /// Type of metric that breached ("failure" or "outflow")
    pub metric_type: Symbol,
    /// Configured threshold value
    pub threshold_value: i128,
    /// Actual value that breached
    pub actual_value: i128,
    /// When breach occurred
    pub timestamp: u64,
    /// Total breaches in this window
    pub breach_count: u32,
}

/// Storage keys for threshold monitoring
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ThresholdKey {
    Config,
    CurrentMetrics,
    PreviousMetrics,
    LastCooldownEnd,
    CooldownMultiplier,
}

fn persistent_key(key: ThresholdKey) -> (Symbol, ThresholdKey) {
    (symbol_short!("th"), key)
}

// ─────────────────────────────────────────────────────────
// Error codes
// ─────────────────────────────────────────────────────────

pub const ERR_THRESHOLD_BREACHED: u32 = 2001;
pub const ERR_INVALID_THRESHOLD_CONFIG: u32 = 2002;
pub const ERR_COOLDOWN_ACTIVE: u32 = 2003;
pub const ERR_WINDOW_NOT_EXPIRED: u32 = 2004;

// ─────────────────────────────────────────────────────────
// Configuration Management
// ─────────────────────────────────────────────────────────

/// Initialize threshold monitoring with default configuration
pub fn init_threshold_monitor(env: &Env) {
    let config = ThresholdConfig::default();
    env.storage()
        .persistent()
        .set(&persistent_key(ThresholdKey::Config), &config);

    let metrics = WindowMetrics::new(env.ledger().timestamp());
    env.storage()
        .persistent()
        .set(&persistent_key(ThresholdKey::CurrentMetrics), &metrics);

    env.storage()
        .persistent()
        .set(&persistent_key(ThresholdKey::CooldownMultiplier), &1u32);

    emit_config_event(env, symbol_short!("th_init"), &config);
}

/// Update threshold configuration (admin only - caller must enforce auth)
pub fn set_threshold_config(env: &Env, config: ThresholdConfig) -> Result<(), u32> {
    // Validate configuration
    config
        .validate()
        .map_err(|_| ERR_INVALID_THRESHOLD_CONFIG)?;

    // Get previous config for event
    let prev_config = get_threshold_config(env);

    // Store new configuration
    env.storage()
        .persistent()
        .set(&persistent_key(ThresholdKey::Config), &config);

    // Emit configuration update event
    emit_config_update_event(env, &prev_config, &config);

    Ok(())
}

/// Get current threshold configuration
pub fn get_threshold_config(env: &Env) -> ThresholdConfig {
    env.storage()
        .persistent()
        .get(&persistent_key(ThresholdKey::Config))
        .unwrap_or(ThresholdConfig::default())
}

// ─────────────────────────────────────────────────────────
// Metrics Tracking and Window Management
// ─────────────────────────────────────────────────────────

/// Record a successful operation
pub fn record_operation_success(env: &Env) {
    rotate_window_if_needed(env);

    let mut metrics = get_current_metrics(env);
    metrics.success_count += 1;

    env.storage()
        .persistent()
        .set(&persistent_key(ThresholdKey::CurrentMetrics), &metrics);
}

/// Record a failed operation
pub fn record_operation_failure(env: &Env) {
    rotate_window_if_needed(env);

    let mut metrics = get_current_metrics(env);
    metrics.failure_count += 1;

    env.storage()
        .persistent()
        .set(&persistent_key(ThresholdKey::CurrentMetrics), &metrics);
}

/// Record an outflow transaction
pub fn record_outflow(env: &Env, amount: i128) {
    rotate_window_if_needed(env);

    let mut metrics = get_current_metrics(env);
    metrics.total_outflow = metrics.total_outflow.saturating_add(amount);

    if amount > metrics.max_single_outflow {
        metrics.max_single_outflow = amount;
    }

    env.storage()
        .persistent()
        .set(&persistent_key(ThresholdKey::CurrentMetrics), &metrics);
}

/// Get current window metrics
pub fn get_current_metrics(env: &Env) -> WindowMetrics {
    env.storage()
        .persistent()
        .get(&persistent_key(ThresholdKey::CurrentMetrics))
        .unwrap_or_else(|| WindowMetrics::new(env.ledger().timestamp()))
}

/// Check if window has expired and rotate if needed
fn rotate_window_if_needed(env: &Env) {
    let config = get_threshold_config(env);
    let metrics = get_current_metrics(env);
    let now = env.ledger().timestamp();

    let window_end = metrics.window_start + config.time_window_secs;

    if now >= window_end {
        // Archive current metrics
        env.storage()
            .persistent()
            .set(&persistent_key(ThresholdKey::PreviousMetrics), &metrics);

        // Emit window rotation event
        emit_window_rotation_event(env, &metrics);

        // Create new window
        let new_metrics = WindowMetrics::new(now);
        env.storage()
            .persistent()
            .set(&persistent_key(ThresholdKey::CurrentMetrics), &new_metrics);
    }
}

// ─────────────────────────────────────────────────────────
// Threshold Checking
// ─────────────────────────────────────────────────────────

/// Check if any thresholds are breached (call before operations)
pub fn check_thresholds(env: &Env) -> Result<(), ThresholdBreach> {
    rotate_window_if_needed(env);

    let config = get_threshold_config(env);
    let metrics = get_current_metrics(env);
    let now = env.ledger().timestamp();

    // Check failure rate threshold
    if metrics.failure_count >= config.failure_rate_threshold {
        let breach = ThresholdBreach {
            metric_type: symbol_short!("failure"),
            threshold_value: config.failure_rate_threshold as i128,
            actual_value: metrics.failure_count as i128,
            timestamp: now,
            breach_count: metrics.breach_count + 1,
        };
        return Err(breach);
    }

    // Check outflow volume threshold
    if metrics.total_outflow >= config.outflow_volume_threshold {
        let breach = ThresholdBreach {
            metric_type: symbol_short!("outflow"),
            threshold_value: config.outflow_volume_threshold,
            actual_value: metrics.total_outflow,
            timestamp: now,
            breach_count: metrics.breach_count + 1,
        };
        return Err(breach);
    }

    // Check max single payout threshold
    if metrics.max_single_outflow >= config.max_single_payout {
        let breach = ThresholdBreach {
            metric_type: symbol_short!("single"),
            threshold_value: config.max_single_payout,
            actual_value: metrics.max_single_outflow,
            timestamp: now,
            breach_count: metrics.breach_count + 1,
        };
        return Err(breach);
    }

    Ok(())
}

/// Check a single payout amount before execution
pub fn check_single_payout_threshold(env: &Env, amount: i128) -> Result<(), ThresholdBreach> {
    let config = get_threshold_config(env);
    let now = env.ledger().timestamp();
    let metrics = get_current_metrics(env);

    if amount >= config.max_single_payout {
        let breach = ThresholdBreach {
            metric_type: symbol_short!("single"),
            threshold_value: config.max_single_payout,
            actual_value: amount,
            timestamp: now,
            breach_count: metrics.breach_count + 1,
        };
        return Err(breach);
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────
// Cooldown and Anti-Flapping Logic
// ─────────────────────────────────────────────────────────

/// Check if cooldown period is active
pub fn is_cooldown_active(env: &Env) -> bool {
    let last_cooldown_end: u64 = env
        .storage()
        .persistent()
        .get(&persistent_key(ThresholdKey::LastCooldownEnd))
        .unwrap_or(0);

    let now = env.ledger().timestamp();
    now < last_cooldown_end
}

/// Get current cooldown multiplier
pub fn get_cooldown_multiplier(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&persistent_key(ThresholdKey::CooldownMultiplier))
        .unwrap_or(1)
}

/// Apply cooldown after circuit opens
pub fn apply_cooldown(env: &Env) {
    let config = get_threshold_config(env);
    let multiplier = get_cooldown_multiplier(env);
    let now = env.ledger().timestamp();

    let cooldown_duration = config.cooldown_period_secs * (multiplier as u64);
    let cooldown_end = now + cooldown_duration;

    env.storage()
        .persistent()
        .set(&persistent_key(ThresholdKey::LastCooldownEnd), &cooldown_end);
}

/// Increase cooldown multiplier for repeated breaches
pub fn increase_cooldown_multiplier(env: &Env) {
    let config = get_threshold_config(env);
    let current_multiplier = get_cooldown_multiplier(env);
    let new_multiplier = current_multiplier * config.cooldown_multiplier;

    env.storage()
        .persistent()
        .set(&persistent_key(ThresholdKey::CooldownMultiplier), &new_multiplier);
}

/// Reset cooldown multiplier after stability period
pub fn reset_cooldown_multiplier(env: &Env) {
    env.storage()
        .persistent()
        .set(&persistent_key(ThresholdKey::CooldownMultiplier), &1u32);
}

// ─────────────────────────────────────────────────────────
// Manual Metrics Reset
// ─────────────────────────────────────────────────────────

/// Manually reset metrics (admin only - caller must enforce auth)
pub fn reset_metrics(env: &Env, admin: &Address) {
    let now = env.ledger().timestamp();

    // Create new window starting now
    let new_metrics = WindowMetrics::new(now);
    env.storage()
        .persistent()
        .set(&persistent_key(ThresholdKey::CurrentMetrics), &new_metrics);

    // Emit reset event
    emit_metrics_reset_event(env, admin, now);
}

// ─────────────────────────────────────────────────────────
// Event Emission
// ─────────────────────────────────────────────────────────

/// Emit threshold breach event
pub fn emit_threshold_breach_event(env: &Env, breach: &ThresholdBreach) {
    env.events().publish(
        (symbol_short!("th_breach"), breach.metric_type.clone()),
        (
            breach.threshold_value,
            breach.actual_value,
            breach.timestamp,
            breach.breach_count,
        ),
    );
}

/// Emit configuration initialization event
fn emit_config_event(env: &Env, event_type: Symbol, config: &ThresholdConfig) {
    env.events().publish(
        (symbol_short!("th_cfg"), event_type),
        (
            config.failure_rate_threshold,
            config.outflow_volume_threshold,
            config.max_single_payout,
            config.time_window_secs,
        ),
    );
}

/// Emit configuration update event
fn emit_config_update_event(env: &Env, prev: &ThresholdConfig, new: &ThresholdConfig) {
    env.events().publish(
        (symbol_short!("th_cfg"), symbol_short!("update")),
        (
            prev.failure_rate_threshold,
            new.failure_rate_threshold,
            prev.outflow_volume_threshold,
            new.outflow_volume_threshold,
        ),
    );
}

/// Emit window rotation event
fn emit_window_rotation_event(env: &Env, metrics: &WindowMetrics) {
    env.events().publish(
        (symbol_short!("th_win"), symbol_short!("rotate")),
        (
            metrics.window_start,
            metrics.failure_count,
            metrics.success_count,
            metrics.total_outflow,
        ),
    );
}

/// Emit metrics reset event
fn emit_metrics_reset_event(env: &Env, admin: &Address, timestamp: u64) {
    env.events()
        .publish((symbol_short!("th_reset"),), (admin.clone(), timestamp));
}
