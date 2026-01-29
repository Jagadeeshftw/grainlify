import { Buffer } from "buffer";
import { AssembledTransaction, Client as ContractClient, Ok, Err } from "@stellar/stellar-sdk/contract";
import type { u32, u64, i128, Option } from "@stellar/stellar-sdk/contract";
export type Result<T> = Ok<T> | Err<any>;
export * from "@stellar/stellar-sdk";
export * as contract from "@stellar/stellar-sdk/contract";
export * as rpc from "@stellar/stellar-sdk/rpc";
export interface AddressState {
    last_operation_timestamp: u64;
    operation_count: u32;
    window_start_timestamp: u64;
}
export type AntiAbuseKey = {
    tag: "Config";
    values: void;
} | {
    tag: "State";
    values: readonly [string];
} | {
    tag: "Whitelist";
    values: readonly [string];
} | {
    tag: "Admin";
    values: void;
};
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
export declare const Errors: {
    /**
     * Returned when attempting to initialize an already initialized contract
     */
    1: {
        message: string;
    };
    /**
     * Returned when calling contract functions before initialization
     */
    2: {
        message: string;
    };
    /**
     * Returned when attempting to lock funds with a duplicate bounty ID
     */
    3: {
        message: string;
    };
    /**
     * Returned when querying or operating on a non-existent bounty
     */
    4: {
        message: string;
    };
    /**
     * Returned when attempting operations on non-LOCKED funds
     */
    5: {
        message: string;
    };
    /**
     * Returned when attempting refund before the deadline has passed
     */
    6: {
        message: string;
    };
    /**
     * Returned when caller lacks required authorization for the operation
     */
    7: {
        message: string;
    };
    /**
     * Returned when amount is invalid (zero, negative, or exceeds available)
     */
    8: {
        message: string;
    };
    /**
     * Returned when deadline is invalid (in the past or too far in the future)
     */
    9: {
        message: string;
    };
    10: {
        message: string;
    };
    11: {
        message: string;
    };
    /**
     * Returned when contract has insufficient funds for the operation
     */
    12: {
        message: string;
    };
    /**
     * Returned when refund is attempted without admin approval
     */
    13: {
        message: string;
    };
};
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
export type DataKey = {
    tag: "Admin";
    values: void;
} | {
    tag: "Token";
    values: void;
} | {
    tag: "Escrow";
    values: readonly [u64];
} | {
    tag: "RefundApproval";
    values: readonly [u64];
} | {
    tag: "ReentrancyGuard";
    values: void;
};
export type RefundMode = {
    tag: "Full";
    values: void;
} | {
    tag: "Partial";
    values: void;
} | {
    tag: "Custom";
    values: void;
};
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
export type EscrowStatus = {
    tag: "Locked";
    values: void;
} | {
    tag: "Released";
    values: void;
} | {
    tag: "Refunded";
    values: void;
} | {
    tag: "PartiallyRefunded";
    values: void;
};
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
    init: ({ admin, token }: {
        admin: string;
        token: string;
    }, options?: any) => Promise<AssembledTransaction<Result<void>>>;
    /**
     * Construct and simulate a refund transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
     * Refund funds with support for Full, Partial, and Custom refunds.
     * - Full: refunds all remaining funds to depositor
     * - Partial: refunds specified amount to depositor
     * - Custom: refunds specified amount to specified recipient (requires admin approval if before deadline)
     */
    refund: ({ bounty_id, amount, recipient, mode }: {
        bounty_id: u64;
        amount: Option<i128>;
        recipient: Option<string>;
        mode: RefundMode;
    }, options?: any) => Promise<AssembledTransaction<Result<void>>>;
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
    lock_funds: ({ depositor, bounty_id, amount, deadline }: {
        depositor: string;
        bounty_id: u64;
        amount: i128;
        deadline: u64;
    }, options?: any) => Promise<AssembledTransaction<Result<void>>>;
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
    get_balance: (options?: any) => Promise<AssembledTransaction<Result<i128>>>;
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
    release_funds: ({ bounty_id, contributor }: {
        bounty_id: u64;
        contributor: string;
    }, options?: any) => Promise<AssembledTransaction<Result<void>>>;
    /**
     * Construct and simulate a approve_refund transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
     * Approve a refund before deadline (admin only).
     * This allows early refunds with admin approval.
     */
    approve_refund: ({ bounty_id, amount, recipient, mode }: {
        bounty_id: u64;
        amount: i128;
        recipient: string;
        mode: RefundMode;
    }, options?: any) => Promise<AssembledTransaction<Result<void>>>;
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
    get_escrow_info: ({ bounty_id }: {
        bounty_id: u64;
    }, options?: any) => Promise<AssembledTransaction<Result<Escrow>>>;
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
    batch_lock_funds: ({ items }: {
        items: Array<LockFundsItem>;
    }, options?: any) => Promise<AssembledTransaction<Result<u32>>>;
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
    get_refund_history: ({ bounty_id }: {
        bounty_id: u64;
    }, options?: any) => Promise<AssembledTransaction<Result<Array<RefundRecord>>>>;
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
    batch_release_funds: ({ items }: {
        items: Array<ReleaseFundsItem>;
    }, options?: any) => Promise<AssembledTransaction<Result<u32>>>;
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
    get_refund_eligibility: ({ bounty_id }: {
        bounty_id: u64;
    }, options?: any) => Promise<AssembledTransaction<Result<readonly [boolean, boolean, i128, Option<RefundApproval>]>>>;
}
export declare class Client extends ContractClient {
    static deploy<T = Client>(
    /** Options for initializing a Client as well as for calling a method, with extras specific to deploying. */
    options: any & Omit<any, "contractId"> & {
        /** The hash of the Wasm blob, which must already be installed on-chain. */
        wasmHash: Buffer | string;
        /** Salt used to generate the contract's ID. Passed through to {@link Operation.createCustomContract}. Default: random. */
        salt?: Buffer | Uint8Array;
        /** The format used to decode `wasmHash`, if it's provided as a string. */
        format?: "hex" | "base64";
    }): Promise<AssembledTransaction<T>>;
    constructor(options: any);
}
