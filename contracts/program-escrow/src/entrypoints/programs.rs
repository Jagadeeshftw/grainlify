#[cfg(feature = "contract")]
#[contractimpl]
impl ProgramEscrowContract {
    pub fn init_program(
        env: Env,
        program_id: String,
        authorized_payout_key: Address,
        token_address: Address,
        creator: Address,
        initial_liquidity: Option<i128>,
        reference_hash: Option<soroban_sdk::Bytes>,
    ) -> ProgramData {
        Self::initialize_program(
            env,
            program_id,
            authorized_payout_key,
            token_address,
            creator,
            initial_liquidity,
            reference_hash,
        )
    }

    /// Internal implementation for initializing a program.
    pub fn initialize_program(
        env: Env,
        program_id: String,
        authorized_payout_key: Address,
        token_address: Address,
        creator: Address,
        initial_liquidity: Option<i128>,
        reference_hash: Option<soroban_sdk::Bytes>,
    ) -> ProgramData {
        // Check if program already exists
        let program_key = DataKey::Program(program_id.clone());
        if env.storage().instance().has(&program_key) {
            panic!("Program already initialized");
        }

        // ── Token allowlist enforcement ──────────────────────────────────────
        // When the allowlist is non-empty, reject any token not on the list.
        // Emits TokenRejectedEvent before panicking so the rejection is always
        // visible on-chain. Deterministic: this check runs before any state
        // mutation so no partial writes occur on rejection.
        Self::enforce_token_allowlist(&env, &token_address, &program_id);

        if !env.storage().instance().has(&FEE_CONFIG) {
            env.storage().instance().set(
                &FEE_CONFIG,
                &FeeConfig {
                    lock_fee_rate: 0,
                    payout_fee_rate: 0,
                    lock_fixed_fee: 0,
                    payout_fixed_fee: 0,
                    fee_recipient: authorized_payout_key.clone(),
                    fee_enabled: false,
                    fee_waivers: 0,
                    insurance_reserve_bps: 0,
                },
            );
        }

        let mut total_funds = 0i128;
        let mut remaining_balance = 0i128;
        let mut init_liquidity = 0i128;

        if let Some(amount) = initial_liquidity {
            if amount > 0 {
                // Transfer initial liquidity from creator to contract
                let contract_address = env.current_contract_address();
                let token_client = token::Client::new(&env, &token_address);
                creator.require_auth();
                token_client.transfer(&creator, &contract_address, &amount);

                let cfg = Self::get_fee_config_internal(&env);
                let fee = Self::combined_fee_amount(
                    amount,
                    cfg.lock_fee_rate,
                    cfg.lock_fixed_fee,
                    cfg.fee_enabled,
                );
                let net = amount.checked_sub(fee).unwrap_or(0);
                if net <= 0 {
                    panic!("Lock fee consumes entire initial liquidity");
                }
                if fee > 0 {
                    let (reserve_share, recipient_share) =
                        Self::split_fee_for_reserve(fee, cfg.insurance_reserve_bps);
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
                        symbol_short!("lock"),
                        fee,
                        cfg.lock_fee_rate,
                        cfg.lock_fixed_fee,
                        cfg.fee_recipient.clone(),
                    );
                }
                total_funds = net;
                remaining_balance = net;
                init_liquidity = net;
            }
        }

        let program_data = ProgramData {
            program_id: program_id.clone(),
            total_funds,
            remaining_balance,
            authorized_payout_key: authorized_payout_key.clone(),
            delegate: None,
            delegate_permissions: 0,
            payout_history: Vec::new(&env),
            token_address: token_address.clone(),
            initial_liquidity: init_liquidity,
            risk_flags: 0,
            reference_hash,
            archived: false,
            archived_at: None,
            status: ProgramStatus::Draft,
            circuit_breaker_threshold: None,
            fot_router: OptionalFotRouter::None,
        };

