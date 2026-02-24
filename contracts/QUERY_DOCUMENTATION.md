# Escrow Query Functions Documentation

## Overview

This document describes the query functions available for both Bounty Escrow and Program Escrow contracts, including usage patterns, filtering capabilities, and performance considerations.

## Bounty Escrow Query Functions

### 1. Query Escrows with Filtering

```rust
pub fn query_escrows(env: Env, filter: EscrowQueryFilter, offset: u32, limit: u32) -> Vec<EscrowWithId>
```

**Purpose**: Retrieve escrows with comprehensive filtering and pagination support.

**Parameters**:

- `filter`: EscrowQueryFilter with optional fields:
  - `status`: Filter by EscrowStatus (Locked, Released, Refunded)
  - `depositor`: Filter by depositor address
  - `min_amount`: Minimum escrow amount
  - `max_amount`: Maximum escrow amount
  - `min_deadline`: Minimum deadline timestamp
  - `max_deadline`: Maximum deadline timestamp
- `offset`: Number of records to skip (for pagination)
- `limit`: Maximum number of records to return

**Example Usage**:

```rust
// Query all locked escrows with amount >= 1000
let filter = EscrowQueryFilter {
    status: Some(EscrowStatus::Locked),
    depositor: None,
    min_amount: Some(1000),
    max_amount: None,
    min_deadline: None,
    max_deadline: None,
};
let results = contract.query_escrows(env, filter, 0, 50);
```

### 2. Query Escrows by Depositor

```rust
pub fn query_escrows_by_depositor(env: Env, depositor: Address, offset: u32, limit: u32) -> Vec<EscrowWithId>
```

**Purpose**: Efficiently retrieve all escrows created by a specific depositor using indexed storage.

**Performance**: O(n) where n = number of escrows by depositor (uses depositor index for fast lookup)

### 3. Get Aggregate Statistics

```rust
pub fn get_aggregate_stats(env: Env) -> AggregateStats
```

**Purpose**: Get comprehensive statistics across all escrows.

**Returns**:

- `total_locked`: Total amount in locked escrows
- `total_released`: Total amount released
- `total_refunded`: Total amount refunded
- `count_locked`: Number of locked escrows
- `count_released`: Number of released escrows
- `count_refunded`: Number of refunded escrows

### 4. Get Escrows by Status

```rust
pub fn get_escrows_by_status(env: Env, status: EscrowStatus, offset: u32, limit: u32) -> Vec<u64>
```

**Purpose**: Retrieve bounty IDs filtered by status with pagination.

### 5. Get Escrow Count

```rust
pub fn get_escrow_count(env: Env) -> u32
```

**Purpose**: Get total number of escrows in the system.

## Program Escrow Query Functions

### 1. Query Payout History

```rust
pub fn query_payout_history(env: Env, filter: PayoutQueryFilter, offset: u32, limit: u32) -> Vec<PayoutRecord>
```

**Purpose**: Retrieve payout history with filtering and pagination.

**Filter Options**:

- `recipient`: Filter by recipient address
- `min_amount`: Minimum payout amount
- `max_amount`: Maximum payout amount
- `min_timestamp`: Minimum timestamp
- `max_timestamp`: Maximum timestamp

### 2. Query Release Schedules

```rust
pub fn query_release_schedules(env: Env, filter: ScheduleQueryFilter, offset: u32, limit: u32) -> Vec<ProgramReleaseSchedule>
```

**Purpose**: Query scheduled releases with comprehensive filtering.

**Filter Options**:

- `recipient`: Filter by recipient address
- `released`: Filter by release status (true/false)
- `min_amount`: Minimum scheduled amount
- `max_amount`: Maximum scheduled amount
- `min_release_timestamp`: Minimum release timestamp
- `max_release_timestamp`: Maximum release timestamp

### 3. Query Release History

```rust
pub fn query_release_history(env: Env, recipient: Option<Address>, offset: u32, limit: u32) -> Vec<ProgramReleaseHistory>
```

**Purpose**: Retrieve completed release history, optionally filtered by recipient.

### 4. Get Program Aggregate Statistics

```rust
pub fn get_program_aggregate_stats(env: Env) -> ProgramAggregateStats
```

**Purpose**: Get comprehensive program statistics.

**Returns**:

- `total_funds`: Total funds locked in program
- `remaining_balance`: Current remaining balance
- `total_paid_out`: Total amount paid out
- `payout_count`: Number of payouts executed
- `scheduled_count`: Number of pending schedules
- `released_count`: Number of completed schedules

### 5. Get Payouts by Recipient

```rust
pub fn get_payouts_by_recipient(env: Env, recipient: Address, offset: u32, limit: u32) -> Vec<PayoutRecord>
```

**Purpose**: Retrieve all payouts for a specific recipient with pagination.

### 6. Get Pending Schedules

```rust
pub fn get_pending_schedules(env: Env) -> Vec<ProgramReleaseSchedule>
```

**Purpose**: Get all schedules that haven't been released yet.

### 7. Get Due Schedules

```rust
pub fn get_due_schedules(env: Env) -> Vec<ProgramReleaseSchedule>
```

**Purpose**: Get all schedules that are ready to be released (timestamp <= now).

### 8. Get Total Scheduled Amount

```rust
pub fn get_total_scheduled_amount(env: Env) -> i128
```

**Purpose**: Calculate total amount in pending schedules.

## Performance Considerations

