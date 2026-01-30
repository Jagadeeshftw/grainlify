import { Keypair } from '@stellar/stellar-sdk';
import { GrainlifyEscrowClient } from '../src/index.ts';
import type { LockFundsItem } from '../src/bindings_escrow/src/index.ts';

// CONFIGURATION
const ESCROW_ID = "CCTJD4MYSLNLAUDQFCWZOGHIZPNN4NR54CO7RG3XUAAJJLCNU2ENHGLV"; // Replace with your deployed contract ID
const RPC_URL = "https://soroban-testnet.stellar.org";

async function main() {
  console.log("📦 Batch Lock Funds Example");

  // Setup client
  const client = new GrainlifyEscrowClient(ESCROW_ID, RPC_URL);

  // Signer
  const signer = Keypair.fromSecret('SC3K...'); // Replace with actual secret
  console.log(`Signer: ${signer.publicKey()}`);

  // Create batch items
  const items: Array<LockFundsItem> = [
    {
      depositor: signer.publicKey(),
      bounty_id: BigInt(Date.now()),
      amount: BigInt(5_000_000),
      deadline: BigInt(Math.floor(Date.now() / 1000) + 86400)
    },
    {
      depositor: signer.publicKey(),
      bounty_id: BigInt(Date.now() + 1),
      amount: BigInt(10_000_000),
      deadline: BigInt(Math.floor(Date.now() / 1000) + 172800)
    }
  ];

  console.log(`Batch locking funds for ${items.length} bounties...`);

  try {
    const result = await client.batchLock(signer, items);
    console.log("✅ Batch lock successful!");
    console.log("Transaction hash:", result.hash);
  } catch (error: any) {
    console.log("❌ Batch lock failed:", error.message);
  }
}

main().catch(console.error);