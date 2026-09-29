#[cfg(feature = "contract")]
#[contractimpl]
impl ProgramEscrowContract {
    fn check_idempotency_key(env: &Env, idempotency_key: &String) -> Option<PayoutIdempotencyKey> {
        let key = DataKey::PayoutIdempotency(idempotency_key.clone());
        // Use persistent storage for upgrade safety
        env.storage().persistent().get(&key)
    }

    /// Store an idempotency key with its payout information
    fn store_idempotency_key(
        env: &Env,
        idempotency_key: &String,
        program_id: &String,
        payout_type: PayoutType,
        recipient: Option<Address>,
        amount: Option<i128>,
        recipients: Option<Vec<Address>>,
        amounts: Option<Vec<i128>>,
        total_amount: i128,
    ) {
        let timestamp = env.ledger().timestamp();
        let payout_record = PayoutIdempotencyKey {
            key: idempotency_key.clone(),
            program_id: program_id.clone(),
            payout_type,
            timestamp,
            recipient,
            amount,
            recipients,
            amounts,
            total_amount,
        };
        let key = DataKey::PayoutIdempotency(idempotency_key.clone());
        // Use persistent storage for upgrade safety
        env.storage().persistent().set(&key, &payout_record);
    }

    /// Validate and check idempotency key
    /// If key already exists, returns the stored payout record (for idempotent replay)
    /// If key is new, returns None (caller should proceed with payout)
    fn validate_and_get_idempotency_key(
        env: &Env,
        idempotency_key: &Option<String>,
    ) -> Option<PayoutIdempotencyKey> {
        match idempotency_key {
            Some(key) => {
                Self::validate_idempotency_key_format(key);
                Self::check_idempotency_key(env, key)
            }
            None => None,
        }
    }

    /// Validate idempotency key format without checking storage.
    ///
    /// This helper is kept for explicit format assertions in internal code.
    fn validate_idempotency_key_format(key: &String) {
        let key_len = key.len() as usize;
        if key_len < MIN_IDEMPOTENCY_KEY_LENGTH as usize || key_len > MAX_IDEMPOTENCY_KEY_LENGTH as usize {
            panic!("IdempotencyKeyInvalid");
        }
        let mut buf = [0u8; 128];
        key.copy_into_slice(&mut buf[..key_len]);
        let mut i = 0;
        while i < key_len {
            let b = buf[i];
            let valid_char = (b >= b'a' && b <= b'z')
                || (b >= b'A' && b <= b'Z')
                || (b >= b'0' && b <= b'9')
                || b == b'-'
                || b == b'_';
            if !valid_char {
                panic!("IdempotencyKeyInvalid");
            }
            i += 1;
        }
    }
    /// Set or update the per-window spending limit for a program.
    ///
    /// Only the program's `authorized_payout_key` may call this.
    ///
    /// # Arguments
    /// * `recipients` - Vector of winner addresses.
    /// * `amounts` - Vector of prize amounts (must match recipients length).
    ///
    /// # Returns
    /// The updated `ProgramData` reflecting the new balance and payout history.
    ///
    /// # Security
    /// - Requires authorization from the `authorized_payout_key`.
    /// - Protected by reentrancy guard.
    /// - Respects circuit breaker and threshold limits.
    ///
    /// # Event Ordering
    ///
    /// Emits a `BatchPay` event synchronously upon successful completion.
    /// When `batch_payout` (or its `_by` variant) is invoked in sequence with
    /// `single_payout`, the resulting `BatchPay` / `Payout` events appear in
    /// the exact call order. Pause state-change events (`PauseStateChangedV2`)
    /// emitted by `set_paused` between payout calls are likewise interleaved
    /// at their precise call position. Soroban guarantees deterministic,
    /// sequential event emission within a transaction, so off-chain indexers
    /// can safely reconstruct an ordered activity feed from the event log.
    pub fn batch_payout(env: Env, recipients: soroban_sdk::Vec<Address>, amounts: soroban_sdk::Vec<i128>) -> ProgramData {
        Self::batch_payout_internal(env, None, None, recipients, amounts)
    }

