#![no_std]
use soroban_sdk::{contracttype, Env};

/// A strict wrapper for Unix timestamps in seconds.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct Timestamp(pub u64);

/// A strict wrapper for time durations in seconds.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct Duration(pub u64);

impl Timestamp {
    /// Returns the current ledger timestamp wrapped in a `Timestamp`.
    pub fn now(env: &Env) -> Self {
        Self(env.ledger().timestamp())
    }

    /// Adds a `Duration` to this `Timestamp`, returning a new `Timestamp`.
    /// Panics on overflow.
    pub fn add_duration(&self, duration: Duration) -> Self {
        Self(self.0.checked_add(duration.0).expect("Timestamp overflow"))
    }

    /// Subtracts a `Duration` from this `Timestamp`, returning a new `Timestamp`.
    /// Panics on underflow.
    pub fn sub_duration(&self, duration: Duration) -> Self {
        Self(self.0.checked_sub(duration.0).expect("Timestamp underflow"))
    }

    /// Calculates the duration between two timestamps.
    /// Returns `Some(Duration)` if `other` is before or equal to `self`.
    pub fn duration_since(&self, other: Timestamp) -> Option<Duration> {
        self.0.checked_sub(other.0).map(Duration)
    }

    /// Returns the inner u64 value.
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl Duration {
    /// Returns the inner u64 value.
    pub fn as_u64(&self) -> u64 {
        self.0
    }

    /// Creates a duration from seconds.
    pub fn from_seconds(seconds: u64) -> Self {
        Self(seconds)
    }

    /// Creates a duration from minutes.
    pub fn from_minutes(minutes: u64) -> Self {
        Self(minutes.checked_mul(60).expect("Duration overflow"))
    }

    /// Creates a duration from hours.
    pub fn from_hours(hours: u64) -> Self {
        Self(hours.checked_mul(3600).expect("Duration overflow"))
    }

    /// Creates a duration from days.
    pub fn from_days(days: u64) -> Self {
        Self(days.checked_mul(86400).expect("Duration overflow"))
    }
}

impl From<u64> for Timestamp {
    fn from(secs: u64) -> Self {
        Self(secs)
    }
}

impl From<Timestamp> for u64 {
    fn from(ts: Timestamp) -> Self {
        ts.0
    }
}

impl From<u64> for Duration {
    fn from(secs: u64) -> Self {
        Self(secs)
    }
}

impl From<Duration> for u64 {
    fn from(d: Duration) -> Self {
        d.0
    }
}
