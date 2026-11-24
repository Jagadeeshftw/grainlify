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
- Read-only simulation clients (`ViewFacadeClient` and `EscrowViewFacadeClient`) requiring no signing keypair

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

### Read-Only View Facade Client (`ViewFacadeClient`)

`ViewFacadeClient` queries the `view-facade` contract using Soroban RPC simulation only. No signing keypair is required; a throwaway source account is used automatically (configurable via `sourceAccount`).

```typescript
import { ViewFacadeClient, NotFoundError, NetworkError } from '@grainlify/contracts-sdk';

const viewFacade = new ViewFacadeClient({
  contractId: 'CVIEW_FACADE_CONTRACT_ID...',
  rpcUrl: 'https://soroban-testnet.stellar.org',
  networkPassphrase: 'Test SDF Network ; September 2015',
  // Optional: override the throwaway simulation source account
  // sourceAccount: 'G...'
});

// Paginate all registered contracts
const allContracts = await viewFacade.fetchAllContracts(50);

// Or iterate lazily page-by-page
for await (const entry of viewFacade.iterateContracts(25)) {
  console.log(entry.kind, entry.address, entry.version);
}

// Look up a single registered contract
const contract = await viewFacade.getContract('CESCROW_CONTRACT_ID...');

// Cached cross-contract program & fee queries
const [programData, feeConfig] = await viewFacade.queryProgramBalanceAndFee(
  'CPROGRAM_ESCROW_ID...',
  'grant-2026'
);
```

### Read-Only Escrow View Facade Client (`EscrowViewFacadeClient`)

`EscrowViewFacadeClient` provides typed, simulation-only reads against `escrow-view-facade` for wallets and dashboards.

```typescript
import { EscrowViewFacadeClient, NotFoundError, NetworkError } from '@grainlify/contracts-sdk';

const escrowView = new EscrowViewFacadeClient({
  contractId: 'CESCROW_VIEW_FACADE_CONTRACT_ID...',
  rpcUrl: 'https://soroban-testnet.stellar.org',
  networkPassphrase: 'Test SDF Network ; September 2015',
});

// Single escrow summary (returns null if not found)
const summary = await escrowView.getEscrowSummary('CBOUNTY_ESCROW_ID...', 42n);

// Or throw NotFoundError when missing (distinguished from NetworkError on RPC failure)
try {
  const required = await escrowView.getEscrowSummaryOrThrow('CBOUNTY_ESCROW_ID...', 42n);
  console.log(required.amount, required.remaining_amount, required.status);
} catch (error) {
  if (error instanceof NotFoundError) {
    console.error('Escrow does not exist');
  } else if (error instanceof NetworkError) {
    console.error('RPC request failed:', error.message);
  }
}

// Batch-paginated summaries
const summaries = await escrowView.fetchAllEscrowSummaries(
  'CBOUNTY_ESCROW_ID...',
  [1n, 2n, 3n, 4n],
  50
);

// User portfolio (as depositor & beneficiary)
const portfolio = await escrowView.getUserPortfolio('CBOUNTY_ESCROW_ID...', 'GUSER...');

// Program delegates & recipient payout history
const delegates = await escrowView.queryAllDelegates('CPROGRAM_ESCROW_ID...', 'grant-2026');
const history = await escrowView.queryRecipientHistory('CPROGRAM_ESCROW_ID...', 'grant-2026', 'GUSER...');
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
