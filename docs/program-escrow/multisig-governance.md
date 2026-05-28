# Multisig Threshold for High-Value Admin Operations

## Overview

The `program-escrow` contract supports an optional M-of-N multisig approval gate
for critical admin operations. When enabled, operations that meet or exceed a
configured `high_value_threshold` must be **proposed**, **approved** by M signers,
and then **executed** — rather than taking effect immediately from a single key.

This reduces the blast radius of a compromised admin key: an attacker who controls
one key cannot unilaterally drain funds or change fee configuration.

## Configuration

### `set_multisig_threshold_config`

```
set_multisig_threshold_config(signers, required_approvals, high_value_threshold)
```

| Parameter | Type | Description |
|---|---|---|
| `signers` | `Vec<Address>` | N addresses authorised to approve ops |
| `required_approvals` | `u32` | M approvals needed (1 ≤ M ≤ N) |
| `high_value_threshold` | `i128` | Minimum value that triggers the gate |

- Admin only.
- Setting `required_approvals = 1` effectively disables the multisig gate (single-sig behaviour).
- Setting `high_value_threshold = 0` requires multisig for **all** admin ops.

## Propose → Approve → Execute Flow

### 1. Propose

```
propose_admin_op(kind, value, payload_hash) -> PendingAdminOp
```

- Caller must be the contract admin.
- The proposer's approval is **automatically counted**.
- Only one pending operation is allowed at a time. A second proposal while the
  first is still active panics with `PendingOpExists`.
- An expired pending op is silently replaced.
- The `payload_hash` is a caller-supplied hash of the operation's arguments.
  It is stored on-chain and must be re-supplied at execute time to prevent
  bait-and-switch attacks.

### 2. Approve

```
approve_admin_op(signer) -> PendingAdminOp
```

- Caller must be one of the configured `signers`.
- Each signer may approve at most once (`AlreadyApproved` otherwise).
- Panics with `PendingOpExpired` if the op has expired.

### 3. Execute

```
execute_admin_op(payload_hash) -> AdminOpKind
```

- Caller must be the contract admin.
- Panics with `InsufficientApprovals` if fewer than `required_approvals` have approved.
- Panics with `PayloadMismatch` if `payload_hash` differs from the stored hash.
- Panics with `PendingOpExpired` if the op has expired (also clears the pending op).
- On success, the pending op is removed from storage and an `AdmExec` event is emitted.

### Cancel

```
cancel_admin_op()
```

Admin can cancel a pending op at any time, clearing it from storage.

## Supported Operation Kinds

| `AdminOpKind` | Description |
|---|---|
| `UpdateFeeConfig` | Change global fee rates or recipient |
| `UpdateMultisigConfig` | Change the multisig threshold config itself |
| `EmergencyWithdraw` | Emergency withdrawal of contract funds |

## Expiry

Pending operations expire after **17,280 ledgers** (~24 hours at 5 s/ledger),
exported as `ADMIN_OP_EXPIRY_LEDGERS`. An expired op cannot be approved or
executed and is replaced on the next `propose_admin_op` call.

## Storage Layout

| Key | Type | Description |
|---|---|---|
| `MultisigThresholdConfig` | `MultisigThresholdConfig` | Global config |
| `PendingAdminOp` | `PendingAdminOp` | At most one pending op |

Both use **instance storage**.

## Events

| Symbol | Payload | Emitted when |
|---|---|---|
| `AdmProp` | `AdminOpProposedEvent` | Op proposed |
| `AdmAppr` | `AdminOpApprovedEvent` | Signer approves |
| `AdmExec` | `AdminOpExecutedEvent` | Op executed |
| `AdmExp` | `AdminOpKind` | Expired op cleared at execute time |

## Security Notes

- The `payload_hash` binding prevents a proposer from changing the operation
  arguments between proposal and execution.
- The proposer's auto-approval means a 1-of-1 config is equivalent to the
  existing single-admin behaviour — no breaking change.
- `approve_admin_op` requires `signer.require_auth()`, so signers must
  actively sign the transaction.
- The pending op is removed **before** the execute event is emitted, preventing
  re-entrancy through event callbacks.
- Non-signers and non-admins cannot interact with the pending op flow.

## Error Codes

| Code | Name | Meaning |
|---|---|---|
| 1200 | `PendingOpExists` | A non-expired pending op already exists |
| 1201 | `NoPendingOp` | No pending op found |
| 1202 | `PendingOpExpired` | Pending op TTL has passed |
| 1203 | `AlreadyApproved` | Signer already approved this op |
| 1204 | `NotASigner` | Caller not in the signers list |
| 1205 | `PayloadMismatch` | Payload hash does not match stored hash |
| 1206 | `InsufficientApprovals` | Not enough approvals to execute |
| 1207 | `InvalidMultisigConfig` | Config parameters are out of range |
