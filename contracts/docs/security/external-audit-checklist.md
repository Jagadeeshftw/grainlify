# External Audit Checklist

This document provides a comprehensive overview of all entry points, access controls, error codes, and threat models for the Grainlify smart contracts.

## 1. Contract Entry Points & Access Controls

### 1.1 Program Escrow (`contracts/program-escrow/`)

| Function | Access Control | Description |
|----------|----------------|-------------|
| `init_program` | Organizer / Authorized Payout Key | Initializes a new program escrow with a specific token and controller. |
| `publish_program` | Authorized Payout Key | Transitions a program from Draft to Active status. |
| `init_program_with_metadata` | Authorized Payout Key | Initializes a program with additional metadata fields. |
| `batch_initialize_programs` | Open (Validation required) | Batch initialization of multiple programs. |
| `lock_program_funds` | Organizer (Implicit) | Locks funds into a program's prize pool. |
| `batch_lock` | Admin (via Pausable check) | Atomically locks funds for multiple programs. |
| `single_payout` | Authorized Payout Key | Distributes a single prize to a winner. |
| `batch_payout` | Authorized Payout Key | Distributes multiple prizes to winners atomically. |
| `batch_release` | Authorized Payout Key | Releases multiple scheduled payouts. |
| `trigger_program_releases` | Authorized Payout Key / Delegate | Triggers all scheduled releases that have passed their deadline. |
| `set_paused` | Admin | Toggles pause state for specific operations (lock, release, refund). |
| `set_maintenance_mode` | Admin | Toggles global maintenance mode. |
| `set_program_risk_flags` | Admin | Updates risk flags for a specific program. |
| `set_program_delegate` | Admin | Assigns a delegate with specific permissions to a program. |
| `revoke_program_delegate` | Admin | Revokes a program's delegate. |
| `update_program_metadata` | Admin / Delegate | Updates metadata for a specific program. |
| `propose_admin_rotation` | Admin | Initiates a two-step admin rotation. |
| `accept_admin_rotation` | Proposed Admin | Completes the admin rotation after the timelock. |
| `propose_controller_rotation` | Admin | Initiates a two-step controller rotation for a program. |
| `accept_controller_rotation` | Proposed Controller | Completes the controller rotation after the timelock. |
| `reset_circuit_breaker` | Admin | Resets the circuit breaker state for a program. |

### 1.2 Bounty Escrow (`contracts/bounty_escrow/`)

| Function | Access Control | Description |
|----------|----------------|-------------|
| `init` | Admin (One-time) | Initializes the contract with an admin and token address. |
| `lock_funds` | Depositor | Locks funds for a specific bounty with a deadline. |
| `batch_lock_funds` | Depositor(s) | Atomically locks funds for multiple bounties. |
| `release_funds` | Admin / Authorized Delegate | Releases escrowed funds to a contributor. |
| `batch_release_funds` | Admin / Authorized Delegate | Atomically releases funds for multiple bounties. |
| `refund` | Depositor (after deadline) / Admin | Refunds locked funds back to the depositor. |
| `approve_refund` | Admin | Pre-approves a refund before the deadline. |
| `set_paused` | Admin | Toggles pause state for contract operations. |
| `set_maintenance_mode` | Admin | Toggles maintenance mode. |
| `set_amount_policy` | Admin | Sets minimum and maximum lock amounts. |
| `set_token_fee_config` | Admin | Configures per-token fee rates and recipients. |
| `set_high_value_config` | Admin | Sets threshold and timelock for high-value releases. |
| `propose_admin` | Admin | Initiates admin rotation. |
| `accept_admin` | Proposed Admin | Completes admin rotation after timelock. |

### 1.3 Grainlify Core (`contracts/grainlify-core/`)

| Function | Access Control | Description |
|----------|----------------|-------------|
| `init_admin` | Admin (One-time) | Initializes the core contract with an admin. |
| `upgrade` | Admin | Upgrades the contract WASM hash (Single-admin path). |
| `propose_upgrade` | Signer | Proposes a new WASM hash for upgrade (Multisig path). |
| `approve_upgrade` | Signer | Approves a pending upgrade proposal. |
| `execute_upgrade` | Any (after threshold + timelock) | Executes a multisig-approved upgrade. |
| `set_timelock_delay` | Admin | Updates the timelock delay for upgrades. |
| `set_read_only` | Admin | Toggles read-only mode for the contract. |
| `commit_migration` | Admin | Pre-commits a migration hash for replay protection. |
| `migrate` | Admin | Executes a state migration for a specific version. |

## 2. Error Codes & Descriptions

### 2.1 Program Escrow (`errors.rs`)