        // Store program data in registry
        let program_key = DataKey::Program(program_id.clone());
        env.storage().instance().set(&program_key, &program_data);

        // Record the initial transition into Draft status
        Self::record_status_transition(
            &env,
            &program_id,
            &ProgramStatus::Draft,
            &ProgramStatus::Draft,
        );

        let mut registry: soroban_sdk::Vec<String> = env
            .storage()
            .instance()
            .get(&PROGRAM_REGISTRY)
            .unwrap_or(Vec::new(&env));
        let mut exists = false;
        for r in registry.iter() {
            if r == program_id {
                exists = true;
                break;
            }
        }
        if !exists {
            registry.push_back(program_id.clone());
            env.storage().instance().set(&PROGRAM_REGISTRY, &registry);
        }

        // Track dependencies (default empty)
        let empty_dependencies: soroban_sdk::Vec<String> = vec![&env];
        env.storage().instance().set(
            &DataKey::ProgramDependencies(program_id.clone()),
            &empty_dependencies,
        );
        env.storage().instance().set(
            &DataKey::DependencyStatus(program_id.clone()),
            &DependencyStatus::Pending,
        );

        // Store program data
        env.storage().instance().set(&PROGRAM_DATA, &program_data);

        if !env.storage().instance().has(&FEE_CONFIG) {
            env.storage().instance().set(
                &FEE_CONFIG,
                &FeeConfig {
                    lock_fee_rate: 0,
                    payout_fee_rate: 0,
                    lock_fixed_fee: 0,
                    payout_fixed_fee: 0,
                    fee_recipient: authorized_payout_key.clone(),
                    fee_enabled: false,
                    fee_waivers: 0,
                    insurance_reserve_bps: 0,
                },
            );
        }

        // Fallback for legacy tests: if admin not set, set it to authorized_payout_key
        if !env.storage().instance().has(&DataKey::Admin) {
            env.storage()
                .instance()
                .set(&DataKey::Admin, &authorized_payout_key);
        }
        if !env.storage().instance().has(&DataKey::MaintenanceMode) {
            env.storage()
                .instance()
                .set(&DataKey::MaintenanceMode, &false);
        }
        if !env.storage().instance().has(&DataKey::PauseFlags) {
            env.storage().instance().set(
                &DataKey::PauseFlags,
                &PauseFlags {
                    lock_paused: false,
                    release_paused: false,
                    refund_paused: false,
                    pause_reason: None,
                    paused_at: 0,
                    lock_unpause_at: None,
                    release_unpause_at: None,
                    refund_unpause_at: None,
                },
            );
        }
        Self::ensure_history_pagination_config(&env);

        // Write upgrade-safe spend-limit schema version marker.
        if !env
            .storage()
            .instance()
            .has(&DataKey::SpendLimitSchemaVersion)
        {
            env.storage().instance().set(
                &DataKey::SpendLimitSchemaVersion,
                &SPEND_LIMIT_SCHEMA_VERSION_V1,
            );
            env.events().publish(
                (SPEND_LIMIT_SCHEMA,),
                SpendLimitSchemaVersionSet {
                    version: EVENT_VERSION_V2,
                    schema_version: SPEND_LIMIT_SCHEMA_VERSION_V1,
                    timestamp: env.ledger().timestamp(),
                },
            );
        }

        // Write upgrade-safe pause flags schema version marker.
        if !env.storage().instance().has(&DataKey::PauseSchemaVersion) {
            env.storage()
                .instance()
                .set(&DataKey::PauseSchemaVersion, &PAUSE_SCHEMA_VERSION_V1);
        }

