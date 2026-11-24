import {
  Address,
  Contract,
  nativeToScVal,
  Keypair,
  nativeToScVal,
  xdr,
} from "@stellar/stellar-sdk";
import {
  NetworkError,
  ValidationError,
  parseContractError,
  ContractError,
} from "./errors";
import {
  SorobanClientConfig,
  SorobanContractClient,
} from "./soroban-contract-client";

export interface ProgramEscrowConfig extends SorobanClientConfig {}
import { ValidationError } from "./errors";
import { BaseClientConfig, BaseContractClient } from "./base-client";

export interface ProgramEscrowConfig extends BaseClientConfig {}

export interface ProgramData {
  program_id: string;
  total_funds: bigint;
  remaining_balance: bigint;
  authorized_payout_key: string;
  payout_history: PayoutRecord[];
  token_address: string;
  risk_flags: number;
  /** Initial liquidity provided by the program creator. */
  initial_liquidity: bigint;
}

export interface PayoutRecord {
  recipient: string;
  amount: bigint;
  timestamp: number;
}

export interface ProgramReleaseSchedule {
  schedule_id: bigint;
  recipient: string;
  amount: bigint;
  release_timestamp: number;
  released: boolean;
  /** Timestamp when the schedule was released (undefined if not yet released). */
  released_at?: number;
  /** Address that triggered the release (undefined if not yet released). */
  released_by?: string;
}

/**
 * Filter options for querying payout history.
 * Maps to the Soroban contract's `PayoutQueryFilter` struct.
 */
export interface PayoutQueryFilter {
  recipient?: string;
  min_amount?: bigint;
  max_amount?: bigint;
  min_timestamp?: number;
  max_timestamp?: number;
}

/**
 * Filter options for querying release schedules.
 * Maps to the Soroban contract's `ScheduleQueryFilter` struct.
 */
export interface ScheduleQueryFilter {
  recipient?: string;
  released?: boolean;
  min_amount?: bigint;
  max_amount?: bigint;
  min_release_timestamp?: number;
  max_release_timestamp?: number;
}

/**
 * Fetches all pages from a paginated contract query by repeatedly calling
 * `fetcher(offset, limit)` until a page shorter than `pageSize` is returned,
 * signalling the last (or only) page.
 *
 * NOTE: Designed for the contract's offset/limit pagination model. If the
 * contract ever adopts cursor-based pagination, update `fetcher` to accept a
 * cursor instead of an offset.
 *
 * Pages are fetched **sequentially** — each awaited before the next — so
 * results are accumulated in order and the RPC endpoint is not flooded.
 *
 * @param fetcher  Async function that accepts (offset, limit) and returns one page.
 * @param pageSize Items per page. Defaults to 50.
 * @returns All items across all pages, concatenated in order.
 */
export async function fetchAllPages<T>(
  fetcher: (offset: number, limit: number) => Promise<T[]>,
  pageSize: number = 50,
): Promise<T[]> {
  const all: T[] = [];
  let offset = 0;
  while (true) {
    const page = await fetcher(offset, pageSize);
    all.push(...page);
    if (page.length < pageSize) {
      break; // partial or empty page — we have reached the end
    }
    offset += pageSize;
  }
  return all;
}

const READ_ONLY_METHODS = new Set(["get_program_info", "get_remaining_balance"]);

/**
 * Client for interacting with the ProgramEscrow Soroban contract
 */
