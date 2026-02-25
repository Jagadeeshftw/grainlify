# Safe Token Rescue Mechanism

Closes #[issue_id]

## Summary

Implements a carefully scoped "token rescue" mechanism that allows recovery of tokens accidentally sent directly to contract addresses, without enabling theft of escrow-managed funds.

## Problem

Users may mistakenly transfer tokens directly to the contract address through:
- Direct token transfers instead of using `lock_funds`
- Incorrect wallet configurations
- Copy-paste errors with addresses
- Integration bugs in frontend applications

Without a rescue mechanism, these tokens would be permanently locked in the contract.

## Solution

### Core Algorithm
```
Untracked Balance = Actual Contract Balance - Tracked Escrow Balance

Where Tracked Escrow Balance = Sum of remaining_amount for escrows with status:
- Locked
- PartiallyRefunded
```

Only the untracked balance can be rescued, ensuring escrow funds are never touched.

### New Functions

1. **`set_treasury(treasury: Address)`** - Admin-only, configure treasury address
2. **`get_treasury() -> Option<Address>`** - Public view, query treasury
3. **`get_untracked_balance() -> i128`** - Public view, calculate rescuable tokens
4. **`rescue_tokens() -> Result<i128, Error>`** - Admin-only, transfer untracked tokens to treasury

### Safety Guarantees

✅ **Escrow funds are protected** - Only untracked tokens can be rescued
✅ **Status filtering** - Only Locked/PartiallyRefunded escrows counted as tracked
✅ **Remaining amount** - Uses `remaining_amount` to account for partial operations
✅ **Admin-only** - Both treasury setup and rescue require admin authorization
✅ **Audit trail** - All rescue operations emit `TokensRescued` events
✅ **No reentrancy** - Atomic calculation and transfer

## Changes

### Code Changes
- **lib.rs**: Added 4 new functions, 2 error codes, 1 DataKey
- **events.rs**: Added `TokensRescued` event
- **test_token_rescue.rs**: 15 comprehensive tests (all passing)

### Documentation
- **TOKEN_RESCUE.md**: Complete feature documentation
- **RESCUE_IMPLEMENTATION_SUMMARY.md**: Implementation details

## Testing

### Test Coverage (15 tests, 100% passing)
- ✅ Treasury configuration
- ✅ Untracked balance calculation with various escrow states
- ✅ Successful rescue operations
- ✅ Escrow fund protection verification
- ✅ Error cases (no treasury, no untracked balance)
- ✅ Authorization checks
- ✅ Partial refund handling
- ✅ Multiple escrow scenarios
- ✅ Event emission

### Test Results
```
test result: ok. 412 passed; 0 failed; 0 ignored
```

All existing tests continue to pass - no breaking changes.

## Usage Example

```rust
// Setup (one-time)
contract.set_treasury(&treasury_address);

// Check for rescuable tokens
let untracked = contract.get_untracked_balance();
if untracked > 0 {
    // Rescue tokens
    let rescued = contract.rescue_tokens()?;
    // TokensRescued event emitted
}
```

## Edge Cases Handled

- ✅ Partial refunds (only remaining amount protected)
- ✅ Released escrows (not counted as tracked)
- ✅ Refunded escrows (not counted as tracked)
- ✅ Multiple escrows (correctly sums all tracked amounts)
- ✅ Zero/negative untracked balance (returns error)
- ✅ Multiple rescue operations (can be called repeatedly)

## Security Considerations

### What Can Be Rescued
- Tokens sent directly to contract (not through lock_funds)
- Tokens from bugs or user errors
- Excess tokens from any source

### What Cannot Be Rescued
- Tokens in Locked escrows
- Tokens in PartiallyRefunded escrows (remaining amount)
- Any tracked escrow balance

### Attack Scenarios Mitigated
1. **Admin Compromise**: Only untracked tokens at risk (escrow funds safe)
2. **Calculation Error**: Atomic operation prevents race conditions
3. **Reentrancy**: No external calls before state changes
4. **Integer Overflow**: Uses saturating arithmetic

## Comparison with Emergency Withdraw

| Feature | Token Rescue | Emergency Withdraw |
|---------|-------------|-------------------|
| Purpose | Recover accidentally sent tokens | Emergency fund recovery |
| Scope | Only untracked balance | All contract balance |
| Precondition | Treasury set, untracked > 0 | Contract paused |
| Safety | Escrow funds protected | Escrow funds included |
| Use Case | Normal operations | Critical emergency only |

## Deployment Checklist

- [ ] Review code changes
- [ ] Verify test coverage
- [ ] Check documentation completeness
- [ ] Set treasury address after deployment
- [ ] Set up monitoring for untracked balance
- [ ] Configure alerts for rescue operations
- [ ] Consider multi-sig treasury for additional security

## Documentation

- [TOKEN_RESCUE.md](contracts/bounty_escrow/contracts/escrow/TOKEN_RESCUE.md) - Complete feature documentation
- [RESCUE_IMPLEMENTATION_SUMMARY.md](contracts/bounty_escrow/contracts/escrow/RESCUE_IMPLEMENTATION_SUMMARY.md) - Implementation details

## Breaking Changes

None. This is a purely additive feature.

## Future Enhancements (Optional)

- Add per-operation rescue limits
- Implement multi-sig rescue for large amounts
- Add whitelist for self-rescue of mistaken transfers
- Track historical rescue operations on-chain
- Optional automatic rescue when threshold exceeded
