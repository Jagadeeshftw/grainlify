#![no_std]
//! Minimal Soroban escrow demo: lock, release, and refund.
//! Parity with main contracts/bounty_escrow where applicable; see soroban/PARITY.md.

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, token, Address, BytesN,
    Env, String, Symbol, Vec,
};

const MAX_PAGE_SIZE: u32 = 20;

mod identity;
pub use identity::*;

#[cfg(test)]
mod identity_test;
mod reentrancy_guard;
#[cfg(test)]
mod test;
#[cfg(test)]
mod test_search;

#[contracterror]
#[derive(Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum Error {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    BountyExists = 3,
    BountyNotFound = 4,
    FundsNotLocked = 5,
    DeadlineNotPassed = 6,
    Unauthorized = 7,
    InsufficientBalance = 8,
    // Identity-related errors
    InvalidSignature = 100,
    ClaimExpired = 101,
    UnauthorizedIssuer = 102,
    InvalidClaimFormat = 103,
    TransactionExceedsLimit = 104,
    InvalidRiskScore = 105,
    InvalidTier = 106,
    JurisdictionKycRequired = 107,
    JurisdictionFundingLimitExceeded = 108,
    JurisdictionPaused = 109,
    InvalidBatchSize = 110,
    ContractDeprecated = 111,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EscrowStatus {
    Locked,
    Released,
    Refunded,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Escrow {
    pub depositor: Address,
    pub amount: i128,
    pub remaining_amount: i128,
    pub status: EscrowStatus,
    pub deadline: u64,
    pub jurisdiction: OptionalJurisdiction,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowJurisdictionConfig {
    pub tag: Option<String>,
    pub requires_kyc: bool,
    pub enforce_identity_limits: bool,
    pub lock_paused: bool,
    pub release_paused: bool,
    pub refund_paused: bool,
    pub max_lock_amount: Option<i128>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OptionalJurisdiction {
    None,
    Some(EscrowJurisdictionConfig),
}

#[contracttype]
pub enum DataKey {
    Admin,
    Token,
    Escrow(u64),
    // Identity-related storage keys
    AddressIdentity(Address),
    AuthorizedIssuer(Address),
    TierLimits,
    RiskThresholds,
    ReentrancyGuard,
    EscrowJurisdiction(u64),
    EscrowIndex,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowSearchCriteria {
    pub status_filter: u32,
    pub depositor: Option<Address>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowRecord {
    pub bounty_id: u64,
    pub depositor: Address,
    pub amount: i128,
    pub status: EscrowStatus,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowPage {
    pub records: Vec<EscrowRecord>,
    pub next_cursor: Option<u64>,
    pub has_more: bool,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IssuerUpdatedEvent {
    pub issuer: Address,
    pub authorized: bool,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityClaimEvent {
    #[topic]
    pub address: Address,
    pub tier: IdentityTier,
    pub risk_score: u32,
    pub expiry: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityClaimStatusEvent {
    #[topic]
    pub address: Address,
    pub status: Symbol,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionLimitEvent {
    #[topic]
    pub address: Address,
    pub amount: i128,
    pub limit: i128,
    pub status: Symbol,
}

#[allow(non_camel_case_types)]
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct juris {
    #[topic]
    pub bounty_id: u64,
    pub tag: Option<String>,
    pub requires_kyc: bool,
    pub enforce_identity_limits: bool,
}

#[contract]
pub struct EscrowContract;

#[contractimpl]
impl EscrowContract {
    /// Initialize with admin and token. Call once.
    pub fn init(env: Env, admin: Address, token: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);

        // Initialize default tier limits and risk thresholds
        let default_limits = TierLimits::default();
        let default_thresholds = RiskThresholds::default();
        env.storage()
            .persistent()
            .set(&DataKey::TierLimits, &default_limits);
        env.storage()
            .persistent()
            .set(&DataKey::RiskThresholds, &default_thresholds);

        Ok(())
    }

    /// Set or update an authorized claim issuer (admin only)
    pub fn set_authorized_issuer(env: Env, issuer: Address, authorized: bool) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();

        env.storage()
            .persistent()
            .set(&DataKey::AuthorizedIssuer(issuer.clone()), &authorized);

        // Emit event for issuer management
        IssuerUpdatedEvent {
            issuer: issuer.clone(),
            authorized,
        }
        .publish(&env);

        Ok(())
    }

    /// Configure tier-based transaction limits (admin only)
    pub fn set_tier_limits(
        env: Env,
        unverified: i128,
        basic: i128,
        verified: i128,
        premium: i128,
    ) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();

        let limits = TierLimits {
            unverified_limit: unverified,
            basic_limit: basic,
            verified_limit: verified,
            premium_limit: premium,
        };

        env.storage()
            .persistent()
            .set(&DataKey::TierLimits, &limits);
        Ok(())
    }

    /// Configure risk-based adjustments (admin only)
    pub fn set_risk_thresholds(
        env: Env,
        high_risk_threshold: u32,
        high_risk_multiplier: u32,
    ) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();

        let thresholds = RiskThresholds {
            high_risk_threshold,
            high_risk_multiplier,
        };

        env.storage()
            .persistent()
            .set(&DataKey::RiskThresholds, &thresholds);
        Ok(())
    }

    /// Submit an identity claim for verification and storage
    pub fn submit_identity_claim(
        env: Env,
        claim: IdentityClaim,
        signature: BytesN<64>,
        issuer_pubkey: BytesN<32>,
    ) -> Result<(), Error> {
        // Require authentication from the address in the claim
        claim.address.require_auth();

        // Check if contract is initialized
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        // Validate claim format
        identity::validate_claim(&claim)?;

        // Check if claim has expired
        if identity::is_claim_expired(&env, claim.expiry) {
            IdentityClaimStatusEvent {
                address: claim.address.clone(),
                status: Symbol::new(&env, "expired"),
            }
            .publish(&env);
            return Err(Error::ClaimExpired);
        }

        // Check if issuer is authorized
        let is_authorized: bool = env
            .storage()
            .persistent()
            .get(&DataKey::AuthorizedIssuer(claim.issuer.clone()))
            .unwrap_or(false);

        if !is_authorized {
            IdentityClaimStatusEvent {
                address: claim.address.clone(),
                status: Symbol::new(&env, "unauth"),
            }
            .publish(&env);
            return Err(Error::UnauthorizedIssuer);
        }

        // Verify claim signature
        identity::verify_claim_signature(&env, &claim, &signature, &issuer_pubkey)?;

        // Store identity data for the address
        let now = env.ledger().timestamp();
        let identity_data = AddressIdentity {
            tier: claim.tier.clone(),
            risk_score: claim.risk_score,
            expiry: claim.expiry,
            last_updated: now,
        };

        env.storage().persistent().set(
            &DataKey::AddressIdentity(claim.address.clone()),
            &identity_data,
        );

        // Emit event for successful claim submission
        IdentityClaimEvent {
            address: claim.address.clone(),
            tier: claim.tier,
            risk_score: claim.risk_score,
            expiry: claim.expiry,
        }
        .publish(&env);

        Ok(())
    }

    /// Query identity data for an address
    pub fn get_address_identity(env: Env, address: Address) -> AddressIdentity {
        let identity: Option<AddressIdentity> = env
            .storage()
            .persistent()
            .get(&DataKey::AddressIdentity(address));

        match identity {
            Some(id) => {
                // Check if claim has expired
                if identity::is_claim_expired(&env, id.expiry) {
                    // Return default unverified tier
                    AddressIdentity::default()
                } else {
                    id
                }
            }
            None => AddressIdentity::default(),
        }
    }

    /// Query effective transaction limit for an address
    pub fn get_effective_limit(env: Env, address: Address) -> i128 {
        let identity = Self::get_address_identity(env.clone(), address);

        let tier_limits: TierLimits = env
            .storage()
            .persistent()
            .get(&DataKey::TierLimits)
            .unwrap_or_default();

        let risk_thresholds: RiskThresholds = env
            .storage()
            .persistent()
            .get(&DataKey::RiskThresholds)
            .unwrap_or_default();

        identity::calculate_effective_limit(&env, &identity, &tier_limits, &risk_thresholds)
    }

    /// Check if an address has a valid (non-expired) claim
    pub fn is_claim_valid(env: Env, address: Address) -> bool {
        let identity: Option<AddressIdentity> = env
            .storage()
            .persistent()
            .get(&DataKey::AddressIdentity(address));

        match identity {
            Some(id) => !identity::is_claim_expired(&env, id.expiry),
            None => false,
        }
    }

    fn enforce_jurisdiction_policy(
        env: &Env,
        jurisdiction: &OptionalJurisdiction,
        depositor: &Address,
        amount: i128,
    ) -> Result<(), Error> {
        if let OptionalJurisdiction::Some(config) = jurisdiction {
            if config.lock_paused {
                return Err(Error::JurisdictionPaused);
            }
            if let Some(max) = config.max_lock_amount {
                if amount > max {
                    return Err(Error::JurisdictionFundingLimitExceeded);
                }
            }
            if config.requires_kyc && !Self::is_claim_valid(env.clone(), depositor.clone()) {
                return Err(Error::JurisdictionKycRequired);
            }
            if config.enforce_identity_limits {
                Self::enforce_transaction_limit(env, depositor, amount)?;
            }
        } else {
            // Default: always enforce identity limits for generic escrows
            Self::enforce_transaction_limit(env, depositor, amount)?;
        }
        Ok(())
    }

    /// Read the stored jurisdiction config for a bounty.
    pub fn get_escrow_jurisdiction(env: Env, bounty_id: u64) -> OptionalJurisdiction {
        let escrow: Option<Escrow> = env.storage().persistent().get(&DataKey::Escrow(bounty_id));
        match escrow {
            Some(e) => e.jurisdiction,
            None => OptionalJurisdiction::None,
        }
    }

    /// Internal: Enforce transaction limit for an address
    fn enforce_transaction_limit(env: &Env, address: &Address, amount: i128) -> Result<(), Error> {
        let effective_limit = Self::get_effective_limit(env.clone(), address.clone());

        if amount > effective_limit {
            // Emit event for limit enforcement failure
            TransactionLimitEvent {
                address: address.clone(),
                amount,
                limit: effective_limit,
                status: Symbol::new(env, "exceed"),
            }
            .publish(env);
            return Err(Error::TransactionExceedsLimit);
        }

        // Emit event for successful limit check
        TransactionLimitEvent {
            address: address.clone(),
            amount,
            limit: effective_limit,
            status: Symbol::new(env, "pass"),
        }
        .publish(env);

        Ok(())
    }

    /// Lock funds: depositor must be authorized; tokens transferred from depositor to contract.
    ///
    /// # Reentrancy
    /// Protected by reentrancy guard. Escrow state is written before the
    /// inbound token transfer (CEI pattern).
    pub fn lock_funds(
        env: Env,
        depositor: Address,
        bounty_id: u64,
        amount: i128,
        deadline: u64,
    ) -> Result<(), Error> {
        Self::lock_funds_with_jurisdiction(
            env,
            depositor,
            bounty_id,
            amount,
            deadline,
            OptionalJurisdiction::None,
        )
    }

    pub fn lock_funds_with_jurisdiction(
        env: Env,
        depositor: Address,
        bounty_id: u64,
        amount: i128,
        deadline: u64,
        jurisdiction: OptionalJurisdiction,
    ) -> Result<(), Error> {
        // GUARD: acquire reentrancy lock
        reentrancy_guard::acquire(&env);

        depositor.require_auth();
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }
        if amount <= 0 {
            return Err(Error::InsufficientBalance);
        }
        if env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyExists);
        }

        // Apply jurisdictional policy
        Self::enforce_jurisdiction_policy(&env, &jurisdiction, &depositor, amount)?;

        // EFFECTS: write escrow state before external call
        let escrow = Escrow {
            depositor: depositor.clone(),
            amount,
            remaining_amount: amount,
            status: EscrowStatus::Locked,
            deadline,
            jurisdiction,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        // Update search index
        let mut index: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::EscrowIndex)
            .unwrap_or(Vec::new(&env));
        index.push_back(bounty_id);
        env.storage()
            .persistent()
            .set(&DataKey::EscrowIndex, &index);

        // INTERACTION: external token transfer is last
        let token = env
            .storage()
            .instance()
            .get::<_, Address>(&DataKey::Token)
            .unwrap();
        let contract = env.current_contract_address();
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&depositor, &contract, &amount);

        // Emit jurisdictional event if tagged
        if let OptionalJurisdiction::Some(config) = &escrow.jurisdiction {
            juris {
                bounty_id,
                tag: config.tag.clone(),
                requires_kyc: config.requires_kyc,
                enforce_identity_limits: config.enforce_identity_limits,
            }
            .publish(&env);
        }

        // GUARD: release reentrancy lock
        reentrancy_guard::release(&env);
        Ok(())
    }

    /// Release funds to contributor. Admin must be authorized. Fails if already released or refunded.
    ///
    /// # Reentrancy
    /// Protected by reentrancy guard. Escrow state is updated to
    /// `Released` *before* the outbound token transfer (CEI pattern).
    pub fn release_funds(env: Env, bounty_id: u64, contributor: Address) -> Result<(), Error> {
        // GUARD: acquire reentrancy lock
        reentrancy_guard::acquire(&env);

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();
        if escrow.status != EscrowStatus::Locked {
            return Err(Error::FundsNotLocked);
        }
        if escrow.remaining_amount <= 0 {
            return Err(Error::InsufficientBalance);
        }

        // Enforce transaction limit for contributor
        Self::enforce_transaction_limit(&env, &contributor, escrow.remaining_amount)?;

        // EFFECTS: update state before external call (CEI)
        let release_amount = escrow.remaining_amount;
        escrow.remaining_amount = 0;
        escrow.status = EscrowStatus::Released;
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        // INTERACTION: external token transfer is last
        let token = env
            .storage()
            .instance()
            .get::<_, Address>(&DataKey::Token)
            .unwrap();
        let contract = env.current_contract_address();
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&contract, &contributor, &release_amount);

        // GUARD: release reentrancy lock
        reentrancy_guard::release(&env);
        Ok(())
    }

    /// Refund remaining funds to depositor. Allowed after deadline.
    ///
    /// # Reentrancy
    /// Protected by reentrancy guard. Escrow state is updated to
    /// `Refunded` *before* the outbound token transfer (CEI pattern).
    pub fn refund(env: Env, bounty_id: u64) -> Result<(), Error> {
        // GUARD: acquire reentrancy lock
        reentrancy_guard::acquire(&env);

        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();
        if escrow.status != EscrowStatus::Locked {
            return Err(Error::FundsNotLocked);
        }
        let now = env.ledger().timestamp();
        if now < escrow.deadline {
            return Err(Error::DeadlineNotPassed);
        }
        if escrow.remaining_amount <= 0 {
            return Err(Error::InsufficientBalance);
        }

        // EFFECTS: update state before external call (CEI)
        let amount = escrow.remaining_amount;
        let depositor = escrow.depositor.clone();
        escrow.remaining_amount = 0;
        escrow.status = EscrowStatus::Refunded;
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        // INTERACTION: external token transfer is last
        let token = env
            .storage()
            .instance()
            .get::<_, Address>(&DataKey::Token)
            .unwrap();
        let contract = env.current_contract_address();
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&contract, &depositor, &amount);

        // GUARD: release reentrancy lock
        reentrancy_guard::release(&env);
        Ok(())
    }

    /// Read escrow state (for tests).
    pub fn get_escrow(env: Env, bounty_id: u64) -> Result<Escrow, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .ok_or(Error::BountyNotFound)
    }

    /// Return the total number of escrows in the search index.
    pub fn get_escrow_count(env: Env) -> u32 {
        let index: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::EscrowIndex)
            .unwrap_or(Vec::new(&env));
        index.len()
    }

    /// Paginated search over escrows.
    pub fn get_escrows(
        env: Env,
        criteria: EscrowSearchCriteria,
        cursor: Option<u64>,
        limit: u32,
    ) -> EscrowPage {
        let effective_limit = if limit == 0 || limit > MAX_PAGE_SIZE {
            MAX_PAGE_SIZE
        } else {
            limit
        };

        let index: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::EscrowIndex)
            .unwrap_or(Vec::new(&env));
        let mut records: Vec<EscrowRecord> = Vec::new(&env);
        let mut collecting = cursor.is_none();
        let mut last_processed_id: Option<u64> = None;
        let mut has_more = false;

        for id in index.iter() {
            if !collecting {
                if Some(id) == cursor {
                    collecting = true;
                }
                continue;
            }

            let escrow: Escrow = env
                .storage()
                .persistent()
                .get(&DataKey::Escrow(id))
                .unwrap();

            // Filter: status
            if criteria.status_filter != 0 {
                let status_match = match (criteria.status_filter, &escrow.status) {
                    (1, EscrowStatus::Locked) => true,
                    (2, EscrowStatus::Released) => true,
                    (3, EscrowStatus::Refunded) => true,
                    _ => false,
                };
                if !status_match {
                    continue;
                }
            }

            // Filter: depositor
            if let Some(ref dep) = criteria.depositor {
                if escrow.depositor != *dep {
                    continue;
                }
            }

            if records.len() >= effective_limit {
                has_more = true;
                break;
            }

            records.push_back(EscrowRecord {
                bounty_id: id,
                depositor: escrow.depositor.clone(),
                amount: escrow.amount,
                status: escrow.status.clone(),
            });
            last_processed_id = Some(id);
        }

        EscrowPage {
            records,
            next_cursor: if has_more { last_processed_id } else { None },
            has_more,
        }
    }
}

