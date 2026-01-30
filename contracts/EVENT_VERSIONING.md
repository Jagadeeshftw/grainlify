# Event Versioning Strategy

## Overview

Event versioning ensures backward compatibility and enables smooth schema evolution as the Grainlify platform grows. This document outlines the versioning strategy, migration paths, and best practices.

## Versioning Scheme

We use semantic versioning for event schemas:

```
Version Format: MAJOR.MINOR.PATCH
- MAJOR: Breaking changes (incompatible with previous versions)
- MINOR: Backward-compatible additions (new optional fields)
- PATCH: Bug fixes (no schema changes)
```

## Version Evolution Rules

### Rule 1: Never Remove Fields

Once a field is added to an event, it must never be removed. Instead:
- Mark as deprecated in documentation
- Provide migration path to new field
- Support both old and new fields for 2+ versions

### Rule 2: New Fields Must Be Optional

When adding new fields to an event:
- New fields must have default values
- Old indexers must continue to work
- New indexers must handle missing fields gracefully

### Rule 3: Type Changes Require Major Version

Changing a field's type requires a major version bump:
- `amount: i128` → `amount: String` = Major version
- `timestamp: u64` → `timestamp: u128` = Major version
- `address: Address` → `address: String` = Major version

### Rule 4: Enum Additions Are Minor Versions

Adding new enum values is a minor version:
- `refund_mode: "expired" | "cancelled"` → `refund_mode: "expired" | "cancelled" | "manual"` = Minor version

## Event Schema Versions

### Bounty Escrow Contract

#### FundsLocked Event

**Version 1.0.0** (Current)
```rust
pub struct FundsLocked {
    pub bounty_id: String,        // Unique bounty identifier
    pub amount: i128,             // Amount locked (stroops)
    pub depositor: Address,       // Address that deposited funds
    pub deadline: u64,            // Refund deadline (Unix timestamp)
    pub timestamp: u64,           // Lock timestamp
}
```

**Planned Version 2.0.0** (Future)
```rust
pub struct FundsLocked {
    pub bounty_id: String,        // Unique bounty identifier
    pub amount: i128,             // Amount locked (stroops)
    pub depositor: Address,       // Address that deposited funds
    pub deadline: u64,            // Refund deadline (Unix timestamp)
    pub timestamp: u64,           // Lock timestamp
    pub metadata: Option<String>, // Optional metadata (NEW)
    pub correlation_id: Option<String>, // Trace ID (NEW)
}
```

**Migration Path:**
1. Deploy contract with version 2.0.0
2. Old events continue to work (version 1.0.0)
3. New events use version 2.0.0
4. Indexers handle both versions
5. After 6 months, deprecate version 1.0.0

---

#### FundsReleased Event

**Version 1.0.0** (Current)
```rust
pub struct FundsReleased {
    pub bounty_id: String,        // Bounty identifier
    pub amount: i128,             // Amount released
    pub recipient: Address,       // Recipient address
    pub timestamp: u64,           // Release timestamp
}
```

**Planned Version 1.1.0** (Minor - Backward Compatible)
```rust
pub struct FundsReleased {
    pub bounty_id: String,        // Bounty identifier
    pub amount: i128,             // Amount released
    pub recipient: Address,       // Recipient address
    pub timestamp: u64,           // Release timestamp
    pub release_reason: Option<String>, // Why funds were released (NEW)
}
```

---

#### FundsRefunded Event

**Version 1.0.0** (Current)
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

**Planned Version 2.0.0** (Major - Breaking Change)
```rust
pub struct FundsRefunded {
    pub bounty_id: String,        // Bounty identifier
    pub amount: i128,             // Amount refunded
    pub refund_to: Address,       // Refund recipient
    pub timestamp: u64,           // Refund timestamp
    pub refund_mode: String,      // "expired" | "cancelled" | "manual" | "partial"
    pub remaining_amount: i128,   // Remaining locked amount
    pub refund_reason: String,    // Detailed reason (NEW - REQUIRED)
    pub refund_tx_hash: String,   // Transaction hash (NEW - REQUIRED)
}
```

---

### Program Escrow Contract

