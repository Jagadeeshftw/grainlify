#[cfg(feature = "contract")]
#[contractimpl]
impl ProgramEscrowContract {
    pub fn lock_program_funds_v2(env: Env, program_id: String, amount: i128) -> ProgramData {
        Self::require_not_read_only(&env);
        // Validation precedence (deterministic ordering):
        // 1. Amount > 0
        // 2. Program exists
        // 3. Program must be in Active status (not Draft)
        // 4. Contract balance check (detects FoT issues if tokens were sent beforehand)

        if amount <= 0 {
            panic!("Amount must be greater than zero");
        }

        let program_key = DataKey::Program(program_id.clone());
        let mut program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"));

        if program_data.status == ProgramStatus::Draft {
            panic!("Program is in Draft status. Publish the program first.");
        }

        let token_client = token::Client::new(&env, &program_data.token_address);
        let contract_address = env.current_contract_address();

        // Ensure contract actually holds enough tokens to cover this lock.
        // If tokens were sent via direct transfer and a fee was taken, this check will catch it.
        if token_client.balance(&contract_address) < amount {
            panic!("Insufficient contract balance to cover lock (possible fee-on-transfer issue)");
        }

        let fee_config = Self::get_fee_config_internal(&env);
        let (fee_amount, net_amount) = if fee_config.fee_enabled && fee_config.lock_fee_rate > 0 {
            token_math::split_amount(amount, fee_config.lock_fee_rate)
        } else {
            (0i128, amount)
        };

        if fee_amount > 0 {
            let (reserve_share, recipient_share) =
                Self::split_fee_for_reserve(fee_amount, fee_config.insurance_reserve_bps);
            if recipient_share > 0 {
                token_client.transfer(
                    &contract_address,
                    &fee_config.fee_recipient,
                    &recipient_share,
                );
            }
            Self::accrue_insurance_reserve(&env, reserve_share);
        }

        program_data.total_funds = program_data
            .total_funds
            .checked_add(amount)
            .expect("Total funds overflow");
        program_data.remaining_balance = program_data
            .remaining_balance
            .checked_add(net_amount)
            .expect("Remaining balance overflow");

        env.storage().instance().set(&program_key, &program_data);

        // Sync with global if applicable
        if let Some(global_data) = env
            .storage()
            .instance()
            .get::<Symbol, ProgramData>(&PROGRAM_DATA)
        {
            if global_data.program_id == program_id {
                env.storage().instance().set(&PROGRAM_DATA, &program_data);
            }
        }

        env.events().publish(
            (FUNDS_LOCKED,),
            FundsLockedEvent {
                version: EVENT_VERSION_V2,
                program_id,
                amount,
                remaining_balance: program_data.remaining_balance,
            },
        );

        program_data
    }

    pub fn single_payout_v2(
        env: Env,
        program_id: String,
        recipient: Address,
        amount: i128,
    ) -> ProgramData {
        Self::require_not_read_only(&env);
        // For now, single_payout still uses global data in several places internally
        // so we just call the existing one but we should ideally update it too.
        // Actually, let's just implement it here to be safe.
        let program_key = DataKey::Program(program_id.clone());
        let mut program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"));

        if program_data.status == ProgramStatus::Draft {
            panic!("Program is in Draft status. Publish the program first.");
        }

        if amount <= 0 || amount > program_data.remaining_balance {
            panic!("Invalid payout amount");
        }

        let token_client = token::Client::new(&env, &program_data.token_address);
        token_client.transfer(&env.current_contract_address(), &recipient, &amount);

        program_data.remaining_balance -= amount;
        env.storage().instance().set(&program_key, &program_data);

        if let Some(global_data) = env
            .storage()
            .instance()
            .get::<Symbol, ProgramData>(&PROGRAM_DATA)
        {
            if global_data.program_id == program_id {
                env.storage().instance().set(&PROGRAM_DATA, &program_data);
            }
        }

        env.events()
            .publish((symbol_short!("Payout"),), (program_id, recipient, amount));

        program_data
    }

    /// Distributes prizes to multiple recipients and stores a Merkle root receipt
    /// for deterministic batch verification.
    pub fn batch_payout_with_receipt(
        env: Env,
        recipients: soroban_sdk::Vec<Address>,
        amounts: soroban_sdk::Vec<i128>,
        merkle_root: soroban_sdk::BytesN<32>,
    ) -> BatchReceipt {
        let program_data =
            Self::batch_payout(env.clone(), recipients.clone(), amounts.clone());

        let batch_id_key = BatchReceiptKey::NextId;
        let batch_id: u64 = env.storage().persistent().get(&batch_id_key).unwrap_or(0);

        // Calculate total
        let mut total_amount: i128 = 0;
        for amount in amounts.iter() {
            total_amount += amount;
        }

        let receipt = BatchReceipt {
            version: BATCH_RECEIPT_VERSION,
            batch_id,
            merkle_root,
            total_amount,
            recipient_count: recipients.len(),
            timestamp: env.ledger().timestamp(),
        };

        env.storage()
            .persistent()
            .set(&BatchReceiptKey::Receipt(batch_id), &receipt);
        env.storage()
            .persistent()
            .set(&batch_id_key, &(batch_id + 1));

        receipt
    }

