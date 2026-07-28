# Program Delegate and Controller Rotation Interaction

This document explains the technical implementation, security implications, and intended behavior regarding the interaction between program delegates and controller (admin) rotations in the `ProgramEscrow` contract.

## Core Concepts

1. **Controller (`authorized_payout_key`)**: The primary owner/admin of a program. This key holds ultimate authority over the program, including the ability to propose a new controller, set delegates, and authorize payouts.
2. **Delegate**: An optional, secondary address assigned by the controller with a specific subset of permissions (e.g., authorizing releases or refunds).
3. **Controller Rotation**: A two-step process to transfer ownership of a program to a new controller.
   - **Step 1:** `propose_controller`
   - **Step 2:** `accept_controller` (subject to `ROTATION_TIMELOCK_DELAY`)

## Interaction Rules

### 1. Reassigning Delegates During an In-Flight Rotation

When a controller rotation is pending (i.e., `propose_controller` has been called but `accept_controller` has not yet occurred), the **outgoing controller retains full authority**.

This means that the outgoing controller may continue to perform administrative actions, including reassigning or revoking the program delegate via `set_program_delegate`.

**Crucially, reassigning the delegate does not invalidate the pending controller rotation.** The rotation proposal remains valid and can still be accepted by the proposed controller once the timelock expires.

### 2. Delegate Carryover on Acceptance

When the incoming controller finally calls `accept_controller`, they assume full control over the program.

At the exact moment of acceptance, **any previously assigned delegate and their permissions carry over automatically**. The `ProgramData` struct is updated to reflect the new `authorized_payout_key`, but the `delegate` field remains unchanged.

### Security Implications and Best Practices

- **For the Outgoing Controller**: You maintain operational continuity during the rotation timelock. If an emergency requires changing a delegate, you can do so without interrupting the ownership transfer.
- **For the Incoming Controller**: You inherit the security posture established by the outgoing controller. **You must proactively review the existing delegate and their permissions.** If you do not trust the existing delegate, you must explicitly call `revoke_program_delegate` immediately after accepting the controller role.

## Summary

- **`propose_controller`**: Does not touch delegates.
- **`set_program_delegate` (while rotation pending)**: Succeeds, rotation remains valid.
- **`accept_controller`**: Replaces the controller; keeps the existing delegate.
