#![no_std]
//! Soroban program escrow contract with cursor-based search helpers.
//!
//! Search reads are backed by a persisted `ProgramIndex` list instead of
//! scanning contract storage directly. Each successful registration appends its
//! `program_id` once, and `get_programs` pages over that list in order.
//!
//! Search indexing assumptions:
//! - registrations append stable `program_id` cursor values to `ProgramIndex`
//! - the query path skips missing program records defensively
//! - callers paginate with cursors rather than requesting unbounded full scans
//! - the returned page size is clamped to `MAX_PAGE_SIZE`
//!
//! Security notes:
//! - search helpers are read-only and never mutate contract state
//! - query work is bounded by the stored index and capped page size
//! - cursor pagination keeps results reviewable and avoids hidden full scans

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, token, Address, Env, String,
    Vec,
};

mod reentrancy_guard;

const MAX_BATCH_SIZE: u32 = 20;
const MAX_PAGE_SIZE: u32 = 20;
const MAX_LABELS: u32 = 10;
const MAX_LABEL_LENGTH: u32 = 32;
const PROGRAM_REGISTERED: soroban_sdk::Symbol = symbol_short!("prg_reg");
const PROGRAM_REGISTRY: soroban_sdk::Symbol = symbol_short!("prg_rgst");
const PROGRAM_LABELS_UPDATED: soroban_sdk::Symbol = symbol_short!("prg_lbl");
const LABEL_CONFIG_UPDATED: soroban_sdk::Symbol = symbol_short!("lbl_cfg");

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
/// Canonical error enum for program-escrow public API.
///
/// All public entrypoints return `Result<T, Error>` to provide stable,
/// machine-readable error codes for clients, indexers, and integrators.
/// Error codes are stable across contract versions and should be used
/// for programmatic error handling.
///
/// # Security Notes
///
/// - Error messages never expose sensitive data (keys, balances, internal state)
/// - Error codes are deterministic and stable for client-side discrimination
/// - All errors are safe to log and display to end users
pub enum Error {
    /// Contract has already been initialized.
    ///
    /// **When raised:** Attempting to call `init()` on an already-initialized contract.
    /// **Client action:** None required; contract is ready for use.
    AlreadyInitialized = 1,

    /// Contract has not been initialized.
    ///
    /// **When raised:** Calling any mutating function before `init()` is called.
    /// **Client action:** Call `init()` with admin and token addresses first.
    NotInitialized = 2,

    /// Program with this ID already exists.
    ///
    /// **When raised:** Attempting to register a program with a `program_id`
    /// that is already in use.
    /// **Client action:** Use a different `program_id` or query existing programs
    /// to find available IDs.
    ProgramExists = 3,

    /// Program with this ID does not exist.
    ///
    /// **When raised:** Attempting to read, update, or query a program that
    /// has not been registered.
    /// **Client action:** Verify the `program_id` is correct and that the
    /// program has been registered.
    ProgramNotFound = 4,

    /// Caller is not authorized to perform this action.
    ///
    /// **When raised:** Attempting to perform an admin-only action without
    /// proper authorization, or updating a program without being the contract
    /// admin or program admin.
    /// **Client action:** Ensure the correct address is signing the transaction.
    Unauthorized = 5,

    /// Invalid batch size.
    ///
    /// **When raised:** Batch registration with 0 items or exceeding `MAX_BATCH_SIZE`.
    /// **Client action:** Ensure batch contains 1-20 items (inclusive).
    InvalidBatchSize = 6,

    /// Duplicate program ID within a single batch.
    ///
    /// **When raised:** Multiple items in a batch registration have the same
    /// `program_id`.
    /// **Client action:** Ensure all `program_id` values in the batch are unique.
    DuplicateProgramId = 7,

    /// Invalid funding amount.
    ///
    /// **When raised:** `total_funding` is zero or negative.
    /// **Client action:** Provide a positive funding amount.
    InvalidAmount = 8,

    /// Invalid program name.
    ///
    /// **When raised:** Program name is empty (zero length).
    /// **Client action:** Provide a non-empty program name.
    InvalidName = 9,

    /// Contract is deprecated and no longer accepts new registrations.
    ///
    /// **When raised:** Attempting to register a program after deprecation
    /// has been enabled.
    /// **Client action:** Check deprecation status before registration;
    /// migrate to the target contract if one is specified.
    ContractDeprecated = 10,

    /// KYC attestation required by jurisdiction.
    ///
    /// **When raised:** Program jurisdiction requires KYC but `kyc_attested`
    /// is `None` or `false`.
    /// **Client action:** Provide valid KYC attestation before registration.
    JurisdictionKycRequired = 11,

    /// Funding exceeds jurisdiction limit.
    ///
    /// **When raised:** `total_funding` exceeds the `max_funding` configured
    /// for the program's jurisdiction.
    /// **Client action:** Reduce funding amount or update jurisdiction limits.
    JurisdictionFundingLimitExceeded = 12,

    /// Registration paused for this jurisdiction.
    ///
    /// **When raised:** Program jurisdiction has `registration_paused` set to `true`.
    /// **Client action:** Wait for jurisdiction to resume registration or use
    /// a different jurisdiction.
    JurisdictionPaused = 13,