// ── NEW public methods ──────────────────────────────────────────────────────

impl EscrowContract {
    /// Return the contract's current token balance.
    /// Added to satisfy the standard EscrowInterface (Issue #574).
    pub fn get_balance(env: Env) -> Result<i128, Error> {
        if !env.storage().instance().has(&DataKey::Token) {
            return Err(Error::NotInitialized);
        }
        let token: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token);
        Ok(client.balance(&env.current_contract_address()))
    }

    /// Alias of `get_escrow` using the standard name from EscrowInterface.
    pub fn get_escrow_info(env: Env, bounty_id: u64) -> Result<Escrow, Error> {
        Self::get_escrow(env, bounty_id)
    }
}

// ── Standard interface traits (local definitions, Issue #574) ───────────────
//
// Mirrors the canonical trait definitions from
// contracts/bounty_escrow/contracts/escrow/src/traits.rs.
// Kept local to avoid a cross-crate dependency on bounty_escrow types.

pub mod traits {
    use super::{Error, Escrow, EscrowContract};
    use soroban_sdk::{Address, Env};

    /// Core lifecycle interface — see bounty_escrow traits.rs for full spec.
    pub trait EscrowInterface {
        fn lock_funds(
            env: &Env,
            depositor: Address,
            bounty_id: u64,
            amount: i128,
            deadline: u64,
        ) -> Result<(), Error>;
        fn release_funds(env: &Env, bounty_id: u64, contributor: Address) -> Result<(), Error>;
        fn refund(env: &Env, bounty_id: u64) -> Result<(), Error>;
        fn get_escrow_info(env: &Env, bounty_id: u64) -> Result<Escrow, Error>;
        fn get_balance(env: &Env) -> Result<i128, Error>;
    }

