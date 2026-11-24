# Bounty Escrow — Events Reference

Module: `bounty_escrow/contracts/escrow/src/events.rs`  
Schema version: **EVENT_VERSION_V2** (`version: u32 = 2`)

---

## Overview

Every event emitted by `BountyEscrowContract` carries:

1. A **topic list** — one or two `Symbol` values that indexers use for prefix-filtering without decoding the payload.
2. A **typed data payload** — a `#[contracttype]` struct whose first field is always `version: u32 = EVENT_VERSION_V2`.

```
topics : (category_sym [, bounty_id: u64])
data   : <EventStruct { version: 2, ... }>
```

The `version` field lives in the *payload*, not the topics, to preserve backwards-compatible topic-filter subscriptions when schemas evolve.

---

## Event Catalogue

### `BountyEscrowInitialized`

Emitted **once** by `init()` on successful contract initialization.

| Topics | `("init",)` |
|--------|-------------|
| `version` | `u32 = 2` |
| `admin` | `Address` — initial admin |
| `token` | `Address` — reward token contract |
| `timestamp` | `u64` — ledger time |

**Invariants checked before emission:**
- `AlreadyInitialized` guard prevents duplicate emission.
- `admin ≠ token` — validated by `validate_init_params`.

---

### `FundsLocked`

Emitted by `lock_funds()` and `batch_lock_funds()` (once per bounty).

| Topics | `("f_lock", bounty_id: u64)` |
|--------|------------------------------|
| `version` | `u32 = 2` |
| `bounty_id` | `u64` |
| `amount` | `i128` — **gross** deposit (before lock fee) |
| `depositor` | `Address` |
| `deadline` | `u64` — claim cut-off timestamp |

---

### `FundsReleased`

Emitted by `release_funds()`, `partial_release()`, and `release_with_capability()`.

| Topics | `("f_rel", bounty_id: u64)` |
|--------|------------------------------|
| `version` | `u32 = 2` |
| `bounty_id` | `u64` |
| `amount` | `i128` — net payout (after release fee) |
| `recipient` | `Address` — contributor wallet |
| `timestamp` | `u64` |

For partial releases, this event is emitted on **every call**. Sum all `FundsReleased` events for a bounty to reconstruct total payout.

---

### `FundsRefunded`

Emitted by `refund()`, `refund_resolved()`, and `refund_with_capability()`.

| Topics | `("f_ref", bounty_id: u64)` |
|--------|------------------------------|
| `version` | `u32 = 2` |
| `bounty_id` | `u64` |
| `amount` | `i128` |
| `refund_to` | `Address` — may differ from depositor on admin-approved refunds |
| `timestamp` | `u64` |

---

### `FeeCollected`

Emitted whenever a non-zero fee is transferred.

| Topics | `("fee",)` |
|--------|------------|
| `operation_type` | `FeeOperationType` — `Lock` or `Release` |
| `amount` | `i128` — actual fee (ceiling-rounded) |
| `fee_rate` | `i128` — basis points applied |
| `recipient` | `Address` |
| `timestamp` | `u64` |

> **Note on ceiling division:** `fee = ⌈amount × rate / 10_000⌉`. This prevents the dust-splitting attack where many small deposits each round the fee to zero.

---

### `FeeConfigUpdated`

Emitted by `update_fee_config()`.

| Topics | `("fee_cfg",)` |
|--------|----------------|
| `lock_fee_rate` | `i128` — new lock rate in bps |
| `release_fee_rate` | `i128` — new release rate in bps |
| `fee_recipient` | `Address` |
| `fee_enabled` | `bool` |
| `timestamp` | `u64` |

---

### `BatchFundsLocked`

Emitted **once** per `batch_lock_funds()` call, after all per-bounty `FundsLocked` events.

| Topics | `("b_lock",)` |
|--------|---------------|
| `count` | `u32` — number of bounties locked |
| `total_amount` | `i128` — sum of all locked amounts |
| `timestamp` | `u64` |

---

### `BatchFundsReleased`

Emitted **once** per `batch_release_funds()` call.

| Topics | `("b_rel",)` |
|--------|--------------|
| `count` | `u32` |
| `total_amount` | `i128` |
| `timestamp` | `u64` |

---

### `FundsLockedAnon`

