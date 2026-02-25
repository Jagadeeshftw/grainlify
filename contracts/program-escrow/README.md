# Program Escrow Contract

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

## Usage Flow

1. **Initialize Program**: Call `init_program()` with program ID, authorized key, and token address
2. **Lock Funds**: Call `lock_program_funds()` to deposit funds (can be called multiple times)
3. **Execute Payouts**: Call `single_payout()` or `batch_payout()` to distribute funds
4. **Replay Safety**: Read `get_nonce(signer)` and pass that nonce to payout entrypoints
4. **Monitor**: Use `get_program_info()` or `get_remaining_balance()` to check status

## Security Considerations

- Only the `authorized_payout_key` can trigger payouts
- Balance validation prevents over-spending
- All amounts must be positive
- Payout history is immutable and auditable
- Token transfers use the Soroban token contract standard
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
