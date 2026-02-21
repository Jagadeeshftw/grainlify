Closes #399

### Escrow Status Transitions Test Suite

This PR adds a comprehensive, fully-exhaustive test suite for all status transitions in the `BountyEscrow` contract, ensuring the runtime behavior rigidly aligns with the intended state machine.

**Testing Approach:**
1. **Table-Driven Tests:** A single `test_all_status_transitions` function enumerates over every transition defined in the matrix below. It mocks the contract's storage to forcefully simulate starting states (even if normally unreachable directly) ensuring exhaustive coverage over all edge cases.
2. **Individual Named Tests:** Separate descriptive test functions test each valid flow (e.g., `test_locked_to_released_succeeds`) and each specific invalid flow (e.g., `test_released_to_locked_fails`). All negative flow asserts strictly check the underlying `Error` variant as correctly thrown by the contract.
3. **Edge Case Tests:** Includes tests tracking the non-mutation of the state variable on failed transition attempts, idempotent failures, and correct fallthroughs of uninitialized escrows.

_Confirmation: All existing tests still pass cleanly without changes to the underlying protocol logic._

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