        // Write upgrade-safe circuit-breaker schema version marker.
        // Ensures future upgrades to circuit breaker storage layout are handled safely.
        if !env
            .storage()
            .instance()
            .has(&DataKey::CircuitBreakerSchemaVersion)
        {
            env.storage()
                .instance()
                .set(&DataKey::CircuitBreakerSchemaVersion, &CIRCUIT_BREAKER_SCHEMA_VERSION_V2);
            // Initialize circuit breaker admin only when none exists yet. Tests (and
            // operators) may call `set_circuit_admin` before the first program init;
            // re-calling with `caller=None` would panic once an admin is present.
            if error_recovery::get_circuit_admin(&env).is_none() {
                error_recovery::set_circuit_admin(&env, authorized_payout_key.clone(), None);
            }
            // Initialize with default configuration
            error_recovery::set_config(
                &env,
                error_recovery::CircuitBreakerConfig {
                    failure_threshold: 3,
                    success_threshold: 1,
                    max_error_log: 10,
                    recovery_window: 0,
                },
            );
            env.events().publish(
                (symbol_short!("circuit"),),
                (
                    symbol_short!("cb_init"),
                    env.ledger().timestamp(),
                    CIRCUIT_BREAKER_SCHEMA_VERSION_V2,
                ),
            );
        }

        // Write upgrade-safe token-allowlist schema version marker.
        if !env
            .storage()
            .instance()
            .has(&DataKey::TokenAllowlistSchemaVersion)
        {
            env.storage().instance().set(
                &DataKey::TokenAllowlistSchemaVersion,
                &TOKEN_ALLOWLIST_SCHEMA_VERSION_V1,
            );

            if !env
                .storage()
                .instance()
                .has(&DataKey::ReleaseTriggerSchemaVersion)
            {
                env.storage().instance().set(
                    &DataKey::ReleaseTriggerSchemaVersion,
                    &RELEASE_TRIGGER_SCHEMA_VERSION_V1,
                );
            }
            env.events().publish(
                (TOKEN_ALLOWLIST_SCHEMA,),
                TokenAllowlistSchemaVersionSet {
                    version: EVENT_VERSION_V2,
                    schema_version: TOKEN_ALLOWLIST_SCHEMA_VERSION_V1,
                    timestamp: env.ledger().timestamp(),
                },
            );
        }

        env.storage()
            .instance()
            .set(&SCHEDULES, &Vec::<ProgramReleaseSchedule>::new(&env));
        env.storage()
            .instance()
            .set(&RELEASE_HISTORY, &Vec::<ProgramReleaseHistory>::new(&env));
        env.storage().instance().set(&NEXT_SCHEDULE_ID, &1_u64);

        // Emit ProgramInitialized event
        env.events().publish(
            (PROGRAM_INITIALIZED,),
            ProgramInitializedEvent {
                version: EVENT_VERSION_V2,
                program_id,
                authorized_payout_key,
                token_address,
                total_funds,
            },
        );

