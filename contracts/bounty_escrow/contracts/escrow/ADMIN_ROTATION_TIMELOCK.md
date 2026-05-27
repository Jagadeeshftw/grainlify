# Two-Step Admin Rotation with Timelock

## Overview

The Bounty Escrow contract implements a **two-step admin rotation mechanism** with configurable timelock to protect against compromised admin keys and ensure secure governance transitions.

## Security Model

### Why Two-Step Rotation?

Single-step admin rotation is vulnerable to:
- **Compromised admin keys**: An attacker with admin access can immediately transfer control
- **Accidental transfers**: Admin mistakes cannot be undone
- **No community visibility**: Changes happen instantly without warning

Two-step rotation with timelock provides:
- **Window for intervention**: Community can react if rotation is malicious
- **Confirmation step**: New admin must explicitly accept responsibility
- **Audit trail**: All rotations are publicly visible on-chain

## Architecture

### Two-Step Flow

```
Step 1: PROPOSE (Current Admin)
   ↓
[TIMELOCK PERIOD] - Configurable delay (1 hour to 30 days)
   ↓
Step 2: ACCEPT (Pending Admin)
   ↓
Rotation Complete
```

### ASCII Timeline

```
T=0          T=0+timelock      T=0+timelock+ε
|  PROPOSE   |   WAITING      |   ACCEPT
|------------|----------------|------------>
              ↑ earliest acceptance point
```

## Contract Interface

### Core Functions

#### 1. `propose_admin_rotation(new_admin: Address) -> Result<u64, Error>`

Proposes a new admin address. The current admin retains full authority until the rotation is accepted.

**Authorization**: Current admin only  
**Returns**: `execute_after` timestamp when rotation becomes acceptable  
**Emits**: `AdminRotationProposed` event

**Constraints**:
- Cannot propose rotation to self
- Cannot propose if rotation already pending
- Contract must be initialized

**Example**:
```rust
let execute_after = client.propose_admin_rotation(&new_admin);
// execute_after is the earliest time the new admin can accept
```

#### 2. `accept_admin_rotation() -> Result<Address, Error>`

Accepts a pending admin rotation after the timelock has elapsed.

**Authorization**: Pending admin only  
**Returns**: New admin address  
**Emits**: `AdminRotationAccepted` event

**Constraints**:
- Rotation must be pending
- Timelock must have elapsed
- Pending admin must authorize the transaction

**Example**:
```rust
// Advance time past timelock
env.ledger().set_timestamp(execute_after + 1);

// Accept the rotation
let new_admin = client.accept_admin_rotation();
```

#### 3. `cancel_admin_rotation() -> Result<(), Error>`

Cancels a pending admin rotation. Can be called at any time before acceptance.

**Authorization**: Current admin only  
**Emits**: `AdminRotationCancelled` event

**Example**:
```rust
// Cancel if rotation was made in error or is suspicious
client.cancel_admin_rotation();
```

#### 4. `set_rotation_timelock_duration(duration: u64) -> Result<(), Error>`

Updates the timelock duration for **future** admin rotations. Does not affect pending rotations.

**Authorization**: Current admin only  
**Constraints**: Must be between 1 hour and 30 days  
**Emits**: `AdminRotationTimelockUpdated` event

**Example**:
```rust
// Set to 48 hours
client.set_rotation_timelock_duration(&172_800);
```

### Query Functions

#### `get_admin_rotation_status() -> Option<AdminRotationStatus>`

Returns comprehensive information about a pending rotation.

**Returns**: `Some(AdminRotationStatus)` if rotation is pending, `None` otherwise

```rust
pub struct AdminRotationStatus {
    pub current_admin: Address,       // Current active admin
    pub pending_admin: Address,       // Admin waiting to accept
    pub execute_after: u64,           // Earliest acceptance timestamp
    pub is_executable: bool,          // Whether timelock has elapsed
    pub remaining_seconds: u64,       // Seconds until executable (0 if ready)
    pub timestamp: u64,               // Query timestamp
}
```

#### `get_admin_rotation_config() -> AdminRotationConfig`

Returns the current admin rotation configuration.

