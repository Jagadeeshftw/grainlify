//! Minimal shared storage-key constants used by contract crates.

use soroban_sdk::{symbol_short, Symbol};

pub mod namespaces {
    pub const PROGRAM_ESCROW: &str = "PE_";
    pub const BOUNTY_ESCROW: &str = "BE_";
    pub const COMMON: &str = "COMMON_";
}

pub mod validation {
    /// Returns `true` when `symbol` carries `expected_prefix`.
    pub fn validate_namespace(symbol: &str, expected_prefix: &str) -> bool {
        symbol.starts_with(expected_prefix)
    }

    /// Validates that `symbol` belongs to the `expected_prefix` namespace.
    ///
    /// A key that is not namespaced with its own contract's prefix aliases a
    /// slot owned by another contract that shares the same storage space, so it
    /// is rejected here. This is what makes the audit fail when a colliding or
    /// mis-namespaced key is introduced.
    ///
    /// Only available on host/test targets, because `Symbol` string rendering
    /// is not provided on `wasm32`; the audit always runs under `cargo test`.
    #[cfg(not(target_family = "wasm"))]
    pub fn validate_storage_key(
        symbol: soroban_sdk::Symbol,
        expected_prefix: &str,
    ) -> Result<(), &'static str> {
        if symbol.to_string().starts_with(expected_prefix) {
            Ok(())
        } else {
            Err("storage key is not namespaced with the expected contract prefix")
        }
    }
}

pub mod shared {
    pub const EVENT_VERSION_V2: u32 = 2;
    pub const BASIS_POINTS: i128 = 10_000;
    pub const RISK_FLAG_HIGH_RISK: u32 = 1 << 0;
    pub const RISK_FLAG_UNDER_REVIEW: u32 = 1 << 1;
    pub const RISK_FLAG_RESTRICTED: u32 = 1 << 2;
    pub const RISK_FLAG_DEPRECATED: u32 = 1 << 3;
}

pub mod program_escrow {
    use super::*;

    pub const PROGRAM_INITIALIZED: Symbol = symbol_short!("PE_INIT");
    pub const FUNDS_LOCKED: Symbol = symbol_short!("PE_FLOCK");
    pub const BATCH_FUNDS_LOCKED: Symbol = symbol_short!("PE_BATLK");
    pub const BATCH_FUNDS_RELEASED: Symbol = symbol_short!("PE_BATRL");
    pub const BATCH_PAYOUT: Symbol = symbol_short!("PE_BPAY");
    pub const PAYOUT: Symbol = symbol_short!("PE_PAY");
    pub const PAUSE_STATE_CHANGED: Symbol = symbol_short!("PE_PAUSE");
    pub const MAINTENANCE_MODE_CHANGED: Symbol = symbol_short!("PE_MAINT");
    pub const READ_ONLY_MODE_CHANGED: Symbol = symbol_short!("PE_READ");
    pub const PROGRAM_RISK_FLAGS_UPDATED: Symbol = symbol_short!("PE_RISK");
    pub const PROGRAM_REGISTRY: Symbol = symbol_short!("PE_PREG");
    pub const PROGRAM_REGISTERED: Symbol = symbol_short!("PE_PRGD");
    pub const RELEASE_SCHEDULED: Symbol = symbol_short!("PE_RSCH");
    pub const SCHEDULE_RELEASED: Symbol = symbol_short!("PE_SREL");
    pub const PROGRAM_DELEGATE_SET: Symbol = symbol_short!("PE_DSET");
    pub const PROGRAM_DELEGATE_REVOKED: Symbol = symbol_short!("PE_DREV");
    pub const PROGRAM_METADATA_UPDATED: Symbol = symbol_short!("PE_META");
    pub const DISPUTE_OPENED: Symbol = symbol_short!("PE_DOP");
    pub const DISPUTE_RESOLVED: Symbol = symbol_short!("PE_DRES");

    pub const PROGRAM_DATA: Symbol = symbol_short!("PE_PDATA");
    pub const RECEIPT_ID: Symbol = symbol_short!("PE_RCID");
    pub const SCHEDULES: Symbol = symbol_short!("PE_SCHED");
    pub const RELEASE_HISTORY: Symbol = symbol_short!("PE_RHIST");
    pub const NEXT_SCHEDULE_ID: Symbol = symbol_short!("PE_NSID");
    pub const PROGRAM_INDEX: Symbol = symbol_short!("PE_PIDX");
    pub const AUTH_KEY_INDEX: Symbol = symbol_short!("PE_AIDX");
    pub const FEE_CONFIG: Symbol = symbol_short!("PE_FCFG");
    pub const FEE_COLLECTED: Symbol = symbol_short!("PE_FCOL");
}

