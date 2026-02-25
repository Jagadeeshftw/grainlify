# Nonce and Replay Protection Strategy

## Overview

This document describes the unified nonce-based replay protection mechanism implemented across all Grainlify smart contracts that accept signed or authorized operations.

## Problem Statement

Without replay protection, an attacker could:
- Capture a valid signed transaction
- Replay it multiple times to drain funds
- Execute the same payout operation repeatedly
- Exploit cross-contract vulnerabilities if nonces aren't unified

## Solution: Unified Per-Signer Nonce Strategy

### Design Principles

1. **Per-Signer Nonces**: Each signer (admin, authorized payout key) maintains one monotonic nonce counter per contract
2. **Sequential Validation**: Nonces must be used in strict sequential order (0, 1, 2, ...)
3. **Atomic Increment**: Nonce validation and increment happen atomically to prevent race conditions
4. **Persistent Storage**: Nonces are stored in persistent contract storage to survive across ledger closes
5. **Simple Reasoning**: Easy to understand, test, and audit

### Implementation

#### Grainlify Core Nonce Module

The nonce logic is centralized in the `grainlify-core` library to ensure consistency across all contracts.

```rust
pub fn get_nonce(env: &Env, signer: &Address) -> u64
pub fn get_nonce_with_domain(env: &Env, signer: &Address, domain: Symbol) -> u64
```
- Returns the current nonce for a given signer (optionally within a domain)
- Returns 0 if no nonce has been set yet

```rust
pub fn validate_and_increment_nonce(env: &Env, signer: &Address, provided_nonce: u64) -> Result<(), NonceError>
pub fn validate_and_increment_nonce_with_domain(env: &Env, signer: &Address, domain: Symbol, provided_nonce: u64) -> Result<(), NonceError>
```
- Validates that `provided_nonce` matches the current nonce
- Returns `Err(NonceError::InvalidNonce)` if nonce is invalid
- Increments the nonce atomically after validation

#### Storage Schema

```rust
#[contracttype]
pub enum NonceKey {
    Signer(Address),
    SignerWithDomain(Address, Symbol),
}
```

Nonces are stored with the key `NonceKey::Signer(address)` in persistent storage.

## Protected Entrypoints

### BountyEscrowContract

#### `release_funds(bounty_id, contributor, nonce)`
- **Who**: Admin (backend)
- **Protection**: Requires admin's nonce
- **Purpose**: Prevents replaying fund releases to contributors

### ProgramEscrowContract

#### `single_payout(recipient, amount, nonce)`
- **Who**: Authorized payout key (backend)
- **Protection**: Requires payout key's nonce
- **Purpose**: Prevents replaying single payouts

#### `batch_payout(recipients, amounts, nonce)`
- **Who**: Authorized payout key (backend)
- **Protection**: Requires payout key's nonce
- **Purpose**: Prevents replaying batch payouts

## Usage Pattern

### Backend Integration

1. **Query Current Nonce**
   ```rust
   let current_nonce = contract.get_nonce(env, signer_address);
   ```

2. **Execute Operation with Nonce**
   ```rust
   contract.single_payout(env, recipient, amount, current_nonce);
   ```

3. **Handle Nonce Errors**
   - If transaction fails with "Invalid nonce", query the current nonce again
   - This indicates either a replay attempt or concurrent transaction

### Example Flow

```
Backend State: nonce = 0

1. Query nonce → 0
2. Execute payout with nonce=0 → Success
3. Query nonce → 1
4. Execute payout with nonce=1 → Success
5. Try to replay with nonce=0 → REJECTED (Invalid nonce)
```

## Security Properties

### Replay Protection
- ✅ Same transaction cannot be executed twice
- ✅ Old transactions cannot be replayed after new ones
- ✅ Nonces cannot be skipped or reused

### Cross-Entrypoint and Cross-Contract Considerations
- ✅ In each contract, all nonce-protected entrypoints share the same signer nonce
- ✅ A nonce used on `single_payout` cannot be reused on `batch_payout` (and vice versa)
- ✅ Soroban authorization binds signatures to contract id + function + args, preventing signature replay on another contract entrypoint
- ✅ Multiple authorized signers still operate independently

### Concurrency Handling
- ✅ Atomic validation and increment prevents race conditions
- ✅ Failed transactions don't increment nonces
- ✅ Backend can retry with correct nonce after conflicts

## Testing Strategy

### Test Coverage

1. **Basic Nonce Increment**
   - Verify nonce starts at 0
   - Verify nonce increments after each operation
   - Verify nonce is per-signer

2. **Replay Attack Prevention**
   - Attempt to reuse old nonce → Should fail
   - Attempt to use nonce=0 after nonce=1 → Should fail

3. **Out-of-Order Detection**
   - Attempt to skip nonces (use 5 when current is 0) → Should fail

4. **Cross-Entrypoint Protection**
   - Nonce increments on single_payout
   - Nonce increments on batch_payout
   - Same nonce cannot be used across different operations

### Test Files

- `/contracts/program-escrow/src/test.rs` - Program escrow replay tests
- `/contracts/bounty_escrow/contracts/escrow/src/test.rs` - Bounty escrow replay tests

## Migration Guide

### For Existing Deployments

If contracts are already deployed without nonce protection:

1. **Deploy New Version**: Deploy updated contracts with nonce support
2. **Initialize Nonces**: All signers start with nonce=0
3. **Update Backend**: Modify backend to query and provide nonces
4. **Gradual Rollout**: Test with small transactions first

### Backend Changes Required

```typescript
// Before
await contract.single_payout(recipient, amount);

// After
const nonce = await contract.get_nonce(signerAddress);
await contract.single_payout(recipient, amount, nonce);
```

## Future Enhancements

### Potential Improvements

1. **Nonce Expiration**: Add optional time-based nonce expiration
2. **Batch Nonce Validation**: Allow multiple operations with sequential nonces in one call
3. **Nonce Reset**: Admin function to reset nonces in emergency scenarios
4. **Gas Optimization**: Optimize storage access patterns for high-frequency operations

### Not Recommended

- ❌ Global nonces (reduces parallelism)
- ❌ Random nonces (harder to reason about, requires more storage)
- ❌ Timestamp-based nonces (vulnerable to clock manipulation)

## References

- EIP-2612: Permit extension for ERC-20 (similar nonce pattern)
- Ethereum account nonces (transaction ordering)
- Soroban persistent storage documentation

## Audit Checklist

- [ ] All signature-based entrypoints require nonces
- [ ] Nonce validation happens before state changes
- [ ] Nonce increment is atomic with validation
- [ ] Clear error messages on nonce mismatch
- [ ] Tests cover replay attacks across all entrypoints
- [ ] Tests verify per-signer nonce isolation
- [ ] Documentation explains usage patterns
- [ ] Backend integration guide is complete
