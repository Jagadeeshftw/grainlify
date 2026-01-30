import { Keypair } from '@stellar/stellar-sdk';
import { GrainlifyEscrowClient } from '../src/index.ts';

// CONFIGURATION
const ESCROW_ID = "CCTJD4MYSLNLAUDQFCWZOGHIZPNN4NR54CO7RG3XUAAJJLCNU2ENHGLV"; // Replace with your deployed contract ID
const RPC_URL = "https://soroban-testnet.stellar.org";

async function main() {
  console.log("🔄 Full Bounty Lifecycle Example");

  // Setup
  const client = new GrainlifyEscrowClient(ESCROW_ID, RPC_URL);
  const depositor = Keypair.fromSecret('SC3K...'); // Replace with depositor's secret
  const admin = Keypair.fromSecret('SC3K...'); // Replace with admin's secret

  const bountyId = BigInt(Date.now());
  const amount = BigInt(10_000_000);
  const deadline = BigInt(Math.floor(Date.now() / 1000) + 86400);

  console.log(`Starting lifecycle for bounty ${bountyId}`);

  try {
    // 1. Lock funds
    console.log("1. Locking funds...");
    await client.lockFunds(depositor, bountyId, amount, deadline);
    console.log("✅ Funds locked");

    // 2. Query escrow
    console.log("2. Querying escrow...");
    const escrow = await client.getEscrow(bountyId);
    console.log("✅ Escrow status:", escrow ? "Found" : "Not found");

    // 3. Release funds (simulate contributor work done)
    console.log("3. Releasing funds...");
    await client.releaseFunds(admin, bountyId, depositor.publicKey());
    console.log("✅ Funds released");

    console.log("🎉 Lifecycle complete!");
  } catch (error: any) {
    console.log("❌ Lifecycle failed:", error.message);
  }
}

main().catch(console.error);