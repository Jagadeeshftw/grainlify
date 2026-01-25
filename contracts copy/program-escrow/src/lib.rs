#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, vec, Address, Env, String, Symbol, Vec,
    token,
};

// Event types
const PROGRAM_INITIALIZED: Symbol = symbol_short!("ProgramInit");
const FUNDS_LOCKED: Symbol = symbol_short!("FundsLocked");
const BATCH_PAYOUT: Symbol = symbol_short!("BatchPayout");
const PAYOUT: Symbol = symbol_short!("Payout");

// Storage keys
const PROGRAM_DATA: Symbol = symbol_short!("ProgramData");

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayoutRecord {
    pub recipient: Address,
    pub amount: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramData {
    pub program_id: String,
    pub total_funds: i128,
    pub remaining_balance: i128,
    pub authorized_payout_key: Address,
    pub payout_history: Vec<PayoutRecord>,
    pub token_address: Address, // Token contract address for transfers
}

#[contract]
pub struct ProgramEscrowContract;

#[contractimpl]
impl ProgramEscrowContract {
    /// Initialize a new program escrow
    /// 
    /// # Arguments
    /// * `program_id` - Unique identifier for the program/hackathon
    /// * `authorized_payout_key` - Address authorized to trigger payouts (backend)
    /// * `token_address` - Address of the token contract to use for transfers
    /// 
    /// # Returns
    /// The initialized ProgramData
    pub fn init_program(
        env: Env,
        program_id: String,
        authorized_payout_key: Address,
        token_address: Address,
    ) -> ProgramData {
        // Check if program already exists
        if env.storage().instance().has(&PROGRAM_DATA) {
            panic!("Program already initialized");
        }

        let contract_address = env.current_contract_address();
        let program_data = ProgramData {
            program_id: program_id.clone(),
            total_funds: 0,
            remaining_balance: 0,
            authorized_payout_key: authorized_payout_key.clone(),
            payout_history: vec![&env],
            token_address: token_address.clone(),
        };

        // Store program data
        env.storage().instance().set(&PROGRAM_DATA, &program_data);

        // Emit ProgramInitialized event
        env.events().publish(
            (PROGRAM_INITIALIZED,),
            (program_id, authorized_payout_key, token_address, 0i128),
        );

        program_data
    }

    /// Lock initial funds into the program escrow
    /// 
    /// # Arguments
    /// * `amount` - Amount of funds to lock (in native token units)
    /// 
    /// # Returns
    /// Updated ProgramData with locked funds
    pub fn lock_program_funds(env: Env, amount: i128) -> ProgramData {
        if amount <= 0 {
            panic!("Amount must be greater than zero");
        }

        let mut program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        // Update balances
        program_data.total_funds += amount;
        program_data.remaining_balance += amount;

        // Store updated data
        env.storage().instance().set(&PROGRAM_DATA, &program_data);

        // Emit FundsLocked event
        env.events().publish(
            (FUNDS_LOCKED,),
            (
                program_data.program_id.clone(),
                amount,
                program_data.remaining_balance,
            ),
        );

        program_data
    }

    /// Execute batch payouts to multiple recipients
    /// 
    /// # Arguments
    /// * `recipients` - Vector of recipient addresses
    /// * `amounts` - Vector of amounts (must match recipients length)
    /// 
    /// # Returns
    /// Updated ProgramData after payouts
    pub fn batch_payout(
        env: Env,
        recipients: Vec<Address>,
        amounts: Vec<i128>,
    ) -> ProgramData {
        // Verify authorization
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        let caller = env.invoker();
        if caller != program_data.authorized_payout_key {
            panic!("Unauthorized: only authorized payout key can trigger payouts");
        }

        // Validate input lengths match
        if recipients.len() != amounts.len() {
            panic!("Recipients and amounts vectors must have the same length");
        }

        if recipients.len() == 0 {
            panic!("Cannot process empty batch");
        }

        // Calculate total payout amount
        let mut total_payout: i128 = 0;
        for amount in amounts.iter() {
            if *amount <= 0 {
                panic!("All amounts must be greater than zero");
            }
            total_payout = total_payout
                .checked_add(*amount)
                .unwrap_or_else(|| panic!("Payout amount overflow"));
        }

        // Validate sufficient balance
        if total_payout > program_data.remaining_balance {
            panic!("Insufficient balance: requested {}, available {}", 
                total_payout, program_data.remaining_balance);
        }

        // Execute transfers
        let mut updated_history = program_data.payout_history.clone();
        let timestamp = env.ledger().timestamp();
        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &program_data.token_address);

        for (i, recipient) in recipients.iter().enumerate() {
            let amount = amounts.get(i).unwrap();
            
            // Transfer funds from contract to recipient
            token_client.transfer(&contract_address, recipient, amount);

            // Record payout
            let payout_record = PayoutRecord {
                recipient: recipient.clone(),
                amount: *amount,
                timestamp,
            };
            updated_history.push_back(payout_record);
        }

        // Update program data
        let mut updated_data = program_data.clone();
        updated_data.remaining_balance -= total_payout;
        updated_data.payout_history = updated_history;

        // Store updated data
        env.storage().instance().set(&PROGRAM_DATA, &updated_data);

        // Emit BatchPayout event
        env.events().publish(
            (BATCH_PAYOUT,),
            (
                updated_data.program_id.clone(),
                recipients.len() as u32,
                total_payout,
                updated_data.remaining_balance,
            ),
        );

        updated_data
    }

    /// Execute a single payout to one recipient
    /// 
    /// # Arguments
    /// * `recipient` - Address of the recipient
    /// * `amount` - Amount to transfer
    /// 
    /// # Returns
    /// Updated ProgramData after payout
    pub fn single_payout(env: Env, recipient: Address, amount: i128) -> ProgramData {
        // Verify authorization
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        let caller = env.invoker();
        if caller != program_data.authorized_payout_key {
            panic!("Unauthorized: only authorized payout key can trigger payouts");
        }

        // Validate amount
        if amount <= 0 {
            panic!("Amount must be greater than zero");
        }

        // Validate sufficient balance
        if amount > program_data.remaining_balance {
            panic!("Insufficient balance: requested {}, available {}", 
                amount, program_data.remaining_balance);
        }

        // Transfer funds from contract to recipient
        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &program_data.token_address);
        token_client.transfer(&contract_address, &recipient, &amount);

        // Record payout
        let timestamp = env.ledger().timestamp();
        let payout_record = PayoutRecord {
            recipient: recipient.clone(),
            amount,
            timestamp,
        };

        let mut updated_history = program_data.payout_history.clone();
        updated_history.push_back(payout_record);

        // Update program data
        let mut updated_data = program_data.clone();
        updated_data.remaining_balance -= amount;
        updated_data.payout_history = updated_history;

        // Store updated data
        env.storage().instance().set(&PROGRAM_DATA, &updated_data);

        // Emit Payout event
        env.events().publish(
            (PAYOUT,),
            (
                updated_data.program_id.clone(),
                recipient,
                amount,
                updated_data.remaining_balance,
            ),
        );

        updated_data
    }

    /// Get program information
    /// 
    /// # Returns
    /// ProgramData containing all program information
    pub fn get_program_info(env: Env) -> ProgramData {
        env.storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"))
    }

    /// Get remaining balance
    /// 
    /// # Returns
    /// Current remaining balance
    pub fn get_remaining_balance(env: Env) -> i128 {
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        program_data.remaining_balance
    }
}

#[cfg(test)]
mod test;
