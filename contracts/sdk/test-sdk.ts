import { Keypair } from '@stellar/stellar-sdk';
import { GrainlifyEscrowClient } from './src/index.ts';

// CONFIGURATION
// Replace with the Escrow Contract ID you deployed earlier
const ESCROW_ID = "CCTJD4MYSLNLAUDQFCWZOGHIZPNN4NR54CO7RG3XUAAJJLCNU2ENHGLV"; 
const RPC_URL = "https://soroban-testnet.stellar.org";

async function main() {
  // 1. Setup
  const client = new GrainlifyEscrowClient(ESCROW_ID, RPC_URL);
  
  // 2. Create a dummy user (Alice)
  // In a real app, you would load this from a secret key
  const alice = Keypair.random(); 
  console.log(`User: ${alice.publicKey()}`);

  // 3. Define Bounty Details
  const bountyId = BigInt(Date.now()); // Unique ID
  const amount = BigInt(100_000_000); // 10 XLM
  const deadline = BigInt(Math.floor(Date.now() / 1000) + 86400); // Tomorrow

  console.log(`Attempting to lock funds for Bounty #${bountyId}...`);

  try {
    // 4. Call the SDK
    // Note: This will fail in this script because Alice has 0 XLM, 
    // but it verifies the SDK constructs the transaction correctly.
    await client.lockFunds(alice, bountyId, amount, deadline);
    console.log("Success!");
  } catch (e: any) {
    console.log("SDK Interaction attempted!");
    console.log("Result:", e.message || "Transaction Failed (Expected due to no funds)");
  }
}

main();