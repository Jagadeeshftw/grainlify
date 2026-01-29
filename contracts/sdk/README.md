# Grainlify Contract Interaction SDK

A comprehensive TypeScript/JavaScript SDK for interacting with Grainlify Soroban smart contracts from off-chain applications.

## Features

- TypeScript bindings generated from contract ABIs
- Client classes for Grainlify Core and Escrow contracts
- Support for common workflows: locking funds, releasing funds, batch operations, and querying escrow status
- Error handling utilities
- Examples for integration

## Installation

```bash
cd contracts/sdk
npm install
```

## Setup

1. Deploy your contracts to Soroban testnet or mainnet.
2. Note the contract IDs.
3. Configure your RPC URL and network passphrase.

## Usage

### GrainlifyCoreClient

```typescript
import { GrainlifyCoreClient } from './src/index.ts';

const coreClient = new GrainlifyCoreClient(
  'CORE_CONTRACT_ID',
  'https://soroban-testnet.stellar.org',
  'Test SDF Network ; September 2015'
);

// Get health status
const health = await coreClient.getHealth();
console.log(health);

// Get version
const version = await coreClient.getVersion();
console.log(version);

// Get analytics
const analytics = await coreClient.getAnalytics();
console.log(analytics);
```

### GrainlifyEscrowClient

```typescript
import { GrainlifyEscrowClient } from './src/index.ts';
import { Keypair } from '@stellar/stellar-sdk';

const escrowClient = new GrainlifyEscrowClient(
  'ESCROW_CONTRACT_ID',
  'https://soroban-testnet.stellar.org',
  'Test SDF Network ; September 2015'
);

const signer = Keypair.fromSecret('YOUR_SECRET_KEY');

// Lock funds
await escrowClient.lockFunds(signer, BigInt(123), BigInt(10000000), BigInt(Date.now() + 86400));

// Release funds
await escrowClient.releaseFunds(signer, BigInt(123), 'CONTRIBUTOR_ADDRESS');

// Query escrow
const escrow = await escrowClient.getEscrow(BigInt(123));
if (escrow) {
  console.log(escrow);
}
```

## API Reference

### GrainlifyCoreClient

- `constructor(contractId: string, rpcUrl?: string, networkPassphrase?: string)`
- `getHealth(): Promise<HealthStatus>`
- `getVersion(): Promise<number>`
- `proposeUpgrade(signer: Keypair, newWasmHash: Buffer, proposerAddress: string): Promise<any>`
- `getAnalytics(): Promise<Analytics>`

### GrainlifyEscrowClient

- `constructor(contractId: string, rpcUrl?: string, networkPassphrase?: string)`
- `lockFunds(signer: Keypair, bountyId: bigint, amount: bigint, deadline: bigint): Promise<any>`
- `releaseFunds(adminSigner: Keypair, bountyId: bigint, contributorAddress: string): Promise<any>`
- `batchLock(signer: Keypair, items: Array<LockFundsItem>): Promise<any>`
- `getEscrow(bountyId: bigint): Promise<Escrow | null>`
- `refund(signer: Keypair, bountyId: bigint): Promise<any>`

## Error Handling

The SDK uses Soroban's Result type. Check `tx.result.isOk()` before unwrapping.

```typescript
import { handleTransactionResult, logError } from './src/index.ts';

try {
  const tx = await client.someMethod();
  const result = handleTransactionResult(tx);
} catch (error) {
  logError(error);
}
```

## Examples

See the `examples/` directory for detailed scripts:

- `lock-funds.ts`: Locking funds for a bounty
- `release-funds.ts`: Releasing funds to a contributor
- `batch-lock.ts`: Batch locking funds
- `query-escrow.ts`: Querying escrow information
- `full-lifecycle.ts`: Complete bounty workflow

Run examples with:

```bash
npx ts-node-esm examples/lock-funds.ts
```

## Testing

Run the test script:

```bash
npx ts-node-esm test-sdk.ts
```

## Contributing

1. Generate bindings after contract changes.
2. Update examples and documentation.
3. Test all methods.