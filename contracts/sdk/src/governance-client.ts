import {
  Address,
  Keypair,
  nativeToScVal,
  xdr,
} from "@stellar/stellar-sdk";
import {
  ContractError,
  NetworkError,
  parseContractError,
  parseContractErrorByCode,
  ValidationError,
} from "./errors";
import {
  SorobanClientConfig,
  SorobanContractClient,
} from "./soroban-contract-client";

export interface GovernanceClientConfig extends SorobanClientConfig {}

export type VotingScheme = "OnePersonOneVote" | "TokenWeighted";

export interface GovernanceConfig {
  voting_period: bigint;
  execution_delay: bigint;
  quorum_percentage: number;
  approval_threshold: number;
  min_proposal_stake: bigint;
  voting_scheme: VotingScheme;
  governance_token: string;
}

export type WasmHash = Buffer | string;

export interface UpgradeProposal {
  proposal_id: bigint;
  proposer: string | null;
  wasm_hash: Buffer;
  expiry: bigint;
  cancelled: boolean;
}

export interface TimelockStatus {
  /** Remaining delay in seconds; null means approval has not started the lock. */
  remainingSeconds: bigint | null;
  ready: boolean;
}

export interface LivenessStatus {
  paused: boolean;
  read_only: boolean;
  healthy: boolean;
  last_ping_ts: bigint;
  is_paused: boolean;
  is_read_only: boolean;
  is_operational: boolean;
  version: number;
  admin_set: boolean;
  timestamp: bigint;
  schema_version: number;
}

/**
 * Client for governance, upgrade, migration, and liveness operations on grainlify-core.
 */
export class GovernanceClient extends SorobanContractClient {
  constructor(config: GovernanceClientConfig) {
    super(config);
  }

  async initGovernance(
    admin: string,
    config: GovernanceConfig,
    sourceKeypair: Keypair,
  ): Promise<void> {
    this.validateAddress(admin, "admin");
    this.validateGovernanceConfig(config);
    await this.runWrite(
      "init_governance",
      [admin, config],
      sourceKeypair,
    );
  }

  async proposeUpgrade(
    proposer: string,
    wasmHash: WasmHash,
    expiry: bigint,
    sourceKeypair: Keypair,
  ): Promise<bigint> {
    this.validateAddress(proposer, "proposer");
    const hash = this.validateHash(wasmHash, "wasmHash");
    if (expiry < 0n) {
      throw new ValidationError("Expiry cannot be negative", "expiry");
    }
    return this.toBigInt(
      await this.runWrite(
        "propose_upgrade",
        [proposer, hash, expiry],
        sourceKeypair,
      ),
      "proposal_id",
    );
  }

  async approveUpgrade(
    proposalId: bigint,
    signer: string,
    sourceKeypair: Keypair,
  ): Promise<void> {
    this.validateProposalId(proposalId);
    this.validateAddress(signer, "signer");
    await this.runWrite("approve_upgrade", [proposalId, signer], sourceKeypair);
  }

  async cancelUpgrade(
    proposalId: bigint,
    canceller: string,
    sourceKeypair: Keypair,
  ): Promise<void> {
    this.validateProposalId(proposalId);
    this.validateAddress(canceller, "canceller");
    await this.runWrite(
      "cancel_upgrade",
      [proposalId, canceller],
      sourceKeypair,
    );
  }

  async executeUpgrade(
    proposalId: bigint,
    sourceKeypair: Keypair,
  ): Promise<void> {
    this.validateProposalId(proposalId);
    await this.runWrite("execute_upgrade", [proposalId], sourceKeypair);
  }

