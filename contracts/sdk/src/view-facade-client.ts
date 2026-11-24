import {
  Account,
  Address,
  BASE_FEE,
  Contract,
  nativeToScVal,
  scValToNative,
  SorobanRpc,
  TransactionBuilder,
  xdr,
} from "@stellar/stellar-sdk";
import {
  ContractError,
  NetworkError,
  NotFoundError,
  ValidationError,
  parseContractError,
} from "./errors";
import {
  fetchAllPages,
  type PayoutRecord,
  type ProgramData,
} from "./program-escrow-client";

/**
 * Default throwaway Stellar public key used as the source account for
 * read-only Soroban simulations when no custom `sourceAccount` is configured.
 *
 * Simulation never requires signing or an on-chain sequence lookup; using a
 * local `Account(publicKey, "0")` follows the Stellar SDK read-only pattern.
 */
export const DEFAULT_SIMULATION_SOURCE_ACCOUNT =
  "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";

/**
 * Configuration for read-only Soroban facade clients.
 * Never requires a signing keypair.
 */
export interface ReadOnlyClientConfig {
  /** Deployed facade contract ID (`C...`). */
  contractId: string;
  /** Soroban RPC endpoint URL. */
  rpcUrl: string;
  /** Stellar network passphrase. */
  networkPassphrase: string;
  /**
   * Optional Stellar account public key (`G...`) used as the throwaway source
   * account for transaction simulation. Defaults to
   * {@link DEFAULT_SIMULATION_SOURCE_ACCOUNT}.
   */
  sourceAccount?: string;
  /**
   * Optional pre-configured `SorobanRpc.Server` instance (useful for testing
   * or sharing a connection pool).
   */
  server?: SorobanRpc.Server;
}

export type ViewFacadeConfig = ReadOnlyClientConfig;

/**
 * Mirrors `ContractKind` in `contracts/view-facade/src/types.rs`.
 */
export enum ContractKind {
  BountyEscrow = "BountyEscrow",
  ProgramEscrow = "ProgramEscrow",
  SorobanEscrow = "SorobanEscrow",
  GrainlifyCore = "GrainlifyCore",
}

/**
 * Mirrors `RegisteredContract` in `contracts/view-facade/src/types.rs`.
 */
export interface RegisteredContract {
  address: string;
  kind: ContractKind;
  version: number;
}

/**
 * Mirrors `FeeConfig` in `contracts/program-escrow/src/types.rs`, returned by
 * `query_fee_config_cached` and `query_program_balance_and_fee`.
 */
export interface FeeConfig {
  lock_fee_rate: bigint;
  payout_fee_rate: bigint;
  lock_fixed_fee: bigint;
  payout_fixed_fee: bigint;
  fee_recipient: string;
  fee_enabled: boolean;
  fee_waivers: number;
  insurance_reserve_bps: number;
}

/**
 * Aggregated tuple returned by `query_program_balance_and_fee`.
 */
export type ProgramBalanceAndFee = [ProgramData, FeeConfig];

// ---------------------------------------------------------------------------
// Shared internal helpers for read-only facade clients
// ---------------------------------------------------------------------------

/** @internal */
export function toPlainObject(value: any): any {
  if (value instanceof Map) {
    return Object.fromEntries(
      [...value.entries()].map(([k, v]) => [String(k), toPlainObject(v)]),
    );
  }
  if (Array.isArray(value)) {
    return value.map((item) => toPlainObject(item));
  }
  return value;
}

/** @internal */
export function decodeEnumVariant<T extends string>(
  raw: any,
  allowed: readonly T[],
  typeName: string,
): T {
  let tag: unknown = raw;
  if (Array.isArray(raw) && raw.length > 0) {
    tag = raw[0];
  } else if (raw && typeof raw === "object" && "tag" in raw) {
    tag = (raw as { tag: unknown }).tag;
  }
  if (typeof tag === "string" && (allowed as readonly string[]).includes(tag)) {
    return tag as T;
  }
  throw new ValidationError(
    `Unknown ${typeName} variant: ${String(tag)}`,
    typeName,
  );
}

/** @internal */
export function decodePayoutRecord(raw: any): PayoutRecord {
  const obj = toPlainObject(raw);
  return {
    recipient: String(obj.recipient),
    amount: BigInt(obj.amount),
    timestamp: Number(obj.timestamp),
  };
}

