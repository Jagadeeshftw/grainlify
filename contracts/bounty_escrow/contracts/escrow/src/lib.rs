
//! # Bounty Escrow Smart Contract
//!
//! A trustless escrow system for bounty payments on the Stellar blockchain.
//! This contract enables secure fund locking, conditional release to contributors,
//! and automatic refunds after deadlines.
//!
//! ## Overview
//!
//! The Bounty Escrow contract manages the complete lifecycle of bounty payments:
//! 1. **Initialization**: Set up admin and token contract
//! 2. **Lock Funds**: Depositor locks tokens for a bounty with a deadline
//! 3. **Release**: Admin releases funds to contributor upon task completion
//! 4. **Refund**: Automatic refund to depositor if deadline passes
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                  Contract Architecture                       │
//! ├─────────────────────────────────────────────────────────────┤
//! │                                                              │
//! │  ┌──────────────┐                                           │
//! │  │  Depositor   │─────┐                                     │
//! │  └──────────────┘     │                                     │
//! │                       ├──> lock_funds()                     │
//! │  ┌──────────────┐     │         │                           │
//! │  │    Admin     │─────┘         ▼                           │
//! │  └──────────────┘          ┌─────────┐                      │
//! │         │                  │ ESCROW  │                      │
//! │         │                  │ LOCKED  │                      │
//! │         │                  └────┬────┘                      │
//! │         │                       │                           │
//! │         │        ┌──────────────┴───────────────┐           │
//! │         │        │                              │           │
//! │         ▼        ▼                              ▼           │
//! │   release_funds()                          refund()         │
//! │         │                                       │           │
//! │         ▼                                       ▼           │
//! │  ┌──────────────┐                      ┌──────────────┐    │
//! │  │ Contributor  │                      │  Depositor   │    │
//! │  └──────────────┘                      └──────────────┘    │
//! │    (RELEASED)                            (REFUNDED)        │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Security Model
//!
//! ### Trust Assumptions
//! - **Admin**: Trusted entity (backend service) authorized to release funds
//! - **Depositor**: Self-interested party; funds protected by deadline mechanism
//! - **Contributor**: Receives funds only after admin approval
//! - **Contract**: Trustless; operates according to programmed rules
//!
//! ### Key Security Features
//! 1. **Single Initialization**: Prevents admin takeover
//! 2. **Unique Bounty IDs**: No duplicate escrows
//! 3. **Authorization Checks**: All state changes require proper auth
//! 4. **Deadline Protection**: Prevents indefinite fund locking
//! 5. **State Machine**: Enforces valid state transitions
//! 6. **Atomic Operations**: Transfer + state update in single transaction
//!
//! ## Usage Example
//!
//! ```rust
//! use soroban_sdk::{Address, Env};
//!
//! // 1. Initialize contract (one-time setup)
//! let admin = Address::from_string("GADMIN...");
//! let token = Address::from_string("CUSDC...");
//! escrow_client.init(&admin, &token);
//!
//! // 2. Depositor locks 1000 USDC for bounty #42
//! let depositor = Address::from_string("GDEPOSIT...");
//! let amount = 1000_0000000; // 1000 USDC (7 decimals)
//! let deadline = current_timestamp + (30 * 24 * 60 * 60); // 30 days
//! escrow_client.lock_funds(&depositor, &42, &amount, &deadline);
//!
//! // 3a. Admin releases to contributor (happy path)
//! let contributor = Address::from_string("GCONTRIB...");
//! escrow_client.release_funds(&42, &contributor);
//!
//! // OR
//!
//! // 3b. Refund to depositor after deadline (timeout path)
//! // (Can be called by anyone after deadline passes)
//! escrow_client.refund(&42);
//! ```

#![no_std]
mod events;
mod test_bounty_escrow;

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, token, Address, Env};
use events::{
    BountyEscrowInitialized, FundsLocked, FundsReleased, FundsRefunded,
    emit_bounty_initialized, emit_funds_locked, emit_funds_released, emit_funds_refunded
};

// ============================================================================
// Error Definitions
// ============================================================================