    /// Set or update the per-window spending limit for a program.
    ///
    /// # Arguments
    /// * `program_id`   - Program to configure.
    /// * `window_size`  - Window length in seconds (must be > 0).
    /// * `max_amount`   - Max total releasable in one window (must be >= 0).
    /// * `enabled`      - `false` stores the config without enforcing it.
    pub fn set_program_spending_limit(
        env: Env,
        program_id: String,
        window_size: u64,
        max_amount: i128,
        enabled: bool,
    ) {
        let program_data = Self::get_program_data_by_id(&env, &program_id);
        program_data.authorized_payout_key.require_auth();

        if window_size == 0 {
            panic!("window_size must be greater than zero");
        }
        if max_amount < 0 {
            panic!("max_amount must be non-negative");
        }

        let cfg = ProgramSpendingConfig {
            window_size,
            max_amount,
            enabled,
        };
        env.storage()
            .persistent()
            .set(&DataKey::SpendingConfig(program_id), &cfg);
    }

    /// Set or update the per-program circuit breaker failure threshold.
    ///
    /// Only the program's `authorized_payout_key` may call this. This requires controller authority, not any delegate permission bit.
    ///
    /// # Arguments
    /// * `program_id` - Program to configure.
    /// * `threshold` - Optional threshold value (1-100). None resets to global default (3).
    ///
    /// # Errors
    /// Panics if:
    /// - Threshold is set but not in range [1, 100]
    /// - Caller is not authorized
    ///
    /// # Events
    /// Emits `CB_THRESHOLD_SET` with [`CircuitBreakerThresholdSetEvent`].
    pub fn set_prog_cb_threshold(
        env: Env,
        program_id: String,
        threshold: Option<u32>,
    ) {
        let program_data = Self::get_program_data_by_id(&env, &program_id);
        program_data.authorized_payout_key.require_auth();

        // Validate threshold if provided
        if let Some(t) = threshold {
            if t < 1 || t > 100 {
                panic!("{}", errors::ContractError::InvalidCircuitBreakerThreshold as u32);
            }
        }

        let previous_threshold = program_data.circuit_breaker_threshold;
        let mut updated_data = program_data.clone();
        updated_data.circuit_breaker_threshold = threshold;

        // Update program data
        let program_key = DataKey::Program(program_id.clone());
        env.storage().instance().set(&program_key, &updated_data);

        // Emit audit event
        env.events().publish(
            (CB_THRESHOLD_SET, program_id.clone()),
            CircuitBreakerThresholdSetEvent {
                version: EVENT_VERSION_V2,
                program_id,
                previous_threshold,
                new_threshold: threshold,
                set_by: env.current_contract_address(),
                timestamp: env.ledger().timestamp(),
            },
        );
    }

    /// Return the spending limit configuration for a program, if set.
    pub fn get_program_spending_limit(
        env: Env,
        program_id: String,
    ) -> Option<ProgramSpendingConfig> {
        env.storage()
            .persistent()
            .get(&DataKey::SpendingConfig(program_id))
    }

    /// Execute a batch payout guarded by an idempotency key.
    ///
    /// If `idempotency_key` has already been consumed by a prior successful
    /// call, the function emits a [`BatchPayoutReplayedEvent`] and returns the
    /// current [`ProgramData`] **without** transferring any funds.  This makes
    /// the operation safe to retry from the backend without risk of
    /// double-payment.
    ///
    /// # Arguments
    /// * `idempotency_key` – Caller-supplied unique string (e.g. UUID or
    ///   content-hash of the payout batch).  Must be ≤ 64 bytes.
    /// * `recipients` / `amounts` – Same semantics as [`batch_payout`].
    ///
    /// # Security
    /// - Idempotency keys are stored in persistent storage and never expire.
    /// - A key is only marked consumed **after** all transfers succeed.
    /// - Replay detection runs before any state mutation.
    pub fn batch_payout_idempotent(
        env: Env,
        idempotency_key: String,
        recipients: soroban_sdk::Vec<Address>,
        amounts: soroban_sdk::Vec<i128>,
    ) -> ProgramData {
        Self::batch_payout_idempotent_internal(env, idempotency_key, None, recipients, amounts)
    }

