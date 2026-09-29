#[cfg(feature = "contract")]
#[contractimpl]
impl ProgramEscrowContract {
    pub fn set_paused(
        env: Env,
        lock: Option<bool>,
        release: Option<bool>,
        refund: Option<bool>,
        reason: Option<String>,
        unpause_at: Option<u64>,
    ) {
        if !env.storage().instance().has(&DataKey::Admin) {
            panic!("Not initialized");
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        // Enforce 256-character bound on reason to prevent storage abuse.
        if let Some(ref r) = reason {
            if r.len() > PAUSE_REASON_MAX_LEN {
                panic!("Pause reason exceeds maximum length of 256 characters");
            }
        }

        let mut flags = Self::get_pause_flags(&env);
        let timestamp = env.ledger().timestamp();

        if reason.is_some() {
            flags.pause_reason = reason.clone();
        }

        if let Some(paused) = lock {
            let previous_paused = flags.lock_paused;
            flags.lock_paused = paused;
            // Store or clear TTL for this mode.
            flags.lock_unpause_at = if paused { unpause_at } else { None };
            let receipt_id = Self::increment_receipt_id(&env);
            env.events().publish(
                (PAUSE_STATE_CHANGED,),
                PauseStateChanged {
                    operation: symbol_short!("lock"),
                    paused,
                    admin: admin.clone(),
                    reason: reason.clone(),
                    timestamp,
                    receipt_id,
                },
            );
            env.events().publish(
                (PAUSE_STATE_CHANGED_V2, symbol_short!("lock")),
                PauseStateChangedV2 {
                    version: EVENT_VERSION_V2,
                    operation: symbol_short!("lock"),
                    previous_paused,
                    paused,
                    actor: admin.clone(),
                    reason: reason.clone(),
                    timestamp,
                    receipt_id,
                    schema_version: PAUSE_SCHEMA_VERSION_V1,
                },
            );
        }

        if let Some(paused) = release {
            let previous_paused = flags.release_paused;
            flags.release_paused = paused;
            flags.release_unpause_at = if paused { unpause_at } else { None };
            let receipt_id = Self::increment_receipt_id(&env);
            env.events().publish(
                (PAUSE_STATE_CHANGED,),
                PauseStateChanged {
                    operation: symbol_short!("release"),
                    paused,
                    admin: admin.clone(),
                    reason: reason.clone(),
                    timestamp,
                    receipt_id,
                },
            );
            env.events().publish(
                (PAUSE_STATE_CHANGED_V2, symbol_short!("release")),
                PauseStateChangedV2 {
                    version: EVENT_VERSION_V2,
                    operation: symbol_short!("release"),
                    previous_paused,
                    paused,
                    actor: admin.clone(),
                    reason: reason.clone(),
                    timestamp,
                    receipt_id,
                    schema_version: PAUSE_SCHEMA_VERSION_V1,
                },
            );
        }

        if let Some(paused) = refund {
            let previous_paused = flags.refund_paused;
            flags.refund_paused = paused;
            flags.refund_unpause_at = if paused { unpause_at } else { None };
            let receipt_id = Self::increment_receipt_id(&env);
            env.events().publish(
                (PAUSE_STATE_CHANGED,),
                PauseStateChanged {
                    operation: symbol_short!("refund"),
                    paused,
                    admin: admin.clone(),
                    reason: reason.clone(),
                    timestamp,
                    receipt_id,
                },
            );
            env.events().publish(
                (PAUSE_STATE_CHANGED_V2, symbol_short!("refund")),
                PauseStateChangedV2 {
                    version: EVENT_VERSION_V2,
                    operation: symbol_short!("refund"),
                    previous_paused,
                    paused,
                    actor: admin.clone(),
                    reason: reason.clone(),
                    timestamp,
                    receipt_id,
                    schema_version: PAUSE_SCHEMA_VERSION_V1,
                },
            );
        }

        let any_paused = flags.lock_paused || flags.release_paused || flags.refund_paused;

        if any_paused {
            if flags.paused_at == 0 {
                flags.paused_at = timestamp;
            }
        } else {
            flags.pause_reason = None;
            flags.paused_at = 0;
        }

        env.storage().instance().set(&DataKey::PauseFlags, &flags);
    }

    pub fn set_program_paused(
        env: Env,
        program_id: String,
        lock: Option<bool>,
        release: Option<bool>,
        refund: Option<bool>,
        reason: Option<String>,
        unpause_at: Option<u64>,
    ) {
        if !env.storage().instance().has(&DataKey::Admin) {
            panic!("Not initialized");
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if let Some(ref r) = reason {
            if r.len() > PAUSE_REASON_MAX_LEN {
                panic!("Pause reason exceeds maximum length of 256 characters");
            }
        }

        let mut flags = Self::get_program_pause_flags(&env, program_id.clone());
        let timestamp = env.ledger().timestamp();

        if reason.is_some() {
            flags.pause_reason = reason.clone();
        }

        if let Some(paused) = lock {
            flags.lock_paused = paused;
            flags.lock_unpause_at = if paused { unpause_at } else { None };
        }

        if let Some(paused) = release {
            flags.release_paused = paused;
            flags.release_unpause_at = if paused { unpause_at } else { None };
        }

        if let Some(paused) = refund {
            flags.refund_paused = paused;
            flags.refund_unpause_at = if paused { unpause_at } else { None };
        }

        let any_paused = flags.lock_paused || flags.release_paused || flags.refund_paused;

        if any_paused {
            if flags.paused_at == 0 {
                flags.paused_at = timestamp;
            }
        } else {
            flags.pause_reason = None;
            flags.paused_at = 0;
        }

        env.storage().instance().set(&DataKey::ProgramPauseFlags(program_id), &flags);
    }

    /// Check if the contract is in maintenance mode
    pub fn is_maintenance_mode(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::MaintenanceMode)
            .unwrap_or(false)
    }

    fn require_not_maintenance_mode(env: &Env) {
        let in_maintenance: bool = env
            .storage()
            .instance()
            .get(&DataKey::MaintenanceMode)
            .unwrap_or(false);
        if in_maintenance {
            panic!("Contract is in read-only maintenance mode");
        }
    }

    /// Update maintenance mode (admin only).
    pub fn set_maintenance_mode(env: Env, enabled: bool) {
        if !env.storage().instance().has(&DataKey::Admin) {
            panic!("Not initialized");
        }
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        env.storage()
            .instance()
            .set(&DataKey::MaintenanceMode, &enabled);
        env.events().publish(
            (MAINTENANCE_MODE_CHANGED,),
            MaintenanceModeChanged {
                enabled,
                admin: admin.clone(),
                timestamp: env.ledger().timestamp(),
            },
        );
    }

    /// Emergency withdraw all program funds (admin only, must have lock_paused = true).
    pub fn emergency_withdraw(env: Env, target: Address) {
        if !env.storage().instance().has(&DataKey::Admin) {
            panic!("Not initialized");
        }
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let flags = Self::get_pause_flags(&env);
        if !flags.lock_paused {
            panic!("Not paused");
        }

        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));
        let token_client = token::TokenClient::new(&env, &program_data.token_address);