    /// Fetches a stored batch receipt by ID (legacy key format)
    pub fn get_batch_receipt_by_batch_id(
        env: Env,
        batch_id: u64,
    ) -> Result<BatchReceipt, BatchError> {
        env.storage()
            .persistent()
            .get(&BatchReceiptKey::Receipt(batch_id))
            .ok_or(BatchError::BatchReceiptNotFound)
    }

    pub fn batch_payout_v2(
        env: Env,
        _program_id: String,
        recipients: soroban_sdk::Vec<Address>,
        amounts: soroban_sdk::Vec<i128>,
    ) -> ProgramData {
        Self::batch_payout(env, recipients, amounts)
    }

    /// Retrieve a stored batch payout receipt by its receipt ID.
    ///
    /// Returns `None` if no receipt exists for the given ID.
    /// Receipts are stored in persistent storage and survive contract upgrades.
    pub fn get_batch_receipt(env: Env, receipt_id: u64) -> Option<BatchReceipt> {
        env.storage()
            .persistent()
            .get(&DataKey::BatchReceipt(receipt_id))
    }

    // --- Payout Splits (Ratio-based) ---

    pub fn set_split_config(
        env: Env,
        program_id: String,
        beneficiaries: soroban_sdk::Vec<BeneficiarySplit>,
    ) -> SplitConfig {
        if let Some(admin) = env.storage().instance().get::<_, Address>(&DataKey::Admin) {
            admin.require_auth();
        } else {
            let program: ProgramData = env
                .storage()
                .instance()
                .get(&PROGRAM_DATA)
                .unwrap_or_else(|| panic!("Program not initialized"));
            program.authorized_payout_key.require_auth();
        }
        payout_splits::set_split_config(&env, &program_id, beneficiaries)
    }

    pub fn get_split_config(env: Env, program_id: String) -> Option<SplitConfig> {
        payout_splits::get_split_config(&env, &program_id)
    }

    pub fn disable_split_config(env: Env, program_id: String) {
        if let Some(admin) = env.storage().instance().get::<_, Address>(&DataKey::Admin) {
            admin.require_auth();
        } else {
            let program: ProgramData = env
                .storage()
                .instance()
                .get(&PROGRAM_DATA)
                .unwrap_or_else(|| panic!("Program not initialized"));
            program.authorized_payout_key.require_auth();
        }
        payout_splits::disable_split_config(&env, &program_id);
    }

    pub fn execute_split_payout(
        env: Env,
        program_id: String,
        total_amount: i128,
    ) -> payout_splits::SplitPayoutResult {
        let program: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        if program.status == ProgramStatus::Draft {
            panic!("Program is in Draft status. Publish the program first.");
        }

        program.authorized_payout_key.require_auth();
        payout_splits::execute_split_payout(&env, &program_id, total_amount)
    }

    pub fn preview_split(
        env: Env,
        program_id: String,
        total_amount: i128,
    ) -> soroban_sdk::Vec<BeneficiarySplit> {
        payout_splits::preview_split(&env, &program_id, total_amount)
    }

