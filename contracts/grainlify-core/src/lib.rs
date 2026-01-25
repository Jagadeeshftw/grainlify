//! # Grainlify Contract Upgrade System
//!
//! A minimal, secure contract upgrade pattern for Soroban smart contracts.
//! This contract implements admin-controlled WASM upgrades with version tracking.
//!
//! ## Overview
//!
//! The Grainlify contract provides a foundational upgrade mechanism that allows
//! authorized administrators to update contract logic while maintaining state
//! persistence. This is essential for bug fixes, feature additions, and security
//! patches in production environments.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │              Contract Upgrade Architecture                   │
//! ├─────────────────────────────────────────────────────────────┤
//! │                                                              │
//! │  ┌──────────────┐                                           │
//! │  │    Admin     │                                           │
//! │  └──────┬───────┘                                           │
//! │         │                                                    │
//! │         │ 1. Compile new WASM                               │
//! │         │ 2. Upload to Stellar                              │
//! │         │ 3. Get WASM hash                                  │
//! │         │                                                    │
//! │         ▼                                                    │
//! │  ┌──────────────────┐                                       │
//! │  │  upgrade(hash)   │────────┐                              │
//! │  └──────────────────┘        │                              │
//! │         │                     │                              │
//! │         │ require_auth()      │                              │
//! │         │                     ▼                              │
//! │         │              ┌─────────────┐                       │
//! │         │              │   Verify    │                       │
//! │         │              │   Admin     │                       │
//! │         │              └──────┬──────┘                       │
//! │         │                     │                              │
//! │         │                     ▼                              │
//! │         │              ┌─────────────┐                       │
//! │         └─────────────>│   Update    │                       │
//! │                        │   WASM      │                       │
//! │                        └──────┬──────┘                       │
//! │                               │                              │
//! │                               ▼                              │
//! │                        ┌─────────────┐                       │
//! │                        │ New Version │                       │
//! │                        │  (Optional) │                       │
//! │                        └─────────────┘                       │
//! │                                                              │
//! │  Storage:                                                    │
//! │  ┌────────────────────────────────────┐                     │
//! │  │ Admin: Address                     │                     │
//! │  │ Version: u32                       │                     │
//! │  └────────────────────────────────────┘                     │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Security Model
//!
//! ### Trust Assumptions
//! - **Admin**: Highly trusted entity with upgrade authority
//! - **WASM Code**: New code must be audited before deployment
//! - **State Preservation**: Upgrades preserve existing contract state
//!
//! ### Security Features
//! 1. **Single Admin**: Only one authorized address can upgrade
//! 2. **Authorization Check**: Every upgrade requires admin signature
//! 3. **Version Tracking**: Auditable upgrade history
//! 4. **State Preservation**: Instance storage persists across upgrades
//! 5. **Immutable After Init**: Admin cannot be changed after initialization
//!
//! ### Security Considerations
//! - Admin key should be secured with hardware wallet or multi-sig
//! - New WASM should be audited before upgrade
//! - Consider implementing timelock for high-value contracts
//! - Version updates should follow semantic versioning
//! - Test upgrades on testnet before mainnet deployment
//!
//! ## Upgrade Process
//!
//! ```rust
//! // 1. Initialize contract (one-time)
//! let admin = Address::from_string("GADMIN...");
//! contract.init(&admin);
//!
//! // 2. Develop and test new version locally
//! // ... make changes to contract code ...
//!
//! // 3. Build new WASM
//! // $ cargo build --release --target wasm32-unknown-unknown
//!
//! // 4. Upload WASM to Stellar and get hash
//! // $ stellar contract install --wasm target/wasm32-unknown-unknown/release/contract.wasm
//! // Returns: hash (e.g., "abc123...")
//!
//! // 5. Perform upgrade
//! let wasm_hash = BytesN::from_array(&env, &[0xab, 0xcd, ...]);
//! contract.upgrade(&wasm_hash);
//!
//! // 6. (Optional) Update version number
//! contract.set_version(&2);
//!
//! // 7. Verify upgrade
//! let version = contract.get_version();
//! assert_eq!(version, 2);
//! ```
//!
//! ## State Migration
//!
//! When upgrading contracts that require state migration:
//!
//! ```rust
//! // In new WASM version, add migration function:
//! pub fn migrate(env: Env) {
//!     let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
//!     admin.require_auth();
//!     
//!     // Perform state migration
//!     // Example: Convert old data format to new format
//!     let old_version = env.storage().instance().get(&DataKey::Version).unwrap_or(0);
//!     
//!     if old_version < 2 {
//!         // Migrate from v1 to v2
//!         migrate_v1_to_v2(&env);
//!     }
//!     
//!     // Update version
//!     env.storage().instance().set(&DataKey::Version, &2u32);
//! }
//! ```
//!
//! ## Best Practices
//!
//! 1. **Version Numbering**: Use semantic versioning (MAJOR.MINOR.PATCH)
//! 2. **Testing**: Always test upgrades on testnet first
//! 3. **Auditing**: Audit new code before mainnet deployment
//! 4. **Documentation**: Document breaking changes between versions
//! 5. **Rollback Plan**: Keep previous WASM hash for emergency rollback
//! 6. **Admin Security**: Use multi-sig or timelock for production
//! 7. **State Validation**: Verify state integrity after upgrade
//!
//! ## Common Pitfalls
//!
//! - ❌ Not testing upgrades on testnet
//! - ❌ Losing admin private key
//! - ❌ Breaking state compatibility between versions
//! - ❌ Not documenting migration steps
//! - ❌ Upgrading without proper testing
//! - ❌ Not having a rollback plan

