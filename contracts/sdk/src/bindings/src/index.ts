import { Buffer } from "buffer";
import { Address } from "@stellar/stellar-sdk";
import {
  AssembledTransaction,
  Client as ContractClient,
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
export * from "@stellar/stellar-sdk";
export * as contract from "@stellar/stellar-sdk/contract";
export * as rpc from "@stellar/stellar-sdk/rpc";

if (typeof window !== "undefined") {
  //@ts-ignore Buffer exists
  window.Buffer = window.Buffer || Buffer;
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
  init: ({signers, threshold}: {signers: Array<string>, threshold: u32}, options?: any) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a upgrade transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Upgrades the contract to new WASM code (single admin version).
   * 
   * # Arguments
   * * `env` - The contract environment
   * * `new_wasm_hash` - Hash of the uploaded WASM code (32 bytes)
   */
  upgrade: ({new_wasm_hash}: {new_wasm_hash: Buffer}, options?: any) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a init_admin transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Initializes the contract with a single admin address.
   * 
   * # Arguments
   * * `env` - The contract environment
   * * `admin` - Address authorized to perform upgrades
   */
  init_admin: ({admin}: {admin: string}, options?: any) => Promise<AssembledTransaction<null>>

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
  get_version: (options?: any) => Promise<AssembledTransaction<u32>>

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
  set_version: ({new_version}: {new_version: u32}, options?: any) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a health_check transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Health check - returns contract health status
   */
  health_check: (options?: any) => Promise<AssembledTransaction<HealthStatus>>

  /**
   * Construct and simulate a get_analytics transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Get analytics - returns usage analytics
   */
  get_analytics: (options?: any) => Promise<AssembledTransaction<Analytics>>

  /**
   * Construct and simulate a approve_upgrade transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Approves an upgrade proposal (multisig version).
   * 
   * # Arguments
   * * `env` - The contract environment
   * * `proposal_id` - The ID of the proposal to approve
   * * `signer` - Address approving the proposal
   */
  approve_upgrade: ({proposal_id, signer}: {proposal_id: u64, signer: string}, options?: any) => Promise<AssembledTransaction<null>>

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
  execute_upgrade: ({proposal_id}: {proposal_id: u64}, options?: any) => Promise<AssembledTransaction<null>>

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
  propose_upgrade: ({proposer, wasm_hash}: {proposer: string, wasm_hash: Buffer}, options?: any) => Promise<AssembledTransaction<u64>>

  /**
   * Construct and simulate a get_state_snapshot transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Get state snapshot - returns current state
   */
  get_state_snapshot: (options?: any) => Promise<AssembledTransaction<StateSnapshot>>

  /**
   * Construct and simulate a get_performance_stats transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Get performance stats for a function
   */
  get_performance_stats: ({function_name}: {function_name: string}, options?: any) => Promise<AssembledTransaction<PerformanceStats>>

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
      new ContractSpec([ "AAAAAQAAAAAAAAAAAAAACUFuYWx5dGljcwAAAAAAAAQAAAAAAAAAC2Vycm9yX2NvdW50AAAAAAYAAAAAAAAACmVycm9yX3JhdGUAAAAAAAQAAAAAAAAAD29wZXJhdGlvbl9jb3VudAAAAAAGAAAAAAAAAAx1bmlxdWVfdXNlcnMAAAAG",
        "AAAAAQAAAAAAAAAAAAAADEhlYWx0aFN0YXR1cwAAAAQAAAAAAAAAEGNvbnRyYWN0X3ZlcnNpb24AAAAQAAAAAAAAAAppc19oZWFsdGh5AAAAAAABAAAAAAAAAA5sYXN0X29wZXJhdGlvbgAAAAAABgAAAAAAAAAQdG90YWxfb3BlcmF0aW9ucwAAAAY=",
        "AAAAAQAAAAAAAAAAAAAADVN0YXRlU25hcHNob3QAAAAAAAAEAAAAAAAAAAl0aW1lc3RhbXAAAAAAAAAGAAAAAAAAAAx0b3RhbF9lcnJvcnMAAAAGAAAAAAAAABB0b3RhbF9vcGVyYXRpb25zAAAABgAAAAAAAAALdG90YWxfdXNlcnMAAAAABg==",
        "AAAAAQAAAAAAAAAAAAAAD09wZXJhdGlvbk1ldHJpYwAAAAAEAAAAAAAAAAZjYWxsZXIAAAAAABMAAAAAAAAACW9wZXJhdGlvbgAAAAAAABEAAAAAAAAAB3N1Y2Nlc3MAAAAAAQAAAAAAAAAJdGltZXN0YW1wAAAAAAAABg==",
        "AAAAAQAAAAAAAAAAAAAAEFBlcmZvcm1hbmNlU3RhdHMAAAAFAAAAAAAAAAhhdmdfdGltZQAAAAYAAAAAAAAACmNhbGxfY291bnQAAAAAAAYAAAAAAAAADWZ1bmN0aW9uX25hbWUAAAAAAAARAAAAAAAAAAtsYXN0X2NhbGxlZAAAAAAGAAAAAAAAAAp0b3RhbF90aW1lAAAAAAAG",
        "AAAAAQAAAAAAAAAAAAAAEVBlcmZvcm1hbmNlTWV0cmljAAAAAAAAAwAAAAAAAAAIZHVyYXRpb24AAAAGAAAAAAAAAAhmdW5jdGlvbgAAABEAAAAAAAAACXRpbWVzdGFtcAAAAAAAAAY=",
        "AAAAAAAAANxJbml0aWFsaXplcyB0aGUgY29udHJhY3Qgd2l0aCBtdWx0aXNpZyBjb25maWd1cmF0aW9uLgoKIyBBcmd1bWVudHMKKiBgZW52YCAtIFRoZSBjb250cmFjdCBlbnZpcm9ubWVudAoqIGBzaWduZXJzYCAtIExpc3Qgb2Ygc2lnbmVyIGFkZHJlc3NlcyBmb3IgbXVsdGlzaWcKKiBgdGhyZXNob2xkYCAtIE51bWJlciBvZiBzaWduYXR1cmVzIHJlcXVpcmVkIHRvIGV4ZWN1dGUgcHJvcG9zYWxzAAAABGluaXQAAAACAAAAAAAAAAdzaWduZXJzAAAAA+oAAAATAAAAAAAAAAl0aHJlc2hvbGQAAAAAAAAEAAAAAA==",
        "AAAAAAAAAKxVcGdyYWRlcyB0aGUgY29udHJhY3QgdG8gbmV3IFdBU00gY29kZSAoc2luZ2xlIGFkbWluIHZlcnNpb24pLgoKIyBBcmd1bWVudHMKKiBgZW52YCAtIFRoZSBjb250cmFjdCBlbnZpcm9ubWVudAoqIGBuZXdfd2FzbV9oYXNoYCAtIEhhc2ggb2YgdGhlIHVwbG9hZGVkIFdBU00gY29kZSAoMzIgYnl0ZXMpAAAAB3VwZ3JhZGUAAAAAAQAAAAAAAAANbmV3X3dhc21faGFzaAAAAAAAA+4AAAAgAAAAAA==",
        "AAAAAAAAAJhJbml0aWFsaXplcyB0aGUgY29udHJhY3Qgd2l0aCBhIHNpbmdsZSBhZG1pbiBhZGRyZXNzLgoKIyBBcmd1bWVudHMKKiBgZW52YCAtIFRoZSBjb250cmFjdCBlbnZpcm9ubWVudAoqIGBhZG1pbmAgLSBBZGRyZXNzIGF1dGhvcml6ZWQgdG8gcGVyZm9ybSB1cGdyYWRlcwAAAAppbml0X2FkbWluAAAAAAABAAAAAAAAAAVhZG1pbgAAAAAAABMAAAAA",
        "AAAAAAAAAv1SZXRyaWV2ZXMgdGhlIGN1cnJlbnQgY29udHJhY3QgdmVyc2lvbiBudW1iZXIuCgojIEFyZ3VtZW50cwoqIGBlbnZgIC0gVGhlIGNvbnRyYWN0IGVudmlyb25tZW50CgojIFJldHVybnMKKiBgdTMyYCAtIEN1cnJlbnQgdmVyc2lvbiBudW1iZXIgKGRlZmF1bHRzIHRvIDAgaWYgbm90IHNldCkKCiMgVXNhZ2UKVXNlIHRoaXMgdG8gdmVyaWZ5IGNvbnRyYWN0IHZlcnNpb24gZm9yOgotIENsaWVudCBjb21wYXRpYmlsaXR5IGNoZWNrcwotIE1pZ3JhdGlvbiBkZWNpc2lvbiBsb2dpYwotIEF1ZGl0IHRyYWlscwotIFZlcnNpb24tc3BlY2lmaWMgYmVoYXZpb3IKCiMgRXhhbXBsZQpgYGBydXN0CmxldCB2ZXJzaW9uID0gY29udHJhY3QuZ2V0X3ZlcnNpb24oJmVudik7CgptYXRjaCB2ZXJzaW9uIHsKMSA9PiBwcmludGxuISgiUnVubmluZyB2MSIpLAoyID0+IHByaW50bG4hKCJSdW5uaW5nIHYyIHdpdGggbmV3IGZlYXR1cmVzIiksCl8gPT4gcHJpbnRsbiEoIlVua25vd24gdmVyc2lvbiIpLAp9CmBgYAoKIyBDbGllbnQtU2lkZSBVc2FnZQpgYGBqYXZhc2NyaXB0Ci8vIENoZWNrIGNvbnRyYWN0IHZlcnNpb24gYmVmb3JlIGludGVyYWN0aW9uCmNvbnN0IHZlcnNpb24gPSBhd2FpdCBjb250cmFjdC5nZXRfdmVyc2lvbigpOwoKaWYgKHZlcnNpb24gPCAyKSB7CnRocm93IG5ldyBFcnJvcigiQ29udHJhY3QgdmVyc2lvbiB0b28gb2xkLCBwbGVhc2UgdXBncmFkZSIpOwp9CmBgYAoKIyBHYXMgQ29zdApWZXJ5IExvdyAtIFNpbmdsZSBzdG9yYWdlIHJlYWQAAAAAAAALZ2V0X3ZlcnNpb24AAAAAAAAAAAEAAAAE",
        "AAAAAAAABABVcGRhdGVzIHRoZSBjb250cmFjdCB2ZXJzaW9uIG51bWJlci4KCiMgQXJndW1lbnRzCiogYGVudmAgLSBUaGUgY29udHJhY3QgZW52aXJvbm1lbnQKKiBgbmV3X3ZlcnNpb25gIC0gTmV3IHZlcnNpb24gbnVtYmVyIHRvIHNldAoKIyBBdXRob3JpemF0aW9uCi0gT25seSBhZG1pbiBjYW4gY2FsbCB0aGlzIGZ1bmN0aW9uCi0gQWRtaW4gbXVzdCBzaWduIHRoZSB0cmFuc2FjdGlvbgoKIyBTdGF0ZSBDaGFuZ2VzCi0gVXBkYXRlcyBWZXJzaW9uIGluIGluc3RhbmNlIHN0b3JhZ2UKCiMgVXNhZ2UKQ2FsbCB0aGlzIGZ1bmN0aW9uIGFmdGVyIHVwZ3JhZGluZyBjb250cmFjdCBXQVNNIHRvIHJlZmxlY3QKdGhlIG5ldyB2ZXJzaW9uIG51bWJlci4gVGhpcyBwcm92aWRlcyBhbiBhdWRpdCB0cmFpbCBvZiB1cGdyYWRlcy4KCiMgVmVyc2lvbiBOdW1iZXJpbmcgU3RyYXRlZ3kKUmVjb21tZW5kIHVzaW5nIHNlbWFudGljIHZlcnNpb25pbmcgZW5jb2RlZCBhcyBzaW5nbGUgdTMyOgotIGAxYCA9IHYxLjAuMAotIGAyYCA9IHYyLjAuMAotIGAxMDFgID0gdjEuMC4xIChwYXRjaCkKLSBgMTEwYCA9IHYxLjEuMCAobWlub3IpCgpPciB1c2Ugc2ltcGxlIGluY3JlbWVudGluZzoKLSBgMWAgPSBGaXJzdCB2ZXJzaW9uCi0gYDJgID0gU2Vjb25kIHZlcnNpb24KLSBgM2AgPSBUaGlyZCB2ZXJzaW9uCgojIEV4YW1wbGUKYGBgcnVzdAovLyBBZnRlciB1cGdyYWRpbmcgV0FTTQpjb250cmFjdC51cGdyYWRlKCZlbnYsICZuZXdfd2FzbV9oYXNoKTsKCi8vIFVwZGF0ZSB2ZXJzaW9uIHRvIHJlZmxlY3QgdGhlIHVwZ3JhZGUKY29udHJhY3Quc2V0X3ZlcnNpb24oJmVudiwgJjIpOwoKLy8gVmVyaWZ5CmFzc2VydF9lcSEoY29udHJhY3QuZ2V0X3ZlcnNpb24oJmVudiksIDIpOwpgYGAKCiMgQmVzdCBQcmFjdGljZQpEb2N1bWVudCB2ZXJzaW9uIGNoYW5nZXM6CmBgYHJ1c3QKLy8gVmVyc2lvbiBIaXN0b3J5OgovLyAxIC0gSW5pdGlhbCByZWxlYXNlCi8vIDIgLSBBZGRlZCBmZWF0dXJlIFgsIGZpeGVkIGJ1ZyBZCi8vIDMgLSBQAAAAC3NldF92ZXJzaW9uAAAAAAEAAAAAAAAAC25ld192ZXJzaW9uAAAAAAQAAAAA",
        "AAAAAAAAAC1IZWFsdGggY2hlY2sgLSByZXR1cm5zIGNvbnRyYWN0IGhlYWx0aCBzdGF0dXMAAAAAAAAMaGVhbHRoX2NoZWNrAAAAAAAAAAEAAAfQAAAADEhlYWx0aFN0YXR1cw==",
        "AAAAAAAAACdHZXQgYW5hbHl0aWNzIC0gcmV0dXJucyB1c2FnZSBhbmFseXRpY3MAAAAADWdldF9hbmFseXRpY3MAAAAAAAAAAAAAAQAAB9AAAAAJQW5hbHl0aWNzAAAA",
        "AAAAAAAAAMBBcHByb3ZlcyBhbiB1cGdyYWRlIHByb3Bvc2FsIChtdWx0aXNpZyB2ZXJzaW9uKS4KCiMgQXJndW1lbnRzCiogYGVudmAgLSBUaGUgY29udHJhY3QgZW52aXJvbm1lbnQKKiBgcHJvcG9zYWxfaWRgIC0gVGhlIElEIG9mIHRoZSBwcm9wb3NhbCB0byBhcHByb3ZlCiogYHNpZ25lcmAgLSBBZGRyZXNzIGFwcHJvdmluZyB0aGUgcHJvcG9zYWwAAAAPYXBwcm92ZV91cGdyYWRlAAAAAAIAAAAAAAAAC3Byb3Bvc2FsX2lkAAAAAAYAAAAAAAAABnNpZ25lcgAAAAAAEwAAAAA=",
        "AAAAAAAABABVcGdyYWRlcyB0aGUgY29udHJhY3QgdG8gbmV3IFdBU00gY29kZS4KCiMgQXJndW1lbnRzCiogYGVudmAgLSBUaGUgY29udHJhY3QgZW52aXJvbm1lbnQKKiBgbmV3X3dhc21faGFzaGAgLSBIYXNoIG9mIHRoZSB1cGxvYWRlZCBXQVNNIGNvZGUgKDMyIGJ5dGVzKQoKIyBBdXRob3JpemF0aW9uCi0gKipDUklUSUNBTCoqOiBPbmx5IGFkbWluIGNhbiBjYWxsIHRoaXMgZnVuY3Rpb24KLSBBZG1pbiBtdXN0IHNpZ24gdGhlIHRyYW5zYWN0aW9uCgojIFN0YXRlIENoYW5nZXMKLSBSZXBsYWNlcyBjdXJyZW50IGNvbnRyYWN0IFdBU00gd2l0aCBuZXcgdmVyc2lvbgotIFByZXNlcnZlcyBhbGwgaW5zdGFuY2Ugc3RvcmFnZSAoYWRtaW4sIHZlcnNpb24sIGV0Yy4pCi0gRG9lcyBOT1QgYXV0b21hdGljYWxseSB1cGRhdGUgdmVyc2lvbiBudW1iZXIgKGNhbGwgYHNldF92ZXJzaW9uYCBzZXBhcmF0ZWx5KQoKIyBTZWN1cml0eSBDb25zaWRlcmF0aW9ucwotICoqQ29kZSBSZXZpZXcqKjogTmV3IFdBU00gbXVzdCBiZSBhdWRpdGVkIGJlZm9yZSBkZXBsb3ltZW50Ci0gKipUZXN0aW5nKio6IFRlc3QgdXBncmFkZSBvbiB0ZXN0bmV0IGZpcnN0Ci0gKipTdGF0ZSBDb21wYXRpYmlsaXR5Kio6IEVuc3VyZSBuZXcgY29kZSBpcyBjb21wYXRpYmxlIHdpdGggZXhpc3Rpbmcgc3RhdGUKLSAqKlJvbGxiYWNrIFBsYW4qKjogS2VlcCBwcmV2aW91cyBXQVNNIGhhc2ggZm9yIGVtZXJnZW5jeSByb2xsYmFjawotICoqVmVyc2lvbiBVcGRhdGUqKjogQ2FsbCBgc2V0X3ZlcnNpb25gIGFmdGVyIHVwZ3JhZGUgaWYgbmVlZGVkCgojIFdvcmtmbG93CjEuIERldmVsb3AgYW5kIHRlc3QgbmV3IGNvbnRyYWN0IHZlcnNpb24KMi4gQnVpbGQgV0FTTTogYGNhcmdvIGJ1aWxkIC0tcmVsZWFzZSAtLXRhcmdldCB3YXNtMzItdW5rbm93bi11bmtub3duYAozLiBVcGxvYWQgV0FTTSB0byBTdGVsbGFyIG5ldHdvcmsKNC4gR2V0IFdBU00gaGFzaCBmcm9tIHVwbG9hZCByZXNwb25zZQo1LiBDYWxsIHRoaXMgZnVuY3Rpb24gd2l0aCB0aGUgAAAAD2V4ZWN1dGVfdXBncmFkZQAAAAABAAAAAAAAAAtwcm9wb3NhbF9pZAAAAAAGAAAAAA==",
        "AAAAAAAAAOhQcm9wb3NlcyBhbiB1cGdyYWRlIHdpdGggYSBuZXcgV0FTTSBoYXNoIChtdWx0aXNpZyB2ZXJzaW9uKS4KCiMgQXJndW1lbnRzCiogYGVudmAgLSBUaGUgY29udHJhY3QgZW52aXJvbm1lbnQKKiBgcHJvcG9zZXJgIC0gQWRkcmVzcyBwcm9wb3NpbmcgdGhlIHVwZ3JhZGUKKiBgd2FzbV9oYXNoYCAtIEhhc2ggb2YgdGhlIG5ldyBXQVNNIGNvZGUKCiMgUmV0dXJucwoqIGB1NjRgIC0gVGhlIHByb3Bvc2FsIElEAAAAD3Byb3Bvc2VfdXBncmFkZQAAAAACAAAAAAAAAAhwcm9wb3NlcgAAABMAAAAAAAAACXdhc21faGFzaAAAAAAAA+4AAAAgAAAAAQAAAAY=",
        "AAAAAAAAACpHZXQgc3RhdGUgc25hcHNob3QgLSByZXR1cm5zIGN1cnJlbnQgc3RhdGUAAAAAABJnZXRfc3RhdGVfc25hcHNob3QAAAAAAAAAAAABAAAH0AAAAA1TdGF0ZVNuYXBzaG90AAAA",
        "AAAAAAAAACRHZXQgcGVyZm9ybWFuY2Ugc3RhdHMgZm9yIGEgZnVuY3Rpb24AAAAVZ2V0X3BlcmZvcm1hbmNlX3N0YXRzAAAAAAAAAQAAAAAAAAANZnVuY3Rpb25fbmFtZQAAAAAAABEAAAABAAAH0AAAABBQZXJmb3JtYW5jZVN0YXRz",
        "AAAAAQAAAEI9PT09PT09PT09PT09PT09PT09PT09PQpQcm9wb3NhbCBTdHJ1Y3R1cmUKPT09PT09PT09PT09PT09PT09PT09PT0AAAAAAAAAAAAIUHJvcG9zYWwAAAACAAAAAAAAAAlhcHByb3ZhbHMAAAAAAAPqAAAAEwAAAAAAAAAIZXhlY3V0ZWQAAAAB",
        "AAAAAQAAAEY9PT09PT09PT09PT09PT09PT09PT09PQpNdWx0aXNpZyBDb25maWd1cmF0aW9uCj09PT09PT09PT09PT09PT09PT09PT09AAAAAAAAAAAADk11bHRpU2lnQ29uZmlnAAAAAAACAAAAAAAAAAdzaWduZXJzAAAAA+oAAAATAAAAAAAAAAl0aHJlc2hvbGQAAAAAAAAE" ]),
      options
    )
  }
}