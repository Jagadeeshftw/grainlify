# Escrow Contract Authorization Model

## Overview

The Grainlify Escrow Contract implements a comprehensive role-based access control (RBAC) system to ensure only authorized parties can trigger critical contract functions. This document details the authorization architecture, security guarantees, and implementation patterns.

## Authorization Model

### Roles

The contract defines two primary roles:

#### 1. **Backend Role**
- **Purpose**: Automated payout operations
- **Permissions**:
  - `release_funds()` - Transfer funds to individual contributors
  - `batch_payout()` - Transfer funds to multiple recipients in one transaction
  - `refund()` - Trigger refunds (inherited from Maintainer)
- **Use Case**: Backend service that processes contributor payouts automatically

#### 2. **Maintainer Role**
- **Purpose**: Manual refund operations and dispute resolution
- **Permissions**:
  - `refund()` - Transfer funds to recipients
- **Use Case**: Project maintainers who need to issue refunds

#### 3. **Admin Role** (Special)
- **Purpose**: Contract administration and key management
- **Permissions**:
  - `initialize()` - One-time contract setup
  - `set_authorized_key()` - Add or update authorized keys
  - `remove_authorized_key()` - Remove authorized keys
- **Use Case**: Contract deployer and administrator

### Role Hierarchy

```
Admin (separate, manages keys)
  ├── Backend (can do all Backend + Maintainer operations)
  └── Maintainer (can do Maintainer operations only)
```

**Key Principle**: Backend role is a superset of Maintainer permissions. Any Backend-authorized address can perform Maintainer operations.

## Authorization Flow

### 1. Initialization Phase

```
Admin calls initialize()
  ├── Requires: admin.require_auth()
  ├── Validates: Admin identity against stored admin
  ├── Stores: Admin address, token address, authorized keys
  └── Prevents: Re-initialization (one-time setup)
```

**Security**: Only the deploying admin can initialize. Contract prevents re-initialization.

### 2. Key Management Phase

```
Admin calls set_authorized_key(key, role)
  ├── Requires: admin.require_auth()
  ├── Validates: Caller is stored admin
  ├── Updates: authorized_keys map
  └── Emits: KeyManagementEvent for audit trail

Admin calls remove_authorized_key(key)
  ├── Requires: admin.require_auth()
  ├── Validates: Caller is stored admin
  ├── Safety Check: Prevents removing last Backend key
  ├── Updates: authorized_keys map
  └── Emits: KeyManagementEvent for audit trail
```

**Security**: Only admin can manage keys. Safety check prevents locking out all backends.

### 3. Operation Authorization Phase

For each state-changing operation:

```
Caller calls operation(args)
  ├── Step 1: caller.require_auth()
  │   └── Soroban SDK verifies caller's signature
  │
  ├── Step 2: verify_authorization(caller, required_role, action)
  │   ├── Checks: is_authorized(caller, required_role)
  │   ├── Emits: AuthorizationEvent for audit trail
  │   └── Panics: If not authorized
  │
  ├── Step 3: Input Validation
  │   ├── Amount > 0
  │   ├── Recipient address valid
  │   └── No overflow/underflow
  │
  ├── Step 4: Reentrancy Protection
  │   ├── check_reentrancy() - Set lock flag
  │   └── Prevents recursive calls
  │
  ├── Step 5: Balance Verification
  │   └── Contract has sufficient funds
  │
  ├── Step 6: Execute Transfer
  │   └── Token transfer via Soroban SDK
  │
  ├── Step 7: Release Lock
  │   └── release_reentrancy() - Clear lock flag
  │
  └── Step 8: Emit Event
      └── FundsReleasedEvent / RefundEvent / BatchPayoutEvent
```

## Authorization Checks

### `is_authorized(address, required_role) -> bool`

Checks if an address has the required role:

```rust
pub fn is_authorized(env: Env, address: Address, required_role: Role) -> bool {
    let auth_keys: Map<Address, Role> = env.storage().instance()
        .get(&"authorized_keys")
        .unwrap_or(Map::new(&env));

    if let Some(role) = auth_keys.get(address) {
        match required_role {
            // Backend can only perform backend operations
            Role::Backend => matches!(role, Role::Backend),
            // Maintainer operations can be performed by Backend or Maintainer
            Role::Maintainer => matches!(role, Role::Backend | Role::Maintainer),
        }
    } else {
        false
    }
}
```

