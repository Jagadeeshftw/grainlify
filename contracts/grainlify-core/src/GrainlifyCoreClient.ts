import { Keypair } from '@stellar/stellar-sdk';
import { 
  Client, 
  HealthStatus, 
  Analytics, 
  StateSnapshot 
} from './bindings';

export class GrainlifyCoreClient {
  private client: Client;

  constructor(
    contractId: string, 
    rpcUrl = "https://soroban-testnet.stellar.org", 
    networkPassphrase = "Test SDF Network ; September 2015"
  ) {
    this.client = new Client({
      contractId,
      rpcUrl,
      networkPassphrase,
    });
  }

  /**
   * Checks if the contract is healthy and running.
   */
  async getHealth(): Promise<HealthStatus> {
    const tx = await this.client.health_check();
    return tx.result;
  }

  /**
   * Gets the current contract version.
   */
  async getVersion(): Promise<number> {
    const tx = await this.client.get_version();
    return tx.result;
  }

  /**
   * PROPOSAL WORKFLOW
   * Proposes a new WASM hash for upgrade.
   */
  async proposeUpgrade(signer: Keypair, newWasmHash: Buffer, proposerAddress: string) {
    const tx = await this.client.propose_upgrade(
      { proposer: proposerAddress, wasm_hash: newWasmHash },
      { 
        publicKey: signer.publicKey(), 
        signTransaction: async (txn) => {
          txn.sign(signer);
          return txn;
        }
      }
    );
    return tx.signAndSend();
  }

  /**
   * ANALYTICS
   * Fetches current usage stats.
   */
  async getAnalytics(): Promise<Analytics> {
    const tx = await this.client.get_analytics();
    return tx.result;
  }
}