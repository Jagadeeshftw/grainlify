import { Buffer } from "buffer";
import { Address } from "@stellar/stellar-sdk";
import {
  AssembledTransaction,
  Client as ContractClient,
  Ok,
  Err,
  Spec as ContractSpec,
} from "@stellar/stellar-sdk/contract";
import type {
  u32,
  i32,
  u64,
  i64,
  u128,
  i128,
  u256,
  i256,
  Option,
  Timepoint,
  Duration,
} from "@stellar/stellar-sdk/contract";
export type Result<T> = Ok<T> | Err<any>;
export * from "@stellar/stellar-sdk";
export * as contract from "@stellar/stellar-sdk/contract";
export * as rpc from "@stellar/stellar-sdk/rpc";

if (typeof window !== "undefined") {
  //@ts-ignore Buffer exists
  window.Buffer = window.Buffer || Buffer;
}





export interface AddressState {
  last_operation_timestamp: u64;
  operation_count: u32;
  window_start_timestamp: u64;
}

export type AntiAbuseKey = {tag: "Config", values: void} | {tag: "State", values: readonly [string]} | {tag: "Whitelist", values: readonly [string]} | {tag: "Admin", values: void};


export interface AntiAbuseConfig {
  cooldown_period: u64;
  max_operations: u32;
  window_size: u64;
}


export interface Analytics {
  error_count: u64;
  error_rate: u32;
  operation_count: u64;
  unique_users: u64;
}


export interface HealthStatus {
  contract_version: string;
  is_healthy: boolean;
  last_operation: u64;
  total_operations: u64;
}


export interface StateSnapshot {
  timestamp: u64;
  total_errors: u64;
  total_operations: u64;
  total_users: u64;
}


export interface OperationMetric {
  caller: string;
  operation: string;
  success: boolean;
  timestamp: u64;
}


export interface PerformanceStats {
  avg_time: u64;
  call_count: u64;
  function_name: string;
  last_called: u64;
  total_time: u64;
}


export interface PerformanceMetric {
  duration: u64;
  function: string;
  timestamp: u64;
}

export const Errors = {
  /**
   * Returned when attempting to initialize an already initialized contract
   */
  1: {message:"AlreadyInitialized"},
  /**
   * Returned when calling contract functions before initialization
   */
  2: {message:"NotInitialized"},
  /**
   * Returned when attempting to lock funds with a duplicate bounty ID
   */
  3: {message:"BountyExists"},
  /**
   * Returned when querying or operating on a non-existent bounty
   */
  4: {message:"BountyNotFound"},
  /**
   * Returned when attempting operations on non-LOCKED funds
   */
  5: {message:"FundsNotLocked"},
  /**
   * Returned when attempting refund before the deadline has passed
   */
  6: {message:"DeadlineNotPassed"},
  /**
   * Returned when caller lacks required authorization for the operation
   */
  7: {message:"Unauthorized"},
  /**
   * Returned when amount is invalid (zero, negative, or exceeds available)
   */
  8: {message:"InvalidAmount"},
  /**
   * Returned when deadline is invalid (in the past or too far in the future)
   */
  9: {message:"InvalidDeadline"},
  10: {message:"BatchSizeMismatch"},
  11: {message:"DuplicateBountyId"},
  /**
   * Returned when contract has insufficient funds for the operation
   */
  12: {message:"InsufficientFunds"},
  /**
   * Returned when refund is attempted without admin approval
   */
  13: {message:"RefundNotApproved"}
}


/**
 * Complete escrow record for a bounty.
 * 
 * # Fields
 * * `depositor` - Address that locked the funds (receives refunds)
 * * `amount` - Token amount held in escrow (in smallest denomination)
 * * `status` - Current state of the escrow (Locked/Released/Refunded)
 * * `deadline` - Unix timestamp after which refunds are allowed
 * 
 * # Storage
 * Stored in persistent storage with key `DataKey::Escrow(bounty_id)`.
 * TTL is automatically extended on access.
 * 
 * # Example
 * ```rust
 * let escrow = Escrow {
 * depositor: depositor_address,
 * amount: 1000_0000000, // 1000 tokens
 * status: EscrowStatus::Locked,
 * deadline: current_time + 2592000, // 30 days
 * };
 * ```
 */
export interface Escrow {
  amount: i128;
  deadline: u64;
  depositor: string;
  refund_history: Array<RefundRecord>;
  remaining_amount: i128;
  status: EscrowStatus;
}

export type DataKey = {tag: "Admin", values: void} | {tag: "Token", values: void} | {tag: "Escrow", values: readonly [u64]} | {tag: "RefundApproval", values: readonly [u64]} | {tag: "ReentrancyGuard", values: void};

export type RefundMode = {tag: "Full", values: void} | {tag: "Partial", values: void} | {tag: "Custom", values: void};

/**
 * Represents the current state of escrowed funds.
 * 
 * # State Transitions
 * ```text
 * NONE → Locked → Released (final)
 * ↓
 * Refunded (final)
 * ```
 * 
 * # States
 * * `Locked` - Funds are held in escrow, awaiting release or refund
 * * `Released` - Funds have been transferred to contributor (final state)
 * * `Refunded` - Funds have been returned to depositor (final state)
 * 
 * # Invariants
 * - Once in Released or Refunded state, no further transitions allowed
 * - Only Locked state allows state changes
 */
export type EscrowStatus = {tag: "Locked", values: void} | {tag: "Released", values: void} | {tag: "Refunded", values: void} | {tag: "PartiallyRefunded", values: void};


export interface RefundRecord {
  amount: i128;
  mode: RefundMode;
  recipient: string;
  timestamp: u64;
}


/**
 * Storage keys for contract data.
 * 
 * # Keys
 * * `Admin` - Stores the admin address (instance storage)
 * * `Token` - Stores the token contract address (instance storage)
 * * `Escrow(u64)` - Stores escrow data indexed by bounty_id (persistent storage)
 * 
 * # Storage Types
 * - **Instance Storage**: Admin and Token (never expires, tied to contract)
 * - **Persistent Storage**: Individual escrow records (extended TTL on access)
 */
export interface LockFundsItem {
  amount: i128;
  bounty_id: u64;
  deadline: u64;
  depositor: string;
}


export interface RefundApproval {
  amount: i128;
  approved_at: u64;
  approved_by: string;
  bounty_id: u64;
  mode: RefundMode;
  recipient: string;
}


export interface ReleaseFundsItem {
  bounty_id: u64;
  contributor: string;
}


/**
 * Event emitted when funds are locked in escrow for a bounty.
 * 
 * # Fields
 * * `bounty_id` - Unique identifier for the bounty
 * * `amount` - Amount of tokens locked (in stroops for XLM)
 * * `depositor` - Address that deposited the funds
 * * `deadline` - Unix timestamp after which refunds are allowed
 * 
 * # Event Topic
 * Symbol: `f_lock`
 * Indexed: `bounty_id` (allows filtering by specific bounty)
 * 
 * # State Transition
 * ```text
 * NONE → LOCKED
 * ```
 * 
 * # Usage
 * Emitted when a bounty creator locks funds for a task. The depositor
 * transfers tokens to the contract, which holds them until release or refund.
 * 
 * # Security Considerations
 * - Amount must be positive and within depositor's balance
 * - Bounty ID must be unique (no duplicates allowed)
 * - Deadline must be in the future
 * - Depositor must authorize the transaction
 * 
 * # Example Usage
 * ```rust
 * // Lock 1000 XLM for bounty #42, deadline in 30 days
 * let deadline = env.ledger().timestamp() + (30 * 24 * 60 * 60);
 * escrow_client.lock_funds(&depositor, &42, &10_000_000_000, &deadline);
 * // → Emits FundsLoc
 */
export interface FundsLocked {
  amount: i128;
  bounty_id: u64;
  deadline: u64;
  depositor: string;
}


/**
 * Event emitted when escrowed funds are refunded to the depositor.
 * 
 * # Fields
 * * `bounty_id` - The bounty identifier
 * * `amount` - Amount refunded to depositor
 * * `refund_to` - Address receiving the refund (original depositor)
 * * `timestamp` - Unix timestamp of refund
 * 
 * # Event Topic
 * Symbol: `f_ref`
 * Indexed: `bounty_id`
 * 
 * # State Transition
 * ```text
 * LOCKED → REFUNDED (final state)
 * ```
 * 
 * # Usage
 * Emitted when funds are returned to the depositor after the deadline
 * has passed without the bounty being completed. This mechanism prevents
 * funds from being locked indefinitely.
 * 
 * # Conditions
 * - Deadline must have passed (timestamp > deadline)
 * - Funds must still be in LOCKED state
 * - Can be triggered by anyone (permissionless but conditional)
 * 
 * # Security Considerations
 * - Time-based protection ensures funds aren't stuck
 * - Permissionless refund prevents admin monopoly
 * - Original depositor always receives refund
 * - Cannot refund if already released or refunded
 * 
 * # Example Usage
 * ```rust
 * // After deadline passes, anyone can trigger refun
 */
export interface FundsRefunded {
  amount: i128;
  bounty_id: u64;
  refund_mode: RefundMode;
  refund_to: string;
  remaining_amount: i128;
  timestamp: u64;
}


