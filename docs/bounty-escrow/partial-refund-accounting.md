# Partial Refund Accounting — `bounty_escrow` EscrowData

> Issue #1294 — Balance invariant verification for `EscrowStatus::PartiallyRefunded`

## Overview

The `bounty_escrow` contract supports partial refunds: an admin can approve returning a
portion of locked funds to the depositor before the full escrow is settled. After each
partial refund the escrow transitions to `EscrowStatus::PartiallyRefunded` and the
remaining claimable balance is decremented exactly.

This document describes the accounting invariant, the state machine, and the test
coverage added in Issue #1294.

---

## Core Invariant

At every point in the escrow lifecycle the following must hold:

```
escrow.amount - sum(refund_history[*].amount) == escrow.remaining_amount
```

Where:

| Field | Description |
|---|---|
| `escrow.amount` | Total amount originally locked (immutable after `lock_funds`) |
| `refund_history` | Append-only log of every refund executed on this escrow |
| `escrow.remaining_amount` | Funds still available for release or further refund |

---

## State Machine

```
Locked ──(partial refund, amount < remaining)──► PartiallyRefunded
Locked ──(full refund or RefundMode::Full)──────► Refunded
PartiallyRefunded ──(partial refund, amount < remaining)──► PartiallyRefunded
PartiallyRefunded ──(refund drains remainder)───────────────► Refunded
```

Key rules enforced by the contract:

1. `approve_refund(amount)` requires `amount > 0 && amount <= remaining_amount`.
2. `refund()` sets status to `Refunded` when `mode == Full` **or** `amount >= remaining_amount`.
3. `refund()` sets status to `PartiallyRefunded` otherwise.
4. `remaining_amount` is decremented by exactly `refund_amount` (integer subtraction, no rounding).
5. Each `refund()` call appends exactly one entry to `refund_history`.

---

## Security Assumptions

- **No underflow**: `remaining_amount` is checked against `refund_amount` before subtraction;
  `checked_sub` is used to catch any arithmetic overflow at the Rust level.
- **No double-spend**: The reentrancy guard prevents concurrent execution of any protected
  function. State is updated (CEI pattern) before the external token transfer.
- **Approval consumed**: The `RefundApproval` storage entry is deleted after a successful
  `refund()` call, preventing replay of the same approval.
- **Status guard**: `approve_refund` and `refund` both reject escrows that are not in
  `Locked` or `PartiallyRefunded` state, preventing refunds on already-settled escrows.
- **Isolation**: Each escrow's `remaining_amount` and `refund_history` are stored under a
  per-`bounty_id` key; partial refunds on one escrow cannot affect another.

---

## Test Coverage (Issue #1294)

All tests live in
`contracts/bounty_escrow/contracts/escrow/src/test_boundary_edge_cases.rs`.

| Test | What it verifies |
|---|---|
| `test_sequential_partial_refunds_balance_invariant` | Invariant holds after each of 3 sequential partial refunds |
| `test_partial_refund_of_full_amount_transitions_to_refunded` | `Partial` mode with `amount == remaining` → `Refunded` |
| `test_zero_amount_partial_refund_is_rejected` | `approve_refund(0)` is rejected; state unchanged |
| `test_two_sequential_partial_refunds_invariant` | Two refunds from `PartiallyRefunded` state; invariant holds |
| `test_partial_refunds_then_full_drain_transitions_to_refunded` | Multiple partials then final drain → `Refunded` |
| `test_partial_refund_isolation_between_escrows` | Refunding escrow A does not affect escrow B |
| `test_minimum_unit_partial_refund_invariant` | 1-stroop refund accepted; invariant holds |
| `test_partial_refund_exceeding_remaining_is_rejected` | `approve_refund(remaining + 1)` rejected; state unchanged |
| `test_full_mode_refund_always_transitions_to_refunded` | `RefundMode::Full` always yields `Refunded` |
| `test_refund_history_grows_per_partial_refund` | `refund_history.len()` increments by 1 per call |

### Running the tests

```bash
cargo test -p escrow -- test_boundary_edge_cases
```

---

## Example: Sequential Partial Refunds

```rust
// Lock 1000 units
client.lock_funds(&depositor, &bounty_id, &1000, &deadline);
// remaining_amount = 1000, status = Locked

// First partial refund: 300
client.approve_refund(&bounty_id, &300, &depositor, &RefundMode::Partial);
client.refund(&bounty_id);
// remaining_amount = 700, status = PartiallyRefunded
// refund_history = [{ amount: 300, ... }]

// Second partial refund: 300
client.approve_refund(&bounty_id, &300, &depositor, &RefundMode::Partial);
client.refund(&bounty_id);
// remaining_amount = 400, status = PartiallyRefunded
// refund_history = [{ amount: 300 }, { amount: 300 }]

// Final refund drains remainder: 400
client.approve_refund(&bounty_id, &400, &depositor, &RefundMode::Partial);
client.refund(&bounty_id);
// remaining_amount = 0, status = Refunded
// refund_history = [{ amount: 300 }, { amount: 300 }, { amount: 400 }]
// Invariant: 1000 - (300 + 300 + 400) == 0 ✓
```

---

## Related Files

- `contracts/bounty_escrow/contracts/escrow/src/lib.rs` — `approve_refund`, `refund`, `EscrowStatus`, `RefundMode`
- `contracts/bounty_escrow/contracts/escrow/src/test_boundary_edge_cases.rs` — tests
- `contracts/bounty_escrow/contracts/escrow/INVARIANTS_ESCROW.md` — broader invariant catalog
