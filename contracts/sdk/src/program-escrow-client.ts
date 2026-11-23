import {
  Address,
  Contract,
  Keypair,
  nativeToScVal,
  xdr,
} from "@stellar/stellar-sdk";
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

  protected encodeArguments(
    method: string,
    args: any[],
    signer: Keypair,
  ): xdr.ScVal[] {
    const address = (value: string) => Address.fromString(value).toScVal();
    const i128 = (value: bigint) => nativeToScVal(value, { type: "i128" });
    const string = (value: string) => nativeToScVal(value, { type: "string" });
    const vector = (values: xdr.ScVal[]) => xdr.ScVal.scvVec(values);

    switch (method) {
      case "init_program":
        return [
          string(args[0]),
          address(args[1]),
          address(args[2]),
          address(signer.publicKey()),
          args[3] > 0n ? i128(args[3]) : xdr.ScVal.scvVoid(),
          xdr.ScVal.scvVoid(),
        ];
      case "lock_program_funds":
        return [i128(args[0])];
      case "batch_payout":
        return [
          vector(args[0].map((value: string) => address(value))),
          vector(args[1].map((value: bigint) => i128(value))),
        ];
      case "single_payout":
        return [address(args[0]), i128(args[1])];
      case "publish_program":
        return [string(args[0]), address(args[1])];
      case "transfer":
        return [address(args[0]), address(args[1]), i128(args[2])];
      case "get_program_info":
      case "get_remaining_balance":
        return [];
      case "trigger_program_releases":
        return [xdr.ScVal.scvVoid()];
      case "create_program_release_schedule":
        return [
          address(args[0]),
          i128(args[1]),
          nativeToScVal(BigInt(args[2]), { type: "u64" }),
        ];
      default:
        throw new Error(`No Soroban argument encoder is defined for ${method}`);
    }
  }

  private parseProgramData(result: any): ProgramData {
    return this.toPlainObject(result) as ProgramData;
  }

  private parseReleaseSchedule(result: any): ProgramReleaseSchedule {
    return this.toPlainObject(result) as ProgramReleaseSchedule;
  }
}
