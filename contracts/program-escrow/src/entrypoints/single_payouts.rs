#[cfg(feature = "contract")]
#[contractimpl]
impl ProgramEscrowContract {
    pub fn single_payout(
        env: Env,
        recipient: Address,
        amount: i128,
        idempotency_key: Option<String>,
    ) -> ProgramData {
        Self::single_payout_internal(env, None, recipient, amount, idempotency_key)
    }

    /// Execute a single payout with a specified caller.
    pub fn single_payout_by(
        env: Env,
        caller: Address,
        recipient: Address,
        amount: i128,
        idempotency_key: Option<String>,
    ) -> ProgramData {
        Self::single_payout_internal(env, Some(caller), recipient, amount, idempotency_key)
    }

    fn single_payout_internal(
        env: Env,
        caller: Option<Address>,
        recipient: Address,
        amount: i128,
        idempotency_key: Option<String>,
    ) -> ProgramData {
        // Validation precedence (deterministic ordering):
        // 1. Reentrancy guard
        // 1b. Idempotency check
        // 2. Contract initialized
        // 3. Paused (operational state)
        // 3b. Dispute guard
        // 3c. Circuit breaker — before all business logic for deterministic rejection
        // 4. Authorization
        // 6. Business logic (sufficient balance)
        // 7. Circuit breaker check

        reentrancy_guard::acquire(&env);

        // 1b. Idempotency check — runs before any state reads so duplicate
        //     submissions are rejected cheaply and deterministically.
        if let Some(ref key) = idempotency_key {
            if env
                .storage()
                .persistent()
                .has(&DataKey::IdempotencyKey(key.clone()))
            {
                panic!("Payout already processed");
            }
        }

        // 2. Contract must be initialized
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        // 2b. Program lifecycle: Draft programs must be published before payouts.
        Self::require_active_program(&program_data);

        // 3. Operational state: paused
        if Self::check_paused(&env, Some(&program_data.program_id), symbol_short!("release")) {
            panic!("Funds Paused");
        }

        // 3b. Dispute guard — payouts blocked while a dispute is open
        if Self::dispute_state(&env) == DisputeState::Open {
            panic!("Payout blocked: dispute open");
        }

        // 3c. Circuit breaker check — runs before all business logic so that
        //     an open circuit produces a deterministic, stable rejection
        //     regardless of balance or threshold state.
        if let Err(err_code) = error_recovery::check_and_allow_with_thresholds(&env) {
            reentrancy_guard::clear_entered(&env);
            if err_code == error_recovery::ERR_CIRCUIT_OPEN {
                panic!("Circuit breaker is OPEN");
            } else {
                panic!("Operation rejected by circuit breaker");
            }
        }

        // 4. Authorization
        Self::authorize_release_actor(&env, &program_data, caller.as_ref());

        // 5. Input validation
        if amount <= 0 {
            panic!("Amount must be greater than zero");
        }

        // 5a. Idempotency key validation (deterministic behavior)
        let executor = caller.unwrap_or_else(|| env.current_contract_address());
        if let Err(existing_record) = Self::handle_idempotency(
            &env,
            idempotency_key.clone(),
            symbol_short!("singlepay"),
            &program_data.program_id,
            amount,
            1, // Single payout has 1 recipient
        ) {
            // Return the same result as the original operation for deterministic behavior
            if existing_record.success {
                // Return the stored program data (simulate successful retry)
                return program_data;
            } else {
                // Retry the same error
                if let Some(error_code) = existing_record.error_code {
                    panic!(
                        "Idempotency retry: operation failed with code {}",
                        error_code
                    );
                } else {
                    panic!("Idempotency retry: operation failed");
                }
            }
        }

        // 6. Business logic: sufficient balance
        // Deterministic error ordering: spend threshold check runs before
        // balance checks, so clients observe stable failures.
        if Self::enforce_spend_threshold(&env, &program_data.program_id, amount).is_err() {
            panic!("Spend threshold exceeded");
        }

        // Per-window spending limit check (after per-payout threshold, before balance)
        Self::enforce_spending_window(&env, &program_data.program_id, amount);

        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &program_data.token_address);
        let cfg = Self::get_fee_config_internal(&env);
        let pay_fee = if Self::is_fee_waived(cfg.fee_waivers, &PayoutType::Single) {
            0
        } else {
            Self::combined_fee_amount(
                amount,
                cfg.payout_fee_rate,
                cfg.payout_fixed_fee,
                cfg.fee_enabled,
            )
        };
        let net = amount.checked_sub(pay_fee).unwrap_or(0);
        if net <= 0 {
            panic!("Payout fee consumes entire payout");
        }

