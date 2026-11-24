# Grainlify Contracts SDK

TypeScript SDK for interacting with Grainlify Soroban smart contracts on the Stellar network.

## Installation

```bash
npm install @grainlify/contracts-sdk
```

## Features

- Type-safe contract interactions
- Comprehensive error handling with typed errors
- Network error detection and reporting
- Input validation
- Support for all ProgramEscrow contract functions
- Governance and timelocked grainlify-core upgrades
- Typed BountyEscrow client for the core bounty lifecycle

## Usage

### Initialize the Client

```typescript
import { ProgramEscrowClient } from '@grainlify/contracts-sdk';

const client = new ProgramEscrowClient({
  contractId: 'YOUR_CONTRACT_ID',
  rpcUrl: 'https://soroban-testnet.stellar.org',
  networkPassphrase: 'Test SDF Network ; September 2015'
});
```

### Initialize a Program

```typescript
import { Keypair } from '@stellar/stellar-sdk';

const sourceKeypair = Keypair.fromSecret('YOUR_SECRET_KEY');

try {
  const programData = await client.initProgram(
    'my-program-id',
    'GAUTHORIZED_PAYOUT_KEY...',
    'GTOKEN_ADDRESS...',
    sourceKeypair
  );
  console.log('Program initialized:', programData);
} catch (error) {
  if (error instanceof ValidationError) {
    console.error('Invalid input:', error.message);
  } else if (error instanceof ContractError) {
    console.error('Contract error:', error.code, error.message);
  } else if (error instanceof NetworkError) {
    console.error('Network error:', error.statusCode, error.message);
  }
}
```

### Lock Funds

```typescript
try {
  const programData = await client.lockProgramFunds(
    10000000n, // Amount in stroops
    sourceKeypair
  );
  console.log('Funds locked. Remaining balance:', programData.remaining_balance);
} catch (error) {
  // Handle errors
}
```

### Execute Batch Payout

```typescript
const recipients = [
  'GRECIPIENT1...',
  'GRECIPIENT2...',
  'GRECIPIENT3...'
];

const amounts = [
  1000000n,
  2000000n,
  1500000n
];

try {
  const programData = await client.batchPayout(
    recipients,
    amounts,
    sourceKeypair
  );
  console.log('Batch payout completed');
} catch (error) {
  // Handle errors
}
```

### Get Program Info

```typescript
try {
  const programData = await client.getProgramInfo();
  console.log('Program ID:', programData.program_id);
  console.log('Total funds:', programData.total_funds);
  console.log('Remaining balance:', programData.remaining_balance);
} catch (error) {
  // Handle errors
}
```

### Govern grainlify-core upgrades

`GovernanceClient` can initialize governance, propose and approve upgrades,
cancel or execute proposals, inspect proposal/timelock status, and commit and
run version migrations. Upgrade hashes accept exactly 32-byte `Buffer` values
or 64-character hexadecimal strings (optionally prefixed with `0x`).

```typescript
import { GovernanceClient } from '@grainlify/contracts-sdk';
import { Keypair } from '@stellar/stellar-sdk';

const governance = new GovernanceClient({
  contractId: 'YOUR_GRAINLIFY_CORE_CONTRACT_ID',
  rpcUrl: 'https://soroban-testnet.stellar.org',
  networkPassphrase: 'Test SDF Network ; September 2015',
  sourceKeypair: Keypair.fromSecret('YOUR_SECRET_KEY')
});

const proposalId = await governance.proposeUpgrade(
  proposerAddress,
  '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef',
  0n, // 0 means no proposal expiry
  sourceKeypair
);
await governance.approveUpgrade(proposalId, signerAddress, sourceKeypair);

const proposal = await governance.getUpgradeProposal(proposalId);
const ready = await governance.canExecute(proposalId);
if (proposal && ready) {
  await governance.executeUpgrade(proposalId, sourceKeypair);
}
```

`canExecute` checks that the proposal exists and is not cancelled, that the
on-chain timelock has elapsed, and that the proposal has not reached its expiry
ledger timestamp. Read methods use the configured `sourceKeypair` for Soroban
simulation; alternatively, provide one as the final argument to each write
method.
### Bounty Escrow

```typescript
import { BountyEscrowClient } from '@grainlify/contracts-sdk';

const bountyClient = new BountyEscrowClient({
  contractId: 'YOUR_BOUNTY_ESCROW_CONTRACT_ID',
  rpcUrl: 'https://soroban-testnet.stellar.org',
  networkPassphrase: 'Test SDF Network ; September 2015',
});
```

#### Lock funds

```typescript
await bountyClient.lockFunds(
  depositorAddress, // G... depositor
  42n, // bounty id (u64)
  10_000_000n, // amount in token base units (i128, must be > 0)
  1_800_000_000n, // refund deadline (u64, unix seconds)
  sourceKeypair,
);
```

#### Batch lock funds

```typescript
const created = await bountyClient.batchLockFunds(
  [
    {
      bounty_id: 43n,
      depositor: depositorAddress,
      amount: 5_000_000n,
      deadline: 1_800_000_000n,
    },
    {
      bounty_id: 44n,
      depositor: depositorAddress,
      amount: 7_500_000n,
      deadline: 1_800_000_000n,
    },
  ],
  sourceKeypair,
);
console.log('Escrows created:', created);
```

#### Release and refund

