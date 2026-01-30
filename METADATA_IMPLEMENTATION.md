# Escrow Metadata and Tagging Implementation

## Overview

This implementation adds structured metadata and tagging capabilities to both bounty escrow and program escrow contracts. This enables better off-chain indexing, filtering, and analytics without changing core escrow logic.

## Features Implemented

### 1. Bounty Escrow Metadata
- **Struct**: `EscrowMetadata`
- **Storage**: Persistent storage keyed by bounty_id
- **Fields**:
  - `repo_id`: Repository identifier (e.g., "owner/repo")
  - `issue_id`: Issue or pull request identifier
  - `bounty_type`: Type classification (e.g., "bug", "feature", "security")
  - `tags`: Custom tags for filtering (Vec<String>)
  - `custom_fields`: Extensible key-value pairs (Map<String, String>)

### 2. Program Escrow Metadata
- **Struct**: `ProgramMetadata`
- **Storage**: Instance storage (single program per contract)
- **Fields**:
  - `event_name`: Full event/hackathon name
  - `event_type`: Classification (e.g., "hackathon", "grant", "bounty-program")
  - `start_date`/`end_date`: Event dates (YYYY-MM-DD format)
  - `website`: Event website URL
  - `tags`: Custom tags for filtering (Vec<String>)
  - `custom_fields`: Extensible key-value pairs (Map<String, String>)

## Size Limits and Constraints

### Bounty Escrow Metadata Limits
- **Total serialized size**: 1024 bytes maximum
- **Tags**: 20 items maximum
- **Custom fields**: 10 key-value pairs maximum
- **Individual strings**: 128 characters maximum

### Program Escrow Metadata Limits
- **Total serialized size**: 2048 bytes maximum
- **Tags**: 30 items maximum
- **Custom fields**: 15 key-value pairs maximum
- **Individual strings**: 256 characters maximum

## Authorization Model

### Bounty Escrow
- **Setting metadata**: Only the original depositor can set/update metadata
- **Rationale**: Prevents unauthorized modification of bounty descriptions

### Program Escrow
- **Setting metadata**: Only the authorized payout key can set/update metadata
- **Rationale**: Maintains centralized control over program information

## New Functions Added

### Bounty Escrow Contract
```rust
// Set/update metadata for a bounty
pub fn set_escrow_metadata(
    env: Env,
    bounty_id: u64,
    metadata: EscrowMetadata,
) -> Result<(), Error>

// Get metadata for a bounty
pub fn get_escrow_metadata(
    env: Env,
    bounty_id: u64,
) -> Result<Option<EscrowMetadata>, Error>

// Get combined escrow info with metadata
pub fn get_escrow_with_metadata(
    env: Env,
    bounty_id: u64,
) -> Result<EscrowWithMetadata, Error>
```

### Program Escrow Contract
```rust
// Set/update program metadata
pub fn set_program_metadata(
    env: Env,
    metadata: ProgramMetadata,
)

// Get program metadata
pub fn get_program_metadata(
    env: Env,
) -> Option<ProgramMetadata>

// Get combined program info with metadata
pub fn get_program_with_metadata(
    env: Env,
) -> ProgramWithMetadata
```

## Usage Examples

### Setting Bounty Metadata
```rust
let metadata = EscrowMetadata {
    repo_id: Some(String::from_str(&env, "stellar/rs-soroban-sdk")),
    issue_id: Some(String::from_str(&env, "123")),
    bounty_type: Some(String::from_str(&env, "bug")),
    tags: vec![
        &env,
        String::from_str(&env, "priority-high"),
        String::from_str(&env, "security"),
    ],
    custom_fields: map![
        &env,
        (String::from_str(&env, "difficulty"), String::from_str(&env, "medium")),
        (String::from_str(&env, "estimated_hours"), String::from_str(&env, "20"))
    ],
};

escrow_client.set_escrow_metadata(&42, &metadata)?;
```

### Setting Program Metadata
```rust
let metadata = ProgramMetadata {
    event_name: Some(String::from_str(&env, "Stellar Hackathon 2024")),
    event_type: Some(String::from_str(&env, "hackathon")),
    start_date: Some(String::from_str(&env, "2024-06-01")),
    end_date: Some(String::from_str(&env, "2024-06-30")),
    website: Some(String::from_str(&env, "https://hackathon.stellar.org")),
    tags: vec![
        &env,
        String::from_str(&env, "blockchain"),
        String::from_str(&env, "defi"),
        String::from_str(&env, "web3"),
    ],
    custom_fields: map![
        &env,
        (String::from_str(&env, "track_count"), String::from_str(&env, "5")),
        (String::from_str(&env, "expected_participants"), String::from_str(&env, "500")),
    ],
};

escrow_client.set_program_metadata(&metadata);
```

### Retrieving Combined Information
```rust
// For bounty escrow
let escrow_view = escrow_client.get_escrow_with_metadata(&42)?;
println!("Amount: {}", escrow_view.escrow.amount);
if let Some(meta) = escrow_view.metadata {
    println!("Repository: {:?}", meta.repo_id);
    println!("Issue: {:?}", meta.issue_id);
    println!("Tags: {:?}", meta.tags);
}

// For program escrow
let program_view = escrow_client.get_program_with_metadata();
println!("Balance: {}", program_view.program.remaining_balance);
if let Some(meta) = program_view.metadata {
    println!("Event: {:?}", meta.event_name);
    println!("Website: {:?}", meta.website);
    println!("Tags: {:?}", meta.tags);
}
```

## Benefits for Indexing and Analytics

### Off-Chain Indexing
- **Structured data**: Enables reliable parsing and categorization
- **Filtering**: Tags allow filtering by priority, type, domain, etc.
- **Search**: Repository and issue IDs enable precise lookups
- **Analytics**: Custom fields support arbitrary metrics and metadata

### Event Enhancement
- **Rich payloads**: Existing events can include metadata references
- **Better UX**: Frontends can display meaningful context
- **Automation**: Backend services can make decisions based on metadata

### Integration Examples
```javascript
// Query bounties by repository
const bounties = await indexer.queryBounties({
    repo_id: "stellar/rs-soroban-sdk",
    status: "Locked"
});

// Filter programs by date range
const activePrograms = await indexer.queryPrograms({
    start_date_gte: "2024-01-01",
    end_date_lte: "2024-12-31"
});

// Find high-priority security bounties
const securityBounties = await indexer.queryBounties({
    tags_contains: ["priority-high", "security"],
    bounty_type: "bug"
});
```

## Testing

Comprehensive tests have been added for both contracts:
- **Basic operations**: Setting and retrieving metadata
- **Authorization**: Verifying only authorized parties can modify metadata
- **Size limits**: Ensuring metadata stays within bounds
- **Edge cases**: Handling optional fields and non-existent entries

Tests are located in:
- `contracts/bounty_escrow/contracts/escrow/tests/metadata_tests.rs`
- `contracts/program-escrow/tests/metadata_tests.rs`

## Deployment Considerations

### Backward Compatibility
- Metadata is optional - existing contracts continue to work
- Core escrow functionality unchanged
- No migration required for existing data

### Storage Impact
- Metadata stored separately from core escrow data
- TTL extension ensures longevity of metadata
- Size limits prevent abuse and excessive costs

### Performance
- View functions are read-only with minimal gas costs
- Metadata setting is bounded by size limits
- No impact on core escrow transaction performance

This implementation provides a robust foundation for enhanced indexing and categorization while maintaining the security and simplicity of the core escrow functionality.