### Storage Strategy

- **Indexed Storage**: Both contracts maintain indexes for efficient queries
  - Bounty Escrow: Global escrow index + per-depositor indexes
  - Program Escrow: Uses in-memory vectors for schedules and history

### Query Performance

- **Linear Scans**: Most queries perform O(n) scans with early termination when limit is reached
- **Indexed Lookups**: Depositor queries use O(1) index lookup + O(m) scan where m = depositor's escrows
- **Pagination**: Always use pagination for large datasets to avoid hitting transaction limits

### Best Practices

1. **Use Pagination**:

   ```rust
   // Good: Paginated query
   let page_size = 50;
   let results = contract.query_escrows(env, filter, 0, page_size);

   // Avoid: Requesting all records at once
   let all_results = contract.query_escrows(env, filter, 0, 10000);
   ```

2. **Leverage Indexes**:

   ```rust
   // Good: Use indexed query for depositor
   let results = contract.query_escrows_by_depositor(env, depositor, 0, 50);

   // Less efficient: Filter all escrows by depositor
   let filter = EscrowQueryFilter {
       depositor: Some(depositor),
       ..Default::default()
   };
   let results = contract.query_escrows(env, filter, 0, 50);
   ```

3. **Combine Filters**:

   ```rust
   // Efficient: Apply multiple filters in single query
   let filter = EscrowQueryFilter {
       status: Some(EscrowStatus::Locked),
       min_amount: Some(1000),
       max_deadline: Some(current_time + 86400),
       ..Default::default()
   };
   ```

4. **Use Aggregate Functions**:

   ```rust
   // Good: Single call for statistics
   let stats = contract.get_aggregate_stats(env);

   // Avoid: Multiple queries to calculate stats
   let locked = contract.get_escrows_by_status(env, EscrowStatus::Locked, 0, 1000);
   let released = contract.get_escrows_by_status(env, EscrowStatus::Released, 0, 1000);
   ```

### Off-Chain Indexing Recommendations

For production systems with high query volumes:

1. **Event-Based Indexing**: Use contract events for real-time indexing
   - `FundsLocked`, `FundsReleased`, `FundsRefunded` (Bounty Escrow)
   - `FUNDS_LOCKED`, `BATCH_PAYOUT`, `PAYOUT` (Program Escrow)

2. **Database Backend**: Maintain off-chain database indexed by:
   - Depositor/recipient addresses
   - Status/release state
   - Timestamps
   - Amounts

3. **Caching Strategy**: Cache aggregate statistics and update on events

4. **Batch Processing**: For analytics, process events in batches rather than querying contract state

### Transaction Limits

- **Soroban Limits**: Be aware of instruction count and memory limits
- **Recommended Page Size**: 50-100 records per query
- **Large Datasets**: For systems with >1000 escrows, rely on off-chain indexing

### Gas Optimization

- Queries are read-only and don't consume gas for state changes
- Larger result sets increase computational cost
- Use specific queries (by depositor, by status) instead of broad filters when possible

## Integration Examples

### Backend Monitoring Service

```rust
// Poll for new locked escrows
let recent_locked = contract.get_escrows_by_status(
    env,
    EscrowStatus::Locked,
    0,
    100
);

// Get system health metrics
let stats = contract.get_aggregate_stats(env);
if stats.total_locked > threshold {
    alert_admin();
}
```

### User Dashboard

```rust
// Show user's escrows
let user_escrows = contract.query_escrows_by_depositor(
    env,
    user_address,
    page * page_size,
    page_size
);

// Show user's payouts
let user_payouts = contract.get_payouts_by_recipient(
    env,
    user_address,
    0,
    50
);
```

### Analytics Service

```rust
// Daily statistics
let stats = contract.get_program_aggregate_stats(env);
let pending = contract.get_pending_schedules(env);
let due = contract.get_due_schedules(env);

// Generate report
generate_daily_report(stats, pending, due);
```

## Migration Notes

Existing contracts without indexes will need to:

1. Deploy updated contract code
2. Rebuild indexes by scanning existing escrows
3. Update client code to use new query functions

For contracts with many existing escrows, consider:

- Gradual index building
- Temporary read-only mode during migration
- Off-chain index generation from historical events

## SDK Pagination Helper

The TypeScript SDK exports a `fetchAllPages` helper that drives any
`(offset, limit) => Promise<T[]>` fetcher, requesting pages sequentially
until a partial or empty response signals the end of data.

### Usage

```typescript
import { fetchAllPages } from "@grainlify/contracts-sdk";

// Fetch every payout record in the program, regardless of total count
const allPayouts = await fetchAllPages(
  (offset, limit) => client.queryPayoutHistory({}, offset, limit),
  50, // page size (defaults to 50 if omitted)
);

// Fetch only unreleased schedules for a specific recipient
const pending = await fetchAllPages(
  (offset, limit) =>
    client.queryReleaseSchedules(
      { recipient: "GABC...XYZ", released: false },
      offset,
      limit,
    ),
  25,
);
```

### Guarantees

| Guarantee            | Detail                                                                      |
| -------------------- | --------------------------------------------------------------------------- |
| **Exact-once**       | Every item is returned exactly once, in insertion order, with no duplicates |
| **Sequential**       | Each page is awaited before the next request — no concurrent RPC calls      |
| **Auto-termination** | Stops as soon as a page shorter than `pageSize` is returned                 |