```rust
pub struct AdminRotationConfig {
    pub timelock_duration: u64,       // Current duration in seconds
    pub min_timelock: u64,            // Minimum allowed (3,600s = 1 hour)
    pub max_timelock: u64,            // Maximum allowed (2,592,000s = 30 days)
    pub has_pending_rotation: bool,   // Whether rotation is in progress
    pub timestamp: u64,               // Query timestamp
}
```

#### `get_rotation_timelock_duration() -> u64`

Returns the current timelock duration (defaults to 24 hours).

#### `get_pending_admin() -> Option<Address>`

Returns the pending admin address if a rotation is in progress.

#### `get_admin_rotation_timelock() -> Option<u64>`

Returns the execute_after timestamp for the pending rotation.

## Configuration

### Timelock Bounds

| Parameter | Value | Description |
|-----------|-------|-------------|
| **Minimum** | 3,600 seconds (1 hour) | Prevents instant rotations |
| **Default** | 86,400 seconds (24 hours) | Balanced security/usability |
| **Maximum** | 2,592,000 seconds (30 days) | Prevents excessive delays |

### Storage Keys

The following `DataKey` variants are used for admin rotation:

```rust
pub enum DataKey {
    // ... other keys ...
    
    /// Pending admin address awaiting timelock expiry and acceptance.
    PendingAdmin,
    
    /// Timestamp after which the pending admin rotation can be executed.
    AdminTimelock,
    
    /// Configurable timelock duration for admin rotations (in seconds).
    TimelockDuration,
    
    /// Deprecated: Legacy admin transfer timestamp (kept for upgrade compatibility).
    AdminTransferTimestamp,
}
```

## Events

### `AdminRotationProposed`

Emitted when current admin proposes a new admin.

```rust
pub struct AdminRotationProposed {
    pub version: u32,              // EVENT_VERSION_V2 (2)
    pub current_admin: Address,    // Proposing admin
    pub pending_admin: Address,    // Proposed new admin
    pub timelock_duration: u64,    // Duration in seconds
    pub execute_after: u64,        // Earliest acceptance timestamp
    pub timestamp: u64,            // Proposal timestamp
}
```

**Topic**: `"admrotp"`

### `AdminRotationAccepted`

Emitted when pending admin accepts the rotation.

```rust
pub struct AdminRotationAccepted {
    pub version: u32,              // EVENT_VERSION_V2 (2)
    pub previous_admin: Address,   // Old admin
    pub new_admin: Address,        // New admin (was pending)
    pub timestamp: u64,            // Acceptance timestamp
}
```

**Topic**: `"admrota"`

### `AdminRotationCancelled`

Emitted when current admin cancels a pending rotation.

```rust
pub struct AdminRotationCancelled {
    pub version: u32,                   // EVENT_VERSION_V2 (2)
    pub admin: Address,                 // Cancelling admin
    pub cancelled_pending_admin: Address, // Cancelled pending admin
    pub timestamp: u64,                 // Cancellation timestamp
}
```

**Topic**: `"admrotc"`

### `AdminRotationTimelockUpdated`

Emitted when timelock duration is changed.

```rust
pub struct AdminRotationTimelockUpdated {
    pub version: u32,              // EVENT_VERSION_V2 (2)
    pub admin: Address,            // Admin making the change
    pub previous_duration: u64,    // Old duration
    pub new_duration: u64,         // New duration
    pub timestamp: u64,            // Update timestamp
}
```

**Topic**: `"admtlcfg"`

## Error Codes

| Error Code | Value | Description |
|------------|-------|-------------|
| `AdminRotationAlreadyPending` | 47 | A prior rotation must be accepted or cancelled first |
| `AdminRotationNotPending` | 48 | No admin rotation is currently pending |
| `AdminRotationTimelockActive` | 49 | Pending admin must wait for timelock to elapse |
| `InvalidAdminRotationTimelock` | 50 | Timelock duration outside accepted bounds |
| `InvalidAdminRotationTarget` | 51 | Proposed admin target is invalid (e.g., self-rotation) |

## Security Invariants

### 1. No Privilege Escalation
- Pending admin has **no authority** until rotation is accepted
- Current admin retains **full control** during timelock period
- Only current admin can cancel pending rotations

### 2. Authorization Requirements
- `propose_admin_rotation`: Requires current admin signature
- `accept_admin_rotation`: Requires pending admin signature
- `cancel_admin_rotation`: Requires current admin signature
- `set_rotation_timelock_duration`: Requires current admin signature

