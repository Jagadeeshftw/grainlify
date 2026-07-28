# Anonymous-lock privacy: query-path audit

`lock_funds_anonymous` shields a depositor's identity: it stores only a
32-byte `depositor_commitment` under `DataKey::EscrowAnon(bounty_id)` and never
persists the depositor's `Address` on-chain. Identity is meant to stay hidden
until the configured `AnonymousResolver` calls `refund_resolved`.

This document tracks which **read/query** functions on `BountyEscrowContract`
are anonymization-aware (safe to call on an anonymously-locked bounty before
resolution) and which are not. Regression coverage lives in
`src/test_anonymization.rs`.

## Anonymization-aware (safe pre-resolution)

| Function | Why it's safe |
|---|---|
| `get_escrow_info` | Only reads `DataKey::Escrow`. An anonymous lock never writes that key (it writes `DataKey::EscrowAnon` instead), so this returns `Error::BountyNotFound` for an anonymous bounty rather than any address-bearing record. |
| `get_metadata` | `EscrowMetadata` has no depositor field at all (`repo_id`, `issue_id`, `bounty_type`, `risk_flags`, `notification_prefs`, `reference_hash`). There is nothing in this record that could carry depositor identity, regardless of lock mode. |
| Raw storage layout | `lock_funds_anonymous` does not add the depositor to `DataKey::DepositorIndex(Address)` — the per-depositor index populated by `lock_funds` / `batch_lock_funds` / recurring locks. This index only ever links a depositor to their *non-anonymous* bounties. |

## Not implemented (out of scope for this audit)

| Function | Status |
|---|---|
| `query_escrows_by_depositor` | **Not currently implemented** on `BountyEscrowContract`. The `DataKey::DepositorIndex` storage it would read exists and is written by the non-anonymous lock paths, but no public contract method exposes a lookup over it yet. Several test files in this crate (`test_query_filters.rs`, `test_compatibility.rs`, `test_analytics_monitoring.rs`) reference a method of this name; none of them are wired into `lib.rs`'s `mod` list, so they do not currently compile or run. When this query is implemented, it **must not** read from a data source that includes anonymous bounty ids — the invariant enforced by `test_anonymous_lock_writes_only_commitment_no_address_indexes` (that `lock_funds_anonymous` never touches `DepositorIndex`) is what keeps a future implementation of this query honest, and should be preserved. |

## Resolution path (the only intended identity-reveal point)

Identity for an anonymous escrow becomes observable only through
`refund_resolved(bounty_id, recipient)`, callable exclusively by the address
set via `set_anonymous_resolver` (admin-only). The `recipient` supplied there
is the sole point at which a real address gets associated with the bounty.
This is exercised end-to-end by
`test_identity_hidden_until_resolver_driven_refund`.

## Coordination note

This audit is scoped to the *read* paths only (issue #1466). It intentionally
does not re-verify `set_anonymous_resolver` / `refund_resolved` write-path
correctness — at the time of writing both are implemented (see
`src/lib.rs`) and covered by their own tests; that logic is tracked and
reviewed separately to avoid duplicating effort.
