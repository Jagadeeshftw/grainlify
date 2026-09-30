#[cfg(feature = "contract")]
#[contractimpl]
impl ProgramEscrowContract {
    fn default_history_pagination_config(_env: &Env) -> HistoryPaginationConfig {
        HistoryPaginationConfig {
            max_limit: 100,
            schema_version: PAGINATION_SCHEMA_VERSION_V1,
        }
    }

    fn get_history_pagination_config(env: &Env) -> HistoryPaginationConfig {
        env.storage()
            .instance()
            .get(&DataKey::HistoryPaginationConfig)
            .unwrap_or_else(|| Self::default_history_pagination_config(env))
    }

    fn ensure_history_pagination_config(env: &Env) {
        if !env
            .storage()
            .instance()
            .has(&DataKey::HistoryPaginationConfig)
        {
            env.storage().instance().set(
                &DataKey::HistoryPaginationConfig,
                &Self::default_history_pagination_config(&env),
            );
        }
    }

    fn validate_pagination_schema(env: &Env) -> Result<(), BatchError> {
        let config = Self::get_history_pagination_config(env);
        if config.schema_version != PAGINATION_SCHEMA_VERSION_V1 {
            return Err(BatchError::InvalidPaginationOffset);
        }
        Ok(())
    }

    fn validate_pagination(env: &Env, limit: u32) -> Result<(), BatchError> {
        if limit == 0 {
            return Err(BatchError::InvalidPaginationLimit);
        }

        // Validate schema version for upgrade safety
        Self::validate_pagination_schema(env)?;

        let cfg = Self::get_history_pagination_config(env);
        if limit > cfg.max_limit {
            return Err(BatchError::PaginationLimitExceeded);
        }
        Ok(())
    }

    fn paginate_filtered<T, F>(
        env: &Env,
        entries: soroban_sdk::Vec<T>,
        offset: u32,
        limit: u32,
        mut predicate: F,
    ) -> Result<soroban_sdk::Vec<T>, BatchError>
    where
        T: Clone
            + soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>
            + soroban_sdk::IntoVal<soroban_sdk::Env, soroban_sdk::Val>,
        F: FnMut(&T) -> bool,
    {
        // Validate offset for deterministic behavior
        if offset >= entries.len() as u32 {
            return Ok(Vec::new(env));
        }

        let mut results = Vec::new(env);
        let mut count = 0u32;
        let mut processed = 0u32;

        // Process entries in deterministic order (as stored)
        for entry in entries.iter() {
            if predicate(&entry) {
                if processed >= offset && count < limit {
                    results.push_back(entry);
                    count += 1;
                }
                processed += 1;
            } else {
                // Count non-matching entries for offset calculation
                processed += 1;
            }
        }

        Ok(results)
    }



