import { Keypair } from '@stellar/stellar-sdk';
import {
  Client,
  Escrow,
  LockFundsItem,
  ReleaseFundsItem,
  RefundMode
} from './bindings_escrow';

export class GrainlifyEscrowClient {
  public client: Client;

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
   * LOCK FUNDS
   * Deposits funds into the escrow for a specific bounty.
   * @param signer - The user depositing funds (must sign tx)
   * @param bountyId - Unique ID for the task
   * @param amount - Amount in stroops (1 XLM = 10,000,000 stroops)
   * @param deadline - Unix timestamp for refund eligibility
   */
  async lockFunds(
    signer: Keypair,
    bountyId: bigint,
    amount: bigint,
    deadline: bigint
  ) {
    const tx = await this.client.lock_funds(
      {
        depositor: signer.publicKey(),
        bounty_id: bountyId,
        amount: amount,
        deadline: deadline
      },
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
   * RELEASE FUNDS
   * Admin releases funds to the contributor.
   */
  async releaseFunds(
    adminSigner: Keypair,
    bountyId: bigint,
    contributorAddress: string
  ) {
    const tx = await this.client.release_funds(
      { bounty_id: bountyId, contributor: contributorAddress },
      {
        publicKey: adminSigner.publicKey(),
        signTransaction: async (txn) => {
          txn.sign(adminSigner);
          return txn;
        }
      }
    );
    return tx.signAndSend();
  }

  /**
   * BATCH LOCK
   * Efficiently lock funds for multiple bounties at once.
   */
  async batchLock(signer: Keypair, items: Array<LockFundsItem>) {
    const tx = await this.client.batch_lock_funds(
      { items },
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
   * GET INFO
   * Read the status of an escrow without sending a transaction.
   */
  async getEscrow(bountyId: bigint): Promise<Escrow | null> {
    const tx = await this.client.get_escrow_info({ bounty_id: bountyId });
    // The result is wrapped in a Result type from the bindings
    if (tx.result.isOk()) {
      return tx.result.unwrap();
    }
    return null;
  }

  /**
   * REFUND
   * Trigger a refund if the deadline has passed.
   */
  async refund(signer: Keypair, bountyId: bigint) {
    // Mode "Full" is represented as { tag: "Full", values: undefined } in bindings
    const fullRefund: RefundMode = { tag: "Full", values: undefined };
    
    const tx = await this.client.refund(
      { 
        bounty_id: bountyId, 
        amount: undefined, 
        recipient: undefined, 
        mode: fullRefund 
      },
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
}