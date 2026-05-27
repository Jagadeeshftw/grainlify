# BuildInfo Event Implementation

## Overview

The `BuildInfo` event is a new contract-level event emitted during smart contract initialization via the `init_admin()` function. This event provides crucial audit trail and deployment metadata for monitoring and verification systems.

## Event Definition

### Rust Structure

```rust
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildInfoEvent {
    /// The admin address that authorized contract initialization
    pub admin: Address,
    /// Initial contract version set during initialization
    pub version: u32,
    /// Ledger timestamp when the contract was initialized
    pub timestamp: u64,
}
```

### Event Publication

The `BuildInfo` event is emitted in the `init_admin()` function with topics `(init, build)`:

```rust
env.events().publish(
    (symbol_short!("init"), symbol_short!("build")),
    BuildInfoEvent {
        admin: admin.clone(),
        version: VERSION,
        timestamp: env.ledger().timestamp(),
    },
);
```

## Fields

| Field | Type | Description | Security Considerations |
|-------|------|-------------|------------------------|
| `admin` | `Address` | The admin address that authorized initialization | Immutable after init; required for access control auditing |
| `version` | `u32` | Initial contract version (typically VERSION constant) | Enables version tracking and history |
| `timestamp` | `u64` | Ledger timestamp of initialization | Provides exact timing for event sequencing |

## Event Topics

- **Primary Topic**: `init` (symbol_short)
- **Secondary Topic**: `build` (symbol_short)

Indexers can filter events using: `(Symbol("init"), Symbol("build"))`

## Security Guarantees

### 1. Authorization Required
- BuildInfo event ONLY emitted when `init_admin()` is called with valid admin authentication
- Requires the admin address to call `require_auth()` before any state changes
- Cannot be manually published or called without proper authorization

### 2. Single Emission Guarantee
- Event is emitted only ONCE during the contract lifecycle
- Subsequent initialization attempts fail with `AlreadyInitialized` error (code 1)
- Prevents replay attacks or re-initialization exploits

### 3. Immutable Event Data
- Event data is part of the Soroban ledger and cannot be modified
- Provides permanent audit trail of initialization
- Suitable for compliance and governance requirements

### 4. Audit Trail
- Records exact admin address for access control verification
- Captures precise ledger timestamp for temporal sequencing
- Enables verification of deployment order across networks

## Test Coverage

### Test Categories

#### 1. Event Emission Tests (5 tests)
- ✅ `test_build_info_event_emitted_on_init` - Event is emitted when init_admin is called
- ✅ `test_build_info_event_admin_field` - Admin field contains correct address
- ✅ `test_build_info_event_version_field` - Version field contains correct version
- ✅ `test_build_info_event_timestamp_accuracy` - Timestamp is accurate and within bounds
- ✅ `test_build_info_event_topics` - Event topics are correctly set

#### 2. Initialization Guards (2 tests)
- ✅ `test_double_initialization_rejected` - Double init fails with AlreadyInitialized
- ✅ `test_build_info_event_emitted_once` - Event is only emitted once

#### 3. Authorization & Security (1 test)
- ✅ `test_build_info_event_requires_admin_auth` - Event only emitted with valid auth

#### 4. Data Consistency (2 tests)
- ✅ `test_build_info_event_serialization` - Event data serializes/deserializes correctly
- ✅ `test_build_info_event_version_matches_get_version` - Event version matches contract state

#### 5. Edge Cases & Multiple Instances (2 tests)
- ✅ `test_build_info_event_with_different_admins` - Works with different admin addresses
- ✅ `test_build_info_event_per_contract_instance` - Events are independent per contract

**Total Test Count**: 13 comprehensive tests
**Coverage Areas**: Emission, Authorization, State Consistency, Edge Cases, Multiple Instances

## Test Execution

### Run All BuildInfo Tests

```bash
cd contracts/grainlify-core
cargo test build_info_event_tests --lib
```

### Run Specific Test

```bash
cargo test build_info_event_tests::test_build_info_event_emitted_on_init --lib
```

### Run with Verbose Output

```bash
cargo test build_info_event_tests --lib -- --nocapture
```

## Integration with Manifest Schema

The BuildInfo event is documented in the contract manifest:

### Manifest Entry