    /// Query payout history by recipient with pagination
    pub fn query_payouts_by_recipient(
        env: Env,
        recipient: Address,
        offset: u32,
        limit: u32,
    ) -> Result<soroban_sdk::Vec<PayoutRecord>, BatchError> {
        Self::validate_pagination(&env, limit)?;
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));
        Self::paginate_filtered(&env, program_data.payout_history, offset, limit, |record| {
            record.recipient == recipient
        })
    }

    /// O(1) recipient history lookup using the lazy-initialized inverted index.
    ///
    /// Returns all [`PayoutRecord`]s for `recipient` in `program_id`, in
    /// chronological insertion order.  Returns an empty `Vec` when the
    /// recipient has never received a payout (the key is simply absent).
    ///
    /// # Storage
    /// Reads from `DataKey::RecipientPayoutIndex(program_id, recipient)` in
    /// persistent storage (written by `single_payout_internal` /
    /// `batch_payout_internal` on every payout to this recipient).
    ///
    /// # Security
    /// - Read-only; never mutates state.
    /// - No authorization required (payout records are public on-chain data).
    /// - `program_id` is caller-supplied but cannot forge records: the index
    ///   is written exclusively by the payout paths under admin auth.
    pub fn query_recipient_history(
        env: Env,
        program_id: String,
        recipient: Address,
    ) -> soroban_sdk::Vec<PayoutRecord> {
        let key = DataKey::RecipientPayoutIndex(program_id, recipient);
        env.storage()
            .persistent()
            .get::<DataKey, soroban_sdk::Vec<PayoutRecord>>(&key)
            .unwrap_or_else(|| soroban_sdk::Vec::new(&env))
    }

    // ─── private helper ───────────────────────────────────────────────────

    /// Append `record` to the persistent recipient index for `(program_id, recipient)`.
    ///
    /// Lazy initialization: the key is created on the first payout; no
    /// storage entry exists until then, keeping cold-storage costs at zero
    /// for programs that have not yet paid out to a given address.
    fn append_recipient_index(
        env: &Env,
        program_id: &String,
        recipient: &Address,
        record: &PayoutRecord,
    ) {
        let key = DataKey::RecipientPayoutIndex(program_id.clone(), recipient.clone());
        let mut index: soroban_sdk::Vec<PayoutRecord> = env
            .storage()
            .persistent()
            .get::<DataKey, soroban_sdk::Vec<PayoutRecord>>(&key)
            .unwrap_or_else(|| soroban_sdk::Vec::new(env));
        index.push_back(record.clone());
        env.storage().persistent().set(&key, &index);
        Self::track_and_extend_program_ttl(env, program_id, Some(&key));
    }

    /// Query idempotency key status
    ///


    /// Query payout history by amount range
    pub fn query_payouts_by_amount(
        env: Env,
        min_amount: i128,
        max_amount: i128,
        offset: u32,
        limit: u32,
    ) -> Result<soroban_sdk::Vec<PayoutRecord>, BatchError> {
        Self::validate_pagination(&env, limit)?;
        if min_amount > max_amount {
            return Err(BatchError::InvalidAmount);
        }
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));
        Self::paginate_filtered(&env, program_data.payout_history, offset, limit, |record| {
            record.amount >= min_amount && record.amount <= max_amount
        })
    }

    /// Query payout history by timestamp range
    pub fn query_payouts_by_timestamp(
        env: Env,
        min_timestamp: u64,
        max_timestamp: u64,
        offset: u32,
        limit: u32,
    ) -> Result<soroban_sdk::Vec<PayoutRecord>, BatchError> {
        Self::validate_pagination(&env, limit)?;
        if min_timestamp > max_timestamp {
            return Err(BatchError::InvalidPaginationOffset);
        }
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));
        Self::paginate_filtered(&env, program_data.payout_history, offset, limit, |record| {
            record.timestamp >= min_timestamp && record.timestamp <= max_timestamp
        })
    }

    /// Query release schedules by recipient with pagination
    pub fn query_schedules_by_recipient(
        env: Env,
        recipient: Address,
        offset: u32,
        limit: u32,
    ) -> Result<soroban_sdk::Vec<ProgramReleaseSchedule>, BatchError> {
        Self::validate_pagination(&env, limit)?;
        let schedules: soroban_sdk::Vec<ProgramReleaseSchedule> = env
            .storage()
            .instance()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));

        Self::paginate_filtered(&env, schedules, offset, limit, |schedule| {
            schedule.recipient == recipient
        })
    }

    /// Query release history with filtering and pagination
    pub fn query_releases_by_recipient(
        env: Env,
        recipient: Address,
        offset: u32,
        limit: u32,
    ) -> Result<soroban_sdk::Vec<ProgramReleaseHistory>, BatchError> {
        Self::validate_pagination(&env, limit)?;
        let history: soroban_sdk::Vec<ProgramReleaseHistory> = env
            .storage()
            .instance()
            .get(&RELEASE_HISTORY)
            .unwrap_or_else(|| Vec::new(&env));
        Self::paginate_filtered(&env, history, offset, limit, |record| {
            record.recipient == recipient
        })
    }

    /// Get aggregate statistics for the program
    pub fn get_program_aggregate_stats(env: Env) -> ProgramAggregateStats {
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));
        let schedules: soroban_sdk::Vec<ProgramReleaseSchedule> = env
            .storage()
            .instance()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));

        let mut scheduled_count = 0u32;
        let mut released_count = 0u32;

        for i in 0..schedules.len() {
            let schedule = schedules.get(i).unwrap();
            if schedule.released {
                released_count += 1;
            } else {
                scheduled_count += 1;
            }
        }

        ProgramAggregateStats {
            total_funds: program_data.total_funds,
            remaining_balance: program_data.remaining_balance,
            total_paid_out: program_data.total_funds - program_data.remaining_balance,
            authorized_payout_key: program_data.authorized_payout_key.clone(),
            payout_history: program_data.payout_history.clone(),
            token_address: program_data.token_address.clone(),
            payout_count: program_data.payout_history.len(),
            scheduled_count,
            released_count,
        }
    }

    /// Get payouts by recipient
    pub fn get_payouts_by_recipient(
        env: Env,
        recipient: Address,
        offset: u32,
        limit: u32,
    ) -> Result<soroban_sdk::Vec<PayoutRecord>, BatchError> {
        Self::validate_pagination(&env, limit)?;
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));
        Self::paginate_filtered(&env, program_data.payout_history, offset, limit, |record| {
            record.recipient == recipient
        })
    }

    /// Get pending schedules (not yet released)
    pub fn get_pending_schedules(env: Env) -> soroban_sdk::Vec<ProgramReleaseSchedule> {
        let schedules: soroban_sdk::Vec<ProgramReleaseSchedule> = env
            .storage()
            .instance()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));
        let mut results = Vec::new(&env);

        for i in 0..schedules.len() {
            let schedule = schedules.get(i).unwrap();
            if !schedule.released {
                results.push_back(schedule);
            }
        }
        results
    }

    /// Get due schedules (ready to be released)
    pub fn get_due_schedules(env: Env) -> soroban_sdk::Vec<ProgramReleaseSchedule> {
        let schedules: soroban_sdk::Vec<ProgramReleaseSchedule> = env
            .storage()
            .instance()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));
        let now = env.ledger().timestamp();
        let mut results = Vec::new(&env);

        for i in 0..schedules.len() {
            let schedule = schedules.get(i).unwrap();
            if !schedule.released && schedule.release_timestamp <= now {
                results.push_back(schedule);
            }
        }
        results
    }

    /// Get total amount in pending schedules
    pub fn get_total_scheduled_amount(env: Env) -> i128 {
        let schedules: soroban_sdk::Vec<ProgramReleaseSchedule> = env
            .storage()
            .instance()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));
        let mut total = 0i128;

        for i in 0..schedules.len() {
            let schedule = schedules.get(i).unwrap();
            if !schedule.released {
                total += schedule.amount;
            }
        }
        total
    }

    pub fn get_program_count(env: Env) -> u32 {
        if env.storage().instance().has(&PROGRAM_DATA) {
            1
        } else {
            0
        }
    }

    pub fn list_programs(env: Env) -> soroban_sdk::Vec<ProgramData> {
        let mut results = Vec::new(&env);
        if env.storage().instance().has(&PROGRAM_DATA) {
            let data = Self::get_program_info(env.clone());
            if !data.archived {
                results.push_back(data);
            }
        }
        results
    }

    /// Query program delegates for a set of registered programs (paginated).
    ///
    /// This returns a vector of `ProgramDelegateInfo` records for the requested
    /// slice of entries from the internal `PROGRAM_REGISTRY`.
    pub fn query_program_delegates(
        env: Env,
        offset: Option<u32>,
        limit: Option<u32>,
    ) -> soroban_sdk::Vec<ProgramDelegateInfo> {
        let registry: soroban_sdk::Vec<String> = env
            .storage()
            .instance()
            .get(&PROGRAM_REGISTRY)
            .unwrap_or(Vec::new(&env));

        let total = registry.len();
        let offset = offset.unwrap_or(0);
        let limit = limit.unwrap_or(total);

        // Validate pagination params conservatively: return empty vec on bad params
        if offset > total || limit == 0 {
            return Vec::new(&env);
        }

        let end = if offset + limit > total { total } else { offset + limit };
        let mut result = Vec::new(&env);
        for i in offset..end {
            let pid = registry.get(i).unwrap();
            let program_data = Self::get_program_data_by_id(&env, &pid);
            result.push_back(ProgramDelegateInfo {
                program_id: pid.clone(),
                delegate: program_data.delegate.clone(),
                permissions: program_data.delegate_permissions,
            });
        }
        result
    }

    pub fn query_all_delegates(env: Env, program_id: String) -> soroban_sdk::Vec<ProgramDelegateInfo> {
        let mut results = soroban_sdk::Vec::new(&env);
        let delegates = Self::query_program_delegates(env.clone(), None, None);
        for d in delegates.iter() {
            // Only surface programs that currently have an active delegate.
            if d.program_id == program_id && d.delegate.is_some() {
                results.push_back(d);
            }
        }
        results
    }

    pub fn get_program_release_schedule(env: Env, schedule_id: u64) -> ProgramReleaseSchedule {
        let schedules = Self::get_release_schedules(env);
        for s in schedules.iter() {
            if s.schedule_id == schedule_id {
                return s;
            }
        }
        panic!("Schedule not found");
    }
}
