# Program Escrow Admin Rotation

## Overview

`program-escrow` uses a two-step admin rotation flow:

1. The current admin calls `propose_admin`.
2. The proposed admin calls `accept_admin`.

This reduces the risk of rotating control to an address that cannot or does not intend to assume the role.

## Current behavior

### Proposal replacement

A new `propose_admin` call **overwrites** any existing pending admin proposal.

This is intentional and security-sensitive:

- the latest admin intent wins
- an older proposed admin cannot later accept after being replaced
- reviewers only need to reason about a single active proposal at a time

### Acceptance authorization

Only the **currently proposed admin** can complete `accept_admin`.

A non-proposed address is rejected because the contract requires authorization from the stored pending admin address.

### Proposal expiry

Each proposal stores transition metadata, including:

- proposer
- proposed role address
- proposal timestamp
- acceptance deadline
- nonce

Acceptance is rejected once the current ledger timestamp exceeds the stored deadline.
Expired proposals are cleared from storage before returning an error so stale state cannot be reused.

## Security notes

### Last-write-wins is safer than multiple pending proposals

Allowing multiple valid pending proposals would make admin rotation ambiguous and could let an outdated candidate unexpectedly seize control. The contract now treats admin rotation as a **single-slot state machine**:

- one pending admin address
- one pending transition record
- any new proposal atomically replaces both

### Expired proposals are invalidated eagerly

On expired acceptance attempts, the contract clears:

- `PendingAdmin`
- `PendingAdminTransition`

That ensures stale proposals cannot linger after their TTL has elapsed.

### Backward-compatibility note

`accept_admin` tolerates legacy state where only `PendingAdmin` is present by falling back to a no-expiry transition record. Newly created proposals always write the full transition metadata.

## Tests added

The RBAC coverage in `contracts/program-escrow/src/test_rbac.rs` now verifies:

1. a second `propose_admin` overwrites the first pending proposal
2. `accept_admin` from a non-proposed address is rejected
3. acceptance after the proposal TTL fails with `RoleTransitionExpired`

The broader regression suite in `contracts/program-escrow/src/test.rs` was also updated so legacy expectations match the new overwrite semantics.

## Reviewer checklist

- Confirm `propose_admin` writes both pending-address and transition metadata.
- Confirm `accept_admin` rejects expired proposals before mutating `Admin`.
- Confirm successful acceptance clears all pending admin rotation state.
- Confirm replacement proposals invalidate the previously proposed address.