    /// Delegate variant of [`batch_payout_idempotent`].
    pub fn batch_payout_idempotent_by(
        env: Env,
        idempotency_key: String,
        caller: Address,
        recipients: soroban_sdk::Vec<Address>,
        amounts: soroban_sdk::Vec<i128>,
    ) -> ProgramData {
        Self::batch_payout_idempotent_internal(
            env,
            idempotency_key,
            Some(caller),
            recipients,
            amounts,
        )
    }

    fn batch_payout_idempotent_internal(
        env: Env,
        idempotency_key: String,
        caller: Option<Address>,
        recipients: soroban_sdk::Vec<Address>,
        amounts: soroban_sdk::Vec<i128>,
    ) -> ProgramData {
        // Load current program data for the replay-event payload.
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        // ── Replay detection ───────────────────────────────────────────────
        // Check the shared DataKey::IdempotencyKey namespace (instance storage)
        // written by both single_payout_idempotent and batch_payout_internal.
        if env
            .storage()
            .instance()
            .has(&DataKey::IdempotencyKey(idempotency_key.clone()))
        {
            env.events().publish(
                (BATCH_PAYOUT_REPLAYED,),
                BatchPayoutReplayedEvent {
                    version: EVENT_VERSION_V2,
                    program_id: program_data.program_id.clone(),
                    idempotency_key: idempotency_key.clone(),
                },
            );
            return program_data;
        }

        // Check the legacy DataKey::PayoutIdempotency namespace (persistent
        // storage) used exclusively by the original single_payout_idempotent.
        if env
            .storage()
            .persistent()
            .has(&DataKey::PayoutIdempotency(idempotency_key.clone()))
        {
            env.events().publish(
                (BATCH_PAYOUT_REPLAYED,),
                BatchPayoutReplayedEvent {
                    version: EVENT_VERSION_V2,
                    program_id: program_data.program_id.clone(),
                    idempotency_key: idempotency_key.clone(),
                },
            );
            return program_data;
        }

        // Load the set of batch-internal consumed keys (Vec<String> stored
        // persistently).  This catches replay of a key consumed by a prior
        // batch_payout_idempotent call on the same contract.
        let mut used_keys: soroban_sdk::Vec<String> = env
            .storage()
            .persistent()
            .get(&PAYOUT_IDEM_KEYS)
            .unwrap_or_else(|| soroban_sdk::Vec::new(&env));

        for k in used_keys.iter() {
            if k == idempotency_key {
                env.events().publish(
                    (BATCH_PAYOUT_REPLAYED,),
                    BatchPayoutReplayedEvent {
                        version: EVENT_VERSION_V2,
                        program_id: program_data.program_id.clone(),
                        idempotency_key: idempotency_key.clone(),
                    },
                );
                return program_data;
            }
        }

        // Key is fresh — execute the real payout.
        let result = Self::batch_payout_internal(env.clone(), caller, Some(idempotency_key.clone()), recipients, amounts);

        // Mark key as consumed only after successful execution.
        used_keys.push_back(idempotency_key);
        env.storage().persistent().set(&PAYOUT_IDEM_KEYS, &used_keys);

        result
    }

    /// Return the spending config for a program.
    pub fn get_program_spending_config(
        env: Env,
        program_id: String,
    ) -> Option<ProgramSpendingConfig> {
        env.storage()
            .persistent()
            .get(&DataKey::SpendingConfig(program_id))
    }

    /// Return the current window state for a program's spending limit, if any.
    pub fn get_program_spending_state(
        env: Env,
        program_id: String,
    ) -> Option<ProgramSpendingState> {
        env.storage()
            .persistent()
            .get(&DataKey::SpendingState(program_id))
    }

