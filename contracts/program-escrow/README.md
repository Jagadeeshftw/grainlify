# Program Escrow Contract

> **Authoritative Contract**: `contracts/program-escrow` (crate: `program-escrow`)  
> **Deployment Status**: Authoritative deployable contract managing hackathon and grant prize pools on Stellar. (For the standalone Soroban workspace contract, see `soroban-program-escrow` in `soroban/contracts/program-escrow`).

A Soroban smart contract for managing program-level escrow funds for hackathons and grant programs. This contract handles prize pools, tracks balances, and enables automated batch payouts to multiple contributors.

## Features

- **Program Initialization**: Create a new escrow program with authorized payout key
- **Fund Locking**: Lock funds into the escrow (tracks total and remaining balance)
- **Single Payout**: Transfer funds to a single recipient
- **Batch Payout**: Transfer funds to multiple recipients in a single transaction
- **Release Schedules (Vesting)**: Queue timestamp-based releases and execute them when due
- **Balance Tracking**: Accurate tracking of total funds and remaining balance
- **Authorization**: Only authorized payout key can trigger payouts
- **Event Emission**: All operations emit events for off-chain tracking
- **Payout History**: Maintains a complete history of all payouts
- **Dispute Resolution**: Admin-controlled dispute lifecycle that blocks payouts while a dispute is open

## Granular Pause Matrix

Pause flags are operation-specific rather than global:

| Operation | Pause flag |
|---|---|
| `lock_program_funds` | `lock_paused` |
| `single_payout` | `release_paused` |
| `batch_payout` | `release_paused` |
| `trigger_program_releases` | `release_paused` |
| `create_pending_claim` | `release_paused` |
| `execute_claim` | `release_paused` |
| `cancel_claim` | `refund_paused` |
| read-only queries | unaffected |

Claim semantics follow the same split used in `bounty_escrow`:

- creating or executing a claim is a release-path action and is blocked only by `release_paused`
- cancelling a pending claim is a refund-path action and is blocked only by `refund_paused`
- claims remain executable when only `lock_paused` is set, so existing approved payouts are not trapped during deposit-only incidents

## Dispute Lifecycle

A dispute can be raised by the contract admin to freeze all payout operations pending investigation.

```text
(no dispute) ──open_dispute()──► Open ──resolve_dispute()──► Resolved
                                   │
                          single_payout()  ← BLOCKED
                          batch_payout()   ← BLOCKED
```

### Entrypoints

| Function                 | Auth          | Description                                 |
| ------------------------ | ------------- | ------------------------------------------- |
| `open_dispute(reason)`   | Admin         | Opens a dispute; blocks all payouts         |
| `resolve_dispute(notes)` | Admin         | Resolves the open dispute; unblocks payouts |
| `get_dispute()`          | Public (view) | Returns the current `DisputeRecord`, if any |

### Rules

- Only **one active dispute** at a time. A second `open_dispute` while one is `Open` panics.
- `resolve_dispute` on a non-open record panics.
- After a dispute is `Resolved`, a new dispute can be opened (fresh incident).
- `lock_program_funds` is **not** blocked by a dispute — only payout operations are.
- Dispute state is stored in instance storage under `DataKey::Dispute`.

### Events

| Symbol    | Payload                | Trigger             |
| --------- | ---------------------- | ------------------- |
| `DspOpen` | `DisputeOpenedEvent`   | `open_dispute()`    |
| `DspRslv` | `DisputeResolvedEvent` | `resolve_dispute()` |

Both events carry `version: 2` for consistency with the rest of the event schema.

## Contract Structure

### Storage

The contract stores a single `ProgramData` structure containing:

- `program_id`: Unique identifier for the program/hackathon
- `total_funds`: Total amount of funds locked
- `remaining_balance`: Current available balance
- `authorized_payout_key`: Address authorized to trigger payouts (backend)
- `payout_history`: Vector of all payout records
- `token_address`: Address of the token contract for transfers

### Functions

#### `init_program(program_id, authorized_payout_key, token_address)`

Initialize a new program escrow.

**Parameters:**

- `program_id`: String identifier for the program
- `authorized_payout_key`: Address that can trigger payouts
- `token_address`: Address of the token contract to use

**Returns:** `ProgramData`

**Events:** `ProgramInitialized`

#### `lock_program_funds(amount)`

Lock funds into the escrow. Updates both `total_funds` and `remaining_balance`.

**Parameters:**

- `amount`: i128 amount to lock (must be > 0)

**Returns:** Updated `ProgramData`