  async getUpgradeProposal(proposalId: bigint): Promise<UpgradeProposal | null> {
    this.validateProposalId(proposalId);
    const result = await this.runRead("get_upgrade_proposal", [proposalId]);
    if (result === null || result === undefined) return null;

    const record = this.toObject(result, [
      "proposal_id",
      "proposer",
      "wasm_hash",
      "expiry",
      "cancelled",
    ]);
    const hash = record.wasm_hash;
    if (!Buffer.isBuffer(hash) && !(hash instanceof Uint8Array)) {
      throw new Error("RPC returned an invalid upgrade proposal WASM hash");
    }
    return {
      proposal_id: this.toBigInt(record.proposal_id, "proposal_id"),
      proposer: record.proposer == null ? null : String(record.proposer),
      wasm_hash: Buffer.from(hash),
      expiry: this.toBigInt(record.expiry, "expiry"),
      cancelled: Boolean(record.cancelled),
    };
  }

  async getTimelockStatus(proposalId: bigint): Promise<TimelockStatus> {
    this.validateProposalId(proposalId);
    const result = await this.runRead("get_timelock_status", [proposalId]);
    const remainingSeconds =
      result === null || result === undefined
        ? null
        : this.toBigInt(result, "remaining timelock");
    return {
      remainingSeconds,
      ready: remainingSeconds === 0n,
    };
  }

  /**
   * Checks approval/timelock readiness and proposal expiry at the latest ledger time.
   */
  async canExecute(proposalId: bigint): Promise<boolean> {
    const [proposal, timelock] = await Promise.all([
      this.getUpgradeProposal(proposalId),
      this.getTimelockStatus(proposalId),
    ]);
    if (!proposal || proposal.cancelled || !timelock.ready) return false;
    if (proposal.expiry === 0n) return true;
    return (await this.getLatestLedgerTime()) < proposal.expiry;
  }

  async commitMigration(
    targetVersion: number,
    hash: WasmHash,
    expiresAt: bigint,
    sourceKeypair: Keypair,
  ): Promise<void> {
    this.validateVersion(targetVersion);
    const validatedHash = this.validateHash(hash, "hash");
    if (expiresAt < 0n) {
      throw new ValidationError("Expiry cannot be negative", "expiresAt");
    }
    await this.runWrite(
      "commit_migration",
      [targetVersion, validatedHash, expiresAt],
      sourceKeypair,
    );
  }

  async migrate(
    targetVersion: number,
    migrationHash: WasmHash,
    sourceKeypair: Keypair,
  ): Promise<void> {
    this.validateVersion(targetVersion);
    const hash = this.validateHash(migrationHash, "migrationHash");
    await this.runWrite("migrate", [targetVersion, hash], sourceKeypair);
  }

  async getVersion(): Promise<number> {
    return Number(await this.runRead("get_version", []));
  }

  async isPaused(): Promise<boolean> {
    return Boolean(await this.runRead("is_paused", []));
  }

  async pause(signer: string, sourceKeypair: Keypair): Promise<void> {
    this.validateAddress(signer, "signer");
    await this.runWrite("pause", [signer], sourceKeypair);
  }

  async unpause(signer: string, sourceKeypair: Keypair): Promise<void> {
    this.validateAddress(signer, "signer");
    await this.runWrite("unpause", [signer], sourceKeypair);
  }

  async getLivenessStatus(): Promise<LivenessStatus> {
    const value = this.toObject(await this.runRead("liveness_watchdog", []), [
      "paused",
      "read_only",
      "healthy",
      "last_ping_ts",
      "is_paused",
      "is_read_only",
      "is_operational",
      "version",
      "admin_set",
      "timestamp",
      "schema_version",
    ]);
    return {
      paused: Boolean(value.paused),
      read_only: Boolean(value.read_only),
      healthy: Boolean(value.healthy),
      last_ping_ts: this.toBigInt(value.last_ping_ts, "last_ping_ts"),
      is_paused: Boolean(value.is_paused),
      is_read_only: Boolean(value.is_read_only),
      is_operational: Boolean(value.is_operational),
      version: this.toSafeNumber(value.version, "version"),
      admin_set: Boolean(value.admin_set),
      timestamp: this.toBigInt(value.timestamp, "timestamp"),
      schema_version: this.toSafeNumber(value.schema_version, "schema_version"),
    };
  }

