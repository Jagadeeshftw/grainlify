# Query Functions Quick Reference

## Bounty Escrow

### Basic Queries
```rust
// Get single escrow
let escrow = contract.get_escrow_info(env, bounty_id)?;

// Get total count
let count = contract.get_escrow_count(env);

// Get contract balance
let balance = contract.get_balance(env)?;
```

### Filtered Queries
```rust
// All locked escrows
let filter = EscrowQueryFilter {
    status: Some(EscrowStatus::Locked),
    depositor: None,
    min_amount: None,
    max_amount: None,
    min_deadline: None,
    max_deadline: None,
};
let results = contract.query_escrows(env, filter, 0, 50);

// Escrows by depositor
let results = contract.query_escrows_by_depositor(env, depositor_addr, 0, 50);

// High-value escrows expiring soon
let filter = EscrowQueryFilter {
    status: Some(EscrowStatus::Locked),
    depositor: None,
    min_amount: Some(10000),
    max_amount: None,
    min_deadline: None,
    max_deadline: Some(current_time + 86400), // 24 hours
};
let results = contract.query_escrows(env, filter, 0, 50);
```

### Status Queries
```rust
// Get locked escrow IDs
let locked_ids = contract.get_escrows_by_status(env, EscrowStatus::Locked, 0, 100);

// Get released escrow IDs
let released_ids = contract.get_escrows_by_status(env, EscrowStatus::Released, 0, 100);
```

### Aggregate Statistics
```rust
let stats = contract.get_aggregate_stats(env);
// stats.total_locked
// stats.total_released
// stats.total_refunded
// stats.count_locked
// stats.count_released
// stats.count_refunded
```

## Program Escrow

### Basic Queries
```rust
// Get program info
let info = contract.get_program_info(env);

// Get remaining balance
let balance = contract.get_remaining_balance(env);

// Get all schedules
let schedules = contract.get_program_release_schedules(env);

// Get release history
let history = contract.get_program_release_history(env);
```

### Payout History Queries
```rust
// All payouts
let filter = PayoutQueryFilter {
    recipient: None,
    min_amount: None,
    max_amount: None,
    min_timestamp: None,
    max_timestamp: None,
};
let payouts = contract.query_payout_history(env, filter, 0, 50);

// Payouts by recipient
let payouts = contract.get_payouts_by_recipient(env, recipient_addr, 0, 50);

// Large payouts in date range
let filter = PayoutQueryFilter {
    recipient: None,
    min_amount: Some(5000),
    max_amount: None,
    min_timestamp: Some(start_date),
    max_timestamp: Some(end_date),
};
let payouts = contract.query_payout_history(env, filter, 0, 50);
```

### Schedule Queries
```rust
// All pending schedules
let pending = contract.get_pending_schedules(env);

// Schedules ready to release
let due = contract.get_due_schedules(env);

// Total scheduled amount
let total = contract.get_total_scheduled_amount(env);

// Filtered schedules
let filter = ScheduleQueryFilter {
    recipient: Some(recipient_addr),
    released: Some(false),
    min_amount: None,
    max_amount: None,
    min_release_timestamp: None,
    max_release_timestamp: None,
};
let schedules = contract.query_release_schedules(env, filter, 0, 50);
```

### Release History Queries
```rust
// All releases
let history = contract.query_release_history(env, None, 0, 50);

// Releases for specific recipient
let history = contract.query_release_history(env, Some(recipient_addr), 0, 50);
```

### Aggregate Statistics
```rust
let stats = contract.get_program_aggregate_stats(env);
// stats.total_funds
// stats.remaining_balance
// stats.total_paid_out
// stats.payout_count
// stats.scheduled_count
// stats.released_count
```

## Common Patterns

### Pagination Loop
```rust
let page_size = 50;
let mut offset = 0;
let mut all_results = Vec::new();

loop {
    let page = contract.query_escrows(env.clone(), filter.clone(), offset, page_size);
    if page.is_empty() {
        break;
    }
    all_results.extend(page);
    offset += page_size;
}
```

### Dashboard Stats
```rust
// Bounty Escrow Dashboard
let stats = contract.get_aggregate_stats(env.clone());
let user_escrows = contract.query_escrows_by_depositor(env.clone(), user_addr, 0, 10);
let total_count = contract.get_escrow_count(env);

// Program Escrow Dashboard
let stats = contract.get_program_aggregate_stats(env.clone());
let user_payouts = contract.get_payouts_by_recipient(env.clone(), user_addr, 0, 10);
let pending = contract.get_pending_schedules(env.clone());
let due = contract.get_due_schedules(env);
```

### Monitoring Alerts
```rust
// Check for expiring escrows
let soon = current_time + 3600; // 1 hour
let filter = EscrowQueryFilter {
    status: Some(EscrowStatus::Locked),
    max_deadline: Some(soon),
    ..Default::default()
};
let expiring = contract.query_escrows(env, filter, 0, 100);

// Check for due releases
let due = contract.get_due_schedules(env);
if !due.is_empty() {
    trigger_release_process();
}
```

### Analytics Queries
```rust
// Daily report
let day_start = get_day_start_timestamp();
let day_end = get_day_end_timestamp();

let filter = PayoutQueryFilter {
    min_timestamp: Some(day_start),
    max_timestamp: Some(day_end),
    ..Default::default()
};
let daily_payouts = contract.query_payout_history(env, filter, 0, 1000);

// Calculate daily total
let daily_total: i128 = daily_payouts.iter().map(|p| p.amount).sum();
```

## Filter Construction Helpers

```rust
// Empty filter (no filtering)
let filter = EscrowQueryFilter {
    status: None,
    depositor: None,
    min_amount: None,
    max_amount: None,
    min_deadline: None,
    max_deadline: None,
};

// Status only
let filter = EscrowQueryFilter {
    status: Some(EscrowStatus::Locked),
    ..Default::default()
};

// Amount range
let filter = EscrowQueryFilter {
    min_amount: Some(1000),
    max_amount: Some(10000),
    ..Default::default()
};

// Time range
let filter = PayoutQueryFilter {
    min_timestamp: Some(start),
    max_timestamp: Some(end),
    ..Default::default()
};
```

## Performance Tips

1. **Use specific queries**: `query_escrows_by_depositor()` is faster than filtering all escrows
2. **Limit page size**: Keep it under 100 records per query
3. **Cache aggregates**: Update on events rather than querying repeatedly
4. **Filter early**: Apply filters to reduce data transfer
5. **Index off-chain**: For complex analytics, maintain a database indexed by events
