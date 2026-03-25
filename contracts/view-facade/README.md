# View Facade

A **read-only aggregation contract** for the Grainlify ecosystem on Stellar/Soroban.

## Purpose

Registers known escrow and core contract addresses so dashboards, indexers, and wallets can discover and interrogate them through a single, stable endpoint — without coupling to a specific contract type or requiring knowledge of individual deployment addresses.

## Security Model

| Property | Detail |
|---|---|
| **No fund custody** | This contract holds no tokens and performs no token transfers |
| **No external writes** | State changes are limited to this contract's own instance storage |
| **Immutable admin** | Set once at `init`; can never be overwritten — prevents privilege escalation |
| **Double-init protection** | A second `init` call is rejected with `FacadeError::AlreadyInitialized` |
| **Admin-gated mutations** | `register` and `deregister` both call `admin.require_auth()` |

## Initialization

The contract must be initialized exactly once before any registry mutations can occur.

```bash
# 1. Deploy the contract
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/view_facade.wasm \
  --source ADMIN_SECRET_KEY

# 2. Initialize (one-time, admin is immutable after this)
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source ADMIN_SECRET_KEY \
  -- init \
  --admin <GADMIN_ADDRESS>
```

> **Tip:** Deploy and initialize in the same transaction (or within the same script) on public networks to prevent front-running.

## Public Interface

### Admin functions

| Function | Description |
|---|---|
| `init(admin)` | Initialize with immutable admin. Returns `FacadeError::AlreadyInitialized` if called twice. |
| `register(address, kind, version)` | Add a contract to the registry. Admin-only. |
| `deregister(address)` | Remove a contract from the registry. Admin-only. No-op if address not found. |

### View functions (no auth required)

| Function | Returns | Description |
|---|---|---|
| `get_admin()` | `Option<Address>` | `Some(admin)` after init, `None` before |
| `list_contracts()` | `Vec<RegisteredContract>` | All registered entries in insertion order |
| `contract_count()` | `u32` | Number of registered contracts |
| `get_contract(address)` | `Option<RegisteredContract>` | Lookup by address |

## Query Examples

Typical dashboard flow:

1. Call `contract_count()` to estimate the size of the registry snapshot.
2. Call `list_contracts()` to render the current list view in registration order.
3. Call `get_contract(address)` when refreshing details for one selected contract card.

## Query Limits

- `get_contract(address)` performs an `O(n)` scan over the registry.
- `list_contracts()` returns the full registry in a single response.
- These tradeoffs are acceptable for the intended small, curated set of Grainlify contracts, but the facade should not be treated as a general-purpose index for unbounded datasets.

### Contract kinds

```rust
pub enum ContractKind {
    BountyEscrow,
    ProgramEscrow,
    SorobanEscrow,
    GrainlifyCore,
}
```

## Events

### `Initialized`

Emitted once when `init` succeeds.

| Field | Type | Description |
|---|---|---|
| `admin` | `Address` | The administrator address stored at initialization |

**Topic:** `("facade", "init")`

## Error Codes

| Code | Value | Meaning |
|---|---|---|
| `AlreadyInitialized` | 1 | `init` was called on an already-initialized contract |
| `NotInitialized` | 2 | `register` or `deregister` called before `init` |

## Testing

Run the full test suite from the `contracts/` workspace:

```bash
cargo test -p view-facade
```

Or from within the crate directory:

```bash
cd contracts/view-facade
cargo test
```

Expected output:

```
running 14 tests
test test::test_contract_count_initially_zero ... ok
test test::test_deregister_before_init_rejected ... ok
test test::test_deregister_contract ... ok
test test::test_deregister_nonexistent_is_noop ... ok
test test::test_double_init_rejected ... ok
test test::test_get_admin_before_init_returns_none ... ok
test test::test_get_contract_returns_first_match_for_duplicate_addresses ... ok
test test::test_get_contract_not_found ... ok
test test::test_init_emits_initialized_event ... ok
test test::test_init_stores_admin ... ok
test test::test_list_and_count_contracts ... ok
test test::test_register_all_contract_kinds ... ok
test test::test_register_and_lookup_contract ... ok
test test::test_register_before_init_rejected ... ok

test result: ok. 14 passed; 0 failed; 0 ignored
```

## Building

```bash
cargo build --release --target wasm32-unknown-unknown
```

## Design Notes

- **Instance storage** is used for both `Admin` and `Registry` keys, ensuring data persists across WASM upgrades.
- `deregister` scans the registry linearly (appropriate for the small registry sizes expected in this use-case).
- The event struct `InitializedEvent` follows the same pattern as `ProgramInitializedEvent` in `program-escrow`.