  protected encodeArguments(
    method: string,
    args: unknown[],
    _signer: Keypair,
  ): xdr.ScVal[] {
    const address = (value: string) => Address.fromString(value).toScVal();
    const u32 = (value: number) => nativeToScVal(value, { type: "u32" });
    const u64 = (value: bigint) => nativeToScVal(value, { type: "u64" });
    const i128 = (value: bigint) => nativeToScVal(value, { type: "i128" });
    const hash = (value: Buffer) => nativeToScVal(value, { type: "bytes" });

    switch (method) {
      case "init_governance": {
        const config = args[1] as GovernanceConfig;
        const encodedConfig = xdr.ScVal.scvVec([
          u64(config.voting_period),
          u64(config.execution_delay),
          u32(config.quorum_percentage),
          u32(config.approval_threshold),
          i128(config.min_proposal_stake),
          nativeToScVal(config.voting_scheme, { type: "symbol" }),
          address(config.governance_token),
        ]);
        return [address(args[0] as string), encodedConfig];
      }
      case "propose_upgrade":
        return [
          address(args[0] as string),
          hash(args[1] as Buffer),
          u64(args[2] as bigint),
        ];
      case "approve_upgrade":
      case "cancel_upgrade":
        return [u64(args[0] as bigint), address(args[1] as string)];
      case "execute_upgrade":
      case "get_upgrade_proposal":
      case "get_timelock_status":
        return [u64(args[0] as bigint)];
      case "commit_migration":
        return [
          u32(args[0] as number),
          hash(args[1] as Buffer),
          u64(args[2] as bigint),
        ];
      case "migrate":
        return [u32(args[0] as number), hash(args[1] as Buffer)];
      case "pause":
      case "unpause":
        return [address(args[0] as string)];
      case "get_version":
      case "is_paused":
      case "liveness_watchdog":
        return [];
      default:
        throw new Error(`No Soroban argument encoder is defined for ${method}`);
    }
  }

  protected isReadOnlyMethod(method: string): boolean {
    return new Set([
      "get_upgrade_proposal",
      "get_timelock_status",
      "get_version",
      "is_paused",
      "liveness_watchdog",
    ]).has(method);
  }

  private async runRead(method: string, args: unknown[]): Promise<unknown> {
    try {
      return await this.invokeContract(method, args);
    } catch (error) {
      throw this.handleGovernanceError(error);
    }
  }

  private async runWrite(
    method: string,
    args: unknown[],
    sourceKeypair: Keypair,
  ): Promise<unknown> {
    try {
      return await this.invokeContract(method, args, sourceKeypair);
    } catch (error) {
      throw this.handleGovernanceError(error);
    }
  }

  private handleGovernanceError(error: unknown): Error {
    if (
      error instanceof ValidationError ||
      error instanceof NetworkError ||
      error instanceof ContractError
    ) {
      return error;
    }
    const message = error instanceof Error ? error.message : String(error);
    const code = message.match(/Error\(Contract,\s*(\d+)\)/);
    return code
      ? parseContractErrorByCode(Number(code[1]), "governance")
      : parseContractError(error);
  }

  private validateAddress(value: string, field: string): void {
    if (!value || !/^G[A-Z0-9]{55}$/.test(value)) {
      throw new ValidationError(`${field} is not a valid Stellar address`, field);
    }
  }

  private validateProposalId(proposalId: bigint): void {
    if (proposalId < 0n) {
      throw new ValidationError("Proposal ID cannot be negative", "proposalId");
    }
  }

  private validateVersion(version: number): void {
    if (!Number.isInteger(version) || version < 0 || version > 0xffffffff) {
      throw new ValidationError(
        "Target version must be a valid u32",
        "targetVersion",
      );
    }
  }

