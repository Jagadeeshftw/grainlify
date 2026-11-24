import { Address, Keypair, nativeToScVal, xdr } from "@stellar/stellar-sdk";
import { BaseClientConfig, BaseContractClient } from "./base-client";
import { ValidationError } from "./errors";

export interface BountyEscrowConfig extends BaseClientConfig {}

export type EscrowStatus =
  | "Draft"
  | "Locked"
  | "Released"
  | "Refunded"
  | "PartiallyRefunded";

export type RefundMode = "Full" | "Partial";

export interface RefundApproval {
  bounty_id: bigint;
  amount: bigint;
  recipient: string;
  mode: RefundMode;
  approved_by: string;
  approved_at: number;
}

export interface RefundRecord {
  amount: bigint;
  recipient: string;
  timestamp: number;
  mode: RefundMode;
}

export interface Escrow {
  depositor: string;
  amount: bigint;
  remaining_amount: bigint;
  status: EscrowStatus;
  deadline: number;
  refund_history: RefundRecord[];
  archived: boolean;
  archived_at?: number;
}

export interface LockFundsItem {
  bounty_id: bigint;
  depositor: string;
  amount: bigint;
  deadline: bigint;
}

export interface PauseFlags {
  lock_paused: boolean;
  release_paused: boolean;
  refund_paused: boolean;
  pause_reason?: string;
  paused_at: number;
}

export interface RefundEligibility {
  canRefund: boolean;
  deadlinePassed: boolean;
  remainingAmount: bigint;
  approval: RefundApproval | null;
}

export interface SetPausedOptions {
  lock?: boolean;
  release?: boolean;
  refund?: boolean;
  reason?: string;
}

const READ_ONLY_METHODS = new Set([
  "get_escrow_info",
  "get_balance",
  "get_refund_eligibility",
  "get_pause_flags",
]);

const ESCROW_STATUSES: ReadonlyArray<EscrowStatus> = [
  "Draft",
  "Locked",
  "Released",
  "Refunded",
  "PartiallyRefunded",
];

/**
 * Client for the BountyEscrow Soroban contract.
 *
 * Covers the core bounty-escrow lifecycle: initialization, locking (single and
 * batch), release (full and partial), refunds, pause flags and read-only
 * escrow queries. Write methods return once the transaction has been confirmed;
 * read methods simulate without submitting.
 */
export class BountyEscrowClient extends BaseContractClient {
  constructor(config: BountyEscrowConfig) {
    super(config);
  }

  protected get contractErrorScope(): string {
    return "bounty_escrow";
  }

  protected isReadOnlyMethod(method: string): boolean {
    return READ_ONLY_METHODS.has(method);
  }