/**
 * Event emitted when escrowed funds are released to a contributor.
 * 
 * # Fields
 * * `bounty_id` - The bounty identifier
 * * `amount` - Amount transferred to recipient
 * * `recipient` - Address receiving the funds (contributor)
 * * `timestamp` - Unix timestamp of release
 * 
 * # Event Topic
 * Symbol: `f_rel`
 * Indexed: `bounty_id`
 * 
 * # State Transition
 * ```text
 * LOCKED → RELEASED (final state)
 * ```
 * 
 * # Usage
 * Emitted when the admin releases funds to a contributor who completed
 * the bounty task. This is a final, irreversible action.
 * 
 * # Authorization
 * - Only the contract admin can trigger fund release
 * - Funds must be in LOCKED state
 * - Cannot release funds that were already released or refunded
 * 
 * # Security Considerations
 * - Admin authorization is critical (should be secure backend)
 * - Recipient address should be verified off-chain before release
 * - Once released, funds cannot be retrieved
 * - Atomic operation: transfer + state update
 * 
 * # Example Usage
 * ```rust
 * // Admin releases 1000 XLM to contributor for bounty #42
 * escrow_client.release_funds(&42,
 */
export interface FundsReleased {
  amount: i128;
  bounty_id: u64;
  recipient: string;
  timestamp: u64;
}


export interface BatchFundsLocked {
  count: u32;
  timestamp: u64;
  total_amount: i128;
}


export interface BatchFundsReleased {
  count: u32;
  timestamp: u64;
  total_amount: i128;
}


/**
 * Event emitted when the Bounty Escrow contract is initialized.
 * 
 * # Fields
 * * `admin` - The administrator address with release authorization
 * * `token` - The token contract address (typically XLM/USDC)
 * * `timestamp` - Unix timestamp of initialization
 * 
 * # Event Topic
 * Symbol: `init`
 * 
 * # Usage
 * This event is emitted once during contract deployment and signals
 * that the contract is ready to accept bounty escrows.
 * 
 * # Security Considerations
 * - Only emitted once; subsequent init attempts should fail
 * - Admin address should be a secure backend service
 * - Token address must be a valid Stellar token contract
 * 
 * # Example Off-chain Indexing
 * ```javascript
 * // Listen for initialization events
 * stellar.events.on('init', (event) => {
 * console.log(`Contract initialized by ${event.admin}`);
 * console.log(`Using token: ${event.token}`);
 * });
 * ```
 */
export interface BountyEscrowInitialized {
  admin: string;
  timestamp: u64;
  token: string;
}

export interface Client {
  /**
   * Construct and simulate a init transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Initializes the Bounty Escrow contract with admin and token addresses.
   * 
   * # Arguments
   * * `env` - The contract environment
   * * `admin` - Address authorized to release funds
   * * `token` - Token contract address for escrow payments (e.g., XLM, USDC)
   * 
   * # Returns
   * * `Ok(())` - Contract successfully initialized
   * * `Err(Error::AlreadyInitialized)` - Contract already initialized
   * 
   * # State Changes
   * - Sets Admin address in instance storage
   * - Sets Token address in instance storage
   * - Emits BountyEscrowInitialized event
   * 
   * # Security Considerations
   * - Can only be called once (prevents admin takeover)
   * - Admin should be a secure backend service address
   * - Token must be a valid Stellar Asset Contract
   * - No authorization required (first-caller initialization)
   * 
   * # Events
   * Emits: `BountyEscrowInitialized { admin, token, timestamp }`
   * 
   * # Example
   * ```rust
   * let admin = Address::from_string("GADMIN...");
   * let usdc_token = Address::from_string("CUSDC...");
   * escrow_client.init(&admin, &usdc_token)?;
   * ```
   * 
   * # Gas Cost
   * Low - Only two storage writes
   */
  init: ({admin, token}: {admin: string, token: string}, options?: any) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a refund transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Refund funds with support for Full, Partial, and Custom refunds.
   * - Full: refunds all remaining funds to depositor
   * - Partial: refunds specified amount to depositor
   * - Custom: refunds specified amount to specified recipient (requires admin approval if before deadline)
   */
  refund: ({bounty_id, amount, recipient, mode}: {bounty_id: u64, amount: Option<i128>, recipient: Option<string>, mode: RefundMode}, options?: any) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a lock_funds transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Locks funds in escrow for a specific bounty.
   * 
   * # Arguments
   * * `env` - The contract environment
   * * `depositor` - Address depositing the funds (must authorize)
   * * `bounty_id` - Unique identifier for this bounty
   * * `amount` - Token amount to lock (in smallest denomination)
   * * `deadline` - Unix timestamp after which refund is allowed
   * 
   * # Returns
   * * `Ok(())` - Funds successfully locked
   * * `Err(Error::NotInitialized)` - Contract not initialized
   * * `Err(Error::BountyExists)` - Bounty ID already in use
   * 
   * # State Changes
   * - Transfers `amount` tokens from depositor to contract
   * - Creates Escrow record in persistent storage
   * - Emits FundsLocked event
   * 
   * # Authorization
   * - Depositor must authorize the transaction
   * - Depositor must have sufficient token balance
   * - Depositor must have approved contract for token transfer
   * 
   * # Security Considerations
   * - Bounty ID must be unique (prevents overwrites)
   * - Amount must be positive (enforced by token contract)
   * - Deadline should be reasonable (recommended: 7-90 days)
   * - Token transfer is atomic with stat
   */
  lock_funds: ({depositor, bounty_id, amount, deadline}: {depositor: string, bounty_id: u64, amount: i128, deadline: u64}, options?: any) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a get_balance transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Returns the current token balance held by the contract.
   * 
   * # Arguments
   * * `env` - The contract environment
   * 
   * # Returns
   * * `Ok(i128)` - Current contract token balance
   * * `Err(Error::NotInitialized)` - Contract not initialized
   * 
   * # Use Cases
   * - Monitoring total locked funds
   * - Verifying contract solvency
   * - Auditing and reconciliation
   * 
   * # Gas Cost
   * Low - Token contract call
   * 
   * # Example
   * ```rust
   * let balance = escrow_client.get_balance()?;
   * println!("Total locked: {} stroops", balance);
   * ```
   */
  get_balance: (options?: any) => Promise<AssembledTransaction<Result<i128>>>

  /**
   * Construct and simulate a release_funds transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Releases escrowed funds to a contributor.
   * 
   * # Arguments
   * * `env` - The contract environment
   * * `bounty_id` - The bounty to release funds for
   * * `contributor` - Address to receive the funds
   * 
   * # Returns
   * * `Ok(())` - Funds successfully released
   * * `Err(Error::NotInitialized)` - Contract not initialized
   * * `Err(Error::Unauthorized)` - Caller is not the admin
   * * `Err(Error::BountyNotFound)` - Bounty doesn't exist
   * * `Err(Error::FundsNotLocked)` - Funds not in LOCKED state
   * 
   * # State Changes
   * - Transfers tokens from contract to contributor
   * - Updates escrow status to Released
   * - Emits FundsReleased event
   * 
   * # Authorization
   * - **CRITICAL**: Only admin can call this function
   * - Admin address must match initialization value
   * 
   * # Security Considerations
   * - This is the most security-critical function
   * - Admin should verify task completion off-chain before calling
   * - Once released, funds cannot be retrieved
   * - Recipient address should be verified carefully
   * - Consider implementing multi-sig for admin
   * 
   * # Events
   * Emits: `FundsReleased { bounty_id, 
   */
  release_funds: ({bounty_id, contributor}: {bounty_id: u64, contributor: string}, options?: any) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a approve_refund transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Approve a refund before deadline (admin only).
   * This allows early refunds with admin approval.
   */
  approve_refund: ({bounty_id, amount, recipient, mode}: {bounty_id: u64, amount: i128, recipient: string, mode: RefundMode}, options?: any) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a get_escrow_info transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Retrieves escrow information for a specific bounty.
   * 
   * # Arguments
   * * `env` - The contract environment
   * * `bounty_id` - The bounty to query
   * 
   * # Returns
   * * `Ok(Escrow)` - The complete escrow record
   * * `Err(Error::BountyNotFound)` - Bounty doesn't exist
   * 
   * # Gas Cost
   * Very Low - Single storage read
   * 
   * # Example
   * ```rust
   * let escrow_info = escrow_client.get_escrow_info(&42)?;
   * println!("Amount: {}", escrow_info.amount);
   * println!("Status: {:?}", escrow_info.status);
   * println!("Deadline: {}", escrow_info.deadline);
   * ```
   */
  get_escrow_info: ({bounty_id}: {bounty_id: u64}, options?: any) => Promise<AssembledTransaction<Result<Escrow>>>

  /**
   * Construct and simulate a batch_lock_funds transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Batch lock funds for multiple bounties in a single transaction.
   * This improves gas efficiency by reducing transaction overhead.
   * 
   * # Arguments
   * * `items` - Vector of LockFundsItem containing bounty_id, depositor, amount, and deadline
   * 
   * # Returns
   * Number of successfully locked bounties
   * 
   * # Errors
   * * InvalidBatchSize - if batch size exceeds MAX_BATCH_SIZE or is zero
   * * BountyExists - if any bounty_id already exists
   * * NotInitialized - if contract is not initialized
   * 
   * # Note
   * This operation is atomic - if any item fails, the entire transaction reverts.
   */
  batch_lock_funds: ({items}: {items: Array<LockFundsItem>}, options?: any) => Promise<AssembledTransaction<Result<u32>>>

