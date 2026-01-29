import { Keypair } from '@stellar/stellar-sdk';
import { 
  Client
} from './bindings/src/index.ts';
import type { HealthStatus, Analytics } from './bindings/src/index.ts'; // Fixed path

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

  async getHealth(): Promise<HealthStatus> {
    const tx = await this.client.health_check();
    return tx.result;
  }

  async getVersion(): Promise<number> {
    const tx = await this.client.get_version();
    return tx.result;
  }

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

  async getAnalytics(): Promise<Analytics> {
    const tx = await this.client.get_analytics();
    return tx.result;
  }
}