export class ProgramEscrowClient extends SorobanContractClient {
  constructor(config: ProgramEscrowConfig) {
    super(config);
export class ProgramEscrowClient extends BaseContractClient {
  constructor(config: ProgramEscrowConfig) {
    super(config);
  }

  protected get contractErrorScope(): string {
    return "program_escrow";
  }

  protected isReadOnlyMethod(method: string): boolean {
    return READ_ONLY_METHODS.has(method);
  }

  /**
   * Initialize a new program escrow
   */
  async initProgram(
    programId: string,
    authorizedPayoutKey: string,
    tokenAddress: string,
    sourceKeypair: Keypair,
    initialLiquidity: bigint = 0n,
  ): Promise<ProgramData> {
    if (!programId || programId.trim().length === 0) {
      throw new ValidationError("Program ID cannot be empty", "programId");
    }

    this.validateAddress(authorizedPayoutKey, "authorizedPayoutKey");
    this.validateContractAddress(tokenAddress, "tokenAddress");

    try {
      const result = await this.invokeContract(
        "init_program",
        [programId, authorizedPayoutKey, tokenAddress, initialLiquidity],
        sourceKeypair,
      );
      return this.parseProgramData(result);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Lock funds into the program escrow
   */
  async lockProgramFunds(
    amount: bigint,
    sourceKeypair: Keypair,
  ): Promise<ProgramData> {
    if (amount <= 0n) {
      throw new ValidationError("Amount must be greater than zero", "amount");
    }

    try {
      const result = await this.invokeContract(
        "lock_program_funds",
        [amount],
        sourceKeypair,
      );
      return this.parseProgramData(result);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /** Publish a newly initialized program so payouts and releases are enabled. */
  async publishProgram(
    programId: string,
    caller: string,
    sourceKeypair: Keypair,
  ): Promise<ProgramData> {
    this.validateAddress(caller, "caller");
    try {
      const result = await this.invokeContract(
        "publish_program",
        [programId, caller],
        sourceKeypair,
      );
      return this.parseProgramData(result);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /** Transfer tokens to the escrow before calling lockProgramFunds. */
  async fundContract(amount: bigint, sourceKeypair: Keypair): Promise<void> {
    if (amount <= 0n) {
      throw new ValidationError("Amount must be greater than zero", "amount");
    }
    const program = await this.getProgramInfo();
    try {
      await this.invokeContract(
        "transfer",
        [sourceKeypair.publicKey(), this.config.contractId, amount],
        sourceKeypair,
        new Contract(program.token_address),
      );
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Execute batch payouts to multiple recipients
   */
  async batchPayout(
    recipients: string[],
    amounts: bigint[],
    sourceKeypair: Keypair,
  ): Promise<ProgramData> {
    if (recipients.length === 0) {
      throw new ValidationError(
        "Recipients array cannot be empty",
        "recipients",
      );
    }

    if (recipients.length !== amounts.length) {
      throw new ValidationError(
        "Recipients and amounts arrays must have the same length",
        "recipients",
      );
    }

    for (let i = 0; i < amounts.length; i++) {
      if (amounts[i] <= 0n) {
        throw new ValidationError(
          `Amount at index ${i} must be greater than zero`,
          "amounts",
        );
      }
    }

    for (let i = 0; i < recipients.length; i++) {
      this.validateAddress(recipients[i], `recipients[${i}]`);
    }

    try {
      const result = await this.invokeContract(
        "batch_payout",
        [recipients, amounts],
        sourceKeypair,
      );
      return this.parseProgramData(result);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Execute a single payout
   */
  async singlePayout(
    recipient: string,
    amount: bigint,
    sourceKeypair: Keypair,
  ): Promise<ProgramData> {
    this.validateAddress(recipient, "recipient");

    if (amount <= 0n) {
      throw new ValidationError("Amount must be greater than zero", "amount");
    }

    try {
      const result = await this.invokeContract(
        "single_payout",
        [recipient, amount],
        sourceKeypair,
      );
      return this.parseProgramData(result);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Get program information
   */
  async getProgramInfo(): Promise<ProgramData> {
    try {
      const result = await this.invokeContract("get_program_info", []);
      return this.parseProgramData(result);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Get remaining balance
   */
  async getRemainingBalance(): Promise<bigint> {
    try {
      const result = await this.invokeContract("get_remaining_balance", []);
      if (
        typeof result !== "string" &&
        typeof result !== "number" &&
        typeof result !== "bigint"
      ) {
        throw new Error("Soroban simulation returned an invalid balance");
      }
      return BigInt(result);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Create a release schedule
   */
  async createProgramReleaseSchedule(
    recipient: string,
    amount: bigint,
    releaseTimestamp: number,
    sourceKeypair: Keypair,
  ): Promise<ProgramReleaseSchedule> {
    this.validateAddress(recipient, "recipient");

    if (amount <= 0n) {
      throw new ValidationError("Amount must be greater than zero", "amount");
    }

    try {
      const result = await this.invokeContract(
        "create_program_release_schedule",
        [recipient, amount, releaseTimestamp],
        sourceKeypair,
      );
      return this.parseReleaseSchedule(result);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Trigger program releases
   */
  async triggerProgramReleases(sourceKeypair: Keypair): Promise<number> {
    try {
      const result = await this.invokeContract(
        "trigger_program_releases",
        [null],
        sourceKeypair,
      );
      return Number(result);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Query payout history with optional filtering and pagination.
   * Wraps the contract's `query_payout_history(filter, offset, limit)` function.
   * The filter object is shallow-copied before forwarding — it is never mutated.
   */
  async queryPayoutHistory(
    filter: PayoutQueryFilter,
    offset: number,
    limit: number,
  ): Promise<PayoutRecord[]> {
    try {
      const result = await this.invokeContract("query_payout_history", [
        { ...filter },
        offset,
        limit,
      ]);
      return result as PayoutRecord[];
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Query release schedules with optional filtering and pagination.
   * Wraps the contract's `query_release_schedules(filter, offset, limit)` function.
   * The filter object is shallow-copied before forwarding — it is never mutated.
   */
  async queryReleaseSchedules(
    filter: ScheduleQueryFilter,
    offset: number,
    limit: number,
  ): Promise<ProgramReleaseSchedule[]> {
    try {
      const result = await this.invokeContract("query_release_schedules", [
        { ...filter },
        offset,
        limit,
      ]);
      return result as ProgramReleaseSchedule[];
    } catch (error) {
      throw this.handleError(error);
    }
  }

  private validateAddress(address: string, fieldName: string): void {
    if (!address || address.trim().length === 0) {
      throw new ValidationError(`${fieldName} cannot be empty`, fieldName);
    }
    // Basic Stellar address validation (starts with G and is 56 chars)
    if (!address.match(/^G[A-Z0-9]{55}$/)) {
      throw new ValidationError(
        `${fieldName} is not a valid Stellar address`,
        fieldName,
      );
    }
  }

  private validateContractAddress(address: string, fieldName: string): void {
    if (!address || !/^C[A-Z0-9]{55}$/.test(address)) {
      throw new ValidationError(
        `${fieldName} is not a valid Stellar contract address`,
        fieldName,
      );
    }
  }

  protected encodeArguments(method: string, args: unknown[], signer: Keypair): xdr.ScVal[] {
    const values = args as any[];
  protected encodeArguments(
    method: string,
    args: any[],
    signer: Keypair,
  ): xdr.ScVal[] {
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
        throw new Error(`Soroban simulation failed for ${method}: ${simulation.error}`);
      }

      const readOnlyMethods = new Set([
        "get_program_info",
        "get_remaining_balance",
      ]);
      if (readOnlyMethods.has(method)) {
        if (!simulation.result) {
          throw new Error(`Soroban simulation returned no result for ${method}`);
        }
        return scValToNative(simulation.result.retval);
      }

      const prepared = SorobanRpc.assembleTransaction(transaction, simulation).build();
      prepared.sign(signer);
      const submitted = await this.server.sendTransaction(prepared);
      if (submitted.status === "ERROR") {
        const code = transactionResultCode(submitted.errorResult);
        throw new Error(
          `Soroban submission failed for ${method}${code ? ` (${code})` : ""}`,
        );
      }

      const deadline = Date.now() + 30_000;
      while (Date.now() < deadline) {
        const result = await this.server.getTransaction(submitted.hash);
        if (result.status === SorobanRpc.Api.GetTransactionStatus.SUCCESS) {
          return result.returnValue ? scValToNative(result.returnValue) : undefined;
        }
        if (result.status === SorobanRpc.Api.GetTransactionStatus.FAILED) {
          const code = transactionResultCode(result.resultXdr);
          throw new Error(
            `Soroban transaction failed for ${method}${code ? ` (${code})` : ""}`,
          );
        }
        await new Promise((resolve) => setTimeout(resolve, 1000));
      }
      throw new NetworkError(`Timed out waiting for Soroban transaction ${method}`);
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

  private encodeArguments(method: string, args: any[], signer: Keypair): xdr.ScVal[] {
    const address = (value: string) => Address.fromString(value).toScVal();
    const i128 = (value: bigint) => nativeToScVal(value, { type: "i128" });
    const string = (value: string) => nativeToScVal(value, { type: "string" });
    const vector = (values: xdr.ScVal[]) => xdr.ScVal.scvVec(values);

    switch (method) {
      case "init_program":
        return [
          string(values[0]),
          address(values[1]),
          address(values[2]),
          address(signer.publicKey()),
          values[3] > 0n ? i128(values[3]) : xdr.ScVal.scvVoid(),
          xdr.ScVal.scvVoid(),
        ];
      case "lock_program_funds":
        return [i128(values[0])];
      case "batch_payout":
        return [
          vector(values[0].map((value: string) => address(value))),
          vector(values[1].map((value: bigint) => i128(value))),
        ];
      case "single_payout":
        return [address(values[0]), i128(values[1])];
      case "publish_program":
        return [string(values[0]), address(values[1])];
      case "transfer":
        return [address(values[0]), address(values[1]), i128(values[2])];
      case "get_program_info":
      case "get_remaining_balance":
        return [];
      case "trigger_program_releases":
        return [xdr.ScVal.scvVoid()];
      case "create_program_release_schedule":
        return [
          address(values[0]),
          i128(values[1]),
          nativeToScVal(BigInt(values[2]), { type: "u64" }),
        ];
      default:
        throw new Error(`No Soroban argument encoder is defined for ${method}`);
    }
  }

  protected isReadOnlyMethod(method: string): boolean {
    return method === "get_program_info" || method === "get_remaining_balance";
  }

  private handleError(error: any): Error {
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

    // Try to parse as contract error
    return parseContractError(error);
  }

  private parseProgramData(result: any): ProgramData {
    return this.toPlainObject(result) as ProgramData;
  }

  private parseReleaseSchedule(result: any): ProgramReleaseSchedule {
    return this.toPlainObject(result) as ProgramReleaseSchedule;
  }
}

/**
 * Extract the result code from a failed transaction so failures such as
 * `txBadSeq` are visible in the thrown error message (the SDK examples retry
 * on that code).
 */
function transactionResultCode(
  result: xdr.TransactionResult | undefined,
): string | undefined {
  if (!result) return undefined;
  try {
    return result.result().switch().name;
  } catch {
    return undefined;
  }
}