    /// Version interface — see bounty_escrow traits.rs for full spec.
    pub trait UpgradeInterface {
        fn get_version(env: &Env) -> u32;
    }

    impl EscrowInterface for EscrowContract {
        fn lock_funds(
            env: &Env,
            depositor: Address,
            bounty_id: u64,
            amount: i128,
            deadline: u64,
        ) -> Result<(), Error> {
            EscrowContract::lock_funds(env.clone(), depositor, bounty_id, amount, deadline)
        }
        fn release_funds(env: &Env, bounty_id: u64, contributor: Address) -> Result<(), Error> {
            EscrowContract::release_funds(env.clone(), bounty_id, contributor)
        }
        fn refund(env: &Env, bounty_id: u64) -> Result<(), Error> {
            EscrowContract::refund(env.clone(), bounty_id)
        }
        fn get_escrow_info(env: &Env, bounty_id: u64) -> Result<Escrow, Error> {
            EscrowContract::get_escrow(env.clone(), bounty_id)
        }
        fn get_balance(env: &Env) -> Result<i128, Error> {
            EscrowContract::get_balance(env.clone())
        }
    }

    impl UpgradeInterface for EscrowContract {
        /// Soroban escrow is pinned at v1 (no WASM upgrade path yet).
        fn get_version(_env: &Env) -> u32 {
            1
        }
    }
}