  /**
   * Construct and simulate a get_refund_history transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Retrieves the refund history for a specific bounty.
   * 
   * # Arguments
   * * `env` - The contract environment
   * * `bounty_id` - The bounty to query
   * 
   * # Returns
   * * `Ok(Vec<RefundRecord>)` - The refund history
   * * `Err(Error::BountyNotFound)` - Bounty doesn't exist
   */
  get_refund_history: ({bounty_id}: {bounty_id: u64}, options?: any) => Promise<AssembledTransaction<Result<Array<RefundRecord>>>>

  /**
   * Construct and simulate a batch_release_funds transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Batch release funds to multiple contributors in a single transaction.
   * This improves gas efficiency by reducing transaction overhead.
   * 
   * # Arguments
   * * `items` - Vector of ReleaseFundsItem containing bounty_id and contributor address
   * 
   * # Returns
   * Number of successfully released bounties
   * 
   * # Errors
   * * InvalidBatchSize - if batch size exceeds MAX_BATCH_SIZE or is zero
   * * BountyNotFound - if any bounty_id doesn't exist
   * * FundsNotLocked - if any bounty is not in Locked status
   * * Unauthorized - if caller is not admin
   * 
   * # Note
   * This operation is atomic - if any item fails, the entire transaction reverts.
   */
  batch_release_funds: ({items}: {items: Array<ReleaseFundsItem>}, options?: any) => Promise<AssembledTransaction<Result<u32>>>

