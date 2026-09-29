#[cfg(feature = "contract")]
#[contractimpl]
impl ProgramEscrowContract {
    /// Atomically lock funds for multiple programs.
    pub fn batch_lock(env: Env, items: Vec<LockItem>) -> Result<u32, BatchError> {
        Self::require_not_read_only(&env);
        reentrancy_guard::check_not_entered(&env);
        reentrancy_guard::set_entered(&env);

        if Self::check_paused(&env, None, symbol_short!("lock")) {
            reentrancy_guard::clear_entered(&env);
            return Err(BatchError::FundsPaused);
        }

        let batch_size = items.len() as u32;
        if batch_size == 0 || batch_size > MAX_BATCH_SIZE {
            reentrancy_guard::clear_entered(&env);
            return Err(BatchError::InvalidBatchSizeProgram);
        }

        // Deterministic ordering to prevent potential deadlocks and ensure predictable behavior
        let ordered_items = Self::order_batch_lock_items(&env, &items);

        // Check for duplicate program IDs in the batch
        let mut seen = Vec::new(&env);
        for item in ordered_items.iter() {
            let mut exists = false;
            for s in seen.iter() {
                if s == item.program_id {
                    exists = true;
                    break;
                }
            }
            if exists {
                reentrancy_guard::clear_entered(&env);
                return Err(BatchError::DuplicateProgramId);
            }
            seen.push_back(item.program_id.clone());
        }

        let mut total_locked: i128 = 0;
        let fee_config = Self::get_fee_config_internal(&env);
        let contract_address = env.current_contract_address();

        for item in ordered_items.iter() {
            if Self::check_paused(&env, Some(&item.program_id), symbol_short!("lock")) {
                reentrancy_guard::clear_entered(&env);
                return Err(BatchError::FundsPaused);
            }

            if item.amount <= 0 {
                reentrancy_guard::clear_entered(&env);
                return Err(BatchError::InvalidAmount);
            }

            let program_key = DataKey::Program(item.program_id.clone());
            let mut program_data: ProgramData =
                env.storage().instance().get(&program_key).ok_or_else(|| {
                    reentrancy_guard::clear_entered(&env);
                    BatchError::ProgramNotFound
                })?;

            if program_data.status == ProgramStatus::Draft {
                reentrancy_guard::clear_entered(&env);
                panic!("Program in Draft status");
            }

            let token_client = token::Client::new(&env, &program_data.token_address);
            if token_client.balance(&contract_address) < item.amount {
                reentrancy_guard::clear_entered(&env);
                panic!("Insufficient contract balance");
            }

            let (fee_amount, net_amount) = if fee_config.fee_enabled && fee_config.lock_fee_rate > 0
            {
                token_math::split_amount(item.amount, fee_config.lock_fee_rate)
            } else {
                (0i128, item.amount)
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
                Self::emit_fee_collected(
                    &env,
                    symbol_short!("lock"),
                    fee_amount,
                    fee_config.lock_fee_rate,
                    fee_config.lock_fixed_fee,
                    fee_config.fee_recipient.clone(),
                );
            }

            program_data.total_funds = program_data
                .total_funds
                .checked_add(item.amount)
                .expect("Total funds overflow");
            program_data.remaining_balance = program_data
                .remaining_balance
                .checked_add(net_amount)
                .expect("Remaining balance overflow");

            env.storage().instance().set(&program_key, &program_data);
            total_locked = total_locked
                .checked_add(item.amount)
                .expect("Total locked overflow");
        }

        env.events().publish(
            (BATCH_FUNDS_LOCKED,),
            BatchFundsLocked {
                count: batch_size,
                total_amount: total_locked,
                timestamp: env.ledger().timestamp(),
            },
        );

        reentrancy_guard::clear_entered(&env);
        Ok(batch_size)
    }

