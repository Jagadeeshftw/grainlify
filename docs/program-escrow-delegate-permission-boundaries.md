# Program Escrow Delegate Permission Boundaries

This document clarifies the permission boundaries for delegates in the Program Escrow contract, specifically regarding administrative configuration setters.

## Administrative Configuration vs Delegate Permissions

Delegates are granted specific permissions via bitmasks (`DELEGATE_PERMISSION_RELEASE`, `DELEGATE_PERMISSION_REFUND`, `DELEGATE_PERMISSION_UPDATE_META`). These permissions allow them to perform specific actions on behalf of the program.

However, certain critical program configurations are strictly reserved for higher-authority roles (e.g., admin or the program's controller/authorized payout key). 

### Spend Limit Threshold (`set_program_spend_threshold`)
- **Authority**: Admin only.
- **Delegate Access**: None. A delegate holding any permission bit (including `DELEGATE_PERMISSION_UPDATE_META`) **cannot** configure the spend limit threshold. This prevents a low-privilege delegate from weakening the program's spend limit protections.

### Circuit Breaker Threshold (`set_program_circuit_breaker_threshold`)
- **Authority**: Program Controller (the `authorized_payout_key`).
- **Delegate Access**: None. A delegate holding any permission bit **cannot** configure the circuit breaker threshold. This prevents a low-privilege delegate from weakening the program's circuit breaker protections.

## Testing
These boundaries are explicitly tested in `contracts/program-escrow/src/rbac_tests.rs`. The tests ensure that delegates with specific permissions (`UPDATE_META`, `RELEASE`, `REFUND`) are rejected when attempting to call `set_program_spend_threshold` or `set_program_circuit_breaker_threshold`.
