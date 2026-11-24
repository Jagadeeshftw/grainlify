//! Fixture used only by `scripts/check-program-escrow-no-traps.py`.
//!
//! Deliberately carries a file-level `#![cfg(test)]` inner attribute and an
//! `unwrap()` so the CI gate can prove that whole-file test-gated sources are
//! exempt from the no-trap scan. This file is NOT compiled into the contract.
//!
//! Usage:
//!   python3 scripts/check-program-escrow-no-traps.py \
//!       --fixture scripts/tests/fixtures/program_escrow_cfg_test_exempt.rs

#![cfg(test)]

/// Deliberate trap site that must be ignored because the file is test-gated.
pub fn fixture_cfg_test_gated_helper(value: Option<u32>) -> u32 {
    value.unwrap()
}