    /// Invalid label format.
    ///
    /// **When raised:** Label is empty or exceeds `MAX_LABEL_LENGTH` (32 chars).
    /// **Client action:** Ensure labels are 1-32 characters in length.
    InvalidLabel = 14,

    /// Too many labels.
    ///
    /// **When raised:** Attempting to add more than `MAX_LABELS` (10) labels
    /// to a program.
    /// **Client action:** Reduce the number of labels to 10 or fewer.
    TooManyLabels = 15,

    /// Label not in allowed list.
    ///
    /// **When raised:** Label configuration is restricted and the provided
    /// label is not in the `allowed_labels` list.
    /// **Client action:** Use an allowed label or request admin to update
    /// the label configuration.
    LabelNotAllowed = 16,
    // Ownership transfer errors
    TransferProposalNotFound = 17,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProgramStatus {
    Active,
    Completed,
    Cancelled,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramJurisdictionConfig {
    pub tag: Option<String>,
    pub requires_kyc: bool,
    pub max_funding: Option<i128>,
    pub registration_paused: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OptionalJurisdiction {
    None,
    Some(ProgramJurisdictionConfig),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Program {
    pub admin: Address,
    pub payout_key: Address,
    pub name: String,
    pub total_funding: i128,
    pub status: ProgramStatus,
    pub jurisdiction: OptionalJurisdiction,
    pub labels: Vec<String>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramRegistrationItem {
    pub program_id: u64,
    pub admin: Address,
    pub name: String,
    pub total_funding: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeprecationState {
    pub deprecated: bool,
    pub migration_target: Option<Address>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramRegistrationWithJurisdictionItem {
    pub program_id: u64,
    pub admin: Address,
    pub name: String,
    pub total_funding: i128,
    pub juris_tag: Option<String>,
    pub juris_requires_kyc: bool,
    pub juris_max_funding: Option<i128>,
    pub juris_registration_paused: bool,
    pub jurisdiction: OptionalJurisdiction,
    pub kyc_attested: Option<bool>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramRegisteredEvent {
    pub version: u32,
    pub program_id: u64,
    pub admin: Address,
    pub total_funding: i128,
    pub jurisdiction_tag: Option<String>,
    pub requires_kyc: bool,
    pub max_funding: Option<i128>,
    pub registration_paused: bool,
    pub labels: Vec<String>,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramRegistryEvent {
    pub version: u32,
    pub program_id: u64,
    pub admin: Address,
    pub payout_key: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LabelConfig {
    pub restricted: bool,
    pub allowed_labels: Vec<String>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramLabelsUpdatedEvent {
    pub version: u32,
    pub program_id: u64,
    pub actor: Address,
    pub labels: Vec<String>,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LabelConfigUpdatedEvent {
    pub version: u32,
    pub admin: Address,
    pub restricted: bool,
    pub allowed_labels: Vec<String>,
    pub timestamp: u64,
}

#[contracttype]
pub enum DataKey {
    Admin,
    Token,
    Program(u64),
    /// Jurisdiction config is stored separately from the main program record.
    ProgramJurisdiction(u64),
    /// Stable index used by `get_programs` and `get_program_count`.
    ProgramIndex,
    DeprecationState,
    LabelConfig,
    ReentrancyGuard,
    // Ownership transfer
    PendingAdmin,
    ProgramPendingAdmin(u64),
}

/// Filter inputs for cursor-based program search.
///
/// `status_filter` values:
/// - `0`: any status
/// - `1`: active
/// - `2`: completed
/// - `3`: cancelled
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramSearchCriteria {
    pub status_filter: u32,
    pub admin: Option<Address>,
}

/// One flattened program entry returned from a search page.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramRecord {
    pub program_id: u64,
    pub admin: Address,
    pub name: String,
    pub total_funding: i128,
    pub status: ProgramStatus,
}

/// One page of program search results.
///
/// When `has_more` is true, pass `next_cursor` back into `get_programs` to
/// continue scanning from the next indexed program.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramPage {
    pub records: Vec<ProgramRecord>,
    pub next_cursor: Option<u64>,
    pub has_more: bool,
}

#[contract]
pub struct ProgramEscrowContract;

#[contractimpl]
impl ProgramEscrowContract {
    fn validate_program_input(name: &String, total_funding: i128) -> Result<(), Error> {
        if total_funding <= 0 {
            return Err(Error::InvalidAmount);
        }
        if name.len() == 0 {
            return Err(Error::InvalidName);
        }
        Ok(())
    }

    fn build_jurisdiction(
        juris_tag: Option<String>,
        juris_requires_kyc: bool,
        juris_max_funding: Option<i128>,
        juris_registration_paused: bool,
        jurisdiction: OptionalJurisdiction,
    ) -> OptionalJurisdiction {
        match jurisdiction {
            OptionalJurisdiction::Some(config) => OptionalJurisdiction::Some(config),
            OptionalJurisdiction::None => {
                let has_juris = juris_tag.is_some()
                    || juris_requires_kyc
                    || juris_max_funding.is_some()
                    || juris_registration_paused;
                if has_juris {
                    OptionalJurisdiction::Some(ProgramJurisdictionConfig {
                        tag: juris_tag,
                        requires_kyc: juris_requires_kyc,
                        max_funding: juris_max_funding,
                        registration_paused: juris_registration_paused,
                    })
                } else {
                    OptionalJurisdiction::None
                }
            }
        }
    }

    fn enforce_jurisdiction_rules(
        jurisdiction: &OptionalJurisdiction,
        total_funding: i128,
        kyc_attested: Option<bool>,
    ) -> Result<(), Error> {
        if let OptionalJurisdiction::Some(config) = jurisdiction {
            if config.registration_paused {
                return Err(Error::JurisdictionPaused);
            }

            if let Some(max_funding) = config.max_funding {
                if total_funding > max_funding {
                    return Err(Error::JurisdictionFundingLimitExceeded);
                }
            }

            if config.requires_kyc && !kyc_attested.unwrap_or(false) {
                return Err(Error::JurisdictionKycRequired);
            }
        }
        Ok(())
    }

    fn emit_program_registered(
        env: &Env,
        program_id: u64,
        admin: Address,
        total_funding: i128,
        jurisdiction: &OptionalJurisdiction,
        labels: &Vec<String>,
    ) {
        let (jurisdiction_tag, requires_kyc, max_funding, registration_paused) =
            if let OptionalJurisdiction::Some(config) = jurisdiction {
                (
                    config.tag.clone(),
                    config.requires_kyc,
                    config.max_funding,
                    config.registration_paused,
                )
            } else {
                (None, false, None, false)
            };

        env.events().publish(
            (PROGRAM_REGISTERED, program_id),
            ProgramRegisteredEvent {
                version: 2,
                program_id,
                admin,
                total_funding,
                jurisdiction_tag,
                requires_kyc,
                max_funding,
                registration_paused,
                labels: labels.clone(),
                timestamp: env.ledger().timestamp(),
            },
        );
    }

    fn default_label_config(env: &Env) -> LabelConfig {
        LabelConfig {
            restricted: false,
            allowed_labels: Vec::new(env),
        }
    }

    fn get_label_config_internal(env: &Env) -> LabelConfig {
        env.storage()
            .persistent()
            .get(&DataKey::LabelConfig)
            .unwrap_or_else(|| Self::default_label_config(env))
    }

    fn validate_single_label(label: &String) -> Result<(), Error> {
        if label.len() == 0 || label.len() > MAX_LABEL_LENGTH {
            return Err(Error::InvalidLabel);
        }
        Ok(())
    }

    fn normalize_labels(env: &Env, labels: Vec<String>) -> Result<Vec<String>, Error> {
        if labels.len() > MAX_LABELS {
            return Err(Error::TooManyLabels);
        }

        let config = Self::get_label_config_internal(env);
        let mut normalized = Vec::new(env);

        for label in labels.iter() {
            Self::validate_single_label(&label)?;

            let mut exists = false;
            for existing in normalized.iter() {
                if existing == label {
                    exists = true;
                    break;
                }
            }
            if exists {
                continue;
            }

            if config.restricted {
                let mut allowed = false;
                for candidate in config.allowed_labels.iter() {
                    if candidate == label {
                        allowed = true;
                        break;
                    }
                }
                if !allowed {
                    return Err(Error::LabelNotAllowed);
                }
            }

            normalized.push_back(label);
        }

        Ok(normalized)
    }

    fn sanitize_label_config(env: &Env, labels: Vec<String>) -> Result<Vec<String>, Error> {
        if labels.len() > MAX_LABELS {
            return Err(Error::TooManyLabels);
        }

        let mut normalized = Vec::new(env);
        for label in labels.iter() {
            Self::validate_single_label(&label)?;

            let mut exists = false;
            for existing in normalized.iter() {
                if existing == label {
                    exists = true;
                    break;
                }
            }
            if !exists {
                normalized.push_back(label);
            }
        }

        Ok(normalized)
    }

    fn order_batch_registration_items(
        env: &Env,
        items: &Vec<ProgramRegistrationItem>,
    ) -> Vec<ProgramRegistrationItem> {
        let mut ordered: Vec<ProgramRegistrationItem> = Vec::new(env);
        for item in items.iter() {
            let mut next: Vec<ProgramRegistrationItem> = Vec::new(env);
            let mut inserted = false;
            for existing in ordered.iter() {
                if !inserted && item.program_id < existing.program_id {
                    next.push_back(item.clone());
                    inserted = true;
                }
                next.push_back(existing);
            }
            if !inserted {
                next.push_back(item.clone());
            }
            ordered = next;
        }
        ordered
    }

    fn ensure_initialized(env: &Env) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            Ok(())
        } else {
            Err(Error::NotInitialized)
        }
    }

    fn ensure_not_deprecated(env: &Env) -> Result<(), Error> {
        if Self::get_deprecation_state(env).deprecated {
            Err(Error::ContractDeprecated)
        } else {
            Ok(())
        }
    }

    fn require_contract_admin(env: &Env) -> Address {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        admin
    }

    fn append_program_id(env: &Env, program_id: u64) {
        let mut index: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::ProgramIndex)
            .unwrap_or_else(|| Vec::new(env));
        index.push_back(program_id);
        env.storage()
            .persistent()
            .set(&DataKey::ProgramIndex, &index);
    }

    fn store_program(env: &Env, program_id: u64, program: &Program) {
        env.storage()
            .persistent()
            .set(&DataKey::Program(program_id), program);

        match &program.jurisdiction {
            OptionalJurisdiction::Some(config) => env
                .storage()
                .persistent()
                .set(&DataKey::ProgramJurisdiction(program_id), config),
            OptionalJurisdiction::None => {
                env.storage()
                    .persistent()
                    .remove(&DataKey::ProgramJurisdiction(program_id));
            }
        }
    }

    fn get_deprecation_state(env: &Env) -> DeprecationState {
        env.storage()
            .instance()
            .get(&DataKey::DeprecationState)
            .unwrap_or(DeprecationState {
                deprecated: false,
                migration_target: None,
            })
    }

    /// Initialize the contract with an admin and token address. Call once.
    pub fn init(env: Env, admin: Address, token: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);
        env.storage()
            .persistent()
            .set(&DataKey::LabelConfig, &Self::default_label_config(&env));
        Ok(())
    }

    /// Initialize a single program with a payout key.
    pub fn init_program(
        env: Env,
        program_id: u64,
        admin: Address,
        payout_key: Address,
    ) -> Result<(), Error> {
        Self::ensure_initialized(&env)?;
        Self::ensure_not_deprecated(&env)?;
        Self::require_contract_admin(&env);

        if env.storage().persistent().has(&DataKey::Program(program_id)) {
            return Err(Error::ProgramExists);
        }

        // Emit registry event for discovery
        env.events().publish(
            (PROGRAM_REGISTRY, program_id),
            ProgramRegistryEvent {
                version: 1,
                program_id,
                admin: admin.clone(),
                payout_key: payout_key.clone(),
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    /// Register a single program.
    pub fn register_program(
        env: Env,
        program_id: u64,
        admin: Address,
        name: String,
        total_funding: i128,
    ) -> Result<(), Error> {
        let empty_labels = Vec::new(&env);
        Self::register_program_with_labels(
            env,
            program_id,
            admin,
            name,
            total_funding,
            empty_labels,
        )
    }

    /// Register a single program with labels.
    pub fn register_program_with_labels(
        env: Env,
        program_id: u64,
        admin: Address,
        name: String,
        total_funding: i128,
        labels: Vec<String>,
    ) -> Result<(), Error> {
        Self::register_program_internal(
            env,
            program_id,
            admin,
            name,
            total_funding,
            None,
            false,
            None,
            false,
            OptionalJurisdiction::None,
            None,
            labels,
        )
    }

    /// Register a single program with optional jurisdiction controls.
    pub fn register_program_juris(
        env: Env,
        program_id: u64,
        admin: Address,
        name: String,
        total_funding: i128,
        juris_tag: Option<String>,
        juris_requires_kyc: bool,
        juris_max_funding: Option<i128>,
        juris_registration_paused: bool,
        jurisdiction: OptionalJurisdiction,
        kyc_attested: Option<bool>,
    ) -> Result<(), Error> {
        let empty_labels = Vec::new(&env);
        Self::register_program_internal(
            env,
            program_id,
            admin,
            name,
            total_funding,
            juris_tag,
            juris_requires_kyc,
            juris_max_funding,
            juris_registration_paused,
            jurisdiction,
            kyc_attested,
            empty_labels,
        )
    }

    fn register_program_internal(
        env: Env,
        program_id: u64,
        admin: Address,
        name: String,
        total_funding: i128,
        juris_tag: Option<String>,
        juris_requires_kyc: bool,
        juris_max_funding: Option<i128>,
        juris_registration_paused: bool,
        jurisdiction: OptionalJurisdiction,
        kyc_attested: Option<bool>,
        labels: Vec<String>,
    ) -> Result<(), Error> {
        // GUARD: acquire reentrancy lock
        reentrancy_guard::acquire(&env);

        if let Err(e) = Self::ensure_initialized(&env) {
            return Err(e);
        }
        if let Err(e) = Self::ensure_not_deprecated(&env) {
            return Err(e);
        }
        Self::require_contract_admin(&env);

        if env
            .storage()
            .persistent()
            .has(&DataKey::Program(program_id))
        {
            return Err(Error::ProgramExists);
        }

        Self::validate_program_input(&name, total_funding)?;

        let jurisdiction = Self::build_jurisdiction(
            juris_tag,
            juris_requires_kyc,
            juris_max_funding,
            juris_registration_paused,
            jurisdiction,
        );
        Self::enforce_jurisdiction_rules(&jurisdiction, total_funding, kyc_attested)?;
        let labels = Self::normalize_labels(&env, labels)?;

        admin.require_auth();

        // EFFECTS: write program state before external call
        let program = Program {
            admin: admin.clone(),
            payout_key: admin.clone(), // Default payout_key to admin for backward compatibility
            name,
            total_funding,
            status: ProgramStatus::Active,
            jurisdiction: jurisdiction.clone(),
            labels: labels.clone(),
        };
        Self::store_program(&env, program_id, &program);
        Self::append_program_id(&env, program_id);

        Self::emit_program_registered(
            &env,
            program_id,
            admin.clone(),
            total_funding,
            &jurisdiction,
            &labels,
        );
        env.events().publish(
            (PROGRAM_LABELS_UPDATED, program_id),
            ProgramLabelsUpdatedEvent {
                version: 1,
                program_id,
                actor: admin.clone(),
                labels,
                timestamp: env.ledger().timestamp(),
            },
        );

        // INTERACTION: external token transfer is last
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token_addr);
        token_client.transfer(&admin, &env.current_contract_address(), &total_funding);

        // GUARD: release reentrancy lock
        reentrancy_guard::release(&env);
        Ok(())
    }

    /// Backward-compatible alias.
    pub fn register_prog_w_juris(
        env: Env,
        program_id: u64,
        admin: Address,
        name: String,
        total_funding: i128,
        juris_tag: Option<String>,
        juris_requires_kyc: bool,
        juris_max_funding: Option<i128>,
        juris_registration_paused: bool,
        jurisdiction: OptionalJurisdiction,
        kyc_attested: Option<bool>,
    ) -> Result<(), Error> {
        Self::register_program_juris(
            env,
            program_id,
            admin,
            name,
            total_funding,
            juris_tag,
            juris_requires_kyc,
            juris_max_funding,
            juris_registration_paused,
            jurisdiction,
            kyc_attested,
        )
    }

    /// Batch register multiple programs in a single transaction.
    pub fn batch_register_programs(
        env: Env,
        items: Vec<ProgramRegistrationItem>,
    ) -> Result<u32, Error> {
        let batch_size = items.len() as u32;
        if batch_size == 0 || batch_size > MAX_BATCH_SIZE {
            return Err(Error::InvalidBatchSize);
        }

        Self::ensure_initialized(&env)?;
        Self::ensure_not_deprecated(&env)?;
        Self::require_contract_admin(&env);

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token_addr);
        let contract_address = env.current_contract_address();
        let ordered_items = Self::order_batch_registration_items(&env, &items);

        for item in ordered_items.iter() {
            if env
                .storage()
                .persistent()
                .has(&DataKey::Program(item.program_id))
            {
                return Err(Error::ProgramExists);
            }
            Self::validate_program_input(&item.name, item.total_funding)?;

            let mut count = 0u32;
            for other in ordered_items.iter() {
                if other.program_id == item.program_id {
                    count += 1;
                }
            }
            if count > 1 {
                return Err(Error::DuplicateProgramId);
            }
        }

        let mut seen_admins: Vec<Address> = Vec::new(&env);
        for item in ordered_items.iter() {
            let mut found = false;
            for seen in seen_admins.iter() {
                if seen == item.admin {
                    found = true;
                    break;
                }
            }
            if !found {
                seen_admins.push_back(item.admin.clone());
                item.admin.require_auth();
            }
        }

        let mut registered_count = 0u32;
        for item in ordered_items.iter() {
            token_client.transfer(&item.admin, &contract_address, &item.total_funding);

            let program = Program {
                admin: item.admin.clone(),
                payout_key: item.admin.clone(),
                name: item.name.clone(),
                total_funding: item.total_funding,
                status: ProgramStatus::Active,
                jurisdiction: OptionalJurisdiction::None,
                labels: Vec::new(&env),
            };
            Self::store_program(&env, item.program_id, &program);
            Self::append_program_id(&env, item.program_id);
            Self::emit_program_registered(
                &env,
                item.program_id,
                item.admin.clone(),
                item.total_funding,
                &OptionalJurisdiction::None,
                &program.labels,
            );
            registered_count += 1;
        }

        Ok(registered_count)
    }

    /// Batch register programs with optional jurisdiction controls.
    pub fn batch_register_juris(
        env: Env,
        items: Vec<ProgramRegistrationWithJurisdictionItem>,
    ) -> Result<u32, Error> {
        let batch_size = items.len() as u32;
        if batch_size == 0 || batch_size > MAX_BATCH_SIZE {
            return Err(Error::InvalidBatchSize);
        }

        Self::ensure_initialized(&env)?;
        Self::ensure_not_deprecated(&env)?;
        Self::require_contract_admin(&env);

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token_addr);
        let contract_address = env.current_contract_address();

        for item in items.iter() {
            if env
                .storage()
                .persistent()
                .has(&DataKey::Program(item.program_id))
            {
                return Err(Error::ProgramExists);
            }
            Self::validate_program_input(&item.name, item.total_funding)?;

            let jurisdiction = Self::build_jurisdiction(
                item.juris_tag.clone(),
                item.juris_requires_kyc,
                item.juris_max_funding,
                item.juris_registration_paused,
                item.jurisdiction.clone(),
            );
            Self::enforce_jurisdiction_rules(&jurisdiction, item.total_funding, item.kyc_attested)?;

            let mut count = 0u32;
            for other in items.iter() {
                if other.program_id == item.program_id {
                    count += 1;
                }
            }
            if count > 1 {
                return Err(Error::DuplicateProgramId);
            }
        }

        let mut seen_admins: Vec<Address> = Vec::new(&env);
        for item in items.iter() {
            let mut found = false;
            for seen in seen_admins.iter() {
                if seen == item.admin {
                    found = true;
                    break;
                }
            }
            if !found {
                seen_admins.push_back(item.admin.clone());
                item.admin.require_auth();
            }
        }

        let mut registered_count = 0u32;
        for item in items.iter() {
            token_client.transfer(&item.admin, &contract_address, &item.total_funding);

            let jurisdiction = Self::build_jurisdiction(
                item.juris_tag.clone(),
                item.juris_requires_kyc,
                item.juris_max_funding,
                item.juris_registration_paused,
                item.jurisdiction.clone(),
            );
            let program = Program {
                admin: item.admin.clone(),
                payout_key: item.admin.clone(),
                name: item.name.clone(),
                total_funding: item.total_funding,
                status: ProgramStatus::Active,
                jurisdiction: jurisdiction.clone(),
                labels: Vec::new(&env),
            };
            Self::store_program(&env, item.program_id, &program);
            Self::append_program_id(&env, item.program_id);
            Self::emit_program_registered(
                &env,
                item.program_id,
                item.admin.clone(),
                item.total_funding,
                &jurisdiction,
                &program.labels,
            );
            registered_count += 1;
        }

        Ok(registered_count)
    }

    /// Backward-compatible alias.
    pub fn batch_reg_progs_w_juris(
        env: Env,
        items: Vec<ProgramRegistrationWithJurisdictionItem>,
    ) -> Result<u32, Error> {
        Self::batch_register_juris(env, items)
    }

    /// Read a program's state.
    pub fn get_program(env: Env, program_id: u64) -> Result<Program, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Program(program_id))
            .ok_or(Error::ProgramNotFound)
    }

    /// Set deprecation and optional migration target.
    ///
    /// Deprecation blocks new registrations while preserving read/query access.
    pub fn set_deprecated(
        env: Env,
        deprecated: bool,
        migration_target: Option<Address>,
    ) -> Result<(), Error> {
        Self::ensure_initialized(&env)?;
        let admin = Self::require_contract_admin(&env);

        let state = DeprecationState {
            deprecated,
            migration_target: migration_target.clone(),
        };
        env.storage()
            .instance()
            .set(&DataKey::DeprecationState, &state);
        env.events().publish(
            (symbol_short!("deprec"),),
            (
                state.deprecated,
                state.migration_target,
                admin,
                env.ledger().timestamp(),
            ),
        );
        Ok(())
    }

    pub fn get_deprecation_status(env: Env) -> DeprecationState {
        Self::get_deprecation_state(&env)
    }

    /// Read the stored jurisdiction config for a program.
    pub fn get_program_jurisdiction(
        env: Env,
        program_id: u64,
    ) -> Result<Option<ProgramJurisdictionConfig>, Error> {
        if !env
            .storage()
            .persistent()
            .has(&DataKey::Program(program_id))
        {
            return Err(Error::ProgramNotFound);
        }
        Ok(env
            .storage()
            .persistent()
            .get(&DataKey::ProgramJurisdiction(program_id)))
    }

    pub fn get_label_config(env: Env) -> LabelConfig {
        Self::get_label_config_internal(&env)
    }

    pub fn set_label_config(
        env: Env,
        restricted: bool,
        allowed_labels: Vec<String>,
    ) -> Result<LabelConfig, Error> {
        Self::ensure_initialized(&env)?;
        let admin = Self::require_contract_admin(&env);
        let normalized = Self::sanitize_label_config(&env, allowed_labels)?;
        let config = LabelConfig {
            restricted,
            allowed_labels: normalized.clone(),
        };
        env.storage()
            .persistent()
            .set(&DataKey::LabelConfig, &config);
        env.events().publish(
            (LABEL_CONFIG_UPDATED,),
            LabelConfigUpdatedEvent {
                version: 1,
                admin,
                restricted,
                allowed_labels: normalized,
                timestamp: env.ledger().timestamp(),
            },
        );
        Ok(config)
    }

    pub fn update_program_labels(
        env: Env,
        actor: Address,
        program_id: u64,
        labels: Vec<String>,
    ) -> Result<Program, Error> {
        Self::ensure_initialized(&env)?;
        let contract_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        let mut program: Program = env
            .storage()
            .persistent()
            .get(&DataKey::Program(program_id))
            .ok_or(Error::ProgramNotFound)?;

        if actor != contract_admin && actor != program.admin {
            return Err(Error::Unauthorized);
        }
        actor.require_auth();

        let labels = Self::normalize_labels(&env, labels)?;
        program.labels = labels.clone();
        Self::store_program(&env, program_id, &program);
        env.events().publish(
            (PROGRAM_LABELS_UPDATED, program_id),
            ProgramLabelsUpdatedEvent {
                version: 1,
                program_id,
                actor,
                labels,
                timestamp: env.ledger().timestamp(),
            },
        );
        Ok(program)
    }

    pub fn get_programs_by_label(
        env: Env,
        label: String,
        cursor: Option<u64>,
        limit: u32,
    ) -> ProgramPage {
        let effective_limit = if limit == 0 || limit > MAX_PAGE_SIZE {
            MAX_PAGE_SIZE
        } else {
            limit
        };
        let index: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::ProgramIndex)
            .unwrap_or_else(|| Vec::new(&env));

        let mut records: Vec<ProgramRecord> = Vec::new(&env);
        let mut collecting = cursor.is_none();
        let mut next_cursor = None;
        let mut has_more = false;

        for i in 0..index.len() {
            let id = index.get(i).unwrap();

            if !collecting {
                if Some(id) == cursor {
                    collecting = true;
                }
                continue;
            }

            let Some(program) = env
                .storage()
                .persistent()
                .get::<_, Program>(&DataKey::Program(id))
            else {
                continue;
            };

            let mut matches = false;
            for program_label in program.labels.iter() {
                if program_label == label {
                    matches = true;
                    break;
                }
            }
            if !matches {
                continue;
            }

            if records.len() >= effective_limit {
                has_more = true;
                break;
            }

            next_cursor = Some(id);
            records.push_back(ProgramRecord {
                program_id: id,
                admin: program.admin,
                name: program.name,
                total_funding: program.total_funding,
                status: program.status,
            });
        }

        if !has_more {
            next_cursor = None;
        }

        ProgramPage {
            records,
            next_cursor,
            has_more,
        }
    }

    /// Return the total number of program ids tracked in the search index.
    pub fn get_program_count(env: Env) -> u32 {
        let index: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::ProgramIndex)
            .unwrap_or_else(|| Vec::new(&env));
        index.len()
    }

    /// Paginated search over programs using the persisted `ProgramIndex`.
    ///
    /// Cursor semantics:
    /// - `None` starts from the beginning of the index
    /// - `Some(program_id)` resumes after that indexed id
    /// - an unknown cursor returns an empty page rather than falling back to a full scan
    ///
    /// Limit semantics:
    /// - `0` is treated as `MAX_PAGE_SIZE`
    /// - values above `MAX_PAGE_SIZE` are clamped
    pub fn get_programs(
        env: Env,
        criteria: ProgramSearchCriteria,
        cursor: Option<u64>,
        limit: u32,
    ) -> ProgramPage {
        let effective_limit = if limit == 0 || limit > MAX_PAGE_SIZE {
            MAX_PAGE_SIZE
        } else {
            limit
        };

        let status_match = match criteria.status_filter {
            1 => Some(ProgramStatus::Active),
            2 => Some(ProgramStatus::Completed),
            3 => Some(ProgramStatus::Cancelled),
            _ => None,
        };

        let index: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::ProgramIndex)
            .unwrap_or_else(|| Vec::new(&env));

        let mut records: Vec<ProgramRecord> = Vec::new(&env);
        let mut collecting = cursor.is_none();
        let mut next_cursor: Option<u64> = None;
        let mut has_more = false;

        for i in 0..index.len() {
            let id = index.get(i).unwrap();

            if !collecting {
                if Some(id) == cursor {
                    collecting = true;
                }
                continue;
            }

            let Some(program) = env
                .storage()
                .persistent()
                .get::<_, Program>(&DataKey::Program(id))
            else {
                continue;
            };

            if let Some(ref status) = status_match {
                if program.status != *status {
                    continue;
                }
            }

            if let Some(ref admin) = criteria.admin {
                if program.admin != *admin {
                    continue;
                }
            }

            if records.len() >= effective_limit {
                has_more = true;
                break;
            }

            next_cursor = Some(id);
            records.push_back(ProgramRecord {
                program_id: id,
                admin: program.admin,
                name: program.name,
                total_funding: program.total_funding,
                status: program.status,
            });
        }

        if !has_more {
            next_cursor = None;
        }

        ProgramPage {
            records,
            next_cursor,
            has_more,
        }
    }
}

// ── Ownership transfer (two-step propose / accept) ──────────────────────────

#[contractimpl]
impl ProgramEscrowContract {
    // ── Contract-level admin transfer ───────────────────────────────────────

