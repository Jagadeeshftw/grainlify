import {
  Address,
  Keypair,
  nativeToScVal,
  xdr,
} from "@stellar/stellar-sdk";
import { SorobanContractClient, SorobanClientConfig } from "./base-client";
import {
  ContractError,
  NetworkError,
  parseContractError,
  parseContractErrorByCode,
  ValidationError,
} from "./errors";

export interface GovernanceClientConfig extends SorobanClientConfig {}

export interface GovernanceConfig {
  voting_period: bigint | number;
  execution_delay: bigint | number;
  quorum_percentage: number;
  approval_threshold: number;
  min_proposal_stake: bigint;
  voting_scheme: "OnePersonOneVote" | "TokenWeighted";
  governance_token: string;
}

export interface UpgradeProposal {
  proposal_id: bigint;
  proposer: string | null;
  wasm_hash: Buffer;
  expiry: bigint;
  cancelled: boolean;
}

export type TimelockStatus = bigint | null;

type LatestLedgerWithCloseTime = {
  closeTime?: string | number;
};

/** Client for governance and contract-upgrade operations on grainlify-core. */
export class GovernanceClient extends SorobanContractClient {
  constructor(config: GovernanceClientConfig) {
    super(config);
  }

  async initGovernance(
    admin: string,
    config: GovernanceConfig,
    sourceKeypair?: Keypair,
  ): Promise<void> {
    this.validateAddress(admin, "admin");
    this.validateAddress(config.governance_token, "governance_token");
    await this.invokeGovernance(
      "init_governance",
      [admin, config],
      sourceKeypair,
    );
  }

  async proposeUpgrade(
    proposer: string,
    wasmHash: Buffer | string,
    expiry: bigint | number,
    sourceKeypair?: Keypair,
  ): Promise<bigint> {
    this.validateAddress(proposer, "proposer");
    const validatedHash = this.validateHash(wasmHash, "wasmHash");
    const result = await this.invokeGovernance(
      "propose_upgrade",
      [proposer, validatedHash, expiry],
      sourceKeypair,
    );
    return BigInt(result);
  }

  async approveUpgrade(
    proposalId: bigint | number,
    signer: string,
    sourceKeypair?: Keypair,
  ): Promise<void> {
    this.validateAddress(signer, "signer");
    await this.invokeGovernance(
      "approve_upgrade",
      [proposalId, signer],
      sourceKeypair,
    );
  }

  async cancelUpgrade(
    proposalId: bigint | number,
    canceller: string,
    sourceKeypair?: Keypair,
  ): Promise<void> {
    this.validateAddress(canceller, "canceller");
    await this.invokeGovernance(
      "cancel_upgrade",
      [proposalId, canceller],
      sourceKeypair,
    );
  }

  async getUpgradeProposal(
    proposalId: bigint | number,
  ): Promise<UpgradeProposal | null> {
    const result = await this.invokeGovernance(
      "get_upgrade_proposal",
      [proposalId],
    );
    if (result === null || result === undefined) {
      return null;
    }

    const proposal = this.toPlainObject(result);
    return {
      proposal_id: BigInt(proposal.proposal_id),
      proposer: proposal.proposer ?? null,
      wasm_hash: Buffer.from(proposal.wasm_hash),
      expiry: BigInt(proposal.expiry),
      cancelled: Boolean(proposal.cancelled),
    };
  }

  async executeUpgrade(
    proposalId: bigint | number,
    sourceKeypair?: Keypair,
  ): Promise<void> {
    await this.invokeGovernance(
      "execute_upgrade",
      [proposalId],
      sourceKeypair,
    );
  }

  async getTimelockStatus(
    proposalId: bigint | number,
  ): Promise<TimelockStatus> {
    const result = await this.invokeGovernance(
      "get_timelock_status",
      [proposalId],
    );
    return result === null || result === undefined ? null : BigInt(result);
  }

  /**
   * Reports whether the proposal is uncancelled, out of timelock, and unexpired.
   * Expiry is compared against the latest ledger close time rather than the
   * local machine clock.
   */
  async canExecute(proposalId: bigint | number): Promise<boolean> {
    const proposal = await this.getUpgradeProposal(proposalId);
    if (!proposal || proposal.cancelled) {
      return false;
    }

    const timelockStatus = await this.getTimelockStatus(proposalId);
    if (timelockStatus !== 0n) {
      return false;
    }

    if (proposal.expiry === 0n) {
      return true;
    }

    const latestLedger = (await this.server.getLatestLedger()) as
      | (Awaited<ReturnType<typeof this.server.getLatestLedger>> &
          LatestLedgerWithCloseTime)
      | LatestLedgerWithCloseTime;
    if (latestLedger.closeTime === undefined) {
      throw new Error("RPC getLatestLedger response did not include closeTime");
    }
    return BigInt(latestLedger.closeTime) < proposal.expiry;
  }

  async commitMigration(
    targetVersion: number,
    hash: Buffer | string,
    expiresAt: bigint | number,
    sourceKeypair?: Keypair,
  ): Promise<void> {
    const validatedHash = this.validateHash(hash, "hash");
    await this.invokeGovernance(
      "commit_migration",
      [targetVersion, validatedHash, expiresAt],
      sourceKeypair,
    );
  }

