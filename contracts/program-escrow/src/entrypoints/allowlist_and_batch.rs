#[cfg(feature = "contract")]
#[contractimpl]
impl ProgramEscrowContract {
    pub fn batch_payout_by(
        env: Env,
        caller: Address,
        recipients: soroban_sdk::Vec<Address>,
        amounts: soroban_sdk::Vec<i128>,
    ) -> ProgramData {
        Self::batch_payout_internal(env, Some(caller), None, recipients, amounts)
    }

    /// Compute a deterministic Merkle root over a batch of `(recipient, amount)` pairs.
    ///
    /// Builds a binary Merkle tree from the ordered leaves. If the leaf count is odd,
    /// the last leaf is duplicated to complete the tree level (standard Merkle padding).
    ///
    /// # Arguments
    /// * `env` - Contract environment
    /// * `recipients` - Ordered vector of recipient addresses
    /// * `amounts` - Ordered vector of amounts (same length as recipients)
    ///
    /// # Returns
    /// SHA-256 Merkle root as `BytesN<32>`
    fn compute_batch_merkle_root(
        env: &Env,
        recipients: &Vec<Address>,
        amounts: &Vec<i128>,
    ) -> BytesN<32> {
        let mut leaves: Vec<BytesN<32>> = Vec::new(env);
        for i in 0..recipients.len() {
            let recipient = recipients.get(i).unwrap();
            let amount = amounts.get(i).unwrap();
            let leaf_data = (recipient, amount).to_xdr(env);
            let leaf_hash: BytesN<32> = env.crypto().sha256(&leaf_data).into();
            leaves.push_back(leaf_hash);
        }

        // Build Merkle tree bottom-up
        let mut level = leaves;
        while level.len() > 1 {
            let mut next_level: Vec<BytesN<32>> = Vec::new(env);
            let mut i = 0;
            while i < level.len() {
                let left = level.get(i).unwrap();
                let right = if i + 1 < level.len() {
                    level.get(i + 1).unwrap()
                } else {
                    left.clone() // Duplicate last leaf if odd count
                };
                let combined = (left, right).to_xdr(env);
                let parent: BytesN<32> = env.crypto().sha256(&combined).into();
                next_level.push_back(parent);
                i += 2;
            }
            level = next_level;
        }
        level.get(0).unwrap()

    }

    /// Returns the current dispute state for the contract.
    /// Returns `DisputeState::None` if no dispute record is stored.
    fn dispute_state(env: &Env) -> DisputeState {
        env.storage()
            .instance()
            .get::<DataKey, DisputeRecord>(&DataKey::Dispute)
            .map(|record| record.state)
            .unwrap_or(DisputeState::None)
    }

