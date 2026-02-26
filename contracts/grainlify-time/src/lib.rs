#![no_std]
use soroban_sdk::Env;

/// Timestamp represented as seconds since Unix epoch.
pub type Timestamp = u64;

/// Duration represented in seconds.
pub type Duration = u64;

/// Block height represented as a sequence number.
pub type BlockHeight = u32;

/// Get the current ledger timestamp.
pub fn now(env: &Env) -> Timestamp {
    env.ledger().timestamp()
}

/// Create a duration from hours.
pub fn from_hours(hours: u64) -> Duration {
    hours.saturating_mul(3600)
}

/// Create a duration from minutes.
pub fn from_minutes(minutes: u64) -> Duration {
    minutes.saturating_mul(60)
}

/// Create a duration from days.
pub fn from_days(days: u64) -> Duration {
    days.saturating_mul(86400)
}

/// Get the current ledger sequence number.
pub fn current_block_height(env: &Env) -> BlockHeight {
    env.ledger().sequence()
}

/// Extension trait for Timestamp logic.
pub trait TimestampExt {
    fn add_duration(&self, duration: Duration) -> Self;
    fn duration_since(&self, earlier: Self) -> Option<Duration>;
}

impl TimestampExt for Timestamp {
    fn add_duration(&self, duration: Duration) -> Self {
        self.saturating_add(duration)
    }

    fn duration_since(&self, earlier: Self) -> Option<Duration> {
        if *self >= earlier {
            Some(self - earlier)
        } else {
            None
        }
    }
}