/** @internal */
export function decodeProgramData(raw: any): ProgramData {
  const obj = toPlainObject(raw);
  return {
    program_id: String(obj.program_id),
    total_funds: BigInt(obj.total_funds),
    remaining_balance: BigInt(obj.remaining_balance),
    authorized_payout_key: String(obj.authorized_payout_key),
    payout_history: Array.isArray(obj.payout_history)
      ? obj.payout_history.map(decodePayoutRecord)
      : [],
    token_address: String(obj.token_address),
    risk_flags: Number(obj.risk_flags ?? 0),
    initial_liquidity: BigInt(obj.initial_liquidity ?? 0n),
  };
}

/** @internal */
export function decodeFeeConfig(raw: any): FeeConfig {
  const obj = toPlainObject(raw);
  return {
    lock_fee_rate: BigInt(obj.lock_fee_rate),
    payout_fee_rate: BigInt(obj.payout_fee_rate),
    lock_fixed_fee: BigInt(obj.lock_fixed_fee),
    payout_fixed_fee: BigInt(obj.payout_fixed_fee),
    fee_recipient: String(obj.fee_recipient),
    fee_enabled: Boolean(obj.fee_enabled),
    fee_waivers: Number(obj.fee_waivers ?? 0),
    insurance_reserve_bps: Number(obj.insurance_reserve_bps ?? 0),
  };
}

/** @internal */
export function decodeProgramBalanceAndFee(raw: any): ProgramBalanceAndFee {
  const arr = toPlainObject(raw);
  if (!Array.isArray(arr) || arr.length < 2) {
    throw new ValidationError(
      "Invalid response for query_program_balance_and_fee: expected [ProgramData, FeeConfig] tuple",
      "result",
    );
  }
  return [decodeProgramData(arr[0]), decodeFeeConfig(arr[1])];
}

