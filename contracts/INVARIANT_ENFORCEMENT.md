# Invariant Enforcement Strategy

This repository enforces invariants in contract major flows and validates that enforcement in CI.

## Where checks run

- `contracts/bounty_escrow/contracts/escrow/src/invariants.rs`

Major state-changing flows call invariant helpers before returning success:

- Bounty escrow: `lock_funds`, `release_funds`, `refund`.

## Meta-tests

Invariant meta-tests validate:

1. Invariant helpers are invoked in major flows (call counter increases).
2. If invariant enforcement is disabled in test mode, core flows panic.

Files:

- `contracts/bounty_escrow/contracts/escrow/src/test_invariants.rs`

## CI integration

GitHub workflows include explicit invariant-focused test execution:

- `.github/workflows/contracts-ci.yml`
- `.github/workflows/contracts.yml`

Each workflow runs `cargo test --lib invariant_checker_ci` for the bounty escrow contract.
