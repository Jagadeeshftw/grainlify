import {
  BASE_FEE,
  Contract,
  scValToNative,
  SorobanRpc,
  TransactionBuilder,
  Keypair,
} from "@stellar/stellar-sdk";
import {
  ContractSpec,
  PROGRAM_ESCROW_SPEC,
  SEP41_TOKEN_SPEC,
} from "./contract-spec";
import {
  NetworkError,
  ValidationError,
  parseContractError,
  ContractError,
} from "./errors";

export interface ProgramEscrowConfig {
  contractId: string;
  rpcUrl: string;
  networkPassphrase: string;
  /** Signer used for simulation-only reads when a method has no signer argument. */
  sourceKeypair?: Keypair;
  /** Optional custom contract spec. Defaults to `PROGRAM_ESCROW_SPEC`. */
  spec?: ContractSpec;
}

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

/**
 * Client for interacting with the ProgramEscrow Soroban contract
 */
export class ProgramEscrowClient {
  private contract: Contract;
  private server: SorobanRpc.Server;
  private config: ProgramEscrowConfig;
  private spec: ContractSpec;

  constructor(config: ProgramEscrowConfig) {
    this.config = config;
    this.spec = config.spec ?? PROGRAM_ESCROW_SPEC;
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

  /**
   * Returns the `ContractSpec` used by this client for argument encoding
   * and read-only method detection.
   */
  getSpec(): ContractSpec {
    return this.spec;
  }

  /**
   * Generic spec-driven contract invocation.
   *
   * Encodes `namedArgs` using the contract spec. Read-only entrypoints
   * (as declared in the spec) are simulated and their decoded return value
   * is returned without submitting a transaction. State-changing entrypoints
   * are simulated, assembled, signed with `sourceKeypair` (or
   * `config.sourceKeypair`), submitted, and polled until finalized.
   *
   * Throws `ValidationError` listing allowed names if `method` is unknown
   * or if `namedArgs` contains unknown or missing required argument names.
   */
  async invoke<T = any>(
    method: string,
    namedArgs: Record<string, unknown> = {},
    sourceKeypair?: Keypair,
  ): Promise<T> {
    try {
      const result = await this.invokeContract(
        method,
        namedArgs,
        sourceKeypair,
      );
      return result as T;
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Generic spec-driven simulation (read-only dry run).
   *
   * Encodes `namedArgs` using the contract spec and simulates the call
   * without signing or submitting a transaction, even for state-changing
   * entrypoints.
   *
   * Throws `ValidationError` listing allowed names if `method` is unknown
   * or if `namedArgs` contains unknown or missing required argument names.
   */
  async simulate<T = any>(
    method: string,
    namedArgs: Record<string, unknown> = {},
    sourceKeypair?: Keypair,
  ): Promise<T> {
    try {
      const result = await this.invokeContract(
        method,
        namedArgs,
        sourceKeypair,
        this.contract,
        this.spec,
        true,
      );
      return result as T;
    } catch (error) {
      throw this.handleError(error);
    }
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
        {
          program_id: programId,
          authorized_payout_key: authorizedPayoutKey,
          token_address: tokenAddress,
          creator: sourceKeypair.publicKey(),
          initial_liquidity: initialLiquidity > 0n ? initialLiquidity : null,
          reference_hash: null,
        },
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
        { amount },
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
        { program_id: programId, caller },
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
        {
          from: sourceKeypair.publicKey(),
          to: this.config.contractId,
          amount,
        },
        sourceKeypair,
        new Contract(program.token_address),
        SEP41_TOKEN_SPEC,
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
        { recipients, amounts },
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
        { recipient, amount },
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
      const result = await this.invokeContract("get_program_info", {});
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
      const result = await this.invokeContract("get_remaining_balance", {});
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
        {
          recipient,
          amount,
          release_timestamp: releaseTimestamp,
        },
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
        { epoch_id: null },
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

  private async invokeContract(
    method: string,
    args: Record<string, unknown> | unknown[],
    sourceKeypair?: Keypair,
    contract: Contract = this.contract,
    spec: ContractSpec = this.spec,
    forceSimulate: boolean = false,
  ): Promise<any> {
    try {
      const namedArgs = Array.isArray(args)
        ? this.positionalToNamedArgs(method, args, spec)
        : args;

      // Validate and encode arguments via the contract spec before touching RPC.
      const encodedArgs = spec.encodeArgs(method, namedArgs);

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
        .addOperation(contract.call(method, ...encodedArgs))
        .setTimeout(30)
        .build();

      const simulation = await this.server.simulateTransaction(transaction);
      if (SorobanRpc.Api.isSimulationError(simulation)) {
        throw new Error(`Soroban simulation failed for ${method}: ${simulation.error}`);
      }

      if (forceSimulate || spec.isReadOnly(method)) {
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
          return result.returnValue ? scValToNative(result.returnValue) : undefined;
        }
        if (result.status === SorobanRpc.Api.GetTransactionStatus.FAILED) {
          throw new Error(`Soroban transaction failed for ${method}`);
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

  private positionalToNamedArgs(
    method: string,
    args: unknown[],
    spec: ContractSpec,
  ): Record<string, unknown> {
    const ep = spec.getMethod(method);
    const named: Record<string, unknown> = {};
    ep.parameters.forEach((param, idx) => {
      if (idx < args.length) {
        named[param.name] = args[idx];
      }
    });
    return named;
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

  private toPlainObject(value: any): any {
    if (value instanceof Map) {
      return Object.fromEntries(
        [...value.entries()].map(([key, item]) => [String(key), this.toPlainObject(item)]),
      );
    }
    if (Array.isArray(value)) return value.map((item) => this.toPlainObject(item));
    return value;
  }
}
