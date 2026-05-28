# Delegation Security — Program Escrow

## Overview

`ProgramEscrowContract` supports a single-level delegate per program. A delegate
is an address that can act on behalf of the program owner (`authorized_payout_key`)
for a specific subset of operations, controlled by a bitmask.

## Permission Bitmask

| Constant                        | Bit | Grants                                      |
|---------------------------------|-----|---------------------------------------------|
| `DELEGATE_PERMISSION_RELEASE`   | 0   | Execute single/batch payouts and schedules  |
| `DELEGATE_PERMISSION_REFUND`    | 1   | Trigger refunds                             |
| `DELEGATE_PERMISSION_UPDATE_META` | 2 | Update program metadata                     |

`DELEGATE_PERMISSION_MASK` is the OR of all three bits. Any bitmask with bits
outside this mask is rejected with `"Unsupported delegate permissions"`.

## Security Invariants

### 1. Only owner or admin can set/revoke a delegate

`set_program_delegate` and `revoke_program_delegate` call
`require_program_owner_or_admin`, which accepts only:

- the program's `authorized_payout_key`, or
- the contract-level admin.

A delegate is **not** in this set. This prevents delegation chains where a
delegate grants its permissions (or a superset) to a third party.

### 2. No permission escalation

Because delegates cannot call `set_program_delegate`, they cannot:

- upgrade their own bitmask,
- grant a superset of their permissions to another address, or
- replace themselves with a different address.

### 3. Immediate revocation

When the owner calls `set_program_delegate` with a new address, the previous
delegate is atomically replaced. The old delegate loses all access on the same
ledger entry write — there is no grace period.

### 4. Delegate ≠ owner

The payout key cannot be registered as its own delegate
(`"Delegate must differ from owner"`). This prevents a degenerate state where
the owner appears to hold delegate-level permissions through a separate code
path.

### 5. Non-empty, valid bitmask required

A zero bitmask is rejected (`"Delegate permissions cannot be empty"`).
Bits outside `DELEGATE_PERMISSION_MASK` are rejected
(`"Unsupported delegate permissions"`). This ensures the stored bitmask always
represents a meaningful, forward-compatible permission set.

## Attack Vectors Mitigated

| Attack                                    | Mitigation                                              |
|-------------------------------------------|---------------------------------------------------------|
| Delegate re-delegates to third party      | `set_program_delegate` requires owner/admin auth        |
| Delegate escalates own bitmask            | Same — delegate is not an authorized caller             |
| Replaced delegate retains access          | Atomic overwrite; old address no longer matches         |
| Delegate self-revokes to clear audit trail| `revoke_program_delegate` requires owner/admin auth     |
| Bitmask with reserved bits set            | `validate_delegate_permissions` rejects unknown bits    |

## Test Coverage (`src/test_rbac.rs`)

| Test                                              | Property verified                                  |
|---------------------------------------------------|----------------------------------------------------|
| `test_delegate_cannot_redelegate_to_third_party`  | Re-delegation rejected                             |
| `test_delegate_with_partial_permissions_cannot_redelegate` | Partial-permission re-delegation rejected |
| `test_delegate_cannot_escalate_own_permissions`   | Self-escalation rejected                           |
| `test_delegate_cannot_revoke_itself`              | Self-revocation rejected                           |
| `test_arbitrary_address_cannot_set_delegate`      | Unauthenticated caller rejected                    |
| `test_admin_can_set_delegate`                     | Admin is a valid setter (positive control)         |
| `test_owner_can_replace_delegate`                 | Owner can atomically replace delegate              |
| `test_replaced_delegate_loses_access`             | Replaced delegate has no residual access           |
| `test_set_delegate_rejects_zero_permissions`      | Zero bitmask rejected                              |
| `test_set_delegate_rejects_unsupported_permission_bits` | Out-of-mask bits rejected                   |
| `test_owner_cannot_be_set_as_own_delegate`        | Owner ≠ delegate invariant enforced                |
