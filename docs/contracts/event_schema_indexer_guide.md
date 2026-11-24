# Grainlify Contracts — Event Schema & Indexer Guide

## Overview

All events across `bounty-escrow`, `program-escrow`, and `grainlify-core`
follow the **EVENT_VERSION_V2** envelope. Every event payload carries a
`version: u32 = 2` field so indexers can detect schema changes without
decoding the full XDR body.

## Topic Layout
```
topics[0]  — category Symbol  (always present, e.g. "f_lock", "fee")
topics[1]  — bounty_id: u64   (present where applicable)
data       — typed event struct (always carries version: u32)
```

All topic strings use `symbol_short!` and are ≤ 9 bytes — enforced at
compile time by the Soroban SDK.

## Full Event Inventory

### bounty-escrow

| Topic[0]    | Topic[1]       | Struct                      | Since |
|-------------|----------------|-----------------------------|-------|
| `init`      | —              | `BountyEscrowInitialized`   | V2    |
| `f_lock`    | `bounty_id`    | `FundsLocked`               | V2    |
| `f_rel`     | `bounty_id`    | `FundsReleased`             | V2    |
| `f_ref`     | `bounty_id`    | `FundsRefunded`             | V2    |
| `pub`       | `bounty_id`    | `EscrowPublished`           | V2    |
| `archive`   | `bounty_id`    | `EscrowArchived`            | V2    |
| `orc_cfg`   | —              | `OracleConfigUpdated`       | V2    |
| `fee`       | —              | `FeeCollected`              | V2    |
| `fee_cfg`   | —              | `FeeConfigUpdated`          | V2    |
| `fee_rte`   | `bounty_id`    | `FeeRoutingUpdated`         | V2    |
| `fee_rt`    | `bounty_id`    | `FeeRouted`                 | V2    |
| `b_lock`    | —              | `BatchFundsLocked`          | V2    |
| `b_rel`     | —              | `BatchFundsReleased`        | V2    |
| `approval`  | `bounty_id`    | `ApprovalAdded`             | V2    |
| `prng_sel`  | `bounty_id`    | `DeterministicSelectionDerived` | V2 |
| `f_lkanon`  | `bounty_id`    | `FundsLockedAnon`           | V2    |
| `deprec`    | —              | `DeprecationStateChanged`   | V2    |
| `maint`     | —              | `MaintenanceModeChanged`    | V2    |
| `pf_mode`   | —              | `ParticipantFilterModeChanged` | V2 |
| `risk`      | `bounty_id`    | `RiskFlagsUpdated`          | V2    |
| `ticket_i`  | `ticket_id`    | `TicketIssued`              | V2    |
| `ticket_c`  | `ticket_id`    | `TicketClaimed`             | V2    |
| `ticket_r`  | `ticket_id`    | `TicketRevoked`             | V2    |
| `pause`     | `operation`    | `PauseStateChanged`         | V2    |
| `em_wtd`    | —              | `EmergencyWithdrawEvent`    | V2    |
| `cap_new`   | `capability_id`| `CapabilityIssued`          | V2    |
| `cap_use`   | `capability_id`| `CapabilityUsed`            | V2    |
| `cap_rev`   | `capability_id`| `CapabilityRevoked`         | V2    |
| `tmlk_cfg`  | —              | `TimelockConfigured`        | V2    |
| `act_prop`  | —              | `AdminActionProposed`       | V2    |
| `act_exec`  | —              | `AdminActionExecuted`       | V2    |
| `act_cncl`  | —              | `AdminActionCancelled`      | V2    |

### program-escrow

| Topic[0]    | Struct                        | Since |
|-------------|-------------------------------|-------|
| `PrgInit`   | `ProgramInitializedEvent`     | V2    |
| `FndsLock`  | `FundsLockedEvent`            | V2    |
| `BatLck`    | `BatchFundsLocked`            | V2    |
| `BatRel`    | `BatchFundsReleased`          | V2    |
| `BatchPay`  | `BatchPayoutEvent`            | V2    |
| `Payout`    | `PayoutEvent`                 | V2    |
| `RelSched`  | `ReleaseScheduledEvent`       | V2    |
| `SchRel`    | `ScheduleReleasedEvent`       | V2    |
| `DspOpen`   | `DisputeOpenedEvent`          | V2    |
| `DspRslv`   | `DisputeResolvedEvent`        | V2    |
| `PauseSt`   | `PauseStateChanged`           | V2    |
| `em_wtd`    | `EmergencyWithdrawEvent`      | V2    |

### grainlify-core

| Topic[0]      | Topic[1]    | Notes                        |
|---------------|-------------|------------------------------|
| `metric`      | `op`        | `OperationMetric`            |
| `metric`      | `perf`      | `PerformanceMetric`          |
| `init`        | `gov`       | Governance initialized       |
| `upgrade`     | `wasm`      | `UpgradeEvent`               |
| `ROModeChg`   | —           | `ReadOnlyModeEvent`          |
| `cfg_snap`    | `create`    | Snapshot created             |
| `cfg_snap`    | `restore`   | Snapshot restored            |
| `migration`   | —           | `MigrationEvent`             |

## Unknown Version Handling (for indexers)

When `version > 2` is encountered:
1. **Do not hard-fail** — emit a warning and skip payload decoding
2. **Preserve raw XDR bytes** for reprocessing after decoder update
3. **Never discard topic data** — topics are stable across versions

## Security Invariants

- Events are emitted **after** all state mutations (CEI ordering)
- No PII, KYC data, or private keys appear in any event payload
- All `symbol_short!` strings are ≤ 9 bytes (compile-time enforced)

## Running Tests
```bash
cargo test -p bounty-escrow
cargo test -p program-escrow
cargo test -p grainlify-core
```