**Events:** `FundsLocked`

#### `single_payout(recipient, amount, nonce)`

Transfer funds to a single recipient. Requires authorization.

**Parameters:**

- `recipient`: Address of the recipient
- `amount`: i128 amount to transfer (must be > 0)
- `nonce`: u64 nonce for replay protection

**Returns:** Updated `ProgramData`

**Events:** `Payout`

**Validation:**

- Only `authorized_payout_key` can call this function
- Amount must be > 0
- Sufficient balance must be available

#### `batch_payout(recipients, amounts)`

Transfer funds to multiple recipients in a single transaction. Requires authorization.

**Parameters:**

- `recipients`: Vec<Address> of recipient addresses
- `amounts`: Vec<i128> of amounts (must match recipients length)
- `nonce`: u64 nonce for replay protection

**Returns:** Updated `ProgramData`

**Events:** `BatchPayout`

**Validation:**

- Only `authorized_payout_key` can call this function
- Recipients and amounts vectors must have same length
- All amounts must be > 0
- Total payout must not exceed remaining balance
- Cannot process empty batch
- Nonce must match signer's current nonce

#### `get_program_info()`

View function to retrieve all program information.

**Returns:** `ProgramData`

#### `get_remaining_balance()`

View function to get the current remaining balance.

**Returns:** i128

#### `create_program_release_schedule(recipient, amount, release_timestamp)`

Create a time-based release that can be executed once the ledger timestamp reaches the schedule timestamp.

#### `trigger_program_releases()`

Execute all due release schedules where `ledger_timestamp >= release_timestamp`.

**Edge-case behavior validated in tests:**

- Exact boundary is accepted: release executes when `now == release_timestamp`
- Early execution is rejected: no release when `now < release_timestamp`
- Late execution is accepted: pending releases execute when `now >> release_timestamp`
- Overlapping schedules are supported: multiple due schedules execute in the same trigger call

## Events

### ProgramInitialized

Emitted when a program is initialized.

```
(ProgramInit, program_id, authorized_payout_key, token_address, total_funds)
```

### FundsLocked

Emitted when funds are locked into the escrow.

```
(FundsLocked, program_id, amount, remaining_balance)
```

### Payout

Emitted when a single payout is executed.

```
(Payout, program_id, recipient, amount, remaining_balance)
```

### BatchPayout

Emitted when a batch payout is executed.

```
(BatchPayout, program_id, recipient_count, total_amount, remaining_balance)
```

## Payout Semantics: Single vs Batch

Both `single_payout()` and `batch_payout()` operations mirror identical event and receipt semantics to ensure consistent auditing:

### Shared Behavior

- **Authorization**: Both require authorization from `authorized_payout_key`
- **Validation**: Both validate positive amounts, sufficient balance, and contract initialization
- **Atomicity**: Both atomically update balance and append to payout history
- **History**: Both append `PayoutRecord` entries with recipient, amount, and timestamp
- **Events**: Both emit versioned events with program_id and updated remaining_balance
- **Security**: Both protected by reentrancy guard, circuit breaker, and threshold monitors
- **Dispute Blocking**: Both operations are blocked when a dispute is open

### Event Differences (by design)

- **Single Payout**: Emits `Payout` event with specific `recipient` address
- **Batch Payout**: Emits `BatchPayout` event with `recipient_count` summary

This design allows off-chain systems to:

1. Audit individual winner payouts via `Payout` event
2. Verify batch operations via `BatchPayout` event metadata
3. Reconstruct full payout history from event log
4. Confirm balance decrements across both paths

### Implementation Coverage

- History appending: ✓ (both paths maintain `payout_history`)
- Balance decrement: ✓ (both paths update `remaining_balance`)
- Event emission: ✓ (both paths emit versioned events)
- Security validation: ✓ (both paths enforce identical checks)
- Test coverage: ✓ (comprehensive test suite in `test_payouts_splits.rs`)

## Usage Flow

1. **Initialize Program**: Call `init_program()` with program ID, authorized key, and token address
2. **Lock Funds**: Call `lock_program_funds()` to deposit funds (can be called multiple times)
3. **Execute Payouts**: Call `single_payout()` or `batch_payout()` to distribute funds
4. **Replay Safety**: Read `get_nonce(signer)` and pass that nonce to payout entrypoints
5. **Monitor**: Use `get_program_info()` or `get_remaining_balance()` to check status

## Security Considerations

