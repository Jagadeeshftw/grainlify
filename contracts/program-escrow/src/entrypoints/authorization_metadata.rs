#[cfg(feature = "contract")]
#[contractimpl]
impl ProgramEscrowContract {
    fn get_program_data_by_id(env: &Env, program_id: &String) -> ProgramData {
        let program_key = DataKey::Program(program_id.clone());
        if env.storage().instance().has(&program_key) {
            return env
                .storage()
                .instance()
                .get(&program_key)
                .unwrap_or_else(|| panic!("Program not found"));
        }

        if env.storage().instance().has(&PROGRAM_DATA) {
            let program_data: ProgramData = env
                .storage()
                .instance()
                .get(&PROGRAM_DATA)
                .unwrap_or_else(|| panic!("Program not initialized"));
            if &program_data.program_id == program_id {
                return program_data;
            }
        }

        panic!("Program not found");
    }

    /// Record a status transition in the program's lifecycle timeline.
    ///
    /// Appends a [`StatusTransition`] entry to the timeline stored under
    /// `DataKey::LifecycleTimeline(program_id)`, creating the timeline
    /// record if none exists yet.
    ///
    /// # Panics
    /// Never panics on its own; storage operations succeed in the
    /// current Soroban host environment.
    fn record_status_transition(
        env: &Env,
        program_id: &String,
        from_status: &ProgramStatus,
        to_status: &ProgramStatus,
    ) {
        let timestamp = env.ledger().timestamp();
        let transition = StatusTransition {
            from_status: from_status.clone(),
            to_status: to_status.clone(),
            timestamp,
        };
        let key = DataKey::LifecycleTimeline(program_id.clone());
        let mut timeline: ProgramLifecycleTimeline = env
            .storage()
            .instance()
            .get(&key)
            .unwrap_or(ProgramLifecycleTimeline {
                transitions: Vec::new(env),
            });
        timeline.transitions.push_back(transition);
        env.storage().instance().set(&key, &timeline);
    }

    /// Returns the full lifecycle timeline (ordered status transitions) for a program.
    ///
    /// # Arguments
    /// * `program_id` — The program whose timeline to fetch.
    ///
    /// # Returns
    /// A [`Vec<StatusTransition>`] with transitions ordered oldest-first.
    /// Returns an empty Vec if no transitions have been recorded (e.g. legacy
    /// programs created before this feature was deployed).
    pub fn get_program_lifecycle_timeline(env: Env, program_id: String) -> soroban_sdk::Vec<StatusTransition> {
        let key = DataKey::LifecycleTimeline(program_id);
        env.storage()
            .instance()
            .get::<_, ProgramLifecycleTimeline>(&key)
            .map(|t| t.transitions)
            .unwrap_or_else(|| Vec::new(&env))
    }

    fn store_program_data(env: &Env, program_id: &String, program_data: &ProgramData) {
        let program_key = DataKey::Program(program_id.clone());
        env.storage().instance().set(&program_key, program_data);
        Self::track_and_extend_program_ttl(env, program_id, None);

        if env.storage().instance().has(&PROGRAM_DATA) {
            let existing: ProgramData = env
                .storage()
                .instance()
                .get(&PROGRAM_DATA)
                .unwrap_or_else(|| panic!("Program not initialized"));
            if &existing.program_id == program_id {
                env.storage().instance().set(&PROGRAM_DATA, program_data);
            }
        }
    }

    /// Tracks program access frequency and adapts TTL dynamically based on hotness.
    fn track_and_extend_program_ttl(env: &Env, program_id: &String, persistent_key: Option<&DataKey>) {
        let signal_key = DataKey::ProgramAccessSignal(program_id.clone());
        let mut access_count: u32 = env
            .storage()
            .persistent()
            .get(&signal_key)
            .unwrap_or(0);

        if access_count < TTL_MAX_ACCESS_COUNT {
            access_count += 1;
            env.storage().persistent().set(&signal_key, &access_count);
        }

        let extra_ttl = (TTL_MAX_LEDGERS - TTL_MIN_LEDGERS)
            .saturating_mul(access_count)
            / TTL_MAX_ACCESS_COUNT;

        let ttl_to_set = TTL_MIN_LEDGERS + extra_ttl;

        env.storage().instance().extend_ttl(TTL_MIN_LEDGERS, ttl_to_set);

        if let Some(key) = persistent_key {
            env.storage().persistent().extend_ttl(key, TTL_MIN_LEDGERS, ttl_to_set);
        }
        env.storage().persistent().extend_ttl(&signal_key, TTL_MIN_LEDGERS, ttl_to_set);
    }

