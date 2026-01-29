import { Keypair } from '@stellar/stellar-sdk';
import { GrainlifyEscrowClient } from '../src/index.ts';

// CONFIGURATION
const ESCROW_ID = "CCTJD4MYSLNLAUDQFCWZOGHIZPNN4NR54CO7RG3XUAAJJLCNU2ENHGLV"; // Replace with your deployed contract ID
const RPC_URL = "https://soroban-testnet.stellar.org";

async function main() {
  console.log("🔒 Locking Funds Example");

  // Setup client
  const client = new GrainlifyEscrowClient(ESCROW_ID, RPC_URL);

  // Create a signer (in real app, load from secure storage)
  const signer = Keypair.random();
  console.log(`Signer: ${signer.publicKey()}`);

  // Bounty details
  const bountyId = BigInt(Date.now());
  const amount = BigInt(10_000_000); // 1 XLM
  const deadline = BigInt(Math.floor(Date.now() / 1000) + 86400); // 24 hours from now

  console.log(`Locking ${amount} stroops for bounty ${bountyId}...`);

  try {
    const result = await client.lockFunds(signer, bountyId, amount, deadline);
    console.log("✅ Funds locked successfully!");
    console.log("Transaction hash:", result.hash);
  } catch (error: any) {
    console.log("❌ Failed to lock funds:", error.message);
    console.log("Note: This may fail if the account has insufficient funds or the contract is not deployed.");
  }
}

main().catch(console.error);