        // Apply FoT routing to compute actual transfer amount needed
        // to deliver the intended net after fee-on-transfer deductions.
        let transfer_amount = fot_routing::apply_fot_router(
            &env,
            &program_data.token_address,
            net,
            &program_data.fot_router,
        );

        // Total debit from remaining_balance = protocol fee + routed transfer
        let total_debit = pay_fee
            .checked_add(transfer_amount)
            .expect("Payout debit overflow");

        // Balance check accounts for the actual outflow including FoT markup
        if total_debit > program_data.remaining_balance {
            panic!("Insufficient balance");
        }

        if pay_fee > 0 {
            let (reserve_share, recipient_share) =
                Self::split_fee_for_reserve(pay_fee, cfg.insurance_reserve_bps);
            if recipient_share > 0 {
                token_client.transfer(&contract_address, &cfg.fee_recipient, &recipient_share);
            }
            Self::accrue_insurance_reserve(&env, reserve_share);
            Self::emit_fee_collected(
                &env,
                symbol_short!("payout"),
                pay_fee,
                cfg.payout_fee_rate,
                cfg.payout_fixed_fee,
                cfg.fee_recipient.clone(),
            );
        }

        token_client.transfer(&contract_address, &recipient, &transfer_amount);

        error_recovery::record_success(&env);
        threshold_monitor::record_operation_success(&env);
        // Record outflow using the amount debited from remaining_balance
        threshold_monitor::record_outflow(&env, total_debit);

        let timestamp = env.ledger().timestamp();
        let payout_record = PayoutRecord {
            recipient: recipient.clone(),
            amount: transfer_amount,
            timestamp,
        };

        let mut updated_history = program_data.payout_history.clone();
        updated_history.push_back(payout_record.clone());

        let mut updated_data = program_data.clone();
        updated_data.remaining_balance = updated_data
            .remaining_balance
            .checked_sub(total_debit)
            .expect("Remaining balance underflow");
        updated_data.payout_history = updated_history;

        Self::store_program_data(&env, &updated_data.program_id, &updated_data);

        // Lazy recipient index — write to persistent storage so the index
        // survives instance TTL eviction.  Initialized on first write only.
        Self::append_recipient_index(
            &env,
            &updated_data.program_id,
            &payout_record.recipient,
            &payout_record,
        );

        // Store idempotency record if key was provided
        if let Some(key) = idempotency_key {
            Self::store_idempotency_record(
                &env,
                key,
                symbol_short!("singlepay"),
                updated_data.program_id.clone(),
                amount,
                1, // Single payout has 1 recipient
                executor,
            );
        }

        env.events().publish(
            (PAYOUT,),
            PayoutEvent {
                version: EVENT_VERSION_V2,
                program_id: updated_data.program_id.clone(),
                recipient: recipient.clone(),
                amount: transfer_amount,
                remaining_balance: updated_data.remaining_balance,
                correlation_id: None,
            },
        );

        reentrancy_guard::release(&env);

