import {
  Address,
  Contract,
  nativeToScVal,
  SorobanRpc,
  xdr,
} from "@stellar/stellar-sdk";
import { NotFoundError, ValidationError } from "./errors";
import {
  fetchAllPages,
  type PayoutRecord,
  type ProgramData,
} from "./program-escrow-client";
import {
  DEFAULT_SIMULATION_SOURCE_ACCOUNT,
  decodeEnumVariant,
  decodeFeeConfig,
  decodePayoutRecord,
  decodeProgramBalanceAndFee,
  decodeProgramData,
  simulateReadOnly,
  toPlainObject,
  validateContractId,
  validateStellarAddress,
  type FeeConfig,
  type ProgramBalanceAndFee,
  type ReadOnlyClientConfig,
} from "./view-facade-client";

export type EscrowViewFacadeConfig = ReadOnlyClientConfig;

/**
 * Mirrors `EscrowStatus` in `contracts/escrow-view-facade/src/types.rs`.
 */
export enum EscrowStatus {
  Locked = "Locked",
  Released = "Released",
  Refunded = "Refunded",
  PartiallyRefunded = "PartiallyRefunded",
}

/**
 * Mirrors `EscrowSummary` in `contracts/escrow-view-facade/src/types.rs`.
 * Uses `bigint` for `i128` (`amount`, `remaining_amount`) and `u64` fields.
 */
export interface EscrowSummary {
  bounty_id: bigint;
  depositor: string;
  amount: bigint;
  remaining_amount: bigint;
  status: EscrowStatus;
  deadline: bigint;
  repo_id: bigint;
  issue_id: bigint;
  bounty_type: string;
  is_paused: boolean;
}

/**
 * Mirrors `UserPortfolio` in `contracts/escrow-view-facade/src/types.rs`.
 */
export interface UserPortfolio {
  /** Escrows funded by this user. */
  as_depositor: EscrowSummary[];
  /** Escrows where this user is the designated beneficiary/contributor. */
  as_beneficiary: EscrowSummary[];
}

/**
 * Mirrors `ProgramDelegateInfo` in `contracts/program-escrow/src/types.rs`.
 */
export interface ProgramDelegateInfo {
  program_id: string;
  delegate: string | null;
  permissions: number;
}

const ESCROW_STATUSES: readonly EscrowStatus[] = [
  EscrowStatus.Locked,
  EscrowStatus.Released,
  EscrowStatus.Refunded,
  EscrowStatus.PartiallyRefunded,
];

function decodeEscrowSummary(raw: any): EscrowSummary {
  const obj = toPlainObject(raw);
  return {
    bounty_id: BigInt(obj.bounty_id),
    depositor: String(obj.depositor),
    amount: BigInt(obj.amount),
    remaining_amount: BigInt(obj.remaining_amount),
    status: decodeEnumVariant(obj.status, ESCROW_STATUSES, "EscrowStatus"),
    deadline: BigInt(obj.deadline),
    repo_id: BigInt(obj.repo_id),
    issue_id: BigInt(obj.issue_id),
    bounty_type: String(obj.bounty_type ?? ""),
    is_paused: Boolean(obj.is_paused),
  };
}

function decodeUserPortfolio(raw: any): UserPortfolio {
  const obj = toPlainObject(raw);
  return {
    as_depositor: Array.isArray(obj?.as_depositor)
      ? obj.as_depositor.map(decodeEscrowSummary)
      : [],
    as_beneficiary: Array.isArray(obj?.as_beneficiary)
      ? obj.as_beneficiary.map(decodeEscrowSummary)
      : [],
  };
}

function decodeProgramDelegateInfo(raw: any): ProgramDelegateInfo {
  const obj = toPlainObject(raw);
  return {
    program_id: String(obj.program_id),
    delegate:
      obj.delegate === null || obj.delegate === undefined
        ? null
        : String(obj.delegate),
    permissions: Number(obj.permissions ?? 0),
  };
}

function toU64BigInt(value: bigint | number, fieldName: string): bigint {
  const big = typeof value === "bigint" ? value : BigInt(Math.trunc(value));
  if (
    typeof value === "number" &&
    (!Number.isFinite(value) || !Number.isInteger(value))
  ) {
    throw new ValidationError(
      `${fieldName} must be an integer`,
      fieldName,
    );
  }
  if (big < 0n) {
    throw new ValidationError(
      `${fieldName} must be non-negative`,
      fieldName,
    );
  }
  return big;
}

/**
 * Read-only TypeScript SDK client for the `escrow-view-facade` Soroban
 * contract.
 *
 * All methods use Soroban RPC transaction simulation with a throwaway source
 * account and never require a signing keypair.
 */
export class EscrowViewFacadeClient {
  private readonly contract: Contract;
  private readonly server: SorobanRpc.Server;
  private readonly config: EscrowViewFacadeConfig;
  private readonly sourceAccount: string;

