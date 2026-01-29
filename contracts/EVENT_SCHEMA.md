# Grainlify Event Schema Documentation

## Overview

This document defines the comprehensive event schema for all Grainlify contracts. Events provide an immutable audit trail and enable efficient off-chain indexing for real-time monitoring and analytics.

## Event Schema Structure

All events follow a standardized structure for consistency and indexing:

```rust
pub struct EventMetadata {
    pub version: u32,           // Schema version for backward compatibility
    pub timestamp: u64,         // Unix timestamp (seconds)
    pub contract_id: String,    // Contract address
    pub event_type: String,     // Event type identifier
    pub correlation_id: String, // For tracing related events
}
```

## Contract Events

### 1. Bounty Escrow Contract

#### BountyEscrowInitialized
**Topic:** `init`
**Version:** 1
**Emitted:** Once during contract initialization

```rust
pub struct BountyEscrowInitialized {
    pub admin: Address,           // Administrator address
    pub token: Address,           // Token contract address
    pub timestamp: u64,           // Initialization timestamp
}
```

**Indexing Strategy:**
- Index by: `admin`, `token`, `timestamp`
- Retention: Permanent (initialization record)
- Use case: Contract deployment tracking, admin verification

**Off-chain Monitoring:**
```javascript
// Listen for contract initialization
events.on('init', (event) => {
  console.log(`Contract initialized: ${event.contract_id}`);
  console.log(`Admin: ${event.admin}`);
  console.log(`Token: ${event.token}`);
});
```

---

#### FundsLocked
**Topic:** `f_lock`
**Version:** 1
**Emitted:** When funds are locked in escrow

```rust
pub struct FundsLocked {
    pub bounty_id: String,        // Unique bounty identifier
    pub amount: i128,             // Amount locked (stroops)
    pub depositor: Address,       // Address that deposited funds
    pub deadline: u64,            // Refund deadline (Unix timestamp)
    pub timestamp: u64,           // Lock timestamp
}
```

**Indexed Fields:**
- Primary: `bounty_id` (enables bounty-specific filtering)
- Secondary: `depositor`, `timestamp`
- Composite: `(bounty_id, timestamp)` for time-series queries

**Retention:** 7 years (regulatory requirement for financial records)

**Use Cases:**
- Track bounty funding lifecycle
- Verify fund availability
- Audit trail for financial reconciliation
- Real-time balance calculations

**Off-chain Indexing:**
```sql
-- Query funds locked by bounty
SELECT * FROM contract_events 
WHERE event_type = 'FundsLocked' 
AND bounty_id = $1 
ORDER BY timestamp DESC;

-- Aggregate locked funds by depositor
SELECT depositor, SUM(amount) as total_locked 
FROM contract_events 
WHERE event_type = 'FundsLocked' 
GROUP BY depositor;
```

---

#### FundsReleased
**Topic:** `f_rel`
**Version:** 1
**Emitted:** When funds are released from escrow

```rust
pub struct FundsReleased {
    pub bounty_id: String,        // Bounty identifier
    pub amount: i128,             // Amount released
    pub recipient: Address,       // Recipient address
    pub timestamp: u64,           // Release timestamp
}
```

**Indexed Fields:**
- Primary: `bounty_id`
- Secondary: `recipient`, `timestamp`
- Composite: `(recipient, timestamp)` for recipient earnings tracking

**Retention:** 7 years

**Use Cases:**
- Track contributor earnings
- Verify payment execution
- Generate payment reports
- Detect payment anomalies

**Off-chain Monitoring:**
```javascript
// Real-time payment tracking
events.on('f_rel', (event) => {
  console.log(`Payment released: ${event.amount} stroops to ${event.recipient}`);
  updateRecipientBalance(event.recipient, event.amount);
});
```

---

#### FundsRefunded
**Topic:** `f_ref`
**Version:** 1
**Emitted:** When funds are refunded to depositor

```rust
pub struct FundsRefunded {
    pub bounty_id: String,        // Bounty identifier
    pub amount: i128,             // Amount refunded
    pub refund_to: Address,       // Refund recipient
    pub timestamp: u64,           // Refund timestamp
    pub refund_mode: String,      // "expired" | "cancelled" | "manual"
    pub remaining_amount: i128,   // Remaining locked amount
}
```

