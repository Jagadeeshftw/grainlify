# Auto-Refund Trigger Permissions Tests

## Overview
Auto-refund permissions are now role-restricted for `refund()` in bounty escrow.

## Test File
- **Location**: `src/test_auto_refund_permissions.rs`
- **Module**: `test_auto_refund_permissions`

## Authorization Rules
- `refund()` now requires authenticated approval from both the contract **admin** and the escrow **depositor**.
- A call missing either required authorization fails at auth validation.
- Deadline and admin-approval eligibility checks still apply exactly as before.

## Test Coverage

### Authorized Roles
- `test_auto_refund_admin_can_trigger_after_deadline`
- `test_auto_refund_depositor_can_trigger_after_deadline`
- `test_auto_refund_admin_and_depositor_same_result`

### Unauthorized Caller
- `test_auto_refund_unauthorized_random_user_panics_on_missing_required_auth`

### Deadline and Idempotency
- `test_auto_refund_fails_before_deadline`
- `test_auto_refund_admin_cannot_bypass_deadline`
- `test_auto_refund_at_exact_deadline`
- `test_auto_refund_idempotent_second_call_fails`
- `test_auto_refund_balance_stable_after_first_refund`

## Security Notes
- Requiring dual authorization prevents unauthorized third-party automation triggers.
- Funds safety remains unchanged: refunds still settle to the depositor (or approved recipient).
- Double-refund remains prevented by escrow state transitions (`Locked` -> `Refunded`).