  constructor(config: EscrowViewFacadeConfig) {
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
   * Safely retrieve an aggregated summary of a single escrow.
   * Returns `null` if the escrow does not exist.
   */
  async getEscrowSummary(
    escrowContract: string,
    bountyId: bigint | number,
  ): Promise<EscrowSummary | null> {
    validateContractId(escrowContract, "escrowContract");
    const id = toU64BigInt(bountyId, "bountyId");

    const result = await this.simulate(
      "get_escrow_summary",
      [
        Address.fromString(escrowContract).toScVal(),
        nativeToScVal(id, { type: "u64" }),
      ],
      "escrow",
      id.toString(),
    );

    if (result === null || result === undefined) {
      return null;
    }
    return decodeEscrowSummary(result);
  }

  /**
   * Retrieve an aggregated summary of a single escrow, throwing
   * {@link NotFoundError} if the escrow does not exist.
   */
  async getEscrowSummaryOrThrow(
    escrowContract: string,
    bountyId: bigint | number,
  ): Promise<EscrowSummary> {
    const summary = await this.getEscrowSummary(escrowContract, bountyId);
    if (!summary) {
      throw new NotFoundError(
        `Escrow summary not found for bounty ${bountyId.toString()}`,
        "escrow",
        bountyId.toString(),
      );
    }
    return summary;
  }

  /**
   * Retrieve summaries for a batch of `bountyIds`.
   * Missing escrows are omitted by the contract.
   */
  async getEscrowSummaries(
    escrowContract: string,
    bountyIds: Array<bigint | number>,
  ): Promise<EscrowSummary[]> {
    validateContractId(escrowContract, "escrowContract");
    if (bountyIds.length === 0) {
      return [];
    }

    const encodedIds = bountyIds.map((id, i) =>
      nativeToScVal(toU64BigInt(id, `bountyIds[${i}]`), { type: "u64" }),
    );

    const result = await this.simulate(
      "get_escrow_summaries",
      [
        Address.fromString(escrowContract).toScVal(),
        xdr.ScVal.scvVec(encodedIds),
      ],
      "escrows",
    );

    if (!Array.isArray(result)) {
      return [];
    }
    return result.map(decodeEscrowSummary);
  }

  /**
   * Fetch summaries for an arbitrarily large list of `bountyIds` in sequential
   * pages using {@link fetchAllPages}.
   */
  async fetchAllEscrowSummaries(
    escrowContract: string,
    bountyIds: Array<bigint | number>,
    batchSize: number = 50,
  ): Promise<EscrowSummary[]> {
    validateContractId(escrowContract, "escrowContract");
    if (!Number.isInteger(batchSize) || batchSize <= 0) {
      throw new ValidationError(
        "batchSize must be a positive integer",
        "batchSize",
      );
    }
    if (bountyIds.length === 0) {
      return [];
    }

    const pages = await fetchAllPages<EscrowSummary[]>(
      async (offset, limit) => {
        const slice = bountyIds.slice(offset, offset + limit);
        if (slice.length === 0) {
          return [];
        }
        const summaries = await this.getEscrowSummaries(escrowContract, slice);
        // Return one entry per requested ID slice so fetchAllPages advances
        // until the full bountyIds list is exhausted, even when some IDs are
        // missing on-chain.
        return slice.length < limit ? [summaries] : new Array(limit).fill([]).map((_, idx) => (idx === 0 ? summaries : []));
      },
      batchSize,
    );

    return pages.flat();
  }

  /**
   * Async generator that yields {@link EscrowSummary} records batch-by-batch
   * for the provided `bountyIds`.
   */
  async *iterateEscrowSummaries(
    escrowContract: string,
    bountyIds: Array<bigint | number>,
    batchSize: number = 50,
  ): AsyncGenerator<EscrowSummary, void, unknown> {
    validateContractId(escrowContract, "escrowContract");
    if (!Number.isInteger(batchSize) || batchSize <= 0) {
      throw new ValidationError(
        "batchSize must be a positive integer",
        "batchSize",
      );
    }
    for (let offset = 0; offset < bountyIds.length; offset += batchSize) {
      const slice = bountyIds.slice(offset, offset + batchSize);
      const summaries = await this.getEscrowSummaries(escrowContract, slice);
      for (const summary of summaries) {
        yield summary;
      }
    }
  }

  /**
   * Retrieve an aggregated view of a user's portfolio, including escrows they
   * deposited into and escrows where they are listed as beneficiary.
   */
  async getUserPortfolio(
    escrowContract: string,
    user: string,
  ): Promise<UserPortfolio> {
    validateContractId(escrowContract, "escrowContract");
    validateStellarAddress(user, "user");

    const result = await this.simulate(
      "get_user_portfolio",
      [
        Address.fromString(escrowContract).toScVal(),
        Address.fromString(user).toScVal(),
      ],
      "portfolio",
      user,
    );

    return decodeUserPortfolio(result);
  }

  /**
   * Query all current delegate assignments for a program escrow.
   * Returns an empty array if the program has no active delegates.
   */
  async queryAllDelegates(
    programContract: string,
    programId: string,
  ): Promise<ProgramDelegateInfo[]> {
    validateContractId(programContract, "programContract");
    if (!programId || programId.trim().length === 0) {
      throw new ValidationError("programId cannot be empty", "programId");
    }

    const result = await this.simulate(
      "query_all_delegates",
      [
        Address.fromString(programContract).toScVal(),
        nativeToScVal(programId, { type: "string" }),
      ],
      "delegates",
      programId,
    );

    if (!Array.isArray(result)) {
      return [];
    }
    return result.map(decodeProgramDelegateInfo);
  }

  /**
   * Query the payout history for a specific recipient within a program.
   * Returns an empty array if the recipient has never received a payout.
   */
  async queryRecipientHistory(
    programContract: string,
    programId: string,
    recipient: string,
  ): Promise<PayoutRecord[]> {
    validateContractId(programContract, "programContract");
    if (!programId || programId.trim().length === 0) {
      throw new ValidationError("programId cannot be empty", "programId");
    }
    validateStellarAddress(recipient, "recipient");

    const result = await this.simulate(
      "query_recipient_history",
      [
        Address.fromString(programContract).toScVal(),
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