pub mod bounty_escrow {
    use super::*;

    pub const BOUNTY_INITIALIZED: Symbol = symbol_short!("BE_INIT");
    pub const FUNDS_LOCKED: Symbol = symbol_short!("BE_FLOCK");
    pub const FUNDS_LOCKED_ANON: Symbol = symbol_short!("BE_FLKAN");
    pub const FUNDS_RELEASED: Symbol = symbol_short!("BE_FREL");
    pub const FUNDS_REFUNDED: Symbol = symbol_short!("BE_FREF");
    pub const ESCROW_PUBLISHED: Symbol = symbol_short!("BE_PUB");
    pub const TICKET_ISSUED: Symbol = symbol_short!("BE_TKIS");
    pub const TICKET_CLAIMED: Symbol = symbol_short!("BE_TKCL");
    pub const MAINTENANCE_MODE_CHANGED: Symbol = symbol_short!("BE_MAINT");
    pub const PAUSE_STATE_CHANGED: Symbol = symbol_short!("BE_PAUSE");
    pub const RISK_FLAGS_UPDATED: Symbol = symbol_short!("BE_RISK");
    pub const DEPRECATION_STATE_CHANGED: Symbol = symbol_short!("BE_DEPR");

    pub const ADMIN: Symbol = symbol_short!("BE_ADMIN");
    pub const TOKEN: Symbol = symbol_short!("BE_TOKEN");
    pub const VERSION: Symbol = symbol_short!("BE_VER");
    pub const ESCROW_INDEX: Symbol = symbol_short!("BE_EIDX");
    pub const DEPOSITOR_INDEX: Symbol = symbol_short!("BE_DIDX");
    pub const ESCROW_FREEZE: Symbol = symbol_short!("BE_EFRZ");
    pub const ADDRESS_FREEZE: Symbol = symbol_short!("BE_AFRZ");
    pub const FEE_CONFIG: Symbol = symbol_short!("BE_FCFG");
    pub const REFUND_APPROVAL: Symbol = symbol_short!("BE_RAPP");
    pub const REENTRANCY_GUARD: Symbol = symbol_short!("BE_REENT");
    pub const MULTISIG_CONFIG: Symbol = symbol_short!("BE_MSIG");
    pub const RELEASE_APPROVAL: Symbol = symbol_short!("BE_LAPP");
    pub const PENDING_CLAIM: Symbol = symbol_short!("BE_PCLM");
    pub const TICKET_COUNTER: Symbol = symbol_short!("BE_TCTR");
    pub const CLAIM_TICKET: Symbol = symbol_short!("BE_CTK");
    pub const CLAIM_TICKET_INDEX: Symbol = symbol_short!("BE_CTIX");
    pub const BENEFICIARY_TICKETS: Symbol = symbol_short!("BE_BTIX");
    pub const CLAIM_WINDOW: Symbol = symbol_short!("BE_CWIN");
    pub const PAUSE_FLAGS: Symbol = symbol_short!("BE_PFLG");
    pub const AMOUNT_POLICY: Symbol = symbol_short!("BE_APOL");
    pub const CAPABILITY_NONCE: Symbol = symbol_short!("BE_CNCE");
    pub const CAPABILITY: Symbol = symbol_short!("BE_CAP");
    pub const NON_TRANSFERABLE_REWARDS: Symbol = symbol_short!("BE_NTR");
    pub const DEPRECATION_STATE: Symbol = symbol_short!("BE_DSTA");
    pub const PARTICIPANT_FILTER_MODE: Symbol = symbol_short!("BE_PFMD");
    pub const ANONYMOUS_RESOLVER: Symbol = symbol_short!("BE_ARES");
    pub const TOKEN_FEE_CONFIG: Symbol = symbol_short!("BE_TFCG");
    pub const CHAIN_ID: Symbol = symbol_short!("BE_CHID");
    pub const NETWORK_ID: Symbol = symbol_short!("BE_NWID");
    pub const MAINTENANCE_MODE: Symbol = symbol_short!("BE_MMOD");
    pub const GAS_BUDGET_CONFIG: Symbol = symbol_short!("BE_GBCF");
    pub const TIMELOCK_CONFIG: Symbol = symbol_short!("BE_TLCF");
    pub const PENDING_ACTION: Symbol = symbol_short!("BE_PACT");
    pub const ACTION_COUNTER: Symbol = symbol_short!("BE_ACTR");
}
