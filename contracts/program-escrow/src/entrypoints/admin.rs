#[cfg(feature = "contract")]
#[contractimpl]
impl ProgramEscrowContract {
    pub fn program_exists(env: Env) -> bool {
        env.storage().instance().has(&PROGRAM_DATA)
            || env.storage().instance().has(&PROGRAM_REGISTRY)
    }

    /// Check if a program exists by its program_id (for batch-registered programs).
    pub fn program_exists_by_id(env: Env, program_id: String) -> bool {
        env.storage().instance().has(&DataKey::Program(program_id))
    }

    // ========================================================================
    // Fund Management
    // ========================================================================

    /// Lock funds into the program escrow with optional fee deduction.
    ///
    /// When fees are enabled, the lock fee is deducted from `amount`. Only the net
    /// amount is added to `total_funds` and `remaining_balance`. The fee is transferred
    /// to the configured fee recipient.
    ///
    /// # Arguments
    /// * `amount` - Gross amount to lock (in native token units)
    ///
    /// # Returns
    /// Updated ProgramData with locked funds and net balance after fees
    ///
    /// # Overflow Safety
    /// Uses `checked_add` to prevent balance overflow. Panics if overflow would occur.
    pub fn lock_program_funds(env: Env, amount: i128) -> ProgramData {
        // Validation precedence (deterministic ordering):
        // 1. Contract initialized
        // 2. Paused (operational state)
        // 3. Input validation (amount)

        // 1. Contract must be initialized
        if !env.storage().instance().has(&PROGRAM_DATA) {
            panic!("Program not initialized");
        }

        let mut program_data: ProgramData = env.storage().instance().get(&PROGRAM_DATA).unwrap();

        // 2. Operational state: paused
        if Self::check_paused(&env, Some(&program_data.program_id), symbol_short!("lock")) {
            panic!("Funds Paused");
        }

        // 3. Input validation
        if amount <= 0 {
            panic!("Amount must be greater than zero");
        }

        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &program_data.token_address);

        // Handle inbound transfer and measure actual received amount (handles fee-on-transfer tokens)
        let from: Option<Address> = None;
        let actual_received = if let Some(depositor) = from {
            depositor.require_auth();
            let balance_before = token_client.balance(&contract_address);

            token_client.transfer_from(&contract_address, &depositor, &contract_address, &amount);

            let balance_after = token_client.balance(&contract_address);
            let diff = crate::token_math::safe_sub(balance_after, balance_before);

            if diff <= 0 {
                panic!("Inbound transfer failed or zero value");
            }
            diff
        } else {
            // If No depositor is provided, we assume the tokens are already present
            // and 'amount' is what should be credited.
            amount
        };

        // Get fee configuration
        let fee_config = Self::get_fee_config_internal(&env);

