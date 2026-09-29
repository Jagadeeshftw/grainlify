## PR: docs(contracts): event schema versioning audit and test fixes

### Summary
Audit of published event topics/payloads across `bounty-escrow`, `program-escrow`,
and `grainlify-core` for consistent `EVENT_VERSION_V2` naming and versioning.

### Changes

#### contracts/bounty_escrow/contracts/escrow/src/events.rs
Added missing `pub version: u32` field to 8 structs that violated the EVENT_VERSION_V2 contract:
- `FeeCollected`
- `BatchFundsLocked`
- `BatchFundsReleased`
- `FeeConfigUpdated`
- `FeeRoutingUpdated`
- `FeeRouted`
- `ApprovalAdded`
- `EmergencyWithdrawEvent`

#### contracts/bounty_escrow/contracts/escrow/src/lib.rs
Added `version: EVENT_VERSION_V2` to all struct initialisers affected by the above fix.

#### contracts/bounty_escrow/contracts/escrow/src/test_event_schema.rs (NEW)
New test module covering:
- version field presence on all fixed structs
- 9-byte topic symbol length enforcement for all 32 topics

#### docs/contracts/event_schema_indexer_guide.md (NEW)
Full indexer guide documenting:
- Topic layout for all 3 crates
- Unknown version handling guidance
- Security invariants
- How to run tests

### Pre-existing errors (not in scope)
14 pre-existing compile errors in `test_timelock.rs` and `gas_budget.rs`
exist before and after this PR. None are in files touched by this audit.

### Test command
```bash
cd contracts/bounty_escrow && cargo test test_event_schema
```