  /**
   * Construct and simulate a get_refund_eligibility transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Gets refund eligibility information for a bounty.
   * 
   * # Arguments
   * * `env` - The contract environment
   * * `bounty_id` - The bounty to query
   * 
   * # Returns
   * * `Ok((bool, bool, i128, Option<RefundApproval>))` - Tuple containing:
   * - can_refund: Whether refund is possible
   * - deadline_passed: Whether the deadline has passed
   * - remaining: Remaining amount in escrow
   * - approval: Optional refund approval if exists
   * * `Err(Error::BountyNotFound)` - Bounty doesn't exist
   */
  get_refund_eligibility: ({bounty_id}: {bounty_id: u64}, options?: any) => Promise<AssembledTransaction<Result<readonly [boolean, boolean, i128, Option<RefundApproval>]>>>

}
export class Client extends ContractClient {
  static async deploy<T = Client>(
    /** Options for initializing a Client as well as for calling a method, with extras specific to deploying. */
    options: any &
      Omit<any, "contractId"> & {
        /** The hash of the Wasm blob, which must already be installed on-chain. */
        wasmHash: Buffer | string;
        /** Salt used to generate the contract's ID. Passed through to {@link Operation.createCustomContract}. Default: random. */
        salt?: Buffer | Uint8Array;
        /** The format used to decode `wasmHash`, if it's provided as a string. */
        format?: "hex" | "base64";
      }
  ): Promise<AssembledTransaction<T>> {
    return ContractClient.deploy(null, options)
  }
  constructor(options: any) {
    super(
      new ContractSpec([ "AAAAAQAAAAAAAAAAAAAADEFkZHJlc3NTdGF0ZQAAAAMAAAAAAAAAGGxhc3Rfb3BlcmF0aW9uX3RpbWVzdGFtcAAAAAYAAAAAAAAAD29wZXJhdGlvbl9jb3VudAAAAAAEAAAAAAAAABZ3aW5kb3dfc3RhcnRfdGltZXN0YW1wAAAAAAAG",
        "AAAAAgAAAAAAAAAAAAAADEFudGlBYnVzZUtleQAAAAQAAAAAAAAAAAAAAAZDb25maWcAAAAAAAEAAAAAAAAABVN0YXRlAAAAAAAAAQAAABMAAAABAAAAAAAAAAlXaGl0ZWxpc3QAAAAAAAABAAAAEwAAAAAAAAAAAAAABUFkbWluAAAA",
        "AAAAAQAAAAAAAAAAAAAAD0FudGlBYnVzZUNvbmZpZwAAAAADAAAAAAAAAA9jb29sZG93bl9wZXJpb2QAAAAABgAAAAAAAAAObWF4X29wZXJhdGlvbnMAAAAAAAQAAAAAAAAAC3dpbmRvd19zaXplAAAAAAY=",
        "AAAAAQAAAAAAAAAAAAAACUFuYWx5dGljcwAAAAAAAAQAAAAAAAAAC2Vycm9yX2NvdW50AAAAAAYAAAAAAAAACmVycm9yX3JhdGUAAAAAAAQAAAAAAAAAD29wZXJhdGlvbl9jb3VudAAAAAAGAAAAAAAAAAx1bmlxdWVfdXNlcnMAAAAG",
        "AAAAAQAAAAAAAAAAAAAADEhlYWx0aFN0YXR1cwAAAAQAAAAAAAAAEGNvbnRyYWN0X3ZlcnNpb24AAAAQAAAAAAAAAAppc19oZWFsdGh5AAAAAAABAAAAAAAAAA5sYXN0X29wZXJhdGlvbgAAAAAABgAAAAAAAAAQdG90YWxfb3BlcmF0aW9ucwAAAAY=",
        "AAAAAQAAAAAAAAAAAAAADVN0YXRlU25hcHNob3QAAAAAAAAEAAAAAAAAAAl0aW1lc3RhbXAAAAAAAAAGAAAAAAAAAAx0b3RhbF9lcnJvcnMAAAAGAAAAAAAAABB0b3RhbF9vcGVyYXRpb25zAAAABgAAAAAAAAALdG90YWxfdXNlcnMAAAAABg==",
        "AAAAAQAAAAAAAAAAAAAAD09wZXJhdGlvbk1ldHJpYwAAAAAEAAAAAAAAAAZjYWxsZXIAAAAAABMAAAAAAAAACW9wZXJhdGlvbgAAAAAAABEAAAAAAAAAB3N1Y2Nlc3MAAAAAAQAAAAAAAAAJdGltZXN0YW1wAAAAAAAABg==",
        "AAAAAQAAAAAAAAAAAAAAEFBlcmZvcm1hbmNlU3RhdHMAAAAFAAAAAAAAAAhhdmdfdGltZQAAAAYAAAAAAAAACmNhbGxfY291bnQAAAAAAAYAAAAAAAAADWZ1bmN0aW9uX25hbWUAAAAAAAARAAAAAAAAAAtsYXN0X2NhbGxlZAAAAAAGAAAAAAAAAAp0b3RhbF90aW1lAAAAAAAG",
        "AAAAAQAAAAAAAAAAAAAAEVBlcmZvcm1hbmNlTWV0cmljAAAAAAAAAwAAAAAAAAAIZHVyYXRpb24AAAAGAAAAAAAAAAhmdW5jdGlvbgAAABEAAAAAAAAACXRpbWVzdGFtcAAAAAAAAAY=",
        "AAAAAAAAA/NJbml0aWFsaXplcyB0aGUgQm91bnR5IEVzY3JvdyBjb250cmFjdCB3aXRoIGFkbWluIGFuZCB0b2tlbiBhZGRyZXNzZXMuCgojIEFyZ3VtZW50cwoqIGBlbnZgIC0gVGhlIGNvbnRyYWN0IGVudmlyb25tZW50CiogYGFkbWluYCAtIEFkZHJlc3MgYXV0aG9yaXplZCB0byByZWxlYXNlIGZ1bmRzCiogYHRva2VuYCAtIFRva2VuIGNvbnRyYWN0IGFkZHJlc3MgZm9yIGVzY3JvdyBwYXltZW50cyAoZS5nLiwgWExNLCBVU0RDKQoKIyBSZXR1cm5zCiogYE9rKCgpKWAgLSBDb250cmFjdCBzdWNjZXNzZnVsbHkgaW5pdGlhbGl6ZWQKKiBgRXJyKEVycm9yOjpBbHJlYWR5SW5pdGlhbGl6ZWQpYCAtIENvbnRyYWN0IGFscmVhZHkgaW5pdGlhbGl6ZWQKCiMgU3RhdGUgQ2hhbmdlcwotIFNldHMgQWRtaW4gYWRkcmVzcyBpbiBpbnN0YW5jZSBzdG9yYWdlCi0gU2V0cyBUb2tlbiBhZGRyZXNzIGluIGluc3RhbmNlIHN0b3JhZ2UKLSBFbWl0cyBCb3VudHlFc2Nyb3dJbml0aWFsaXplZCBldmVudAoKIyBTZWN1cml0eSBDb25zaWRlcmF0aW9ucwotIENhbiBvbmx5IGJlIGNhbGxlZCBvbmNlIChwcmV2ZW50cyBhZG1pbiB0YWtlb3ZlcikKLSBBZG1pbiBzaG91bGQgYmUgYSBzZWN1cmUgYmFja2VuZCBzZXJ2aWNlIGFkZHJlc3MKLSBUb2tlbiBtdXN0IGJlIGEgdmFsaWQgU3RlbGxhciBBc3NldCBDb250cmFjdAotIE5vIGF1dGhvcml6YXRpb24gcmVxdWlyZWQgKGZpcnN0LWNhbGxlciBpbml0aWFsaXphdGlvbikKCiMgRXZlbnRzCkVtaXRzOiBgQm91bnR5RXNjcm93SW5pdGlhbGl6ZWQgeyBhZG1pbiwgdG9rZW4sIHRpbWVzdGFtcCB9YAoKIyBFeGFtcGxlCmBgYHJ1c3QKbGV0IGFkbWluID0gQWRkcmVzczo6ZnJvbV9zdHJpbmcoIkdBRE1JTi4uLiIpOwpsZXQgdXNkY190b2tlbiA9IEFkZHJlc3M6OmZyb21fc3RyaW5nKCJDVVNEQy4uLiIpOwplc2Nyb3dfY2xpZW50LmluaXQoJmFkbWluLCAmdXNkY190b2tlbik/OwpgYGAKCiMgR2FzIENvc3QKTG93IC0gT25seSB0d28gc3RvcmFnZSB3cml0ZXMAAAAABGluaXQAAAACAAAAAAAAAAVhZG1pbgAAAAAAABMAAAAAAAAABXRva2VuAAAAAAAAEwAAAAEAAAPpAAAD7QAAAAAAAAAD",
        "AAAAAAAAAQlSZWZ1bmQgZnVuZHMgd2l0aCBzdXBwb3J0IGZvciBGdWxsLCBQYXJ0aWFsLCBhbmQgQ3VzdG9tIHJlZnVuZHMuCi0gRnVsbDogcmVmdW5kcyBhbGwgcmVtYWluaW5nIGZ1bmRzIHRvIGRlcG9zaXRvcgotIFBhcnRpYWw6IHJlZnVuZHMgc3BlY2lmaWVkIGFtb3VudCB0byBkZXBvc2l0b3IKLSBDdXN0b206IHJlZnVuZHMgc3BlY2lmaWVkIGFtb3VudCB0byBzcGVjaWZpZWQgcmVjaXBpZW50IChyZXF1aXJlcyBhZG1pbiBhcHByb3ZhbCBpZiBiZWZvcmUgZGVhZGxpbmUpAAAAAAAABnJlZnVuZAAAAAAABAAAAAAAAAAJYm91bnR5X2lkAAAAAAAABgAAAAAAAAAGYW1vdW50AAAAAAPoAAAACwAAAAAAAAAJcmVjaXBpZW50AAAAAAAD6AAAABMAAAAAAAAABG1vZGUAAAfQAAAAClJlZnVuZE1vZGUAAAAAAAEAAAPpAAAD7QAAAAAAAAAD",
        "AAAABAAAAAAAAAAAAAAABUVycm9yAAAAAAAADQAAAEZSZXR1cm5lZCB3aGVuIGF0dGVtcHRpbmcgdG8gaW5pdGlhbGl6ZSBhbiBhbHJlYWR5IGluaXRpYWxpemVkIGNvbnRyYWN0AAAAAAASQWxyZWFkeUluaXRpYWxpemVkAAAAAAABAAAAPlJldHVybmVkIHdoZW4gY2FsbGluZyBjb250cmFjdCBmdW5jdGlvbnMgYmVmb3JlIGluaXRpYWxpemF0aW9uAAAAAAAOTm90SW5pdGlhbGl6ZWQAAAAAAAIAAABBUmV0dXJuZWQgd2hlbiBhdHRlbXB0aW5nIHRvIGxvY2sgZnVuZHMgd2l0aCBhIGR1cGxpY2F0ZSBib3VudHkgSUQAAAAAAAAMQm91bnR5RXhpc3RzAAAAAwAAADxSZXR1cm5lZCB3aGVuIHF1ZXJ5aW5nIG9yIG9wZXJhdGluZyBvbiBhIG5vbi1leGlzdGVudCBib3VudHkAAAAOQm91bnR5Tm90Rm91bmQAAAAAAAQAAAA3UmV0dXJuZWQgd2hlbiBhdHRlbXB0aW5nIG9wZXJhdGlvbnMgb24gbm9uLUxPQ0tFRCBmdW5kcwAAAAAORnVuZHNOb3RMb2NrZWQAAAAAAAUAAAA+UmV0dXJuZWQgd2hlbiBhdHRlbXB0aW5nIHJlZnVuZCBiZWZvcmUgdGhlIGRlYWRsaW5lIGhhcyBwYXNzZWQAAAAAABFEZWFkbGluZU5vdFBhc3NlZAAAAAAAAAYAAABDUmV0dXJuZWQgd2hlbiBjYWxsZXIgbGFja3MgcmVxdWlyZWQgYXV0aG9yaXphdGlvbiBmb3IgdGhlIG9wZXJhdGlvbgAAAAAMVW5hdXRob3JpemVkAAAABwAAAEZSZXR1cm5lZCB3aGVuIGFtb3VudCBpcyBpbnZhbGlkICh6ZXJvLCBuZWdhdGl2ZSwgb3IgZXhjZWVkcyBhdmFpbGFibGUpAAAAAAANSW52YWxpZEFtb3VudAAAAAAAAAgAAABIUmV0dXJuZWQgd2hlbiBkZWFkbGluZSBpcyBpbnZhbGlkIChpbiB0aGUgcGFzdCBvciB0b28gZmFyIGluIHRoZSBmdXR1cmUpAAAAD0ludmFsaWREZWFkbGluZQAAAAAJAAAAAAAAABFCYXRjaFNpemVNaXNtYXRjaAAAAAAAAAoAAAAAAAAAEUR1cGxpY2F0ZUJvdW50eUlkAAAAAAAACwAAAD9SZXR1cm5lZCB3aGVuIGNvbnRyYWN0IGhhcyBpbnN1ZmZpY2llbnQgZnVuZHMgZm9yIHRoZSBvcGVyYXRpb24AAAAAEUluc3VmZmljaWVudEZ1bmRzAAAAAAAADAAAADhSZXR1cm5lZCB3aGVuIHJlZnVuZCBpcyBhdHRlbXB0ZWQgd2l0aG91dCBhZG1pbiBhcHByb3ZhbAAAABFSZWZ1bmROb3RBcHByb3ZlZAAAAAAAAA0=",
        "AAAAAQAAAmtDb21wbGV0ZSBlc2Nyb3cgcmVjb3JkIGZvciBhIGJvdW50eS4KCiMgRmllbGRzCiogYGRlcG9zaXRvcmAgLSBBZGRyZXNzIHRoYXQgbG9ja2VkIHRoZSBmdW5kcyAocmVjZWl2ZXMgcmVmdW5kcykKKiBgYW1vdW50YCAtIFRva2VuIGFtb3VudCBoZWxkIGluIGVzY3JvdyAoaW4gc21hbGxlc3QgZGVub21pbmF0aW9uKQoqIGBzdGF0dXNgIC0gQ3VycmVudCBzdGF0ZSBvZiB0aGUgZXNjcm93IChMb2NrZWQvUmVsZWFzZWQvUmVmdW5kZWQpCiogYGRlYWRsaW5lYCAtIFVuaXggdGltZXN0YW1wIGFmdGVyIHdoaWNoIHJlZnVuZHMgYXJlIGFsbG93ZWQKCiMgU3RvcmFnZQpTdG9yZWQgaW4gcGVyc2lzdGVudCBzdG9yYWdlIHdpdGgga2V5IGBEYXRhS2V5OjpFc2Nyb3coYm91bnR5X2lkKWAuClRUTCBpcyBhdXRvbWF0aWNhbGx5IGV4dGVuZGVkIG9uIGFjY2Vzcy4KCiMgRXhhbXBsZQpgYGBydXN0CmxldCBlc2Nyb3cgPSBFc2Nyb3cgewpkZXBvc2l0b3I6IGRlcG9zaXRvcl9hZGRyZXNzLAphbW91bnQ6IDEwMDBfMDAwMDAwMCwgLy8gMTAwMCB0b2tlbnMKc3RhdHVzOiBFc2Nyb3dTdGF0dXM6OkxvY2tlZCwKZGVhZGxpbmU6IGN1cnJlbnRfdGltZSArIDI1OTIwMDAsIC8vIDMwIGRheXMKfTsKYGBgAAAAAAAAAAAGRXNjcm93AAAAAAAGAAAAAAAAAAZhbW91bnQAAAAAAAsAAAAAAAAACGRlYWRsaW5lAAAABgAAAAAAAAAJZGVwb3NpdG9yAAAAAAAAEwAAAAAAAAAOcmVmdW5kX2hpc3RvcnkAAAAAA+oAAAfQAAAADFJlZnVuZFJlY29yZAAAAAAAAAAQcmVtYWluaW5nX2Ftb3VudAAAAAsAAAAAAAAABnN0YXR1cwAAAAAH0AAAAAxFc2Nyb3dTdGF0dXM=",
        "AAAAAgAAAAAAAAAAAAAAB0RhdGFLZXkAAAAABQAAAAAAAAAAAAAABUFkbWluAAAAAAAAAAAAAAAAAAAFVG9rZW4AAAAAAAABAAAAAAAAAAZFc2Nyb3cAAAAAAAEAAAAGAAAAAQAAAAAAAAAOUmVmdW5kQXBwcm92YWwAAAAAAAEAAAAGAAAAAAAAAAAAAAAPUmVlbnRyYW5jeUd1YXJkAA==",
        "AAAAAAAABABMb2NrcyBmdW5kcyBpbiBlc2Nyb3cgZm9yIGEgc3BlY2lmaWMgYm91bnR5LgoKIyBBcmd1bWVudHMKKiBgZW52YCAtIFRoZSBjb250cmFjdCBlbnZpcm9ubWVudAoqIGBkZXBvc2l0b3JgIC0gQWRkcmVzcyBkZXBvc2l0aW5nIHRoZSBmdW5kcyAobXVzdCBhdXRob3JpemUpCiogYGJvdW50eV9pZGAgLSBVbmlxdWUgaWRlbnRpZmllciBmb3IgdGhpcyBib3VudHkKKiBgYW1vdW50YCAtIFRva2VuIGFtb3VudCB0byBsb2NrIChpbiBzbWFsbGVzdCBkZW5vbWluYXRpb24pCiogYGRlYWRsaW5lYCAtIFVuaXggdGltZXN0YW1wIGFmdGVyIHdoaWNoIHJlZnVuZCBpcyBhbGxvd2VkCgojIFJldHVybnMKKiBgT2soKCkpYCAtIEZ1bmRzIHN1Y2Nlc3NmdWxseSBsb2NrZWQKKiBgRXJyKEVycm9yOjpOb3RJbml0aWFsaXplZClgIC0gQ29udHJhY3Qgbm90IGluaXRpYWxpemVkCiogYEVycihFcnJvcjo6Qm91bnR5RXhpc3RzKWAgLSBCb3VudHkgSUQgYWxyZWFkeSBpbiB1c2UKCiMgU3RhdGUgQ2hhbmdlcwotIFRyYW5zZmVycyBgYW1vdW50YCB0b2tlbnMgZnJvbSBkZXBvc2l0b3IgdG8gY29udHJhY3QKLSBDcmVhdGVzIEVzY3JvdyByZWNvcmQgaW4gcGVyc2lzdGVudCBzdG9yYWdlCi0gRW1pdHMgRnVuZHNMb2NrZWQgZXZlbnQKCiMgQXV0aG9yaXphdGlvbgotIERlcG9zaXRvciBtdXN0IGF1dGhvcml6ZSB0aGUgdHJhbnNhY3Rpb24KLSBEZXBvc2l0b3IgbXVzdCBoYXZlIHN1ZmZpY2llbnQgdG9rZW4gYmFsYW5jZQotIERlcG9zaXRvciBtdXN0IGhhdmUgYXBwcm92ZWQgY29udHJhY3QgZm9yIHRva2VuIHRyYW5zZmVyCgojIFNlY3VyaXR5IENvbnNpZGVyYXRpb25zCi0gQm91bnR5IElEIG11c3QgYmUgdW5pcXVlIChwcmV2ZW50cyBvdmVyd3JpdGVzKQotIEFtb3VudCBtdXN0IGJlIHBvc2l0aXZlIChlbmZvcmNlZCBieSB0b2tlbiBjb250cmFjdCkKLSBEZWFkbGluZSBzaG91bGQgYmUgcmVhc29uYWJsZSAocmVjb21tZW5kZWQ6IDctOTAgZGF5cykKLSBUb2tlbiB0cmFuc2ZlciBpcyBhdG9taWMgd2l0aCBzdGF0AAAACmxvY2tfZnVuZHMAAAAAAAQAAAAAAAAACWRlcG9zaXRvcgAAAAAAABMAAAAAAAAACWJvdW50eV9pZAAAAAAAAAYAAAAAAAAABmFtb3VudAAAAAAACwAAAAAAAAAIZGVhZGxpbmUAAAAGAAAAAQAAA+kAAAPtAAAAAAAAAAM=",
        "AAAAAAAAAdtSZXR1cm5zIHRoZSBjdXJyZW50IHRva2VuIGJhbGFuY2UgaGVsZCBieSB0aGUgY29udHJhY3QuCgojIEFyZ3VtZW50cwoqIGBlbnZgIC0gVGhlIGNvbnRyYWN0IGVudmlyb25tZW50CgojIFJldHVybnMKKiBgT2soaTEyOClgIC0gQ3VycmVudCBjb250cmFjdCB0b2tlbiBiYWxhbmNlCiogYEVycihFcnJvcjo6Tm90SW5pdGlhbGl6ZWQpYCAtIENvbnRyYWN0IG5vdCBpbml0aWFsaXplZAoKIyBVc2UgQ2FzZXMKLSBNb25pdG9yaW5nIHRvdGFsIGxvY2tlZCBmdW5kcwotIFZlcmlmeWluZyBjb250cmFjdCBzb2x2ZW5jeQotIEF1ZGl0aW5nIGFuZCByZWNvbmNpbGlhdGlvbgoKIyBHYXMgQ29zdApMb3cgLSBUb2tlbiBjb250cmFjdCBjYWxsCgojIEV4YW1wbGUKYGBgcnVzdApsZXQgYmFsYW5jZSA9IGVzY3Jvd19jbGllbnQuZ2V0X2JhbGFuY2UoKT87CnByaW50bG4hKCJUb3RhbCBsb2NrZWQ6IHt9IHN0cm9vcHMiLCBiYWxhbmNlKTsKYGBgAAAAAAtnZXRfYmFsYW5jZQAAAAAAAAAAAQAAA+kAAAALAAAAAw==",
        "AAAAAgAAAAAAAAAAAAAAClJlZnVuZE1vZGUAAAAAAAMAAAAAAAAAAAAAAARGdWxsAAAAAAAAAAAAAAAHUGFydGlhbAAAAAAAAAAAAAAAAAZDdXN0b20AAA==",
        "AAAAAAAABABSZWxlYXNlcyBlc2Nyb3dlZCBmdW5kcyB0byBhIGNvbnRyaWJ1dG9yLgoKIyBBcmd1bWVudHMKKiBgZW52YCAtIFRoZSBjb250cmFjdCBlbnZpcm9ubWVudAoqIGBib3VudHlfaWRgIC0gVGhlIGJvdW50eSB0byByZWxlYXNlIGZ1bmRzIGZvcgoqIGBjb250cmlidXRvcmAgLSBBZGRyZXNzIHRvIHJlY2VpdmUgdGhlIGZ1bmRzCgojIFJldHVybnMKKiBgT2soKCkpYCAtIEZ1bmRzIHN1Y2Nlc3NmdWxseSByZWxlYXNlZAoqIGBFcnIoRXJyb3I6Ok5vdEluaXRpYWxpemVkKWAgLSBDb250cmFjdCBub3QgaW5pdGlhbGl6ZWQKKiBgRXJyKEVycm9yOjpVbmF1dGhvcml6ZWQpYCAtIENhbGxlciBpcyBub3QgdGhlIGFkbWluCiogYEVycihFcnJvcjo6Qm91bnR5Tm90Rm91bmQpYCAtIEJvdW50eSBkb2Vzbid0IGV4aXN0CiogYEVycihFcnJvcjo6RnVuZHNOb3RMb2NrZWQpYCAtIEZ1bmRzIG5vdCBpbiBMT0NLRUQgc3RhdGUKCiMgU3RhdGUgQ2hhbmdlcwotIFRyYW5zZmVycyB0b2tlbnMgZnJvbSBjb250cmFjdCB0byBjb250cmlidXRvcgotIFVwZGF0ZXMgZXNjcm93IHN0YXR1cyB0byBSZWxlYXNlZAotIEVtaXRzIEZ1bmRzUmVsZWFzZWQgZXZlbnQKCiMgQXV0aG9yaXphdGlvbgotICoqQ1JJVElDQUwqKjogT25seSBhZG1pbiBjYW4gY2FsbCB0aGlzIGZ1bmN0aW9uCi0gQWRtaW4gYWRkcmVzcyBtdXN0IG1hdGNoIGluaXRpYWxpemF0aW9uIHZhbHVlCgojIFNlY3VyaXR5IENvbnNpZGVyYXRpb25zCi0gVGhpcyBpcyB0aGUgbW9zdCBzZWN1cml0eS1jcml0aWNhbCBmdW5jdGlvbgotIEFkbWluIHNob3VsZCB2ZXJpZnkgdGFzayBjb21wbGV0aW9uIG9mZi1jaGFpbiBiZWZvcmUgY2FsbGluZwotIE9uY2UgcmVsZWFzZWQsIGZ1bmRzIGNhbm5vdCBiZSByZXRyaWV2ZWQKLSBSZWNpcGllbnQgYWRkcmVzcyBzaG91bGQgYmUgdmVyaWZpZWQgY2FyZWZ1bGx5Ci0gQ29uc2lkZXIgaW1wbGVtZW50aW5nIG11bHRpLXNpZyBmb3IgYWRtaW4KCiMgRXZlbnRzCkVtaXRzOiBgRnVuZHNSZWxlYXNlZCB7IGJvdW50eV9pZCwgAAAADXJlbGVhc2VfZnVuZHMAAAAAAAACAAAAAAAAAAlib3VudHlfaWQAAAAAAAAGAAAAAAAAAAtjb250cmlidXRvcgAAAAATAAAAAQAAA+kAAAPtAAAAAAAAAAM=",
        "AAAAAAAAAF1BcHByb3ZlIGEgcmVmdW5kIGJlZm9yZSBkZWFkbGluZSAoYWRtaW4gb25seSkuClRoaXMgYWxsb3dzIGVhcmx5IHJlZnVuZHMgd2l0aCBhZG1pbiBhcHByb3ZhbC4AAAAAAAAOYXBwcm92ZV9yZWZ1bmQAAAAAAAQAAAAAAAAACWJvdW50eV9pZAAAAAAAAAYAAAAAAAAABmFtb3VudAAAAAAACwAAAAAAAAAJcmVjaXBpZW50AAAAAAAAEwAAAAAAAAAEbW9kZQAAB9AAAAAKUmVmdW5kTW9kZQAAAAAAAQAAA+kAAAPtAAAAAAAAAAM=",
        "AAAAAgAAAd1SZXByZXNlbnRzIHRoZSBjdXJyZW50IHN0YXRlIG9mIGVzY3Jvd2VkIGZ1bmRzLgoKIyBTdGF0ZSBUcmFuc2l0aW9ucwpgYGB0ZXh0Ck5PTkUg4oaSIExvY2tlZCDihpIgUmVsZWFzZWQgKGZpbmFsKQrihpMKUmVmdW5kZWQgKGZpbmFsKQpgYGAKCiMgU3RhdGVzCiogYExvY2tlZGAgLSBGdW5kcyBhcmUgaGVsZCBpbiBlc2Nyb3csIGF3YWl0aW5nIHJlbGVhc2Ugb3IgcmVmdW5kCiogYFJlbGVhc2VkYCAtIEZ1bmRzIGhhdmUgYmVlbiB0cmFuc2ZlcnJlZCB0byBjb250cmlidXRvciAoZmluYWwgc3RhdGUpCiogYFJlZnVuZGVkYCAtIEZ1bmRzIGhhdmUgYmVlbiByZXR1cm5lZCB0byBkZXBvc2l0b3IgKGZpbmFsIHN0YXRlKQoKIyBJbnZhcmlhbnRzCi0gT25jZSBpbiBSZWxlYXNlZCBvciBSZWZ1bmRlZCBzdGF0ZSwgbm8gZnVydGhlciB0cmFuc2l0aW9ucyBhbGxvd2VkCi0gT25seSBMb2NrZWQgc3RhdGUgYWxsb3dzIHN0YXRlIGNoYW5nZXMAAAAAAAAAAAAADEVzY3Jvd1N0YXR1cwAAAAQAAAAAAAAAAAAAAAZMb2NrZWQAAAAAAAAAAAAAAAAACFJlbGVhc2VkAAAAAAAAAAAAAAAIUmVmdW5kZWQAAAAAAAAAAAAAABFQYXJ0aWFsbHlSZWZ1bmRlZAAAAA==",
        "AAAAAQAAAAAAAAAAAAAADFJlZnVuZFJlY29yZAAAAAQAAAAAAAAABmFtb3VudAAAAAAACwAAAAAAAAAEbW9kZQAAB9AAAAAKUmVmdW5kTW9kZQAAAAAAAAAAAAlyZWNpcGllbnQAAAAAAAATAAAAAAAAAAl0aW1lc3RhbXAAAAAAAAAG",
        "AAAAAAAAAfdSZXRyaWV2ZXMgZXNjcm93IGluZm9ybWF0aW9uIGZvciBhIHNwZWNpZmljIGJvdW50eS4KCiMgQXJndW1lbnRzCiogYGVudmAgLSBUaGUgY29udHJhY3QgZW52aXJvbm1lbnQKKiBgYm91bnR5X2lkYCAtIFRoZSBib3VudHkgdG8gcXVlcnkKCiMgUmV0dXJucwoqIGBPayhFc2Nyb3cpYCAtIFRoZSBjb21wbGV0ZSBlc2Nyb3cgcmVjb3JkCiogYEVycihFcnJvcjo6Qm91bnR5Tm90Rm91bmQpYCAtIEJvdW50eSBkb2Vzbid0IGV4aXN0CgojIEdhcyBDb3N0ClZlcnkgTG93IC0gU2luZ2xlIHN0b3JhZ2UgcmVhZAoKIyBFeGFtcGxlCmBgYHJ1c3QKbGV0IGVzY3Jvd19pbmZvID0gZXNjcm93X2NsaWVudC5nZXRfZXNjcm93X2luZm8oJjQyKT87CnByaW50bG4hKCJBbW91bnQ6IHt9IiwgZXNjcm93X2luZm8uYW1vdW50KTsKcHJpbnRsbiEoIlN0YXR1czogezo/fSIsIGVzY3Jvd19pbmZvLnN0YXR1cyk7CnByaW50bG4hKCJEZWFkbGluZToge30iLCBlc2Nyb3dfaW5mby5kZWFkbGluZSk7CmBgYAAAAAAPZ2V0X2VzY3Jvd19pbmZvAAAAAAEAAAAAAAAACWJvdW50eV9pZAAAAAAAAAYAAAABAAAD6QAAB9AAAAAGRXNjcm93AAAAAAAD",
        "AAAAAQAAAZdTdG9yYWdlIGtleXMgZm9yIGNvbnRyYWN0IGRhdGEuCgojIEtleXMKKiBgQWRtaW5gIC0gU3RvcmVzIHRoZSBhZG1pbiBhZGRyZXNzIChpbnN0YW5jZSBzdG9yYWdlKQoqIGBUb2tlbmAgLSBTdG9yZXMgdGhlIHRva2VuIGNvbnRyYWN0IGFkZHJlc3MgKGluc3RhbmNlIHN0b3JhZ2UpCiogYEVzY3Jvdyh1NjQpYCAtIFN0b3JlcyBlc2Nyb3cgZGF0YSBpbmRleGVkIGJ5IGJvdW50eV9pZCAocGVyc2lzdGVudCBzdG9yYWdlKQoKIyBTdG9yYWdlIFR5cGVzCi0gKipJbnN0YW5jZSBTdG9yYWdlKio6IEFkbWluIGFuZCBUb2tlbiAobmV2ZXIgZXhwaXJlcywgdGllZCB0byBjb250cmFjdCkKLSAqKlBlcnNpc3RlbnQgU3RvcmFnZSoqOiBJbmRpdmlkdWFsIGVzY3JvdyByZWNvcmRzIChleHRlbmRlZCBUVEwgb24gYWNjZXNzKQAAAAAAAAAADUxvY2tGdW5kc0l0ZW0AAAAAAAAEAAAAAAAAAAZhbW91bnQAAAAAAAsAAAAAAAAACWJvdW50eV9pZAAAAAAAAAYAAAAAAAAACGRlYWRsaW5lAAAABgAAAAAAAAAJZGVwb3NpdG9yAAAAAAAAEw==",
        "AAAAAAAAAh9CYXRjaCBsb2NrIGZ1bmRzIGZvciBtdWx0aXBsZSBib3VudGllcyBpbiBhIHNpbmdsZSB0cmFuc2FjdGlvbi4KVGhpcyBpbXByb3ZlcyBnYXMgZWZmaWNpZW5jeSBieSByZWR1Y2luZyB0cmFuc2FjdGlvbiBvdmVyaGVhZC4KCiMgQXJndW1lbnRzCiogYGl0ZW1zYCAtIFZlY3RvciBvZiBMb2NrRnVuZHNJdGVtIGNvbnRhaW5pbmcgYm91bnR5X2lkLCBkZXBvc2l0b3IsIGFtb3VudCwgYW5kIGRlYWRsaW5lCgojIFJldHVybnMKTnVtYmVyIG9mIHN1Y2Nlc3NmdWxseSBsb2NrZWQgYm91bnRpZXMKCiMgRXJyb3JzCiogSW52YWxpZEJhdGNoU2l6ZSAtIGlmIGJhdGNoIHNpemUgZXhjZWVkcyBNQVhfQkFUQ0hfU0laRSBvciBpcyB6ZXJvCiogQm91bnR5RXhpc3RzIC0gaWYgYW55IGJvdW50eV9pZCBhbHJlYWR5IGV4aXN0cwoqIE5vdEluaXRpYWxpemVkIC0gaWYgY29udHJhY3QgaXMgbm90IGluaXRpYWxpemVkCgojIE5vdGUKVGhpcyBvcGVyYXRpb24gaXMgYXRvbWljIC0gaWYgYW55IGl0ZW0gZmFpbHMsIHRoZSBlbnRpcmUgdHJhbnNhY3Rpb24gcmV2ZXJ0cy4AAAAAEGJhdGNoX2xvY2tfZnVuZHMAAAABAAAAAAAAAAVpdGVtcwAAAAAAA+oAAAfQAAAADUxvY2tGdW5kc0l0ZW0AAAAAAAABAAAD6QAAAAQAAAAD",
        "AAAAAQAAAAAAAAAAAAAADlJlZnVuZEFwcHJvdmFsAAAAAAAGAAAAAAAAAAZhbW91bnQAAAAAAAsAAAAAAAAAC2FwcHJvdmVkX2F0AAAAAAYAAAAAAAAAC2FwcHJvdmVkX2J5AAAAABMAAAAAAAAACWJvdW50eV9pZAAAAAAAAAYAAAAAAAAABG1vZGUAAAfQAAAAClJlZnVuZE1vZGUAAAAAAAAAAAAJcmVjaXBpZW50AAAAAAAAEw==",
        "AAAAAAAAAPdSZXRyaWV2ZXMgdGhlIHJlZnVuZCBoaXN0b3J5IGZvciBhIHNwZWNpZmljIGJvdW50eS4KCiMgQXJndW1lbnRzCiogYGVudmAgLSBUaGUgY29udHJhY3QgZW52aXJvbm1lbnQKKiBgYm91bnR5X2lkYCAtIFRoZSBib3VudHkgdG8gcXVlcnkKCiMgUmV0dXJucwoqIGBPayhWZWM8UmVmdW5kUmVjb3JkPilgIC0gVGhlIHJlZnVuZCBoaXN0b3J5CiogYEVycihFcnJvcjo6Qm91bnR5Tm90Rm91bmQpYCAtIEJvdW50eSBkb2Vzbid0IGV4aXN0AAAAABJnZXRfcmVmdW5kX2hpc3RvcnkAAAAAAAEAAAAAAAAACWJvdW50eV9pZAAAAAAAAAYAAAABAAAD6QAAA+oAAAfQAAAADFJlZnVuZFJlY29yZAAAAAM=",
        "AAAAAQAAAAAAAAAAAAAAEFJlbGVhc2VGdW5kc0l0ZW0AAAACAAAAAAAAAAlib3VudHlfaWQAAAAAAAAGAAAAAAAAAAtjb250cmlidXRvcgAAAAAT",
        "AAAAAAAAAlFCYXRjaCByZWxlYXNlIGZ1bmRzIHRvIG11bHRpcGxlIGNvbnRyaWJ1dG9ycyBpbiBhIHNpbmdsZSB0cmFuc2FjdGlvbi4KVGhpcyBpbXByb3ZlcyBnYXMgZWZmaWNpZW5jeSBieSByZWR1Y2luZyB0cmFuc2FjdGlvbiBvdmVyaGVhZC4KCiMgQXJndW1lbnRzCiogYGl0ZW1zYCAtIFZlY3RvciBvZiBSZWxlYXNlRnVuZHNJdGVtIGNvbnRhaW5pbmcgYm91bnR5X2lkIGFuZCBjb250cmlidXRvciBhZGRyZXNzCgojIFJldHVybnMKTnVtYmVyIG9mIHN1Y2Nlc3NmdWxseSByZWxlYXNlZCBib3VudGllcwoKIyBFcnJvcnMKKiBJbnZhbGlkQmF0Y2hTaXplIC0gaWYgYmF0Y2ggc2l6ZSBleGNlZWRzIE1BWF9CQVRDSF9TSVpFIG9yIGlzIHplcm8KKiBCb3VudHlOb3RGb3VuZCAtIGlmIGFueSBib3VudHlfaWQgZG9lc24ndCBleGlzdAoqIEZ1bmRzTm90TG9ja2VkIC0gaWYgYW55IGJvdW50eSBpcyBub3QgaW4gTG9ja2VkIHN0YXR1cwoqIFVuYXV0aG9yaXplZCAtIGlmIGNhbGxlciBpcyBub3QgYWRtaW4KCiMgTm90ZQpUaGlzIG9wZXJhdGlvbiBpcyBhdG9taWMgLSBpZiBhbnkgaXRlbSBmYWlscywgdGhlIGVudGlyZSB0cmFuc2FjdGlvbiByZXZlcnRzLgAAAAAAABNiYXRjaF9yZWxlYXNlX2Z1bmRzAAAAAAEAAAAAAAAABWl0ZW1zAAAAAAAD6gAAB9AAAAAQUmVsZWFzZUZ1bmRzSXRlbQAAAAEAAAPpAAAABAAAAAM=",
        "AAAAAAAAAcBHZXRzIHJlZnVuZCBlbGlnaWJpbGl0eSBpbmZvcm1hdGlvbiBmb3IgYSBib3VudHkuCgojIEFyZ3VtZW50cwoqIGBlbnZgIC0gVGhlIGNvbnRyYWN0IGVudmlyb25tZW50CiogYGJvdW50eV9pZGAgLSBUaGUgYm91bnR5IHRvIHF1ZXJ5CgojIFJldHVybnMKKiBgT2soKGJvb2wsIGJvb2wsIGkxMjgsIE9wdGlvbjxSZWZ1bmRBcHByb3ZhbD4pKWAgLSBUdXBsZSBjb250YWluaW5nOgotIGNhbl9yZWZ1bmQ6IFdoZXRoZXIgcmVmdW5kIGlzIHBvc3NpYmxlCi0gZGVhZGxpbmVfcGFzc2VkOiBXaGV0aGVyIHRoZSBkZWFkbGluZSBoYXMgcGFzc2VkCi0gcmVtYWluaW5nOiBSZW1haW5pbmcgYW1vdW50IGluIGVzY3JvdwotIGFwcHJvdmFsOiBPcHRpb25hbCByZWZ1bmQgYXBwcm92YWwgaWYgZXhpc3RzCiogYEVycihFcnJvcjo6Qm91bnR5Tm90Rm91bmQpYCAtIEJvdW50eSBkb2Vzbid0IGV4aXN0AAAAFmdldF9yZWZ1bmRfZWxpZ2liaWxpdHkAAAAAAAEAAAAAAAAACWJvdW50eV9pZAAAAAAAAAYAAAABAAAD6QAAA+0AAAAEAAAAAQAAAAEAAAALAAAD6AAAB9AAAAAOUmVmdW5kQXBwcm92YWwAAAAAAAM=",
        "AAAAAQAABABFdmVudCBlbWl0dGVkIHdoZW4gZnVuZHMgYXJlIGxvY2tlZCBpbiBlc2Nyb3cgZm9yIGEgYm91bnR5LgoKIyBGaWVsZHMKKiBgYm91bnR5X2lkYCAtIFVuaXF1ZSBpZGVudGlmaWVyIGZvciB0aGUgYm91bnR5CiogYGFtb3VudGAgLSBBbW91bnQgb2YgdG9rZW5zIGxvY2tlZCAoaW4gc3Ryb29wcyBmb3IgWExNKQoqIGBkZXBvc2l0b3JgIC0gQWRkcmVzcyB0aGF0IGRlcG9zaXRlZCB0aGUgZnVuZHMKKiBgZGVhZGxpbmVgIC0gVW5peCB0aW1lc3RhbXAgYWZ0ZXIgd2hpY2ggcmVmdW5kcyBhcmUgYWxsb3dlZAoKIyBFdmVudCBUb3BpYwpTeW1ib2w6IGBmX2xvY2tgCkluZGV4ZWQ6IGBib3VudHlfaWRgIChhbGxvd3MgZmlsdGVyaW5nIGJ5IHNwZWNpZmljIGJvdW50eSkKCiMgU3RhdGUgVHJhbnNpdGlvbgpgYGB0ZXh0Ck5PTkUg4oaSIExPQ0tFRApgYGAKCiMgVXNhZ2UKRW1pdHRlZCB3aGVuIGEgYm91bnR5IGNyZWF0b3IgbG9ja3MgZnVuZHMgZm9yIGEgdGFzay4gVGhlIGRlcG9zaXRvcgp0cmFuc2ZlcnMgdG9rZW5zIHRvIHRoZSBjb250cmFjdCwgd2hpY2ggaG9sZHMgdGhlbSB1bnRpbCByZWxlYXNlIG9yIHJlZnVuZC4KCiMgU2VjdXJpdHkgQ29uc2lkZXJhdGlvbnMKLSBBbW91bnQgbXVzdCBiZSBwb3NpdGl2ZSBhbmQgd2l0aGluIGRlcG9zaXRvcidzIGJhbGFuY2UKLSBCb3VudHkgSUQgbXVzdCBiZSB1bmlxdWUgKG5vIGR1cGxpY2F0ZXMgYWxsb3dlZCkKLSBEZWFkbGluZSBtdXN0IGJlIGluIHRoZSBmdXR1cmUKLSBEZXBvc2l0b3IgbXVzdCBhdXRob3JpemUgdGhlIHRyYW5zYWN0aW9uCgojIEV4YW1wbGUgVXNhZ2UKYGBgcnVzdAovLyBMb2NrIDEwMDAgWExNIGZvciBib3VudHkgIzQyLCBkZWFkbGluZSBpbiAzMCBkYXlzCmxldCBkZWFkbGluZSA9IGVudi5sZWRnZXIoKS50aW1lc3RhbXAoKSArICgzMCAqIDI0ICogNjAgKiA2MCk7CmVzY3Jvd19jbGllbnQubG9ja19mdW5kcygmZGVwb3NpdG9yLCAmNDIsICYxMF8wMDBfMDAwXzAwMCwgJmRlYWRsaW5lKTsKLy8g4oaSIEVtaXRzIEZ1bmRzTG9jAAAAAAAAAAtGdW5kc0xvY2tlZAAAAAAEAAAAAAAAAAZhbW91bnQAAAAAAAsAAAAAAAAACWJvdW50eV9pZAAAAAAAAAYAAAAAAAAACGRlYWRsaW5lAAAABgAAAAAAAAAJZGVwb3NpdG9yAAAAAAAAEw==",
        "AAAAAQAABABFdmVudCBlbWl0dGVkIHdoZW4gZXNjcm93ZWQgZnVuZHMgYXJlIHJlZnVuZGVkIHRvIHRoZSBkZXBvc2l0b3IuCgojIEZpZWxkcwoqIGBib3VudHlfaWRgIC0gVGhlIGJvdW50eSBpZGVudGlmaWVyCiogYGFtb3VudGAgLSBBbW91bnQgcmVmdW5kZWQgdG8gZGVwb3NpdG9yCiogYHJlZnVuZF90b2AgLSBBZGRyZXNzIHJlY2VpdmluZyB0aGUgcmVmdW5kIChvcmlnaW5hbCBkZXBvc2l0b3IpCiogYHRpbWVzdGFtcGAgLSBVbml4IHRpbWVzdGFtcCBvZiByZWZ1bmQKCiMgRXZlbnQgVG9waWMKU3ltYm9sOiBgZl9yZWZgCkluZGV4ZWQ6IGBib3VudHlfaWRgCgojIFN0YXRlIFRyYW5zaXRpb24KYGBgdGV4dApMT0NLRUQg4oaSIFJFRlVOREVEIChmaW5hbCBzdGF0ZSkKYGBgCgojIFVzYWdlCkVtaXR0ZWQgd2hlbiBmdW5kcyBhcmUgcmV0dXJuZWQgdG8gdGhlIGRlcG9zaXRvciBhZnRlciB0aGUgZGVhZGxpbmUKaGFzIHBhc3NlZCB3aXRob3V0IHRoZSBib3VudHkgYmVpbmcgY29tcGxldGVkLiBUaGlzIG1lY2hhbmlzbSBwcmV2ZW50cwpmdW5kcyBmcm9tIGJlaW5nIGxvY2tlZCBpbmRlZmluaXRlbHkuCgojIENvbmRpdGlvbnMKLSBEZWFkbGluZSBtdXN0IGhhdmUgcGFzc2VkICh0aW1lc3RhbXAgPiBkZWFkbGluZSkKLSBGdW5kcyBtdXN0IHN0aWxsIGJlIGluIExPQ0tFRCBzdGF0ZQotIENhbiBiZSB0cmlnZ2VyZWQgYnkgYW55b25lIChwZXJtaXNzaW9ubGVzcyBidXQgY29uZGl0aW9uYWwpCgojIFNlY3VyaXR5IENvbnNpZGVyYXRpb25zCi0gVGltZS1iYXNlZCBwcm90ZWN0aW9uIGVuc3VyZXMgZnVuZHMgYXJlbid0IHN0dWNrCi0gUGVybWlzc2lvbmxlc3MgcmVmdW5kIHByZXZlbnRzIGFkbWluIG1vbm9wb2x5Ci0gT3JpZ2luYWwgZGVwb3NpdG9yIGFsd2F5cyByZWNlaXZlcyByZWZ1bmQKLSBDYW5ub3QgcmVmdW5kIGlmIGFscmVhZHkgcmVsZWFzZWQgb3IgcmVmdW5kZWQKCiMgRXhhbXBsZSBVc2FnZQpgYGBydXN0Ci8vIEFmdGVyIGRlYWRsaW5lIHBhc3NlcywgYW55b25lIGNhbiB0cmlnZ2VyIHJlZnVuAAAAAAAAAA1GdW5kc1JlZnVuZGVkAAAAAAAABgAAAAAAAAAGYW1vdW50AAAAAAALAAAAAAAAAAlib3VudHlfaWQAAAAAAAAGAAAAAAAAAAtyZWZ1bmRfbW9kZQAAAAfQAAAAClJlZnVuZE1vZGUAAAAAAAAAAAAJcmVmdW5kX3RvAAAAAAAAEwAAAAAAAAAQcmVtYWluaW5nX2Ftb3VudAAAAAsAAAAAAAAACXRpbWVzdGFtcAAAAAAAAAY=",
        "AAAAAQAABABFdmVudCBlbWl0dGVkIHdoZW4gZXNjcm93ZWQgZnVuZHMgYXJlIHJlbGVhc2VkIHRvIGEgY29udHJpYnV0b3IuCgojIEZpZWxkcwoqIGBib3VudHlfaWRgIC0gVGhlIGJvdW50eSBpZGVudGlmaWVyCiogYGFtb3VudGAgLSBBbW91bnQgdHJhbnNmZXJyZWQgdG8gcmVjaXBpZW50CiogYHJlY2lwaWVudGAgLSBBZGRyZXNzIHJlY2VpdmluZyB0aGUgZnVuZHMgKGNvbnRyaWJ1dG9yKQoqIGB0aW1lc3RhbXBgIC0gVW5peCB0aW1lc3RhbXAgb2YgcmVsZWFzZQoKIyBFdmVudCBUb3BpYwpTeW1ib2w6IGBmX3JlbGAKSW5kZXhlZDogYGJvdW50eV9pZGAKCiMgU3RhdGUgVHJhbnNpdGlvbgpgYGB0ZXh0CkxPQ0tFRCDihpIgUkVMRUFTRUQgKGZpbmFsIHN0YXRlKQpgYGAKCiMgVXNhZ2UKRW1pdHRlZCB3aGVuIHRoZSBhZG1pbiByZWxlYXNlcyBmdW5kcyB0byBhIGNvbnRyaWJ1dG9yIHdobyBjb21wbGV0ZWQKdGhlIGJvdW50eSB0YXNrLiBUaGlzIGlzIGEgZmluYWwsIGlycmV2ZXJzaWJsZSBhY3Rpb24uCgojIEF1dGhvcml6YXRpb24KLSBPbmx5IHRoZSBjb250cmFjdCBhZG1pbiBjYW4gdHJpZ2dlciBmdW5kIHJlbGVhc2UKLSBGdW5kcyBtdXN0IGJlIGluIExPQ0tFRCBzdGF0ZQotIENhbm5vdCByZWxlYXNlIGZ1bmRzIHRoYXQgd2VyZSBhbHJlYWR5IHJlbGVhc2VkIG9yIHJlZnVuZGVkCgojIFNlY3VyaXR5IENvbnNpZGVyYXRpb25zCi0gQWRtaW4gYXV0aG9yaXphdGlvbiBpcyBjcml0aWNhbCAoc2hvdWxkIGJlIHNlY3VyZSBiYWNrZW5kKQotIFJlY2lwaWVudCBhZGRyZXNzIHNob3VsZCBiZSB2ZXJpZmllZCBvZmYtY2hhaW4gYmVmb3JlIHJlbGVhc2UKLSBPbmNlIHJlbGVhc2VkLCBmdW5kcyBjYW5ub3QgYmUgcmV0cmlldmVkCi0gQXRvbWljIG9wZXJhdGlvbjogdHJhbnNmZXIgKyBzdGF0ZSB1cGRhdGUKCiMgRXhhbXBsZSBVc2FnZQpgYGBydXN0Ci8vIEFkbWluIHJlbGVhc2VzIDEwMDAgWExNIHRvIGNvbnRyaWJ1dG9yIGZvciBib3VudHkgIzQyCmVzY3Jvd19jbGllbnQucmVsZWFzZV9mdW5kcygmNDIsAAAAAAAAAA1GdW5kc1JlbGVhc2VkAAAAAAAABAAAAAAAAAAGYW1vdW50AAAAAAALAAAAAAAAAAlib3VudHlfaWQAAAAAAAAGAAAAAAAAAAlyZWNpcGllbnQAAAAAAAATAAAAAAAAAAl0aW1lc3RhbXAAAAAAAAAG",
        "AAAAAQAAAAAAAAAAAAAAEEJhdGNoRnVuZHNMb2NrZWQAAAADAAAAAAAAAAVjb3VudAAAAAAAAAQAAAAAAAAACXRpbWVzdGFtcAAAAAAAAAYAAAAAAAAADHRvdGFsX2Ftb3VudAAAAAs=",
        "AAAAAQAAAAAAAAAAAAAAEkJhdGNoRnVuZHNSZWxlYXNlZAAAAAAAAwAAAAAAAAAFY291bnQAAAAAAAAEAAAAAAAAAAl0aW1lc3RhbXAAAAAAAAAGAAAAAAAAAAx0b3RhbF9hbW91bnQAAAAL",
        "AAAAAQAAAzRFdmVudCBlbWl0dGVkIHdoZW4gdGhlIEJvdW50eSBFc2Nyb3cgY29udHJhY3QgaXMgaW5pdGlhbGl6ZWQuCgojIEZpZWxkcwoqIGBhZG1pbmAgLSBUaGUgYWRtaW5pc3RyYXRvciBhZGRyZXNzIHdpdGggcmVsZWFzZSBhdXRob3JpemF0aW9uCiogYHRva2VuYCAtIFRoZSB0b2tlbiBjb250cmFjdCBhZGRyZXNzICh0eXBpY2FsbHkgWExNL1VTREMpCiogYHRpbWVzdGFtcGAgLSBVbml4IHRpbWVzdGFtcCBvZiBpbml0aWFsaXphdGlvbgoKIyBFdmVudCBUb3BpYwpTeW1ib2w6IGBpbml0YAoKIyBVc2FnZQpUaGlzIGV2ZW50IGlzIGVtaXR0ZWQgb25jZSBkdXJpbmcgY29udHJhY3QgZGVwbG95bWVudCBhbmQgc2lnbmFscwp0aGF0IHRoZSBjb250cmFjdCBpcyByZWFkeSB0byBhY2NlcHQgYm91bnR5IGVzY3Jvd3MuCgojIFNlY3VyaXR5IENvbnNpZGVyYXRpb25zCi0gT25seSBlbWl0dGVkIG9uY2U7IHN1YnNlcXVlbnQgaW5pdCBhdHRlbXB0cyBzaG91bGQgZmFpbAotIEFkbWluIGFkZHJlc3Mgc2hvdWxkIGJlIGEgc2VjdXJlIGJhY2tlbmQgc2VydmljZQotIFRva2VuIGFkZHJlc3MgbXVzdCBiZSBhIHZhbGlkIFN0ZWxsYXIgdG9rZW4gY29udHJhY3QKCiMgRXhhbXBsZSBPZmYtY2hhaW4gSW5kZXhpbmcKYGBgamF2YXNjcmlwdAovLyBMaXN0ZW4gZm9yIGluaXRpYWxpemF0aW9uIGV2ZW50cwpzdGVsbGFyLmV2ZW50cy5vbignaW5pdCcsIChldmVudCkgPT4gewpjb25zb2xlLmxvZyhgQ29udHJhY3QgaW5pdGlhbGl6ZWQgYnkgJHtldmVudC5hZG1pbn1gKTsKY29uc29sZS5sb2coYFVzaW5nIHRva2VuOiAke2V2ZW50LnRva2VufWApOwp9KTsKYGBgAAAAAAAAABdCb3VudHlFc2Nyb3dJbml0aWFsaXplZAAAAAADAAAAAAAAAAVhZG1pbgAAAAAAABMAAAAAAAAACXRpbWVzdGFtcAAAAAAAAAYAAAAAAAAABXRva2VuAAAAAAAAEw==" ]),
      options
    )
  }
}