  async migrate(
    targetVersion: number,
    migrationHash: Buffer | string,
    sourceKeypair?: Keypair,
  ): Promise<void> {
    const validatedHash = this.validateHash(migrationHash, "migrationHash");
    await this.invokeGovernance(
      "migrate",
      [targetVersion, validatedHash],
      sourceKeypair,
    );
  }

  async getVersion(): Promise<number> {
    const result = await this.invokeGovernance("get_version", []);
    return Number(result);
  }

  async isPaused(): Promise<boolean> {
    return Boolean(await this.invokeGovernance("is_paused", []));
  }

  async pause(signer: string, sourceKeypair?: Keypair): Promise<void> {
    this.validateAddress(signer, "signer");
    await this.invokeGovernance("pause", [signer], sourceKeypair);
  }

  async unpause(signer: string, sourceKeypair?: Keypair): Promise<void> {
    this.validateAddress(signer, "signer");
    await this.invokeGovernance("unpause", [signer], sourceKeypair);
  }

  protected encodeArguments(
    method: string,
    args: any[],
    _signer: Keypair,
  ): xdr.ScVal[] {
    const address = (value: string) => Address.fromString(value).toScVal();
    const u32 = (value: number) => nativeToScVal(value, { type: "u32" });
    const i128 = (value: bigint) => nativeToScVal(value, { type: "i128" });
    const bytesN32 = (value: Buffer) =>
      nativeToScVal(value, { type: "bytes" });

    switch (method) {
      case "init_governance": {
        const config = args[1] as GovernanceConfig;
        const entries = ([
          ["voting_period", this.u64(config.voting_period)],
          ["execution_delay", this.u64(config.execution_delay)],
          ["quorum_percentage", u32(config.quorum_percentage)],
          ["approval_threshold", u32(config.approval_threshold)],
          ["min_proposal_stake", i128(config.min_proposal_stake)],
          [
            "voting_scheme",
            xdr.ScVal.scvVec([
              xdr.ScVal.scvSymbol(config.voting_scheme),
            ]),
          ],
          ["governance_token", address(config.governance_token)],
        ] as [string, xdr.ScVal][]).sort(([left], [right]) =>
          left.localeCompare(right),
        );
        return [
          address(args[0]),
          xdr.ScVal.scvMap(
            entries.map(
              ([key, value]) =>
                new xdr.ScMapEntry({
                  key: xdr.ScVal.scvSymbol(key),
                  val: value,
                }),
            ),
          ),
        ];
      }
      case "propose_upgrade":
        return [
          address(args[0]),
          bytesN32(args[1]),
          this.u64(args[2]),
        ];
      case "approve_upgrade":
      case "cancel_upgrade":
        return [this.u64(args[0]), address(args[1])];
      case "get_upgrade_proposal":
      case "execute_upgrade":
      case "get_timelock_status":
        return [this.u64(args[0])];
      case "commit_migration":
        return [u32(args[0]), bytesN32(args[1]), this.u64(args[2])];
      case "migrate":
        return [u32(args[0]), bytesN32(args[1])];
      case "pause":
      case "unpause":
        return [address(args[0])];
      case "get_version":
      case "is_paused":
        return [];
      default:
        throw new Error(`No Soroban argument encoder is defined for ${method}`);
    }
  }

  protected isReadOnlyMethod(method: string): boolean {
    return (
      method === "get_upgrade_proposal" ||
      method === "get_timelock_status" ||
      method === "get_version" ||
      method === "is_paused"
    );
  }

  private async invokeGovernance(
    method: string,
    args: any[],
    sourceKeypair?: Keypair,
  ): Promise<any> {
    try {
      return await this.invokeContract(method, args, sourceKeypair);
    } catch (error) {
      throw this.handleGovernanceError(error);
    }
  }

  private validateAddress(address: string, field: string): void {
    if (!address || address.trim().length === 0) {
      throw new ValidationError(`${field} cannot be empty`, field);
    }
    try {
      Address.fromString(address);
    } catch {
      throw new ValidationError(`${field} is not a valid Stellar address`, field);
    }
  }

  private validateHash(value: Buffer | string, field: string): Buffer {
    if (Buffer.isBuffer(value)) {
      if (value.length !== 32) {
        throw new ValidationError(`${field} must be exactly 32 bytes`, field);
      }
      return Buffer.from(value);
    }

    const hex = value.startsWith("0x") ? value.slice(2) : value;
    if (!/^[0-9a-fA-F]{64}$/.test(hex)) {
      throw new ValidationError(
        `${field} must be a 32-byte Buffer or 64-character hex string`,
        field,
      );
    }
    return Buffer.from(hex, "hex");
  }

  private handleGovernanceError(error: any): Error {
    if (
      error instanceof ValidationError ||
      error instanceof NetworkError ||
      error instanceof ContractError
    ) {
      return error;
    }

    const contractErrorCode =
      typeof error?.contractErrorCode === "number"
        ? error.contractErrorCode
        : typeof error?.code === "number"
          ? error.code
          : error?.message?.match(/Error\(Contract,\s*#(\d+)\)/)?.[1];
    if (contractErrorCode !== undefined) {
      return parseContractErrorByCode(Number(contractErrorCode), "governance");
    }
    return parseContractError(error);
  }
}
