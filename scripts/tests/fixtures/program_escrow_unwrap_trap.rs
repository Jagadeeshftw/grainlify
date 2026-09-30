//! Fixture used only by `scripts/check-program-escrow-no-traps.py --expect-fail`.
//! Intentionally contains a deployable-style `unwrap()` so the CI gate can prove
//! it rejects new traps. This file is NOT compiled into the contract.

pub fn fixture_should_fail_check(value: Option<u32>) -> u32 {
    // Deliberate trap site for CI validation of the no-unwrap gate.
    value.unwrap()
}
