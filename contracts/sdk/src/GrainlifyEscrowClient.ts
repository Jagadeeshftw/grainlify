import { Keypair } from '@stellar/stellar-sdk';
import {
  Client
} from './bindings_escrow/src/index.ts';
import type {
  Escrow,
  LockFundsItem,
  RefundMode
} from './bindings_escrow/src/index.ts'; // Fixed path

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

  async getEscrow(bountyId: bigint): Promise<Escrow | null> {
    const tx = await this.client.get_escrow_info({ bounty_id: bountyId });
    if (tx.result.isOk()) {
      return tx.result.unwrap();
    }
    return null;
  }

  async refund(signer: Keypair, bountyId: bigint) {
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
