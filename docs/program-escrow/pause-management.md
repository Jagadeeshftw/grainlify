# Program Escrow Pause Management

This document describes the three granular pause flags used by `contracts/program-escrow` and the security expectations they enforce.

## Overview

The contract exposes three independent pause booleans:

- `lock_paused`
- `release_paused`
- `refund_paused`

These flags are intentionally **orthogonal**. Pausing one operation family must not accidentally pause or unpause another.

## Operation Mapping

### `lock_paused`
Blocks only inbound fund-locking operations.

- `lock_program_funds` → blocked when `lock_paused = true`

### `release_paused`
Blocks operations that authorize or execute fund release from escrow.

- `single_payout` → blocked when `release_paused = true`
- `create_pending_claim` → blocked when `release_paused = true`
- `execute_claim` → blocked when `release_paused = true`
- `trigger_program_releases` → blocked when `release_paused = true`

### `refund_paused`
Blocks only refund-path operations.

- `cancel_claim` → blocked when `refund_paused = true`

## Exhaustive 8-Combination Matrix

| lock_paused | release_paused | refund_paused | lock_program_funds | single_payout | create_pending_claim | execute_claim | trigger_program_releases | cancel_claim |
|-------------|----------------|---------------|--------------------|---------------|----------------------|---------------|--------------------------|--------------|
| false | false | false | allow | allow | allow | allow | allow | allow |
| true  | false | false | block | allow | allow | allow | allow | allow |
| false | true  | false | allow | block | block | block | block | allow |
| false | false | true  | allow | allow | allow | allow | allow | block |
| true  | true  | false | block | block | block | block | block | allow |
| true  | false | true  | block | allow | allow | allow | allow | block |
| false | true  | true  | allow | block | block | block | block | block |
| true  | true  | true  | block | block | block | block | block | block |

## Security Notes

1. **Flag isolation is a security property**  
   A partial unpause must mutate only the explicitly targeted flag. For example, clearing `release_paused` must not clear `lock_paused` or `refund_paused`.

2. **Release-path protection must cover both direct and deferred execution**  
   `release_paused` is not limited to direct payouts. It also blocks claim creation, claim execution, and scheduled-release triggering so operators can halt all outbound fund movement with one control.

3. **Refund-path protection remains independent**  
   `refund_paused` controls claim cancellation without affecting lock or release behavior. This supports incident response where outbound releases may resume while refunds remain frozen, or vice versa.

4. **Read-only and maintenance controls are separate layers**  
   Pause flags are targeted operational controls. They should be reasoned about independently from broader maintenance or read-only modes.

5. **Exhaustive tests are required to prevent regressions**  
   Combinatorial bugs often appear when one flag is cleared while others remain set. The test suite therefore covers all eight boolean combinations and includes a focused partial-unpause regression test.

## Code References

- Contract logic: `contracts/program-escrow/src/lib.rs`
- Exhaustive tests: `contracts/program-escrow/src/test_granular_pause.rs`

## Validation

Run the package tests with:

```sh
cargo test -p program-escrow
```

If the environment cannot complete a full build, at minimum verify that:

- `contracts/program-escrow/src/lib.rs` has no parser diagnostics
- the granular pause module is compiled by `lib.rs`
- the eight matrix cases in `test_granular_pause.rs` remain aligned with the operation mapping above