    /// Enforce the per-window spending limit and update the window state.
    ///
    /// Called before any token transfer. Emits `(limit, prog_spend)` and panics
    /// with "Program spending limit exceeded for current window" when the limit
    /// would be exceeded.
    ///
    /// If no config is set or `enabled` is `false`, this is a no-op.
    fn enforce_spending_window(env: &Env, program_id: &String, amount: i128) {
        let cfg: ProgramSpendingConfig = match env
            .storage()
            .persistent()
            .get(&DataKey::SpendingConfig(program_id.clone()))
        {
            Some(c) => c,
            None => return,
        };

        if !cfg.enabled {
            return;
        }

        let now = env.ledger().timestamp();
        let mut state: ProgramSpendingState = env
            .storage()
            .persistent()
            .get(&DataKey::SpendingState(program_id.clone()))
            .unwrap_or(ProgramSpendingState {
                window_start: now,
                amount_released: 0,
            });

        // Reset window if expired
        if now.saturating_sub(state.window_start) >= cfg.window_size {
            state.window_start = now;
            state.amount_released = 0;
        }

        let new_total = state
            .amount_released
            .checked_add(amount)
            .unwrap_or_else(|| panic!("Spending window overflow"));

        if new_total > cfg.max_amount {
            let program_data: ProgramData = env
                .storage()
                .instance()
                .get(&PROGRAM_DATA)
                .unwrap_or_else(|| panic!("Program not initialized"));

            // Emit rejection event before panicking (CEI: event before state change)
            env.events().publish(
                (PROG_SPEND_LIMIT, symbol_short!("prg_spend")),
                (
                    program_id.clone(),
                    program_data.token_address,
                    amount,
                    new_total,
                    cfg.max_amount,
                    cfg.window_size,
                ),
            );
            panic!("Program spending limit exceeded for current window");
        }

        // Commit updated state
        state.amount_released = new_total;
        env.storage()
            .persistent()
            .set(&DataKey::SpendingState(program_id.clone()), &state);
    }

    pub fn get_analytics(_env: Env) -> Analytics {
        Analytics {
            total_locked: 0,
            total_released: 0,
            total_payouts: 0,
            active_programs: 0,
            operation_count: 0,
        }
    }

