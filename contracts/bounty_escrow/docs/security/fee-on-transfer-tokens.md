# Fee-on-Transfer Token Security Analysis

## Overview

A **fee-on-transfer (FoT) token** is a token contract that silently deducts a
fee from the transferred amount at the token level, delivering less than the
declared amount to the recipient. This class of token is common in DeFi and is
a well-known source of accounting vulnerabilities in escrow and vault contracts
that blindly trust the declared transfer amount.

This document describes how the Bounty Escrow contract defends against FoT
tokens, what assumptions are in place, and how those defences are verified.

---

## The Attack Vector

### Normal token behaviour

```
token.transfer(depositor, escrow_contract, 1_000)
  → escrow_contract receives exactly 1_000
  → escrow records remaining_amount = 1_000
  → sum(escrow remaining) == token.balance(escrow_contract)  ✓ INV-2 holds
```

### Fee-on-transfer token (50% fee)

```
token.transfer(depositor, escrow_contract, 1_000)
  → escrow_contract receives only 500  (token charged 500 as fee)
  → escrow records remaining_amount = 1_000  ← declared, not received
  → sum(escrow remaining) = 1_000 ≠ token.balance(escrow_contract) = 500  ✗ INV-2 violated
```

### Worst case: 100% fee (full drain)

```
token.transfer(depositor, escrow_contract, 1_000)
  → escrow_contract receives 0
  → escrow records remaining_amount = 1_000
  → Later: release_funds tries to transfer 1_000 to contributor
  → Token panics: "Insufficient balance"  — contributor cannot be paid
```

---

## Contract Defences

### 1. INV-2 — Aggregate-to-Ledger Invariant (primary defence)

**Location:** `multitoken_invariants::assert_after_lock`

**Trigger:** Called at the end of every `lock_funds`, `publish`, and
`execute_recurring_lock` call, immediately after token transfers.

**Check:**
```
sum(escrow.remaining_amount  for all Locked/PartiallyRefunded escrows)
  == token.balance(escrow_contract_address)
```

**On failure:** Panics with `"INV-2 violated after lock: escrow sum (X) != balance (Y)"`.
The Soroban host rolls back **all** state mutations in the transaction atomically,
including the token transfer itself. The depositor's tokens are not lost.

**Result:** A fee-on-transfer token causes `lock_funds` to fail at the invariant
check. The entire call is reverted, protecting both the depositor and the escrow.

### 2. Net-amount guard (secondary defence)

**Location:** `lock_funds_logic`, lines after `combined_fee_amount`

**Check:**
```rust
let net_amount = amount.checked_sub(fee_amount).unwrap_or(amount);
if net_amount <= 0 {
    return Err(Error::InvalidAmount);
}
```

**Scope:** This guard targets the **protocol fee** (lock_fee_rate + lock_fixed_fee),
not the token-level fee. It prevents zero-value escrows when the protocol fee rate
is configured at its maximum (50%).

### 3. publish() INV-2 check

**Location:** `publish_logic`, line after status transition

**Check:** Identical to the lock-time INV-2 check.

**Significance:** Any Draft escrow that was created with a FoT token (e.g., via a
recurring lock with INV-2 disabled) will be caught by `publish()` before it
becomes a live Locked escrow eligible for `release_funds`.

---

## Security Assumptions

| Assumption | Justification |
|------------|---------------|
| Honest token for normal operation | The contract is designed for the Stellar Asset Contract (SAC). Custom tokens must correctly implement the SEP-41 interface. Operators should only register trusted, audited tokens. |
| INV-2 is never permanently disabled in production | The `InvOff` storage key exists solely to support adversarial-state unit tests. It must not be set in a production deployment. |
| Soroban host atomicity | Panics within a contract call cause a complete rollback of all storage and token-balance mutations in that transaction. The contract relies on this guarantee. |
| Single token per escrow contract | The contract tracks one token address (`DataKey::Token`). INV-2 sums balances across all escrows backed by that single token, so cross-token accounting is not in scope. |

---

## What the Contract Does NOT Protect Against

1. **Rebasing or supply-changing tokens.** A token that increases or decreases
   all balances by a global factor (e.g., interest-bearing aTokens) will cause
   INV-2 to mismatch over time, breaking the escrow. Such tokens should not be
   used with this contract.

2. **Tokens with transfer hooks.** Tokens that invoke external contracts during
   transfer could re-enter the escrow. The reentrancy guard (`DataKey::ReentrancyGuard`)
   blocks this for all protected entry points.

3. **Operator misconfiguration.** If the operator deliberately sets `InvOff = true`
   in production and uses a FoT token, the accounting discrepancy will silently
   accumulate. This is a configuration error, not a contract vulnerability.

---

## Test Coverage

All defences are verified in
`contracts/escrow/src/test_fee_on_transfer.rs`.

| Test | Group | Verifies |
|------|-------|----------|
| `test_full_fee_drain_detected_by_inv2_on_lock` | INV-2 active | 100% FoT token → lock panics, rollback |
| `test_partial_fee_imbalance_detected_by_inv2_on_lock` | INV-2 active | 50% FoT token → lock panics |
| `test_over_hundred_pct_fee_drain_detected_by_inv2_on_lock` | INV-2 active | 200% fee (over-drain) → lock panics |
| `test_zero_fee_token_completes_full_lifecycle` | Baseline | 0% fee → full lock/release succeeds |
| `test_escrow_data_invariants_remain_valid_with_drained_token` | INV-1 | Escrow data never negative (INV-2 bypassed) |
| `test_partial_fee_creates_documented_accounting_discrepancy` | INV-1 | Discrepancy = fee charged by token |
| `test_release_panics_when_contract_balance_drained_by_fee_token` | Downstream | Release fails after drain |
| `test_lock_returns_invalid_amount_when_protocol_fee_equals_deposit` | Net-amount guard | Protocol fee = 100% → `Error::InvalidAmount` |
| `test_publish_detects_token_balance_shortfall_via_inv2` | publish() | Draft escrow with shortfall → publish panics |
| `test_publish_succeeds_when_token_balance_matches_escrow` | publish() | Healthy Draft → publish succeeds |
| `test_publish_nonexistent_bounty_returns_bounty_not_found` | publish() | Bad input → `Error::BountyNotFound` |
| `test_publish_on_locked_escrow_returns_funds_not_locked` | publish() | Already-Locked → `Error::FundsNotLocked` |
| `test_inv2_panic_rolls_back_depositor_balance` | Atomicity | Full rollback after INV-2 panic |

---

## Recommended Token Allowlist

Operators should restrict the token address to one of the following
battle-tested Stellar token types:

- **Stellar Asset Contract (SAC)** for native XLM and anchored assets
  (USDC, AQUA, etc.) — no fees on transfer.
- **Circle USDC** (`GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN`)
  on Stellar mainnet — standard SAC.

Custom token contracts should be audited for fee-on-transfer behaviour before
being registered with the escrow contract.

---

## References

- `contracts/escrow/src/multitoken_invariants.rs` — INV-2 implementation
- `contracts/escrow/src/lib.rs` — `lock_funds_logic`, `publish_logic`
- `contracts/escrow/src/test_fee_on_transfer.rs` — full test suite
- [Soroban SEP-41 Token Interface](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0041.md)
- [SWC-112: Delegatecall to Untrusted Callee](https://swcregistry.io/docs/SWC-112/) (analogous EVM pattern)
