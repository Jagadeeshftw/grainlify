# simulate_payout — View-Facade Read-Only Entrypoint

**Branch:** `feature/simulate-payout-view-facade`  
**File:** `contracts/view-facade/src/lib.rs`  
**Function:** `ViewFacade::simulate_payout`  
**Status:** ✅ 54 tests passing · 1 doc-test · 0 warnings

---

## Table of Contents

1. [Purpose](#1-purpose)
2. [Function Signature](#2-function-signature)
3. [Return Type — SimulationResult](#3-return-type)
4. [What Is Computed](#4-what-is-computed)
5. [Warning Catalogue](#5-warning-catalogue)
6. [Fee Arithmetic](#6-fee-arithmetic)
7. [Security Assumptions](#7-security-assumptions)
8. [Integration Guide (UI / Wallet)](#8-integration-guide)
9. [Test Coverage](#9-test-coverage)
10. [File Map](#10-file-map)
11. [How to Run Tests](#11-how-to-run-tests)
12. [Commit Message](#12-commit-message)

---

## 1. Purpose

When contributors or maintainers review a payout proposal in the Grainlify
UI, they need to see:

- Exactly how much each recipient will receive after fees
- Whether the escrow has sufficient balance
- Whether the circuit breaker or any other condition would block the real transaction

Previously, this required submitting a real (mutating) transaction and
inspecting the revert reason — expensive, slow, and destructive.

`simulate_payout` provides a **safe, read-only dry-run** that computes all
the same values as the real payout would, using the live `ProgramData` and
`FeeConfig` stored on-chain, without writing to any storage key or
transferring any tokens.

---

## 2. Function Signature

```rust
/// Simulates a batch payout without mutating any on-chain state.
///
/// # Parameters
/// - `program_id`   Identifies which program's FeeConfig and balance to use.
/// - `recipients`   Vec of (address, gross_amount) pairs to simulate.
///
/// # Returns
/// A `SimulationResult` with per-recipient net amounts, total fees,
/// total net, effective rate, and advisory warnings.
pub fn simulate_payout(
    &self,
    program_id: &str,
    recipients: Vec<Recipient>,
) -> SimulationResult
```

The function signature maps directly to the `#[contractimpl]` entrypoint
in a Soroban deployment. In the standalone Rust implementation used for
testing, `&self` borrows an immutable `Storage` reference, enforcing the
read-only contract at the type level.

---

## 3. Return Type

```rust
pub struct SimulationResult {
    /// Per-recipient breakdown: address, net_amount, and fee for each entry.
    pub net_amounts: Vec<NetEntry>,

    /// Sum of all fees across all recipients.
    pub total_fees: u128,

    /// Sum of all net payouts across all recipients.
    pub total_net: u128,

    /// Weighted-average fee rate in basis points (advisory / display only).
    pub effective_rate_bp: u32,

    /// Non-fatal advisory messages. Empty = no issues detected.
    pub warnings: Vec<Warning>,
}

pub struct NetEntry {
    pub address:    Address,
    pub net_amount: u128,   // what the recipient actually receives
    pub fee:        u128,   // amount deducted as platform fee
}
```

### Invariants

For every well-formed result:

```
net_amounts[i].fee + net_amounts[i].net_amount == gross_amounts[i]
total_fees == Σ net_amounts[i].fee
total_net  == Σ net_amounts[i].net_amount
total_fees + total_net == Σ gross_amounts[i]
```

These invariants hold even when warnings are present. The result is
always returned; warnings are advisory and do not abort the simulation.

---

## 4. What Is Computed

The function executes these steps in order, all in memory, with no storage writes:

```
1. Empty recipient list? → emit Warning::EmptyRecipientList, return early.

2. Circuit breaker open? → emit Warning::CircuitBreakerOpen (continue).

3. Program inactive?     → emit Warning::ProgramInactive (continue).

4. Resolve FeeConfig:
   - If no FeeConfig is stored → default to 0 bp (zero fee).

5. Detect duplicate addresses → emit Warning::DuplicateAddress for each.

6. For each recipient:
   a. Gross amount == 0?
      → emit Warning::ZeroAmountRecipient, push entry with fee=0 net=0.
   b. resolve_fee_rate(fee_config, gross_amount) → rate_bp (capped at 1000 bp).
   c. compute_fee(gross_amount, rate_bp) → fee  (floor division, overflow-safe).
   d. net = gross_amount − fee.
   e. net == 0? → emit Warning::NetAmountZero.
   f. Accumulate total_fees, total_net, total_gross.

7. total_gross > remaining_balance?
   → emit Warning::InsufficientBalance { required, available }.

8. Compute weighted effective_rate_bp (saturating arithmetic).

9. Return SimulationResult.
```

---

## 5. Warning Catalogue

| Warning | Trigger | Severity |
|---------|---------|----------|
| `CircuitBreakerOpen` | `CIRCUIT_OPEN` key is true | Real payout blocked |
| `ProgramInactive` | Program's `is_active == false` | Real payout would be rejected |
| `InsufficientBalance` | Total gross > remaining balance | Real payout would underflow escrow |
| `EmptyRecipientList` | `recipients.len() == 0` | No simulation performed |
| `ZeroAmountRecipient` | A recipient has `gross_amount == 0` | Recipient receives nothing |
| `NetAmountZero` | Fee floors net payout to 0 | Recipient receives nothing (fees too high) |
| `DuplicateAddress` | Same address appears more than once | Double-payout risk in real transaction |

All warnings implement `Display`:

```rust
let w = Warning::CircuitBreakerOpen;
println!("{}", w);
// → "CIRCUIT_BREAKER_OPEN: real payouts are currently suspended"
```

---

## 6. Fee Arithmetic

### Bracket resolution

```rust
fn resolve_fee_rate(config: &FeeConfig, gross_amount: u128) -> u32 {
    // 1. Walk brackets in order; first match wins.
    // 2. If no bracket matches, fall back to default_rate_bp.
    // 3. Cap at MAX_FEE_RATE_BP = 1000 regardless of stored value.
}
```

Bracket example (configured in `FeeConfig`):

| Bracket | Ceiling | Rate |
|---------|---------|------|
| 1 | ≤ 10,000 | 100 bp (1 %) |
| 2 | ≤ 50,000 | 200 bp (2 %) |
| 3 | unlimited | 300 bp (3 %) |

### Fee computation

```
fee = floor(gross_amount × rate_bp / 10_000)
net = gross_amount − fee
```

Implemented via the split-division algorithm to avoid `u128` overflow:

```rust
let q = gross_amount / 10_000;
let r = gross_amount % 10_000;
fee = q * rate_bp + r * rate_bp / 10_000;
```

**Overflow proof:**
- `q * rate_bp` ≤ `(u128::MAX / 10_000) × 1_000` = `u128::MAX / 10` ✓
- `r * rate_bp` ≤ `9_999 × 1_000` = `9_999_000` ✓

### Fee ceiling

The fee rate is always capped at `MAX_FEE_RATE_BP = 1000` (10 %), even if
the stored `FeeConfig` contains a higher value.  This prevents a corrupted
or maliciously set config from extracting more than 10 % of any payout.

---

## 7. Security Assumptions

| Assumption | Enforcement |
|------------|------------|
| No state mutation | `ViewFacade` holds `&Storage` (immutable borrow); no `set_*` called |
| No token transfer | No `transfer`, `approve`, or escrow entrypoints called |
| No authentication required | All entrypoints permissionless |
| Fee rate hard-capped at 10 % | `resolve_fee_rate` always applies `.min(MAX_FEE_RATE_BP)` |
| Overflow-safe arithmetic | Split-division in `compute_fee`; saturating_add for weighted rate |
| Fee ≤ gross per recipient | `compute_fee(amount, rate ≤ 1000) ≤ amount` proven by cap |
| Subtraction never underflows | `net = gross - fee` safe because `fee ≤ gross` |
| Circuit breaker only advisory | Breaker state read but does not abort simulation |
| Conservation invariant | `fee + net == gross` for every recipient, every call |
| Idempotent | Two calls with identical inputs return identical results |

---

## 8. Integration Guide (UI / Wallet)

### Typical wallet preview flow

```typescript
// 1. Fetch live simulation from the view-facade contract
const result = await viewFacade.simulate_payout("prog-1", [
  { address: "GABC...", gross_amount: 10_000n },
  { address: "GXYZ...", gross_amount: 20_000n },
]);

// 2. Show per-recipient breakdown
for (const entry of result.net_amounts) {
  console.log(`${entry.address}: receives ${entry.net_amount}, fee=${entry.fee}`);
}

// 3. Surface warnings prominently
for (const warning of result.warnings) {
  if (warning === "CircuitBreakerOpen") {
    showBanner("⚠️ Payouts are currently suspended.");
  }
  if (warning.type === "InsufficientBalance") {
    showBanner(`⚠️ Insufficient balance: need ${warning.required}, have ${warning.available}`);
  }
}

// 4. Only enable the "Send Payout" button if no blocking warnings
const isBlocked = result.warnings.some(w =>
  w === "CircuitBreakerOpen" || w === "ProgramInactive" || w.type === "InsufficientBalance"
);
document.getElementById("send-btn").disabled = isBlocked;
```

### No-state-mutation guarantee

Because `simulate_payout` is a view function, it can be called:
- Without a signed transaction
- Without gas payment (in supported RPC modes)
- As many times as needed during the review flow
- Concurrently from multiple users

---

## 9. Test Coverage

**File:** `contracts/view-facade/src/tests/simulate_payout_tests.rs`

| # | Test | Category |
|---|------|----------|
| 1–3 | Single recipient: net, zero fee, no warnings | Happy path |
| 4–6 | Multiple recipients: totals, individuals, order | Happy path |
| 7–10 | Fee floors, max rate, missing config, rate cap | Flat fee |
| 11–15 | Bracket tier 1/2/3, mixed tiers, fallback | Bracket fee |
| 16–18 | u128::MAX no overflow, conservation | Overflow safety |
| 19–20 | Circuit breaker warning, no abort | Circuit breaker |
| 21–22 | Program inactive warning, result still returned | Program state |
| 23–24 | Insufficient balance warning, exact match OK | Balance check |
| 25–26 | Zero amount warning, zero fee+net | Zero amount |
| 27–28 | Net-zero warning path, zero-gross path | Net zero |
| 29–30 | Empty list warning, zero totals | Empty list |
| 31–33 | Duplicate warning, both entries processed, triple | Duplicates |
| 34–36 | Balance unchanged, breaker unchanged, idempotent | Read-only |
| 37–40 | get_program, get_fee_config, is_circuit_open | Other queries |
| 41–46 | resolve_fee_rate: no brackets, tier 1/2/3, default, cap | Rate helper |
| 47–51 | compute_fee: zero rate, zero amount, exact, floor, MAX | Fee helper |
| 52–54 | Per-recipient conservation, total conservation, zero effective rate | Invariants |

**Result: 54/54 passing · 1 doc-test passing · 0 warnings**

---

## 10. File Map

```
contracts/view-facade/
├── Cargo.toml                                       ← [profile.test] overflow-checks=true
└── src/
    ├── lib.rs                                       ← ViewFacade, simulate_payout, all types
    └── tests/
        ├── mod.rs
        └── simulate_payout_tests.rs                 ← 54 unit tests

docs/
└── simulate-payout.md                               ← this document
```

---

## 11. How to Run Tests

```bash
# All tests
cargo test -p view-facade

# Only simulate_payout tests
cargo test -p view-facade simulate_payout

# Verbose output
cargo test -p view-facade -- --nocapture

# Zero warnings check
cargo test -p view-facade 2>&1 | grep -E "^warning|^error"
# must return nothing
```

Expected:
```
running 54 tests
test tests::simulate_payout_tests::... ok  (×54)
test result: ok. 54 passed; 0 failed

Doc-tests view-facade
running 1 test
test src/lib.rs - ViewFacade::simulate_payout ... ok
test result: ok. 1 passed; 0 failed
```

---

## 12. Commit Message

```
feat: add simulate_payout read-only entrypoint to view-facade

Adds ViewFacade::simulate_payout to contracts/view-facade/src/lib.rs.

What it does:
- Computes fee deductions and net amounts for a Vec<Recipient> using
  the live ProgramData and FeeConfig without mutating any storage key.
- Supports flat-rate and graduated bracket fee schedules.
- Returns SimulationResult { net_amounts, total_fees, total_net,
  effective_rate_bp, warnings }.
- Emits warnings (non-aborting) for: circuit breaker open, program
  inactive, insufficient balance, zero amount, net-zero amount,
  empty recipient list, duplicate addresses.
- Circuit-breaker state surfaced in warnings even when open.

Security:
- ViewFacade holds &Storage (immutable borrow) — no set_* can be called.
- Fee rate hard-capped at MAX_FEE_RATE_BP = 1000 (10 %).
- compute_fee uses split-division to avoid u128 intermediate overflow.
- Weighted effective rate uses saturating_add at MAX values.
- Conservation invariant: fee + net == gross for every recipient.

Tests: 54 unit tests + 1 doc-test, 0 warnings.
Docs:  docs/simulate-payout.md
```