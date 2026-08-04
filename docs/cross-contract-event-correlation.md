# Cross-Contract Event Correlation Specification

This document details the cross-contract event correlation identifier (`CorrelationId`) convention used across `grainlify-core`, `program-escrow`, and `bounty_escrow`.

---

## 1. Overview & Context

Multi-contract operations in Grainlify—such as a `grainlify-core` governance upgrade that updates contract WASM and subsequently affects escrow payout logic, or facade-triggered multi-escrow actions—emit distinct event streams across independent contract instances.

Previously, off-chain indexers and audit services had to infer event relationships heuristically using block timestamps and transaction hash matching.

The `CorrelationId` mechanism provides a deterministic, additive identifier (`Option<CorrelationId>`) embedded directly in high-traffic event structs, linking related multi-contract actions together.

---

## 2. Shared CorrelationId Convention

The `CorrelationId` is defined once in `grainlify-core` and reused across all Grainlify contracts:

```rust
use soroban_sdk::{contracttype, BytesN};

/// Shared correlation identifier type convention across all Grainlify contracts.
/// Encapsulates a 32-byte hash identifying a single logical multi-contract action.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CorrelationId(pub BytesN<32>);
```

---

## 3. Deterministic Generation Helper

Contracts and clients derive correlation IDs deterministically using the helper function exported by `grainlify-core::correlation`:

```rust
pub fn generate_correlation_id(
    env: &Env,
    initiator: &Address,
    nonce: u64,
    domain: Option<&Symbol>,
) -> CorrelationId
```

### Derivation Formula
```text
CorrelationId = SHA256(
    XDR(initiator) ||
    XDR(nonce) ||
    XDR(domain)? ||
    XDR(current_contract_address)
)
```

### Parameters
| Parameter | Type | Description |
|-----------|------|-------------|
| `env` | `&Env` | Soroban environment handle |
| `initiator` | `&Address` | Account or contract address initiating the transaction |
| `nonce` | `u64` | Caller-supplied sequence or nonce for uniqueness and replay isolation |
| `domain` | `Option<&Symbol>` | Optional domain tag to prevent cross-domain collisions (e.g. `symbol_short!("payout")`) |

---

## 4. Event Schema Integration

An optional `correlation_id: Option<CorrelationId>` field has been added to high-traffic event types as an **additive, non-breaking** change.

### High-Traffic Event Types by Contract

#### `grainlify-core`
- `UpgradeEvent`: Emitted during single-admin or multisig contract WASM upgrades.
- `MigrationEvent`: Emitted upon execution of state/schema migrations.
- `ReadOnlyModeEvent`: Emitted when emergency read-only mode is toggled.

#### `program-escrow`
- `PayoutEvent`: Emitted on single prize/fund payout execution.
- `BatchPayoutEvent`: Emitted on atomic multi-recipient batch payouts.
- `ReleaseScheduledEvent`: Emitted when a time-locked release schedule is created.
- `ScheduleReleasedEvent`: Emitted when a scheduled release matures and pays out.

#### `bounty_escrow`
- `FundsLocked`: Emitted when bounty escrow deposits are locked.
- `FundsReleased`: Emitted when bounty funds are released to contributors.
- `FundsRefunded`: Emitted on depositor, auto-expiry, or oracle refunds.
- `BatchFundsLocked`: Emitted on bulk bounty lock operations.

---

## 5. Off-Chain Indexing & Querying

Indexers can query and correlate events across contracts using the optional `correlation_id` field in the payload data:

```json
{
  "contract_id": "CC...CORE",
  "topic": ["upgrade", "wasm"],
  "payload": {
    "version": 2,
    "new_wasm_hash": "a1b2...",
    "correlation_id": "0x4f8a9e..."
  }
}
```

```json
{
  "contract_id": "CD...PROGRAM_ESCROW",
  "topic": ["Payout"],
  "payload": {
    "version": 2,
    "program_id": "PROG_2026",
    "amount": 50000,
    "correlation_id": "0x4f8a9e..."
  }
}
```

Indexers match events across contract boundaries by indexing records where `correlation_id` matches `0x4f8a9e...`.

---

## 6. Security & Compatibility Considerations

1. **Additive Schema Compatibility**: `Option<CorrelationId>` is added to event structs. Deserializing legacy payloads with missing `correlation_id` fields defaults to `None` without panic.
2. **Domain Separation**: Including optional `Symbol` domain tags prevents accidental correlation collisions between distinct operations performed by the same initiator using the same nonce.
3. **Privacy & Non-Sensitivity**: Correlation IDs contain SHA-256 hashes of public transaction parameters and contain no PII, secret keys, or sensitive payload data.
4. **Replay Protection Integration**: Generating correlation IDs from nonces ensures uniqueness across distinct calls.
