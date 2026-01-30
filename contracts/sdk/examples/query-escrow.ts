import { GrainlifyEscrowClient } from '../src/index.ts';

// CONFIGURATION
const ESCROW_ID = "CCTJD4MYSLNLAUDQFCWZOGHIZPNN4NR54CO7RG3XUAAJJLCNU2ENHGLV"; // Replace with your deployed contract ID
const RPC_URL = "https://soroban-testnet.stellar.org";

async function main() {
  console.log("🔍 Query Escrow Status Example");

  // Setup client
  const client = new GrainlifyEscrowClient(ESCROW_ID, RPC_URL);

  // Bounty ID to query
  const bountyId = BigInt(123456789); // Replace with actual bounty ID

  console.log(`Querying escrow for bounty ${bountyId}...`);

  try {
    const escrow = await client.getEscrow(bountyId);
    if (escrow) {
      console.log("✅ Escrow found:");
      console.log(JSON.stringify(escrow, null, 2));
    } else {
      console.log("❌ No escrow found for this bounty ID");
    }
  } catch (error: any) {
    console.log("❌ Query failed:", error.message);
  }
}

main().catch(console.error);