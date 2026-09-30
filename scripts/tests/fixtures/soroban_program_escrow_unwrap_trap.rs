//! Fixture used only by `scripts/check-program-escrow-no-traps.py --expect-fail`.
//! Intentionally contains a deployable-style `unwrap()` so the CI gate can prove
//! it rejects new traps in the **Soroban** program-escrow tree.
//! This file is NOT compiled into any contract.

pub fn soroban_fixture_should_fail_check(value: Option<u32>) -> u32 {
    // Deliberate trap site for CI validation of the no-unwrap gate (soroban tree).
    value.unwrap()
}