- Only the `authorized_payout_key` can trigger payouts
- Balance validation prevents over-spending
- All amounts must be positive
- Payout history is immutable and auditable
- Token transfers use the Soroban token contract standard
- **Token Math Safety**: All token arithmetic (addition, subtraction, multiplication) is centralized in `token_math.rs` and utilizes checked mathematical operations arrayed with explicit panic messages to securely prevent overflow and underflow vulnerabilities.
- `token_address` must be a contract address (not an account address)
- Shared asset id rules are documented in `contracts/ASSET_ID_STRATEGY.md`

## Testing

Run tests with:

```bash
cargo test --target wasm32-unknown-unknown
```

## Building

Build the contract with:

```bash
soroban contract build
```

## Deployment

Deploy using Soroban CLI:

```bash
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/program_escrow.wasm \
  --source <your-account> \
  --network <network>
```

## Integration with Backend

The backend should:

1. Initialize the contract with the backend's authorized key
2. Monitor events for program state changes
3. Query `get_nonce()` and include nonce when calling payout entrypoints
4. Call `batch_payout()`/`single_payout()` after computing final scores and verifying KYC
5. Track payout history for audit purposes

## Error Codes

The contract uses a canonical error enum for all public entrypoints. Clients should parse these error codes to handle failures consistently.

### Error Code Table

| Code | Error | Description |
|------|-------|-------------|
| 1 | `AlreadyInitialized` | Program has already been initialized |
| 2 | `NotInitialized` | Program has not been initialized |
| 3 | `Unauthorized` | Caller is not authorized for this operation |
| 4 | `InsufficientBalance` | Insufficient balance for the requested operation |
| 5 | `InvalidAmount` | Amount must be greater than zero |
| 6 | `InvalidRecipient` | Recipient address is invalid |
| 7 | `InvalidNonce` | Nonce does not match expected value |
| 8 | `BatchTooLarge` | Batch size exceeds maximum allowed |
| 9 | `EmptyBatch` | Batch cannot be empty |
| 10 | `MismatchedLengths` | Recipients and amounts vectors must have same length |
| 11 | `ReleaseScheduleNotFound` | Release schedule does not exist |
| 12 | `ReleaseNotDue` | Release is not yet due for execution |
| 13 | `ReleaseAlreadyExecuted` | Release has already been executed |
| 14 | `DisputeAlreadyOpen` | A dispute is already open |
| 15 | `NoActiveDispute` | No active dispute to resolve |
| 16 | `PayoutsBlocked` | Payouts are blocked due to an open dispute |
| 17 | `LockPaused` | Lock operations are paused |
| 18 | `ReleasePaused` | Release operations are paused |
| 19 | `RefundPaused` | Refund operations are paused |
| 20 | `CircuitBreakerOpen` | Circuit breaker is open, operations temporarily blocked |
| 21 | `ThresholdExceeded` | Operation would exceed threshold limits |
| 22 | `InvalidTimestamp` | Release timestamp must be in the future |
| 23 | `DuplicateRecipient` | Duplicate recipient in batch payout |

### Error Handling Best Practices

1. **Parse Error Codes**: Always parse the error code from the contract response
2. **User-Friendly Messages**: Map error codes to user-friendly messages in your UI
3. **Retry Logic**: Implement retry logic for transient errors (e.g., circuit breaker open)
4. **Logging**: Log error codes for debugging and monitoring
5. **Validation**: Validate inputs client-side before submitting transactions

### Example Error Handling

```rust
use soroban_sdk::Error;

match contract.try_batch_payout(&recipients, &amounts, &nonce) {
    Ok(data) => {
        // Success
    }
    Err(Error::ContractError(code)) => {
        match code {
            4 => println!("Insufficient balance for payout"),
            8 => println!("Batch too large, reduce number of recipients"),
            16 => println!("Payouts blocked due to open dispute"),
            _ => println!("Error code: {}", code),
        }
    }
    Err(e) => {
        println!("Transaction failed: {:?}", e);
    }
}
```

## Example

```rust
// Initialize
let program_data = contract.init_program(
    &env,
    String::from_str(&env, "stellar-hackathon-2024"),
    backend_address,
    token_address
);

// Lock funds (50,000 XLM in stroops)
contract.lock_program_funds(&env, 50_000_000_000);

// Batch payout to winners
let recipients = vec![&env, winner1, winner2, winner3];
let amounts = vec![&env, 20_000_000_000, 15_000_000_000, 10_000_000_000];
let nonce = contract.get_nonce(&env, backend_address.clone());
contract.batch_payout(&env, recipients, amounts, nonce);

// Check remaining balance
let balance = contract.get_remaining_balance(&env);
```
