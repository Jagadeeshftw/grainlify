//! `#[cfg(test)]` test submodules.
//!
//! Conventionally this crate's unit tests live as flat `src/test_*.rs` files
//! declared inside `lib.rs`.  This submodule hierarchy groups post-hoc
//! suites that are large enough to warrant their own directory:
//!
//! - [`fee_rounding_props`] — property-based fee arithmetic invariants
//! - [`bulk_release_optimization_tests`] — host-side O(1) release_schedule
//! - [`chaos_batch_payout_tests`] — deterministic chaos harness for
//!   `batch_payout` / `batch_payout_idempotent` failure injection
//!
//! All submodules are gated by `#[cfg(test)]` so they are compiled only when
//! the crate is built with `--tests` or `cargo test`.

#[cfg(test)]
mod fee_rounding_props;

#[cfg(test)]
mod bulk_release_optimization_tests;

#[cfg(test)]
mod chaos_batch_payout_tests;

#[cfg(test)]
mod delegate_metadata_dos_tests;
