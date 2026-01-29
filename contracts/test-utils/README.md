# Grainlify Test Utilities

A comprehensive testing library for Grainlify smart contracts.

## Modules

- **assertions**: Custom assertions for contract interactions (e.g., `assert_invocation`).
- **events**: Helpers for verifying Soroban events (`expect_events`).
- **generators**: Data generators for tests (`generate_random_address`).
- **mocks**: Mock contract factories.
- **setup**: Environment setup helpers (`TestEnv`).

## Usage

Add to `[dev-dependencies]`:

```toml
test-utils = { path = "contracts/test-utils" }
```

### Example

```rust
use test_utils::{TestEnv, assert_invocation};

#[test]
fn test_example() {
    let env = TestEnv::new();
    // Your test logic here
}
```