**Indexed Fields:**
- Primary: `bounty_id`
- Secondary: `refund_to`, `refund_mode`, `timestamp`
- Composite: `(refund_to, timestamp)` for refund tracking

**Retention:** 7 years

**Use Cases:**
- Track refund lifecycle
- Audit refund reasons
- Verify fund return
- Detect refund anomalies

---

#### BatchFundsLocked
**Topic:** `b_lock`
**Version:** 1
**Emitted:** When multiple bounties are locked in a batch operation

```rust
pub struct BatchFundsLocked {
    pub count: u32,               // Number of bounties locked
    pub total_amount: i128,       // Total amount locked
    pub timestamp: u64,           // Batch operation timestamp
}
```

**Indexed Fields:**
- Primary: `timestamp`
- Secondary: `count`

**Retention:** 7 years

**Use Cases:**
- Track batch operations
- Monitor throughput
- Detect batch processing issues

---

#### BatchFundsReleased
**Topic:** `b_rel`
**Version:** 1
**Emitted:** When multiple bounties are released in a batch operation

```rust
pub struct BatchFundsReleased {
    pub count: u32,               // Number of bounties released
    pub total_amount: i128,       // Total amount released
    pub timestamp: u64,           // Batch operation timestamp
}
```

**Indexed Fields:**
- Primary: `timestamp`
- Secondary: `count`

**Retention:** 7 years

---

### 2. Program Escrow Contract

#### ProgramInitialized
**Topic:** `prog_init`
**Version:** 1
**Emitted:** When a program is registered

```rust
pub struct ProgramInitialized {
    pub program_id: String,           // Program identifier
    pub authorized_payout_key: Address, // Authorized payout address
    pub token_address: Address,       // Token contract address
    pub total_funds: i128,            // Total funds allocated
    pub timestamp: u64,               // Initialization timestamp
}
```

**Indexed Fields:**
- Primary: `program_id`
- Secondary: `authorized_payout_key`, `timestamp`

**Retention:** 7 years

---

#### ProgramFundsLocked
**Topic:** `prog_lock`
**Version:** 1
**Emitted:** When funds are locked for a program

```rust
pub struct ProgramFundsLocked {
    pub program_id: String,       // Program identifier
    pub amount: i128,             // Amount locked
    pub locked_at: u64,           // Lock timestamp
}
```

**Indexed Fields:**
- Primary: `program_id`
- Secondary: `locked_at`

**Retention:** 7 years

---

#### ProgramFundsReleased
**Topic:** `prog_rel`
**Version:** 1
**Emitted:** When funds are released from program

```rust
pub struct ProgramFundsReleased {
    pub program_id: String,       // Program identifier
    pub recipient: Address,       // Recipient address
    pub amount: i128,             // Amount released
    pub remaining_balance: i128,  // Remaining program balance
    pub timestamp: u64,           // Release timestamp
}
```

**Indexed Fields:**
- Primary: `program_id`
- Secondary: `recipient`, `timestamp`
- Composite: `(recipient, timestamp)` for earnings tracking

**Retention:** 7 years

---

#### BatchPayout
**Topic:** `batch_pay`
**Version:** 1
**Emitted:** When batch payouts are executed

```rust
pub struct BatchPayout {
    pub program_id: String,           // Program identifier
    pub recipient_count: u32,         // Number of recipients
    pub total_amount: i128,           // Total amount paid
    pub remaining_balance: i128,      // Remaining program balance
    pub timestamp: u64,               // Payout timestamp
}
```

**Indexed Fields:**
- Primary: `program_id`
- Secondary: `timestamp`

**Retention:** 7 years

---

### 3. Grainlify Core Contract

#### OperationMetric
**Topic:** `metric:op`
**Version:** 1
**Emitted:** After each operation for monitoring

```rust
pub struct OperationMetric {
    pub operation: String,        // Operation name
    pub caller: Address,          // Caller address
    pub success: bool,            // Operation success status
    pub timestamp: u64,           // Operation timestamp
}
```

**Indexed Fields:**
- Primary: `operation`, `timestamp`
- Secondary: `caller`, `success`

