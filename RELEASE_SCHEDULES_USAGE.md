# Time-Based Release Schedules (Vesting) - Usage Guide

## Overview

The time-based release schedules feature enables vesting and milestone-based payouts for both bounty escrow and program escrow contracts. This allows funds to be released automatically or manually at specific intervals.

## Features

- ✅ **Time-based vesting** with specific release timestamps
- ✅ **Automatic releases** (anyone can trigger after timestamp)
- ✅ **Manual releases** (admin/authorized key can trigger anytime)
- ✅ **Multiple schedules** per bounty/program
- ✅ **Complete audit trail** with release history
- ✅ **Event emission** for all operations
- ✅ **Comprehensive view functions** for querying schedules

## Bounty Escrow Usage

### 1. Create a Release Schedule

```rust
// Admin creates a vesting schedule for a bounty
contract.create_release_schedule(
    &bounty_id,           // u64: Bounty identifier
    &amount,              // i128: Amount to release (in smallest token unit)
    &release_timestamp,   // u64: Unix timestamp when release becomes available
    &recipient_address,   // Address: Who receives the funds
);
```

### 2. Automatic Release (After Timestamp)

```rust
// Anyone can trigger automatic release after timestamp
contract.release_schedule_automatic(
    &bounty_id,    // u64: Bounty identifier
    &schedule_id,  // u64: Schedule identifier
);
```

### 3. Manual Release (Admin Only)

```rust
// Admin can release anytime, even before timestamp
contract.release_schedule_manual(
    &bounty_id,    // u64: Bounty identifier
    &schedule_id,  // u64: Schedule identifier
);
```

### 4. Query Schedules

```rust
// Get specific schedule
let schedule = contract.get_release_schedule(&bounty_id, &schedule_id);

// Get all schedules for a bounty
let all_schedules = contract.get_all_release_schedules(&bounty_id);

// Get pending (unreleased) schedules
let pending = contract.get_pending_schedules(&bounty_id);

// Get schedules ready for automatic release
let due = contract.get_due_schedules(&bounty_id);

// Get complete release history
let history = contract.get_release_history(&bounty_id);
```

## Program Escrow Usage

### 1. Create a Program Release Schedule

```rust
// Authorized payout key creates a vesting schedule
contract.create_program_release_schedule(
    &program_id,         // String: Program identifier
    &amount,             // i128: Amount to release
    &release_timestamp,  // u64: Unix timestamp
    &recipient_address,  // Address: Winner/participant
);
```

### 2. Automatic Release (After Timestamp)

```rust
// Anyone can trigger automatic release after timestamp
contract.release_prog_schedule_automatic(
    &program_id,   // String: Program identifier
    &schedule_id,  // u64: Schedule identifier
);
```

### 3. Manual Release (Authorized Key Only)

```rust
// Authorized key can release anytime
contract.release_prog_schedule_manual(
    &program_id,   // String: Program identifier
    &schedule_id,  // u64: Schedule identifier
);
```

### 4. Query Program Schedules

```rust
// Get specific program schedule
let schedule = contract.get_program_release_schedule(&program_id, &schedule_id);

// Get all schedules for a program
let all_schedules = contract.get_all_prog_release_schedules(&program_id);

// Get pending schedules
let pending = contract.get_pending_program_schedules(&program_id);

// Get schedules ready for automatic release
let due = contract.get_due_program_schedules(&program_id);

// Get complete release history
let history = contract.get_program_release_history(&program_id);
```

## Event Types

### Schedule Created Events

**Bounty Escrow:**
```rust
ScheduleCreated {
    bounty_id: u64,
    schedule_id: u64,
    amount: i128,
    release_timestamp: u64,
    recipient: Address,
    created_by: Address,
}
```

**Program Escrow:**
```rust
ProgramScheduleCreated {
    program_id: String,
    schedule_id: u64,
    amount: i128,
    release_timestamp: u64,
    recipient: Address,
    created_by: Address,
}
```

### Schedule Released Events

**Bounty Escrow:**
```rust
ScheduleReleased {
    bounty_id: u64,
    schedule_id: u64,
    amount: i128,
    recipient: Address,
    released_at: u64,
    released_by: Address,
    release_type: ReleaseType, // Automatic | Manual
}
```

**Program Escrow:**
```rust
ProgramScheduleReleased {
    program_id: String,
    schedule_id: u64,
    amount: i128,
    recipient: Address,
    released_at: u64,
    released_by: Address,
    release_type: ReleaseType, // Automatic | Manual
}
```

## Use Case Examples

### 1. Long-term Project Vesting

```rust
// 4-year vesting schedule with quarterly releases
let quarterly_timestamp = start_time + (90 * 24 * 60 * 60);
let quarterly_amount = total_amount / 16;

for quarter in 0..16 {
    contract.create_release_schedule(
        &bounty_id,
        &quarterly_amount,
        &quarterly_timestamp + (quarter * 90 * 24 * 60 * 60),
        &developer_address,
    );
}
```

### 2. Hackathon Prize Distribution

```rust
// Immediate prize + milestone bonuses
contract.create_program_release_schedule(
    &"Hackathon2024",
    &1000_0000000,  // Main prize
    &current_time,   // Immediate
    &winner_address,
);

contract.create_program_release_schedule(
    &"Hackathon2024",
    &200_0000000,   // Documentation bonus
    &(current_time + 7 * 24 * 60 * 60),  // 1 week later
    &winner_address,
);
```

### 3. Milestone-based Development

```rust
// Development milestones with different completion times
let milestones = vec![
    (30 * 24 * 60 * 60, 500_0000000),  // 30 days: MVP
    (60 * 24 * 60 * 60, 300_0000000),  // 60 days: Beta
    (90 * 24 * 60 * 60, 200_0000000),  // 90 days: Production
];

for (days_offset, amount) in milestones {
    contract.create_release_schedule(
        &bounty_id,
        &amount,
        &(start_time + days_offset),
        &developer_address,
    );
}
```

## Security Considerations

1. **Authorization**: Only admins can create bounty schedules; only authorized payout keys can create program schedules
2. **Balance Validation**: Schedules cannot exceed available escrow/program funds
3. **Timestamp Validation**: Release timestamps must be in the future
4. **Duplicate Prevention**: Each schedule ID is unique per bounty/program
5. **Audit Trail**: Complete history tracking for transparency

## Best Practices

1. **Plan Ahead**: Create all schedules upfront when funds are locked
2. **Use Clear Timestamps**: Use Unix timestamps for consistency
3. **Monitor Events**: Listen for schedule creation and release events
4. **Check Due Schedules**: Regularly query for schedules ready for automatic release
5. **Validate Balances**: Ensure sufficient funds before creating schedules

## Error Handling

Common errors and their meanings:

- `ScheduleAlreadyReleased`: Attempting to release an already released schedule
- `ScheduleNotFound`: Querying a non-existent schedule
- `InsufficientBalance`: Creating schedules that exceed available funds
- `InvalidTimestamp`: Using a past timestamp for release
- `Unauthorized`: Non-admin trying to create bounty schedules or non-authorized key for program schedules

## Integration Tips

1. **Event Monitoring**: Set up event listeners for `schedule_created` and `schedule_released` events
2. **Automated Releases**: Create bots to automatically trigger releases when timestamps pass
3. **Dashboard Integration**: Use view functions to display pending and due schedules
4. **Batch Operations**: Process multiple due schedules in batches for efficiency