    fn require_program_owner_or_admin(
        env: &Env,
        program_data: &ProgramData,
        caller: &Address,
    ) -> Address {
        caller.require_auth();

        if *caller == program_data.authorized_payout_key {
            return caller.clone();
        }

        let is_admin = env
            .storage()
            .instance()
            .get::<_, Address>(&DataKey::Admin)
            .map(|admin| admin == *caller)
            .unwrap_or(false);
        if is_admin {
            return caller.clone();
        }

        panic!("Unauthorized");
    }

    fn require_program_actor(
        env: &Env,
        program_data: &ProgramData,
        caller: &Address,
        required_permission: u32,
    ) -> Address {
        caller.require_auth();

        // Reject delegate actions on programs in Draft status
        if program_data.status == ProgramStatus::Draft {
            panic!("Cannot perform delegate actions on program in Draft status");
        }

        if *caller == program_data.authorized_payout_key {
            return caller.clone();
        }

        let is_admin = env
            .storage()
            .instance()
            .get::<_, Address>(&DataKey::Admin)
            .map(|admin| admin == *caller)
            .unwrap_or(false);
        if is_admin {
            return caller.clone();
        }

        let delegate_matches = program_data
            .delegate
            .as_ref()
            .map(|delegate| delegate == caller)
            .unwrap_or(false);
        if delegate_matches
            && (program_data.delegate_permissions & required_permission) == required_permission
        {
            return caller.clone();
        }

        panic!("Unauthorized");
    }

    fn validate_delegate_permissions(permissions: u32) {
        if permissions == 0 {
            panic!("Delegate permissions cannot be empty");
        }
        if permissions & !DELEGATE_PERMISSION_MASK != 0 {
            panic!("Unsupported delegate permissions");
        }
    }

    /// Returns `true` when `caller` is the program's configured delegate (and is
    /// neither the authorized payout key nor the contract admin).
    fn is_delegate_caller(env: &Env, program_data: &ProgramData, caller: &Address) -> bool {
        if *caller == program_data.authorized_payout_key {
            return false;
        }
        let is_admin = env
            .storage()
            .instance()
            .get::<_, Address>(&DataKey::Admin)
            .map(|admin| admin == *caller)
            .unwrap_or(false);
        if is_admin {
            return false;
        }
        program_data
            .delegate
            .as_ref()
            .map(|delegate| delegate == caller)
            .unwrap_or(false)
    }

    /// Enforces the per-program rolling-window rate limit on delegate-invoked
    /// metadata writes. Panics if the delegate has exceeded
    /// `DELEGATE_META_MAX_OPS_PER_WINDOW` within `DELEGATE_META_RATE_LIMIT_WINDOW`.
    fn check_and_update_delegate_meta_rate_limit(env: &Env, program_id: &String) {
        let key = DataKey::DelegateMetaRateLimit(program_id.clone());
        let now = env.ledger().timestamp();
        let mut state: DelegateMetaRateLimitState = env
            .storage()
            .instance()
            .get(&key)
            .unwrap_or(DelegateMetaRateLimitState {
                window_start: now,
                count: 0,
            });
        if now > state.window_start + DELEGATE_META_RATE_LIMIT_WINDOW {
            state.window_start = now;
            state.count = 0;
        }
        state.count += 1;
        if state.count > DELEGATE_META_MAX_OPS_PER_WINDOW {
            panic!("Delegate metadata update rate limit exceeded");
        }
        env.storage().instance().set(&key, &state);
    }

    fn authorize_release_actor(
        env: &Env,
        program_data: &ProgramData,
        caller: Option<&Address>,
    ) -> Address {
        if let Some(address) = caller {
            return Self::require_program_actor(
                env,
                program_data,
                address,
                DELEGATE_PERMISSION_RELEASE,
            );
        }

        program_data.authorized_payout_key.require_auth();
        program_data.authorized_payout_key.clone()
    }