**Logic**:
- Backend role: Exact match required
- Maintainer role: Backend OR Maintainer accepted (hierarchy)

### `verify_authorization(caller, required_role, action)`

Combines authorization check with audit event emission:

```rust
fn verify_authorization(env: &Env, caller: Address, required_role: Role, action: u32) {
    let is_auth = Self::is_authorized(env.clone(), caller.clone(), required_role);
    
    // Emit authorization event for audit trail
    AuthorizationEvent {
        caller: caller.clone(),
        action,
        authorized: is_auth,
    }.publish(env);

    if !is_auth {
        panic!("Unauthorized: insufficient permissions for this operation");
    }
}
```

**Benefits**:
- Centralized authorization logic
- Automatic audit trail for all operations
- Consistent error messages

## Security Guarantees

### 1. Authentication
- **Mechanism**: Soroban SDK's `require_auth()` verifies caller's cryptographic signature
- **Guarantee**: Only the address holder can invoke operations
- **Implementation**: Called on every state-changing function

### 2. Authorization
- **Mechanism**: Role-based access control with role hierarchy
- **Guarantee**: Only authorized roles can perform specific operations
- **Implementation**: `is_authorized()` checks before operation execution

### 3. Input Validation
- **Amount Validation**: All amounts must be > 0
- **Address Validation**: Recipient addresses must be valid
- **Overflow Protection**: `checked_add()` prevents arithmetic overflow
- **Empty Payload Check**: Batch operations reject empty lists

### 4. Reentrancy Protection
- **Mechanism**: Lock flag set before external calls, cleared after
- **Guarantee**: Prevents recursive calls during token transfers
- **Implementation**: `check_reentrancy()` and `release_reentrancy()`

### 5. Balance Verification
- **Mechanism**: Check contract balance before each transfer
- **Guarantee**: Prevents transfers exceeding available funds
- **Implementation**: Query token balance before transfer

### 6. Audit Trail
- **Mechanism**: Events emitted for all operations
- **Guarantee**: All actions are logged on-chain
- **Events**:
  - `AuthorizationEvent` - Authorization attempts (success/failure)
  - `FundsReleasedEvent` - Fund releases with authorized_by
  - `RefundEvent` - Refunds with authorized_by
  - `BatchPayoutEvent` - Batch payouts with authorized_by
  - `KeyManagementEvent` - Key additions/removals

## Operation-Specific Authorization

### `release_funds(backend, contributor, amount)`
- **Required Role**: Backend
- **Authorization Check**: `is_authorized(backend, Role::Backend)`
- **Audit Event**: AuthorizationEvent (action: 0)
- **Transfer Event**: FundsReleasedEvent with authorized_by

### `refund(caller, recipient, amount)`
- **Required Role**: Maintainer (Backend can also call)
- **Authorization Check**: `is_authorized(caller, Role::Maintainer)`
- **Audit Event**: AuthorizationEvent (action: 1)
- **Transfer Event**: RefundEvent with authorized_by

### `batch_payout(backend, payouts)`
- **Required Role**: Backend
- **Authorization Check**: `is_authorized(backend, Role::Backend)`
- **Audit Event**: AuthorizationEvent (action: 2)
- **Transfer Event**: BatchPayoutEvent with authorized_by

### `set_authorized_key(admin, key, role)`
- **Required Role**: Admin (exact match)
- **Authorization Check**: `admin == stored_admin`
- **Audit Event**: KeyManagementEvent (action: 0 = added)

### `remove_authorized_key(admin, key)`
- **Required Role**: Admin (exact match)
- **Authorization Check**: `admin == stored_admin`
- **Safety Check**: Prevents removing last Backend key
- **Audit Event**: KeyManagementEvent (action: 1 = removed)

### `get_authorized_keys_count()` (Read-Only)
- **Required Role**: None (public read-only)
- **Purpose**: Get count of authorized keys
- **Use Case**: Monitoring and auditing
- **Returns**: Number of authorized keys

### `check_authorization(address, role)` (Read-Only)
- **Required Role**: None (public read-only)
- **Purpose**: Check if address is authorized for role
- **Use Case**: Frontend/UI authorization checks
- **Returns**: true if authorized, false otherwise

## Security Considerations

### 1. Never Allow Unauthorized Fund Transfers
- ✅ All transfer functions require authorization
- ✅ Authorization checked before any state changes
- ✅ Unauthorized calls panic immediately

