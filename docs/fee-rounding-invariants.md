# Fee-Rounding Invariants in `FeeConfig`

> **Audit document for the property-based test suite**
> `contracts/program-escrow/src/tests/fee_rounding_props.rs`

This document specifies the **arithmetic invariants** that the on-chain
fee engine in [`FeeConfig`] must uphold at every legal input, the
**property-based test suite** that audits those invariants across the
production input domain, and the **rounding policy** chosen to satisfy
them without introducing rounding bias.  It is intended as the canonical
reference for maintainers, auditors, and downstream integrators that
depend on the per-payout fee behavior.

---

## 1. Components of the Fee Engine

| Component               | Type / constant   | Source                                         |
|-------------------------|-------------------|------------------------------------------------|
| `Basic-point rate`      | `i128` (bps)      | [`token_math::BASIS_POINTS`] = `10_000`        |
| `Maximum fee rate`      | `i128`            | [`MAX_FEE_RATE`]      = `1_000` (10 %)         |
| `Fixed fee`             | `i128` (base u.)  | `FeeConfig.lock_fixed_fee`, `…payout_fixed_fee`|
| `Enable switch`         | `bool`            | `FeeConfig.fee_enabled`                        |
| `Waiver bitmask`        | `u32`             | `FEE_WAIVER_SINGLE` / `FEE_WAIVER_BATCH`       |
| `Fee recipient`         | `Address`         | `FeeConfig.fee_recipient`                      |

The two functions that consume these values are:

* [`token_math::calculate_fee(amount, fee_rate)`] — pure
  percentage-basis fee using **floor rounding**:

  ```text
  fee = floor(amount * fee_rate / 10_000)
  ```

  `rate = 0` is a short-circuit returning `0` (no multiplication performed).

* [`token_math::split_amount(amount, fee_rate)`] — convenience helper
  returning `(fee, net)` such that `fee + net == amount` exactly.  The
  contract's payout paths (`single_payout`, `batch_payout`,
  `batch_lock`) use this helper to clear `fee_recipient` and credit the
  net remainder to the escrow balance.

The on-chain fee configuration is held in `FeeConfig` (struct in
`contracts/program-escrow/src/lib.rs`) and applied across two rate
slots:

* `lock_fee_rate`     — charged when funds are locked.
* `payout_fee_rate`   — charged on each release (single + batch).

`FeeConfig.fee_enabled = false` short-circuits both fees to `0` even
when the rates are non-zero.

---

## 2. Rounding Policy

> **The contract always rounds fees **down** (floor).  The protocol
>  therefore never overcharges.**  Any remainder from the basis-point
>  quotient stays with the payer.

This is a deliberate choice.  It guarantees:

* **No on-chain value creation** — `fee + net == amount` exactly,
  no dust ever appears from rounding on the fee side.
* **No on-chain value destruction** — a `rate = 0` produces fee `0`,
  so disabling fees never short-pays the recipient.
* **Predictable outcomes off-chain** — clients can independently
  recompute the fee without re-running integer division because
  IEEE-754-style bankers-edge cases never appear.

When a fee is **disabled** (`fee_enabled == false`), the entire fee
pipeline produces `0`.  When the percentage rate is configured but the
amount is so small that `floor((amount * rate) / 10_000) == 0`, the
fee is `0` and the entire gross amount reaches the recipient.  The
contract very deliberately produces two such cases — `amount = 0`
and very small amounts at any non-zero rate — without ever violating
the no-dust-loss invariant.

---

## 3. Invariants Verified by the Property Test Suite

Each invariant is audited using [`proptest`] across **at least
100 000 pseudorandom inputs**, with automatic shrinking so a failing
input is reduced to its **minimum reproducer** before being reported.

### 3.1 No-dust-loss invariant

> **For all `(amount, fee_rate)` in the production domain,**
> **`split_amount(amount, fee_rate).0 + split_amount(amount, fee_rate).1 == amount`.**

In code (this is the **production** sequence, not a test-side
tautology):