        program_data
    }

    /// Require the initialized program to be Active before moving escrowed funds.
    ///
    /// # Panics
    /// Panics with `ERR_PROGRAM_NOT_ACTIVE` (107) when the program is still Draft.
    fn require_active_program(program_data: &ProgramData) {
        if program_data.status != ProgramStatus::Active {
            panic!("{}", errors::ERR_PROGRAM_NOT_ACTIVE);
        }
    }

    /// Publish a program, transitioning it from Draft to Active status.
    /// Only the contract admin or the program's authorized_payout_key (controller) may call this.
    ///
    /// # Arguments
    /// * `env` - The contract environment.
    /// * `program_id` - The unique identifier of the program to publish.
    /// * `caller` - The address of the caller (admin or controller) that must authorize.
    ///
    /// # Returns
    /// The updated ProgramData.
    ///
    /// # Panics
    /// Panics if the program is not initialized, if the caller is not authorized,
    /// or if the program is already in Active status.
    pub fn publish_program(env: Env, program_id: String, caller: Address) -> ProgramData {
        let mut program_data = Self::get_program_data_by_id(&env, &program_id);
        // Authorization: caller must be either admin or authorized_payout_key.
        Self::require_program_owner_or_admin(&env, &program_data, &caller);

        if program_data.status != ProgramStatus::Draft {
            panic!("Program already published");
        }

        program_data.status = ProgramStatus::Active;
        Self::store_program_data(&env, &program_id, &program_data);

        // Record the Draft → Active status transition
        Self::record_status_transition(
            &env,
            &program_id,
            &ProgramStatus::Draft,
            &ProgramStatus::Active,
        );

        // Emit ProgramPublished after the status write so indexers only see committed transitions.
        env.events().publish(
            (PROGRAM_PUBLISHED,),
            ProgramPublishedEvent {
                version: EVENT_VERSION_V2,
                program_id: program_data.program_id.clone(),
                publisher: caller.clone(),
                timestamp: env.ledger().timestamp(),
            },
        );

        program_data
    }

    /// Initialize a program with associated metadata.
    pub fn init_program_with_metadata(
        env: Env,
        program_id: String,
        authorized_payout_key: Address,
        token_address: Address,
        organizer: Option<Address>,
        metadata: Option<ProgramMetadata>,
    ) -> ProgramData {
        // Apply rate limiting
        anti_abuse::check_rate_limit(&env, authorized_payout_key.clone());

        let _start = env.ledger().timestamp();
        let caller = authorized_payout_key.clone();

        // Validate program_id (basic length check)
        if program_id.len() == 0 {
            panic!("Program ID cannot be empty");
        }

        if let Some(ref meta) = metadata {
            // Validate metadata fields (basic checks)
            if let Some(ref name) = meta.program_name {
                if name.len() == 0 {
                    panic!("Program name cannot be empty if provided");
                }
            }
            // Enforce custom_fields size/length limits (shared with update path).
            if meta.custom_fields.len() > MAX_PROGRAM_METADATA_CUSTOM_FIELDS {
                panic!("Metadata custom fields exceed limit");
            }
            validate_metadata_custom_fields(meta);
        }

        let mut program_data = Self::initialize_program(
            env.clone(),
            program_id,
            authorized_payout_key,
            token_address,
            organizer.unwrap_or(caller),
            None,
            None,
        );

        if let Some(ref pm) = metadata {
            // Store in legacy format for existing readers.
            env.storage()
                .instance()
                .set(&DataKey::Metadata(program_data.program_id.clone()), pm);
            // Store in compressed format for reduced storage cost.
            let compressed = CompressedProgramMetadata::from_legacy(&env, pm);
            env.storage().instance().set(
                &DataKey::MetadataV2(program_data.program_id.clone()),
                &compressed,
            );
        }

        program_data
    }

    /// Batch-initialize multiple programs in one transaction (all-or-nothing).
    ///
    /// # Atomicity guarantee
    /// This function performs pre-validation (batch size, duplicate detection,
    /// existence checks) **before** any storage mutation. If the registry-update
    /// loop fails partway through — for any reason, including token-allowlist
    /// rejection via `enforce_token_allowlist` or an invalid item — the Soroban
    /// runtime rolls back all storage writes from earlier iterations, and the
    /// `PROGRAM_REGISTRY` is **not** updated. No partially-initialized programs
    /// are left behind.
    ///
    /// # Pre-validation passes
    /// 1. **Batch size** — empty or `> MAX_BATCH_SIZE` ⇒ `InvalidBatchSizeProgram`
    /// 2. **Duplicate program_id** — duplicate IDs within `items` ⇒ `DuplicateProgramId`
    /// 3. **Existence check** — program_id already in storage ⇒ `ProgramAlreadyExists`
    ///
    /// # Errors
    /// * `BatchError::InvalidBatchSizeProgram` — empty, `> MAX_BATCH_SIZE`, or empty `program_id`
    /// * `BatchError::DuplicateProgramId` — duplicate `program_id` within `items`
    /// * `BatchError::ProgramAlreadyExists` — a `program_id` already registered
    ///
    /// # Panics
    /// * `"Token not on allowlist"` — if a token in an item is not on the allowlist
    ///
    /// # Benchmark note
    /// Pre-validation runs in O(n log n) for deduplication (insertion sort) plus
    /// O(n) for existence checks. At `MAX_BATCH_SIZE=100` the full call path
    /// (including the registry-update loop) costs ~X CPU instructions; see
    /// `docs/program-escrow-batch-init-atomicity.md` for the empirical table.
    pub fn batch_initialize_programs(
        env: Env,
        items: Vec<ProgramInitItem>,
    ) -> Result<u32, BatchError> {
        let batch_size = items.len() as u32;
        if batch_size == 0 || batch_size > MAX_BATCH_SIZE {
            return Err(BatchError::InvalidBatchSizeProgram);
        }
        {
            let mut program_ids: soroban_sdk::Vec<String> = soroban_sdk::Vec::new(&env);
            for i in 0..batch_size {
                program_ids.push_back(items.get(i).unwrap().program_id.clone());
            }
            let deduped = gas_optimization::deduplicate_program_ids(&env, &program_ids);
            if deduped.len() < program_ids.len() {
                return Err(BatchError::DuplicateProgramId);
            }
        }
        for i in 0..batch_size {
            let program_key = DataKey::Program(items.get(i).unwrap().program_id.clone());
            if env.storage().instance().has(&program_key) {
                return Err(BatchError::ProgramAlreadyExists);
            }
        }

        // Update registry
        let mut registry: soroban_sdk::Vec<String> = env
            .storage()
            .instance()
            .get(&PROGRAM_REGISTRY)
            .unwrap_or(vec![&env]);

        for i in 0..batch_size {
            let item = items.get(i).unwrap();
            let program_id = item.program_id.clone();
            let authorized_payout_key = item.authorized_payout_key.clone();
            let token_address = item.token_address.clone();

            if program_id.is_empty() {
                return Err(BatchError::InvalidBatchSizeProgram);
            }

            Self::enforce_token_allowlist(&env, &token_address, &program_id);

            let program_data = ProgramData {
                program_id: program_id.clone(),
                total_funds: 0,
                remaining_balance: 0,
                authorized_payout_key: authorized_payout_key.clone(),
                delegate: None,
                delegate_permissions: 0,
                payout_history: Vec::new(&env),
                token_address: token_address.clone(),
                initial_liquidity: 0,
                risk_flags: 0,
                reference_hash: item.reference_hash.clone(),
                archived: false,
                archived_at: None,
                status: ProgramStatus::Draft,
                circuit_breaker_threshold: None,
                fot_router: OptionalFotRouter::None,
            };
            let program_key = DataKey::Program(program_id.clone());
            env.storage().instance().set(&program_key, &program_data);

            // Record the initial transition into Draft status for this program
            Self::record_status_transition(
                &env,
                &program_id,
                &ProgramStatus::Draft,
                &ProgramStatus::Draft,
            );

            if i == 0 {
                let fee_config = FeeConfig {
                    lock_fee_rate: 0,
                    payout_fee_rate: 0,
                    lock_fixed_fee: 0,
                    payout_fixed_fee: 0,
                    fee_recipient: authorized_payout_key.clone(),
                    fee_enabled: false,
                    fee_waivers: 0,
                    insurance_reserve_bps: 0,
                };
                env.storage().instance().set(&FEE_CONFIG, &fee_config);
            }

            let multisig_config = MultisigConfig {
                threshold_amount: i128::MAX,
                signers: vec![&env],
                required_signatures: 0,
            };
            env.storage().persistent().set(
                &DataKey::MultisigConfig(program_id.clone()),
                &multisig_config,
            );

            registry.push_back(program_id.clone());
            env.events().publish(
                (PROGRAM_REGISTERED,),
                (program_id, authorized_payout_key, token_address, 0i128),
            );
        }
        env.storage().instance().set(&PROGRAM_REGISTRY, &registry);

        Ok(batch_size as u32)
    }

}