Emitted by `lock_funds_anonymous()`. The depositor's address is **never** stored or emitted.

| Topics | `("f_lkanon", bounty_id: u64)` |
|--------|-------------------------------|
| `version` | `u32 = 2` |
| `bounty_id` | `u64` |
| `amount` | `i128` |
| `depositor_commitment` | `BytesN<32>` — hash commitment of depositor identity |
| `deadline` | `u64` |

---

### `DeprecationStateChanged`

Emitted by `set_deprecated()`.

| Topics | `("deprec",)` |
|--------|---------------|
| `deprecated` | `bool` — new state |
| `migration_target` | `Option<Address>` |
| `admin` | `Address` |
| `timestamp` | `u64` |

When `deprecated = true`, all subsequent `lock_funds` / `batch_lock_funds` calls return `Error::ContractDeprecated`.

---

### `MaintenanceModeChanged`

Emitted by `set_maintenance_mode()`.

| Topics | `("maint",)` |
|--------|--------------|
| `enabled` | `bool` |
| `admin` | `Address` |
| `timestamp` | `u64` |

---

### `ParticipantFilterModeChanged`

Emitted by `set_filter_mode()`.

| Topics | `("pf_mode",)` |
|--------|----------------|
| `previous_mode` | `ParticipantFilterMode` |
| `new_mode` | `ParticipantFilterMode` |
| `admin` | `Address` |
| `timestamp` | `u64` |

---

### `RiskFlagsUpdated`

Emitted by `set_escrow_risk_flags()` and `clear_escrow_risk_flags()`.

| Topics | `("risk", bounty_id: u64)` |
|--------|---------------------------|
| `version` | `u32 = 2` |
| `bounty_id` | `u64` |
| `previous_flags` | `u32` |
| `new_flags` | `u32` |
| `admin` | `Address` |
| `timestamp` | `u64` |

**Defined flag bits:**

| Bit | Constant | Meaning |
|-----|----------|---------|
| 0 | `RISK_FLAG_HIGH_RISK` | Elevated risk |
| 1 | `RISK_FLAG_UNDER_REVIEW` | Under review |
| 2 | `RISK_FLAG_RESTRICTED` | Payout restricted |
| 3 | `RISK_FLAG_DEPRECATED` | Bounty deprecated |

---

### `TicketIssued`

Emitted by `issue_claim_ticket()` and `issue_claim_ticket_deterministic()`.

| Topics | `("ticket_i", ticket_id: u64)` |
|--------|-------------------------------|
| `ticket_id` | `u64` — monotonic |
| `bounty_id` | `u64` |
| `beneficiary` | `Address` |
| `amount` | `i128` |
| `expires_at` | `u64` |
| `issued_at` | `u64` |

---

### `TicketClaimed`

Emitted when a claim ticket is redeemed.

| Topics | `("ticket_c", ticket_id: u64)` |
|--------|-------------------------------|
| `ticket_id` | `u64` |
| `bounty_id` | `u64` |
| `claimer` | `Address` |
| `claimed_at` | `u64` |
| `amount` | `i128` — amount paid out |

---

### `TicketRevoked`

Emitted by `revoke_claim_ticket()` when the admin or depositor revokes an outstanding ticket and its reservation is returned to the escrow.

| Topics | `("ticket_r", ticket_id: u64)` |
|--------|-------------------------------|
| `ticket_id` | `u64` |
| `bounty_id` | `u64` |
| `beneficiary` | `Address` |
| `amount` | `i128` — reservation released |
| `revoked_by` | `Address` |
| `revoked_at` | `u64` |

---

### `DeterministicSelectionDerived`

Emitted by `issue_claim_ticket_deterministic()` before ticket issuance.

| Topics | `("prng_sel", bounty_id: u64)` |
|--------|-------------------------------|
| `bounty_id` | `u64` |
| `selected_index` | `u32` — zero-based index into candidates |
| `candidate_count` | `u32` |
| `selected_beneficiary` | `Address` |
| `seed_hash` | `BytesN<32>` — for off-chain verification |
| `winner_score` | `BytesN<32>` |
| `timestamp` | `u64` |

---

### `EmergencyWithdrawEvent`

Emitted by `emergency_withdraw()`.

