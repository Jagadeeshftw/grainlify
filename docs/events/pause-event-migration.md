# PauseStateChanged Event: V1 → V2 Migration Guide

## Overview

The `PauseStateChanged` event emitted by the `program-escrow` contract was
extended in V2 with a new `receipt_id` field. This document describes the
schema change, the wire-format compatibility guarantee, and the steps indexer
operators must take to upgrade.

## Schema Comparison

| Field        | V1  | V2  | Type            | Notes                                    |
|--------------|-----|-----|-----------------|------------------------------------------|
| `operation`  | ✅  | ✅  | `Symbol`        | `"lock"`, `"release"`, or `"refund"`     |
| `paused`     | ✅  | ✅  | `bool`          | `true` = paused, `false` = unpaused      |
| `admin`      | ✅  | ✅  | `Address`       | Address that triggered the state change  |
| `reason`     | ✅  | ✅  | `Option<String>`| Human-readable reason, may be `None`     |
| `timestamp`  | ✅  | ✅  | `u64`           | Ledger timestamp (seconds since epoch)   |
| `receipt_id` | ❌  | ✅  | `u64`           | **New in V2** — monotonic receipt counter|

## Wire-Format Compatibility

Soroban encodes `#[contracttype]` structs as **XDR maps keyed by field name**
(sorted lexicographically). This means:

- A **V1 parser** reading a V2 XDR blob will successfully decode all five
  original fields and silently ignore the unknown `receipt_id` key.
- A **V2 parser** reading a V1 XDR blob will fail to find `receipt_id` and
  may return an error or default to `0`, depending on the SDK version.

The migration tests in
`contracts/program-escrow/src/test_pause_event_migration.rs` verify both
directions.

## V2 XDR Golden (reference)

The following hex is the canonical V2 encoding of a `PauseStateChanged` event
with a fixed admin address (`0x05…05`), `operation = "lock"`, `paused = true`,
`reason = None`, `timestamp = 12345`, `receipt_id = 1`:

```
0000001100000001000000060000000f0000000561646d696e000000000000120000000105050505
050505050505050505050505050505050505050505050505050505050000000f000000096f706572
6174696f6e0000000000000f000000046c6f636b0000000f00000006706175736564000000000000
000000010000000f00000006726561736f6e0000000000010000000f0000000a726563656970745f
696400000000000500000000000000010000000f0000000974696d657374616d7000000000000005
0000000000003039
```

The map has **6 entries** (V1 had 5). The `receipt_id` entry appears between
`reason` and `timestamp` in lexicographic key order.

## Upgrade Steps for Indexer Operators

### Option A — Ignore `receipt_id` (minimal change)

If your indexer only needs the five original fields, no code change is required.
Your V1 parser will silently skip `receipt_id` and continue working correctly.

**Verify** by running your parser against the golden hex above and confirming
all five fields decode as expected.

### Option B — Consume `receipt_id` (recommended)

Add `receipt_id: u64` to your event schema. This enables:

- **Deduplication**: detect and discard replayed events using `receipt_id` as
  a unique key per contract instance.
- **Ordering**: `receipt_id` is monotonically increasing, so you can detect
  gaps in event streams.

### Option C — Replay historical V1 events with a V2 parser

If you are replaying historical events emitted before the V2 upgrade, your V2
parser will encounter XDR blobs without `receipt_id`. Handle this by:

1. Catching the decode error and treating `receipt_id` as `0`.
2. Or maintaining a separate code path for events before the upgrade ledger.

The upgrade ledger will be announced in the contract changelog.

## Security Notes

- `receipt_id` is **informational only**. It does not affect fund safety.
- A V1 parser that drops `receipt_id` loses deduplication capability but
  cannot be tricked into double-processing a payout.
- The `operation` field distinguishes which pause flag changed (`lock`,
  `release`, or `refund`). V1 parsers already handle this correctly.

## Test Coverage

The migration tests in `test_pause_event_migration.rs` cover:

| Test | What it verifies |
|---|---|
| `test_v1_parser_decodes_v2_xdr_without_panic` | V1 schema decodes V2 XDR, all 5 fields match |
| `test_v1_parser_decodes_v2_xdr_no_reason` | Same with `reason = None` |
| `test_v2_parser_decodes_v1_xdr_receipt_id_defaults` | V2 parser on V1 XDR: no silent corruption |
| `test_v2_roundtrip` | V2 encodes and decodes correctly |
| `test_v2_roundtrip_unpause_no_reason` | V2 round-trip with `paused = false` |
| `test_v1_roundtrip` | V1 encodes and decodes correctly |
| `test_v2_xdr_larger_than_v1` | V2 XDR is strictly larger (extra field present) |
| `test_v1_parser_all_operations` | V1 parse succeeds for lock/release/refund ops |
| `test_v2_xdr_golden_field_count` | V2 XDR size sanity check |