### 3. Timelock Enforcement
- Acceptance before timelock expiry **always fails**
- Timelock is calculated at proposal time, not acceptance time
- Changing timelock duration does **not** affect pending rotations

### 4. State Cleanup
- Successful acceptance clears `PendingAdmin` and `AdminTimelock`
- Cancellation clears `PendingAdmin` and `AdminTimelock`
- No stale state remains after rotation completes or is cancelled

## Upgrade Safety

### Backward Compatibility

The admin rotation system is designed for upgrade safety:

1. **Additive Storage Keys**: New keys (`PendingAdmin`, `AdminTimelock`, `TimelockDuration`) are optional-by-default
2. **Legacy Compatibility**: Old `AdminTransferTimestamp` key is preserved for upgrade migration
3. **Safe Defaults**: `TimelockDuration` defaults to 24 hours if not set
4. **No Breaking Changes**: Existing admin functions continue to work

### Migration Notes

When upgrading from legacy admin transfer (`propose_admin`/`accept_admin`):

```rust
// Legacy system (deprecated but still functional):
// - propose_admin(new_admin)
// - accept_admin()
// - Uses hardcoded 24-hour timelock
// - Storage: PendingAdmin, AdminTransferTimestamp

// New system (recommended):
// - propose_admin_rotation(new_admin)
// - accept_admin_rotation()
// - Uses configurable timelock
// - Storage: PendingAdmin, AdminTimelock, TimelockDuration
```

**Important**: The legacy and new systems share the `PendingAdmin` key. Do not mix them in the same rotation cycle.

## Testing

Run the comprehensive test suite:

```bash
cd contracts/bounty_escrow/contracts/escrow
cargo test test_admin_rotation
```

### Test Coverage

The test suite validates:

- ✅ Proposal flow with proper authorization
- ✅ Timelock enforcement (cannot accept before delay)
- ✅ Acceptance flow with pending admin authorization
- ✅ Cancellation by current admin
- ✅ Timelock duration configuration (min/max bounds)
- ✅ Edge cases (duplicate proposals, self-rotation, etc.)
- ✅ Upgrade safety (storage keys persist correctly)
- ✅ Event emission for all state changes
- ✅ Query functions return correct state
- ✅ Full end-to-end rotation flows
- ✅ Multiple sequential rotations

## Best Practices

### For Admins

1. **Verify New Admin Address**: Double-check the pending admin address before proposing
2. **Communicate Rotation**: Notify community/stakeholders before proposing rotation
3. **Monitor Pending Rotations**: Regularly check for unauthorized rotation attempts
4. **Cancel Suspicious Rotations**: Use `cancel_admin_rotation` immediately if compromise suspected
5. **Choose Appropriate Timelock**: Balance security (longer) vs. usability (shorter)

### For Pending Admins

1. **Verify Contract State**: Check escrow balances and state before accepting
2. **Accept Promptly**: Don't leave rotations pending longer than necessary
3. **Secure Keys**: Ensure admin keys are properly secured before accepting

### For Indexers/Monitoring

1. **Subscribe to Events**: Monitor all admin rotation events
2. **Track Rotation Status**: Use `get_admin_rotation_status()` for real-time monitoring
3. **Alert on Proposals**: Notify stakeholders when rotations are proposed
4. **Verify Acceptance**: Confirm rotations complete successfully

## Comparison: Legacy vs New System

| Feature | Legacy System | New System |
|---------|--------------|------------|
| **Functions** | `propose_admin`, `accept_admin` | `propose_admin_rotation`, `accept_admin_rotation` |
| **Timelock** | Hardcoded 24 hours | Configurable (1 hour - 30 days) |
| **Error Handling** | Panics on failure | Returns `Result<T, Error>` |
| **Query Functions** | Limited | Comprehensive status/config queries |
| **Event Versioning** | Basic | EVENT_VERSION_V2 with full payloads |
| **Upgrade Safety** | Basic | Enhanced with migration support |

## Future Enhancements

Potential improvements for future versions:

1. **Multi-sig Approval**: Require multiple admins to propose/accept rotations
2. **Emergency Override**: Time-locked emergency rotation for critical situations
3. **Rotation History**: On-chain log of all past rotations
4. **Veto Mechanism**: Community veto power for suspicious rotations
5. **Gradual Permission Transfer**: Phased admin permission migration