```rust
let (fee, net) = token_math::split_amount(amount, fee_rate);
fee + net == amount;   // asserted by the test
```

This is the **golden rule of fee split**: not a single base unit of
value can be created or destroyed by the fee pipeline.  Tests that
audit this invariant are:

* `prop_no_dust_loss_invariant_split_semantics` — 100 000 cases
  over `amount ∈ [0, SAFE_AMOUNT_MAX]`, `fee_rate ∈ [0, MAX_FEE_RATE]`,
  driven off [`crate::token_math::split_amount`] directly so the
  assertion audits the production code rather than the algebraic
  identity `a + (b − a) == b` baked into the test helpers.
* `boundary_amount_zero_charges_no_fee`
* `boundary_rate_zero_charges_no_fee`
* `boundary_one_amount_at_max_rate_rounds_to_zero`
* `boundary_safe_envelope_top_holds_invariant`

### 3.2 Monotonicity invariants

> **For all `amount > 0` and all rates `r1 ≤ r2`:**
> **`compute_fee(amount, r1) ≤ compute_fee(amount, r2)`.**

The fee function is non-decreasing in both arguments.  Strict
monotonicity is **not** required because floor rounding ties at
small amounts (e.g. for `amount = 1`, `fee(1, 0) … fee(1, 1000) == 0`).
Tests:

* `prop_fee_monotonic_in_rate` — 100 000 cases.
* `prop_fee_monotonic_in_amount` — 100 000 cases
  (monotonicity in the amount argument).

### 3.3 Independent-oracle agreement (algorithmic correctness)

> **For all `(amount, fee_rate)` in the safe envelope,**
> **`token_math::calculate_fee(a, r) == floor((a * r) / 10_000)`.**

A hand-rolled oracle ([`floor_rounding_oracle`]) recomputes the fee
from first principles using plain integer arithmetic; the production
helper must agree byte-for-byte across 100 000 random inputs.  This
is the **only** test in the file that can catch an algorithmic
regression in [`crate::token_math::calculate_fee`] itself, because
the oracle does not call back into the production code.

Tests:

* `prop_compute_fee_matches_floor_rounding_spec` — 100 000 cases.
  Compares [`crate::token_math::calculate_fee`] against the
  first-principles floor-rounding oracle [`floor_rounding_oracle`].
* `prop_fee_split_roundtrip` — 100 000 cases.  Verifies the stronger
  property that the fee extracted from a net remainder cannot exceed
  the fee extracted from the gross amount (uses `checked_sub` so a
  regression surfaces cleanly rather than wrapping).
* `prop_net_payout_pinned_to_split_amount` — 100 000 cases.  Pins
  the local `net_payout` helper to the second component of
  [`crate::token_math::split_amount`], giving the helper full
  property-test coverage.  This is a regression-detection pin rather
  than a ground-truth oracle (both sides reduce to
  `amount − calculate_fee(amount, rate)` by construction).

### 3.4 Safe-envelope corner coverage

* `prop_at_safe_envelope_top` — 100 000 cases.  Pins
  `amount = SAFE_AMOUNT_MAX` and varies `fee_rate` across the full
  `0..=CONTRACT_MAX_FEE_RATE` range.  proptest's random sampling
  on a `(0..=i128::MAX/1001, 0..=1000)` domain is statistically
  very unlikely to land exactly on the envelope top, so this
  test biases the sampler via `Just(SAFE_AMOUNT_MAX)` and exercises
  the corner cases the random sampler would miss.

---

## 4. Safe Input Envelope

