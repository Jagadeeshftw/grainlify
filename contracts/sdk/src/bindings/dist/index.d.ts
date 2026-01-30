import { Buffer } from "buffer";
import { AssembledTransaction, Client as ContractClient } from "@stellar/stellar-sdk/contract";
import type { u32, u64 } from "@stellar/stellar-sdk/contract";
export * from "@stellar/stellar-sdk";
export * as contract from "@stellar/stellar-sdk/contract";
export * as rpc from "@stellar/stellar-sdk/rpc";
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
/**
 * =======================
 * Proposal Structure
 * =======================
 */
export interface Proposal {
    approvals: Array<string>;
    executed: boolean;
}
/**
 * =======================
 * Multisig Configuration
 * =======================
 */
export interface MultiSigConfig {
    signers: Array<string>;
    threshold: u32;
}
export interface Client {
    /**
     * Construct and simulate a init transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
     * Initializes the contract with multisig configuration.
     *
     * # Arguments
     * * `env` - The contract environment
     * * `signers` - List of signer addresses for multisig
     * * `threshold` - Number of signatures required to execute proposals
     */
    init: ({ signers, threshold }: {
        signers: Array<string>;
        threshold: u32;
    }, options?: any) => Promise<AssembledTransaction<null>>;
    /**
     * Construct and simulate a upgrade transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
     * Upgrades the contract to new WASM code (single admin version).
     *
     * # Arguments
     * * `env` - The contract environment
     * * `new_wasm_hash` - Hash of the uploaded WASM code (32 bytes)
     */
    upgrade: ({ new_wasm_hash }: {
        new_wasm_hash: Buffer;
    }, options?: any) => Promise<AssembledTransaction<null>>;
    /**
     * Construct and simulate a init_admin transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
     * Initializes the contract with a single admin address.
     *
     * # Arguments
     * * `env` - The contract environment
     * * `admin` - Address authorized to perform upgrades
     */
    init_admin: ({ admin }: {
        admin: string;
    }, options?: any) => Promise<AssembledTransaction<null>>;
    /**
     * Construct and simulate a get_version transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
     * Retrieves the current contract version number.
     *
     * # Arguments
     * * `env` - The contract environment
     *
     * # Returns
     * * `u32` - Current version number (defaults to 0 if not set)
     *
     * # Usage
     * Use this to verify contract version for:
     * - Client compatibility checks
     * - Migration decision logic
     * - Audit trails
     * - Version-specific behavior
     *
     * # Example
     * ```rust
     * let version = contract.get_version(&env);
     *
     * match version {
     * 1 => println!("Running v1"),
     * 2 => println!("Running v2 with new features"),
     * _ => println!("Unknown version"),
     * }
     * ```
     *
     * # Client-Side Usage
     * ```javascript
     * // Check contract version before interaction
     * const version = await contract.get_version();
     *
     * if (version < 2) {
     * throw new Error("Contract version too old, please upgrade");
     * }
     * ```
     *
     * # Gas Cost
     * Very Low - Single storage read
     */
    get_version: (options?: any) => Promise<AssembledTransaction<u32>>;
    /**
     * Construct and simulate a set_version transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
     * Updates the contract version number.
     *
     * # Arguments
     * * `env` - The contract environment
     * * `new_version` - New version number to set
     *
     * # Authorization
     * - Only admin can call this function
     * - Admin must sign the transaction
     *
     * # State Changes
     * - Updates Version in instance storage
     *
     * # Usage
     * Call this function after upgrading contract WASM to reflect
     * the new version number. This provides an audit trail of upgrades.
     *
     * # Version Numbering Strategy
     * Recommend using semantic versioning encoded as single u32:
     * - `1` = v1.0.0
     * - `2` = v2.0.0
     * - `101` = v1.0.1 (patch)
     * - `110` = v1.1.0 (minor)
     *
     * Or use simple incrementing:
     * - `1` = First version
     * - `2` = Second version
     * - `3` = Third version
     *
     * # Example
     * ```rust
     * // After upgrading WASM
     * contract.upgrade(&env, &new_wasm_hash);
     *
     * // Update version to reflect the upgrade
     * contract.set_version(&env, &2);
     *
     * // Verify
     * assert_eq!(contract.get_version(&env), 2);
     * ```
     *
     * # Best Practice
     * Document version changes:
     * ```rust
     * // Version History:
     * // 1 - Initial release
     * // 2 - Added feature X, fixed bug Y
     * // 3 - P
     */
    set_version: ({ new_version }: {
        new_version: u32;
    }, options?: any) => Promise<AssembledTransaction<null>>;
    /**
     * Construct and simulate a health_check transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
     * Health check - returns contract health status
     */
    health_check: (options?: any) => Promise<AssembledTransaction<HealthStatus>>;
    /**
     * Construct and simulate a get_analytics transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
     * Get analytics - returns usage analytics
     */
    get_analytics: (options?: any) => Promise<AssembledTransaction<Analytics>>;
    /**
     * Construct and simulate a approve_upgrade transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
     * Approves an upgrade proposal (multisig version).
     *
     * # Arguments
     * * `env` - The contract environment
     * * `proposal_id` - The ID of the proposal to approve
     * * `signer` - Address approving the proposal
     */
    approve_upgrade: ({ proposal_id, signer }: {
        proposal_id: u64;
        signer: string;
    }, options?: any) => Promise<AssembledTransaction<null>>;
    /**
     * Construct and simulate a execute_upgrade transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
     * Upgrades the contract to new WASM code.
     *
     * # Arguments
     * * `env` - The contract environment
     * * `new_wasm_hash` - Hash of the uploaded WASM code (32 bytes)
     *
     * # Authorization
     * - **CRITICAL**: Only admin can call this function
     * - Admin must sign the transaction
     *
     * # State Changes
     * - Replaces current contract WASM with new version
     * - Preserves all instance storage (admin, version, etc.)
     * - Does NOT automatically update version number (call `set_version` separately)
     *
     * # Security Considerations
     * - **Code Review**: New WASM must be audited before deployment
     * - **Testing**: Test upgrade on testnet first
     * - **State Compatibility**: Ensure new code is compatible with existing state
     * - **Rollback Plan**: Keep previous WASM hash for emergency rollback
     * - **Version Update**: Call `set_version` after upgrade if needed
     *
     * # Workflow
     * 1. Develop and test new contract version
     * 2. Build WASM: `cargo build --release --target wasm32-unknown-unknown`
     * 3. Upload WASM to Stellar network
     * 4. Get WASM hash from upload response
     * 5. Call this function with the
     */
    execute_upgrade: ({ proposal_id }: {
        proposal_id: u64;
    }, options?: any) => Promise<AssembledTransaction<null>>;
    /**
     * Construct and simulate a propose_upgrade transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
     * Proposes an upgrade with a new WASM hash (multisig version).
     *
     * # Arguments
     * * `env` - The contract environment
     * * `proposer` - Address proposing the upgrade
     * * `wasm_hash` - Hash of the new WASM code
     *
     * # Returns
     * * `u64` - The proposal ID
     */
    propose_upgrade: ({ proposer, wasm_hash }: {
        proposer: string;
        wasm_hash: Buffer;
    }, options?: any) => Promise<AssembledTransaction<u64>>;
    /**
     * Construct and simulate a get_state_snapshot transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
     * Get state snapshot - returns current state
     */
    get_state_snapshot: (options?: any) => Promise<AssembledTransaction<StateSnapshot>>;
    /**
     * Construct and simulate a get_performance_stats transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
     * Get performance stats for a function
     */
    get_performance_stats: ({ function_name }: {
        function_name: string;
    }, options?: any) => Promise<AssembledTransaction<PerformanceStats>>;
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