```json
{
  "name": "ContractInitialized",
  "description": "Contract initialization completed with BuildInfo metadata",
  "data": [
    {
      "name": "admin",
      "type": "Address",
      "description": "Administrator address that authorized initialization"
    },
    {
      "name": "version",
      "type": "u32",
      "description": "Initial contract version"
    },
    {
      "name": "timestamp",
      "type": "u64",
      "description": "Initialization timestamp (ledger time)"
    }
  ],
  "trigger": "Contract initialization via init_admin()",
  "security_notes": "Only emitted during first-time initialization; requires admin authorization"
}
```

## Usage Examples

### Indexing Initialization Events

Indexers should listen for BuildInfo events to:

1. **Track Deployment Timeline**
   ```
   Event: (init, build)
   Admin: 0xABCD...
   Timestamp: 1704067200
   Version: 2
   ```

2. **Verify Admin Address**
   - Extract `admin` field from event data
   - Compare with expected admin from deployment configuration
   - Alert if mismatch detected

3. **Audit Event Sequencing**
   - Combine with upgrade events for full contract lifecycle
   - Ensure single initialization per contract instance
   - Track version progression

### Off-Chain Monitoring

```javascript
// Example: Monitor BuildInfo events
contract.on('(init, build)', (event) => {
  const buildInfo = event.data;
  console.log(`Contract initialized by: ${buildInfo.admin}`);
  console.log(`Initial version: ${buildInfo.version}`);
  console.log(`Timestamp: ${new Date(buildInfo.timestamp * 1000)}`);
  
  // Verify against expected configuration
  if (buildInfo.admin !== expectedAdmin) {
    alertSecurity("Unexpected admin address!");
  }
});
```

## Implementation Notes

### Performance Characteristics

- **Gas Cost**: Minimal - event publication is an efficient operation
- **Storage**: No additional storage used (events are not persisted in contract state)
- **Ledger Impact**: Single ledger entry per contract initialization

### Compatibility

- **Soroban SDK Version**: 21.7.7+
- **Contract Feature**: Standard feature (no special flags required)
- **Event Protocol**: Compatible with v2 event schema

## Changelog

### Version 2.0.0
- ✅ Added BuildInfo event to init_admin function
- ✅ Updated contract manifest schema with event documentation
- ✅ Created comprehensive test suite (13 tests)
- ✅ Added security documentation and guidelines

## Future Enhancements

### Potential Extensions

1. **Extended Metadata**
   - Add WASM hash of initial contract code
   - Include chain/network identifier
   - Store initial configuration snapshot

2. **Event Filtering**
   - Filter by admin address range
   - Query by timestamp range
   - Index by version

3. **Migration Support**
   - Track initialization across contract versions
   - Compare with upgrade events for history
   - Detect orphaned contracts

## Security Considerations for Developers

### Best Practices

1. **Verify Events**
   - Always verify BuildInfo event was emitted on initialization
   - Cross-check admin address with expected value
   - Confirm timestamp is reasonable

2. **Authorization Chains**
   - BuildInfo event proves admin authorization occurred
   - Use event as proof of proper initialization
   - Include in compliance reports

3. **Monitoring**
   - Set up indexer to catch initialization events
   - Alert on unexpected admin addresses
   - Track deployment across environments

### Common Pitfalls

❌ **Do NOT**:
- Assume initialization without checking event
- Use event timestamp as the only time source
- Modify event data post-emission

✅ **DO**:
- Index events for off-chain verification
- Combine with other events for full audit trail
- Use event as part of security monitoring

## Troubleshooting

### Event Not Emitted

**Issue**: BuildInfo event is missing from ledger

**Solutions**:
1. Verify `init_admin()` was called (check for `AlreadyInitialized` error)
2. Confirm admin address called with proper authentication
3. Check indexer filters match event topics

### Incorrect Admin in Event

**Issue**: Event shows different admin than expected

**Solutions**:
1. Verify deployment script used correct admin address
2. Check for contract address confusion (wrong contract instance)
3. Audit initialization authorization chain

## References

- [Contract Manifest Schema](../contract-manifest-schema.json)
- [Event Schema Documentation](../EVENT_SCHEMA.md)
- [Initialization Guide](./INITIALIZATION.md)
- [Security Best Practices](./SECURITY.md)
