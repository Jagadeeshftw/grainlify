#![no_std]
//! Minimal Soroban escrow demo: lock, release, and refund.
//! Parity with main contracts/bounty_escrow where applicable; see soroban/PARITY.md.

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, token, Address, Env, BytesN};

mod identity;
pub use identity::*;

mod reentrancy_guard;

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
        env.storage().persistent().set(&DataKey::TierLimits, &default_limits);
        env.storage().persistent().set(&DataKey::RiskThresholds, &default_thresholds);
        
        Ok(())
    }

    /// Set or update an authorized claim issuer (admin only)
    pub fn set_authorized_issuer(
        env: Env,
        issuer: Address,
        authorized: bool,
    ) -> Result<(), Error> {
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
        env.events().publish(
            (soroban_sdk::symbol_short!("issuer"), issuer.clone()),
            if authorized { soroban_sdk::symbol_short!("add") } else { soroban_sdk::symbol_short!("remove") },
        );

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

        env.storage().persistent().set(&DataKey::TierLimits, &limits);
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
            env.events().publish(
                (soroban_sdk::symbol_short!("claim"), claim.address.clone()),
                soroban_sdk::symbol_short!("expired"),
            );
            return Err(Error::ClaimExpired);
        }

        // Check if issuer is authorized
        let is_authorized: bool = env
            .storage()
            .persistent()
            .get(&DataKey::AuthorizedIssuer(claim.issuer.clone()))
            .unwrap_or(false);

        if !is_authorized {
            env.events().publish(
                (soroban_sdk::symbol_short!("claim"), claim.address.clone()),
                soroban_sdk::symbol_short!("unauth"),
            );
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

        env.storage()
            .persistent()
            .set(&DataKey::AddressIdentity(claim.address.clone()), &identity_data);

        // Emit event for successful claim submission
        env.events().publish(
            (soroban_sdk::symbol_short!("claim"), claim.address.clone()),
            (claim.tier, claim.risk_score, claim.expiry),
        );

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

    /// Internal: Enforce transaction limit for an address
    fn enforce_transaction_limit(env: &Env, address: &Address, amount: i128) -> Result<(), Error> {
        let effective_limit = Self::get_effective_limit(env.clone(), address.clone());

        if amount > effective_limit {
            // Emit event for limit enforcement failure
            env.events().publish(
                (soroban_sdk::symbol_short!("limit"), address.clone()),
                (soroban_sdk::symbol_short!("exceed"), amount, effective_limit),
            );
            return Err(Error::TransactionExceedsLimit);
        }

        // Emit event for successful limit check
        env.events().publish(
            (soroban_sdk::symbol_short!("limit"), address.clone()),
            (soroban_sdk::symbol_short!("pass"), amount, effective_limit),
        );

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

        // Enforce transaction limit based on identity tier
        Self::enforce_transaction_limit(&env, &depositor, amount)?;
        
        // EFFECTS: write escrow state before external call
        let escrow = Escrow {
            depositor: depositor.clone(),
            amount,
            remaining_amount: amount,
            status: EscrowStatus::Locked,
            deadline,
        };
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
        token_client.transfer(&depositor, &contract, &amount);

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
}

mod test;
mod identity_test;
