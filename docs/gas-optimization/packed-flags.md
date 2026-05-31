# Packed-Flag Storage Optimization

## Overview

Packed-flag storage is a gas optimization technique that stores multiple boolean flags in a single `u32` value, reducing storage slot consumption from N booleans to 1 packed value. This document describes the implementation and test coverage for the program-escrow contract's packed pause flags.

## Bit Layout

The `packed_storage` module defines 8 granular pause flags:

| Bit | Constant | Description |
|-----|----------|-------------|
| 0 | `PAUSE_LOCK` | Locks program fund deposits |
| 1 | `PAUSE_RELEASE` | Locks scheduled fund releases |
| 2 | `PAUSE_REFUND` | Locks refund/cancel claims |
| 3 | `PAUSE_ARCHIVE` | Locks archival operations |
| 4 | `PAUSE_BATCH_LOCK` | Locks batch deposit operations |
| 5 | `PAUSE_BATCH_RELEASE` | Locks batch release operations |
| 6 | `PAUSE_BATCH_REFUND` | Locks batch refund operations |
| 7 | `PAUSE_EMERGENCY` | Locks emergency operations |

## Gas Savings

| Operation | Before (separate bools) | After (packed u32) | Savings |
|-----------|------------------------|-------------------|---------|
| 8 flag writes | 8 storage writes | 1 storage write | ~7 storage ops |
| 8 flag reads | 8 storage reads | 1 storage read + 8 bitwise ops | ~7 storage ops |
| Per-operation check | Storage read + bool read | Bitwise check on cached u32 | ~1 storage op |

Estimated gas savings: ~2000 units per flag operation (Soroban storage unit costs).

## API Reference

### Setting Flags

```rust
use crate::gas_optimization::packed_storage::{set_packed_flags, PAUSE_LOCK, PAUSE_RELEASE};
use soroban_sdk::Symbol;

/// Set specific pause flags atomically
pub fn set_packed_flags(env: &Env, key: &Symbol, flags: u32)
```

**Security Notes:** Uses atomic bitwise OR to combine new flags with existing state. Multiple flags can be combined with `|` operator.

### Clearing Flags

```rust
use crate::gas_optimization::packed_storage::{clear_packed_flags, PAUSE_LOCK};

/// Clear specific flags while preserving others
pub fn clear_packed_flags(env: &Env, key: &Symbol, flags: u32)
```

**Security Notes:** Uses atomic bitwise AND with complement to clear only specified bits. Never clears unrelated flags.

### Checking Flags

```rust
use crate::gas_optimization::packed_storage::{has_packed_flags, PAUSE_LOCK};

/// Check if specific flags are set (all must be set)
pub fn has_packed_flags(env: &Env, key: &Symbol, flags: u32) -> bool

/// Check if any of the specified flags are set
pub fn has_any_packed_flags(env: &Env, key: &Symbol, flags: u32) -> bool

/// Retrieve the full packed value
pub fn get_packed_flags(env: &Env, key: &Symbol) -> u32
```

## Security Invariants

1. **Bitwise Isolation:** Setting bit A never affects bit B
2. **Round-trip Integrity:** `get_packed_flags(set_packed_flags(x)) == x` for all x in [0, 255]
3. **Atomic Updates:** Flags are written in a single storage operation
4. **No Overflow:** All 8 flags fit within a u32 without overflow risk

## Test Coverage

The packed-flag coverage tests verify:

- All 256 combinations (2^8) of flag states are correctly stored and retrieved
- Setting flag A does not corrupt flag B via bitwise operations
- Clearing flag A does not affect unrelated flags
- Storage size reduction is achieved (1 slot for 8 flags)
- Edge cases: empty storage, selective operations, toggle behavior

### Running Tests

```bash
cd contracts/program-escrow
cargo test --package program-escrow packed_flag_coverage
```

## Integration with ProgramData

The `ProgramData` struct in `lib.rs` uses the following packed fields:

```rust
pub struct ProgramData {
    pub risk_flags: u32,              // Packed: HIGH_RISK, UNDER_REVIEW, RESTRICTED, DEPRECATED
    pub delegate_permissions: u32,    // Packed: RELEASE, REFUND, UPDATE_META
    // ... other fields
}
```

### Delegate Permissions Bitmask

| Bit | Constant | Description |
|-----|----------|-------------|
| 0 | `DELEGATE_PERMISSION_RELEASE` | Can trigger scheduled releases |
| 1 | `DELEGATE_PERMISSION_REFUND` | Can cancel/claim refunds |
| 2 | `DELEGATE_PERMISSION_UPDATE_META` | Can update program metadata |

## Best Practices

1. **Use bitwise constants:** Always reference flag constants rather than magic numbers
2. **Combine with `|`:** Multiple flags can be set in one call: `set_packed_flags(env, key, PAUSE_LOCK | PAUSE_RELEASE)`
3. **Clear selectively:** Use `clear_packed_flags` to remove specific flags while preserving others
4. **Check before operations:** Use `has_packed_flags` or `has_any_packed_flags` for guard checks

## Migration Notes

When adding new flags:

1. Assign a new bit position (bits 0-7 are currently used)
2. Update `FLAG_MASK_8BITS` if expanding beyond 8 flags
3. Add corresponding test cases in `packed_flag_coverage_tests`
4. Document the new flag in this file

## References

- [Gas Optimization Summary](../GAS_OPTIMIZATION_SUMMARY.md)
- [Pause Management](../program-escrow/pause-management.md)
- [Batch Payout Benchmarks](batch-payout-benchmarks.md)