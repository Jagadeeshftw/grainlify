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
    contract, contracterror, contractimpl, contracttype, symbol_short, token, Address, Env,
    String, Vec,
};

const MAX_BATCH_SIZE: u32 = 20;
const MAX_PAGE_SIZE: u32 = 20;
const PROGRAM_REGISTERED: soroban_sdk::Symbol = symbol_short!("prg_reg");

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    ProgramExists = 3,
    ProgramNotFound = 4,
    Unauthorized = 5,
    InvalidBatchSize = 6,
    DuplicateProgramId = 7,
    InvalidAmount = 8,
    InvalidName = 9,
    ContractDeprecated = 10,
    JurisdictionKycRequired = 11,
    JurisdictionFundingLimitExceeded = 12,
    JurisdictionPaused = 13,
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
    pub name: String,
    pub total_funding: i128,
    pub status: ProgramStatus,
    pub jurisdiction: OptionalJurisdiction,
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
                timestamp: env.ledger().timestamp(),
            },
        );
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
        env.storage().persistent().set(&DataKey::ProgramIndex, &index);
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
        Self::register_program_juris(
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
        Self::ensure_initialized(&env)?;
        Self::ensure_not_deprecated(&env)?;
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

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token_addr);
        admin.require_auth();
        token_client.transfer(&admin, &env.current_contract_address(), &total_funding);

        let program = Program {
            admin: admin.clone(),
            name,
            total_funding,
            status: ProgramStatus::Active,
            jurisdiction: jurisdiction.clone(),
        };
        Self::store_program(&env, program_id, &program);
        Self::append_program_id(&env, program_id);
        Self::emit_program_registered(&env, program_id, admin, total_funding, &jurisdiction);
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
                name: item.name.clone(),
                total_funding: item.total_funding,
                status: ProgramStatus::Active,
                jurisdiction: OptionalJurisdiction::None,
            };
            Self::store_program(&env, item.program_id, &program);
            Self::append_program_id(&env, item.program_id);
            Self::emit_program_registered(
                &env,
                item.program_id,
                item.admin.clone(),
                item.total_funding,
                &OptionalJurisdiction::None,
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
            Self::enforce_jurisdiction_rules(
                &jurisdiction,
                item.total_funding,
                item.kyc_attested,
            )?;

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
                name: item.name.clone(),
                total_funding: item.total_funding,
                status: ProgramStatus::Active,
                jurisdiction: jurisdiction.clone(),
            };
            Self::store_program(&env, item.program_id, &program);
            Self::append_program_id(&env, item.program_id);
            Self::emit_program_registered(
                &env,
                item.program_id,
                item.admin.clone(),
                item.total_funding,
                &jurisdiction,
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
        env.storage().instance().set(&DataKey::DeprecationState, &state);
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
        if !env.storage().persistent().has(&DataKey::Program(program_id)) {
            return Err(Error::ProgramNotFound);
        }
        Ok(env
            .storage()
            .persistent()
            .get(&DataKey::ProgramJurisdiction(program_id)))
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

#[cfg(test)]
mod test;
#[cfg(test)]
mod test_search;
