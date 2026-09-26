# Batch Failure Semantics

Every batch entry point in this repository is **all-or-nothing**. This document
is the single place that says so, what the return value does and does not tell
you, and how size limits are enforced. It is the answer to "can a caller tell
which elements settled?" — the short version is *they do not need to, because
they all settle together or none do.*

Introduced by [#1877](https://github.com/Jagadeeshftw/grainlify/issues/1877),
which found the semantics written down in three places (two test-file headers,
one rustdoc block) and enforced by a test suite that was disabled in `lib.rs`.

## The model in one paragraph

Every batch validates everything before it mutates anything. If any element
fails a check, the call aborts and the Soroban host rolls back every storage
write and token transfer made so far in that call. The contract ends the call in
exactly the state it started in. There is no partial settle, so there is never a
mixed set of outcomes to report.

## The model in a table

| Contract | Entry point | Shape | All-or-nothing | Size cap | Return |
| --- | --- | --- | --- | --- | --- |
| `bounty-escrow` | `batch_lock_funds` | AoS `Vec<LockFundsItem>` | yes | 1..=20 | `Result<u32, Error>` |
| `bounty-escrow` | `batch_lock` | alias of `batch_lock_funds` | yes | 1..=20 | `Result<u32, Error>` |
| `bounty-escrow` | `batch_lock_funds_soa` | SoA, 4 parallel arrays | yes | 1..=20 | `Result<u32, Error>` |
| `bounty-escrow` | `batch_release_funds` | AoS `Vec<ReleaseFundsItem>` | yes | 1..=20 | `Result<u32, Error>` |
| `bounty-escrow` | `batch_release_funds_soa` | SoA, 2 parallel arrays | yes | 1..=20 | `Result<u32, Error>` |
| `program-escrow` | `batch_initialize_programs` | AoS `Vec<ProgramInitItem>` | yes | 1..=100 | `Result<u32, BatchError>` |
| `program-escrow` | `batch_lock` | AoS `Vec<LockItem>` | yes | 1..=100 | `Result<u32, BatchError>` |
| `program-escrow` | `batch_release` | AoS `Vec<ReleaseItem>` | yes | 1..=100 | `Result<u32, BatchError>` |
| `program-escrow` | `batch_payout` | parallel `recipients`/`amounts` | yes | 1..=100 | `ProgramData` |
| `program-escrow` | `batch_payout_by` | + explicit `caller` | yes | 1..=100 | `ProgramData` |
| `program-escrow` | `batch_payout_idempotent` | + `idempotency_key` | yes | 1..=100 | `ProgramData` |
| `program-escrow` | `batch_payout_idempotent_by` | key + caller | yes | 1..=100 | `ProgramData` |
| `program-escrow` | `batch_payout_with_receipt` | + `merkle_root` | yes | 1..=100 | `BatchReceipt` |
| `program-escrow` | `batch_payout_v2` | + explicit `program_id` | yes | 1..=100 | `ProgramData` |

`bounty-escrow` caps at **20**; `program-escrow` caps at **100**.

## What the return value tells you

This is the part worth being precise about, because the return types look like
they should carry per-element information and do not.

**The `u32` is a count of settled elements, not a per-element result vector.**

- `Ok(n)` — `n` always equals the number of items submitted. Every element
  settled. There is no "3 of 5 locked" outcome to interpret.
- `Err(e)` — **zero** elements settled. The contract is in its pre-call state.

So the return value *does* identify the outcome of every element — it just does
so by making the answer uniform. Every element has the same outcome, and the
count plus the Ok/Err tells you which outcome it is.

**What it does not carry is the index of the offending element.** The error names
the *condition* (`InvalidAmount`, `DuplicateBountyId`, `BountyNotFound`,
`BatchTooLarge`, …) but not *which* item tripped it. For settlement this is
moot — nothing settled — but a caller that wants to know which element to fix has
to re-validate its own input.

### Why a count is a sufficient return value

Only because of atomicity. A count would be dangerously ambiguous under partial
application: `Ok(3)` for a 5-item batch is indistinguishable from "3 settled, 2
did not". Since the contract guarantees `Ok(n) => n == items.len()` and
`Err => 0 settled`, the count is unambiguous.

`test_batch_failure_semantics::return_value_contract_identifies_every_element`
pins both halves of that reading so this document cannot drift from the code.

## "Retry of the remainder" means retry of the whole batch

Because a failed batch settles nothing, **there is no remainder to resume**. The
state after a failure is identical to the state before the call, so a caller
retries by re-submitting the corrected batch in full.

This is the part that is easy to get wrong, and worth stating twice: a caller
that assumed a partial settle and retried only the un-processed suffix would
silently skip elements. A caller that retries the whole batch cannot double-spend,
because the failed attempt moved no funds and consumed no ids.

Two consequences that are easy to miss:

- **Bounty/program ids are not consumed by a failed batch.** They stay
  claimable, so a later batch can legitimately use them. If a failed attempt had
  left a partial write, the retry would get `BountyExists` / `ProgramAlreadyExists`
  instead. Both are asserted in
  `failed_batch_does_not_consume_its_bounty_ids`.
- **The window that idempotency keys exist for is a timeout, not a failure.** If
  the call *succeeded* on-chain but the submitter never saw the receipt,
  re-submitting without a key would double-pay.
  `batch_payout_idempotent` records the outcome against a caller-supplied key and
  returns the original result on replay. That is the supported way to make a retry
  safe, and it is orthogonal to the all-or-nothing rule.

## Size limits

| Contract | Hard cap | Runtime-adjustable | Rejection |
| --- | ---: | --- | --- |
| `bounty-escrow` | `MAX_BATCH_SIZE = 20` | yes, downward only, via `set_batch_size_caps` | `InvalidBatchSize` |
| `program-escrow` | `MAX_BATCH_SIZE = 100` | no | `InvalidBatchSizeProgram` (init/lock/release), `BatchTooLarge` code 410 (payout family) |

Both reject an **empty** batch as well as an oversized one, and both do it
**pre-flight** — before any element is interpreted — so a rejected batch leaves
no trace.

`bounty-escrow`'s cap is a ceiling, not a fixed value: an admin can lower the
effective lock and release caps with `set_batch_size_caps` (never above 20) to
cut the gas footprint of a single call, and the entry points read the effective
cap per call via `get_max_batch_size` / `get_max_release_batch_size`.

`program-escrow`'s 100 is calibrated against Soroban's 100 M instruction
per-invocation ceiling. That margin is real but **not large** — a full 100-item
`batch_initialize_programs` measures ~69.7 M instructions host-side, about 70 %
of the ceiling. See
[program-escrow-batch-init-atomicity.md](program-escrow-batch-init-atomicity.md)
for the table and the reasoning.

## The SoA variants

`batch_lock_funds_soa` and `batch_release_funds_soa` take parallel arrays instead
of an array of structs, to cut host-to-guest deserialization. They are
all-or-nothing exactly like their AoS counterparts and add one precondition of
their own: the arrays must be the same length, checked first and reported as
`BatchSizeMismatch` before any element is interpreted.

After the alignment check they zip into the AoS item type and delegate, so
ordering, duplicate detection, the size cap and rollback are identical.

## Error → condition

`bounty-escrow` (`Error`):

| Condition | Error |
| --- | --- |
| batch empty or above the effective cap | `InvalidBatchSize` |
| parallel arrays of differing length (SoA) | `BatchSizeMismatch` |
| same `bounty_id` twice in one batch | `DuplicateBountyId` |
| `bounty_id` already in storage | `BountyExists` |
| any element's `amount <= 0` | `InvalidAmount` |
| `bounty_id` not found (release) | `BountyNotFound` |
| escrow not `Locked` (release) | `FundsNotLocked` |
| lock/release paused | `FundsPaused` |
| contract killed via `set_deprecated` | `ContractDeprecated` |
| batch cap set above `MAX_BATCH_SIZE` | `InvalidBatchSizeCap` |

`program-escrow` (`BatchError`):

| Condition | Error |
| --- | --- |
| batch empty or above `MAX_BATCH_SIZE` (init/lock/release) | `InvalidBatchSizeProgram` |
| batch above `MAX_BATCH_SIZE` (payout family) | `BatchTooLarge` (410) |
| same `program_id` twice in one batch | `DuplicateProgramId` |
| `program_id` already registered (init) | `ProgramAlreadyExists` |
| `program_id` not registered | `ProgramNotFound` |
| any element's `amount <= 0` | `InvalidAmount` |
| release paused globally or per program | `FundsPaused` |
| `recipients.len() != amounts.len()` | length-mismatch panic |

The `program-escrow` payout path signals failure with `panic!` /
`panic_with_error!` rather than `Err`, so on the wire a caller sees a host error
carrying the contract error code. The rollback guarantee is identical — the host
rolls back on panic too, which is exactly what makes these batches atomic.

## Where this is enforced

| Suite | Location | Tests |
| --- | --- | ---: |
| `test_batch_failure_semantics` | `contracts/bounty_escrow/contracts/escrow/src/` | 12 |
| `test_batch_failure_modes` | same crate | 24 |
| `test_batch_failure_mode` | same crate | 44 |
| `test_batch_operations` | `contracts/program-escrow/src/` | 40 |
| `test_batch_limits` | same crate | 6 |
| `test_program_batch_registration` | same crate | 14 |

`test_batch_failure_semantics` is the one that pins *this document* — the
all-or-nothing contract across all five `bounty-escrow` entry points, the
return-value reading, the size boundary, the runtime cap, and the retry
semantics. The other suites enumerate which error a given malformed batch
produces.

### Running them

```bash
bash contracts/scripts/ci-batch-tests.sh
```

All 164 tests above are green and run from that one script.

In CI they run as the **`batch-tests` job** in `.github/workflows/contracts-ci.yml`,
*not* as a step inside the `build-test` job. That separation is deliberate:
`build-test` currently fails on pre-existing `bounty_escrow` failures unrelated to
batching, and a step placed after a failed step never executes. A batch gate
inlined into that job would therefore have silently stopped gating anything —
which is exactly the state #1877 found `test_batch_operations.rs` in (disabled in
`lib.rs`, absent from CI). As its own job it runs and reports on every pull
request regardless.

`contracts/scripts/ci-contracts.sh` also calls it, so a local run of the contract
gates covers batching too.

The module list lives in `ci-batch-tests.sh` and is enumerated per module rather
than filtered on a `batch` substring. A substring filter also sweeps in unrelated
suites that are red for reasons that have nothing to do with batch semantics, and
a gate that is red for the wrong reason stops being read.

## Known unrelated failures

While making these suites run, several **pre-existing** failures surfaced that
belong to other issues and are not fixed here. Recorded so nobody mistakes them
for regressions from this work:

| Suite | Failing | Cause |
| --- | ---: | --- |
| `test_deterministic_event_ordering` (escrow) | 2 | anti-abuse cooldown rejects the batch lock |
| `test_event_payload_fixtures` (escrow) | 3 | fixture/event struct drift |
| `test_gas_ci_thresholds` (escrow) | 1 | gas threshold budget |
| `test_metadata_tagging` (program-escrow) | 8 | metadata aggregate limit |
| `test_event_schema` (program-escrow) | 4 | event correlation fields |

These were verified failing on a clean `master` before any change in #1877.