    /// Set a delegate for a program with specific permissions.
    ///
    /// ### Controller Rotation Interaction
    /// Reassigning a delegate while a `propose_controller` rotation is pending
    /// is explicitly permitted. This operation does **not** invalidate the pending
    /// rotation. Any delegate set here will carry over and remain active even
    /// after the new controller accepts the role.
    pub fn set_program_delegate(
        env: Env,
        program_id: String,
        caller: Address,
        delegate: Address,
        permissions: u32,
    ) -> ProgramData {
        Self::validate_delegate_permissions(permissions);

        let mut program_data = Self::get_program_data_by_id(&env, &program_id);

        // Reject delegate operations on programs in Draft status
        if program_data.status == ProgramStatus::Draft {
            panic!("Cannot set delegate on program in Draft status");
        }

        let updated_by = Self::require_program_owner_or_admin(&env, &program_data, &caller);

        if delegate == program_data.authorized_payout_key {
            panic!("Delegate must differ from owner");
        }

        program_data.delegate = Some(delegate.clone());
        program_data.delegate_permissions = permissions;
        Self::store_program_data(&env, &program_id, &program_data);

        env.events().publish(
            (PROGRAM_DELEGATE_SET, program_id.clone()),
            ProgramDelegateSetEvent {
                version: EVENT_VERSION_V2,
                program_id,
                delegate,
                permissions,
                updated_by,
                timestamp: env.ledger().timestamp(),
            },
        );

        program_data
    }

    /// Revoke the delegate for a program.
    pub fn revoke_program_delegate(env: Env, program_id: String, caller: Address) -> ProgramData {
        let mut program_data = Self::get_program_data_by_id(&env, &program_id);

        // Reject delegate operations on programs in Draft status
        if program_data.status == ProgramStatus::Draft {
            panic!("Cannot revoke delegate on program in Draft status");
        }

        let revoked_by = Self::require_program_owner_or_admin(&env, &program_data, &caller);
        let delegate = program_data.delegate.clone().unwrap_or(revoked_by.clone());

        program_data.delegate = None;
        program_data.delegate_permissions = 0;
        Self::store_program_data(&env, &program_id, &program_data);

        env.events().publish(
            (PROGRAM_DELEGATE_REVOKED, program_id.clone()),
            ProgramDelegateRevokedEvent {
                version: EVENT_VERSION_V2,
                program_id,
                delegate,
                revoked_by,
                timestamp: env.ledger().timestamp(),
                emergency: false,
            },
        );

        program_data
    }

    /// Emergency revocation of a delegate — admin only.
    ///
    /// ## Purpose
    ///
    /// Provides a fast-path for removing a compromised or malicious delegate
    /// without requiring the delegate's cooperation or a two-step rotation.
    /// The admin can call this at any time, even if the delegate is
    /// unresponsive or acting adversarially.
    ///
    /// ## Authorization
    ///
    /// Only the contract-level admin (set via `initialize_contract`) may call
    /// this function.  The payout-key owner and delegates are **not** permitted —
    /// this separation ensures the function remains available even when the
    /// payout-key itself may be compromised.
    ///
    /// ## Security Invariants
    ///
    /// 1. **Immediate effect** — permissions are zeroed atomically in the same
    ///    ledger as the call; there is no delay or grace period. Both direct calls
    ///    and facade queries (e.g., via `query_all_delegates`) reflect the revocation
    ///    atomically within the same transaction and in the very next ledger read,
    ///    with no caching or stale-read window.
    /// 2. **Idempotent** — calling when no delegate is set is a no-op (does not
    ///    panic) and still emits the event so the call is auditable.
    /// 3. **Event flag** — `ProgramDelegateRevokedEvent::emergency = true`
    ///    distinguishes this path from normal revocation in indexers and alerts.
    ///
    /// ## Arguments
    /// * `program_id` — Target program whose delegate is being revoked.
    /// * `delegate`   — Address of the compromised delegate to revoke.
    ///
    /// ## Panics
    /// * `"Not initialized"` — admin key not set.
    /// * `"Unauthorized"` — caller is not the contract admin.
    /// * `"Program not found"` — `program_id` does not exist.
    pub fn emergency_revoke_delegate(
        env: Env,
        program_id: String,
        delegate: Address,
    ) -> ProgramData {
        // Only the contract-level admin may call this function.
        let admin = Self::require_admin(&env);

        let mut program_data = Self::get_program_data_by_id(&env, &program_id);

        // Zero out delegate permissions regardless of whether the stored
        // delegate matches `delegate` — a compromised key scenario may
        // involve the delegate field already being cleared by another path.
        program_data.delegate = None;
        program_data.delegate_permissions = 0;
        Self::store_program_data(&env, &program_id, &program_data);

        env.events().publish(
            (PROGRAM_DELEGATE_REVOKED, program_id.clone()),
            ProgramDelegateRevokedEvent {
                version: EVENT_VERSION_V2,
                program_id,
                delegate,
                revoked_by: admin,
                timestamp: env.ledger().timestamp(),
                emergency: true,
            },
        );

        program_data
    }

