# Admin Rotation Quick Reference

## Two-Step Admin Rotation with Timelock

### Core Flow

```
Current Admin                Timelock Period              Pending Admin
     │                            │                            │
     ├── propose_admin_rotation ──▶│                            │
     │                            ├── WAIT (1h - 30 days) ─────▶│
     │                            │                            ├── accept_admin_rotation
     │                            │                            │
     │◀─────────────────────────────────────────────────────────┤
     │                 Rotation Complete                        │
```

### Functions

| Function | Auth | Returns | Description |
|----------|------|---------|-------------|
| `propose_admin_rotation(new_admin)` | Current Admin | `execute_after: u64` | Start rotation |
| `accept_admin_rotation()` | Pending Admin | `new_admin: Address` | Complete rotation |
| `cancel_admin_rotation()` | Current Admin | `()` | Abort rotation |
| `set_rotation_timelock_duration(secs)` | Current Admin | `()` | Update timelock |

### Query Functions

| Function | Returns | Description |
|----------|---------|-------------|
| `get_admin_rotation_status()` | `Option<AdminRotationStatus>` | Current rotation state |
| `get_admin_rotation_config()` | `AdminRotationConfig` | Timelock configuration |
| `get_rotation_timelock_duration()` | `u64` | Current timelock (seconds) |
| `get_pending_admin()` | `Option<Address>` | Pending admin address |
| `get_admin_rotation_timelock()` | `Option<u64>` | Execute-after timestamp |

### Configuration

| Parameter | Value | Description |
|-----------|-------|-------------|
| Default Timelock | 86,400s (24 hours) | Used if not configured |
| Minimum Timelock | 3,600s (1 hour) | Security lower bound |
| Maximum Timelock | 2,592,000s (30 days) | Usability upper bound |

### Error Codes

| Error | Code | When |
|-------|------|------|
| `AdminRotationAlreadyPending` | 47 | Duplicate proposal |
| `AdminRotationNotPending` | 48 | No rotation to accept/cancel |
| `AdminRotationTimelockActive` | 49 | Acceptance too early |
| `InvalidAdminRotationTimelock` | 50 | Duration out of bounds |
| `InvalidAdminRotationTarget` | 51 | Self-rotation attempt |

### Events

| Event | Topic | Emitted When |
|-------|-------|--------------|
| `AdminRotationProposed` | `"admrotp"` | Rotation proposed |
| `AdminRotationAccepted` | `"admrota"` | Rotation accepted |
| `AdminRotationCancelled` | `"admrotc"` | Rotation cancelled |
| `AdminRotationTimelockUpdated` | `"admtlcfg"` | Timelock changed |

### Example Usage

```rust
// 1. Initialize
client.init(&admin, &token);

// 2. Propose rotation (admin only)
let execute_after = client.propose_admin_rotation(&new_admin);

// 3. Check status
let status = client.get_admin_rotation_status();
// Returns: Some(AdminRotationStatus { current_admin, pending_admin, 
//          execute_after, is_executable, remaining_seconds, timestamp })

// 4. Wait for timelock
env.ledger().set_timestamp(execute_after + 1);

// 5. Accept rotation (pending admin only)
client.accept_admin_rotation();

// 6. Verify
assert_eq!(client.get_admin(), new_admin);
assert_eq!(client.get_pending_admin(), None);
```

### Security Checklist

- [ ] Proposed admin address verified (no typos)
- [ ] Community notified of rotation
- [ ] Timelock duration appropriate for security needs
- [ ] Pending admin ready to accept (keys secured)
- [ ] Monitor for unauthorized rotation attempts
- [ ] Cancel immediately if compromise suspected

### Testing

```bash
# Run admin rotation tests
cargo test test_admin_rotation

# Run specific test
cargo test test_propose_admin_rotation_success
```

### Documentation

- **Full Guide**: [ADMIN_ROTATION_TIMELOCK.md](ADMIN_ROTATION_TIMELOCK.md)
- **Implementation Summary**: [ADMIN_ROTATION_IMPLEMENTATION.md](ADMIN_ROTATION_IMPLEMENTATION.md)
- **Tests**: [src/test_admin_rotation.rs](src/test_admin_rotation.rs)
