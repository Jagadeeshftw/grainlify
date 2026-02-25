# Invariant Enforcement Strategy

This repository enforces invariants in contract major flows and validates that enforcement in CI (see Issue #73, #486).

## Where checks run

- `contracts/bounty_escrow/contracts/escrow/src/invariants.rs`

Major state-changing flows call invariant helpers before returning success:

- Bounty escrow: `lock_funds`, `release_funds`, `refund`.

## Meta-tests

Invariant meta-tests validate:

1. **Called in major flows** – Invariant helpers are invoked in lock, release, and refund; a call counter is incremented and asserted in tests.
2. **Exact count** – A test runs all three flows (lock, release, refund) and asserts the exact expected number of invariant calls. If any flow stops calling `assert_escrow`, the test fails, preventing bypasses.
3. **Fail when disabled** – If invariant enforcement is disabled in test mode (via the test-only flag), core flows panic, so tests fail when checks are bypassed.

Files:

- `contracts/bounty_escrow/contracts/escrow/src/test_invariants.rs`

## CI integration

GitHub workflows include explicit invariant-focused test execution:

- `.github/workflows/contracts-ci.yml`
- `.github/workflows/contracts.yml`

Each workflow runs `cargo test --lib invariant_checker_ci` for the bounty escrow contract. Invariants are always enabled during test runs; the “disabled” path is only used by the meta-test that asserts disabling causes a panic.
