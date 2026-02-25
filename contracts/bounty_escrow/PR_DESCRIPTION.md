Closes #399

### Delegated Capability Tokens for Fine-Grained Authorization

This PR introduces a capability-token authorization pattern for escrow operations, enabling short-lived delegated rights without granting full admin roles.

#### Capability model

- `Capability { owner, holder, action, bounty_id, amount_limit, remaining_amount, expiry, remaining_uses, revoked }`
- `CapabilityAction`: `Claim`, `Release`, `Refund`
- Capabilities are explicit, scoped to one bounty, amount-bounded, usage-bounded, expiring, and revocable.

#### New contract APIs

- `issue_capability(owner, holder, action, bounty_id, amount_limit, expiry, max_uses) -> capability_id`
- `revoke_capability(owner, capability_id)`
- `get_capability(capability_id) -> Capability`
- Capability-enabled entrypoints:
  - `claim_with_capability(bounty_id, holder, capability_id)`
  - `release_with_capability(bounty_id, contributor, payout_amount, holder, capability_id)`
  - `refund_with_capability(bounty_id, amount, holder, capability_id)`

#### Security constraints

- Capabilities cannot exceed issuer authority:
  - `Release` and `Refund` capabilities must be issued by the current admin.
  - `Claim` capabilities must be issued by the current claim recipient.
  - `amount_limit` cannot exceed currently authorized escrow/claim amount.
- At use-time, authority is re-validated again (not only at issue-time), so stale capabilities cannot outlive authority changes.
- Expired or revoked capabilities are rejected.

#### Capability lifecycle events

- `cap_new`: capability issued
- `cap_use`: capability consumed
- `cap_rev`: capability revoked

#### Example flows

1. **Release up to X by delegate**
   - Admin issues `Release` capability (`amount_limit=600`, `max_uses=2`) to an operator.
   - Operator releases `400`, leaving `200` delegated budget.
2. **Refund once by delegate**
   - Admin issues `Refund` capability (`amount_limit=500`, `max_uses=1`) for one-time delegated refund.
3. **Claim by delegated caller**
   - Claim recipient issues `Claim` capability to a relayer who executes the claim on their behalf.

### Escrow Status Transitions Test Suite

This PR adds a comprehensive, fully-exhaustive test suite for all status transitions in the `BountyEscrow` contract, ensuring the runtime behavior rigidly aligns with the intended state machine.

### Security Fixes and Logic Refinement

In addition to the test suite, this PR includes critical improvements to the contract logic:

1. **🛡️ Pending Claim Guard**: Implemented a fix for Issue #391. The `refund` function now correctly blocks withdrawals if a claim/dispute is pending, preventing depositors from bypassing disputes.
2. **🔓 Enhanced Refund Support**:
    - **Early Refunds**: Admins can now approve refunds before the deadline.
    - **Partial Refunds**: Improved handling of partial releases followed by refunds, including the new `PartiallyRefunded` status.
    - **Refund History**: Added structured logging for all refund actions to facilitate auditing.
3. **🛠️ Architectural Alignment**: Synchronized the contract and tests with the latest internal structural changes, ensuring full CI compatibility.

**Testing Approach:**
1. **Table-Driven Tests:** A single `test_all_status_transitions` function enumerates over every transition defined in the matrix below. It mocks the contract's storage to forcefully simulate starting states (even if normally unreachable directly) ensuring exhaustive coverage over all edge cases.
2. **Individual Named Tests:** Separate descriptive test functions test each valid flow (e.g., `test_locked_to_released_succeeds`) and each specific invalid flow (e.g., `test_released_to_locked_fails`). All negative flow asserts strictly check the underlying `Error` variant as correctly thrown by the contract.
3. **Edge Case Tests:** Includes tests tracking the non-mutation of the state variable on failed transition attempts, idempotent failures, and correct fallthroughs of uninitialized escrows.

_Confirmation: All existing tests still pass cleanly, and new security invariants are strictly enforced._

### Transition Matrix
| FROM        | TO          | EXPECTED RESULT |
|-------------|-------------|-----------------|
| Locked      | Locked      | Err (invalid - BountyExists) |
| Locked      | Released    | Ok (allowed)    |
| Locked      | Refunded    | Ok (allowed)    |
| Released    | Locked      | Err (invalid - BountyExists) |
| Released    | Released    | Err (invalid - FundsNotLocked) |
| Released    | Refunded    | Err (invalid - FundsNotLocked) |
| Refunded    | Locked      | Err (invalid - BountyExists) |
| Refunded    | Released    | Err (invalid - FundsNotLocked) |
| Refunded    | Refunded    | Err (invalid - FundsNotLocked) |