        updated_data
    }

    /// Execute a single payout with idempotency support.
    ///
    /// # Arguments
    /// * `recipient` - Address of the winner.
    /// * `amount` - Amount to transfer.
    /// * `idempotency_key` - Optional unique key to ensure idempotent behavior.
    ///
    /// # Returns
    /// The updated `ProgramData` reflecting the new balance and payout history.
    ///
    /// # Idempotency
    /// - If `idempotency_key` is provided and already used, returns the stored result without re-executing.
    /// - If `idempotency_key` is provided and new, executes the payout and stores the key.
    /// - If `idempotency_key` is None, behaves like regular single_payout.
    ///
    /// # Security
    /// - Requires authorization from the `authorized_payout_key`.
    /// - Protected by reentrancy guard.
    /// - Respects circuit breaker and threshold limits.
    pub fn single_payout_idempotent(
        env: Env,
        recipient: Address,
        amount: i128,
        idempotency_key: Option<String>,
    ) -> ProgramData {
        Self::single_payout_idempotent_internal(env, None, recipient, amount, idempotency_key)
    }

    pub fn single_payout_idempotent_by(
        env: Env,
        caller: Address,
        recipient: Address,
        amount: i128,
        idempotency_key: Option<String>,
    ) -> ProgramData {
        Self::single_payout_idempotent_internal(
            env,
            Some(caller),
            recipient,
            amount,
            idempotency_key,
        )
    }

    fn single_payout_idempotent_internal(
        env: Env,
        caller: Option<Address>,
        recipient: Address,
        amount: i128,
        idempotency_key: Option<String>,
    ) -> ProgramData {
        // ── Replay detection ───────────────────────────────────────────────
        // Check the shared DataKey::IdempotencyKey namespace (instance storage)
        // first.  This catches replay of a key consumed by batch_payout_idempotent
        // (or a prior single_payout_idempotent that stored via the shared path).
        if let Some(ref key) = idempotency_key {
            if let Some(record) = Self::get_idempotency_record(&env, key) {
                let program_data: ProgramData = env
                    .storage()
                    .instance()
                    .get(&PROGRAM_DATA)
                    .unwrap_or_else(|| panic!("Program not initialized"));

                env.events().publish(
                    (symbol_short!("IdmReplay"),),
                    (
                        record.idempotency_key.clone(),
                        record.program_id.clone(),
                        record.total_amount,
                    ),
                );

                return program_data;
            }
        }

        // Check legacy DataKey::PayoutIdempotency namespace (persistent storage)
        // for backwards compatibility with keys stored before the shared namespace.
        if let Some(existing_record) =
            Self::validate_and_get_idempotency_key(&env, &idempotency_key)
        {
            let program_data: ProgramData = env
                .storage()
                .instance()
                .get(&PROGRAM_DATA)
                .unwrap_or_else(|| panic!("Program not initialized"));

            env.events().publish(
                (symbol_short!("IdmReplay"),),
                (
                    existing_record.key.clone(),
                    existing_record.program_id.clone(),
                    existing_record.total_amount,
                ),
            );

            return program_data;
        }

        // Execute normal payout
        let program_data =
            Self::single_payout_internal(env.clone(), caller.clone(), recipient.clone(), amount, None);

        // Store idempotency key if provided
        if let Some(key) = &idempotency_key {
            // Legacy storage (DataKey::PayoutIdempotency, persistent)
            Self::store_idempotency_key(
                &env,
                key,
                &program_data.program_id,
                PayoutType::Single,
                Some(recipient),
                Some(amount),
                None,
                None,
                amount,
            );

            // Shared namespace (DataKey::IdempotencyKey, instance) so that
            // batch_payout_idempotent and is_payout_processed can detect it.
            let executor = caller.unwrap_or_else(|| env.current_contract_address());
            Self::store_idempotency_record(
                &env,
                key.clone(),
                symbol_short!("singlepay"),
                program_data.program_id.clone(),
                amount,
                1,
                executor,
            );
        }

        program_data
    }

}
