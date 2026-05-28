# Admin & Controller Rotation — `program-escrow`

## Overview

The `ProgramEscrowContract` uses a **two-step rotation** pattern for both the contract admin and per-program controller (authorized payout key). A mandatory **24-hour time-lock** is enforced between proposing and accepting a rotation, giving the current admin time to cancel a proposal made by a compromised key.

---

## Security Motivation

Without a time-lock, an attacker who briefly compromises the proposer key can immediately complete the rotation before the team notices. The 24-hour delay provides a detection and cancellation window.

---

## Admin Rotation

### Step 1 — `propose_admin`

```rust
pub fn propose_admin(env: Env, proposed_admin: Address) -> Result<(), ContractError>
```

- Requires the current admin to authorize.
- Stores the proposed address and a `RoleTransitionState` (including `proposed_at` timestamp) in contract storage.
- Emits `AdminProposedEvent`.

### Step 2 — `accept_admin`

```rust
pub fn accept_admin(env: Env) -> Result<(), ContractError>
```

- Requires the proposed admin to authorize.
- **Enforces a 24-hour delay**: reverts with `RotationTimelockActive` if `now < proposed_at + ROTATION_TIMELOCK_DELAY`.
- On success, atomically updates the admin and clears the pending state.
- Emits `AdminAcceptedEvent`.

### Cancel — `cancel_admin_rotation`

```rust
pub fn cancel_admin_rotation(env: Env) -> Result<(), ContractError>
```

- Requires the **current admin** to authorize.
- Can be called at any time during the timelock window (or after).
- Clears both `PendingAdmin` and `PendingAdminState` from storage.
- Emits `AdminRotationCancelledEvent`.

---

## Controller Rotation

### Step 1 — `propose_controller`

```rust
pub fn propose_controller(
    env: Env,
    program_id: String,
    caller: Address,
    proposed_controller: Address,
) -> Result<ProgramData, ContractError>
```

- Requires the current controller or admin to authorize.
- Stores the proposed address and a `RoleTransitionState` in contract storage.
- Emits `ControllerProposedEvent`.

### Step 2 — `accept_controller`

```rust
pub fn accept_controller(env: Env, program_id: String) -> Result<ProgramData, ContractError>
```

- Requires the proposed controller to authorize.
- **Enforces a 24-hour delay**: reverts with `RotationTimelockActive` if `now < proposed_at + ROTATION_TIMELOCK_DELAY`.
- On success, atomically updates the program's `authorized_payout_key` and clears the pending state.
- Emits `ControllerAcceptedEvent`.

### Cancel — `cancel_controller_rotation`

```rust
pub fn cancel_controller_rotation(
    env: Env,
    program_id: String,
    caller: Address,
) -> Result<ProgramData, ContractError>
```

- Requires the current controller or admin to authorize.
- Can be called at any time during the timelock window (or after).
- Clears both `PendingController` and `PendingControllerState` from storage.
- Emits `ControllerRotationCancelledEvent`.

---

## Time-Lock Constant

| Constant | Value | Description |
|----------|-------|-------------|
| `ROTATION_TIMELOCK_DELAY` | `86_400` | Mandatory delay in seconds (24 hours) between proposal and acceptance |

---

## Error Codes

| Error | Code | Description |
|-------|------|-------------|
| `AdminRotationInProgress` | 1200 | A rotation is already pending; cancel it first |
| `NoAdminRotationInProgress` | 1201 | No pending proposal to accept or cancel |
| `InvalidAdminRotationState` | 1202 | Storage inconsistency (should not occur in normal operation) |
| `ControllerRotationInProgress` | 1203 | A rotation is already pending for this program |
| `NoControllerRotationInProgress` | 1204 | No pending proposal for this program |
| `InvalidControllerRotationState` | 1205 | Storage inconsistency |
| `RotationTimelockActive` | 1209 | The 24-hour delay has not yet elapsed since the proposal |

---

## Storage Keys

| Key | Type | Description |
|-----|------|-------------|
| `PendingAdmin` | `Address` | Proposed admin address |
| `PendingAdminState` | `RoleTransitionState` | Full transition state including `proposed_at` |
| `PendingController(program_id)` | `Address` | Proposed controller address |
| `PendingControllerState(program_id)` | `RoleTransitionState` | Full transition state including `proposed_at` |

---

## Sequence Diagram

```
Current Admin          Proposed Admin         Contract
     |                      |                    |
     |-- propose_admin() --->|                    |
     |                      |  stores PendingAdmin + PendingAdminState(proposed_at=T)
     |                      |                    |
     |   [24h window — admin can cancel]          |
     |                      |                    |
     |                      |-- accept_admin() -->|
     |                      |  if now < T + 86400 → RotationTimelockActive
     |                      |  if now >= T + 86400 → success, admin updated
```

---

## Example

```rust
// Step 1: propose at timestamp T
env.ledger().with_mut(|li| li.timestamp = T);
contract.propose_admin(&new_admin);

// Step 2: accept after 24h
env.ledger().with_mut(|li| li.timestamp = T + 86_400);
contract.accept_admin(); // succeeds

// Or cancel within the window
env.ledger().with_mut(|li| li.timestamp = T + 3_600); // 1h later
contract.cancel_admin_rotation(); // cancels safely
```
