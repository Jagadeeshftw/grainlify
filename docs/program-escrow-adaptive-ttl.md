# Adaptive TTL Extension for Program-Escrow

## Overview

The Program Escrow contract manages the lifecycle of hackathon and program prize pools. A core part of storage management in Soroban is TTL (Time-To-Live) extension, which prevents active ledger entries from being archived and evicted from the state. 

Previously, `ProgramData` and `PayoutRecord` entries received a uniform TTL extension, regardless of how often a program was accessed. This meant a frequently-active program paid the same relative TTL-extension overhead as a rarely-touched program, which wasn't efficient for long-running but inactive programs approaching archival.

This document outlines the **Access-Frequency-Adaptive TTL Extension Policy** introduced to optimize storage costs.

## Mechanism

### Lightweight Tracking

To adapt TTL based on activity, the contract tracks a lightweight hotness signal for each program:

- **Signal Key**: `DataKey::ProgramAccessSignal(program_id)`
- **Signal Format**: A simple `u32` counter representing the total number of accesses/writes for a program.
- **Bounds**: The counter is capped at `100` (`TTL_MAX_ACCESS_COUNT`). This prevents unbounded storage value growth and caps the maximum TTL possible.

The signal increments whenever critical program state changes, specifically during:
1. Payout distributions (`store_program_data` and `append_recipient_index`).
2. Program locking and general state updates (`store_program_data`).

### Adaptive Extension Policy

When a write occurs, the contract determines the new TTL to set for the program's instance and persistent storage keys based on its access count. The policy scales the extension linearly between a safe minimum and maximum bound:

- **Cold Programs (Access Count = 0)**: Gets the base TTL extension of `518,400` ledgers (~30 days). This is the cheapest extension.
- **Hot Programs (Access Count = 100)**: Gets the maximum TTL extension of `3,110,400` ledgers (~180 days).

For programs in between, the formula is:
```rust
extra_ttl = (MAX_TTL - MIN_TTL) * access_count / MAX_ACCESS_COUNT
ttl_to_set = MIN_TTL + extra_ttl
```

### Affected Data Keys
- **Instance Storage**: Since `ProgramData` is stored in the contract's instance storage, the adaptive TTL policy extends the entire instance's TTL.
- **Persistent Storage**: For `PayoutRecord` entries (stored via `RecipientPayoutIndex` keys) and the `ProgramAccessSignal` itself, the contract extends the TTL for the specific persistent key.

## Reasoning for Operators

Operators can reason about worst-case archival timing easily:
- If a program becomes completely inactive, its last state change will leave it with a TTL proportional to its historical activity.
- Extremely hot programs will survive untouched for up to 180 days before falling to archival.
- Extremely cold/unsuccessful programs will expire much faster (within 30 days of their last action), saving network state rent.

## Security and Efficiency Assumptions
- The `access_count` is tracked using standard safe math. Because it is explicitly capped at `100`, overflow is structurally impossible, preserving security.
- The tracking adds minimal overhead (one `u32` read/write).
- The linear scaling requires simple integer arithmetic without expensive loops or arrays, making the gas profile cheap and deterministic.
