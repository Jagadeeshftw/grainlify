import {
  BASE_FEE,
  Contract,
  Keypair,
  SorobanRpc,
  TransactionBuilder,
  scValToNative,
  xdr,
} from "@stellar/stellar-sdk";
import { NetworkError, ValidationError } from "./errors";

export interface SorobanClientConfig {
  contractId: string;
  rpcUrl: string;
  networkPassphrase: string;
  sourceKeypair?: Keypair;
}

export abstract class SorobanContractClient {
  protected readonly contract: Contract;
  protected readonly server: SorobanRpc.Server;
  protected readonly config: SorobanClientConfig;

  protected constructor(config: SorobanClientConfig) {
    this.config = config;
    try {
      this.contract = new Contract(config.contractId);
    } catch {
      this.contract = null as unknown as Contract;
    }
    try {
      this.server = new SorobanRpc.Server(config.rpcUrl, { allowHttp: true });
    } catch {
      this.server = null as unknown as SorobanRpc.Server;
    }
  }

  protected abstract encodeArguments(
    method: string,
    args: unknown[],
    signer: Keypair,
  ): xdr.ScVal[];

  protected abstract isReadOnlyMethod(method: string): boolean;

  protected async invokeContract(
    method: string,
    args: unknown[],
    sourceKeypair?: Keypair,
    contract: Contract = this.contract,
  ): Promise<unknown> {
    try {
      const signer = sourceKeypair ?? this.config.sourceKeypair;
      if (!signer) {
        throw new ValidationError(
          "A sourceKeypair is required to simulate a Soroban invocation",
          "sourceKeypair",
        );
      }

      const account = await this.server.getAccount(signer.publicKey());
      const transaction = new TransactionBuilder(account, {
        fee: BASE_FEE,
        networkPassphrase: this.config.networkPassphrase,
      })
        .addOperation(
          contract.call(method, ...this.encodeArguments(method, args, signer)),
        )
        .setTimeout(30)
        .build();

      const simulation = await this.server.simulateTransaction(transaction);
      if (SorobanRpc.Api.isSimulationError(simulation)) {
        throw new Error(
          `Soroban simulation failed for ${method}: ${simulation.error}`,
        );
      }

      if (this.isReadOnlyMethod(method)) {
        if (!simulation.result) {
          throw new Error(`Soroban simulation returned no result for ${method}`);
        }
        return scValToNative(simulation.result.retval);
      }

      const prepared = SorobanRpc.assembleTransaction(transaction, simulation).build();
      prepared.sign(signer);
      const submitted = await this.server.sendTransaction(prepared);
      if (submitted.status === "ERROR") {
        throw new Error(`Soroban submission failed for ${method}`);
      }

      const deadline = Date.now() + 30_000;
      while (Date.now() < deadline) {
        const result = await this.server.getTransaction(submitted.hash);
        if (result.status === SorobanRpc.Api.GetTransactionStatus.SUCCESS) {
          return result.returnValue
            ? scValToNative(result.returnValue)
            : undefined;
        }
        if (result.status === SorobanRpc.Api.GetTransactionStatus.FAILED) {
          throw new Error(`Soroban transaction failed for ${method}`);
        }
        await new Promise((resolve) => setTimeout(resolve, 1000));
      }
      throw new NetworkError(`Timed out waiting for Soroban transaction ${method}`);
    } catch (error: any) {
      if (error.code === "ECONNREFUSED" || error.code === "ETIMEDOUT") {
        throw new NetworkError(
          `Failed to connect to RPC server: ${this.config.rpcUrl}`,
          undefined,
          error,
        );
      }

      if (error.response?.status) {
        throw new NetworkError(
          `RPC request failed with status ${error.response.status}`,
          error.response.status,
          error,
        );
      }

      throw error;
    }
  }
}
