# Fee Routing Immutability and Audit Trail

**Contract:** `contracts/bounty_escrow/contracts/escrow/src/lib.rs`
**Entrypoints:** `set_fee_routing`, `set_fee_routing_with_reason`, `get_fee_routing`
**Event:** `FeeRoutingChanged` (`contracts/bounty_escrow/contracts/escrow/src/events.rs`)
**Tests:** `contracts/bounty_escrow/contracts/escrow/src/test_fee_routing.rs`

## The problem

`set_fee_routing` lets an admin configure a `PerBountyFeeRouting` override
that decides **where** the fee collected for a given `bounty_id` is sent —
split between a primary treasury and an optional partner.

As previously written, the only checks were that the bounty existed and that
the basis-point shares summed to 100%. Nothing prevented the destination from
being changed **after** the bounty had already transitioned to `Locked` with
depositor funds held. An admin could therefore silently redirect where the
eventual fee landed after depositors had committed funds under a different,
publicly observable routing — with the only trace being a `FeeRoutingUpdated`
event that recorded the new destination but not the old one, making the
substitution invisible to anyone not diffing consecutive events.

## The fix

### 1. Immutability guard on the default path

`set_fee_routing` now rejects any change once the bounty's funds are
committed:

```rust
if !allow_post_lock && Self::fee_routing_is_locked(&env, bounty_id) {
    return Err(Error::FeeRoutingLocked);   // error code 60
}
```

`fee_routing_is_locked` treats routing as mutable **only** while a regular
escrow is in `Draft` status. Any later status (`Locked`, `Released`,
`Refunded`, `PartiallyRefunded`) is immutable. Anonymous escrows
(`lock_funds_anonymous`) are created directly in `Locked` status and never
pass through `Draft`, so they are immutable from creation.

The boundary is exactly the `Draft -> Locked` transition performed by
`publish`: routing configured during `Draft` is what depositors see when funds
are committed, and it is frozen from that moment.

### 2. Audited override path

Legitimate post-lock changes still exist — a compromised treasury key, a
terminated partner agreement, a treasury migration. Rather than forbidding
them outright and leaving operators stuck, they are routed through a distinct,
higher-friction entrypoint:

```rust
set_fee_routing_with_reason(
    bounty_id, treasury_recipient, treasury_bps,
    partner_recipient, partner_bps,
    reason,            // mandatory, non-empty
)
```

An empty `reason` is rejected with `InvalidAmount` — the audit trail is the
entire purpose of the path, so a blank justification defeats it. Share
invariants are enforced identically to the default path.

### 3. `FeeRoutingChanged` audit event

Every accepted routing change — on **both** paths — emits `FeeRoutingChanged`
alongside the pre-existing `FeeRoutingUpdated`:

| Field | Meaning |
|---|---|
| `old_treasury_recipient` | Destination before this change (`None` on first configuration) |
| `old_partner_recipient` | Partner before this change, if any |
| `new_treasury_recipient` | Destination after this change |
| `new_partner_recipient` | Partner after this change, if any |
| `changed_by` | Admin address that performed the change |
| `post_lock_override` | `true` when the audited post-lock path was used |
| `reason` | Mandatory justification on the post-lock path; `None` pre-lock |
| `bounty_id`, `version`, `timestamp` | Envelope fields (topic `"fee_rchg"`, EVENT_VERSION_V2) |

Because the event carries the **previous** destination as well as the new one,
an indexer can reconstruct the complete routing history for any bounty from
events alone, and a post-lock redirect is trivially detectable by filtering on
`post_lock_override == true`.

## Security rationale

- **Depositors get what they committed to.** Once funds are locked, the fee
  destination visible at commit time cannot be changed through the ordinary
  admin path. The window for silent substitution is closed.
- **No functionality removed, only made loud.** Post-lock changes remain
  possible for genuine operational needs, but they are impossible to perform
  *silently*: a separate entrypoint, a mandatory on-chain reason, and an event
  flagged `post_lock_override = true`.
- **Old destination is on-chain.** Recording only the new destination (the
  prior behavior) made redirects invisible without cross-referencing earlier
  events; `FeeRoutingChanged` makes each change self-describing.
- **Fail-closed ordering.** Existence is checked before the guard, and the
  guard before share validation, so a rejected call never writes storage nor
  emits an event (asserted in tests).
- **Compatibility.** `FeeRoutingUpdated` is still emitted unchanged, so
  existing indexers keep working; `FeeRoutingChanged` is purely additive.

### Residual risk (accepted)

The post-lock path remains available to the admin — it is a
*higher-privilege, audited* path, not a prohibition. A malicious admin can
still redirect fees post-lock, but cannot do so without leaving an
attributable, indexable on-chain record naming themselves, the old and new
destinations, and a reason. Deployments wanting a hard prohibition can leave
`set_fee_routing_with_reason` unexposed in their admin tooling or gate it
behind the existing multisig/timelock machinery.

Note also that this guard governs the *fee destination* only. The fee
*amount* is computed from the global or per-token rate (see
`docs/fee-config-precedence.md`), which has its own admin controls.

## Test coverage

`test_fee_routing.rs` (19 tests):

- **Pre-lock succeeds** — `test_set_routing_on_draft_succeeds`,
  `test_set_routing_with_partner_on_draft_succeeds`,
  `test_routing_can_be_revised_while_draft`,
  `test_draft_routing_survives_publish_and_routes_release_fee` (end-to-end:
  routing set while `Draft` actually determines where the release fee lands).
- **Post-lock rejected** — `test_set_routing_after_lock_rejected`,
  `test_publish_locks_routing` (the guard engages exactly at the
  `Draft -> Locked` transition), `test_set_routing_after_release_rejected`,
  `test_set_routing_on_anonymous_escrow_rejected`; rejected calls are asserted
  to leave no stored routing.
- **Audited path** — `test_with_reason_succeeds_after_lock`,
  `test_with_reason_rejects_empty_reason`,
  `test_with_reason_missing_bounty_rejected`,
  `test_with_reason_works_on_draft_too`.
- **Invariants on both paths** — `test_share_invariants_enforced_on_plain_path`,
  `test_share_invariants_enforced_on_reason_path`.
- **Audit event** — `test_audit_event_on_first_configuration` (old fields
  `None`, `post_lock_override == false`),
  `test_audit_event_on_post_lock_override` (previous treasury *and* partner
  captured, override flag set, reason recorded).
- **Views** — `test_get_fee_routing_none_when_unset`,
  `test_get_fee_routing_roundtrip`.