#### ProgramFundsLocked Event

**Version 1.0.0** (Current)
```rust
pub struct ProgramFundsLocked {
    pub program_id: String,       // Program identifier
    pub amount: i128,             // Amount locked
    pub locked_at: u64,           // Lock timestamp
}
```

**Planned Version 1.1.0** (Minor - Backward Compatible)
```rust
pub struct ProgramFundsLocked {
    pub program_id: String,       // Program identifier
    pub amount: i128,             // Amount locked
    pub locked_at: u64,           // Lock timestamp
    pub lock_reason: Option<String>, // Why funds were locked (NEW)
    pub batch_id: Option<String>, // Batch operation ID (NEW)
}
```

---

#### BatchPayout Event

**Version 1.0.0** (Current)
```rust
pub struct BatchPayout {
    pub program_id: String,           // Program identifier
    pub recipient_count: u32,         // Number of recipients
    pub total_amount: i128,           // Total amount paid
    pub remaining_balance: i128,      // Remaining program balance
    pub timestamp: u64,               // Payout timestamp
}
```

**Planned Version 2.0.0** (Major - Breaking Change)
```rust
pub struct BatchPayout {
    pub program_id: String,           // Program identifier
    pub recipient_count: u32,         // Number of recipients
    pub total_amount: i128,           // Total amount paid
    pub remaining_balance: i128,      // Remaining program balance
    pub timestamp: u64,               // Payout timestamp
    pub batch_id: String,             // Unique batch ID (NEW - REQUIRED)
    pub payout_details: Vec<PayoutDetail>, // Individual payouts (NEW - REQUIRED)
}

pub struct PayoutDetail {
    pub recipient: Address,
    pub amount: i128,
}
```

---

## Migration Strategies

### Strategy 1: Additive Migration (Minor Version)

**Scenario:** Adding optional fields

**Steps:**
1. Add new optional field to event struct
2. Bump minor version
3. Deploy new contract
4. Old indexers continue to work (ignore new field)
5. New indexers handle both old and new events

**Example:**
```rust
// Version 1.0.0
pub struct FundsLocked {
    pub bounty_id: String,
    pub amount: i128,
    pub depositor: Address,
    pub deadline: u64,
    pub timestamp: u64,
}

// Version 1.1.0 (Backward compatible)
pub struct FundsLocked {
    pub bounty_id: String,
    pub amount: i128,
    pub depositor: Address,
    pub deadline: u64,
    pub timestamp: u64,
    pub metadata: Option<String>, // NEW - Optional
}
```

**Indexer Handling:**
```go
// Old indexer (v1.0.0)
type FundsLocked struct {
    BountyID  string
    Amount    int64
    Depositor string
    Deadline  int64
    Timestamp int64
}

// New indexer (v1.1.0)
type FundsLocked struct {
    BountyID  string
    Amount    int64
    Depositor string
    Deadline  int64
    Timestamp int64
    Metadata  *string // NEW - Optional
}

// Handles both versions
func (f *FundsLocked) UnmarshalJSON(data []byte) error {
    type Alias FundsLocked
    aux := &struct {
        *Alias
    }{
        Alias: (*Alias)(f),
    }
    return json.Unmarshal(data, &aux)
}
```

---

### Strategy 2: Replacement Migration (Major Version)

**Scenario:** Breaking changes (type changes, required new fields)

**Steps:**
1. Create new event type or major version
2. Deploy new contract with both old and new events
3. Gradually migrate to new event
4. After transition period, deprecate old event
5. Remove old event in next major release