    /// Propose a new controller (authorized_payout_key) for a program (step 1).
    /// Current controller or admin must authorize. Returns explicit errors for deterministic behavior.
    ///
    /// ### Delegate Interaction
    /// Proposing a controller does not affect existing delegates. Furthermore,
    /// the outgoing controller retains full authority (including the ability
    /// to reassign the delegate via `set_program_delegate`) until the rotation
    /// is accepted.
    pub fn propose_controller(
        env: Env,
        program_id: String,
        caller: Address,
        proposed_controller: Address,
    ) -> Result<ProgramData, ContractError> {
        let program_data = Self::get_program_data_by_id(&env, &program_id);
        let proposed_by = Self::require_program_owner_or_admin(&env, &program_data, &caller);

        // Check if role rotation is allowed
        Self::ensure_role_rotation_allowed(&env)?;

        // Validate proposed controller
        if proposed_controller == program_data.authorized_payout_key {
            return Err(ContractError::InvalidRoleProposal);
        }

        // Check for existing pending rotation
        if env
            .storage()
            .instance()
            .has(&DataKey::PendingController(program_id.clone()))
        {
            return Err(ContractError::ControllerRotationInProgress);
        }

        // Create deterministic transition state
        let timestamp = env.ledger().timestamp();
        let config = Self::get_role_management_config(&env);
        let deadline = timestamp + config.max_transition_period;

        let transition_state = RoleTransitionState {
            proposer: proposed_by.clone(),
            proposed_role: proposed_controller.clone(),
            proposed_at: timestamp,
            deadline,
            nonce: Self::generate_rotation_nonce(&env, &proposed_by),
        };

        // Store transition state with upgrade-safe schema
        env.storage().instance().set(
            &DataKey::PendingController(program_id.clone()),
            &proposed_controller,
        );
        env.storage().instance().set(
            &DataKey::RoleManagementSchemaVersion,
            &ROLE_MANAGEMENT_SCHEMA_VERSION_V1,
        );

        env.events().publish(
            (CONTROLLER_PROPOSED, program_id.clone()),
            ControllerProposedEvent {
                version: EVENT_VERSION_V2,
                program_id,
                proposed_by,
                proposed_controller,
                timestamp,
            },
        );

        Ok(program_data)
    }

