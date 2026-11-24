//! Superseded by `crate::test_timelock_boundary`.
//!
//! This suite previously duplicated timelock delay, threshold, boundary, and
//! execution-timing coverage that is already exercised by
//! `contracts/grainlify-core/src/test_timelock_boundary.rs`. It remains wired so
//! the orphaned module compiles as part of `cargo test -p grainlify-core`, but
//! intentionally contains no duplicate tests.
#![cfg(test)]