    fn batch_payout_internal(
        env: Env,
        caller: Option<Address>,
        idempotency_key: Option<String>,
        recipients: soroban_sdk::Vec<Address>,
        amounts: soroban_sdk::Vec<i128>,
    ) -> ProgramData {
        // Validation precedence (deterministic ordering):
        // 1.  Reentrancy guard
        // 1b. Idempotency check (early-exit before any state reads)
        // 2.  Contract initialized
        // 3.  Paused (operational state)
        // 3b. Dispute guard
        // 3c. Circuit breaker (single check, before all business logic)
        // 4.  Authorization
        // 5a. Length / empty / batch-size checks
        // 5b. Per-entry validation: zero amounts, duplicate recipients
        // 6.  Compute total atomically (overflow check)
        // 6b. Idempotency key deduplication (needs total_payout)
        // 7.  Business logic: spend threshold, balance
        // 8.  Pre-validate fees for every entry (atomicity — no partial state)
        // 9.  Execute transfers

        reentrancy_guard::acquire(&env);

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
        let program_data: ProgramData = match env.storage().instance().get(&PROGRAM_DATA) {
            Some(d) => d,
            None => panic!("Program not initialized"),
        };

        // 2b. Program lifecycle: Draft programs must be published before payouts.
        Self::require_active_program(&program_data);

        // 3. Operational state: paused
        //    PRECEDENCE LAYER 1 (highest): Pause / maintenance mode.
        //    Checked BEFORE read-only mode and circuit breaker so that an
        //    operator's explicit emergency stop is always honoured first,
        //    regardless of automated circuit-breaker state.
        //    See docs/program-escrow/CIRCUIT_BREAKER_ENFORCEMENT.md §Layer Definitions.
        if Self::check_paused(&env, Some(&program_data.program_id), symbol_short!("release")) {
            panic!("Funds Paused");
        }

        if Self::dispute_state(&env) == DisputeState::Open {
            panic!("Payout blocked: dispute open");
        }

        // 3c. Circuit breaker — single authoritative check before all business
        //     logic so clients observe a stable, deterministic rejection.
        if let Err(err_code) = error_recovery::check_and_allow_with_thresholds(&env) {
            reentrancy_guard::release(&env);
            if err_code == error_recovery::ERR_CIRCUIT_OPEN {
                panic!("Circuit breaker is OPEN");
            } else {
                panic!("Operation rejected by circuit breaker");
            }
        }

        Self::authorize_release_actor(&env, &program_data, caller.as_ref());

        // 5a. Length / empty / batch-size checks (deterministic ordering)
        if recipients.len() != amounts.len() {
            panic!("Recipients and amounts vectors must have the same length");
        }

        if recipients.len() == 0 {
            panic!("Cannot process empty batch");
        }

        if recipients.len() > MAX_BATCH_SIZE {
            panic_with_error!(&env, BatchError::BatchTooLarge);
        }

        for i in 0..amounts.len() {
            if amounts.get(i).unwrap() <= 0 {
                panic!("All amounts must be greater than zero");
            }
        }
        for i in 0..recipients.len() {
            for j in (i + 1)..recipients.len() {
                if recipients.get(i).unwrap() == recipients.get(j).unwrap() {
                    panic!("Duplicate recipient in batch");
                }
            }
        }

        let mut total_payout: i128 = 0;
        for amount in amounts.iter() {
            total_payout = match total_payout.checked_add(amount) {
                Some(v) => v,
                None => panic!("Payout amount overflow"),
            };
        }

        // 6b. Idempotency key deduplication (now that we have total_payout)
        let executor = caller.unwrap_or_else(|| env.current_contract_address());
        if let Err(existing_record) = Self::handle_idempotency(
            &env,
            idempotency_key.clone(),
            symbol_short!("batchpay"),
            &program_data.program_id,
            total_payout,
            recipients.len() as u32,
        ) {
            // Return deterministic result for retry: mirror the original outcome.
            if existing_record.success {
                return program_data;
            } else {
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

        // 7. Business logic: spend threshold then balance.
        //    Deterministic ordering: threshold before balance so clients observe
        //    stable failures regardless of current balance.
        if Self::enforce_spend_threshold(&env, &program_data.program_id, total_payout).is_err() {
            panic!("Spend threshold exceeded");
        }
        Self::enforce_spending_window(&env, &program_data.program_id, total_payout);
        if total_payout > program_data.remaining_balance {
            panic!("Insufficient balance");
        }

        // 8. Pre-validate fees for every entry BEFORE any transfer.
        //    This guarantees atomicity: if any fee would consume an entire payout
        //    the whole batch is rejected with no state changes.
        let cfg = Self::get_fee_config_internal(&env);
        let batch_fee_waived = Self::is_fee_waived(cfg.fee_waivers, &PayoutType::Batch(0));
        let mut net_amounts: soroban_sdk::Vec<i128> = soroban_sdk::Vec::new(&env);
        let mut fee_amounts: soroban_sdk::Vec<i128> = soroban_sdk::Vec::new(&env);
        let mut transfer_amounts: soroban_sdk::Vec<i128> = soroban_sdk::Vec::new(&env);
        let mut total_actual_outflow: i128 = 0;
        for i in 0..recipients.len() {
            let gross = amounts.get(i).unwrap();
            let pay_fee = if batch_fee_waived {
                0
            } else {
                Self::combined_fee_amount(
                    gross,
                    cfg.payout_fee_rate,
                    cfg.payout_fixed_fee,
                    cfg.fee_enabled,
                )
            };
            let net = match gross.checked_sub(pay_fee) {
                Some(v) if v > 0 => v,
                _ => panic!("Payout fee consumes entire payout"),
            };

            // Apply FoT routing to compute actual transfer amount needed
            // to deliver the intended net after fee-on-transfer deductions.
            let transfer_amount = fot_routing::apply_fot_router(
                &env,
                &program_data.token_address,
                net,
                &program_data.fot_router,
            );

            let debit = pay_fee
                .checked_add(transfer_amount)
                .expect("Batch payout debit overflow");
            total_actual_outflow = total_actual_outflow
                .checked_add(debit)
                .expect("Batch total outflow overflow");

            net_amounts.push_back(net);
            fee_amounts.push_back(pay_fee);
            transfer_amounts.push_back(transfer_amount);
        }

        // Balance check uses the actual total outflow including FoT markup.
        if total_actual_outflow > program_data.remaining_balance {
            panic!("Insufficient balance");
        }

        // 9. Execute transfers — all pre-validation passed; this section must not fail.
        let mut updated_history = program_data.payout_history.clone();
        let timestamp = env.ledger().timestamp();
        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &program_data.token_address);

        for i in 0..recipients.len() {
            let recipient = recipients.get(i).unwrap().clone();
            let transfer_amount = transfer_amounts.get(i).unwrap();
            let pay_fee = fee_amounts.get(i).unwrap();
            let _gross = amounts.get(i).unwrap();

            if pay_fee > 0 {
                let (reserve_share, recipient_share) =
                    Self::split_fee_for_reserve(pay_fee, cfg.insurance_reserve_bps);
                if recipient_share > 0 {
                    token_client.transfer(
                        &contract_address,
                        &cfg.fee_recipient,
                        &recipient_share,
                    );
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
            // Chaos harness (test-only): may panic to simulate a mid-batch
            // cross-contract transfer failure before the real token call.
            #[cfg(test)]
            chaos::tick_before_transfer(&env, i);

            token_client.transfer(&contract_address, &recipient, &transfer_amount);
            error_recovery::record_success(&env);
            threshold_monitor::record_operation_success(&env);
            threshold_monitor::record_outflow(&env, pay_fee + transfer_amount);
            let record = PayoutRecord {
                recipient: recipient.clone(),
                amount: transfer_amount,
                timestamp,
            };
            updated_history.push_back(record.clone());
            // Lazy recipient index
            Self::append_recipient_index(
                &env,
                &program_data.program_id,
                &recipient,
                &record,
            );
        }

        // Update program data atomically after all transfers succeed.
        let mut updated_data = program_data.clone();
        updated_data.remaining_balance = updated_data
            .remaining_balance
            .checked_sub(total_actual_outflow)
            .expect("Remaining balance underflow");
        updated_data.payout_history = updated_history;
        // Keep legacy PROGRAM_DATA and keyed program registry in sync so
        // `get_program_info_v2` reflects payouts performed via batch_payout*.
        Self::store_program_data(&env, &updated_data.program_id, &updated_data);

        // Store idempotency record (CEI: after state mutation, before event).
        if let Some(ref key) = idempotency_key {
            Self::store_idempotency_record(
                &env,
                key.clone(),
                symbol_short!("batchpay"),
                updated_data.program_id.clone(),
                total_actual_outflow,
                recipients.len() as u32,
                executor,
            );
        }

        // Emit BatchPayout event.
        env.events().publish(
            (BATCH_PAYOUT,),
            BatchPayoutEvent {
                version: EVENT_VERSION_V2,
                program_id: updated_data.program_id.clone(),
                recipient_count: recipients.len() as u32,
                total_amount: total_actual_outflow,
                remaining_balance: updated_data.remaining_balance,
                idempotency_key,
                correlation_id: None,
            },
        );

        // Release reentrancy guard on success.
        reentrancy_guard::release(&env);
        updated_data
    }

    /// Returns the batch payout storage schema version written during `init_program`.
    /// Returns `0` on legacy deployments where the marker was never written.
    pub fn get_batch_payout_schema_version(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::BatchPayoutSchemaVersion)
            .unwrap_or(0u32)
    }

}