### 2. Validate All Input Addresses
- ✅ Recipient addresses validated as non-zero
- ✅ Admin address validated against stored admin
- ✅ Backend/Maintainer addresses validated via authorization check

### 3. Prevent Reentrancy Attacks
- ✅ Lock flag set before external calls
- ✅ Lock flag cleared after operation completes
- ✅ Recursive calls detected and rejected

### 4. Use Soroban's Built-in Authorization
- ✅ `require_auth()` verifies cryptographic signatures
- ✅ Prevents unauthorized address spoofing
- ✅ Leverages Soroban SDK's security infrastructure

### 5. Admin Key Compromise Mitigation
- ✅ Admin can only manage keys, not directly transfer funds
- ✅ Backend/Maintainer keys are separate from admin
- ✅ Multiple Backend keys can be configured for redundancy

### 6. No Unauthorized Fund Transfers
- ✅ All fund transfers require proper authorization
- ✅ Authorization hierarchy prevents privilege escalation
- ✅ Audit trail tracks all authorization attempts

## Testing Authorization

The contract includes comprehensive tests for authorization:

### Test Coverage (17 Tests)

1. **Authorization Hierarchy**
   - ✅ `test_authorization_hierarchy()` - Verifies role hierarchy
   - ✅ `test_backend_can_perform_maintainer_operations()` - Backend can do Maintainer ops
   - ✅ `test_maintainer_cannot_perform_backend_operations()` - Maintainer cannot do Backend ops
   - ✅ `test_maintainer_can_refund()` - Maintainer can refund

2. **Unauthorized Access Attempts**
   - ✅ `test_unauthorized_release_funds()` - Rejects unauthorized backend calls
   - ✅ `test_unauthorized_refund()` - Rejects unauthorized maintainer calls
   - ✅ `test_unauthorized_batch_payout()` - Rejects unauthorized backend calls
   - ✅ `test_unauthorized_set_key()` - Rejects unauthorized admin calls
   - ✅ `test_unauthorized_set_key_non_admin()` - Rejects non-admin key management
   - ✅ `test_unauthorized_remove_key_non_admin()` - Rejects non-admin key removal

3. **Input Validation**
   - ✅ `test_invalid_amount_release_funds()` - Rejects zero/negative amounts
   - ✅ `test_empty_batch_payout()` - Rejects empty payout lists

4. **Safety Checks**
   - ✅ `test_prevent_reinitialization()` - Prevents re-initialization
   - ✅ `test_cannot_remove_last_backend_key()` - Prevents removing last backend

5. **Key Management**
   - ✅ `test_set_and_remove_authorized_keys()` - Add/remove keys correctly

6. **Read-Only Operations**
   - ✅ `test_get_authorized_keys_count()` - Count authorized keys
   - ✅ `test_check_authorization_read_only()` - Check authorization without auth

### Running Tests

```bash
cd grainlify/contracts/escrow
cargo test
```

**Test Results**: All 17 tests pass ✅

## Deployment Checklist

- [ ] Deploy contract with secure admin address
- [ ] Initialize with Backend key (automated payouts)
- [ ] Initialize with Maintainer keys (refunds)
- [ ] Verify authorization checks in test environment
- [ ] Monitor AuthorizationEvent logs for unauthorized attempts
- [ ] Implement key rotation policy
- [ ] Document admin key backup procedures
- [ ] Set up monitoring for failed authorization attempts

## Future Enhancements

### Optional Multi-Signature Support
For critical operations, implement multi-signature requirements:
- Require N-of-M signatures for fund releases above threshold
- Require multi-sig for admin key changes
- Implement time-lock for sensitive operations

### Rate Limiting
- Limit number of operations per time period
- Limit maximum amount per transaction
- Implement daily/weekly caps

### Emergency Pause
- Admin can pause contract operations
- Prevents new operations during security incidents
- Allows recovery without re-deployment

## Conclusion

The Grainlify Escrow Contract implements a robust, multi-layered authorization system that:
- ✅ Prevents unauthorized fund transfers
- ✅ Validates all inputs and addresses
- ✅ Prevents reentrancy attacks
- ✅ Leverages Soroban's built-in security
- ✅ Maintains comprehensive audit trail
- ✅ Supports role-based access control with hierarchy
- ✅ Includes extensive test coverage

This authorization model ensures that only trusted parties (Backend for automated payouts, Maintainers for refunds) can trigger critical contract functions, while maintaining full auditability and security.