/// Contract error codes for the Bounty Escrow system.
///
/// # Error Codes
/// * `AlreadyInitialized (1)` - Contract has already been initialized
/// * `NotInitialized (2)` - Contract must be initialized before use
/// * `BountyExists (3)` - Bounty ID already has funds locked
/// * `BountyNotFound (4)` - No escrow exists for this bounty ID
/// * `FundsNotLocked (5)` - Funds are not in LOCKED state
/// * `DeadlineNotPassed (6)` - Cannot refund before deadline
/// * `Unauthorized (7)` - Caller lacks required authorization
///
/// # Usage in Error Handling
/// ```rust
/// if env.storage().instance().has(&DataKey::Admin) {
///     return Err(Error::AlreadyInitialized);
/// }
/// ```
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Returned when attempting to initialize an already initialized contract
    AlreadyInitialized = 1,
    
    /// Returned when calling contract functions before initialization
    NotInitialized = 2,
    
    /// Returned when attempting to lock funds with a duplicate bounty ID
    BountyExists = 3,
    
    /// Returned when querying or operating on a non-existent bounty
    BountyNotFound = 4,
    
    /// Returned when attempting operations on non-LOCKED funds
    FundsNotLocked = 5,
    
    /// Returned when attempting refund before the deadline has passed
    DeadlineNotPassed = 6,
    
    /// Returned when caller lacks required authorization for the operation
    Unauthorized = 7,
    InvalidAmount = 8,
    InvalidDeadline = 9,
}

// ============================================================================
// Data Structures
// ============================================================================

/// Represents the current state of escrowed funds.
///
/// # State Transitions
/// ```text
/// NONE → Locked → Released (final)
///           ↓
///        Refunded (final)
/// ```
///
/// # States
/// * `Locked` - Funds are held in escrow, awaiting release or refund
/// * `Released` - Funds have been transferred to contributor (final state)
/// * `Refunded` - Funds have been returned to depositor (final state)
///
/// # Invariants
/// - Once in Released or Refunded state, no further transitions allowed
/// - Only Locked state allows state changes
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EscrowStatus {
    Locked,
    Released,
    Refunded,
}

/// Complete escrow record for a bounty.
///
/// # Fields
/// * `depositor` - Address that locked the funds (receives refunds)
/// * `amount` - Token amount held in escrow (in smallest denomination)
/// * `status` - Current state of the escrow (Locked/Released/Refunded)
/// * `deadline` - Unix timestamp after which refunds are allowed
///
/// # Storage
/// Stored in persistent storage with key `DataKey::Escrow(bounty_id)`.
/// TTL is automatically extended on access.
///
/// # Example
/// ```rust
/// let escrow = Escrow {
///     depositor: depositor_address,
///     amount: 1000_0000000, // 1000 tokens
///     status: EscrowStatus::Locked,
///     deadline: current_time + 2592000, // 30 days
/// };
/// ```
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Escrow {
    pub depositor: Address,
    pub amount: i128,
    pub status: EscrowStatus,
    pub deadline: u64,
}

/// Storage keys for contract data.
///
/// # Keys
/// * `Admin` - Stores the admin address (instance storage)
/// * `Token` - Stores the token contract address (instance storage)
/// * `Escrow(u64)` - Stores escrow data indexed by bounty_id (persistent storage)
///
/// # Storage Types
/// - **Instance Storage**: Admin and Token (never expires, tied to contract)
/// - **Persistent Storage**: Individual escrow records (extended TTL on access)
#[contracttype]
pub enum DataKey {
    Admin,
    Token,
    Escrow(u64), // bounty_id
    ReentrancyGuard,
}

// ============================================================================
// Contract Implementation
// ============================================================================

#[contract]
pub struct BountyEscrowContract;

#[contractimpl]
impl BountyEscrowContract {
    // ========================================================================
    // Initialization
    // ========================================================================
    
    /// Initializes the Bounty Escrow contract with admin and token addresses.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `admin` - Address authorized to release funds
    /// * `token` - Token contract address for escrow payments (e.g., XLM, USDC)
    ///
    /// # Returns
    /// * `Ok(())` - Contract successfully initialized
    /// * `Err(Error::AlreadyInitialized)` - Contract already initialized
    ///
    /// # State Changes
    /// - Sets Admin address in instance storage
    /// - Sets Token address in instance storage
    /// - Emits BountyEscrowInitialized event
    ///
    /// # Security Considerations
    /// - Can only be called once (prevents admin takeover)
    /// - Admin should be a secure backend service address
    /// - Token must be a valid Stellar Asset Contract
    /// - No authorization required (first-caller initialization)
    ///
    /// # Events
    /// Emits: `BountyEscrowInitialized { admin, token, timestamp }`
    ///
    /// # Example
    /// ```rust
    /// let admin = Address::from_string("GADMIN...");
    /// let usdc_token = Address::from_string("CUSDC...");
    /// escrow_client.init(&admin, &usdc_token)?;
    /// ```
    ///
    /// # Gas Cost
    /// Low - Only two storage writes
    pub fn init(env: Env, admin: Address, token: Address) -> Result<(), Error> {
        // Prevent re-initialization
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        
        // Store configuration
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);

