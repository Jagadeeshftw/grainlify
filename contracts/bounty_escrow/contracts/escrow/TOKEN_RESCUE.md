# Safe Token Rescue Mechanism

## Overview

The token rescue mechanism allows recovery of tokens that were accidentally sent directly to the contract address, without enabling theft of escrow-managed funds. This is a critical safety feature for production deployments where users may mistakenly transfer tokens to the contract instead of using the proper `lock_funds` function.

## Problem Statement

Users may accidentally send tokens directly to the contract address through:
- Direct token transfers instead of using `lock_funds`
- Incorrect wallet configurations
- Copy-paste errors with addresses
- Integration bugs in frontend applications

Without a rescue mechanism, these tokens would be permanently locked in the contract, as they are not associated with any escrow record.

## Solution Design

### Core Principle: Untracked Balance Calculation

The rescue mechanism is based on a simple but powerful concept:

```
Untracked Balance = Actual Contract Balance - Tracked Escrow Balance
```

Where:
- **Actual Contract Balance**: The total token balance held by the contract (queried from the token contract)
- **Tracked Escrow Balance**: Sum of `remaining_amount` for all escrows with status `Locked` or `PartiallyRefunded`

Only the untracked balance can be rescued, ensuring escrow funds are never touched.

### Key Functions

#### 1. `set_treasury(treasury: Address)`
- **Authorization**: Admin only
- **Purpose**: Configure the destination address for rescued tokens
- **Usage**: Must be called before any rescue operations

#### 2. `get_treasury() -> Option<Address>`
- **Authorization**: Public view
- **Purpose**: Query the current treasury address
- **Returns**: `Some(address)` if set, `None` otherwise

#### 3. `get_untracked_balance() -> i128`
- **Authorization**: Public view
- **Purpose**: Calculate the amount of tokens that can be rescued
- **Algorithm**:
  1. Query actual token balance of contract
  2. Iterate through all escrows in `EscrowIndex`
  3. Sum `remaining_amount` for escrows with status `Locked` or `PartiallyRefunded`
  4. Return `actual_balance - tracked_balance`
- **Returns**: Amount of untracked tokens (can be 0 or negative if contract is underfunded)

#### 4. `rescue_tokens() -> Result<i128, Error>`
- **Authorization**: Admin only
- **Purpose**: Transfer all untracked tokens to the treasury
- **Preconditions**:
  - Contract must be initialized
  - Treasury must be set
  - Untracked balance must be > 0
- **Effects**:
  - Transfers untracked balance to treasury
  - Emits `TokensRescued` event
- **Returns**: Amount of tokens rescued

## Safety Guarantees

### 1. Escrow Funds Are Protected

The rescue mechanism only touches untracked tokens. Escrow funds are protected by:

- **Status Filtering**: Only `Locked` and `PartiallyRefunded` escrows are counted as tracked
- **Remaining Amount**: Uses `remaining_amount` (not original `amount`) to account for partial releases/refunds
- **Atomic Calculation**: Balance calculation and transfer happen in the same transaction

### 2. Authorization Controls

- **Admin-Only Operations**: Both `set_treasury` and `rescue_tokens` require admin authorization
- **Treasury Configuration**: Separate from admin address, allowing flexible governance models
- **Audit Trail**: All rescue operations emit events with full details

### 3. No Reentrancy Risk

The rescue function:
- Queries balances first
- Performs transfer second
- Does not call back into the contract
- Uses Soroban's built-in token client (no custom token logic)

## Usage Examples

### Setup Treasury

```rust
// Admin sets treasury address (one-time setup)
contract.set_treasury(&treasury_address);
```

### Check Untracked Balance

```rust
// Anyone can query untracked balance
let untracked = contract.get_untracked_balance();
if untracked > 0 {
    println!("Rescuable tokens: {}", untracked);
}
```

### Rescue Tokens

```rust
// Admin rescues untracked tokens
let rescued_amount = contract.rescue_tokens()?;
println!("Rescued {} tokens to treasury", rescued_amount);
```

## Event Schema

### TokensRescued Event

```rust
pub struct TokensRescued {
    pub version: u32,        // Event version (currently 2)
    pub amount: i128,        // Amount of tokens rescued
    pub treasury: Address,   // Destination address
    pub admin: Address,      // Admin who initiated rescue
    pub timestamp: u64,      // Ledger timestamp
}
```

**Topic**: `("rescue",)`

## Error Codes

| Error | Code | Description |
|-------|------|-------------|
| `NoUntrackedBalance` | 30 | No untracked tokens available to rescue |
| `TreasuryNotSet` | 31 | Treasury address not configured |
| `NotInitialized` | 2 | Contract not initialized |
| `Unauthorized` | 7 | Caller is not admin |

## Edge Cases Handled

### 1. Partial Refunds
When an escrow is partially refunded, only the `remaining_amount` is counted as tracked. The refunded portion is no longer protected.