  /** Initialize the bounty escrow with an admin and token address. */
  async init(
    admin: string,
    token: string,
    sourceKeypair: Keypair,
  ): Promise<void> {
    this.validateAddress(admin, "admin");
    this.validateContractAddress(token, "token");

    try {
      await this.invokeContract("init", [admin, token], sourceKeypair);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /** Lock funds for a single bounty. */
  async lockFunds(
    depositor: string,
    bountyId: bigint,
    amount: bigint,
    deadline: bigint,
    sourceKeypair: Keypair,
  ): Promise<void> {
    this.validateAddress(depositor, "depositor");
    this.validateBountyId(bountyId);
    this.validatePositiveAmount(amount, "amount");
    this.validateDeadline(deadline);

    try {
      await this.invokeContract(
        "lock_funds",
        [depositor, bountyId, amount, deadline],
        sourceKeypair,
      );
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Lock funds for several bounties in one transaction.
   *
   * @returns The number of escrows created by the contract.
   */
  async batchLockFunds(
    items: LockFundsItem[],
    sourceKeypair: Keypair,
  ): Promise<number> {
    if (!Array.isArray(items) || items.length === 0) {
      throw new ValidationError("items cannot be empty", "items");
    }

    const seen = new Set<string>();
    items.forEach((item, index) => {
      this.validateBountyId(item.bounty_id, `items[${index}].bounty_id`);
      this.validateAddress(item.depositor, `items[${index}].depositor`);
      this.validatePositiveAmount(item.amount, `items[${index}].amount`);
      this.validateDeadline(item.deadline, `items[${index}].deadline`);

      const key = item.bounty_id.toString();
      if (seen.has(key)) {
        throw new ValidationError(
          `Duplicate bounty id in batch: ${key}`,
          `items[${index}].bounty_id`,
        );
      }
      seen.add(key);
    });

    try {
      const result = await this.invokeContract(
        "batch_lock_funds",
        [items],
        sourceKeypair,
      );
      return Number(result);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /** Release all remaining funds for a bounty to a contributor. */
  async releaseFunds(
    bountyId: bigint,
    contributor: string,
    sourceKeypair: Keypair,
  ): Promise<void> {
    this.validateBountyId(bountyId);
    this.validateAddress(contributor, "contributor");

    try {
      await this.invokeContract(
        "release_funds",
        [bountyId, contributor],
        sourceKeypair,
      );
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /** Release part of a bounty's remaining balance to a contributor. */
  async partialRelease(
    bountyId: bigint,
    contributor: string,
    payoutAmount: bigint,
    sourceKeypair: Keypair,
  ): Promise<void> {
    this.validateBountyId(bountyId);
    this.validateAddress(contributor, "contributor");
    this.validatePositiveAmount(payoutAmount, "payoutAmount");

    try {
      await this.invokeContract(
        "partial_release",
        [bountyId, contributor, payoutAmount],
        sourceKeypair,
      );
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /** Refund an escrow to its depositor. */
  async refund(bountyId: bigint, sourceKeypair: Keypair): Promise<void> {
    this.validateBountyId(bountyId);

    try {
      await this.invokeContract("refund", [bountyId], sourceKeypair);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /** Approve a full or partial refund before the escrow deadline (admin only). */
  async approveRefund(
    bountyId: bigint,
    amount: bigint,
    recipient: string,
    mode: RefundMode,
    sourceKeypair: Keypair,
  ): Promise<void> {
    this.validateBountyId(bountyId);
    this.validatePositiveAmount(amount, "amount");
    this.validateAddress(recipient, "recipient");

    if (mode !== "Full" && mode !== "Partial") {
      throw new ValidationError(
        'mode must be either "Full" or "Partial"',
        "mode",
      );
    }

    try {
      await this.invokeContract(
        "approve_refund",
        [bountyId, amount, recipient, mode],
        sourceKeypair,
      );
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /** Read an escrow's current state without submitting a transaction. */
  async getEscrowInfo(bountyId: bigint): Promise<Escrow> {
    this.validateBountyId(bountyId);

    try {
      const result = await this.invokeContract("get_escrow_info", [bountyId]);
      return this.parseEscrow(result);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /** Read the contract's total escrowed balance without submitting. */
  async getBalance(): Promise<bigint> {
    try {
      const result = await this.invokeContract("get_balance", []);
      return BigInt(result);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Read refund eligibility for a bounty without submitting a transaction.
   * Mirrors the contract's `(can_refund, deadline_passed, amount, approval)`
   * tuple.
   */
  async getRefundEligibility(bountyId: bigint): Promise<RefundEligibility> {
    this.validateBountyId(bountyId);

    try {
      const result = await this.invokeContract("get_refund_eligibility", [
        bountyId,
      ]);
      const [canRefund, deadlinePassed, remainingAmount, approval] =
        result as any[];

      return {
        canRefund: Boolean(canRefund),
        deadlinePassed: Boolean(deadlinePassed),
        remainingAmount: BigInt(remainingAmount),
        approval: approval
          ? (this.toPlainObject(approval) as RefundApproval)
          : null,
      };
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /** Pause or unpause lock, release or refund operations (admin only). */
  async setPaused(
    options: SetPausedOptions,
    sourceKeypair: Keypair,
  ): Promise<void> {
    try {
      await this.invokeContract(
        "set_paused",
        [options.lock, options.release, options.refund, options.reason],
        sourceKeypair,
      );
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /** Read the current pause flags without submitting a transaction. */
  async getPauseFlags(): Promise<PauseFlags> {
    try {
      const result = await this.invokeContract("get_pause_flags", []);
      const flags = this.toPlainObject(result) as any;

      return {
        lock_paused: Boolean(flags.lock_paused),
        release_paused: Boolean(flags.release_paused),
        refund_paused: Boolean(flags.refund_paused),
        pause_reason:
          flags.pause_reason === null || flags.pause_reason === undefined
            ? undefined
            : String(flags.pause_reason),
        paused_at: Number(flags.paused_at),
      };
    } catch (error) {
      throw this.handleError(error);
    }
  }

  protected encodeArguments(
    method: string,
    args: any[],
    _signer: Keypair,
  ): xdr.ScVal[] {
    const address = (value: string) => Address.fromString(value).toScVal();
    const i128 = (value: bigint) => nativeToScVal(value, { type: "i128" });
    const u64 = (value: bigint) => nativeToScVal(value, { type: "u64" });
    const string = (value: string) => nativeToScVal(value, { type: "string" });
    const option = <T>(
      value: T | undefined,
      encode: (inner: T) => xdr.ScVal,
    ): xdr.ScVal =>
      value === undefined || value === null
        ? xdr.ScVal.scvVoid()
        : encode(value);
    const refundMode = (value: RefundMode): xdr.ScVal =>
      xdr.ScVal.scvVec([xdr.ScVal.scvSymbol(value)]);

    const lockFundsItem = (item: LockFundsItem): xdr.ScVal =>
      xdr.ScVal.scvMap([
        // Keys must be sorted lexicographically for Soroban struct encoding.
        new xdr.ScMapEntry({
          key: xdr.ScVal.scvSymbol("amount"),
          val: i128(item.amount),
        }),
        new xdr.ScMapEntry({
          key: xdr.ScVal.scvSymbol("bounty_id"),
          val: u64(item.bounty_id),
        }),
        new xdr.ScMapEntry({
          key: xdr.ScVal.scvSymbol("deadline"),
          val: u64(item.deadline),
        }),
        new xdr.ScMapEntry({
          key: xdr.ScVal.scvSymbol("depositor"),
          val: address(item.depositor),
        }),
      ]);

    switch (method) {
      case "init":
        return [address(args[0]), address(args[1])];
      case "lock_funds":
        return [address(args[0]), u64(args[1]), i128(args[2]), u64(args[3])];
      case "batch_lock_funds":
        return [xdr.ScVal.scvVec(args[0].map(lockFundsItem))];
      case "release_funds":
        return [u64(args[0]), address(args[1])];
      case "partial_release":
        return [u64(args[0]), address(args[1]), i128(args[2])];
      case "refund":
        return [u64(args[0])];
      case "approve_refund":
        return [
          u64(args[0]),
          i128(args[1]),
          address(args[2]),
          refundMode(args[3]),
        ];
      case "get_escrow_info":
        return [u64(args[0])];
      case "get_balance":
      case "get_pause_flags":
        return [];
      case "get_refund_eligibility":
        return [u64(args[0])];
      case "set_paused":
        return [
          option(args[0], (value: boolean) => xdr.ScVal.scvBool(value)),
          option(args[1], (value: boolean) => xdr.ScVal.scvBool(value)),
          option(args[2], (value: boolean) => xdr.ScVal.scvBool(value)),
          option(args[3], string),
        ];
      default:
        throw new Error(`No Soroban argument encoder is defined for ${method}`);
    }
  }

  private parseEscrow(result: any): Escrow {
    const escrow = this.toPlainObject(result) as any;
    return {
      depositor: String(escrow.depositor),
      amount: BigInt(escrow.amount),
      remaining_amount: BigInt(escrow.remaining_amount),
      status: normalizeEscrowStatus(escrow.status),
      deadline: Number(escrow.deadline),
      refund_history: Array.isArray(escrow.refund_history)
        ? escrow.refund_history.map((record: any) => {
            const plain = this.toPlainObject(record);
            return {
              amount: BigInt(plain.amount),
              recipient: String(plain.recipient),
              timestamp: Number(plain.timestamp),
              mode: normalizeRefundMode(plain.mode),
            };
          })
        : [],
      archived: Boolean(escrow.archived),
      archived_at:
        escrow.archived_at === null || escrow.archived_at === undefined
          ? undefined
          : Number(escrow.archived_at),
    };
  }
}

function normalizeEscrowStatus(value: unknown): EscrowStatus {
  const raw = Array.isArray(value) ? value[0] : value;
  const text = String(raw);

  if ((ESCROW_STATUSES as ReadonlyArray<string>).includes(text)) {
    return text as EscrowStatus;
  }

  throw new Error(`Unknown escrow status returned by contract: ${text}`);
}

function normalizeRefundMode(value: unknown): RefundMode {
  const raw = Array.isArray(value) ? value[0] : value;
  const text = String(raw);

  if (text === "Full" || text === "Partial") {
    return text;
  }

  throw new Error(`Unknown refund mode returned by contract: ${text}`);
}

export default BountyEscrowClient;