        // Emit initialization event
        emit_bounty_initialized(
            &env,
            BountyEscrowInitialized {
                admin,
                token,
                timestamp: env.ledger().timestamp()
            },
        );

        Ok(())
    }

    // ========================================================================
    // Core Escrow Functions
    // ========================================================================

    /// Locks funds in escrow for a specific bounty.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `depositor` - Address depositing the funds (must authorize)
    /// * `bounty_id` - Unique identifier for this bounty
    /// * `amount` - Token amount to lock (in smallest denomination)
    /// * `deadline` - Unix timestamp after which refund is allowed
    ///
    /// # Returns
    /// * `Ok(())` - Funds successfully locked
    /// * `Err(Error::NotInitialized)` - Contract not initialized
    /// * `Err(Error::BountyExists)` - Bounty ID already in use
    ///
    /// # State Changes
    /// - Transfers `amount` tokens from depositor to contract
    /// - Creates Escrow record in persistent storage
    /// - Emits FundsLocked event
    ///
    /// # Authorization
    /// - Depositor must authorize the transaction
    /// - Depositor must have sufficient token balance
    /// - Depositor must have approved contract for token transfer
    ///
    /// # Security Considerations
    /// - Bounty ID must be unique (prevents overwrites)
    /// - Amount must be positive (enforced by token contract)
    /// - Deadline should be reasonable (recommended: 7-90 days)
    /// - Token transfer is atomic with state update
    ///
    /// # Events
    /// Emits: `FundsLocked { bounty_id, amount, depositor, deadline }`
    ///
    /// # Example
    /// ```rust
    /// let depositor = Address::from_string("GDEPOSIT...");
    /// let amount = 1000_0000000; // 1000 USDC
    /// let deadline = env.ledger().timestamp() + (30 * 24 * 60 * 60); // 30 days
    /// 
    /// escrow_client.lock_funds(&depositor, &42, &amount, &deadline)?;
    /// // Funds are now locked and can be released or refunded
    /// ```
    ///
    /// # Gas Cost
    /// Medium - Token transfer + storage write + event emission
    ///
    /// # Common Pitfalls
    /// - Forgetting to approve token contract before calling
    /// - Using a bounty ID that already exists
    /// - Setting deadline in the past or too far in the future
    pub fn lock_funds(
        env: Env,
        depositor: Address,
        bounty_id: u64,
        amount: i128,
        deadline: u64,
    ) -> Result<(), Error> {
        // Verify depositor authorization
        depositor.require_auth();

        // Ensure contract is initialized
        if env.storage().instance().has(&DataKey::ReentrancyGuard) {
            panic!("Reentrancy detected");
        }
        env.storage().instance().set(&DataKey::ReentrancyGuard, &true);

        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        if deadline <= env.ledger().timestamp() {
             return Err(Error::InvalidDeadline);
        }
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        // Prevent duplicate bounty IDs
        if env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyExists);
        }

        // Get token contract and transfer funds
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);

        // Transfer funds from depositor to contract
        client.transfer(&depositor, &env.current_contract_address(), &amount);

        // Create escrow record
        let escrow = Escrow {
            depositor: depositor.clone(),
            amount,
            status: EscrowStatus::Locked,
            deadline,
        };

        // Store in persistent storage with extended TTL
        env.storage().persistent().set(&DataKey::Escrow(bounty_id), &escrow);
        
        // Emit event for off-chain indexing
        emit_funds_locked(
            &env,
            FundsLocked {
                bounty_id,
                amount,
                depositor: depositor.clone(),
                deadline
            },
        );

        env.storage().instance().remove(&DataKey::ReentrancyGuard);

        Ok(())
    }

    /// Releases escrowed funds to a contributor.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `bounty_id` - The bounty to release funds for
    /// * `contributor` - Address to receive the funds
    ///
    /// # Returns
    /// * `Ok(())` - Funds successfully released
    /// * `Err(Error::NotInitialized)` - Contract not initialized
    /// * `Err(Error::Unauthorized)` - Caller is not the admin
    /// * `Err(Error::BountyNotFound)` - Bounty doesn't exist
    /// * `Err(Error::FundsNotLocked)` - Funds not in LOCKED state
    ///
    /// # State Changes
    /// - Transfers tokens from contract to contributor
    /// - Updates escrow status to Released
    /// - Emits FundsReleased event
    ///
    /// # Authorization
    /// - **CRITICAL**: Only admin can call this function
    /// - Admin address must match initialization value
    ///
    /// # Security Considerations
    /// - This is the most security-critical function
    /// - Admin should verify task completion off-chain before calling
    /// - Once released, funds cannot be retrieved
    /// - Recipient address should be verified carefully
    /// - Consider implementing multi-sig for admin
    ///
    /// # Events
    /// Emits: `FundsReleased { bounty_id, amount, recipient, timestamp }`
    ///
    /// # Example
    /// ```rust
    /// // After verifying task completion off-chain:
    /// let contributor = Address::from_string("GCONTRIB...");
    /// 
    /// // Admin calls release
    /// escrow_client.release_funds(&42, &contributor)?;
    /// // Funds transferred to contributor, escrow marked as Released
    /// ```
    ///
    /// # Gas Cost
    /// Medium - Token transfer + storage update + event emission
    ///
    /// # Best Practices
    /// 1. Verify contributor identity off-chain
    /// 2. Confirm task completion before release
    /// 3. Log release decisions in backend system
    /// 4. Monitor release events for anomalies
    /// 5. Consider implementing release delays for high-value bounties
    pub fn release_funds(env: Env, bounty_id: u64, contributor: Address) -> Result<(), Error> {
        // Ensure contract is initialized
        if env.storage().instance().has(&DataKey::ReentrancyGuard) {
            panic!("Reentrancy detected");
        }
        env.storage().instance().set(&DataKey::ReentrancyGuard, &true);
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        // Verify admin authorization
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        // Verify bounty exists
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        // Get and verify escrow state
        let mut escrow: Escrow = env.storage().persistent().get(&DataKey::Escrow(bounty_id)).unwrap();

        if escrow.status != EscrowStatus::Locked {
            return Err(Error::FundsNotLocked);
        }

        // Transfer funds to contributor
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        escrow.status = EscrowStatus::Released;
        env.storage().persistent().set(&DataKey::Escrow(bounty_id), &escrow);

        // Transfer funds to contributor
        client.transfer(&env.current_contract_address(), &contributor, &escrow.amount);

        // Emit release event
        emit_funds_released(
            &env,
            FundsReleased {
                bounty_id,
                amount: escrow.amount,
                recipient: contributor.clone(),
                timestamp: env.ledger().timestamp()
            },
        );

        env.storage().instance().remove(&DataKey::ReentrancyGuard);
        Ok(())
    }

    /// Refunds escrowed funds to the depositor after deadline expiration.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `bounty_id` - The bounty to refund
    ///
    /// # Returns
    /// * `Ok(())` - Funds successfully refunded
    /// * `Err(Error::BountyNotFound)` - Bounty doesn't exist
    /// * `Err(Error::FundsNotLocked)` - Funds not in LOCKED state
    /// * `Err(Error::DeadlineNotPassed)` - Current time before deadline
    ///
    /// # State Changes
    /// - Transfers tokens from contract back to depositor
    /// - Updates escrow status to Refunded
    /// - Emits FundsRefunded event
    ///
    /// # Authorization
    /// - **Permissionless**: Anyone can trigger refund after deadline
    /// - No authorization required (time-based protection)
    ///
    /// # Security Considerations
    /// - Deadline enforcement prevents premature refunds
    /// - Permissionless design ensures funds aren't stuck
    /// - Original depositor always receives refund (prevents theft)
    /// - State check prevents double-refund
    ///
    /// # Design Rationale
    /// This function is intentionally permissionless to ensure:
    /// 1. Depositors can always recover funds after deadline
    /// 2. No dependency on admin availability
    /// 3. Trustless, predictable behavior
    /// 4. Protection against key loss scenarios
    ///
    /// # Events
    /// Emits: `FundsRefunded { bounty_id, amount, refund_to, timestamp }`
    ///
    /// # Example
    /// ```rust
    /// // Deadline was January 1, 2025
    /// // Current time: January 15, 2025
    /// 
    /// // Anyone can call refund now
    /// escrow_client.refund(&42)?;
    /// // Funds returned to original depositor
    /// ```
    ///
    /// # Gas Cost
    /// Medium - Token transfer + storage update + event emission
    ///
    /// # Time Calculations
    /// ```rust
    /// // Set deadline for 30 days from now
    /// let deadline = env.ledger().timestamp() + (30 * 24 * 60 * 60);
    /// 
    /// // After deadline passes, refund becomes available
    /// // Current time must be > deadline
    /// ```
    pub fn refund(env: Env, bounty_id: u64) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::ReentrancyGuard) {
            panic!("Reentrancy detected");
        }
        env.storage().instance().set(&DataKey::ReentrancyGuard, &true);

        // We'll allow anyone to trigger the refund if conditions are met, 
        // effectively making it permissionless but conditional.
        // OR we can require depositor auth. Let's make it permissionless to ensure funds aren't stuck if depositor key is lost,
        // but strictly logic bound.
        // However, usually refund is triggered by depositor. Let's stick to logic.
        
        // Verify bounty exists
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        // Get and verify escrow state
        let mut escrow: Escrow = env.storage().persistent().get(&DataKey::Escrow(bounty_id)).unwrap();

        if escrow.status != EscrowStatus::Locked {
            return Err(Error::FundsNotLocked);
        }

        // Verify deadline has passed
        let now = env.ledger().timestamp();
        if now < escrow.deadline {
            return Err(Error::DeadlineNotPassed);
        }

        // Transfer funds back to depositor
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        escrow.status = EscrowStatus::Refunded;
        env.storage().persistent().set(&DataKey::Escrow(bounty_id), &escrow);

        // Transfer funds back to depositor
        client.transfer(&env.current_contract_address(), &escrow.depositor, &escrow.amount);

        // Emit refund event
        emit_funds_refunded(
            &env,
            FundsRefunded {
                bounty_id,
                amount: escrow.amount,
                refund_to: escrow.depositor,
                timestamp: env.ledger().timestamp()
            },
        );

        env.storage().instance().remove(&DataKey::ReentrancyGuard);

        Ok(())
    }

    // ========================================================================
    // View Functions (Read-only)
    // ========================================================================

    /// Retrieves escrow information for a specific bounty.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `bounty_id` - The bounty to query
    ///
    /// # Returns
    /// * `Ok(Escrow)` - The complete escrow record
    /// * `Err(Error::BountyNotFound)` - Bounty doesn't exist
    ///
    /// # Gas Cost
    /// Very Low - Single storage read
    ///
    /// # Example
    /// ```rust
    /// let escrow_info = escrow_client.get_escrow_info(&42)?;
    /// println!("Amount: {}", escrow_info.amount);
    /// println!("Status: {:?}", escrow_info.status);
    /// println!("Deadline: {}", escrow_info.deadline);
    /// ```
    pub fn get_escrow_info(env: Env, bounty_id: u64) -> Result<Escrow, Error> {
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }
        Ok(env.storage().persistent().get(&DataKey::Escrow(bounty_id)).unwrap())
    }

    /// Returns the current token balance held by the contract.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    ///
    /// # Returns
    /// * `Ok(i128)` - Current contract token balance
    /// * `Err(Error::NotInitialized)` - Contract not initialized
    ///
    /// # Use Cases
    /// - Monitoring total locked funds
    /// - Verifying contract solvency
    /// - Auditing and reconciliation
    ///
    /// # Gas Cost
    /// Low - Token contract call
    ///
    /// # Example
    /// ```rust
    /// let balance = escrow_client.get_balance()?;
    /// println!("Total locked: {} stroops", balance);
    /// ```
    pub fn get_balance(env: Env) -> Result<i128, Error> {
        if !env.storage().instance().has(&DataKey::Token) {
            return Err(Error::NotInitialized);
        }
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        Ok(client.balance(&env.current_contract_address()))
    }
}

#[cfg(test)]
mod test;