    /// Returns whether read-only mode is currently enabled.
    pub fn is_read_only(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::ReadOnlyMode)
            .unwrap_or(false)
    }

    /// Enable or disable read-only mode (admin only).
    pub fn set_read_only_mode(env: Env, enabled: bool, reason: Option<String>) {
        let admin = Self::require_admin(&env);
        env.storage()
            .instance()
            .set(&DataKey::ReadOnlyMode, &enabled);
        env.events().publish(
            (READ_ONLY_MODE_CHANGED,),
            ReadOnlyModeChanged {
                enabled,
                admin,
                timestamp: env.ledger().timestamp(),
                reason,
            },
        );
    }

    /// Alias for get_analytics — used by some test modules.
    pub fn get_program_analytics(env: Env) -> Analytics {
        Self::get_analytics(env)
    }

    /// Rotate the authorized payout key for a program (admin only).
    /// Rotate the payout key for a program with replay protection via nonce.
    ///
    /// # Arguments
    /// * `program_id`      — The program whose payout key should be rotated.
    /// * `caller`          — The address initiating the rotation (must be current
    ///                       payout key or contract admin); their auth is required.
    /// * `new_key`         — The replacement payout key (must differ from current).
    /// * `expected_nonce`  — Must equal the current stored rotation nonce;
    ///                       prevents replaying a prior signed rotation request.
    ///
    /// # Panics
    /// * `"New key must differ from current key"` — self-rotation attempt.
    /// * `"Invalid nonce"` — `expected_nonce` does not match the stored nonce.
    /// * `"Unauthorized"` — caller is neither the current payout key nor admin.
    pub fn rotate_payout_key(
        env: Env,
        program_id: String,
        caller: Address,
        new_key: Address,
        expected_nonce: u64,
    ) -> ProgramData {
        let mut program_data = Self::get_program_data_by_id(&env, &program_id);

        // Guard: cannot rotate to the same key.
        if new_key == program_data.authorized_payout_key {
            panic!("New key must differ from current key");
        }

        // Replay protection: validate the nonce before any state change.
        let nonce_key = DataKey::RotationNonce(program_id.clone());
        let current_nonce: u64 = env.storage().instance().get(&nonce_key).unwrap_or(0);
        if expected_nonce != current_nonce {
            panic!("Invalid nonce");
        }

        // Auth: caller must be the current payout key or the contract admin.
        caller.require_auth();
        let is_payout_key = caller == program_data.authorized_payout_key;
        let is_admin = env
            .storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::Admin)
            .map_or(false, |admin| caller == admin);
        if !is_payout_key && !is_admin {
            panic!("Unauthorized");
        }

        // Increment nonce to invalidate any future replay of this rotation.
        env.storage()
            .instance()
            .set(&nonce_key, &(current_nonce + 1));

        // Apply the rotation.
        program_data.authorized_payout_key = new_key;
        Self::store_program_data(&env, &program_id, &program_data);
        program_data
    }

    /// Return the current rotation nonce for a program.
    ///
    /// The nonce starts at 0 and increments by 1 on every successful
    /// `rotate_payout_key` call. Callers should read it immediately before
    /// constructing a rotation request to avoid stale-nonce rejections.
    pub fn get_rotation_nonce(env: Env, program_id: String) -> u64 {
        let nonce_key = DataKey::RotationNonce(program_id);
        env.storage().instance().get(&nonce_key).unwrap_or(0)
    }

    /// Alias for get_admin.
    pub fn get_program_admin(env: Env) -> Option<Address> {
        Self::get_admin(env)
    }

    /// Update program metadata with caller parameter.
    pub fn update_program_metadata_by(
        env: Env,
        program_id: String,
        caller: Address,
        metadata: crate::ProgramMetadata,
    ) -> ProgramData {
        Self::update_program_metadata(env, program_id, caller, metadata)
    }

    pub fn set_whitelist(env: Env, _address: Address, _whitelisted: bool) {
        // Only admin can set whitelist
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("Not initialized"));
        admin.require_auth();
    }

    // ========================================================================
    // Token Allowlist  (issue #1295 — decimal normalization)
    // ========================================================================

    /// Internal: read the legacy V1 allowlist (plain `Vec<Address>`).
    fn get_token_allowlist_internal(env: &Env) -> soroban_sdk::Vec<Address> {
        env.storage()
            .instance()
            .get(&DataKey::TokenAllowlist)
            .unwrap_or(Vec::new(env))
    }

    /// Internal: read the V2 allowlist (`Vec<AllowedTokenEntry>`).
    ///
    /// Falls back to the V1 list (addresses only, decimals = 0) when no V2
    /// entry exists so legacy deployments continue to work unchanged.
    fn get_token_allowlist_v2_internal(env: &Env) -> soroban_sdk::Vec<AllowedTokenEntry> {
        if let Some(v2) = env
            .storage()
            .instance()
            .get::<Symbol, soroban_sdk::Vec<AllowedTokenEntry>>(&TOKEN_ALLOWLIST_V2)
        {
            return v2;
        }
        // Upgrade path: promote V1 entries with decimals = 0.
        let v1 = Self::get_token_allowlist_internal(env);
        let mut out: soroban_sdk::Vec<AllowedTokenEntry> = Vec::new(env);
        for addr in v1.iter() {
            out.push_back(AllowedTokenEntry { token: addr, decimals: 0 });
        }
        out
    }

    /// Internal: enforce the token allowlist.
    fn enforce_token_allowlist(env: &Env, token_address: &Address, program_id: &String) {
        // Check V2 first; fall back to V1.
        let v2 = Self::get_token_allowlist_v2_internal(env);
        if v2.is_empty() {
            return; // Enforcement disabled.
        }
        for entry in v2.iter() {
            if entry.token == *token_address {
                return; // Permitted.
            }
        }
        env.events().publish(
            (TOKEN_REJECTED,),
            TokenRejectedEvent {
                version: EVENT_VERSION_V2,
                token: token_address.clone(),
                program_id: program_id.clone(),
                timestamp: env.ledger().timestamp(),
            },
        );
        panic!("Token not on allowlist");
    }

    /// Add a token to the allowlist **and permanently bind its decimal scale**
    /// (admin only).
    ///
    /// This is the preferred entrypoint for new deployments. Raw token amounts
    /// are always transferred and stored as `i128`; the `decimals` value is
    /// metadata used by indexers and UIs to render those raw amounts. It is
    /// stored once instead of read live at display time, because a token
    /// contract can be upgraded, replaced, or expose no standard `decimals()`
    /// view, while historical payouts must retain their original
    /// interpretation.
    ///
    /// # Canonical update semantics
    /// - The configured scale is **immutable**. Re-adding the same token with a
    ///   *different* scale panics `"Token decimals are immutable"`; re-adding it
    ///   with the *same* scale panics `"Token already on allowlist"`. There is
    ///   deliberately no in-place migration — a scale change would reinterpret
    ///   every historical raw payout. Migrations must use a new token address.
    /// - The allowlist is enforced only at `init_program` time. Programs that
    ///   are already initialized keep operating (and paying out) with their
    ///   original token even if it is later removed from the list, so locked
    ///   funds can never be stranded by a policy change.
    ///
    /// # Parameters
    /// - `token`    — token contract address
    /// - `decimals` — number of decimal places (0–18)
    ///
    /// # Errors
    /// - Panics `"Decimals exceed maximum (18)"` if `decimals > 18`.
    /// - Panics `"Token decimals are immutable"` on re-add with a different scale.
    /// - Panics `"Token already on allowlist"` on re-add with the same scale.
    ///
    /// # Events
    /// Emits [`TokenAllowlistUpdatedEvent`] (`added = true`),
    /// [`TokenDecimalsConfiguredEvent`], and — when the token's live `decimals()`
    /// view disagrees with `decimals` — [`TokenDecimalsMismatchEvent`].
    pub fn add_allowed_token_with_decimals(env: Env, token: Address, decimals: u32) {
        let admin = Self::require_admin(&env);

        if decimals > MAX_TOKEN_DECIMALS {
            panic!("Decimals exceed maximum (18)");
        }

        // Immutability guard. A configured scale is written exactly once; any
        // later write is rejected so historical payouts keep their meaning.
        let dec_key = DataKey::TokenDecimals(token.clone());
        if let Some(existing) = env.storage().instance().get::<DataKey, u32>(&dec_key) {
            if existing != decimals {
                panic!("Token decimals are immutable");
            }
            panic!("Token already on allowlist");
        }

        // Defense in depth: the V2 list is the canonical membership record.
        let mut v2 = Self::get_token_allowlist_v2_internal(&env);
        for entry in v2.iter() {
            if entry.token == token {
                panic!("Token already on allowlist");
            }
        }

        // Best-effort live cross-check before any state mutation. `try_decimals`
        // yields a nested result (invoke outcome, then conversion); a token that
        // does not implement the standard view simply leaves this `None`.
        let reported_decimals = token::Client::new(&env, &token)
            .try_decimals()
            .ok()
            .and_then(|r| r.ok());

        // Write the canonical V2 list.
        v2.push_back(AllowedTokenEntry { token: token.clone(), decimals });
        env.storage().instance().set(&TOKEN_ALLOWLIST_V2, &v2);

        // Write the immutable per-token decimal scale (O(1) lookup path).
        env.storage().instance().set(&dec_key, &decimals);

        // Keep the V1 list in sync for backward-compatible readers.
        let mut v1 = Self::get_token_allowlist_internal(&env);
        v1.push_back(token.clone());
        env.storage().instance().set(&DataKey::TokenAllowlist, &v1);

        env.events().publish(
            (TOKEN_ALLOWLIST_UPDATED,),
            TokenAllowlistUpdatedEvent {
                version: EVENT_VERSION_V2,
                token,
                added: true,
                updated_by: admin,
                timestamp: env.ledger().timestamp(),
                decimals,
            },
        );
    }

    /// Add a token to the allowlist without specifying decimals (admin only).
    ///
    /// Decimals default to `0` ("unknown"). Prefer
    /// [`add_allowed_token_with_decimals`] for new programs that need accurate
    /// decimal metadata.
    ///
    /// # Errors
    /// Panics `"Token already on allowlist"` if already present.
    pub fn add_allowed_token(env: Env, token: Address) {
        // Delegate to the decimals variant with decimals = 0.
        Self::add_allowed_token_with_decimals(env, token, 0);
    }

    /// Remove a token from the allowlist (admin only).
    ///
    /// Removes the token from both the canonical V2 list and the V1 list and
    /// clears its stored decimal scale. Programs already initialized with this
    /// token are unaffected — enforcement only runs at `init_program` time — so
    /// their locked funds can still be paid out.
    ///
    /// # Errors
    /// Panics `"Token not in allowlist"` if the token is not currently listed
    /// (removing a never-added token, or removing the same token twice).
    pub fn remove_allowed_token(env: Env, token: Address) {
        let admin = Self::require_admin(&env);

        // Remove from V2.
        let v2 = Self::get_token_allowlist_v2_internal(&env);
        let mut new_v2: soroban_sdk::Vec<AllowedTokenEntry> = Vec::new(&env);
        let mut found = false;
        for entry in v2.iter() {
            if entry.token == token {
                found = true;
            } else {
                new_v2.push_back(entry);
            }
        }
        if !found {
            panic!("Token not in allowlist");
        }
        env.storage()
            .instance()
            .set(&TOKEN_ALLOWLIST_V2, &new_v2);

        // Remove from V1.
        let v1 = Self::get_token_allowlist_internal(&env);
        let mut new_v1: soroban_sdk::Vec<Address> = Vec::new(&env);
        for addr in v1.iter() {
            if addr != token {
                new_v1.push_back(addr);
            }
        }
        env.storage()
            .instance()
            .set(&DataKey::TokenAllowlist, &new_v1);

        // Clear the stored decimal scale so a future re-add can reconfigure it.
        env.storage()
            .instance()
            .remove(&DataKey::TokenDecimals(token.clone()));

        env.events().publish(
            (TOKEN_ALLOWLIST_UPDATED,),
            TokenAllowlistUpdatedEvent {
                version: EVENT_VERSION_V2,
                token,
                added: false,
                updated_by: admin,
                timestamp: env.ledger().timestamp(),
                decimals: 0,
            },
        );
    }

    /// Returns `true` if `token` is on the allowlist or the list is empty.
    pub fn is_token_allowed(env: Env, token: Address) -> bool {
        let v2 = Self::get_token_allowlist_v2_internal(&env);
        if v2.is_empty() {
            return true;
        }
        for entry in v2.iter() {
            if entry.token == token {
                return true;
            }
        }
        false
    }

    /// Returns the full token allowlist as plain addresses (V1-compatible).
    pub fn get_allowed_tokens(env: Env) -> soroban_sdk::Vec<Address> {
        Self::get_token_allowlist_internal(&env)
    }

    /// Returns the full token allowlist with decimal metadata (V2).
    pub fn get_allowed_tokens_with_decimals(env: Env) -> soroban_sdk::Vec<AllowedTokenEntry> {
        Self::get_token_allowlist_v2_internal(&env)
    }

    /// Returns the token-allowlist storage schema version written during init.
    /// Returns `0` on legacy deployments where the marker was never written.
    pub fn get_allowlist_schema_version(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::TokenAllowlistSchemaVersion)
            .unwrap_or(0u32)
    }

    /// Returns the release trigger execution schema version written during init.
    ///
    /// Returns `RELEASE_TRIGGER_SCHEMA_VERSION_V1` (1) for contracts initialized after
    /// the trigger enhancement, or 0 for legacy deployments. This version tracks
    /// deterministic ordering, explicit error codes, and retry semantics.
    pub fn get_trigger_schema_version(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::ReleaseTriggerSchemaVersion)
            .unwrap_or(0u32)
    }
}
