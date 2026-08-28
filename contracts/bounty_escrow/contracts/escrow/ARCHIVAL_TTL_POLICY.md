# Persistent Record TTL Policy

This policy is the source of truth for storage rent and archival behavior of
the bounty escrow contract's program records, claims, capability commitments,
and indexes.

TTL values are ledger counts. The day estimates assume a five-second ledger
close and are descriptive only; tests and contract behavior use ledger counts.

| Record class | Storage keys | Minimum live TTL | Minimum archival TTL |
|---|---|---:|---:|
| Program escrow | `Escrow`, `EscrowAnon` | 518,400 ledgers, about 30 days | 1,555,200 ledgers, about 90 days |
| Pending claim | `PendingClaim` | 120,960 ledgers, about 7 days | 518,400 ledgers, about 30 days |
| Capability commitment | `Capability` | 120,960 ledgers, about 7 days | 518,400 ledgers, about 30 days |
| Global and depositor indexes | `EscrowIndex`, `DepositorIndex` | 1,555,200 ledgers, about 90 days | 3,110,400 ledgers, about 180 days |

Active records use the live TTL. Terminal records use the archival TTL:

- A program escrow is terminal after release, full refund, or explicit archival.
- A claim is terminal after execution or expiration. Cancelling a claim deletes
  both the claim and its tracking marker, so its typed status becomes `Missing`.
- A capability commitment is terminal after revocation, expiration, exhaustion
  of uses, or exhaustion of its remaining amount.
- Indexes use live retention while active escrow operations write or query them.
  A terminal escrow operation upgrades the global index and its depositor index,
  when present, to archival retention.

Every tracked record has a persistent marker containing the last ledger through
which the contract guarantees that the record is live. Markers use a longer
6,220,800-ledger TTL so the probe can return a typed status after the underlying
record expires:

- `Missing`: no tracked record or marker exists.
- `Live`: the current ledger is before or exactly at the guaranteed live-until
  ledger.
- `Archived`: the current ledger is after the guaranteed live-until ledger.

The status is a preflight result. `Archived` does not restore storage. The
transaction submitter must restore the archived ledger entry before invoking an
operation that reads or writes it.

Live records created before tracking markers were introduced are reported as
`Live` when their underlying entry is still accessible. Once an untracked
legacy entry expires, no on-chain marker exists to distinguish it from a record
that was never created.

## Renewal Thresholds

Writes and record-returning reads renew a tracked entry when its remaining TTL
is at or below half of the applicable live or archival TTL. This avoids paying
for an extension on every access while restoring the full policy TTL when the
threshold is reached. A call one ledger before the threshold does not extend
the entry; a call exactly at the threshold does. After the record's guaranteed
live-until ledger, the probe reports `Archived` until a restore operation makes
the entry available again.

The caller submitting the contract invocation pays the resource and rent cost
of any TTL extension caused by that invocation. The account submitting a
restore transaction pays the restoration cost. The contract does not charge a
separate escrow participant or administrator automatically.

## Program Escrow Contract

The separate `contracts/program-escrow` package keeps its established adaptive
policy documented in `docs/program-escrow-adaptive-ttl.md`: program instance
state and recipient payout indexes have a 518,400-ledger minimum and scale up
to 3,110,400 ledgers based on access frequency. This bounty escrow policy does
not change that contract's existing behavior.

## Deterministic Coverage

`src/test_archival_ttl.rs` advances ledger sequence numbers to one ledger before,
exactly at, and one ledger after each tracked expiration boundary. It also
covers read renewal, write renewal, payout retention, explicit archival,
terminal claims and commitments, typed missing state, and snapshot restoration.