**Example**:
- Original escrow: 1000 tokens
- Partial refund: 400 tokens
- Remaining tracked: 600 tokens
- If 500 tokens sent directly → 500 can be rescued

### 2. Released Escrows
Released escrows (status = `Released`) are not counted as tracked, since the funds have already left the contract.

### 3. Multiple Escrows
The calculation correctly sums across all active escrows:
- Escrow 1: 500 tokens (Locked)
- Escrow 2: 700 tokens (Locked)
- Escrow 3: 300 tokens (Released) ← not counted
- Total tracked: 1200 tokens

### 4. Zero or Negative Untracked Balance
If the contract is underfunded (tracked > actual), `get_untracked_balance()` returns 0 or negative, and `rescue_tokens()` will fail with `NoUntrackedBalance`.

### 5. Multiple Rescue Operations
Rescue can be called multiple times as tokens accumulate. Each operation transfers only the current untracked balance.

## Integration Guidelines

### For Frontend Developers

1. **Display Untracked Balance**: Show users if there are rescuable tokens
   ```typescript
   const untracked = await contract.get_untracked_balance();
   if (untracked > 0) {
     showRescueButton();
   }
   ```

2. **Rescue UI**: Provide admin interface for rescue operations
   ```typescript
   const rescued = await contract.rescue_tokens();
   showNotification(`Rescued ${rescued} tokens`);
   ```

3. **Treasury Display**: Show configured treasury address
   ```typescript
   const treasury = await contract.get_treasury();
   displayTreasury(treasury);
   ```

### For Backend Integrations

1. **Monitor Untracked Balance**: Set up alerts for untracked tokens
   ```rust
   let untracked = contract.get_untracked_balance();
   if untracked > threshold {
       alert_admin("Untracked tokens detected");
   }
   ```

2. **Automated Rescue**: Consider periodic rescue operations
   ```rust
   // Run daily or weekly
   if contract.get_untracked_balance() > 0 {
       contract.rescue_tokens()?;
   }
   ```

3. **Event Monitoring**: Track rescue events for accounting
   ```rust
   // Listen for TokensRescued events
   on_event("rescue", |event: TokensRescued| {
       log_rescue(event.amount, event.treasury, event.timestamp);
   });
   ```

## Testing

The implementation includes comprehensive tests covering:

- ✅ Basic rescue with no escrows
- ✅ Rescue with active escrows (funds protected)
- ✅ Rescue with released escrows (not counted)
- ✅ Rescue with refunded escrows (not counted)
- ✅ Rescue with partially refunded escrows (remaining counted)
- ✅ Multiple escrows with mixed statuses
- ✅ Multiple rescue operations
- ✅ Error cases (no treasury, no untracked balance)
- ✅ Authorization checks
- ✅ Event emission

Run tests:
```bash
cd contracts/bounty_escrow/contracts/escrow
cargo test test_token_rescue
```

## Security Considerations

### Audited Scenarios

1. **Cannot Steal Escrow Funds**: Tracked balance calculation ensures escrow funds are never included in rescue
2. **Admin Compromise**: If admin is compromised, only untracked tokens can be stolen (escrow funds remain safe)
3. **Treasury Misconfiguration**: Treasury can be updated by admin if initially set incorrectly
4. **Race Conditions**: No race conditions possible due to atomic balance calculation and transfer

### Recommended Practices

1. **Multi-Sig Treasury**: Use a multi-signature wallet as treasury for additional security
2. **Timelock**: Consider adding a timelock delay for treasury changes
3. **Monitoring**: Set up alerts for rescue operations
4. **Regular Audits**: Periodically verify tracked vs actual balances match expectations

## Comparison with Emergency Withdraw

| Feature | Token Rescue | Emergency Withdraw |
|---------|-------------|-------------------|
| **Purpose** | Recover accidentally sent tokens | Emergency fund recovery |
| **Authorization** | Admin only | Admin only |
| **Precondition** | Treasury set, untracked > 0 | Contract must be paused (lock_paused = true) |
| **Scope** | Only untracked balance | All contract balance |
| **Safety** | Escrow funds protected | Escrow funds included (emergency only) |
| **Use Case** | Normal operations | Critical emergency |

**Key Difference**: Token rescue is safe for regular use, while emergency withdraw is a last resort that affects all funds.

## Future Enhancements

Potential improvements for future versions:

1. **Rescue Limits**: Add per-operation or time-based limits
2. **Multi-Sig Rescue**: Require multiple admin signatures for large rescues
3. **Whitelist**: Allow specific addresses to rescue their own mistaken transfers
4. **Rescue History**: Track historical rescue operations on-chain
5. **Automated Rescue**: Optional automatic rescue when untracked balance exceeds threshold

## Changelog

### Version 1.0 (Initial Implementation)
- Added `set_treasury` and `get_treasury` functions
- Added `get_untracked_balance` calculation
- Added `rescue_tokens` function
- Added `TokensRescued` event
- Added comprehensive test suite
- Added error codes `NoUntrackedBalance` and `TreasuryNotSet`