    /// Propose transferring contract-level admin to `new_owner`.
    /// Only the current admin may call this. Overwrites any prior pending proposal.
    ///
    /// Time complexity: O(1)  Space complexity: O(1)
    pub fn propose_transfer_ownership(env: Env, new_owner: Address) -> Result<(), Error> {
        let admin = Self::require_contract_admin(&env);
        Self::ensure_initialized(&env)?;

        env.storage()
            .instance()
            .set(&DataKey::PendingAdmin, &new_owner);

        env.events().publish(
            (symbol_short!("own_prop"), admin),
            (new_owner, env.ledger().timestamp()),
        );
        Ok(())
    }

    /// Accept a pending contract-level admin transfer.
    /// Only the proposed new owner may call this.
    ///
    /// Time complexity: O(1)  Space complexity: O(1)
    pub fn accept_transfer_ownership(env: Env) -> Result<(), Error> {
        Self::ensure_initialized(&env)?;
        let pending: Address = env
            .storage()
            .instance()
            .get(&DataKey::PendingAdmin)
            .ok_or(Error::TransferProposalNotFound)?;
        pending.require_auth();

        let old_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        env.storage().instance().set(&DataKey::Admin, &pending);
        env.storage().instance().remove(&DataKey::PendingAdmin);

        env.events().publish(
            (symbol_short!("own_xfer"), old_admin),
            (pending, env.ledger().timestamp()),
        );
        Ok(())
    }

