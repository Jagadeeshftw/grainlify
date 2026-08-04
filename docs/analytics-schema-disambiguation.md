# Cross-Contract ABI Stability Matrix: Analytics Schema Disambiguation

## Overview

The `grainlify` project contains a naming collision between two distinct `Analytics` structs in different modules and contracts. While they share the same name (`Analytics`), they have entirely different, incompatible schemas.

This document serves as an explicit entry in the cross-contract ABI stability matrix to warn SDK authors, indexers, and developers of potential conflicts and data corruption if the structs are conflated.

## The Two Analytics Schemas

### 1. Financial Analytics (Top-Level Program Escrow)

- **Location**: `contracts/program-escrow/src/lib.rs` (Top-level scope)
- **Purpose**: Tracks financial metrics for a program escrow, including total funds locked and released.
- **Fields**:
  - `total_locked: i128`
  - `total_released: i128`
  - `total_payouts: u32`
  - `active_programs: u32`
  - `operation_count: u32`

### 2. Operational Analytics (Internal Monitoring)

- **Location**: `contracts/bounty_escrow/contracts/escrow/src/lib.rs` and `contracts/program-escrow/src/lib.rs` (Inside the `monitoring` module)
- **Purpose**: Tracks operational and system health metrics such as operation counts and error rates.
- **Fields**:
  - `operation_count: u64`
  - `unique_users: u64`
  - `error_count: u64`
  - `error_rate: u32`

## ABI Stability and Disambiguation Strategy

Due to backward compatibility requirements, renaming the top-level `Analytics` struct in `program-escrow` to something like `EscrowAnalytics` could break the cross-contract ABI. Therefore, we preserve the existing name and adopt the following disambiguation strategy:

1. **Doc Comments**: Both `Analytics` struct definitions include explicit, strongly-worded doc comments warning about the naming collision and cross-referencing the other.
2. **Compile-Time Assertions**: A test-time structural distinctness assertion has been added to `program-escrow/src/lib.rs`. It uses `core::mem::size_of` to assert that the two schemas (`Analytics` and `monitoring::Analytics`) do not accidentally converge in size or structure. If future modifications make the structs identical, the test will fail loudly to alert the developer.
3. **Integration Guidance**: 
   - Off-chain indexers and SDK consumers should use aliases (e.g., mapping the top-level struct to `EscrowAnalytics`) to avoid compilation errors and data deserialization failures.
   - When importing the struct in cross-contract SDKs, explicitly qualify the path (e.g., `program_escrow::Analytics` vs `program_escrow::monitoring::Analytics`) to prevent overlap.
