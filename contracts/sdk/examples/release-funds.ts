import { Keypair } from '@stellar/stellar-sdk';
import { GrainlifyEscrowClient } from '../src/index.ts';

// CONFIGURATION
const ESCROW_ID = "CCTJD4MYSLNLAUDQFCWZOGHIZPNN4NR54CO7RG3XUAAJJLCNU2ENHGLV"; // Replace with your deployed contract ID
const RPC_URL = "https://soroban-testnet.stellar.org";

async function main() {
  console.log("💰 Releasing Funds Example");

  // Setup client
  const client = new GrainlifyEscrowClient(ESCROW_ID, RPC_URL);

  // Admin signer (must be authorized to release funds)
  const adminSigner = Keypair.fromSecret('SC3K...'); // Replace with actual secret
  console.log(`Admin: ${adminSigner.publicKey()}`);

  // Bounty and contributor details
  const bountyId = BigInt(123456789); // Replace with actual bounty ID
  const contributorAddress = 'GABC...'; // Replace with contributor's public key

  console.log(`Releasing funds for bounty ${bountyId} to ${contributorAddress}...`);

  try {
    const result = await client.releaseFunds(adminSigner, bountyId, contributorAddress);
    console.log("✅ Funds released successfully!");
    console.log("Transaction hash:", result.hash);
  } catch (error: any) {
    console.log("❌ Failed to release funds:", error.message);
  }
}

main().catch(console.error);