**Example:**
```rust
// Version 1.0.0 (Old)
pub struct FundsRefunded {
    pub bounty_id: String,
    pub amount: i128,
    pub refund_to: Address,
    pub timestamp: u64,
    pub refund_mode: String,
    pub remaining_amount: i128,
}

// Version 2.0.0 (New - Breaking change)
pub struct FundsRefunded {
    pub bounty_id: String,
    pub amount: i128,
    pub refund_to: Address,
    pub timestamp: u64,
    pub refund_mode: String,
    pub remaining_amount: i128,
    pub refund_reason: String,    // NEW - REQUIRED
    pub refund_tx_hash: String,   // NEW - REQUIRED
}

// Emit both during transition
fn refund_funds(env: &Env, bounty_id: String, amount: i128, refund_to: Address) {
    // Emit old event for backward compatibility
    emit_funds_refunded_v1(env, FundsRefundedV1 {
        bounty_id: bounty_id.clone(),
        amount,
        refund_to: refund_to.clone(),
        timestamp: env.ledger().timestamp(),
        refund_mode: "manual".to_string(),
        remaining_amount: 0,
    });
    
    // Emit new event
    emit_funds_refunded_v2(env, FundsRefundedV2 {
        bounty_id,
        amount,
        refund_to,
        timestamp: env.ledger().timestamp(),
        refund_mode: "manual".to_string(),
        remaining_amount: 0,
        refund_reason: "Manual refund".to_string(),
        refund_tx_hash: "tx_hash".to_string(),
    });
}
```

---

### Strategy 3: Parallel Versioning

**Scenario:** Supporting multiple versions simultaneously

**Steps:**
1. Maintain multiple event versions in code
2. Emit all versions during transition
3. Indexers handle all versions
4. Gradually deprecate old versions
5. Remove old versions after deprecation period

**Example:**
```rust
// Version 1.0.0
pub struct FundsLocked {
    pub bounty_id: String,
    pub amount: i128,
    pub depositor: Address,
    pub deadline: u64,
    pub timestamp: u64,
}

// Version 1.1.0
pub struct FundsLockedV1_1 {
    pub bounty_id: String,
    pub amount: i128,
    pub depositor: Address,
    pub deadline: u64,
    pub timestamp: u64,
    pub metadata: Option<String>,
}

// Version 2.0.0
pub struct FundsLockedV2 {
    pub bounty_id: String,
    pub amount: i128,
    pub depositor: Address,
    pub deadline: u64,
    pub timestamp: u64,
    pub metadata: Option<String>,
    pub correlation_id: String,
}

// Emit all versions
fn lock_funds(env: &Env, bounty_id: String, amount: i128, depositor: Address, deadline: u64) {
    let timestamp = env.ledger().timestamp();
    
    // Emit v1.0.0
    emit_funds_locked_v1(env, FundsLocked {
        bounty_id: bounty_id.clone(),
        amount,
        depositor: depositor.clone(),
        deadline,
        timestamp,
    });
    
    // Emit v1.1.0
    emit_funds_locked_v1_1(env, FundsLockedV1_1 {
        bounty_id: bounty_id.clone(),
        amount,
        depositor: depositor.clone(),
        deadline,
        timestamp,
        metadata: None,
    });
    
    // Emit v2.0.0
    emit_funds_locked_v2(env, FundsLockedV2 {
        bounty_id,
        amount,
        depositor,
        deadline,
        timestamp,
        metadata: None,
        correlation_id: generate_correlation_id(),
    });
}
```

---

## Deprecation Timeline

### Phase 1: Introduction (Months 1-2)
- New version released
- Both old and new versions emitted
- Documentation updated
- Indexers updated to handle both versions

### Phase 2: Transition (Months 3-6)
- New version becomes default
- Old version still emitted for compatibility
- Monitoring for old version usage
- Migration guides published

### Phase 3: Deprecation (Months 7-12)
- Old version marked as deprecated
- Warnings in logs when old version used
- Migration deadline announced
- Support for old version reduced

### Phase 4: Removal (Month 13+)
- Old version removed from code
- Only new version emitted
- Old version no longer supported
- Migration complete

---

## Indexer Compatibility

### Handling Multiple Versions

```go
// Generic event handler that supports multiple versions
func handleFundsLockedEvent(event ContractEvent) error {
    var data map[string]interface{}
    if err := json.Unmarshal(event.Data, &data); err != nil {
        return err
    }
    
    version := event.Version
    
    switch version {
    case 1:
        return handleFundsLockedV1(data)
    case 2:
        return handleFundsLockedV2(data)
    default:
        return fmt.Errorf("unsupported version: %d", version)
    }
}

func handleFundsLockedV1(data map[string]interface{}) error {
    bountyID := data["bounty_id"].(string)
    amount := data["amount"].(float64)
    // Handle v1 event
    return nil
}

func handleFundsLockedV2(data map[string]interface{}) error {
    bountyID := data["bounty_id"].(string)
    amount := data["amount"].(float64)
    metadata := data["metadata"].(string) // NEW field
    // Handle v2 event
    return nil
}
```

