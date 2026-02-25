#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, token, Address, Env, String, Vec,
};

const MAX_BATCH_SIZE: u32 = 20;

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
pub struct Program {
    pub admin: Address,
    pub name: String,
    pub total_funding: i128,
    pub status: ProgramStatus,
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
pub enum DataKey {
    Admin,
    Token,
    Program(u64),
}

#[contract]
pub struct ProgramEscrowContract;

#[contractimpl]
impl ProgramEscrowContract {
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
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }
        let contract_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        contract_admin.require_auth();

        if env
            .storage()
            .persistent()
            .has(&DataKey::Program(program_id))
        {
            return Err(Error::ProgramExists);
        }
        if total_funding <= 0 {
            return Err(Error::InvalidAmount);
        }
        if name.len() == 0 {
            return Err(Error::InvalidName);
        }

        // Transfer funding from the program admin to the contract
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token_addr);
        admin.require_auth();
        token_client.transfer(&admin, &env.current_contract_address(), &total_funding);

        let program = Program {
            admin,
            name,
            total_funding,
            status: ProgramStatus::Active,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Program(program_id), &program);
        Ok(())
    }

    /// Batch register multiple programs in a single transaction.
    ///
    /// This operation is atomic — if any item fails validation, the entire
    /// batch is rejected and no programs are registered.
    ///
    /// # Errors
    /// * `InvalidBatchSize` — batch is empty or exceeds `MAX_BATCH_SIZE`
    /// * `ProgramExists` — a program_id already exists in storage
    /// * `DuplicateProgramId` — duplicate program_ids within the batch
    /// * `InvalidAmount` — zero or negative funding amount
    /// * `InvalidName` — empty program name
    /// * `NotInitialized` — contract has not been initialized
    pub fn batch_register_programs(
        env: Env,
        items: Vec<ProgramRegistrationItem>,
    ) -> Result<u32, Error> {
        let batch_size = items.len() as u32;
        if batch_size == 0 || batch_size > MAX_BATCH_SIZE {
            return Err(Error::InvalidBatchSize);
        }

        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }
        let contract_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        contract_admin.require_auth();

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token_addr);
        let contract_address = env.current_contract_address();

        // --- Validation pass (all-or-nothing) ---
        for item in items.iter() {
            if env
                .storage()
                .persistent()
                .has(&DataKey::Program(item.program_id))
            {
                return Err(Error::ProgramExists);
            }
            if item.total_funding <= 0 {
                return Err(Error::InvalidAmount);
            }
            if item.name.len() == 0 {
                return Err(Error::InvalidName);
            }

            // Detect duplicate program_ids within the batch
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

        // Collect unique admins and require auth once per admin
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

        // --- Processing pass (atomic) ---
        let mut registered_count = 0u32;
        for item in items.iter() {
            token_client.transfer(&item.admin, &contract_address, &item.total_funding);

            let program = Program {
                admin: item.admin.clone(),
                name: item.name.clone(),
                total_funding: item.total_funding,
                status: ProgramStatus::Active,
            };
            env.storage()
                .persistent()
                .set(&DataKey::Program(item.program_id), &program);

            registered_count += 1;
        }

        Ok(registered_count)
    }

    /// Read a program's state.
    pub fn get_program(env: Env, program_id: u64) -> Result<Program, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Program(program_id))
            .ok_or(Error::ProgramNotFound)
    }
}

mod test;
