# Capability Token Replay-Attack Prevention

> Closes #1379

## Overview

A **CapabilityToken** (`BytesN<32>`) is a cryptographically-opaque, unforgeable
identifier that grants a specific `holder` the right to perform one action
(`Release`, `Refund`, or `Claim`) on a specific bounty up to a configured amount
and number of uses.

Replay attacks occur when an actor re-submits a previously-observed token to
trigger an action that should no longer be valid. This document describes the
threat model, the on-chain defences, and the regression test coverage added in
this PR.

---

## Threat Model

| Attack vector | Description |
|---|---|
| **Revoke-then-reuse** | Admin revokes a capability; attacker holds the raw bytes and retries the call |
| **Expired replay** | Token's `expiry` ledger timestamp passes; attacker replays the original transaction |
| **Cross-escrow replay** | Token issued for bounty A is submitted against bounty B |
| **Exact byte replay** | Attacker retains the `BytesN<32>` identifier and reuses it byte-for-byte |
| **Partial-drain across revoke** | Token used once legitimately, then revoked; attacker tries subsequent uses |

---

## On-Chain Defences

### 1. Permanent revocation flag

When `revoke_capability` is called, the stored `Capability` struct has its
`revoked` field set to `true`.  `consume_capability` checks this flag **first**,
before any other validation:

```rust
if capability.revoked {
    return Err(Error::CapabilityRevoked);  // exits immediately
}
```

The token ID (`BytesN<32>`) remains in storage so that subsequent lookup calls
still return the record with `revoked = true` — an audit trail that also blocks
all future use of that exact token.

### 2. Expiry enforcement

The `expiry` field records the ledger timestamp after which the token becomes
invalid.  `consume_capability` checks:

```rust
if env.ledger().timestamp() > capability.expiry {
    return Err(Error::CapabilityExpired);
}
```

This check runs after the revocation check but **before** any state mutation or
auth call.

### 3. Bounty-ID binding

Every capability carries the exact `bounty_id` for which it was issued.
Attempting to use it against a different bounty triggers:

```rust
if capability.bounty_id != bounty_id {
    return Err(Error::CapabilityActionMismatch);
}
```

This makes cross-escrow replay impossible even if the attacker controls both
contracts.

### 4. Holder binding

The `holder` field is checked before `require_auth` is called.  A capability
issued to Alice cannot be consumed by Bob:

```rust
if capability.holder != holder.clone() {
    return Err(Error::Unauthorized);
}
```

### 5. Use-count exhaustion

Each capability has a `remaining_uses` counter.  When it reaches 0, further
consumption returns `Error::CapabilityUsesExhausted`.  This limits the blast
radius of a leaked token.

### 6. Amount limit

The `remaining_amount` field monotonically decreases with each consumption.
Attempting to extract more than the remaining amount returns
`Error::CapabilityAmountExceeded`, preventing drain-attacks on partial-use
tokens.

---

## Validation Order in `consume_capability`

Checks are applied in this deterministic order to ensure errors are returned
**before any state mutation**:

1. Token exists in storage (`CapabilityNotFound`)
2. `revoked == false` (`CapabilityRevoked`)  ← replay guard #1
3. `action == expected_action` (`CapabilityActionMismatch`)
4. `bounty_id` matches (`CapabilityActionMismatch`)  ← cross-escrow guard
5. `holder == caller` (`Unauthorized`)
6. `timestamp <= expiry` (`CapabilityExpired`)  ← expired-replay guard
7. `remaining_uses > 0` (`CapabilityUsesExhausted`)
8. `amount <= remaining_amount` (`CapabilityAmountExceeded`)
9. `holder.require_auth()` (Soroban auth frame)
10. Owner authority re-validated at execution time

State is only mutated (decrement `remaining_amount`, decrement `remaining_uses`,
persist) **after** all checks pass.

---

## Regression Tests (`src/capability_replay_tests.rs`)

| Test | Scenario | Expected error |
|---|---|---|
| `test_revoke_then_reuse_release` | Revoke → `release_with_capability` | `CapabilityRevoked` |
| `test_revoke_then_reuse_refund` | Revoke → `refund_with_capability` | `CapabilityRevoked` |
| `test_revoke_prevents_all_subsequent_uses` | 3-use cap, 1 legitimate use, revoke, retry | `CapabilityRevoked` |
| `test_expired_token_replay` | Advance clock past expiry → `release_with_capability` | `CapabilityExpired` |
| `test_expired_refund_capability_replay` | Advance clock past expiry → `refund_with_capability` | `CapabilityExpired` |
| `test_cross_escrow_token_replay` | Cap for bounty A used against bounty B (release) | `CapabilityActionMismatch` |
| `test_cross_escrow_refund_replay` | Cap for bounty A used against bounty B (refund) | `CapabilityActionMismatch` |
| `test_exact_byte_replay_after_revoke` | Retain `BytesN<32>`, revoke, replay twice | `CapabilityRevoked` |
| `test_single_use_token_byte_replay` | 1-use cap exhausted, byte-identical retry | err (no funds extractable) |

All tests additionally verify:
- Escrow `remaining_amount` and `status` are **unchanged** after a failed replay
- Token balances of involved addresses are **unchanged** after a failed replay

---

## Running the Tests

```bash
cd contracts/bounty_escrow
cargo test -p bounty-escrow capability_replay
```

To run the entire test suite:

```bash
cargo test -p bounty-escrow
```

---

## Security Guarantees

- A revoked token is **permanently invalid** — no time-based expiry or state
  change can re-activate it.
- Cross-escrow usage is **structurally impossible** via the `bounty_id` binding.
- Byte-for-byte replays are blocked because the `revoked` flag is persisted in
  the same storage slot as the token ID.
- All replay checks run **before** any storage write or token transfer, meaning
  failed replay attempts have zero observable side effects.