  private validateGovernanceConfig(config: GovernanceConfig): void {
    if (config.voting_period < 0n || config.execution_delay < 0n) {
      throw new ValidationError("Governance durations cannot be negative", "config");
    }
    if (
      !Number.isInteger(config.quorum_percentage) ||
      config.quorum_percentage <= 0 ||
      config.quorum_percentage > 10000
    ) {
      throw new ValidationError("Invalid quorum percentage", "quorum_percentage");
    }
    if (
      !Number.isInteger(config.approval_threshold) ||
      config.approval_threshold < 5000 ||
      config.approval_threshold > 10000
    ) {
      throw new ValidationError(
        "Invalid approval threshold",
        "approval_threshold",
      );
    }
    if (config.min_proposal_stake < 0n) {
      throw new ValidationError(
        "Minimum proposal stake cannot be negative",
        "min_proposal_stake",
      );
    }
    if (
      config.voting_scheme !== "OnePersonOneVote" &&
      config.voting_scheme !== "TokenWeighted"
    ) {
      throw new ValidationError("Invalid voting scheme", "voting_scheme");
    }
    this.validateAddress(config.governance_token, "governance_token");
  }

  private validateHash(value: WasmHash, field: string): Buffer {
    let hash: Buffer;
    if (Buffer.isBuffer(value)) {
      hash = Buffer.from(value);
    } else if (typeof value === "string") {
      const hex = value.startsWith("0x") ? value.slice(2) : value;
      if (!/^[0-9a-fA-F]{64}$/.test(hex)) {
        throw new ValidationError(`${field} must be a 32-byte hex string`, field);
      }
      hash = Buffer.from(hex, "hex");
    } else {
      throw new ValidationError(`${field} must be a 32-byte Buffer or hex string`, field);
    }
    if (hash.length !== 32) {
      throw new ValidationError(`${field} must be exactly 32 bytes`, field);
    }
    return hash;
  }

  private toObject(
    value: unknown,
    fields?: string[],
  ): Record<string, unknown> {
    if (Array.isArray(value) && fields) {
      return Object.fromEntries(fields.map((field, index) => [field, value[index]]));
    }
    if (value instanceof Map) {
      return Object.fromEntries(
        [...value.entries()].map(([key, item]) => [
          String(key),
          item instanceof Map ? this.toObject(item) : item,
        ]),
      );
    }
    if (typeof value === "object" && value !== null) {
      return value as Record<string, unknown>;
    }
    throw new Error("RPC returned an invalid contract record");
  }

  private toBigInt(value: unknown, field: string): bigint {
    if (typeof value === "bigint" || typeof value === "string") {
      return BigInt(value);
    }
    if (typeof value === "number" && Number.isSafeInteger(value)) {
      return BigInt(value);
    }
    throw new Error(`RPC returned an invalid ${field}`);
  }

  private toSafeNumber(value: unknown, field: string): number {
    if (typeof value === "number" && Number.isSafeInteger(value)) return value;
    if (
      typeof value === "bigint" &&
      value >= BigInt(Number.MIN_SAFE_INTEGER) &&
      value <= BigInt(Number.MAX_SAFE_INTEGER)
    ) {
      return Number(value);
    }
    throw new Error(`RPC returned an invalid ${field}`);
  }

  private async getLatestLedgerTime(): Promise<bigint> {
    const latestLedger = await this.server.getLatestLedger();
    const response = await fetch(this.config.rpcUrl, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        method: "getLedger",
        params: { ledger: latestLedger.sequence },
      }),
    });
    if (!response.ok) {
      throw new NetworkError(
        `RPC request failed with status ${response.status}`,
        response.status,
      );
    }
    const payload = (await response.json()) as {
      result?: { ledger?: string };
      error?: { message?: string };
    };
    if (!payload.result?.ledger) {
      throw new Error(payload.error?.message ?? "RPC getLedger returned no ledger data");
    }
    const metadata = xdr.LedgerCloseMeta.fromXDR(payload.result.ledger, "base64");
    const closeTime = metadata
      .value()
      .ledgerHeader()
      .header()
      .scpValue()
      .closeTime();
    return BigInt(closeTime.toString());
  }
}