| Code | Variant | Description |
|------|---------|-------------|
| 1 | `Unauthorized` | Caller is not authorized (not admin, not payout key, or lacks permissions). |
| 2 | `InvalidAmount` | Amount is zero, negative, exceeds maximum, or causes overflow. |
| 3 | `Paused` | Contract operation is paused (lock, release, or refund). |
| 4 | `MaintenanceMode` | Contract is in maintenance mode. |
| 5 | `ReadOnlyMode` | Contract is in read-only mode. |
| 6 | `InvalidProgramId` | Program ID is empty, too long, or contains invalid characters. |
| 7 | `ProgramNotFound` | Program does not exist in storage. |
| 8 | `ProgramAlreadyExists` | Program ID is already registered. |
| 9 | `ProgramArchived` | Program is archived and cannot be modified. |
| 10 | `InsufficientBalance` | Program balance is insufficient for payout. |
| 11 | `Overflow` | Arithmetic overflow occurred. |
| 107 | `ProgramNotActive` | Operation requires Active status (currently Draft). |
| 415 | `IdempotencyKeyConflict` | Idempotency key already used with different parameters. |
| 1001 | `CircuitOpen` | Circuit breaker is open due to consecutive failures. |

### 2.2 Bounty Escrow (`Error` enum in `lib.rs`)

| Code | Variant | Description |
|------|---------|-------------|
| 1 | `AlreadyInitialized` | Contract already initialized. |
| 7 | `Unauthorized` | Caller lacks required permissions. |
| 13 | `InvalidAmount` | Amount is zero, negative, or exceeds available. |
| 14 | `InvalidDeadline` | Deadline is in the past or too far in the future. |
| 16 | `InsufficientFunds` | Contract has insufficient funds for the operation. |
| 23 | `TicketNotFound` | Claim ticket not found. |
| 24 | `TicketAlreadyUsed` | Claim ticket already used (replay prevention). |
| 34 | `ContractDeprecated` | New locks/registrations disabled. |
| 56 | `BountyNotFound` | Bounty ID does not exist. |

### 2.3 Grainlify Core (`ContractError` enum in `lib.rs`)

| Code | Variant | Description |
|------|---------|-------------|
| 1 | `AlreadyInitialized` | Contract already initialized. |
| 3 | `NotAdmin` | Caller is not the admin. |
| 101 | `ThresholdNotMet` | Multisig threshold not met for proposal. |
| 103 | `MigrationCommitmentNotFound` | Migration hash commitment not found. |
| 104 | `MigrationHashMismatch` | Migration hash does not match committed hash. |
| 105 | `TimelockDelayTooHigh` | Timelock delay exceeds maximum allowed (30 days). |

## 3. Threat Model

### 3.1 Reentrancy
*   **Threat**: An attacker triggers a payout to a malicious contract that calls back into the escrow contract to drain funds before state is updated.
*   **Mitigation**: 
    *   The contracts use a `reentrancy_guard` (Check-Effect-Interaction pattern).
    *   Soroban's `require_auth` mechanism and atomic transaction model inherently limit certain types of reentrancy.
    *   State (like `remaining_balance`) is updated *before* external token transfers where possible.

### 3.2 Oracle Manipulation
*   **Threat**: If the contract relied on an external price or state oracle, an attacker could manipulate the oracle to trigger excessive payouts.
*   **Mitigation**: 
    *   Grainlify contracts primarily rely on "Authorized Payout Keys" (trusted backends) rather than on-chain oracles.
    *   The "Authorized Payout Key" acts as a trusted oracle for winner selection.
    *   Risk is mitigated by the two-step controller rotation and the ability for the Admin to revoke/pause programs.

### 3.3 Fee Drain
*   **Threat**: An admin or attacker with configuration access sets fee rates or recipients to drain program funds or redirect fees.
*   **Mitigation**: 
    *   `MAX_FEE_RATE` is hardcoded (e.g., 10% in program-escrow, 50% in bounty-escrow) to prevent total drainage.
    *   Fee configurations require high-privilege (Admin) authorization.
    *   Audit events are emitted for every fee configuration change and fee collection.

## 4. Remediation Tracking Table

| Finding | Severity | Status | Owner |
|---------|----------|--------|-------|
| Missing `require_active_program` in `batch_payout` | High | Fixed | Dev |
| Potential integer overflow in `total_funds` | Medium | Fixed | Dev |
| Reentrancy risk in `single_payout` | High | Mitigated | Security |
| Unauthorized access to `reset_circuit_breaker` | Critical | Fixed | Admin |
| Lack of input validation on `program_id` | Low | Fixed | Dev |