    fn order_batch_lock_items(env: &Env, items: &Vec<LockItem>) -> soroban_sdk::Vec<LockItem> {
        let mut ordered: soroban_sdk::Vec<LockItem> = Vec::new(env);
        for item in items.iter() {
            let mut next: soroban_sdk::Vec<LockItem> = Vec::new(env);
            let mut inserted = false;
            for existing in ordered.iter() {
                // String comparison for deterministic ordering
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

    fn order_batch_release_items(
        env: &Env,
        items: &Vec<ReleaseItem>,
    ) -> soroban_sdk::Vec<ReleaseItem> {
        let mut ordered: soroban_sdk::Vec<ReleaseItem> = Vec::new(env);
        for item in items.iter() {
            let mut next: soroban_sdk::Vec<ReleaseItem> = Vec::new(env);
            let mut inserted = false;
            for existing in ordered.iter() {
                // Sort by program_id then schedule_id
                let cmp = if item.program_id < existing.program_id {
                    true
                } else if item.program_id == existing.program_id {
                    item.schedule_id < existing.schedule_id
                } else {
                    false
                };

                if !inserted && cmp {
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

    fn increment_receipt_id(env: &Env) -> u64 {
        let mut id: u64 = env.storage().instance().get(&RECEIPT_ID).unwrap_or(0);
        id += 1;
        env.storage().instance().set(&RECEIPT_ID, &id);
        id
    }

    // ========================================================================
    // Idempotency Key Management
    // ========================================================================

    /// Validate idempotency key format and constraints
    fn validate_idempotency_key(idempotency_key: &String) {
        Self::validate_idempotency_key_format(idempotency_key);
    }

    /// Check if an idempotency key has been used before
    fn get_idempotency_record(env: &Env, idempotency_key: &String) -> Option<IdempotencyRecord> {
        env.storage()
            .instance()
            .get(&DataKey::IdempotencyKey(idempotency_key.clone()))
    }

    /// Store a new idempotency record for a successful operation
    fn store_idempotency_record(
        env: &Env,
        idempotency_key: String,
        operation_type: Symbol,
        program_id: String,
        total_amount: i128,
        recipient_count: u32,
        executor: Address,
    ) {
        let record = IdempotencyRecord {
            idempotency_key: idempotency_key.clone(),
            operation_type,
            success: true,
            executed_at: env.ledger().timestamp(),
            executor,
            program_id,
            total_amount,
            recipient_count,
            error_code: None,
        };

        env.storage()
            .instance()
            .set(&DataKey::IdempotencyKey(idempotency_key), &record);

        // Emit idempotency key used event
        env.events().publish(
            (IDEMPOTENCY_KEY_USED,),
            IdempotencyKeyUsedEvent {
                version: EVENT_VERSION_V2,
                idempotency_key: record.idempotency_key.clone(),
                operation_type: record.operation_type,
                program_id: record.program_id,
                total_amount: record.total_amount,
                recipient_count: record.recipient_count,
                executor: record.executor,
                executed_at: record.executed_at,
            },
        );
    }

    /// Store idempotency record for a failed operation
    fn store_idempotency_failure(
        env: &Env,
        idempotency_key: String,
        operation_type: Symbol,
        program_id: String,
        total_amount: i128,
        recipient_count: u32,
        executor: Address,
        error_code: u32,
    ) {
        let record = IdempotencyRecord {
            idempotency_key: idempotency_key.clone(),
            operation_type,
            success: false,
            executed_at: env.ledger().timestamp(),
            executor,
            program_id,
            total_amount,
            recipient_count,
            error_code: Some(error_code),
        };

        env.storage()
            .instance()
            .set(&DataKey::IdempotencyKey(idempotency_key), &record);
    }

    /// Handle idempotency key validation and retry logic
    fn handle_idempotency(
        env: &Env,
        idempotency_key: Option<String>,
        _operation_type: Symbol,
        _program_id: &String,
        _total_amount: i128,
        _recipient_count: u32,
    ) -> Result<(), IdempotencyRecord> {
        // If no idempotency key provided, proceed with normal operation
        let idempotency_key = match idempotency_key {
            Some(key) => {
                Self::validate_idempotency_key(&key);
                key
            }
            None => return Ok(()), // No idempotency key, proceed normally
        };

        // Check if this idempotency key has been used before
        if let Some(existing_record) = Self::get_idempotency_record(env, &idempotency_key) {
            // Emit retry event for audit trail
            env.events().publish(
                (IDEMPOTENCY_KEY_USED,),
                IdempotencyKeyRetryEvent {
                    version: EVENT_VERSION_V2,
                    idempotency_key: idempotency_key.clone(),
                    original_success: existing_record.success,
                    original_executed_at: existing_record.executed_at,
                    original_executor: existing_record.executor.clone(),
                    retry_attempt_at: env.ledger().timestamp(),
                    retry_by: env.current_contract_address(),
                },
            );

            // Return the existing record to signal a retry attempt
            return Err(existing_record);
        }

        // New idempotency key, proceed with operation
        Ok(())
    }

}
