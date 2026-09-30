# Grainlify Escrow Storage Layout

This document defines the storage layout for the `escrow` contract. Any modifications to structural types or addition of keys must be reflected here.

## Storage Schema Version: 1

Below are all storage keys utilized by the contract.

| Key | Variant/Constant | Tier | Type | Notes |
|-----|-----------------|------|------|-------|
| `DataKey::Admin` | `Admin` | Instance | `Address` | Set once at initialization |
| `DataKey::Token` | `Token` | Instance | `Address` | Token contract address |
| `DataKey::Escrow(u64)` | `Escrow(id)` | Persistent | `Escrow` | Per-bounty escrow state |
| `DataKey::EscrowIndex` | `EscrowIndex` | Instance | `u64` | Monotone escrow index counter |
| `DataKey::LabelConfig` | `LabelConfig` | Persistent | `LabelConfig` | Restricted labels configuration |
| `// Identity-related storage keys` | | | | |
| `DataKey::AddressIdentity(Address)` | `AddressIdentity(addr)` | Persistent | `Address` | Identity address mapping |
| `DataKey::AuthorizedIssuer(Address)` | `AuthorizedIssuer(issuer)` | Persistent | `Address` | Authorized issuer for tier updates |
| `DataKey::TierLimits` | `TierLimits` | Instance | `TierLimits` | Rate limit configuration |
| `DataKey::RiskThresholds` | `RiskThresholds` | Instance | `RiskThresholds` | Risk threshold settings |
| `DataKey::ReentrancyGuard` | `ReentrancyGuard` | Instance | `ReentrancyGuard` | Reentrancy protection state |
| `DataKey::EscrowJurisdiction(u64)` | `EscrowJurisdiction(id)` | Persistent | `Jurisdiction` | Jurisdiction configuration per bounty |
| `// Ownership transfer` | | | | |
| `DataKey::PendingAdmin` | `PendingAdmin` | Instance | `Address` | Pending admin transfer |
| `DataKey::EscrowPendingDepositor(u64)` | `EscrowPendingDepositor(id)` | Persistent | `Address` | Pending depositor for escrow |

## Migration Steps
If modifying the schema:
1. Bump `STORAGE_SCHEMA_VERSION` in `lib.rs`.
2. Update this layout document.
3. Write `migrate` implementations that gracefully handle reading old variants and overwriting them with new variants.
4. Update `verify_storage_layout()` assertions to reflect the new requirements.