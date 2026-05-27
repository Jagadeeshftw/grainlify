# Delegate Authorization — Program Escrow RBAC

This document describes the delegate permission model for the `program-escrow` contract,
including normal delegation, permission bitmasks, and the emergency revocation fast-path.

---

## Overview

The program-escrow contract uses a two-tier access control model:

| Role                    | Set by                 | Capabilities |
|-------------------------|------------------------|--------------|
| **Admin**               | `initialize_contract`  | All operations, including emergency revocation |
| **Authorized Payout Key** | `init_program`       | Day-to-day payouts, delegate management |
| **Delegate**            | `set_program_delegate` | Subset of operations granted via bitmask |

---

## Permission Bitmask

Delegate permissions are stored as a `u32` bitmask. The following bits are defined:

| Constant                        | Bit | Value | Description                              |
|---------------------------------|-----|-------|------------------------------------------|
| `DELEGATE_PERMISSION_RELEASE`   | 0   | `0x1` | May trigger scheduled releases           |
| `DELEGATE_PERMISSION_REFUND`    | 1   | `0x2` | May initiate refunds                     |
| `DELEGATE_PERMISSION_UPDATE_META` | 2 | `0x4` | May update program metadata              |
| `DELEGATE_PERMISSION_MASK`      | —   | `0x7` | All valid bits combined                  |

Combining bits grants multiple permissions, e.g. `RELEASE | UPDATE_META = 0x5`.

Any bits outside `DELEGATE_PERMISSION_MASK` are rejected with a panic:
`"Unsupported delegate permissions"`.

---

## Setting a Delegate

```rust
client.set_program_delegate(
    &program_id,
    &caller,       // must be authorized_payout_key or admin
    &delegate,     // must differ from authorized_payout_key
    &permissions,  // bitmask, must be within DELEGATE_PERMISSION_MASK
);
```

**Emits:** `ProgramDelegateSetEvent`

```
topics : (PrgDlgS, program_id)
data   : ProgramDelegateSetEvent {
    version:    u32,         // EVENT_VERSION_V2 = 2
    program_id: String,
    delegate:   Address,
    permissions: u32,        // granted bitmask
    updated_by: Address,
    timestamp:  u64,
}
```

---

## Normal Revocation

The payout key owner or admin can revoke a delegate at any time:

```rust
client.revoke_program_delegate(&program_id, &caller);
```

**Emits:** `ProgramDelegateRevokedEvent` with `emergency: false`

---

## Emergency Revocation

### Motivation

`revoke_program_delegate` requires the caller to be either the **authorized payout key**
or the **admin**. If the payout key itself is compromised or unresponsive, a fast-path
is needed that only requires the **admin**.

`emergency_revoke_delegate` addresses this gap:

- Callable **only by the contract admin** (set via `initialize_contract`).
- **Immediately** zeros all delegate permissions in the same ledger as the call.
- **No grace period** — the revocation is atomic and instantaneous.
- **Idempotent** — safe to call even when no delegate is currently set.
- Emits `ProgramDelegateRevokedEvent` with `emergency: true` to distinguish it from
  normal revocation in indexers and monitoring systems.

### Usage

```rust
client.emergency_revoke_delegate(
    &program_id,
    &delegate,   // address of the compromised delegate
);
```

### Authorization Matrix

| Caller                  | `revoke_program_delegate` | `emergency_revoke_delegate` |
|-------------------------|:-------------------------:|:---------------------------:|
| Contract admin          | ✅                        | ✅                          |
| Authorized payout key   | ✅                        | ❌                          |
| Delegate                | ❌                        | ❌                          |
| Arbitrary third party   | ❌                        | ❌                          |

### Event Schema

```
topics : (PrgDlgR, program_id)
data   : ProgramDelegateRevokedEvent {
    version:    u32,     // EVENT_VERSION_V2 = 2
    program_id: String,
    delegate:   Address, // the address whose permissions were zeroed
    revoked_by: Address, // admin that performed the revocation
    timestamp:  u64,
    emergency:  bool,    // true  = emergency_revoke_delegate
                         // false = revoke_program_delegate
}
```

Indexers **must** check the `emergency` field to distinguish the two revocation paths.
A monitoring system should alert on any event where `emergency: true`.

### Error Codes

| Condition                        | Behaviour                           |
|----------------------------------|-------------------------------------|
| Admin key not initialized        | panic `"Not initialized"`           |
| Caller is not admin              | panic `"Unauthorized"`              |
| `program_id` does not exist      | panic `"Program not found"`         |
| No delegate currently set        | No-op; event still emitted          |

---

## Security Assumptions

1. **Admin key compromise is out of scope** — if the admin key is compromised the entire
   contract must be treated as compromised. Protect it with a hardware security module
   (HSM) or multi-sig.
2. **Delegate cannot self-revoke** — a delegate cannot call `emergency_revoke_delegate`
   on itself; this would allow a malicious delegate to prevent its own future liability.
3. **Permissions are zeroed atomically** — there is no window between the auth check and
   the storage write where a delegate could act with stale permissions.
4. **Idempotency** — repeated calls with the same arguments have no additional effect
   beyond the first; this prevents griefing via repeated event emission.

---

## Related Errors (`errors.rs`)

| Error Code                        | `u32` | Meaning                                      |
|-----------------------------------|-------|----------------------------------------------|
| `ContractError::Unauthorized`     | 1     | Caller lacks required role                   |
| `ContractError::DelegateNotSet`   | 102   | Operation requires a delegate that is absent |
| `ContractError::DelegatePermissionsInsufficient` | 103 | Delegate lacks required permission |

---

## See Also

- [contracts/DELEGATE_PERMISSION_MATRIX.md](../../contracts/DELEGATE_PERMISSION_MATRIX.md)
- [docs/events/full-event-schema-reference.md](../events/full-event-schema-reference.md)
- [contracts/program-escrow/src/lib.rs](../../contracts/program-escrow/src/lib.rs) — `set_program_delegate`, `revoke_program_delegate`, `emergency_revoke_delegate`
- [contracts/program-escrow/src/test_rbac.rs](../../contracts/program-escrow/src/test_rbac.rs) — RBAC test coverage
