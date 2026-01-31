# Contract Invariants Documentation

## Overview

This document formally defines and documents the critical invariants that must hold true across all states of the Grainlify bounty escrow smart contracts. These invariants are essential for maintaining contract correctness, security, and economic soundness.

## Table of Contents

1. [Bounty Escrow Contract Invariants](#bounty-escrow-contract-invariants)
2. [Program Escrow Contract Invariants](#program-escrow-contract-invariants)
3. [Invariant Testing Approach](#invariant-testing-approach)
4. [Violation Detection](#violation-detection)

---

## Bounty Escrow Contract Invariants

### I1: Balance Consistency Invariant

**Statement**: The sum of all locked escrow amounts must never exceed the actual token balance held by the contract.

```
∀ bounties: Σ(escrow[bounty_id].amount where status = Locked) ≤ contract_token_balance
```

**Rationale**: Prevents over-commitment of funds and ensures all locked funds are actually backed by tokens in the contract.

**Violation Consequences**: Could lead to inability to release/refund legitimate escrows, contract insolvency.

**Check Frequency**: After every `lock_funds`, `release_funds`, `refund`, and `batch_*` operation.

---

### I2: Status Transition Invariant

**Statement**: Escrow status transitions must follow valid state machine rules. Once in a final state (Released or Refunded), no further transitions are allowed.

**Valid Transitions**:
- `None → Locked`
- `Locked → Released` (final)
- `Locked → PartiallyRefunded`
- `Locked → Refunded` (final)
- `PartiallyRefunded → PartiallyRefunded` (via additional partial refunds)
- `PartiallyRefunded → Refunded` (final, when remaining_amount reaches 0)

**Invalid Transitions**:
- `Released → *` (any transition from Released)
- `Refunded → *` (any transition from Refunded)
- `* → Locked` (cannot re-lock)

**Rationale**: Enforces immutability of final states, preventing double-spending or state manipulation.

**Violation Consequences**: Double-release, double-refund, or unauthorized state changes.

**Check Frequency**: Before and after every state-changing operation.

---

### I3: No Double-Release/Refund Invariant

**Statement**: Each bounty can only be released OR refunded, never both. Once released or fully refunded, the escrow is permanently finalized.

```
∀ bounty_id: (status = Released ⟹ never refunded) ∧ (status = Refunded ⟹ never released)
```

**Rationale**: Prevents double-spending of escrowed funds.

**Violation Consequences**: Economic loss, fund theft, contract drain.

**Check Frequency**: After every `release_funds` and `refund` operation.

---

### I4: Amount Non-Negativity Invariant

**Statement**: All amounts (locked, remaining, refunded) must be non-negative at all times.

```
∀ bounty_id: 
  escrow.amount ≥ 0 ∧
  escrow.remaining_amount ≥ 0 ∧
  ∀ refund ∈ refund_history: refund.amount ≥ 0
```

**Rationale**: Negative amounts are economically meaningless and could indicate arithmetic underflow.

**Violation Consequences**: Integer underflow, incorrect balance tracking.

**Check Frequency**: After every amount-modifying operation.

---

### I5: Remaining Amount Consistency Invariant

**Statement**: For any escrow, the remaining amount must equal the original amount minus the sum of all refunds.

```
∀ bounty_id with PartiallyRefunded or Refunded status:
  escrow.remaining_amount = escrow.amount - Σ(refund_history[].amount)
```

**Rationale**: Ensures refund tracking is accurate and prevents over-refunding.

**Violation Consequences**: Ability to refund more than was locked, fund drain.

**Check Frequency**: After every refund operation.

---

### I6: Refunded Amount Bounds Invariant

**Statement**: The total refunded amount for any bounty cannot exceed the original locked amount.

```
∀ bounty_id: Σ(refund_history[].amount) ≤ escrow.amount
```

**Rationale**: Prevents over-refunding beyond what was initially locked.

**Violation Consequences**: Contract fund drain, economic exploit.

**Check Frequency**: After every refund operation.

---

### I7: Deadline Validity Invariant

**Statement**: Deadlines must be in the future at the time of locking, and refunds can only occur after the deadline (unless admin-approved).

```
At lock_funds: deadline > current_timestamp
At refund (non-approved): current_timestamp ≥ escrow.deadline
```

**Rationale**: Ensures time-based protections work correctly.

**Violation Consequences**: Immediate refunds, bypassing escrow period.

**Check Frequency**: At `lock_funds` and `refund` operations.

---

### I8: Unique Bounty ID Invariant

**Statement**: Each bounty ID can only be used once. No duplicate escrow records can exist.

```
∀ bounty_id₁, bounty_id₂: bounty_id₁ = bounty_id₂ ⟹ escrow₁ = escrow₂
```

**Rationale**: Prevents overwriting existing escrows and state confusion.

**Violation Consequences**: Lost escrow data, fund misallocation.

**Check Frequency**: Before every `lock_funds` operation.

---

### I9: Released Funds Finality Invariant

**Statement**: When an escrow is marked as Released, the remaining_amount must be 0.

```
∀ bounty_id: status = Released ⟹ remaining_amount = 0
```

**Rationale**: Ensures complete fund transfer on release.

**Violation Consequences**: Partial releases not reflected in state, accounting errors.

**Check Frequency**: After every `release_funds` operation.

---

### I10: Refund History Monotonicity Invariant

**Statement**: Refund history is append-only. Once a refund is recorded, it cannot be modified or removed.

```
∀ bounty_id, t₁ < t₂: 
  len(refund_history@t₁) ≤ len(refund_history@t₂) ∧
  ∀ i ∈ [0, len(refund_history@t₁)): refund_history@t₁[i] = refund_history@t₂[i]
```

**Rationale**: Provides immutable audit trail of all refunds.

**Violation Consequences**: Loss of audit trail, potential fraud.

**Check Frequency**: After every refund operation.

---

### I11: Fee Calculation Correctness Invariant

**Statement**: When fees are enabled, the net amount transferred plus fee must equal the gross amount, and fees must be within configured limits.

```
When fee_enabled:
  net_amount + fee_amount = gross_amount ∧
  fee_amount = (gross_amount × fee_rate) / BASIS_POINTS ∧
  0 ≤ fee_rate ≤ MAX_FEE_RATE
```

**Rationale**: Ensures fee calculations are correct and don't exceed limits.

**Violation Consequences**: Incorrect fee collection, user fund loss.

**Check Frequency**: After fee-enabled `lock_funds` and `release_funds` operations.

---

### I12: Batch Operation Atomicity Invariant

**Statement**: Batch operations must be all-or-nothing. Either all items succeed, or none do.

```
∀ batch_operation:
  (∀ item ∈ batch: success(item)) ∨ (∀ item ∈ batch: ¬success(item))
```

**Rationale**: Prevents partial execution that could leave contract in inconsistent state.

**Violation Consequences**: Partial batch execution, state inconsistency.

**Check Frequency**: Throughout batch operation execution.

---

## Program Escrow Contract Invariants

### PI1: Total Locked vs Balance Invariant

**Statement**: For each program, the sum of all scheduled and unreleased amounts must not exceed the remaining balance.

```
∀ program_id:
  Σ(pending_schedules[].amount) ≤ program.remaining_balance ≤ contract_balance
```

**Rationale**: Prevents over-scheduling of funds.

**Violation Consequences**: Inability to fulfill scheduled releases.

**Check Frequency**: After `lock_program_funds`, `create_program_release_schedule`, and payout operations.

---

### PI2: Remaining Balance Consistency Invariant

**Statement**: Remaining balance equals total funds minus sum of all payouts.

```
∀ program_id:
  program.remaining_balance = program.total_funds - Σ(payout_history[].amount)
```

**Rationale**: Ensures accurate balance tracking.

**Violation Consequences**: Incorrect balance accounting, over-spending.

**Check Frequency**: After every payout operation.

---

### PI3: Payout History Integrity Invariant

**Statement**: Total payouts cannot exceed total locked funds.

```
∀ program_id:
  Σ(payout_history[].amount) ≤ program.total_funds
```

**Rationale**: Prevents paying out more than was locked.

**Violation Consequences**: Contract insolvency.

**Check Frequency**: After every payout operation.

---

### PI4: Program Isolation Invariant

**Statement**: Operations on one program must not affect the state of other programs.

```
∀ program_id₁ ≠ program_id₂, operation on program_id₁:
  state(program_id₂)@before = state(program_id₂)@after
```

**Rationale**: Ensures program funds are kept separate.

**Violation Consequences**: Cross-program fund contamination.

**Check Frequency**: In multi-program test scenarios.

---

### PI5: Schedule Release Finality Invariant

**Statement**: Once a schedule is marked as released, it cannot be released again.

```
∀ schedule: schedule.released = true ⟹ ∀ future_time: schedule.released = true
```

**Rationale**: Prevents double-release of scheduled funds.

**Violation Consequences**: Double payment, fund drain.

**Check Frequency**: After schedule release operations.

---

### PI6: Schedule Timestamp Validity Invariant

**Statement**: Release timestamps must be in the future when created, and can only be automatically released after the timestamp.

```
At creation: schedule.release_timestamp > current_timestamp
At auto-release: current_timestamp ≥ schedule.release_timestamp
```

**Rationale**: Ensures time-based release mechanism works correctly.

**Violation Consequences**: Premature fund release.

**Check Frequency**: At schedule creation and release.

---

### PI7: Batch Payout Amount Consistency Invariant

**Statement**: In batch payouts, the sum of individual amounts must equal the total deducted from remaining balance.

```
∀ batch_payout:
  Σ(amounts[]) = remaining_balance@before - remaining_balance@after
```

**Rationale**: Ensures no funds are lost or created in batch operations.

**Violation Consequences**: Incorrect balance updates.

**Check Frequency**: After batch payout operations.

---

## Invariant Testing Approach

### Testing Strategy

1. **Invariant Checker Functions**: Create dedicated functions that verify each invariant
2. **Automatic Integration**: Call checkers after every state-changing operation
3. **Violation Tests**: Write tests that deliberately attempt to violate invariants
4. **Continuous Validation**: Run invariant checks in all existing tests

### Test File Structure

```rust
// Invariant checker functions (test module)
mod invariants {
    fn check_balance_consistency(env: &Env, contract: &Contract) { ... }
    fn check_status_transitions(escrow_before: &Escrow, escrow_after: &Escrow) { ... }
    fn check_no_double_spend(escrow: &Escrow) { ... }
    // ... other checkers
    
    // Composite checker - runs all relevant checks
    fn verify_all_invariants(env: &Env, contract: &Contract) { ... }
}

// Integration into existing tests
#[test]
fn test_lock_funds_success() {
    // ... test logic ...
    invariants::verify_all_invariants(&env, &contract);
}
```

### Violation Detection Tests

Each invariant should have a corresponding test that attempts to violate it:

```rust
#[test]
#[should_panic(expected = "Invariant violated: balance consistency")]
fn test_invariant_violation_balance() {
    // Deliberately create state that violates I1
    // Invariant checker should panic
}
```

---

## Implementation Guidelines

### 1. Checker Function Design

- **Pure functions**: Checkers should not modify state
- **Clear error messages**: Include invariant ID and violation details
- **Efficient**: Minimize gas/computation cost
- **Composable**: Allow checking subsets of invariants

### 2. When to Check

- **After state changes**: Always check after operations that modify escrow state
- **Before critical operations**: Check preconditions before releases/refunds
- **Batch operations**: Check both before and after entire batch

### 3. Error Reporting

```rust
fn check_invariant_i1(env: &Env, contract: &Contract) {
    let total_locked = calculate_total_locked(env);
    let contract_balance = get_contract_balance(env);
    
    if total_locked > contract_balance {
        panic!(
            "Invariant I1 violated: total_locked ({}) > contract_balance ({})",
            total_locked, contract_balance
        );
    }
}
```

### 4. Test Coverage

- **Happy path**: All normal operations should maintain invariants
- **Edge cases**: Boundary conditions, maximum values, zero amounts
- **Error paths**: Failed operations should not violate invariants
- **Complex scenarios**: Multi-operation workflows should maintain invariants throughout

---

## Monitoring and Maintenance

### Continuous Validation

- Run invariant tests in CI/CD pipeline
- Include invariant checks in integration tests
- Monitor for new invariants as contract evolves

### Documentation Updates

When adding new features:
1. Identify new invariants introduced
2. Document in this file
3. Implement checker functions
4. Add to test suite
5. Add violation tests

### Audit Trail

- Log invariant check results in test output
- Maintain history of invariant violations found during development
- Use findings to improve contract design

---

## References

- Soroban Documentation: https://soroban.stellar.org/docs
- Contract Source: `contracts/bounty_escrow/contracts/escrow/src/lib.rs`
- Test Files: 
  - `contracts/bounty_escrow/contracts/escrow/src/test.rs`
  - `contracts/bounty_escrow/contracts/escrow/src/test_bounty_escrow.rs`

---

**Last Updated**: 2024-01-30
**Version**: 1.0.0
**Maintainer**: Grainlify Core Team
