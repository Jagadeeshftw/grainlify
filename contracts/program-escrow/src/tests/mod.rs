//! `#[cfg(test)]` test submodules.
//!
//! Conventionally this crate's unit tests live as flat `src/test_*.rs` files
//! declared inside `lib.rs`.  This submodule hierarchy groups post-hoc
//! property-based test suites that touch internal math primitives and that
//! are large enough to warrant their own directory.
//!
//! All submodules are gated by `#[cfg(test)]` so they are compiled only when
//! the crate is built with `--tests` or `cargo test`.  They do not influence
//! the WASM contract binary (`crate-type = ["cdylib"]`).

#[cfg(test)]
mod fee_rounding_props;
