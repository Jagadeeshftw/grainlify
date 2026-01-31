# Test Utilities Library

Comprehensive testing utilities for Soroban smart contract development. This library provides reusable helpers, factories, and assertion utilities to simplify test development and reduce boilerplate code.

## Features

- **Contract Factories**: Easy creation of test contracts (escrow, token)
- **Test Setup Helpers**: Comprehensive `TestSetup` struct with all common components
- **Assertion Utilities**: Common assertions for escrow status, amounts, balances, etc.
- **Test Data Generators**: Generate test data (addresses, amounts, deadlines, etc.)
- **Time Manipulation**: Helpers for advancing time and creating deadlines
- **Balance Verification**: Utilities for checking and verifying token balances

## Installation

Add to your `Cargo.toml`:

```toml
[dev-dependencies]
test-utils = { path = "../test-utils" }
bounty-escrow = { path = "../bounty_escrow/contracts/escrow" }
```

## Usage Examples

### Basic Test Setup

```rust
use test_utils::TestSetup;
use test_utils::assertions::*;
use test_utils::time::*;
use bounty_escrow::EscrowStatus;

#[test]
fn test_lock_funds() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = future_deadline(&setup.env, Some(3600));
    
    setup.lock_funds(bounty_id, amount, deadline);
    
    assert_escrow_status(&setup.escrow, bounty_id, EscrowStatus::Locked);
    assert_escrow_amount(&setup.escrow, bounty_id, amount);
}
```

### Using Generators

```rust
use test_utils::generators::*;
use test_utils::TestSetup;

#[test]
fn test_with_generated_data() {
    let setup = TestSetup::new();
    let bounty_id = generate_bounty_id(None);
    let amount = generate_amount(1000, Some(10)); // 10000
    let deadline = generate_deadline(&setup.env, Some(86400));
    
    setup.lock_funds(bounty_id, amount, deadline);
}
```

### Time Manipulation

```rust
use test_utils::time::*;
use test_utils::TestSetup;

#[test]
fn test_deadline_passed() {
    let setup = TestSetup::new();
    let deadline = past_deadline(&setup.env, Some(100));
    
    setup.lock_funds(1, 1000, deadline);
    
    // Advance time to pass deadline
    advance_time(&setup.env, 200);
    
    // Now refund should be allowed
}
```

### Balance Verification

```rust
use test_utils::balances::*;
use test_utils::TestSetup;

#[test]
fn test_balance_changes() {
    let setup = TestSetup::new();
    let initial = get_initial_balance(&setup.token, &setup.contributor);
    
    setup.lock_funds(1, 1000, future_deadline(&setup.env, None));
    setup.release_funds(1, None);
    
    verify_balance_change(&setup.token, &setup.contributor, initial, 1000);
}
```

### Multiple Contributors

```rust
use test_utils::TestSetup;

#[test]
fn test_multiple_contributors() {
    let (setup, contributors) = TestSetup::with_contributors(3);
    
    setup.lock_funds(1, 3000, future_deadline(&setup.env, None));
    
    // Release to each contributor
    for contributor in contributors.iter() {
        setup.escrow.release_funds(&1, contributor);
    }
}
```

### Custom Mint Amount

```rust
use test_utils::TestSetup;

#[test]
fn test_custom_mint() {
    let setup = TestSetup::with_mint_amount(5_000_000);
    // Depositor now has 5,000,000 tokens
}
```

## Module Reference

### `factories`

- `create_token_contract(env, admin)` - Create a token contract
- `create_escrow_contract(env)` - Create an escrow contract
- `create_initialized_escrow(env, admin)` - Create fully initialized escrow with token

### `setup`

- `TestSetup::new()` - Create standard test setup
- `TestSetup::with_mint_amount(amount)` - Create setup with custom mint amount
- `TestSetup::with_contributors(count)` - Create setup with multiple contributors
- `TestSetup::lock_funds(bounty_id, amount, deadline)` - Convenience method
- `TestSetup::release_funds(bounty_id, contributor)` - Convenience method

### `assertions`

- `assert_escrow_status(escrow, bounty_id, status)` - Assert escrow status
- `assert_escrow_amount(escrow, bounty_id, amount)` - Assert escrow amount
- `assert_escrow_depositor(escrow, bounty_id, depositor)` - Assert depositor
- `assert_escrow_deadline(escrow, bounty_id, deadline)` - Assert deadline
- `assert_balance(token, address, balance)` - Assert token balance
- `assert_balances(token, expected_balances)` - Assert multiple balances
- `assert_escrow_exists(escrow, bounty_id)` - Assert escrow exists

### `generators`

- `generate_bounty_id(index)` - Generate bounty ID
- `generate_amount(base, multiplier)` - Generate amount
- `generate_deadline(env, offset_seconds)` - Generate deadline
- `generate_addresses(env, count)` - Generate multiple addresses
- `standard_amount()` - Standard test amount (1000)
- `large_amount()` - Large test amount (1,000,000)
- `small_amount()` - Small test amount (100)

### `time`

- `advance_time(env, seconds)` - Advance ledger timestamp
- `set_time(env, timestamp)` - Set ledger timestamp
- `current_time(env)` - Get current timestamp
- `past_deadline(env, seconds_ago)` - Create past deadline
- `future_deadline(env, seconds_from_now)` - Create future deadline

### `balances`

- `get_initial_balance(token, address)` - Get initial balance
- `verify_balance_change(token, address, initial, expected_change)` - Verify balance change
- `verify_all_zero(token, addresses)` - Verify all addresses have zero balance

## Best Practices

1. **Use TestSetup for comprehensive tests**: It provides all commonly needed components
2. **Use generators for test data**: Makes tests more readable and maintainable
3. **Use assertion utilities**: Provides better error messages and reduces boilerplate
4. **Use time helpers**: Makes time-based tests more reliable
5. **Verify balances**: Always verify balances after transactions

## Contributing

When adding new utilities:

1. Add to the appropriate module
2. Include comprehensive documentation
3. Add usage examples
4. Update this README
