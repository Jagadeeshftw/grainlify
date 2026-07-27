# Program Escrow: Delegate Revocation & Atomicity

## Overview
The `program-escrow` contract allows a program's authorized payout key to delegate permissions (e.g., for payouts, releases, refunds) to secondary keys. To protect funds in the event a delegate is compromised or acts maliciously, the contract provides an `emergency_revoke_delegate` function. 

## Emergency Revocation
Unlike a standard delegate rotation or removal which may require cooperative actions, `emergency_revoke_delegate` provides a fast-path for the contract admin to strip a compromised delegate of all authority immediately.

### Security Invariants & Atomicity Guarantee
The most critical property of `emergency_revoke_delegate` is its **Atomicity Guarantee**:

1. **Immediate Effect**: Delegate permissions are zeroed atomically in the same ledger execution as the call. There is no grace period, unbonding delay, or complex state transition.
2. **Read Consistency**: Any query for the delegates of a program (e.g., `query_all_delegates`) reflects the revocation instantly. There is no caching or stale-read window. If an admin revokes a delegate, the very next ledger read in the same transaction will show the delegate as removed.
3. **Cross-Contract Consistency**: The view facade layer (`escrow-view-facade`) propagates this atomicity. The facade's `query_all_delegates` passes the read through to the underlying contract directly, ensuring off-chain indexers and user interfaces reading through the facade never see a revoked delegate.
4. **In-Flight Rejection**: Because the revocation applies instantly to the contract storage, any in-flight operations (such as `single_payout_by` or `batch_payout_by`) authorized by the revoked delegate will fail immediately. This neutralizes race-condition attacks where a compromised delegate attempts a front-running extraction payout in the same ledger close as the revocation.

## Auditing and Events
A successful emergency revocation emits a `ProgramDelegateRevokedEvent` with `emergency: true`. This explicit flag allows off-chain monitoring systems, indexers, and alerts to distinguish an emergency intervention from a routine key rotation, ensuring swift security response coordination.
