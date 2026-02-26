# Fee Override Design Document

## Overview

This document describes the per-program and per-escrow fee override feature that allows customizing platform fees at granular levels for partnerships, promotions, and special campaigns.

## Motivation

The global fee configuration may not suit all use cases:
- **Partner Programs**: Strategic partners may negotiate reduced fees
- **Promotional Campaigns**: Time-limited fee waivers to drive adoption
- **Special Events**: Custom fee structures for hackathons or competitions
- **Testing**: Fee-free environments for development and testing

## Architecture

### Fee Precedence Hierarchy

The system resolves fees using the following precedence (highest to lowest):

```
Escrow/Program Override > Global Configuration
```

This simple two-tier system provides flexibility while maintaining clarity.

### Data Structures

#### Bounty Escrow Contract

```rust
pub struct Escrow {
    // ... existing fields ...
    
    /// Optional per-escrow fee rate override for lock operations (basis points)
    /// If None, uses global fee rate
    pub lock_fee_override: Option<i128>,
    
    /// Optional per-escrow fee rate override for release operations (basis points)
    /// If None, uses global fee rate
    pub release_fee_override: Option<i128>,
}
```

#### Program Escrow Contract

```rust
pub struct ProgramData {
    // ... existing fields ...
    
    /// Optional per-program fee rate override for lock operations (basis points)
    /// If None, uses global fee rate
    pub lock_fee_override: Option<i128>,
    
    /// Optional per-program fee rate override for payout operations (basis points)
    /// If None, uses global fee rate
    pub payout_fee_override: Option<i128>,
}
```

### Fee Resolution Functions

#### Bounty Escrow

```rust
/// Resolve effective lock fee rate with precedence: escrow override > global
fn get_effective_lock_fee_rate(env: &Env, bounty_id: u64) -> (i128, bool)

/// Resolve effective release fee rate with precedence: escrow override > global
fn get_effective_release_fee_rate(env: &Env, bounty_id: u64) -> (i128, bool)
```

#### Program Escrow

```rust
/// Resolve effective lock fee rate with precedence: program override > global
fn get_effective_lock_fee_rate(env: &Env, program_id: &String) -> (i128, bool)

/// Resolve effective payout fee rate with precedence: program override > global
fn get_effective_payout_fee_rate(env: &Env, program_id: &String) -> (i128, bool)
```

Both functions return a tuple of `(effective_rate, is_overridden)` for transparency.

## Public API

### Bounty Escrow Contract

```rust
/// Set per-escrow fee overrides (admin only)
/// 
/// # Parameters
/// - `bounty_id`: The escrow to configure
/// - `lock_fee_override`: Optional lock fee rate (0-5000 basis points), None to use global
/// - `release_fee_override`: Optional release fee rate (0-5000 basis points), None to use global
///
/// # Authorization
/// - Requires admin authorization
///
/// # Errors
/// - `NotInitialized`: Contract not initialized
/// - `BountyNotFound`: Escrow doesn't exist
/// - `InvalidFeeRate`: Rate outside valid range (0-5000)
/// - `Unauthorized`: Caller is not admin
pub fn set_escrow_fee_override(
    env: Env,
    bounty_id: u64,
    lock_fee_override: Option<i128>,
    release_fee_override: Option<i128>,
) -> Result<(), Error>
```

### Program Escrow Contract

```rust
/// Set per-program fee overrides (admin only)
///
/// # Parameters
/// - `program_id`: The program to configure
/// - `lock_fee_override`: Optional lock fee rate (0-1000 basis points), None to use global
/// - `payout_fee_override`: Optional payout fee rate (0-1000 basis points), None to use global
///
/// # Authorization
/// - Requires admin authorization
///
/// # Panics
/// - "Admin not set": No admin configured
/// - "Program not found": Program doesn't exist
/// - "Invalid lock/payout fee override": Rate outside valid range (0-1000)
pub fn set_program_fee_override(
    env: Env,
    program_id: String,
    lock_fee_override: Option<i128>,
    payout_fee_override: Option<i128>,
)
```

## Events

### Bounty Escrow Fee Override Event

```rust
// Event topic: "fee_ovr"
// Event data: (bounty_id, lock_fee_override, release_fee_override, timestamp)
// Note: -1 indicates None (no override)
```

### Program Escrow Fee Override Event

```rust
// Event topic: "prg_fovr"
// Event data: (program_id, lock_fee_override, payout_fee_override, timestamp)
// Note: -1 indicates None (no override)
```