**Retention:** 90 days (operational metrics)

**Use Cases:**
- Real-time operation monitoring
- Success rate tracking
- Caller activity analysis

---

#### PerformanceMetric
**Topic:** `metric:perf`
**Version:** 1
**Emitted:** After each operation for performance tracking

```rust
pub struct PerformanceMetric {
    pub operation: String,        // Operation name
    pub duration_ms: u64,         // Operation duration (milliseconds)
    pub timestamp: u64,           // Metric timestamp
}
```

**Indexed Fields:**
- Primary: `operation`, `timestamp`
- Secondary: `duration_ms`

**Retention:** 30 days (performance metrics)

**Use Cases:**
- Performance monitoring
- Bottleneck identification
- SLA tracking

---

## Event Versioning Strategy

### Version Evolution

Events use semantic versioning for schema changes:

```
Version Format: MAJOR.MINOR.PATCH
- MAJOR: Breaking changes (new required fields)
- MINOR: Backward-compatible additions (new optional fields)
- PATCH: Bug fixes (no schema changes)
```

### Backward Compatibility

**Version 1 → Version 2 (Minor):**
```rust
// Old version
pub struct FundsLocked {
    pub bounty_id: String,
    pub amount: i128,
    pub depositor: Address,
    pub deadline: u64,
    pub timestamp: u64,
}

// New version (backward compatible)
pub struct FundsLocked {
    pub bounty_id: String,
    pub amount: i128,
    pub depositor: Address,
    pub deadline: u64,
    pub timestamp: u64,
    pub metadata: Option<String>,  // New optional field
}
```

**Migration Strategy:**
1. Deploy new contract with version 2
2. Old events continue to work (version 1)
3. New events use version 2
4. Indexers handle both versions
5. After 6 months, deprecate version 1

---

## Event Indexing Strategies

### 1. Time-Series Indexing

**Purpose:** Efficient querying of events over time ranges

```sql
-- Create time-series index
CREATE INDEX idx_events_type_timestamp 
ON contract_events(event_type, timestamp DESC);

-- Query events in time range
SELECT * FROM contract_events 
WHERE event_type = 'FundsLocked' 
AND timestamp BETWEEN $1 AND $2 
ORDER BY timestamp DESC;
```

### 2. Entity-Based Indexing

**Purpose:** Efficient querying by entity (bounty, program, recipient)

```sql
-- Index by bounty_id
CREATE INDEX idx_events_bounty_id 
ON contract_events(bounty_id, timestamp DESC);

-- Query all events for a bounty
SELECT * FROM contract_events 
WHERE bounty_id = $1 
ORDER BY timestamp DESC;
```

### 3. Composite Indexing

**Purpose:** Efficient multi-field queries

```sql
-- Composite index for recipient earnings
CREATE INDEX idx_events_recipient_timestamp 
ON contract_events(recipient, timestamp DESC) 
WHERE event_type IN ('FundsReleased', 'ProgramFundsReleased');

-- Query recipient earnings
SELECT SUM(amount) as total_earnings 
FROM contract_events 
WHERE recipient = $1 
AND event_type IN ('FundsReleased', 'ProgramFundsReleased')
AND timestamp >= $2;
```

### 4. Aggregation Indexing

**Purpose:** Pre-computed aggregations for fast reporting

```sql
-- Materialized view for daily statistics
CREATE MATERIALIZED VIEW daily_event_stats AS
SELECT 
    DATE(to_timestamp(timestamp)) as date,
    event_type,
    COUNT(*) as event_count,
    SUM(amount) as total_amount,
    AVG(amount) as avg_amount
FROM contract_events
GROUP BY DATE(to_timestamp(timestamp)), event_type;

-- Refresh strategy: Daily at 2 AM UTC
REFRESH MATERIALIZED VIEW CONCURRENTLY daily_event_stats;
```

---

## Event Filtering Examples

### Filter by Event Type and Time Range

```javascript
// Get all fund locks in the last 24 hours
const recentLocks = await eventIndex.query({
  eventType: 'FundsLocked',
  startTime: Date.now() - 24 * 60 * 60 * 1000,
  endTime: Date.now(),
  limit: 1000
});
```