```typescript
// Release the full remaining balance to a contributor
await bountyClient.releaseFunds(42n, contributorAddress, sourceKeypair);

// Or release part of it
await bountyClient.partialRelease(42n, contributorAddress, 2_500_000n, sourceKeypair);

// Refund the depositor
await bountyClient.refund(42n, sourceKeypair);

// Admin-approved refund before the deadline
await bountyClient.approveRefund(42n, 10_000_000n, depositorAddress, 'Full', adminKeypair);
```

#### Read escrow state (simulation only, no transaction submitted)

```typescript
const escrow = await bountyClient.getEscrowInfo(42n);
console.log(escrow.status, escrow.remaining_amount);

const balance = await bountyClient.getBalance();
const eligibility = await bountyClient.getRefundEligibility(42n);
const flags = await bountyClient.getPauseFlags();
```

#### Pause operations

```typescript
await bountyClient.setPaused(
  { lock: true, refund: true, reason: 'maintenance' },
  adminKeypair,
);
```

## Error Handling

The SDK provides three main error types:

### ValidationError

Thrown when input parameters are invalid before making a contract call.

```typescript
import { ValidationError } from '@grainlify/contracts-sdk';

try {
  await client.lockProgramFunds(0n, keypair); // Invalid: amount must be > 0
} catch (error) {
  if (error instanceof ValidationError) {
    console.error('Field:', error.field);
    console.error('Message:', error.message);
  }
}
```

### ContractError

Thrown when the smart contract returns an error.

```typescript
import { ContractError, ContractErrorCode } from '@grainlify/contracts-sdk';

try {
  await client.singlePayout(recipient, amount, keypair);
} catch (error) {
  if (error instanceof ContractError) {
    switch (error.code) {
      case ContractErrorCode.NOT_INITIALIZED:
        console.error('Program not initialized');
        break;
      case ContractErrorCode.UNAUTHORIZED:
        console.error('Unauthorized access');
        break;
      case ContractErrorCode.INSUFFICIENT_BALANCE:
        console.error('Insufficient balance');
        break;
      // ... handle other cases
    }
  }
}
```

### NetworkError

Thrown when there are network or transport issues.

```typescript
import { NetworkError } from '@grainlify/contracts-sdk';

try {
  await client.getProgramInfo();
} catch (error) {
  if (error instanceof NetworkError) {
    console.error('Status code:', error.statusCode);
    console.error('Cause:', error.cause);
    // Implement retry logic
  }
}
```

## Contract Error Codes

- `NOT_INITIALIZED` - Program not initialized
- `UNAUTHORIZED` - Caller does not have permission
- `INSUFFICIENT_BALANCE` - Insufficient balance for operation
- `INVALID_AMOUNT` - Amount must be greater than zero
- `ALREADY_INITIALIZED` - Program already initialized
- `EMPTY_BATCH` - Cannot process empty batch
- `LENGTH_MISMATCH` - Recipients and amounts arrays must match
- `OVERFLOW` - Payout amount overflow

## Testing

Run the test suite:

```bash
npm test
```

Run tests with coverage:

```bash
npm run test:coverage
```

## End-to-End Examples (Local Quickstart)

`src/__tests__/examples.e2e.test.ts` exercises the modules in `examples/` against a real Soroban deployment. It is skipped unless every `GRAINLIFY_SDK_E2E_*` variable is set, or `GRAINLIFY_SDK_E2E_REQUIRED=1` is set.

The repository script starts an ephemeral [Stellar Quickstart](https://github.com/stellar/quickstart) container, waits for Soroban RPC and friendbot, builds and deploys `program_escrow.wasm`, funds a throwaway key, and runs the e2e test:

```bash
# from the repository root
bash scripts/sdk-e2e-local.sh
```

The default image is pinned to `stellar/quickstart:v695-b1655.1-latest`; override it with `QUICKSTART_IMAGE=stellar/quickstart:<tag>`. Avoid the moving `latest` tag when you need reproducible runs. Prerequisites are Docker, Rust with the `wasm32-unknown-unknown` target, Node.js 20 with the SDK dependencies installed (`cd contracts/sdk && npm ci`), and Stellar CLI 23.0.1 (`cargo install --locked stellar-cli --version 23.0.1`).

The container is stopped when the script exits. To keep it running for repeated test runs, use `KEEP_QUICKSTART=1`; the script then leaves the generated environment in `/tmp/grainlify-sdk-e2e.env`:

```bash
KEEP_QUICKSTART=1 bash scripts/sdk-e2e-local.sh
source /tmp/grainlify-sdk-e2e.env
cd contracts/sdk && npm test -- --runInBand src/__tests__/examples.e2e.test.ts
```

The e2e test generates a unique program id for every attempt and makes up to three attempts when the network reports `txBadSeq`, which can happen while ledger state catches up after rapid transactions. Readiness is checked by `scripts/wait-for-quickstart.sh`, which polls the RPC `getHealth` method and the friendbot endpoint and prints the container log tail on timeout.

## Error Handling Tests

The SDK includes comprehensive error handling tests covering:

- Input validation errors
- Contract error parsing and mapping
- Network and transport errors
- HTTP status code handling
- Error type hierarchy and properties
- Error recovery scenarios

See `src/__tests__/error-handling.test.ts` and `src/__tests__/network-errors.test.ts` for detailed test cases.

## Development

Build the SDK:

```bash
npm run build
```

Watch mode for tests:

```bash
npm run test:watch
```

## License

MIT