const NOT_FOUND_PATTERNS = [
  /not[\s_-]?found/i,
  /Error\(Contract,\s*#202\)/i,
  /ProgramNotFound/i,
  /BountyNotFound/i,
  /ContractNotFound/i,
  /does not exist/i,
];

/** @internal */
export function isNotFoundMessage(message: string): boolean {
  return NOT_FOUND_PATTERNS.some((pattern) => pattern.test(message));
}

/** @internal */
export function classifyReadError(
  error: any,
  rpcUrl: string,
  resource?: string,
  identifier?: string,
): Error {
  if (
    error instanceof ValidationError ||
    error instanceof NotFoundError ||
    error instanceof NetworkError ||
    error instanceof ContractError
  ) {
    return error;
  }

  if (
    error?.code === "ECONNREFUSED" ||
    error?.code === "ETIMEDOUT" ||
    error?.code === "ENOTFOUND" ||
    error?.code === "ECONNRESET" ||
    error?.code === "EAI_AGAIN"
  ) {
    return new NetworkError(
      `Failed to connect to RPC server: ${rpcUrl}`,
      undefined,
      error,
    );
  }

  if (error?.response?.status) {
    return new NetworkError(
      `RPC request failed with status ${error.response.status}`,
      error.response.status,
      error,
    );
  }

  const message =
    typeof error === "string"
      ? error
      : typeof error?.message === "string"
        ? error.message
        : String(error ?? "Unknown error");

  if (isNotFoundMessage(message)) {
    return new NotFoundError(message, resource, identifier);
  }

  if (
    /Error\(Contract,\s*#\d+\)/i.test(message) ||
    /HostError/i.test(message)
  ) {
    return parseContractError(error);
  }

  return new NetworkError(
    `RPC simulation failed: ${message}`,
    undefined,
    error instanceof Error ? error : undefined,
  );
}

/** @internal */
export async function simulateReadOnly(
  server: SorobanRpc.Server,
  contract: Contract,
  networkPassphrase: string,
  sourceAccountPublicKey: string,
  rpcUrl: string,
  method: string,
  args: xdr.ScVal[],
  resource?: string,
  identifier?: string,
): Promise<any> {
  try {
    const sourceAccount = new Account(sourceAccountPublicKey, "0");
    const transaction = new TransactionBuilder(sourceAccount, {
      fee: BASE_FEE,
      networkPassphrase,
    })
      .addOperation(contract.call(method, ...args))
      .setTimeout(30)
      .build();

    const simulation = await server.simulateTransaction(transaction);

    if (SorobanRpc.Api.isSimulationError(simulation)) {
      const simError = simulation.error || `Simulation failed for ${method}`;
      if (isNotFoundMessage(simError)) {
        throw new NotFoundError(simError, resource, identifier);
      }
      if (/Error\(Contract,\s*#\d+\)/i.test(simError)) {
        throw parseContractError(simError);
      }
      throw new NetworkError(`Soroban simulation failed for ${method}: ${simError}`);
    }

    if (!simulation.result) {
      throw new NetworkError(`Soroban simulation returned no result for ${method}`);
    }

    return scValToNative(simulation.result.retval);
  } catch (error: any) {
    throw classifyReadError(error, rpcUrl, resource, identifier);
  }
}

/** @internal */
export function validateStellarAddress(
  address: string,
  fieldName: string,
): void {
  if (!address || address.trim().length === 0) {
    throw new ValidationError(`${fieldName} cannot be empty`, fieldName);
  }
  if (!/^[GC][A-Z0-9]{55}$/.test(address)) {
    throw new ValidationError(
      `${fieldName} is not a valid Stellar address`,
      fieldName,
    );
  }
}

/** @internal */
export function validateContractId(address: string, fieldName: string): void {
  if (!address || !/^C[A-Z0-9]{55}$/.test(address)) {
    throw new ValidationError(
      `${fieldName} is not a valid Stellar contract address`,
      fieldName,
    );
  }
}

const CONTRACT_KINDS: readonly ContractKind[] = [
  ContractKind.BountyEscrow,
  ContractKind.ProgramEscrow,
  ContractKind.SorobanEscrow,
  ContractKind.GrainlifyCore,
];

function decodeRegisteredContract(raw: any): RegisteredContract {
  const obj = toPlainObject(raw);
  return {
    address: String(obj.address),
    kind: decodeEnumVariant(obj.kind, CONTRACT_KINDS, "ContractKind"),
    version: Number(obj.version),
  };
}

/**
 * Read-only TypeScript SDK client for the `view-facade` Soroban contract.
 *
 * All methods use Soroban RPC transaction simulation with a throwaway source
 * account and never require a signing keypair.
 */
export class ViewFacadeClient {
  private readonly contract: Contract;
  private readonly server: SorobanRpc.Server;
  private readonly config: ViewFacadeConfig;
  private readonly sourceAccount: string;

  constructor(config: ViewFacadeConfig) {
    this.config = config;
    this.sourceAccount =
      config.sourceAccount ?? DEFAULT_SIMULATION_SOURCE_ACCOUNT;
    try {
      this.contract = new Contract(config.contractId);
    } catch {
      this.contract = null as any;
    }
    if (config.server) {
      this.server = config.server;
    } else {
      try {
        this.server = new SorobanRpc.Server(config.rpcUrl, { allowHttp: true });
      } catch {
        this.server = null as any;
      }
    }
  }

  /**
   * Return the administrator address, or `null` if the facade has not yet been
   * initialized.
   */
  async getAdmin(): Promise<string | null> {
    const result = await this.simulate("get_admin", [], "admin");
    if (result === null || result === undefined) {
      return null;
    }
    return String(result);
  }

  /**
   * Look up a registered contract by its on-chain address.
   * Returns `null` if the contract is not registered.
   */
  async getContract(address: string): Promise<RegisteredContract | null> {
    validateStellarAddress(address, "address");
    const result = await this.simulate(
      "get_contract",
      [Address.fromString(address).toScVal()],
      "contract",
      address,
    );
    if (result === null || result === undefined) {
      return null;
    }
    return decodeRegisteredContract(result);
  }

  /**
   * Look up a registered contract by address, throwing {@link NotFoundError}
   * if it is not registered.
   */
  async getContractOrThrow(address: string): Promise<RegisteredContract> {
    const entry = await this.getContract(address);
    if (!entry) {
      throw new NotFoundError(
        `Registered contract not found: ${address}`,
        "contract",
        address,
      );
    }
    return entry;
  }

  /**
   * Return a paginated slice of registered contracts.
   *
   * @param offset Optional number of entries to skip (defaults to 0 on-chain).
   * @param limit  Optional maximum number of entries to return.
   */
  async listContracts(
    offset?: number,
    limit?: number,
  ): Promise<RegisteredContract[]> {
    if (offset !== undefined && (!Number.isInteger(offset) || offset < 0)) {
      throw new ValidationError(
        "offset must be a non-negative integer",
        "offset",
      );
    }
    if (limit !== undefined && (!Number.isInteger(limit) || limit <= 0)) {
      throw new ValidationError("limit must be a positive integer", "limit");
    }

    const args: xdr.ScVal[] = [
      offset !== undefined
        ? nativeToScVal(offset, { type: "u32" })
        : xdr.ScVal.scvVoid(),
      limit !== undefined
        ? nativeToScVal(limit, { type: "u32" })
        : xdr.ScVal.scvVoid(),
    ];

    const result = await this.simulate("list_contracts", args, "contracts");
    if (!Array.isArray(result)) {
      return [];
    }
    return result.map(decodeRegisteredContract);
  }

  /**
   * Return all registered contracts in a single call (legacy entrypoint).
   */
  async listContractsAll(): Promise<RegisteredContract[]> {
    const result = await this.simulate("list_contracts_all", [], "contracts");
    if (!Array.isArray(result)) {
      return [];
    }
    return result.map(decodeRegisteredContract);
  }

  /**
   * Return the total number of registered contracts.
   */
  async contractCount(): Promise<number> {
    const result = await this.simulate("contract_count", [], "contracts");
    return Number(result);
  }

  /**
   * Fetch all registered contracts across all pages using {@link fetchAllPages}.
   */
  async fetchAllContracts(pageSize: number = 50): Promise<RegisteredContract[]> {
    if (!Number.isInteger(pageSize) || pageSize <= 0) {
      throw new ValidationError(
        "pageSize must be a positive integer",
        "pageSize",
      );
    }
    return fetchAllPages(
      (offset, limit) => this.listContracts(offset, limit),
      pageSize,
    );
  }

  /**
   * Async generator that lazily iterates over all registered contracts page by
   * page.
   */
  async *iterateContracts(
    pageSize: number = 50,
  ): AsyncGenerator<RegisteredContract, void, unknown> {
    if (!Number.isInteger(pageSize) || pageSize <= 0) {
      throw new ValidationError(
        "pageSize must be a positive integer",
        "pageSize",
      );
    }
    let offset = 0;
    while (true) {
      const page = await this.listContracts(offset, pageSize);
      for (const entry of page) {
        yield entry;
      }
      if (page.length < pageSize) {
        break;
      }
      offset += pageSize;
    }
  }

  /**
   * Query all payout records for `recipient` within `programId` via a
   * registered `ProgramEscrow` contract.
   */
  async queryRecipientHistory(
    escrow: string,
    programId: string,
    recipient: string,
  ): Promise<PayoutRecord[]> {
    validateContractId(escrow, "escrow");
    if (!programId || programId.trim().length === 0) {
      throw new ValidationError("programId cannot be empty", "programId");
    }
    validateStellarAddress(recipient, "recipient");

    const result = await this.simulate(
      "query_recipient_history",
      [
        Address.fromString(escrow).toScVal(),
        nativeToScVal(programId, { type: "string" }),
        Address.fromString(recipient).toScVal(),
      ],
      "recipient_history",
      `${programId}:${recipient}`,
    );

    if (!Array.isArray(result)) {
      return [];
    }
    return result.map(decodePayoutRecord);
  }

  /**
   * Fetch `ProgramData` for `programId` on `escrow` using the facade's
   * per-invocation query cache.
   */
  async queryProgramDataCached(
    escrow: string,
    programId: string,
  ): Promise<ProgramData> {
    validateContractId(escrow, "escrow");
    if (!programId || programId.trim().length === 0) {
      throw new ValidationError("programId cannot be empty", "programId");
    }

    const result = await this.simulate(
      "query_program_data_cached",
      [
        Address.fromString(escrow).toScVal(),
        nativeToScVal(programId, { type: "string" }),
      ],
      "program",
      programId,
    );
    return decodeProgramData(result);
  }

  /**
   * Fetch `FeeConfig` for `escrow` using the facade's per-invocation query
   * cache.
   */
  async queryFeeConfigCached(escrow: string): Promise<FeeConfig> {
    validateContractId(escrow, "escrow");

    const result = await this.simulate(
      "query_fee_config_cached",
      [Address.fromString(escrow).toScVal()],
      "fee_config",
      escrow,
    );
    return decodeFeeConfig(result);
  }

  /**
   * Aggregated query returning both `ProgramData` and `FeeConfig` in a single
   * simulated call.
   */
  async queryProgramBalanceAndFee(
    escrow: string,
    programId: string,
  ): Promise<ProgramBalanceAndFee> {
    validateContractId(escrow, "escrow");
    if (!programId || programId.trim().length === 0) {
      throw new ValidationError("programId cannot be empty", "programId");
    }

    const result = await this.simulate(
      "query_program_balance_and_fee",
      [
        Address.fromString(escrow).toScVal(),
        nativeToScVal(programId, { type: "string" }),
      ],
      "program",
      programId,
    );
    return decodeProgramBalanceAndFee(result);
  }

  private async simulate(
    method: string,
    args: xdr.ScVal[],
    resource?: string,
    identifier?: string,
  ): Promise<any> {
    return simulateReadOnly(
      this.server,
      this.contract,
      this.config.networkPassphrase,
      this.sourceAccount,
      this.config.rpcUrl,
      method,
      args,
      resource,
      identifier,
    );
  }
}