        let contract_address = env.current_contract_address();
        let balance = token_client.balance(&contract_address);

        if balance > 0 {
            token_client.transfer(&contract_address, &target, &balance);
            let receipt_id = Self::increment_receipt_id(&env);
            env.events().publish(
                (symbol_short!("em_wtd"),),
                EmergencyWithdrawEvent {
                    admin,
                    target: target.clone(),
                    amount: balance,
                    timestamp: env.ledger().timestamp(),
                    receipt_id,
                },
            );
        }
    }

    /// Get current pause flags
    pub fn get_pause_flags(env: &Env) -> PauseFlags {
        env.storage()
            .instance()
            .get(&DataKey::PauseFlags)
            .unwrap_or(PauseFlags {
                lock_paused: false,
                release_paused: false,
                refund_paused: false,
                pause_reason: None,
                paused_at: 0,
                lock_unpause_at: None,
                release_unpause_at: None,
                refund_unpause_at: None,
            })
    }

    pub fn get_program_pause_flags(env: &Env, program_id: String) -> PauseFlags {
        env.storage()
            .instance()
            .get(&DataKey::ProgramPauseFlags(program_id.clone()))
            .unwrap_or(PauseFlags {
                lock_paused: false,
                release_paused: false,
                refund_paused: false,
                pause_reason: None,
                paused_at: 0,
                lock_unpause_at: None,
                release_unpause_at: None,
                refund_unpause_at: None,
            })
    }

    /// Returns the stored pause flags schema version.
    ///
    /// Returns `PAUSE_SCHEMA_VERSION_V1` (1) for contracts initialized after
    /// this upgrade. Returns `0` for legacy contracts that predate the schema
    /// version marker — callers should treat `0` as "unknown / pre-v1".
    pub fn get_pause_schema_version(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::PauseSchemaVersion)
            .unwrap_or(0)
    }

    /// Returns the idempotency storage schema version written during initialization.
    /// Returns `IDEMPOTENCY_SCHEMA_VERSION_V1` (1) for contracts initialized after
    /// this upgrade. Returns `0` for legacy contracts that predate the schema
    /// version marker — callers should treat `0` as "unknown / pre-v1".
    pub fn get_idempotency_schema_version(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::IdempotencySchemaVersion)
            .unwrap_or(0)
    }

    /// Check if an operation is paused, applying TTL-based auto-unpause if needed.
    ///
    /// If a pause mode has `unpause_at` set and the current ledger timestamp strictly
    /// exceeds that value, the mode is automatically cleared, storage is updated, and
    /// an `AUTO_UNPAUSE` event is emitted with `actor = "system"`. This is an O(1)
    /// check with no iteration. Repeated calls after clearing do NOT re-emit.
    fn check_paused(env: &Env, program_id: Option<&String>, operation: Symbol) -> bool {
        if Self::is_maintenance_mode(env.clone()) && operation == symbol_short!("lock") {
            return true;
        }

        let mut flags = Self::get_pause_flags(env);
        let current_time = env.ledger().timestamp();
        let mut flags_changed = false;

        // TTL check for lock mode.
        if flags.lock_paused {
            if let Some(unpause_at) = flags.lock_unpause_at {
                if current_time > unpause_at {
                    flags.lock_paused = false;
                    flags.lock_unpause_at = None;
                    flags_changed = true;
                    let receipt_id = Self::increment_receipt_id(env);
                    env.events().publish(
                        (AUTO_UNPAUSE, symbol_short!("lock")),
                        AutoUnpauseEvent {
                            version: EVENT_VERSION_V2,
                            operation: symbol_short!("lock"),
                            actor: String::from_str(env, "system"),
                            unpause_at,
                            triggered_at: current_time,
                            receipt_id,
                        },
                    );
                }
            }
        }

        // TTL check for release mode.
        if flags.release_paused {
            if let Some(unpause_at) = flags.release_unpause_at {
                if current_time > unpause_at {
                    flags.release_paused = false;
                    flags.release_unpause_at = None;
                    flags_changed = true;
                    let receipt_id = Self::increment_receipt_id(env);
                    env.events().publish(
                        (AUTO_UNPAUSE, symbol_short!("release")),
                        AutoUnpauseEvent {
                            version: EVENT_VERSION_V2,
                            operation: symbol_short!("release"),
                            actor: String::from_str(env, "system"),
                            unpause_at,
                            triggered_at: current_time,
                            receipt_id,
                        },
                    );
                }
            }
        }

        // TTL check for refund mode.
        if flags.refund_paused {
            if let Some(unpause_at) = flags.refund_unpause_at {
                if current_time > unpause_at {
                    flags.refund_paused = false;
                    flags.refund_unpause_at = None;
                    flags_changed = true;
                    let receipt_id = Self::increment_receipt_id(env);
                    env.events().publish(
                        (AUTO_UNPAUSE, symbol_short!("refund")),
                        AutoUnpauseEvent {
                            version: EVENT_VERSION_V2,
                            operation: symbol_short!("refund"),
                            actor: String::from_str(env, "system"),
                            unpause_at,
                            triggered_at: current_time,
                            receipt_id,
                        },
                    );
                }
            }
        }

        if flags_changed {
            // Clear shared pause metadata if all modes are now unpaused.
            let any_paused = flags.lock_paused || flags.release_paused || flags.refund_paused;
            if !any_paused {
                flags.pause_reason = None;
                flags.paused_at = 0;
            }
            env.storage().instance().set(&DataKey::PauseFlags, &flags);
        }

        let mut global_paused = false;
        if operation == symbol_short!("lock") {
            global_paused = flags.lock_paused;
        } else if operation == symbol_short!("release") {
            global_paused = flags.release_paused;
        } else if operation == symbol_short!("refund") {
            global_paused = flags.refund_paused;
        }

        if global_paused {
            return true;
        }

        if let Some(pid) = program_id {
            let mut program_flags = Self::get_program_pause_flags(env, pid.clone());
            let mut p_flags_changed = false;

            if program_flags.lock_paused {
                if let Some(unpause_at) = program_flags.lock_unpause_at {
                    if current_time > unpause_at {
                        program_flags.lock_paused = false;
                        program_flags.lock_unpause_at = None;
                        p_flags_changed = true;
                    }
                }
            }
            if program_flags.release_paused {
                if let Some(unpause_at) = program_flags.release_unpause_at {
                    if current_time > unpause_at {
                        program_flags.release_paused = false;
                        program_flags.release_unpause_at = None;
                        p_flags_changed = true;
                    }
                }
            }
            if program_flags.refund_paused {
                if let Some(unpause_at) = program_flags.refund_unpause_at {
                    if current_time > unpause_at {
                        program_flags.refund_paused = false;
                        program_flags.refund_unpause_at = None;
                        p_flags_changed = true;
                    }
                }
            }

            if p_flags_changed {
                let any_paused = program_flags.lock_paused || program_flags.release_paused || program_flags.refund_paused;
                if !any_paused {
                    program_flags.pause_reason = None;
                    program_flags.paused_at = 0;
                }
                env.storage().instance().set(&DataKey::ProgramPauseFlags(pid.clone()), &program_flags);
            }

            if operation == symbol_short!("lock") {
                return program_flags.lock_paused;
            } else if operation == symbol_short!("release") {
                return program_flags.release_paused;
            } else if operation == symbol_short!("refund") {
                return program_flags.refund_paused;
            }
        }

        false
    }

    // --- Circuit Breaker & Rate Limit ---

    pub fn set_circuit_admin(env: Env, new_admin: Address, caller: Option<Address>) {
        error_recovery::set_circuit_admin(&env, new_admin, caller);
    }

    pub fn get_circuit_admin(env: Env) -> Option<Address> {
        error_recovery::get_circuit_admin(&env)
    }

    /// Return a full snapshot of the circuit breaker state.
    ///
    /// Upgrade-safe: reads from persistent storage; returns defaults for
    /// legacy deployments that have never written circuit breaker state.
    pub fn get_circuit_breaker_status(env: Env) -> error_recovery::CircuitBreakerStatus {
        error_recovery::get_status(&env)
    }

    pub fn reset_circuit_breaker(env: Env, caller: Address) {
        caller.require_auth();
        let admin = error_recovery::get_circuit_admin(&env).expect("Circuit admin not set");
        if caller != admin {
            panic!("Unauthorized: only circuit admin can reset");
        }
        error_recovery::reset_circuit_breaker(&env, &admin);
    }

    pub fn configure_circuit_breaker(
        env: Env,
        caller: Address,
        failure_threshold: u32,
        success_threshold: u32,
        max_error_log: u32,
        recovery_window: u64,
    ) {
        caller.require_auth();
        let admin = error_recovery::get_circuit_admin(&env).expect("Circuit admin not set");
        if caller != admin {
            panic!("Unauthorized: only circuit admin can configure");
        }

        let config = error_recovery::CircuitBreakerConfig {
            failure_threshold,
            success_threshold,
            max_error_log,
            recovery_window,
        };
        error_recovery::set_config(&env, config);
    }

    /// Return a full snapshot of the circuit breaker's current status.
    ///
    /// Includes state, failure/success counts, timestamps, and configured thresholds.
    /// Safe to call at any time; never modifies state.
    pub fn get_circuit_status(env: Env) -> error_recovery::CircuitBreakerStatus {
        error_recovery::get_status(&env)
    }

    /// Return the full circuit breaker error log (last N entries).
    pub fn get_circuit_error_log(env: Env) -> soroban_sdk::Vec<error_recovery::ErrorEntry> {
        error_recovery::get_error_log(&env)
    }

    /// Archive hot circuit breaker failure logs for a program.
    ///
    /// Requires authorization from the registered circuit breaker admin. Archived
    /// timestamps are stored as compact offsets to reduce persistent storage.
    pub fn archive_circuit_breaker_logs(
        env: Env,
        program_id: String,
    ) -> error_recovery::CompactFailureArchive {
        error_recovery::archive_circuit_breaker_logs(&env, program_id)
    }

    /// Return the compact archived circuit breaker failures for a program.
    pub fn get_circuit_failure_archive(
        env: Env,
        program_id: String,
    ) -> error_recovery::CompactFailureArchive {
        error_recovery::get_failure_archive(&env, program_id)
    }

    /// Emergency-open the circuit breaker (circuit admin only).
    ///
    /// Immediately transitions the circuit to `Open`, blocking all payouts.
    /// Use when a security incident is detected and payouts must be halted
    /// before the failure threshold is naturally reached.
    ///
    /// Emits a `cb_open` audit event with reason `"emergency"`.
    pub fn emergency_open_circuit(env: Env, admin: Address) {
        admin.require_auth();
        let stored = error_recovery::get_circuit_admin(&env).expect("Circuit admin not set");
        if admin != stored {
            panic!("Unauthorized: only circuit admin can emergency-open circuit");
        }
        error_recovery::open_circuit(&env);
    }

    /// Initialize threshold monitoring with default configuration.
    ///
    /// Must be called once after contract deployment to enable threshold-based
    /// circuit breaking. Idempotent — safe to call multiple times.
    pub fn init_threshold_monitoring(env: Env) {
        threshold_monitor::init_threshold_monitor(&env);
    }

    /// Return the current threshold monitoring configuration.
    pub fn get_threshold_config(env: Env) -> threshold_monitor::ThresholdConfig {
        threshold_monitor::get_threshold_config(&env)
    }

    /// Return the upgrade-safe circuit-breaker schema version.
    /// Returns `0` on legacy deployments where the marker was never written.
    pub fn get_cb_schema_version(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::CircuitBreakerSchemaVersion)
            .unwrap_or(0u32)
    }

    /// Update the global rate limit configuration.
    ///
    /// # Precedence Note
    /// The global `RateLimitConfig` is currently not strictly enforced for payout batch sizes
    /// or cumulative payout volumes. The per-program spend threshold (set via `set_program_spend_threshold`)
    /// acts as the most restrictive and only effective limit for payouts. The per-program value
    /// implicitly overrides this global configuration for payout bounds.
    pub fn update_rate_limit_config(
        env: Env,
        window_size: u64,
        max_operations: u32,
        cooldown_period: u64,
    ) {
        // Only admin can update rate limit config
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let config = RateLimitConfig {
            window_size,
            max_operations,
            cooldown_period,
        };
        env.storage()
            .instance()
            .set(&DataKey::RateLimitConfig, &config);

        // Emit audit event for rate limit config update
        env.events().publish(
            (symbol_short!("rate_lim"), symbol_short!("update")),
            (
                window_size,
                max_operations,
                cooldown_period,
                admin,
                env.ledger().timestamp(),
            ),
        );
    }

    pub fn get_rate_limit_config(env: Env) -> RateLimitConfig {
        env.storage()
            .instance()
            .get(&DataKey::RateLimitConfig)
            .unwrap_or(RateLimitConfig {
                window_size: 3600,
                max_operations: 10,
                cooldown_period: 60,
            })
    }

    /// Set the per-program spend threshold.
    ///
    /// # Invariant
    /// After this call, any single payout or batch total exceeding
    /// `threshold_amount` will be rejected with `SpendLimitExceeded` and
    /// a `SpendLimitExceededEvent` audit event will be emitted.
    ///
    /// # Security and deterministic behavior
    /// - Admin only. This requires admin authority, not any delegate permission bit.
    /// - `threshold_amount` must be strictly positive; zero or negative
    ///   values are rejected with `InvalidAmount`.
    /// - Payout validation checks this threshold **before** balance checks
    ///   so clients observe stable, deterministic failures.
    /// - Emits `SpendLimitSetEvent` after the new value is persisted.
    ///
    /// # Precedence Note
    /// This per-program threshold is the strictly enforced limit for payouts.
    /// It effectively overrides any global limits such as `RateLimitConfig`, which
    /// are not actively enforced as blocking limits for batch sizes or volumes.
    pub fn set_program_spend_threshold(env: Env, program_id: String, threshold_amount: i128) {
        let admin = Self::require_admin(&env);
        if threshold_amount <= 0 {
            panic!("Invalid spend threshold");
        }

        let mut cfg: MultisigConfig = env
            .storage()
            .persistent()
            .get(&DataKey::MultisigConfig(program_id.clone()))
            .unwrap_or(MultisigConfig {
                threshold_amount: i128::MAX,
                signers: vec![&env],
                required_signatures: 0,
            });

        let previous_threshold = cfg.threshold_amount;
        cfg.threshold_amount = threshold_amount;
        env.storage()
            .persistent()
            .set(&DataKey::MultisigConfig(program_id.clone()), &cfg);

        // Emit audit event after storage write (CEI ordering).
        env.events().publish(
            (SPEND_LIMIT_SET, program_id.clone()),
            SpendLimitSetEvent {
                version: EVENT_VERSION_V2,
                program_id,
                previous_threshold,
                new_threshold: threshold_amount,
                set_by: admin,
                timestamp: env.ledger().timestamp(),
            },
        );
    }

    /// Read per-program spend threshold. Returns `i128::MAX` when unset (unlimited).
    pub fn get_program_spend_threshold(env: Env, program_id: String) -> i128 {
        let cfg: MultisigConfig = env
            .storage()
            .persistent()
            .get(&DataKey::MultisigConfig(program_id))
            .unwrap_or(MultisigConfig {
                threshold_amount: i128::MAX,
                signers: vec![&env],
                required_signatures: 0,
            });
        cfg.threshold_amount
    }

    /// Returns the spend-limit storage schema version written during `init_program`.
    /// Returns `0` on legacy deployments where the marker was never written.
    pub fn get_spend_limit_schema_version(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::SpendLimitSchemaVersion)
            .unwrap_or(0u32)
    }

    /// Enforce the per-program spend threshold.
    ///
    /// Returns `Err(())` and emits a `SpendLimitExceededEvent` when
    /// `requested_amount > threshold`. The caller is responsible for
    /// clearing the reentrancy guard and panicking with the appropriate
    /// error before any token transfer occurs.
    fn enforce_spend_threshold(
        env: &Env,
        program_id: &String,
        requested_amount: i128,
    ) -> Result<(), ()> {
        let cfg: MultisigConfig = env
            .storage()
            .persistent()
            .get(&DataKey::MultisigConfig(program_id.clone()))
            .unwrap_or(MultisigConfig {
                threshold_amount: i128::MAX,
                signers: vec![env],
                required_signatures: 0,
            });
        if requested_amount > cfg.threshold_amount {
            // Emit audit event before returning the error so the rejection
            // is always visible on-chain even if the caller panics.
            env.events().publish(
                (SPEND_LIMIT_EXCEEDED, program_id.clone()),
                SpendLimitExceededEvent {
                    version: EVENT_VERSION_V2,
                    program_id: program_id.clone(),
                    requested_amount,
                    threshold: cfg.threshold_amount,
                    timestamp: env.ledger().timestamp(),
                },
            );
            return Err(());
        }
        Ok(())
    }

}
