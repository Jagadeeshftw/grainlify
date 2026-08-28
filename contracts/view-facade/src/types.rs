//! Data structures, storage keys and errors used by the view-facade contract.
//!
//! This module is purely declarative — it holds the `#[contracttype]` definitions
//! that make up the contract's on-chain data model. Keeping them separate from the
//! query adapters ([`crate::query`]) and the entrypoint ([`crate::ViewFacade`])
//! keeps the read-only ABI surface easy to review.

use soroban_sdk::{contracterror, contracttype, Address};

/// Typed error codes returned by fallible entry-points.
///
/// Using a `#[contracterror]` enum instead of bare `panic!` strings gives
/// callers a stable integer discriminant they can match on and surfaces
/// clearer diagnostics in simulation tools.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum FacadeError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    RegistryFull = 3,
    InvalidPagination = 4,
}

/// Identifies the two slots this contract writes in instance storage.
///
/// Instance storage persists across contract upgrades, which ensures the
/// admin and the registry survive a WASM swap.
#[contracttype]
pub enum DataKey {
    /// The immutable administrator [`Address`] stored at initialization.
    Admin,
    /// The ordered list of [`RegisteredContract`] entries.
    Registry,
}

/// Maximum number of contracts that can be registered in the facade.
///
/// This limit prevents unbounded storage growth and ensures predictable
/// gas costs for all operations.
///
/// ## Rationale
///
/// - **Gas efficiency**: Each registry entry requires storage reads/writes
/// - **Indexer friendliness**: Bounded size enables predictable pagination
/// - **Operational safety**: Prevents storage exhaustion attacks
pub const MAX_REGISTRY_SIZE: u32 = 1000;

/// Distinguishes the role / type of a registered contract.
///
/// This allows consumers to filter the registry (e.g. "show me all bounty
/// escrows") without querying individual contracts.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContractKind {
    /// A `BountyEscrow` contract managing individual bounty funds.
    BountyEscrow,
    /// A `ProgramEscrow` contract managing hackathon/grant prize pools.
    ProgramEscrow,
    /// A Soroban-native escrow contract variant.
    SorobanEscrow,
    /// The `GrainlifyCore` upgrade-management contract.
    GrainlifyCore,
}

/// A single entry in the view-facade registry.
///
/// Represents one contract deployment that the admin has chosen to expose
/// through this aggregation endpoint.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegisteredContract {
    /// On-chain address of the registered contract.
    pub address: Address,
    /// High-level role of the contract within the Grainlify ecosystem.
    pub kind: ContractKind,
    /// Numeric version reported by the contract at registration time.
    pub version: u32,
}

/// Emitted once when the facade is successfully initialized.
///
/// Off-chain indexers can use this event as a reliable signal that the
/// contract is ready to accept `register` calls.
///
/// # Event Topic
/// `("facade", "init")`
#[contracttype]
#[derive(Clone, Debug)]
pub struct InitializedEvent {
    /// The administrator address stored at initialization.
    pub admin: Address,
}
