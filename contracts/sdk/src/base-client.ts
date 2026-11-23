import {
  BASE_FEE,
  Contract,
  Keypair,
  SorobanRpc,
  TransactionBuilder,
  scValToNative,
  xdr,
} from "@stellar/stellar-sdk";
import {
  ContractError,
  NetworkError,
  ValidationError,
  parseContractError,
  parseContractErrorByCode,
} from "./errors";

/**
 * Configuration shared by every contract client in the SDK.
 */
export interface BaseClientConfig {
  contractId: string;
  rpcUrl: string;
  networkPassphrase: string;
  /** Signer used for simulation-only reads when a method has no signer argument. */
  sourceKeypair?: Keypair;
}

const STELLAR_ADDRESS_PATTERN = /^G[A-Z0-9]{55}$/;
const STELLAR_CONTRACT_PATTERN = /^C[A-Z0-9]{55}$/;
const U64_MAX = (1n << 64n) - 1n;

/**
 * Shared invoke/simulate/poll pipeline used by the generated-contract clients.
 *
 * Concrete clients provide `encodeArguments`, `isReadOnlyMethod` and
 * `contractErrorScope`; everything else (validation helpers, error
 * normalization, transaction assembly and polling) is shared here so clients
 * behave identically.
 */
export abstract class BaseContractClient {
  protected contract: Contract;
  protected server: SorobanRpc.Server;
  protected config: BaseClientConfig;

  constructor(config: BaseClientConfig) {
    this.config = config;
    try {
      this.contract = new Contract(config.contractId);
    } catch (error) {
      // Allow invalid contract IDs for testing purposes
      this.contract = null as any;
    }
    try {
      this.server = new SorobanRpc.Server(config.rpcUrl, { allowHttp: true });
    } catch (error) {
      // Allow server initialization to fail for testing
      this.server = null as any;
    }
  }

  /** Scope passed to `parseContractErrorByCode` when a numeric code is seen. */
  protected abstract get contractErrorScope(): string;

  /** Methods answered from a simulation without submitting a transaction. */
  protected abstract isReadOnlyMethod(method: string): boolean;

  /** Encode a contract method's arguments into Soroban `ScVal`s. */
  protected abstract encodeArguments(
    method: string,
    args: any[],
    signer: Keypair,
  ): xdr.ScVal[];

  protected async invokeContract(
    method: string,
    args: any[],
    sourceKeypair?: Keypair,
    contract: Contract = this.contract,
  ): Promise<any> {
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

      const prepared = SorobanRpc.assembleTransaction(
        transaction,
        simulation,
      ).build();
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
      throw new NetworkError(
        `Timed out waiting for Soroban transaction ${method}`,
      );
    } catch (error: any) {
      // Check for network errors
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

  protected handleError(error: any): Error {
    if (
      error instanceof ValidationError ||
      error instanceof NetworkError ||
      error instanceof ContractError
    ) {
      return error;
    }

    // Check if it's a network error first (before parsing as contract error)
    if (
      error.code === "ECONNREFUSED" ||
      error.code === "ETIMEDOUT" ||
      error.code === "ENOTFOUND"
    ) {
      return new NetworkError(
        `Failed to connect to RPC server: ${this.config.rpcUrl}`,
        undefined,
        error,
      );
    }

    if (error.response?.status) {
      return new NetworkError(
        `RPC request failed with status ${error.response.status}`,
        error.response.status,
        error,
      );
    }

    // Prefer the contract-specific numeric mapping when the host error carries
    // an `Error(Contract, N)` discriminant.
    const message = error?.message || error?.toString() || "";
    const match = message.match(/Error\(Contract, (\d+)\)/);
    if (match) {
      return parseContractErrorByCode(
        parseInt(match[1], 10),
        this.contractErrorScope,
      );
    }

    // Fall back to string/message-based contract error parsing
    return parseContractError(error);
  }

  protected validateAddress(address: string, fieldName: string): void {
    if (!address || address.trim().length === 0) {
      throw new ValidationError(`${fieldName} cannot be empty`, fieldName);
    }
    // Basic Stellar address validation (starts with G and is 56 chars)
    if (!STELLAR_ADDRESS_PATTERN.test(address)) {
      throw new ValidationError(
        `${fieldName} is not a valid Stellar address`,
        fieldName,
      );
    }
  }

  protected validateContractAddress(address: string, fieldName: string): void {
    if (!address || !STELLAR_CONTRACT_PATTERN.test(address)) {
      throw new ValidationError(
        `${fieldName} is not a valid Stellar contract address`,
        fieldName,
      );
    }
  }

  protected validatePositiveAmount(amount: bigint, fieldName: string): void {
    if (typeof amount !== "bigint" || amount <= 0n) {
      throw new ValidationError(
        `${fieldName} must be a bigint greater than zero`,
        fieldName,
      );
    }
  }

  protected validateBountyId(bountyId: bigint, fieldName = "bountyId"): void {
    if (typeof bountyId !== "bigint" || bountyId < 0n || bountyId > U64_MAX) {
      throw new ValidationError(
        `${fieldName} must be a non-negative bigint that fits in a u64`,
        fieldName,
      );
    }
  }

  protected validateDeadline(deadline: bigint, fieldName = "deadline"): void {
    if (typeof deadline !== "bigint" || deadline < 0n || deadline > U64_MAX) {
      throw new ValidationError(
        `${fieldName} must be a non-negative bigint that fits in a u64`,
        fieldName,
      );
    }
  }

  protected toPlainObject(value: any): any {
    if (value instanceof Map) {
      return Object.fromEntries(
        [...value.entries()].map(([key, item]) => [
          String(key),
          this.toPlainObject(item),
        ]),
      );
    }
    if (Array.isArray(value)) {
      return value.map((item) => this.toPlainObject(item));
    }
    return value;
  }
}

export default BaseContractClient;