#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, BytesN, Env};

// ============================================================================
// Contract Definition
// ============================================================================

#[contract]
pub struct GrainlifyContract;

// ============================================================================
// Data Structures
// ============================================================================

/// Storage keys for contract data.
///
/// # Keys
/// * `Admin` - Stores the administrator address (set once at initialization)
/// * `Version` - Stores the current contract version number
///
/// # Storage Type
/// Instance storage - Persists across contract upgrades
///
/// # Security Note
/// These keys use instance storage to ensure data survives WASM upgrades.
/// The admin address is immutable after initialization.
#[contracttype]
#[derive(Clone)]
enum DataKey {
    /// Administrator address with upgrade authority
    Admin,
    
    /// Current version number (increments with upgrades)
    Version,
}

// ============================================================================
// Constants
// ============================================================================

/// Current contract version.
///
/// This constant should be incremented with each contract upgrade:
/// - MAJOR version: Breaking changes
/// - MINOR version: New features (backward compatible)
/// - PATCH version: Bug fixes
///
/// # Version History
/// - v1: Initial release with basic upgrade functionality
///
/// # Usage
/// Set during initialization and can be updated via `set_version()`.
const VERSION: u32 = 1;

// ============================================================================
// Contract Implementation
// ============================================================================

#[contractimpl]
impl GrainlifyContract {
    // ========================================================================
    // Initialization
    // ========================================================================
    
    /// Initializes the contract with an admin address.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `admin` - Address authorized to perform upgrades
    ///
    /// # Panics
    /// * If contract is already initialized
    ///
    /// # State Changes
    /// - Sets Admin address in instance storage
    /// - Sets initial Version number
    ///
    /// # Security Considerations
    /// - Can only be called once (prevents admin takeover)
    /// - Admin address is immutable after initialization
    /// - Admin should be a secure address (hardware wallet/multi-sig)
    /// - No authorization required for initialization (first-caller pattern)
    ///
    /// # Example
    /// ```rust
    /// use soroban_sdk::{Address, Env};
    /// 
    /// let env = Env::default();
    /// let admin = Address::generate(&env);
    /// 
    /// // Initialize contract
    /// contract.init(&env, &admin);
    /// 
    /// // Subsequent init attempts will panic
    /// // contract.init(&env, &another_admin); // ❌ Panics!
    /// ```
    ///
    /// # Gas Cost
    /// Low - Two storage writes
    ///
    /// # Production Deployment
    /// ```bash
    /// # Deploy contract
    /// stellar contract deploy \
    ///   --wasm target/wasm32-unknown-unknown/release/grainlify.wasm \
    ///   --source ADMIN_SECRET_KEY
    ///
    /// # Initialize with admin address
    /// stellar contract invoke \
    ///   --id CONTRACT_ID \
    ///   --source ADMIN_SECRET_KEY \
    ///   -- init \
    ///   --admin GADMIN_ADDRESS
    /// ```
    pub fn init(env: Env, admin: Address) {
        // Prevent re-initialization to protect admin immutability
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Already initialized");
        }
        
