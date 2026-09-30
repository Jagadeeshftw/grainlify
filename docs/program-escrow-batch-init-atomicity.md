# Batch Initialize Programs — Atomicity & Pre-validation Benchmark

**Contract:** `contracts/program-escrow/src/lib.rs`  
**Function:** `batch_initialize_programs`  
**Issue:** [#1499](https://github.com/anomalyco/grainlify/issues/1499)

---

## Atomicity Model

`batch_initialize_programs` provides an **all-or-nothing** guarantee:

1. **Pre-validation phase** — runs before any storage mutation:
   - Batch size check (`InvalidBatchSizeProgram`)
   - Duplicate program_id detection via insertion-sort dedup (`DuplicateProgramId`)
   - Existence check for each program_id (`ProgramAlreadyExists`)

2. **Registry-update loop** — iterates over items, writing `ProgramData`, multisig config, and emitting events for each. The `PROGRAM_REGISTRY` is committed **only once**, after the loop completes.

3. **Failure recovery** — if the loop fails partway through (panic from `enforce_token_allowlist`, or `Err` from an empty `program_id`), the Soroban runtime rolls back all storage writes from earlier iterations. No partially-initialized programs are left behind, and `PROGRAM_REGISTRY` is never updated.

### What the tests cover

| Test | Failure mechanism | Failure position | What's verified |
|------|-------------------|------------------|-----------------|
| `test_batch_init_atomicity_token_allowlist_mid_batch` | `enforce_token_allowlist` panic | After first item written | No `DataKey::Program` entries; `program_exists()` false |
| `test_batch_init_atomicity_empty_program_id_mid_batch` | Empty `program_id` guard returns `Err` | After first item written | `Err(InvalidBatchSizeProgram)`; first item rolled back |
| `test_batch_init_atomicity_valid_unlisted_valid` | `enforce_token_allowlist` panic (middle of 3 items) | After item 0; before item 2 | Items 0 and 2 also rolled back |
| `test_batch_init_atomicity_registry_unaffected_after_failure` | `enforce_token_allowlist` panic | After existing program | Pre-existing program and registry survive |

### Security notes

- The atomicity guarantee holds because Soroban's host environment rolls back storage changes on both panics and `Err` returns from public functions.
- Pre-validation prevents gas waste: duplicate/existence errors are detected before any storage write, so callers are not charged for partial writes.
- A malicious caller cannot cause partial program initialization by intentionally triggering a mid-loop failure.

---

## Pre-validation CPU Cost

Measured with `soroban-sdk 21.7.7` via
`cargo test test_batch_init_prevalidation_bench -- --nocapture`. Each row
captures the full `batch_initialize_programs` call — pre-validation *and* the
registry-update loop — for a batch of all-unique valid items.

| Batch size | CPU instructions | % of 100 M budget |
|------------|-----------------:|------------------:|
| 1 | 255,830 | 0.26 % |
| 10 | 1,810,684 | 1.81 % |
| 50 | 16,559,332 | 16.56 % |
| 100 | 69,727,396 | 69.73 % |

### Observations

- Pre-validation (dedup + existence) scales **O(n log n)** due to insertion sort.
- The registry-update loop scales **O(n)** — one `set`, one event per item.
- Cost is noticeably superlinear in practice: 50 items cost 16.6 M but 100 cost
  69.7 M, i.e. 4.2× the instructions for 2× the items. The insertion-sort dedup
  and the growing `PROGRAM_REGISTRY` vector both contribute.

### ⚠️ Headroom is ~30 %, not >90 %

An earlier revision of this document claimed "the current `MAX_BATCH_SIZE = 100`
leaves a >90 % safety margin below the 100 M ceiling". **That was wrong**, and
it survived only because the benchmark that produces these numbers was disabled
behind `#[cfg(any())]` in `lib.rs` and therefore had never been executed. The
figures above are its first real output.

A full-size `batch_initialize_programs` consumes ~70 % of the per-invocation
budget. Two caveats in opposite directions:

- These are **host-side Rust** figures. The SDK documents host CPU cost as
  *under*-estimating the WASM equivalent, so real on-chain cost may be higher —
  this is the pessimistic direction and the number to plan against.
- The 100 M ceiling is per invocation, and an operator only pays it by choosing
  a maximum-size batch. Smaller batches have proportionally more headroom.

The test now asserts against the real 100 M ceiling rather than a guessed
threshold, so a future change that pushes a full batch over the ceiling fails the
build. Lowering `MAX_BATCH_SIZE` for this entry point, or replacing the
insertion-sort dedup, is the follow-up this measurement argues for.

---

## Related files

| File | Purpose |
|------|---------|
| `contracts/program-escrow/src/lib.rs` | `batch_initialize_programs` implementation + doc comments |
| `contracts/program-escrow/src/test_batch_operations.rs` | Atomicity tests + inline benchmark |
| `contracts/program-escrow/src/gas_optimization.rs` | `deduplicate_program_ids` helper |
| `benchmarks/program-escrow/thresholds.json` | CI gate thresholds (provisional) |
| `docs/gas-optimization/batch-size-tuning.md` | `MAX_BATCH_SIZE` derivation for `batch_payout` |
| `docs/gas-optimization/batch-payout-benchmarks.md` | Benchmark collection process |
| `docs/batch-failure-semantics.md` | Failure model for **every** batch entry point in both contracts |

---

## Cross-reference

The separate [duplicate-check gas-optimization issue] will quantify the marginal cost of `deduplicate_program_ids` vs. alternative strategies (e.g. `Set`-based dedup or hash-map approaches). The table above provides a baseline for comparison.