| Topics | `("em_wtd",)` |
|--------|---------------|
| `admin` | `Address` |
| `recipient` | `Address` |
| `amount` | `i128` — entire contract balance drained |
| `timestamp` | `u64` |

---

### Capability Events

| Event | Topics | Key fields |
|-------|--------|------------|
| `CapabilityIssued` | `("cap_new", capability_id)` | `owner`, `holder`, `action`, `bounty_id`, `amount_limit`, `expires_at`, `max_uses` |
| `CapabilityUsed` | `("cap_use", capability_id)` | `holder`, `action`, `amount_used`, `remaining_amount`, `remaining_uses` |
| `CapabilityRevoked` | `("cap_rev", capability_id)` | `owner`, `revoked_at` |

---

## State → Event Matrix

```
init()                           → BountyEscrowInitialized
lock_funds()                     → FundsLocked [+ FeeCollected if fee > 0]
batch_lock_funds()               → FundsLocked × N + BatchFundsLocked
lock_funds_anonymous()           → FundsLockedAnon
release_funds()                  → FundsReleased [+ FeeCollected if fee > 0]
partial_release()                → FundsReleased
release_with_capability()        → FundsReleased + CapabilityUsed
batch_release_funds()            → FundsReleased × N + BatchFundsReleased
refund()                         → FundsRefunded
refund_resolved()                → FundsRefunded
refund_with_capability()         → FundsRefunded + CapabilityUsed
set_deprecated()                 → DeprecationStateChanged
set_maintenance_mode()           → MaintenanceModeChanged
set_paused()                     → PauseStateChanged (per operation)
set_filter_mode()                → ParticipantFilterModeChanged
update_fee_config()              → FeeConfigUpdated
set_escrow_risk_flags()          → RiskFlagsUpdated
clear_escrow_risk_flags()        → RiskFlagsUpdated
issue_claim_ticket()             → TicketIssued
claim_with_ticket()              → TicketClaimed
revoke_claim_ticket()            → TicketRevoked
issue_claim_ticket_deterministic()→ DeterministicSelectionDerived + TicketIssued
emergency_withdraw()             → EmergencyWithdrawEvent
issue_capability()               → CapabilityIssued
revoke_capability()              → CapabilityRevoked
approve_large_release()          → ApprovalAdded
```

---

## Indexing Guide

### Filter all bounty-escrow events (Horizon RPC)

```json
{ "topic1": "f_lock" }
{ "topic1": "f_rel" }
{ "topic1": "f_ref" }
```

### Filter by bounty_id (topic2)

```json
{ "topic1": "f_lock", "topic2": "0x000000000000002a" }
```
*(topic2 is the bounty_id encoded as a u64 XDR integer)*

### Decode a `FundsLocked` payload (JavaScript)

```typescript
import { xdr, scValToNative } from "@stellar/stellar-sdk";

function decodeFundsLocked(base64Data: string) {
  const val = xdr.ScVal.fromXDR(base64Data, "base64");
  return scValToNative(val);
  // Returns: { version: 2, bounty_id: ..., amount: ..., depositor: ..., deadline: ... }
}
```

---

## Security Notes

1. **CEI ordering** — All events are emitted *after* state mutations and token transfers. An emitted event is a reliable indicator that the corresponding on-chain state change occurred.
2. **No PII on-chain** — Events carry wallet addresses only. KYC identity data remains off-chain per Grainlify's privacy model.
3. **`symbol_short!` length limit** — All topic strings are ≤ 8 bytes. Soroban silently truncates longer strings, which would corrupt topic-based filtering. Enforced by `symbol_short!` macro at compile time.
4. **Version in payload, not topics** — Placing the version field in the data payload (rather than topics[0]) allows indexers to subscribe to a stable topic like `"f_lock"` without needing to re-subscribe when the schema version bumps.
5. **Re-entrancy** — Events are only published after the reentrancy guard is held, so duplicate events from re-entrant calls are structurally impossible.
6. **Anonymous escrow privacy** — `FundsLockedAnon` publishes a 32-byte commitment, never the depositor address. The commitment must be computed off-chain using a collision-resistant function.

---

## Changelog

| Version | Change |
|---------|--------|
| `EVENT_VERSION_V2` (= 2) | Initial versioned schema for all bounty-escrow events. All payload structs carry `version: u32 = 2`. |