        // Store admin address (immutable after this point)
        env.storage().instance().set(&DataKey::Admin, &admin);
        
        // Set initial version
        env.storage().instance().set(&DataKey::Version, &VERSION);
    }

    // ========================================================================
    // Upgrade Functions
    // ========================================================================

    /// Upgrades the contract to new WASM code.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `new_wasm_hash` - Hash of the uploaded WASM code (32 bytes)
    ///
    /// # Authorization
    /// - **CRITICAL**: Only admin can call this function
    /// - Admin must sign the transaction
    ///
    /// # State Changes
    /// - Replaces current contract WASM with new version
    /// - Preserves all instance storage (admin, version, etc.)
    /// - Does NOT automatically update version number (call `set_version` separately)
    ///
    /// # Security Considerations
    /// - **Code Review**: New WASM must be audited before deployment
    /// - **Testing**: Test upgrade on testnet first
    /// - **State Compatibility**: Ensure new code is compatible with existing state
    /// - **Rollback Plan**: Keep previous WASM hash for emergency rollback
    /// - **Version Update**: Call `set_version` after upgrade if needed
    ///
    /// # Workflow
    /// 1. Develop and test new contract version
    /// 2. Build WASM: `cargo build --release --target wasm32-unknown-unknown`
    /// 3. Upload WASM to Stellar network
    /// 4. Get WASM hash from upload response
    /// 5. Call this function with the hash
    /// 6. (Optional) Call `set_version` to update version number
    ///
    /// # Example
    /// ```rust
    /// use soroban_sdk::{BytesN, Env};
    /// 
    /// let env = Env::default();
    /// 
    /// // Upload new WASM and get hash (done off-chain)
    /// let wasm_hash = BytesN::from_array(
    ///     &env,
    ///     &[0xab, 0xcd, 0xef, ...] // 32 bytes
    /// );
    /// 
    /// // Perform upgrade (requires admin authorization)
    /// contract.upgrade(&env, &wasm_hash);
    /// 
    /// // Update version number
    /// contract.set_version(&env, &2);
    /// ```
    ///
    /// # Production Upgrade Process
    /// ```bash
    /// # 1. Build new WASM
    /// cargo build --release --target wasm32-unknown-unknown
    ///
    /// # 2. Upload WASM to Stellar
    /// stellar contract install \
    ///   --wasm target/wasm32-unknown-unknown/release/grainlify.wasm \
    ///   --source ADMIN_SECRET_KEY
    /// # Output: WASM_HASH (e.g., abc123...)
    ///
    /// # 3. Upgrade contract
    /// stellar contract invoke \
    ///   --id CONTRACT_ID \
    ///   --source ADMIN_SECRET_KEY \
    ///   -- upgrade \
    ///   --new_wasm_hash WASM_HASH
    ///
    /// # 4. Update version (optional)
    /// stellar contract invoke \
    ///   --id CONTRACT_ID \
    ///   --source ADMIN_SECRET_KEY \
    ///   -- set_version \
    ///   --new_version 2
    /// ```
    ///
    /// # Gas Cost
    /// High - WASM code replacement is expensive
    ///
    /// # Emergency Rollback
    /// If new version has issues, rollback to previous WASM:
    /// ```bash
    /// stellar contract invoke \
    ///   --id CONTRACT_ID \
    ///   --source ADMIN_SECRET_KEY \
    ///   -- upgrade \
    ///   --new_wasm_hash PREVIOUS_WASM_HASH
    /// ```
    ///
    /// # Panics
    /// * If admin address is not set (contract not initialized)
    /// * If caller is not the admin
    pub fn upgrade(env: Env, new_wasm_hash: BytesN<32>) {
        // Verify admin authorization
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap();
        admin.require_auth();

        // Perform WASM upgrade
        env.deployer().update_current_contract_wasm(new_wasm_hash);
    }

    // ========================================================================
    // Version Management
    // ========================================================================

    /// Retrieves the current contract version number.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    ///
    /// # Returns
    /// * `u32` - Current version number (defaults to 0 if not set)
    ///
    /// # Usage
    /// Use this to verify contract version for:
    /// - Client compatibility checks
    /// - Migration decision logic
    /// - Audit trails
    /// - Version-specific behavior
    ///
    /// # Example
    /// ```rust
    /// let version = contract.get_version(&env);
    /// 
    /// match version {
    ///     1 => println!("Running v1"),
    ///     2 => println!("Running v2 with new features"),
    ///     _ => println!("Unknown version"),
    /// }
    /// ```
    ///
    /// # Client-Side Usage
    /// ```javascript
    /// // Check contract version before interaction
    /// const version = await contract.get_version();
    /// 
    /// if (version < 2) {
    ///     throw new Error("Contract version too old, please upgrade");
    /// }
    /// ```
    ///
    /// # Gas Cost
    /// Very Low - Single storage read
    pub fn get_version(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::Version)
            .unwrap_or(0)
    }

    /// Updates the contract version number.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `new_version` - New version number to set
    ///
    /// # Authorization
    /// - Only admin can call this function
    /// - Admin must sign the transaction
    ///
    /// # State Changes
    /// - Updates Version in instance storage
    ///
    /// # Usage
    /// Call this function after upgrading contract WASM to reflect
    /// the new version number. This provides an audit trail of upgrades.
    ///
    /// # Version Numbering Strategy
    /// Recommend using semantic versioning encoded as single u32:
    /// - `1` = v1.0.0
    /// - `2` = v2.0.0
    /// - `101` = v1.0.1 (patch)
    /// - `110` = v1.1.0 (minor)
    ///
    /// Or use simple incrementing:
    /// - `1` = First version
    /// - `2` = Second version
    /// - `3` = Third version
    ///
    /// # Example
    /// ```rust
    /// // After upgrading WASM
    /// contract.upgrade(&env, &new_wasm_hash);
    /// 
    /// // Update version to reflect the upgrade
    /// contract.set_version(&env, &2);
    /// 
    /// // Verify
    /// assert_eq!(contract.get_version(&env), 2);
    /// ```
    ///
    /// # Best Practice
    /// Document version changes:
    /// ```rust
    /// // Version History:
    /// // 1 - Initial release
    /// // 2 - Added feature X, fixed bug Y
    /// // 3 - Performance improvements
    /// contract.set_version(&env, &3);
    /// ```
    ///
    /// # Security Note
    /// This function does NOT perform the actual upgrade.
    /// It only updates the version metadata. Always call
    /// `upgrade()` first, then `set_version()`.
    ///
    /// # Gas Cost
    /// Very Low - Single storage write
    ///
    /// # Panics
    /// * If admin address is not set (contract not initialized)
    /// * If caller is not the admin
    pub fn set_version(env: Env, new_version: u32) {
        // Verify admin authorization
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap();
        admin.require_auth();
        
        // Update version number
        env.storage().instance().set(&DataKey::Version, &new_version);
    }
}

// ============================================================================
// Testing Module
// ============================================================================

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    #[test]
    fn test_init() {
        let env = Env::default();
        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.init(&admin);

        assert_eq!(client.get_version(), VERSION);
    }

    #[test]
    #[should_panic(expected = "Already initialized")]
    fn test_init_twice_panics() {
        let env = Env::default();
        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.init(&admin);
        client.init(&admin); // Should panic
    }

    #[test]
    fn test_set_version() {
        let env = Env::default();
        env.mock_all_auths();
        
        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.init(&admin);

        client.set_version(&2);
        assert_eq!(client.get_version(), 2);
    }
}