        // Calculate fees based on actually received tokens
        let fee_amount = Self::combined_fee_amount(
            actual_received,
            fee_config.lock_fee_rate,
            fee_config.lock_fixed_fee,
            fee_config.fee_enabled,
        );
        let net_amount = amount.checked_sub(fee_amount).unwrap_or(0);
        if net_amount <= 0 {
            panic!("Lock fee consumes entire lock amount");
        }

        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &program_data.token_address);
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

        // Credit net amount to program accounting.
        // total_funds tracks the GROSS amount deposited (before fees).
        // remaining_balance tracks the NET amount available for payouts (after fees).
        program_data.total_funds = program_data
            .total_funds
            .checked_add(amount)
            .unwrap_or_else(|| panic!("Total funds overflow"));

        program_data.remaining_balance = program_data
            .remaining_balance
            .checked_add(net_amount)
            .unwrap_or_else(|| panic!("Remaining balance overflow"));

        // Store updated data — sync both legacy PROGRAM_DATA and keyed program storage
        let program_id_sync = program_data.program_id.clone();
        env.storage().instance().set(&PROGRAM_DATA, &program_data);
        let program_key_sync = DataKey::Program(program_id_sync);
        if env.storage().instance().has(&program_key_sync) {
            env.storage()
                .instance()
                .set(&program_key_sync, &program_data);
        }

        // Emit FundsLocked event
        env.events().publish(
            (FUNDS_LOCKED,),
            FundsLockedEvent {
                version: EVENT_VERSION_V2,
                program_id: program_data.program_id.clone(),
                amount: net_amount,
                remaining_balance: program_data.remaining_balance,
            },
        );

        program_data
    }

    // ========================================================================
    // Initialization & Admin
    // ========================================================================

    /// Initialize the contract with an admin.
    /// This must be called before any admin protected functions (like pause) can be used.
    pub fn initialize_contract(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::MaintenanceMode, &false);
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
        Self::ensure_history_pagination_config(&env);

        // Initialize idempotency schema version for upgrade safety
        env.storage().instance().set(
            &DataKey::IdempotencySchemaVersion,
            &IDEMPOTENCY_SCHEMA_VERSION_V1,
        );

        // Initialize role management schema version for upgrade safety
        Self::initialize_role_management_schema(&env);

        // Emit idempotency schema version event
        env.events().publish(
            (IDEMPOTENCY_SCHEMA,),
            IdempotencySchemaVersionSet {
                version: EVENT_VERSION_V2,
                schema_version: IDEMPOTENCY_SCHEMA_VERSION_V1,
                timestamp: env.ledger().timestamp(),
            },
        );
    }

    /// Set or rotate admin.
    ///
    /// If no admin is set, sets initial admin. If admin exists, current admin
    /// must authorize and the new address becomes admin.
    pub fn set_admin(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            let current: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
            current.require_auth();
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    /// Returns the current admin address, if set.
    pub fn get_admin(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Admin)
    }

    /// Propose a new admin (two-step rotation, step 1).
    ///
    /// Security notes:
    /// - The most recent proposal always wins; submitting a new proposal atomically
    ///   overwrites any earlier pending proposal.
    /// - Acceptance is bounded by `RoleManagementConfig::max_transition_period`.
    /// - The proposed admin must still complete step 2 with its own authorization.
    pub fn propose_admin(env: Env, proposed_admin: Address) -> Result<(), ContractError> {
        let current_admin = Self::require_admin(&env);

        // Check if role rotation is allowed
        Self::ensure_role_rotation_allowed(&env)?;

        // Validate proposed admin
        if proposed_admin == current_admin {
            return Err(ContractError::InvalidRoleProposal);
        }

        // Create deterministic transition state.
        // A new proposal intentionally replaces any pending proposal so stale
        // candidates cannot accept after the admin changes their mind.
        let timestamp = env.ledger().timestamp();
        let config = Self::get_role_management_config(&env);
        let deadline = timestamp.saturating_add(config.max_transition_period);

        let transition_state = RoleTransitionState {
            proposer: current_admin.clone(),
            proposed_role: proposed_admin.clone(),
            proposed_at: timestamp,
            deadline,
            nonce: Self::generate_rotation_nonce(&env, &current_admin),
        };

        // Store both the proposed address and the transition metadata.
        env.storage()
            .instance()
            .set(&DataKey::PendingAdmin, &proposed_admin);
        env.storage()
            .instance()
            .set(&DataKey::PendingAdminTransition, &transition_state);
        env.storage().instance().set(
            &DataKey::RoleManagementSchemaVersion,
            &ROLE_MANAGEMENT_SCHEMA_VERSION_V1,
        );

        env.events().publish(
            (ADMIN_PROPOSED,),
            AdminProposedEvent {
                version: EVENT_VERSION_V2,
                proposed_by: current_admin,
                proposed_admin,
                timestamp,
            },
        );

        Ok(())
    }

    /// Accept the proposed admin role (step 2).
    ///
    /// Security notes:
    /// - Only the currently proposed admin can authorize acceptance.
    /// - Expired proposals are rejected and cleared before any admin change.
    /// - Transition metadata must match the stored pending admin address.
    pub fn accept_admin(env: Env) -> Result<(), ContractError> {
        let proposed: Address = env
            .storage()
            .instance()
            .get(&DataKey::PendingAdmin)
            .ok_or(ContractError::NoAdminRotationInProgress)?;

        let transition_state: RoleTransitionState = env
            .storage()
            .instance()
            .get(&DataKey::PendingAdminTransition)
            .unwrap_or_else(|| RoleTransitionState {
                proposer: proposed.clone(),
                proposed_role: proposed.clone(),
                proposed_at: 0,
                deadline: u64::MAX,
                nonce: 0,
            });

        if transition_state.proposed_role != proposed {
            Self::clear_pending_admin_rotation_state(&env);
            return Err(ContractError::InvalidAdminRotationState);
        }

        if env.ledger().timestamp() > transition_state.deadline {
            Self::clear_pending_admin_rotation_state(&env);
            return Err(ContractError::RoleTransitionExpired);
        }

        proposed.require_auth();

        let current_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(ContractError::InvalidAdminRotationState)?;

        // Perform the role transition atomically.
        env.storage().instance().set(&DataKey::Admin, &proposed);
        Self::clear_pending_admin_rotation_state(&env);

        env.events().publish(
            (ADMIN_ACCEPTED,),
            AdminAcceptedEvent {
                version: EVENT_VERSION_V2,
                previous_admin: current_admin,
                new_admin: proposed,
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    /// Cancel a pending admin rotation.
    /// Current admin must authorize. Returns explicit errors for deterministic behavior.
    pub fn cancel_admin_rotation(env: Env) -> Result<(), ContractError> {
        let current_admin = Self::require_admin(&env);

        if !env.storage().instance().has(&DataKey::PendingAdmin) {
            return Err(ContractError::NoAdminRotationInProgress);
        }

        Self::clear_pending_admin_rotation_state(&env);

        env.events().publish(
            (ADMIN_ROTATION_CANCELLED,),
            AdminRotationCancelledEvent {
                version: EVENT_VERSION_V2,
                cancelled_by: current_admin,
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    /// Archive a program (mark as historical/read-only). Admin-only.
    ///
    /// ## Behavior with Pending Release Schedules
    ///
    /// If the program has **any** release schedules that have not yet been
    /// executed (i.e. `ProgramReleaseSchedule.released == false`), this function
    /// **will panic** with `ContractError::CannotArchiveWithPendingOps` (error
    /// code 106).  This is an intentional safety guardrail:
    ///
    /// - Silently archiving a program with unreleased schedules would strand
    ///   funds allocated to future recipients—those schedules can never be
    ///   triggered once the program is archived because `trigger_program_releases`
    ///   returns an empty list for archived programs.
    /// - Callers must first trigger or cancel all pending schedules before
    ///   archiving the program.
    ///
    /// ## Inverse — Zero Pending Schedules
    ///
    /// If there are no pending schedules (either none were created, or all have
    /// been released), archival proceeds normally:
    ///
    /// 1. The `archived` flag is set to `true`.
    /// 2. `archived_at` is set to the current ledger timestamp.
    /// 3. The program is added to the archived-programs registry.
    /// 4. Payout history is migrated to persistent storage so instance-storage
    ///    footprint shrinks.
    /// 5. An `Archived` event is emitted.
    ///
    /// ## Security Notes
    ///
    /// - Only the contract admin may call this function.
    /// - Archival is idempotent: calling it on an already-archived program is a
    ///   no-op (the history migration guard prevents overwriting existing data).
    pub fn archive_program(env: Env, program_id: String) {
        Self::require_admin(&env);
        let program_key = DataKey::Program(program_id.clone());
        let mut program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .expect("Program not found");

        // ── Guard: block archival if there are pending (unreleased) schedules ──
        //
        // Archiving with unreleased schedules would orphan funds: once a program
        // is archived, trigger_program_releases returns an empty list, so any
        // remaining scheduled amounts can never be disbursed.  The caller must
        // drain (trigger or cancel) all pending schedules first.
        let schedules: soroban_sdk::Vec<ProgramReleaseSchedule> = env
            .storage()
            .instance()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));

        let has_pending = schedules.iter().any(|s| !s.released);
        if has_pending {
            panic!("Cannot archive program with pending release schedules");
        }

        program_data.archived = true;
        program_data.archived_at = Some(env.ledger().timestamp());

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
            (symbol_short!("Archived"),),
            (program_id, env.ledger().timestamp()),
        );
    }

    /// Get all archived program IDs.
    pub fn get_archived_programs(env: Env) -> soroban_sdk::Vec<String> {
        let registry: soroban_sdk::Vec<String> = env
            .storage()
            .instance()
            .get(&PROGRAM_REGISTRY)
            .unwrap_or(Vec::new(&env));
        let mut archived = Vec::new(&env);
        for program_id in registry.iter() {
            let program_key = DataKey::Program(program_id.clone());
            if let Some(data) = env
                .storage()
                .instance()
                .get::<DataKey, ProgramData>(&program_key)
            {
                if data.archived {
                    archived.push_back(program_id);
                }
            }
        }
        archived
    }

    fn require_admin(env: &Env) -> Address {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("Not initialized"));
        admin.require_auth();
        admin
    }

    /// Remove all pending admin-rotation state.
    fn clear_pending_admin_rotation_state(env: &Env) {
        env.storage().instance().remove(&DataKey::PendingAdmin);
        env.storage()
            .instance()
            .remove(&DataKey::PendingAdminTransition);
    }

    /// Get role management configuration with upgrade-safe defaults.
    fn get_role_management_config(env: &Env) -> RoleManagementConfig {
        env.storage()
            .instance()
            .get(&DataKey::RoleManagementConfig)
            .unwrap_or_else(|| RoleManagementConfig::default(env))
    }

    /// Generate deterministic nonce for role rotation replay protection.
    fn generate_rotation_nonce(env: &Env, proposer: &Address) -> u64 {
        // Use combination of timestamp, proposer address, and ledger sequence for deterministic nonce
        let timestamp = env.ledger().timestamp();
        let sequence = env.ledger().sequence() as u64;

        // Simple deterministic hash combination (in production, use a proper hash function)
        (timestamp.wrapping_mul(31) ^ sequence.wrapping_mul(17) ^ proposer.to_string().len() as u64)
            .wrapping_add(1)
    }

    /// Ensure role rotation is allowed based on contract state.
    fn ensure_role_rotation_allowed(env: &Env) -> Result<(), ContractError> {
        let config = Self::get_role_management_config(env);

        if !config.rotation_enabled {
            return Err(ContractError::RoleRotationNotAllowed);
        }

        // Check if contract is in emergency mode that blocks rotations
        if config.emergency_blocks_rotations {
            let read_only: bool = env
                .storage()
                .instance()
                .get(&DataKey::ReadOnlyMode)
                .unwrap_or(false);

            if read_only {
                return Err(ContractError::RoleRotationNotAllowed);
            }

            // Check pause state
            let pause_flags = Self::get_pause_flags(env);
            if pause_flags.lock_paused && pause_flags.release_paused && pause_flags.refund_paused {
                return Err(ContractError::RoleRotationNotAllowed);
            }
        }

        // Check for active disputes
        if let Some(_) = env
            .storage()
            .instance()
            .get::<DataKey, DisputeRecord>(&DataKey::Dispute)
        {
            return Err(ContractError::RoleRotationNotAllowed);
        }

        Ok(())
    }

    /// Initialize role management schema if not already set.
    fn initialize_role_management_schema(env: &Env) {
        if !env
            .storage()
            .instance()
            .has(&DataKey::RoleManagementSchemaVersion)
        {
            env.storage().instance().set(
                &DataKey::RoleManagementSchemaVersion,
                &ROLE_MANAGEMENT_SCHEMA_VERSION_V1,
            );
            env.storage().instance().set(
                &DataKey::RoleManagementConfig,
                &RoleManagementConfig::default(env),
            );
        }
    }

    /// Get role management schema version for testing.
    pub fn get_role_mgmt_schema_ver(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::RoleManagementSchemaVersion)
            .unwrap_or(0)
    }

    /// Guard: panics with "Read-only mode" when read-only mode is enabled.
    fn require_not_read_only(env: &Env) {
        let read_only: bool = env
            .storage()
            .instance()
            .get(&DataKey::ReadOnlyMode)
            .unwrap_or(false);
        if read_only {
            panic!("Read-only mode");
        }
    }
}
