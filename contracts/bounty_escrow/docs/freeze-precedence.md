# Freeze Precedence: Escrow-Level vs Address-Level Freezes

**Contract:** `contracts/bounty_escrow/contracts/escrow/src/lib.rs`
**Checks:** `ensure_escrow_not_frozen`, `ensure_address_not_frozen`
**Tests:** `contracts/bounty_escrow/contracts/escrow/src/test_frozen_balance.rs`

## The question

The escrow contract exposes two independent freeze mechanisms:

1. **Escrow-level** — `freeze_escrow` / `unfreeze_escrow`, keyed by
   `bounty_id`, stored at `EscrowFreeze(bounty_id)`.
2. **Address-level** — `freeze_address` / `unfreeze_address`, keyed by an
   `Address`, stored at `AddressFreeze(address)`.

What happens when both apply simultaneously to the same bounty and
depositor — or when only one layer applies? Undefined precedence could let an
operation slip through when it should be blocked, or block one that
shouldn't be.

## Locked-in behavior

**Either freeze independently blocks.** Every funds-out path checks both
layers, escrow first, then the escrow's **depositor** address:

```rust
Self::ensure_escrow_not_frozen(&env, bounty_id)?;   // -> Error::EscrowFrozen
Self::ensure_address_not_frozen(&env, &escrow.depositor)?; // -> Error::AddressFrozen
```

Gated paths: `release_funds`, `partial_release`, `batch_release_funds`,
`release_with_capability`, `release_with_conversion`, `execute_queued_release`,
`refund`, `refund_with_capability`, `authorize_claim`, `claim`,
`claim_with_capability`, `renew_escrow`, and the dry-run/eligibility views.

### Outcome matrix (release, refund, and claim)

| Escrow frozen | Depositor frozen | Outcome |
|---|---|---|
| no  | no  | ✅ operation proceeds |
| yes | no  | ❌ `Error::EscrowFrozen` |
| no  | yes | ❌ `Error::AddressFrozen` |
| yes | yes | ❌ `Error::EscrowFrozen` (escrow check runs first — deterministic error ordering) |

### Record independence

The two layers never touch each other's storage:

- Unfreezing the escrow removes only `EscrowFreeze(bounty_id)`; an active
  `AddressFreeze(depositor)` keeps blocking (and vice versa). The operation
  proceeds only once **both** freezes are lifted, in either order.
- `get_escrow_freeze_record` and `get_address_freeze_record` remain
  independently queryable and accurate throughout overlapping
  freeze/unfreeze sequences — each keeps its own `reason`, `frozen_at`, and
  `frozen_by`.
- Re-freezing one layer updates only that layer's record.

### Scope boundaries (intentional)

- **Address freezes are depositor-keyed.** The address is matched against
  `escrow.depositor` only. Freezing a contributor or claim recipient does
  **not** block release or claim. To stop a payout to a specific recipient,
  freeze the escrow itself (`freeze_escrow`).
- **Funds-out only.** A frozen depositor can still `lock_funds` (money in is
  never blocked; money out is), and read-only queries always succeed while
  any freeze is active.

## Security rationale

- **No bypass through the narrower key.** An admin investigating a depositor
  can freeze the address once and every present and future escrow they own
  is blocked — an individually-unfrozen escrow cannot slip through.
- **No accidental unblocking.** Lifting an escrow-level hold (e.g. a resolved
  per-bounty dispute) cannot silently release funds still held under an
  address-level compliance hold, and vice versa: both layers must clear.
- **Deterministic errors.** With both layers frozen the caller always sees
  `EscrowFrozen`, keeping error ordering stable for clients and monitoring.
- **Checks precede transfers.** Both checks run before any state mutation or
  token transfer on every gated path (CEI-consistent), so a blocked
  operation leaves no partial state.

## Regression tests

`test_frozen_balance.rs` locks this in:

- `test_release_freeze_precedence_matrix`, `test_refund_freeze_precedence_matrix`,
  `test_claim_freeze_precedence_matrix` — all four
  {escrow frozen} × {depositor frozen} combinations per operation, asserting
  the exact error and that blocked operations leave the escrow `Locked`.
- `test_overlapping_freezes_unfreeze_escrow_first` / `..._address_first` /
  `..._block_refund_until_both_lifted` — overlapping sequences stay blocked
  until both layers are lifted, in either order, with records asserted
  accurate at every step.
- `test_refreeze_updates_only_its_own_record`,
  `test_no_phantom_records_from_overlapping_freezes` — record independence.
- `test_contributor_freeze_does_not_block_release`,
  `test_recipient_freeze_does_not_block_claim`,
  `test_frozen_depositor_can_still_lock_new_funds` — the documented scope
  boundaries.
- `test_authorize_claim_blocked_by_either_freeze` — the claim-authorization
  path is gated the same way.