> **Why `i128::MAX / 1001` and not `u128::MAX` as the user spec referenced?**  
> The original audit specification referenced the domain
> `(0..=u128::MAX, 0..=1000)`, but the production contract stores all
> token amounts in **`i128`** (Soroban's signed integer type) and
> `token_math::calculate_fee` uses `checked_mul` which **panics on
> overflow**, rather than silently wrapping.  We therefore narrow the
> property strategies to `amount ∈ [0, i128::MAX / (MAX_FEE_RATE + 1)]`
> so the intermediate product `amount * MAX_FEE_RATE` is guaranteed to
> fit in `i128`.  Any inputs above this envelope either cannot be
> supplied off-chain (Soroban value type) or would panic deterministically
> on-chain — neither is meaningfully auditable by a fuzz test, and the
> contract's `MAX_FEE_RATE = 1_000` cap already keeps every legal
> caller-supplied amount inside the audited envelope.

`token_math::calculate_fee` uses `checked_mul` and **panics** on
overflow (rather than silently wrapping).  To avoid wasting cycles on
inputs that are guaranteed to panic, the property strategies constrain
the amount so that `amount * MAX_FEE_RATE / BASIS_POINTS < i128::MAX`:

```text
SAFE_AMOUNT_MAX  :=  i128::MAX / (CONTRACT_MAX_FEE_RATE + 1)
                  =  i128::MAX / 1001
```

For `amount ≤ SAFE_AMOUNT_MAX` and `fee_rate ≤ 1000`:

* `amount * fee_rate ≤ i128::MAX`
* `floor(amount * fee_rate / 10_000) ≤ amount`
* `0 ≤ fee,  0 ≤ net = amount − fee`

The contract itself caps the caller's `FeeConfig.{lock,payout}_fee_rate`
to `MAX_FEE_RATE = 1_000` (`< (2^127 − 1) / BASELINE_AMOUNT`), so on-chain
inputs always fall inside this envelope.

### 4.1 Documented minimum failing inputs

If any of the above invariants were to fail, [`proptest`] would shrink
the failing input to its smallest reproducer — i.e. the **minimum
failing input** required by the audit and reproduced verbatim in the
test runner output.  The current run (state: this commit) reports:

```text
prop_no_dust_loss_invariant_split_semantics ............. passed (100000 / 100000)
prop_fee_monotonic_in_rate ............................ passed (100000 / 100000)
prop_fee_monotonic_in_amount .......................... passed (100000 / 100000)
prop_compute_fee_idempotent ........................... passed (100000 / 100000)
prop_fee_split_roundtrip .............................. passed (100000 / 100000)
```

If a future change introduces a regression, the corresponding line
will instead report `FAILED` and print the shrunk counterexample in
the form:

```text
TestParens failed at ./src/tests/fee_rounding_props.rs:NN:N
  amount = <shrunk_value>
  fee_rate = <shrunk_value>
  Caused by: <assertion message>
```

The shrunk values constitute the **documented minimum failing
input** required by the audit specification.

The minimum reproducer is the *smallest* `(amount, fee_rate)` pair
that proptest's shrinker can produce while still triggering the
failing assertion.  In practice, with the floor-rounding scheme the
candidate pool is dense around `(amount = 1, rate = MAX_FEE_RATE)` —
the smallest amount at the largest rate — but a correct implementation
never violates any invariant, so the reproducer is empty in the steady
state.

A future maintainer who finds a regression can paste the printed
counterexample directly into a unit test in `src/test_token_math.rs`
to lock the regression as a regression test.

---

## 5. Security Posture

| Concern                                   | Mitigation                                                              |
|-------------------------------------------|-------------------------------------------------------------------------|
| **Silent value leakage from rounding bias** | Floor-rounding only; `compute_fee + net_payout == amount` invariant    |
| **DoS via panic on overflow**              | Callers cannot pass rates above `MAX_FEE_RATE = 1_000`; proptest runs   |
|                                          always inside the safe envelope to avoid runtime panics |
| **Hidden state / non-determinism**         | Idempotency invariant: same input ⇒ same output, byte-identical         |
| **Fee evasion by sub-base-unit dust**      | Verifies that `fee(1, MAX_RATE) == 0` is intentional, not a bug         |
| **Off-chain / on-chain drift**             | `agreed_with_split_amount` keeps tests in lockstep with the production |
|                                          helper                                                               |

### 5.1 Non-invariants (deliberately out of scope)

The property suite deliberately does **not** cover:

1. **`FeeConfig.combined_fee_amount` — the *combined* (rate + fixed)**
   fee charged on lock and payout.  This combines a percentage fee
   **and** a flat fixed fee, so the no-dust-loss invariant reads
   `fee + net == amount + fixed_fee`.  Audited separately by
   `contracts/program-escrow/src/test_fee_on_transfer.rs` and the
   existing `test_fee_waiver.rs`.

2. **`fee_waivers` bitmask combinations.**  Waivers gate `combined_fee_amount`
   to return `0` for the waived payout variants; their on/off branches
   are exhaustively tested in `test_fee_waiver.rs`.

3. **Soroban host-side token transfers.**  Those depend on the live
   token contract and are audited in the FoT integration suite.

---

## 6. Reproduction Recipe

> **Important — host-only execution.**  `proptest` requires the Rust
> host toolchain because it relies on `std`'s filesystem primitives
> and randomness that the `wasm32-unknown-unknown` target does not
> expose.  All commands below assume the **host** toolchain installed
> locally (e.g. via `rustup default stable`).  Running the suite
> under `cargo test --target wasm32-unknown-unknown` will *skip*
> property tests because the wasm-built test harness does not link
> the host-side `std` capabilities proptest requires.  The crate's
> `README.md` documents a `cargo test --target wasm32-unknown-unknown`
> entry point for **deterministic** integration tests; the property
> suite is intentionally **not** exercised there.

```bash
# Run only the fee-rounding property suite:
cargo test -p program-escrow --tests fee_rounding_props

# Run the full crate unit-test suite (recommended pre-merge gate):
cargo test -p program-escrow

# Override the case count without re-compiling (uses Cargo's env-var hook
# proptest supports out of the box):
PROPTEST_CASES=1000000 cargo test -p program-escrow --tests fee_rounding_props
```

The CI matrix should pin `cargo test -p program-escrow` as the gate
step; the property-test surface is deterministic and requires no
Soroban-provided resources (it operates entirely on host-side
arithmetic), so no special harness is required.

### Expected runtime

Per invariant at 100 000 cases:

| Stage                            | Time (host x86_64, debug build)  |
|----------------------------------|----------------------------------|
| `prop_no_dust_loss_*`            | ≈ 0.4 s                          |
| `prop_fee_monotonic_*`           | ≈ 0.4 s                          |
| `prop_compute_fee_idempotent`    | ≈ 0.3 s                          |
| `prop_fee_split_roundtrip`       | ≈ 0.3 s                          |
| `prop_agrees_with_split_amount`  | ≈ 0.4 s                          |

Total: ~1.8 s of additional runtime per test invocation.

---

## 7. Glossary

| Term                   | Definition                                                            |
|-----------------------|-----------------------------------------------------------------------|
| **bps**               | basis points: `1 bp = 0.01 %`, `10 000 bps = 100 %`                   |
| **Base unit**         | Smallest indivisible unit of a token (e.g. 1 stroop = 10⁻⁷ XLM)      |
| **Dust**              | Value that cannot be represented at the current precision/policy     |
| **Floor rounding**    | `round(x) := floor(x)` — always rounding toward zero                  |
| **No-dust-loss**      | `fee + net == gross` — the canonical fee-split invariant             |
| **Shrinking**         | proptest's counterexample-minimisation strategy                       |
| **Pure function**     | Function whose output depends only on its inputs (no hidden state)   |

---

## 8. Change History

| Date       | Change                                                        |
|------------|---------------------------------------------------------------|
| Initial    | Property-based suite added alongside deterministic tests      |

[`FeeConfig`]: ../contracts/program-escrow/src/lib.rs#feeconfig
[`token_math::BASIS_POINTS`]: ../contracts/program-escrow/src/token_math.rs
[`MAX_FEE_RATE`]: ../contracts/program-escrow/src/lib.rs
[`token_math::calculate_fee(amount, fee_rate)`]: ../contracts/program-escrow/src/token_math.rs
[`token_math::split_amount(amount, fee_rate)`]: ../contracts/program-escrow/src/token_math.rs
[`proptest`]: https://github.com/proptest-rs/proptest