    /// Cancel a pending contract-level admin transfer.
    /// Only the current admin may call this.
    ///
    /// Time complexity: O(1)  Space complexity: O(1)
    pub fn cancel_transfer_ownership(env: Env) -> Result<(), Error> {
        Self::ensure_initialized(&env)?;
        let admin = Self::require_contract_admin(&env);

        if !env.storage().instance().has(&DataKey::PendingAdmin) {
            return Err(Error::TransferProposalNotFound);
        }
        env.storage().instance().remove(&DataKey::PendingAdmin);

        env.events()
            .publish((symbol_short!("own_cncl"), admin), env.ledger().timestamp());
        Ok(())
    }

    /// Read the pending contract-level admin, if any.
    ///
    /// Time complexity: O(1)  Space complexity: O(1)
    pub fn get_pending_owner(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::PendingAdmin)
    }

    // ── Per-program admin transfer ─────────────────────────────────────────

    /// Propose transferring a program's admin to `new_admin`.
    /// Only the current program admin may call this.
    ///
    /// Time complexity: O(1)  Space complexity: O(1)
    pub fn propose_program_transfer(
        env: Env,
        program_id: u64,
        new_admin: Address,
    ) -> Result<(), Error> {
        Self::ensure_initialized(&env)?;
        let program: Program = env
            .storage()
            .persistent()
            .get(&DataKey::Program(program_id))
            .ok_or(Error::ProgramNotFound)?;
        program.admin.require_auth();

        env.storage()
            .persistent()
            .set(&DataKey::ProgramPendingAdmin(program_id), &new_admin);

        env.events().publish(
            (symbol_short!("prg_prop"), program_id),
            (program.admin, new_admin, env.ledger().timestamp()),
        );
        Ok(())
    }