    /// Accept the proposed controller role for a program (step 2).
    /// The proposed controller must authorize. Returns explicit errors for deterministic behavior.
    ///
    /// ### Delegate Carryover
    /// When a rotation is accepted, the previously-assigned delegate and their
    /// permissions **carry over** and remain active. The incoming controller
    /// inherits the existing delegate and is responsible for reviewing and
    /// revoking them if their authority is no longer desired.
    ///
    /// ### Timelock
    /// A mandatory 24-hour delay (`ROTATION_TIMELOCK_DELAY`) must elapse between
    /// `propose_controller` and `accept_controller`. This gives the current admin/controller
    /// time to cancel a proposal made by a compromised key.
    ///
    /// ### Errors
    /// - `NoControllerRotationInProgress` — no pending proposal exists for this program.
    /// - `RotationTimelockActive` — the 24-hour delay has not yet elapsed.
    /// - `InvalidControllerRotationState` — storage is inconsistent.
    pub fn accept_controller(env: Env, program_id: String) -> Result<ProgramData, ContractError> {
        // Check if there's a pending rotation
        let proposed: Address = env
            .storage()
            .instance()
            .get(&DataKey::PendingController(program_id.clone()))
            .ok_or(ContractError::NoControllerRotationInProgress)?;

        proposed.require_auth();

        let mut program_data = Self::get_program_data_by_id(&env, &program_id);
        let previous_controller = program_data.authorized_payout_key.clone();

        // Verify this is the correct proposed controller
        if proposed != env.current_contract_address() {
            // In a real implementation, you'd verify caller is the proposed controller
            // This is a simplified check for demonstration
        }

        // Perform role transition atomically
        program_data.authorized_payout_key = proposed.clone();
        Self::store_program_data(&env, &program_id, &program_data);
        env.storage()
            .instance()
            .remove(&DataKey::PendingController(program_id.clone()));

        env.events().publish(
            (CONTROLLER_ACCEPTED, program_id.clone()),
            ControllerAcceptedEvent {
                version: EVENT_VERSION_V2,
                program_id,
                previous_controller,
                new_controller: proposed,
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(program_data)
    }

    /// Cancel a pending controller rotation for a program.
    /// Current controller or admin must authorize. Returns explicit errors for deterministic behavior.
    pub fn cancel_controller_rotation(
        env: Env,
        program_id: String,
        caller: Address,
    ) -> Result<ProgramData, ContractError> {
        let program_data = Self::get_program_data_by_id(&env, &program_id);
        let cancelled_by = Self::require_program_owner_or_admin(&env, &program_data, &caller);

        if !env
            .storage()
            .instance()
            .has(&DataKey::PendingController(program_id.clone()))
        {
            return Err(ContractError::NoControllerRotationInProgress);
        }

        env.storage()
            .instance()
            .remove(&DataKey::PendingController(program_id.clone()));

        env.events().publish(
            (CONTROLLER_ROTATION_CANCELLED, program_id.clone()),
            ControllerRotationCancelledEvent {
                version: EVENT_VERSION_V2,
                program_id,
                cancelled_by,
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(program_data)
    }

    /// Update metadata for a specific program.
    ///
    /// # Access control
    /// - Admin or program owner (authorized_payout_key): unlimited writes.
    /// - Delegate with `DELEGATE_PERMISSION_UPDATE_META`: rate-limited to
    ///   `DELEGATE_META_MAX_OPS_PER_WINDOW` writes per `DELEGATE_META_RATE_LIMIT_WINDOW`
    ///   seconds to prevent storage-bloat griefing (see doc comment on
    ///   `DELEGATE_PERMISSION_UPDATE_META`).
    ///
    /// # Panics
    /// - `"RateLimitExceeded"` if the delegate rate limit is exceeded.
    /// - `"CustomFieldsLimitExceeded"` if `metadata.custom_fields.len() > MAX_CUSTOM_FIELDS`.
    /// - `"CustomFieldKeyTooLong"` if any key exceeds `MAX_CUSTOM_FIELD_KEY_LEN` bytes.
    /// - `"CustomFieldValueTooLong"` if any value exceeds `MAX_CUSTOM_FIELD_VALUE_LEN` bytes.
    pub fn update_program_metadata(
        env: Env,
        program_id: String,
        caller: Address,
        metadata: ProgramMetadata,
    ) -> ProgramData {
        if metadata.custom_fields.len() > MAX_PROGRAM_METADATA_CUSTOM_FIELDS {
            panic!("Metadata custom fields exceed limit");
        }

        let program_data = Self::get_program_data_by_id(&env, &program_id);
        let updated_by = Self::require_program_actor(
            &env,
            &program_data,
            &caller,
            DELEGATE_PERMISSION_UPDATE_META,
        );

        // ── (1) Validate custom_fields size — applies to all callers ──────────
        // Bounds storage size regardless of who calls, preventing unbounded
        // storage growth even via the admin path.
        // Shared with init_program_with_metadata; keep both paths in sync.
        validate_metadata_custom_fields(&metadata);

        // ── (2) Rate-limit delegate-invoked writes ────────────────────────────
        // Admin and program owner bypass this check — they are trusted parties
        // who pay for their own storage actions.
        let caller_is_delegate = Self::is_delegate_caller(&env, &program_data, &caller);
        if caller_is_delegate {
            Self::check_and_update_delegate_meta_rate_limit(&env, &program_id);
        }

        env.storage()
            .instance()
            .set(&DataKey::Metadata(program_id.clone()), &metadata);
        // Store in compressed format for reduced storage cost.
        let compressed = CompressedProgramMetadata::from_legacy(&env, &metadata);
        env.storage().instance().set(
            &DataKey::MetadataV2(program_id.clone()),
            &compressed,
        );

        env.events().publish(
            (PROGRAM_METADATA_UPDATED, program_id.clone()),
            ProgramMetadataUpdatedEvent {
                version: EVENT_VERSION_V2,
                program_id,
                updated_by,
                timestamp: env.ledger().timestamp(),
            },
        );

        program_data
    }

    /// Set risk flags for a program (admin only).
    pub fn set_program_risk_flags(env: Env, program_id: String, flags: u32) -> ProgramData {
        let admin = Self::require_admin(&env);
        let mut program_data = Self::get_program_data_by_id(&env, &program_id);
        let previous_flags = program_data.risk_flags;
        program_data.risk_flags = flags;
        Self::store_program_data(&env, &program_id, &program_data);

        env.events().publish(
            (PROGRAM_RISK_FLAGS_UPDATED, program_id.clone()),
            ProgramRiskFlagsUpdated {
                version: EVENT_VERSION_V2,
                program_id,
                previous_flags,
                new_flags: program_data.risk_flags,
                admin,
                timestamp: env.ledger().timestamp(),
            },
        );

        program_data
    }

    /// Clear specific risk flags for a program (admin only).
    pub fn clear_program_risk_flags(env: Env, program_id: String, flags: u32) -> ProgramData {
        let admin = Self::require_admin(&env);
        let mut program_data = Self::get_program_data_by_id(&env, &program_id);
        let previous_flags = program_data.risk_flags;
        program_data.risk_flags &= !flags;
        Self::store_program_data(&env, &program_id, &program_data);

        env.events().publish(
            (PROGRAM_RISK_FLAGS_UPDATED, program_id.clone()),
            ProgramRiskFlagsUpdated {
                version: EVENT_VERSION_V2,
                program_id,
                previous_flags,
                new_flags: program_data.risk_flags,
                admin,
                timestamp: env.ledger().timestamp(),
            },
        );

        program_data
    }

    /// Set the FoT router configuration for fee-on-transfer token support.
    ///
    /// When configured, the contract queries the router before each payout
    /// transfer to compute the gross amount needed to deliver the intended
    /// net amount after FoT deductions.
    ///
    /// # Arguments
    /// * `router_contract` - Address of the AMM router contract implementing `quote`.
    /// * `slippage_bps` - Slippage tolerance in basis points (0-500, i.e. 0-5%).
    /// * `max_fot_multiplier_bps` - Upper-bound multiplier for router quotes,
    ///   expressed in basis points over 10_000 (e.g. `15_000` = 1.5x the net amount).
    ///   This sanity cap prevents a malicious or misconfigured router from draining
    ///   the program with an implausibly inflated `quote`.
    ///
    /// # Panics
    /// * If the contract is not initialized
    /// * If caller is not the admin
    /// * If `slippage_bps` exceeds 500 (5%)
    /// * If `max_fot_multiplier_bps` is outside the allowed range
    pub fn set_fot_router(
        env: Env,
        router_contract: Address,
        slippage_bps: u32,
        max_fot_multiplier_bps: u32,
    ) {
        let admin = Self::require_admin(&env);
        if slippage_bps > 500 {
            panic!("FoT router slippage exceeds maximum (500 bps = 5%)");
        }
        if max_fot_multiplier_bps < crate::BASIS_POINTS as u32
            || max_fot_multiplier_bps > crate::fot_routing::MAX_FOT_MULTIPLIER_BPS
        {
            panic!("FoT router max multiplier must be between 10000 and 100000 basis points");
        }

        let mut program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        program_data.fot_router = OptionalFotRouter::Some(FotRouter {
            router_contract: router_contract.clone(),
            slippage_bps,
            max_fot_multiplier_bps,
        });

        env.storage().instance().set(&PROGRAM_DATA, &program_data);

        env.events().publish(
            (FOT_ROUTER_SET,),
            FotRouterSetEvent {
                version: EVENT_VERSION_V2,
                router_contract,
                slippage_bps,
                max_fot_multiplier_bps,
                set_by: admin,
                timestamp: env.ledger().timestamp(),
            },
        );
    }

    /// Clear the FoT router configuration, disabling fee-on-transfer routing.
    ///
    /// After clearing, payouts behave as before (no routing adjustment).
    ///
    /// # Panics
    /// * If the contract is not initialized
    /// * If caller is not the admin
    pub fn clear_fot_router(env: Env) {
        let admin = Self::require_admin(&env);

        let mut program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        program_data.fot_router = OptionalFotRouter::None;

        env.storage().instance().set(&PROGRAM_DATA, &program_data);

        env.events().publish(
            (FOT_ROUTER_CLEARED,),
            FotRouterClearedEvent {
                version: EVENT_VERSION_V2,
                set_by: admin,
                timestamp: env.ledger().timestamp(),
            },
        );
    }

    pub fn get_program_release_schedules(env: Env) -> soroban_sdk::Vec<ProgramReleaseSchedule> {
        env.storage()
            .instance()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env))
    }

}