### Version Detection

```go
// Detect event version from schema
func detectEventVersion(event ContractEvent) int {
    var data map[string]interface{}
    json.Unmarshal(event.Data, &data)
    
    // Check for version-specific fields
    if _, hasMetadata := data["metadata"]; hasMetadata {
        if _, hasCorrelationID := data["correlation_id"]; hasCorrelationID {
            return 2 // Version 2.0.0
        }
        return 1 // Version 1.1.0
    }
    return 1 // Version 1.0.0
}
```

---

## Best Practices

### 1. Always Include Version Field

```rust
pub struct FundsLocked {
    pub version: u32,             // Always include version
    pub bounty_id: String,
    pub amount: i128,
    pub depositor: Address,
    pub deadline: u64,
    pub timestamp: u64,
}
```

### 2. Document Version Changes

```rust
/// Event emitted when funds are locked in escrow.
///
/// # Version History
/// - v1.0.0: Initial release
/// - v1.1.0: Added optional metadata field
/// - v2.0.0: Added required correlation_id field
///
/// # Backward Compatibility
/// - v1.0.0 events are still supported
/// - v1.1.0 is backward compatible with v1.0.0
/// - v2.0.0 requires migration from v1.x
pub struct FundsLocked {
    pub bounty_id: String,
    pub amount: i128,
    pub depositor: Address,
    pub deadline: u64,
    pub timestamp: u64,
}
```

### 3. Provide Migration Utilities

```go
// Migrate v1 event to v2
func migrateEventV1ToV2(v1Event map[string]interface{}) map[string]interface{} {
    v2Event := make(map[string]interface{})
    
    // Copy existing fields
    v2Event["bounty_id"] = v1Event["bounty_id"]
    v2Event["amount"] = v1Event["amount"]
    v2Event["depositor"] = v1Event["depositor"]
    v2Event["deadline"] = v1Event["deadline"]
    v2Event["timestamp"] = v1Event["timestamp"]
    
    // Add new required fields with defaults
    v2Event["refund_reason"] = "Migrated from v1"
    v2Event["refund_tx_hash"] = "unknown"
    
    return v2Event
}
```

### 4. Test Version Compatibility

```rust
#[test]
fn test_event_version_compatibility() {
    let env = Env::default();
    
    // Test v1 event
    let v1_event = FundsLockedV1 {
        bounty_id: "bounty-1".to_string(),
        amount: 1000,
        depositor: Address::random(&env),
        deadline: 1000000,
        timestamp: 1000,
    };
    
    // Test v2 event
    let v2_event = FundsLockedV2 {
        bounty_id: "bounty-1".to_string(),
        amount: 1000,
        depositor: Address::random(&env),
        deadline: 1000000,
        timestamp: 1000,
        metadata: Some("test".to_string()),
    };
    
    // Both should be processable
    assert!(process_event(&v1_event).is_ok());
    assert!(process_event(&v2_event).is_ok());
}
```

---

## Version Roadmap

### Q1 2025
- v1.0.0: Current stable version
- All events at v1.0.0

### Q2 2025
- v1.1.0: Add optional metadata fields
- FundsLocked → v1.1.0
- FundsReleased → v1.1.0
- ProgramFundsLocked → v1.1.0

### Q3 2025
- v2.0.0: Add correlation IDs and required fields
- FundsRefunded → v2.0.0
- BatchPayout → v2.0.0
- Deprecate v1.0.0

### Q4 2025
- v2.1.0: Add analytics fields
- All events → v2.1.0
- Remove v1.0.0 support

---

## References

- [Semantic Versioning](https://semver.org/)
- [API Versioning Best Practices](https://swagger.io/blog/api-versioning-best-practices/)
- [Event Schema Evolution](https://www.confluent.io/blog/event-schema-evolution/)