    /// Atomically release multiple scheduled payouts.
    ///
    /// # Arguments
    /// * `items` - Vector of ReleaseItem containing program_id and schedule_id.
    ///
    /// # Returns
    /// Number of successfully released payouts.
    /// Atomically release multiple scheduled payouts.
    pub fn batch_release(env: Env, items: Vec<ReleaseItem>) -> Result<u32, BatchError> {
        Self::require_not_read_only(&env);
        reentrancy_guard::check_not_entered(&env);
        reentrancy_guard::set_entered(&env);

        if Self::check_paused(&env, None, symbol_short!("release")) {
            reentrancy_guard::clear_entered(&env);
            return Err(BatchError::FundsPaused);
        }

        let batch_size = items.len() as u32;
        if batch_size == 0 || batch_size > MAX_BATCH_SIZE {
            reentrancy_guard::clear_entered(&env);
            return Err(BatchError::InvalidBatchSizeProgram);
        }

        // Deterministic ordering to ensure predictable state transitions
        let ordered_items = Self::order_batch_release_items(&env, &items);

        let mut total_released: i128 = 0;
        let now = env.ledger().timestamp();
        let contract_address = env.current_contract_address();

        for item in ordered_items.iter() {
            if Self::check_paused(&env, Some(&item.program_id), symbol_short!("release")) {
                reentrancy_guard::clear_entered(&env);
                return Err(BatchError::FundsPaused);
            }

            let program_key = DataKey::Program(item.program_id.clone());
            let mut program_data: ProgramData =
                env.storage().instance().get(&program_key).ok_or_else(|| {
                    reentrancy_guard::clear_entered(&env);
                    BatchError::ProgramNotFound
                })?;

            if program_data.status == ProgramStatus::Draft {
                reentrancy_guard::clear_entered(&env);
                panic!("Program in Draft status");
            }

            let mut schedules: soroban_sdk::Vec<ProgramReleaseSchedule> = env
                .storage()
                .instance()
                .get(&SCHEDULES)
                .unwrap_or_else(|| Vec::new(&env));

            let mut found = false;
            for i in 0..schedules.len() {
                let mut schedule = schedules.get(i).unwrap();
                if schedule.schedule_id == item.schedule_id {
                    if schedule.released {
                        reentrancy_guard::clear_entered(&env);
                        return Err(BatchError::AlreadyReleased);
                    }
                    if schedule.release_timestamp > now {
                        reentrancy_guard::clear_entered(&env);
                        panic!("Schedule not yet due");
                    }
                    if schedule.amount > program_data.remaining_balance {
                        reentrancy_guard::clear_entered(&env);
                        panic!("Insufficient program balance for release");
                    }

                    // Circuit breaker check
                    if let Err(_) = error_recovery::check_and_allow_with_thresholds(&env) {
                        reentrancy_guard::clear_entered(&env);
                        return Err(BatchError::FundsPaused);
                    }

                    let token_client = token::Client::new(&env, &program_data.token_address);
                    token_client.transfer(&contract_address, &schedule.recipient, &schedule.amount);

                    schedule.released = true;
                    schedule.released_at = Some(now);
                    schedule.released_by = Some(env.current_contract_address()); // System released
                    schedules.set(i, schedule.clone());

                    program_data.remaining_balance = program_data
                        .remaining_balance
                        .checked_sub(schedule.amount)
                        .expect("Balance underflow");

                    total_released = total_released
                        .checked_add(schedule.amount)
                        .expect("Total released overflow");
                    found = true;
                    break;
                }
            }

            if !found {
                reentrancy_guard::clear_entered(&env);
                return Err(BatchError::ScheduleNotFound);
            }

            env.storage().instance().set(&SCHEDULES, &schedules);
            env.storage().instance().set(&program_key, &program_data);
        }

        env.events().publish(
            (BATCH_FUNDS_RELEASED,),
            BatchFundsReleased {
                count: batch_size,
                total_amount: total_released,
                timestamp: now,
            },
        );

        reentrancy_guard::clear_entered(&env);
        Ok(batch_size)
    }

}
