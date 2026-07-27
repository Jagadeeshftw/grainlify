# Program Escrow — Chaos Testing Harness for `batch_payout`

> Deterministic failure-injection harness for
> `contracts/program-escrow/src/tests/chaos_batch_payout_tests.rs`

This document describes the **chaos-testing harness** that randomly
composes `batch_payout` / `batch_payout_idempotent` batches and injects
simulated cross-contract call failures.  It exists so circuit-breaker
state, rollback semantics, and idempotency-key bookkeeping are validated
under **unpredictable failure interleavings**, not only the hand-written
scenarios in the deterministic suite.

---

## 1. Why a separate harness?

Existing program-escrow tests cover well-formed happy paths and a few
known error paths (`Funds Paused`, insufficient balance, mismatched
arrays, open circuit).  They do **not** systematically explore:

* a token transfer that reverts partway through a batch
* an unauthorized delegate mid-call
* a pause flipped while a batch is in flight
* randomized batch sizes / amount compositions across many seeds

The chaos harness fills that gap as a **dedicated, separately-invocable**
test target so CI and local debugging stay fast and reproducible.

---

## 2. How to run

```bash
# Chaos suite only (recommended day-to-day)
cargo test -p program-escrow chaos_batch_payout -- --nocapture

# Full package (includes chaos + property + unit tests)
cargo test -p program-escrow
```

Every assertion message includes the **scenario seed**.  To reproduce a
failure, either re-run the filtered test or construct
`ChaosScenario::generate(seed)` in a unit test.

The harness constructs each `Env` with
`EnvTestConfig { capture_snapshot_at_drop: false }` so the seeded sweep
does not flood `test_snapshots/` with dozens of ledger JSON files.

---

## 3. Architecture

```text
┌─────────────────────────────────────────────────────────────┐
│  chaos_batch_payout_tests.rs                                │
│    ChaosRng (LCG) → ChaosScenario { batch, amounts, fail }  │
│                         │                                   │
│                         ▼                                   │
│              apply_failure_preconditions()                  │
│                         │                                   │
│         ┌───────────────┼────────────────┐                  │
│         ▼               ▼                ▼                  │
│   try_batch_payout  try_*_idempotent  try_*_by              │
│         │                                                   │
│         ▼                                                   │
│   assert_invariants (balance / CB / idempotency)            │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼  (cfg(test) only)
┌─────────────────────────────────────────────────────────────┐
│  crate::chaos  (contracts/program-escrow/src/lib.rs)        │
│    configure_transfer_fail(at)                              │
│    configure_pause_mid_batch(at)                            │
│    tick_before_transfer(env, index)  ← called in loop       │
└─────────────────────────────────────────────────────────────┘
```

### Test-only injection hooks (`crate::chaos`)

| Mode | Behavior |
|------|----------|
| `MODE_NONE` | No-op (production-equivalent path) |
| `MODE_TRANSFER_FAIL` | Panic with `CHAOS_INJECTED_TRANSFER_FAILURE` at index N |
| `MODE_PAUSE_MID` | Set `release_paused` then panic with `Funds Paused` at index N |

Hooks are compiled **only** under `cfg(test)` and store state in
**temporary** storage so they never ship in the WASM `cdylib`.

Injection points sit immediately before each `token_client.transfer` in
`batch_payout_internal`, so a panic aborts the Soroban invocation and
the host rolls back all ledger writes from that call (atomic
all-or-nothing).

---

## 4. Injected failure catalogue

| Failure | How it is induced | Expected outcome |
|---------|-------------------|------------------|
| `None` | — | Success; balances debit correctly |
| `TransferFailAt(i)` | `chaos::configure_transfer_fail` | Panic + full rollback |
| `PauseMidBatch(i)` | `chaos::configure_pause_mid_batch` | Panic + full rollback |
| `UnauthorizedDelegate` | `batch_payout_by` with stranger | Rejected; no mutation |
| `CircuitOpen` | `record_failure` past threshold | Rejected; CB stays Open |
| `PausedEntry` | `set_paused(release=true)` | Rejected; no mutation |

Each scenario also randomly chooses whether to route through
`batch_payout_idempotent`.

---

## 5. Invariants (asserted after every run)

1. **No double-payment** — on failure, recipient token balances equal the
   pre-call snapshot; on success, they increase by exactly the scenario
   amounts.  Idempotent replay after success does not debit again.
2. **`remaining_balance >= 0`** always.
3. **Circuit breaker consistency** — rolled-back failures leave CB state
   identical to the pre-call snapshot; successful payouts never leave
   the breaker `Open`.
4. **Idempotency bookkeeping** — keys are consumed only after success.
   After a failed idempotent call the same key remains usable once the
   external precondition (pause / open circuit) is cleared.

---

## 6. Security notes

* Chaos hooks **must not** exist in release WASM.  They are gated with
  `#[cfg(test)]` at the module boundary and at the call site inside
  `batch_payout_internal`.
* Temporary storage keys (`ChaosMod`, `ChaosAt`, `ChaosCnt`) are never
  written by production entrypoints.
* The harness always calls `env.mock_all_auths()` so authorization
  failures under test are intentional (unauthorized-delegate case), not
  accidental missing mocks.
* Mid-batch pause injection deliberately mutates `PauseFlags` before
  panicking; because the panic aborts the invocation, that mutation is
  rolled back by the Soroban host — matching production atomicity.

---

## 7. Extending the harness

1. Add a new `InjectedFailure` variant and map it in `from_rng`.
2. Teach `apply_failure_preconditions` how to arm it.
3. Update the invariants table above.
4. Keep seeds deterministic — never read wall-clock or OS entropy.

Suggested local stress run:

```bash
# Temporarily raise SEEDS in chaos_batch_payout_seeded_sweep, then:
cargo test -p program-escrow chaos_batch_payout_seeded_sweep -- --nocapture
```