### Enhanced Fee Collected Event

```rust
pub struct FeeCollected {
    pub version: u32,
    pub operation_type: FeeOperationType,
    pub amount: i128,
    pub fee_rate: i128,              // Global/configured rate
    pub effective_fee_rate: i128,    // Actual rate used (after overrides)
    pub recipient: Address,
    pub timestamp: u64,
}
```

The `effective_fee_rate` field provides transparency about which rate was actually applied.

## Usage Examples

### Example 1: Partner Program with Reduced Fees

```rust
// Set up a partner program with 50% reduced fees
let partner_program_id = String::from_str(&env, "PartnerHackathon2024");

// Global fees: 2% lock, 3% payout
// Partner fees: 1% lock, 1.5% payout
client.set_program_fee_override(
    &partner_program_id,
    &Some(100),  // 1% lock fee (100 basis points)
    &Some(150),  // 1.5% payout fee (150 basis points)
);
```

### Example 2: Fee-Free Promotional Escrow

```rust
// Create a promotional bounty with zero fees
let promo_bounty_id = 12345;

client.set_escrow_fee_override(
    &promo_bounty_id,
    &Some(0),  // 0% lock fee
    &Some(0),  // 0% release fee
);
```

### Example 3: Remove Override (Revert to Global)

```rust
// Remove overrides to use global fees again
client.set_escrow_fee_override(
    &bounty_id,
    &None,  // Use global lock fee
    &None,  // Use global release fee
);
```

## Security Considerations

### Authorization

- Only contract admin can set fee overrides
- Prevents unauthorized fee manipulation
- Admin key should be properly secured (multisig recommended)

### Validation

- Override rates must be within valid range (0-MAX_FEE_RATE)
- Bounty/program must exist before setting overrides
- Invalid rates are rejected with clear error messages

### Audit Trail

- All override changes emit events with timestamps
- Events include both old and new values (via -1 for None)
- Fee collection events include effective rate for transparency

### Rate Limits

- Same MAX_FEE_RATE applies to overrides as global config
- Bounty Escrow: 0-5000 basis points (0-50%)
- Program Escrow: 0-1000 basis points (0-10%)

## Migration Path

### Existing Escrows/Programs

- Existing escrows/programs have `None` for override fields
- They automatically use global fees (backward compatible)
- No migration required

### Storage Impact

- Adds 2 optional fields per escrow/program
- Minimal storage overhead (Option<i128> is compact)
- No impact on existing data structures

## Testing Strategy

### Unit Tests

1. **Override Precedence**
   - Verify escrow override takes precedence over global
   - Verify program override takes precedence over global
   - Verify None falls back to global

2. **Validation**
   - Test invalid rates (negative, above MAX_FEE_RATE)
   - Test non-existent bounty/program
   - Test unauthorized access

3. **Edge Cases**
   - Zero fee overrides
   - Maximum fee overrides
   - Removing overrides (None)
   - Multiple override changes

4. **Events**
   - Verify override events are emitted
   - Verify effective rate in fee collection events
   - Verify event data accuracy

### Integration Tests

1. **End-to-End Flows**
   - Lock funds with override → verify correct fee
   - Release funds with override → verify correct fee
   - Payout with override → verify correct fee

2. **Mixed Scenarios**
   - Some escrows with overrides, some without
   - Some programs with overrides, some without
   - Changing overrides mid-lifecycle

## Future Enhancements

### Potential Extensions

1. **Time-Bound Overrides**
   - Add start_time and end_time to overrides
   - Automatically revert to global after period

2. **Tiered Overrides**
   - Different rates based on amount ranges
   - Volume-based discounts

3. **Category-Based Overrides**
   - Override by bounty type or program category
   - Bulk override management

4. **Override Templates**
   - Predefined override profiles
   - Easy application to multiple entities

## References

- [Bounty Escrow Contract](./bounty_escrow/contracts/escrow/src/lib.rs)
- [Program Escrow Contract](./program-escrow/src/lib.rs)
- [Fee Configuration](./bounty_escrow/contracts/escrow/src/lib.rs#L580-L605)
- [Event Definitions](./bounty_escrow/contracts/escrow/src/events.rs)

## Changelog

- **2024-02-26**: Initial design document
  - Added per-escrow and per-program fee overrides
  - Implemented precedence hierarchy
  - Added admin-only override functions
  - Enhanced fee collection events with effective rate