    /// Accept a pending program admin transfer.
    /// Only the proposed new admin may call this.
    ///
    /// Time complexity: O(1)  Space complexity: O(1)
    pub fn accept_program_transfer(env: Env, program_id: u64) -> Result<(), Error> {
        Self::ensure_initialized(&env)?;
        let pending: Address = env
            .storage()
            .persistent()
            .get(&DataKey::ProgramPendingAdmin(program_id))
            .ok_or(Error::TransferProposalNotFound)?;
        pending.require_auth();

        let mut program: Program = env
            .storage()
            .persistent()
            .get(&DataKey::Program(program_id))
            .ok_or(Error::ProgramNotFound)?;
        let old_admin = program.admin.clone();
        program.admin = pending.clone();
        Self::store_program(&env, program_id, &program);
        env.storage()
            .persistent()
            .remove(&DataKey::ProgramPendingAdmin(program_id));

        env.events().publish(
            (symbol_short!("prg_xfer"), program_id),
            (old_admin, pending, env.ledger().timestamp()),
        );
        Ok(())
    }

    /// Cancel a pending program admin transfer.
    /// Only the current program admin may call this.
    ///
    /// Time complexity: O(1)  Space complexity: O(1)
    pub fn cancel_program_transfer(env: Env, program_id: u64) -> Result<(), Error> {
        Self::ensure_initialized(&env)?;
        let program: Program = env
            .storage()
            .persistent()
            .get(&DataKey::Program(program_id))
            .ok_or(Error::ProgramNotFound)?;
        program.admin.require_auth();

        if !env
            .storage()
            .persistent()
            .has(&DataKey::ProgramPendingAdmin(program_id))
        {
            return Err(Error::TransferProposalNotFound);
        }
        env.storage()
            .persistent()
            .remove(&DataKey::ProgramPendingAdmin(program_id));

        env.events().publish(
            (symbol_short!("prg_cncl"), program_id),
            (program.admin, env.ledger().timestamp()),
        );
        Ok(())
    }

    /// Read the pending admin for a specific program, if any.
    ///
    /// Time complexity: O(1)  Space complexity: O(1)
    pub fn get_pending_program_admin(env: Env, program_id: u64) -> Option<Address> {
        env.storage()
            .persistent()
            .get(&DataKey::ProgramPendingAdmin(program_id))
    }
}

#[cfg(test)]
mod test;
#[cfg(test)]
mod test_ownership_transfer;
#[cfg(test)]
mod test_search;
#[cfg(test)]
mod test_full_lifecycle;
#[cfg(test)]
mod test_max_counts;
#[cfg(test)]
mod test_error_discrimination;