### Filter by Entity

```javascript
// Get all events for a specific bounty
const bountyEvents = await eventIndex.query({
  bountyId: 'bounty-123',
  orderBy: 'timestamp',
  order: 'DESC'
});
```

### Filter by Recipient

```javascript
// Get all payments to a recipient
const recipientPayments = await eventIndex.query({
  eventTypes: ['FundsReleased', 'ProgramFundsReleased'],
  recipient: 'recipient-address',
  startTime: Date.now() - 30 * 24 * 60 * 60 * 1000
});
```

### Aggregate Queries

```javascript
// Get total funds locked by program
const programStats = await eventIndex.aggregate({
  eventType: 'ProgramFundsLocked',
  groupBy: 'program_id',
  aggregation: 'SUM(amount)'
});
```

---

## Event Retention Policy

| Event Type | Retention | Reason |
|-----------|-----------|--------|
| FundsLocked | 7 years | Financial/regulatory requirement |
| FundsReleased | 7 years | Financial/regulatory requirement |
| FundsRefunded | 7 years | Financial/regulatory requirement |
| BatchFundsLocked | 7 years | Financial/regulatory requirement |
| BatchFundsReleased | 7 years | Financial/regulatory requirement |
| OperationMetric | 90 days | Operational monitoring |
| PerformanceMetric | 30 days | Performance tracking |
| ProgramInitialized | 7 years | Program lifecycle |
| ProgramFundsLocked | 7 years | Financial/regulatory requirement |
| ProgramFundsReleased | 7 years | Financial/regulatory requirement |
| BatchPayout | 7 years | Financial/regulatory requirement |

---

## Event Monitoring Hooks

### Real-time Alerts

```javascript
// Alert on large transactions
eventMonitor.on('FundsLocked', (event) => {
  if (event.amount > LARGE_TRANSACTION_THRESHOLD) {
    alert({
      severity: 'INFO',
      message: `Large transaction: ${event.amount} stroops`,
      bountyId: event.bounty_id,
      timestamp: event.timestamp
    });
  }
});

// Alert on failed operations
eventMonitor.on('OperationMetric', (event) => {
  if (!event.success) {
    alert({
      severity: 'WARNING',
      message: `Operation failed: ${event.operation}`,
      caller: event.caller,
      timestamp: event.timestamp
    });
  }
});
```

### Performance Monitoring

```javascript
// Track operation performance
eventMonitor.on('PerformanceMetric', (event) => {
  const sla = OPERATION_SLAS[event.operation];
  if (event.duration_ms > sla) {
    alert({
      severity: 'WARNING',
      message: `SLA violation: ${event.operation} took ${event.duration_ms}ms`,
      sla: sla,
      timestamp: event.timestamp
    });
  }
});
```

### Anomaly Detection

```javascript
// Detect unusual patterns
eventMonitor.on('FundsReleased', (event) => {
  const recipientHistory = getRecipientHistory(event.recipient);
  const avgAmount = recipientHistory.avgAmount;
  
  if (event.amount > avgAmount * 3) {
    alert({
      severity: 'INFO',
      message: `Unusual payment amount detected`,
      recipient: event.recipient,
      amount: event.amount,
      avgAmount: avgAmount,
      timestamp: event.timestamp
    });
  }
});
```

---

## Implementation Checklist

- [x] Define comprehensive event schema
- [x] Document event versioning strategy
- [x] Create indexing strategies
- [x] Define retention policies
- [x] Provide filtering examples
- [x] Document monitoring hooks
- [ ] Implement backend event retrieval from Soroban RPC
- [ ] Create event indexing database schema
- [ ] Implement event monitoring API
- [ ] Add event filtering utilities
- [ ] Create monitoring dashboard
- [ ] Add event replay capability
- [ ] Implement event versioning in contracts
- [ ] Add correlation IDs to events
- [ ] Create event documentation API

---

## References

- [Soroban Events Documentation](https://developers.stellar.org/learn/smart-contract-internals/events)
- [Event Sourcing Pattern](https://martinfowler.com/eaaDev/EventSourcing.html)
- [Time-Series Database Best Practices](https://www.timescale.com/blog/what-is-a-time-series-database/)
