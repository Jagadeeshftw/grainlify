# Invariant Catalog - Bounty Escrow Contract

## Overview

This document catalogs all invariants enforced by the bounty escrow contract. Invariants are properties that must always hold true for the contract to be in a valid state.

---

## Core Invariants (invariants.rs)

### INV-CORE-1: Non-Negative Amount
**Rule:** `escrow.amount >= 0`

**Rationale:** A bounty cannot have negative total value.

**Enforcement:** `assert_escrow()` panics if violated.

---

### INV-CORE-2: Non-Negative Remaining
**Rule:** `escrow.remaining_amount >= 0`

**Rationale:** Remaining balance cannot be negative.

**Enforcement:** `assert_escrow()` panics if violated.

---

### INV-CORE-3: Remaining ≤ Amount
**Rule:** `escrow.remaining_amount <= escrow.amount`

**Rationale:** Cannot have more remaining than was originally deposited.

**Enforcement:** `assert_escrow()` panics if violated.

---

### INV-CORE-4: Released State Consistency
**Rule:** `escrow.status == Released => escrow.remaining_amount == 0`

**Rationale:** A released escrow must have distributed all funds.

**Enforcement:** `assert_escrow()` panics if violated.

---

## Multi-Token Invariants (multitoken_invariants.rs)

### INV-MT-1: Per-Escrow Sanity
**Rule:** For every escrow:
- `amount > 0`
- `remaining_amount >= 0`
- `remaining_amount <= amount`
- `status == Released => remaining_amount == 0`
- `status == Refunded => remaining_amount == 0`

**Rationale:** Individual escrow sanity prevents state corruption.

**Enforcement:** `check_escrow_sanity()` returns false if violated.

---

### INV-MT-2: Aggregate-to-Ledger (Conservation of Value)
**Rule:** `sum(active_escrows.remaining_amount) == contract.token_balance(token)`

**Rationale:** Total tracked balance must match actual token holdings. This is the fundamental conservation law.

**Enforcement:** `check_all_invariants()` reports mismatch.

---

### INV-MT-3: Fee Separation
**Rule:** Fees are transferred immediately upon collection and are NOT included in escrow remaining amounts.

**Rationale:** Prevents fee misdirection or double-counting.

**Enforcement:** Structural (fees transferred at collection time).

---

### INV-MT-4: Refund Consistency
**Rule:** For every escrow: `sum(refund_history) <= (amount - remaining_amount)`

**Rationale:** Cannot claim more refunds than were actually processed.

**Enforcement:** `check_refund_consistency()` returns false if violated.

---

### INV-MT-5: Index Completeness
**Rule:** Every bounty_id in the EscrowIndex has a corresponding Escrow or AnonymousEscrow entry.

**Rationale:** Prevents index pollution and ghost entries.

**Enforcement:** `count_orphaned_index_entries()` detects violations.

---

## Invariant Checking

### Bounded Read-Only Audit

`BountyEscrowContract::verify_state(cursor)` audits the escrow index in pages of
at most 20 entries. Begin with `None`; pass each report's `next_cursor`
unchanged until it is `None`. Per-escrow sanity, refund-history consistency,
index completeness, and the aggregate active-liability/token-balance invariant
are reported in `VerificationReport`. `balance_matches` is populated only on
the final page. The endpoint does not modify contract state; run all pages
without intervening escrow mutations for a consistent whole-state result.

### Automatic Enforcement

Invariants are automatically checked after critical operations:

| Operation | Invariant Check | Module |
|-----------|-----------------|--------|
| `lock_funds()` | INV-MT-2 (aggregate) | `assert_after_lock()` |
| `release_funds()` | INV-MT-2 (aggregate) | `assert_after_disbursement()` |
| `refund()` | INV-MT-2 (aggregate) | `assert_after_disbursement()` |
| All state changes | INV-CORE-1 to INV-CORE-4 | `assert_escrow()` |

### View Functions

Query invariant status on-chain:

```rust
// Quick health check
let is_healthy: bool = contract.verify_all_invariants();

// Detailed report
let report = contract.check_invariants();
// Returns InvariantReport with:
// - healthy: bool
// - sum_remaining: i128
// - token_balance: i128
// - per_escrow_failures: u32
// - orphaned_index_entries: u32
// - refund_inconsistencies: u32
// - violations: Vec<String>
```

---

## Testing

### Test Coverage

| Module | Tests | Coverage |
|--------|-------|----------|
| invariants.rs | 15+ | 100% |
| multitoken_invariants.rs | 31 | 96% |
| Total | 46+ | 96%+ |

### Running Tests

```bash
cd contracts/bounty_escrow/contracts/escrow

# All invariant tests
cargo test --lib test_invariant

# Core invariants
cargo test --lib test_invariant_checker

# Multi-token invariants
cargo test --lib test_multitoken
cargo test --lib test_inv1  # Per-escrow sanity
cargo test --lib test_inv2  # Aggregate balance
cargo test --lib test_inv4  # Refund consistency
cargo test --lib test_inv5  # Index completeness
```

---

## Security Properties

### Attack Prevention

| Attack Vector | Prevented By | Result |
|--------------|--------------|--------|
| Fund inflation | INV-MT-1 + INV-MT-2 | Transaction aborts |
| Token extraction | INV-MT-2 | Mismatch detected |
| State rollback | INV-MT-2 | Atomic execution |
| Index corruption | INV-MT-5 | Orphan detection |
| Refund fraud | INV-MT-4 | Consistency check |

### Guarantee

**If any invariant is violated, the contract is in an invalid state and requires immediate investigation.**

---

## Monitoring

### On-Chain Monitoring

```rust
// Check health before critical operations
if !contract.verify_all_invariants() {
    // Alert / halt operations
}
```

### Off-Chain Monitoring

```typescript
// Watchtower service (run every 5 minutes)
async function monitorInvariants(contractId) {
    const report = await contract.checkInvariants();
    
    if (!report.healthy) {
        alert(`CRITICAL: Invariant violation!`);
        alert(`Violations: ${report.violation_count}`);
        alert(`Sum: ${report.sum_remaining}, Balance: ${report.token_balance}`);
    }
}
```

### Alert Thresholds

| Condition | Severity | Action |
|-----------|----------|--------|
| `healthy == false` | CRITICAL | Immediate investigation |
| `per_escrow_failures > 0` | HIGH | Review affected escrows |
| `orphaned_index_entries > 0` | HIGH | Index cleanup required |
| `refund_inconsistencies > 0` | MEDIUM | Audit refund history |

---

## Implementation Details

### Source Files

```
contracts/bounty_escrow/contracts/escrow/src/
├── invariants.rs                    (Core invariants, 71 LOC)
├── multitoken_invariants.rs         (Multi-token invariants, 325 LOC)
├── test_invariants.rs               (Core tests, 343 LOC)
└── test_multitoken_invariants.rs    (Multi-token tests, 850+ LOC)
```

### Gas Costs

| Check Type | Gas Cost | Usage |
|------------|----------|-------|
| Per-op assertion | 500-1000 | Hot paths (lock/release/refund) |
| Full invariant check | 5000-15000 | View functions only |

---

## References

- Issue #795: Bounty escrow invariants module
- Issue #591: Multi-token balance invariants
- README_MULTITOKEN_INVARIANTS.md: Integration guide
- SECURITY_NOTES.md: Threat analysis

---

*Last updated: 2026-03-25*
*Version: 1.0*
