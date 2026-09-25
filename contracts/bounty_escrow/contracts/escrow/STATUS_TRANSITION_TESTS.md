# Escrow Status Transitions Test Suite

## Overview
This suite pins the escrow lifecycle as a finite-state machine and asserts that every status pair is either an explicit legal edge or a rejected illegal transition.

## States
The escrow contract has five statuses:
- **Draft**: initial unpublished record before funds are locked
- **Locked**: active escrow holding funds
- **Released**: terminal success state after payout
- **Refunded**: terminal full-refund state
- **PartiallyRefunded**: intermediate refund state with remaining balance still owed to the depositor

## Legal edges

| From | To | Entry point | Result |
|------|-----|-------------|--------|
| Draft | Locked | `publish` / lock creation path | allowed |
| Locked | Released | `release_funds` | allowed |
| Locked | Refunded | `refund` | allowed |
| Locked | PartiallyRefunded | `approve_refund` + `refund` | allowed |
| PartiallyRefunded | Refunded | `refund` | allowed |
| PartiallyRefunded | PartiallyRefunded | subsequent partial refund | allowed |

## Illegal edges
All other state pairs are rejected.

- `Error::FundsNotLocked` rejects re-entry into a released or refunded state, or any transition out of a terminal state.
- `Error::BountyExists` rejects attempts to lock an escrow that already exists, including re-locking a `Locked`, `Released`, `Refunded`, or `PartiallyRefunded` escrow.

| From | To | Rejection |
|------|-----|-----------|
| Draft | Released | `Error::FundsNotLocked` |
| Draft | Refunded | `Error::FundsNotLocked` |
| Draft | PartiallyRefunded | `Error::FundsNotLocked` |
| Locked | Locked | `Error::BountyExists` |
| Locked | Draft | `Error::FundsNotLocked` |
| Released | Locked | `Error::BountyExists` |
| Released | Released | `Error::FundsNotLocked` |
| Released | Refunded | `Error::FundsNotLocked` |
| Released | PartiallyRefunded | `Error::FundsNotLocked` |
| Released | Draft | `Error::FundsNotLocked` |
| Refunded | Locked | `Error::BountyExists` |
| Refunded | Released | `Error::FundsNotLocked` |
| Refunded | Refunded | `Error::FundsNotLocked` |
| Refunded | PartiallyRefunded | `Error::FundsNotLocked` |
| Refunded | Draft | `Error::FundsNotLocked` |
| PartiallyRefunded | Locked | `Error::BountyExists` |
| PartiallyRefunded | Released | `Error::FundsNotLocked` |
| PartiallyRefunded | Draft | `Error::FundsNotLocked` |

## Test implementation
- **File**: `src/test_status_transitions.rs`
- **Coverage**: every status pair in the $5 \times 5$ matrix is enumerated in `test_status_pair_matrix_is_exhaustive_and_documented`
- **Contract invariant**: legal edges are fixed, and every non-edge is rejected with a named error

## Verification
Run:
```bash
cargo test --lib test_status_pair_matrix_is_exhaustive_and_documented -- --nocapture
```
