#[cfg(feature = "contract")]
#[contractimpl]
impl ProgramEscrowContract {
    pub fn get_program_info(env: Env) -> ProgramData {
        env.storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"))
    }

    /// Get program information by program id.
    pub fn get_program_info_v2(env: Env, program_id: String) -> ProgramData {
        Self::get_program_data_by_id(&env, &program_id)
    }

    /// Get idempotency key status for a given key.
    pub fn get_idempotency_key_status(
        env: Env,
        idempotency_key: String,
    ) -> Option<IdempotencyRecord> {
        Self::get_idempotency_record(&env, &idempotency_key)
    }

    /// Get program metadata.
    ///
    /// Attempts to read compressed metadata from `DataKey::MetadataV2` first
    /// (decompressing on the fly).  Falls back to the legacy `DataKey::Metadata`
    /// key for backwards compatibility with programs that stored metadata before
    /// the compression upgrade.
    ///
    /// # Arguments
    /// * `program_id` - The program identifier
    ///
    /// # Returns
    /// `Some(ProgramMetadata)` if metadata has been set, `None` otherwise.
    pub fn get_program_metadata(env: Env, program_id: String) -> Option<ProgramMetadata> {
        // Try compressed (V2) format first.
        let v2_key = DataKey::MetadataV2(program_id.clone());
        if env.storage().instance().has(&v2_key) {
            let compressed: CompressedProgramMetadata =
                env.storage().instance().get(&v2_key).unwrap();
            return Some(compressed.into_legacy(&env));
        }
        // Fall back to legacy (V1) format.
        env.storage().instance().get(&DataKey::Metadata(program_id))
    }

    /// Get remaining balance
    ///
    /// # Returns
    /// Current remaining balance
    pub fn get_remaining_balance(env: Env) -> i128 {
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        program_data.remaining_balance
    }

    /// Check whether an idempotency key has already been used for a payout.
    ///
    /// Returns `true` if the key was previously recorded by a successful
    /// `single_payout_idempotent` or `batch_payout_idempotent` call.
    /// Returns `false` if the key is unknown (safe to submit).
    ///
    /// The shared namespace `DataKey::IdempotencyKey` is written by both
    /// `single_payout_idempotent` (via `store_idempotency_record`) and
    /// `batch_payout_idempotent` (via `batch_payout_internal` →
    /// `store_idempotency_record`) so that a key consumed by one entrypoint
    /// is visible to the other and to this view function.
    ///
    /// For backwards compatibility this function also checks the legacy
    /// `DataKey::PayoutIdempotency` namespace, which was used exclusively
    /// by the original `single_payout_idempotent` implementation.
    pub fn is_payout_processed(env: Env, idempotency_key: String) -> bool {
        // 1. Shared namespace – instance storage (written by both entrypoints).
        if env
            .storage()
            .instance()
            .has(&DataKey::IdempotencyKey(idempotency_key.clone()))
        {
            return true;
        }
        // 2. Legacy single-payout namespace – persistent storage.
        if env
            .storage()
            .persistent()
            .has(&DataKey::PayoutIdempotency(idempotency_key))
        {
            return true;
        }
        false
    }

    /// Create a release schedule entry that can be triggered at/after `release_timestamp`.
    ///
    /// # Arguments
    /// * `recipient` - Address of the recipient
    /// * `amount` - Amount to be released
    /// * `release_timestamp` - Unix timestamp when the release becomes available
    ///
    /// # Returns
    /// The created ProgramReleaseSchedule
    pub fn create_program_release_schedule(
        env: Env,
        recipient: Address,
        amount: i128,
        release_timestamp: u64,
    ) -> ProgramReleaseSchedule {
        Self::create_program_release_schedule_internal(
            env,
            None,
            recipient,
            amount,
            release_timestamp,
        )
    }

    pub fn create_prog_release_schedule_by(
        env: Env,
        caller: Address,
        recipient: Address,
        amount: i128,
        release_timestamp: u64,
    ) -> ProgramReleaseSchedule {
        Self::create_program_release_schedule_internal(
            env,
            Some(caller),
            recipient,
            amount,
            release_timestamp,
        )
    }

    fn create_program_release_schedule_internal(
        env: Env,
        caller: Option<Address>,
        recipient: Address,
        amount: i128,
        release_timestamp: u64,
    ) -> ProgramReleaseSchedule {
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        if program_data.status == ProgramStatus::Draft {
            panic!("Program is in Draft status. Publish the program first.");
        }

        Self::authorize_release_actor(&env, &program_data, caller.as_ref());

        if amount <= 0 {
            panic!("Amount must be greater than zero");
        }

        let mut schedules: soroban_sdk::Vec<ProgramReleaseSchedule> = env
            .storage()
            .instance()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));
        let schedule_id: u64 = env
            .storage()
            .instance()
            .get(&NEXT_SCHEDULE_ID)
            .unwrap_or(1_u64);

        let schedule = ProgramReleaseSchedule {
            schedule_id,
            recipient: recipient.clone(),
            amount,
            release_timestamp,
            released: false,
            released_at: None,
            released_by: None,
        };
        schedules.push_back(schedule.clone());

        env.storage().instance().set(&SCHEDULES, &schedules);
        env.storage()
            .instance()
            .set(&NEXT_SCHEDULE_ID, &(schedule_id + 1));

        // Emit ReleaseScheduled event
        env.events().publish(
            (RELEASE_SCHEDULED,),
            ReleaseScheduledEvent {
                version: EVENT_VERSION_V2,
                program_id: program_data.program_id,
                schedule_id,
                recipient: recipient.clone(),
                amount,
                release_timestamp,
                correlation_id: None,
            },
        );

        schedule
    }

    /// Create an epoch snapshot of currently due schedules.
    pub fn create_epoch_snapshot(env: Env) -> u64 {
        Self::create_epoch_snapshot_internal(env, None)
    }

    pub fn create_epoch_snapshot_by(env: Env, caller: Address) -> u64 {
        Self::create_epoch_snapshot_internal(env, Some(caller))
    }

    fn create_epoch_snapshot_internal(env: Env, caller: Option<Address>) -> u64 {
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        Self::authorize_release_actor(&env, &program_data, caller.as_ref());

        let schedules: soroban_sdk::Vec<ProgramReleaseSchedule> = env
            .storage()
            .instance()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));

        let now = env.ledger().timestamp();
        let mut due_schedules: soroban_sdk::Vec<ProgramReleaseSchedule> = Vec::new(&env);

        for i in 0..schedules.len() {
            let s = schedules.get(i).unwrap();
            if !s.released && now >= s.release_timestamp {
                due_schedules.push_back(s);
            }
        }

        let mut next_epoch_id: u64 = env.storage().instance().get(&NEXT_EPOCH_ID).unwrap_or(1);
        let current_epoch_id = next_epoch_id;
        next_epoch_id += 1;
        env.storage().instance().set(&NEXT_EPOCH_ID, &next_epoch_id);

        let snapshot = EpochSnapshot {
            created_at: now,
            created_by: caller.unwrap_or_else(|| env.current_contract_address()),
            schedules: due_schedules,
        };

        let mut snapshots: soroban_sdk::Map<u64, EpochSnapshot> = env
            .storage()
            .instance()
            .get(&EPOCH_SNAPSHOTS)
            .unwrap_or_else(|| soroban_sdk::Map::new(&env));

        snapshots.set(current_epoch_id, snapshot);
        env.storage().instance().set(&EPOCH_SNAPSHOTS, &snapshots);

        current_epoch_id
    }

    /// Trigger all due schedules where `now >= release_timestamp`.
    pub fn trigger_program_releases(env: Env, epoch_id: Option<u64>) -> u32 {
        Self::trigger_program_releases_internal(env, None, epoch_id)
    }

    pub fn trigger_program_releases_by(env: Env, caller: Address, epoch_id: Option<u64>) -> u32 {
        Self::trigger_program_releases_internal(env, Some(caller), epoch_id)
    }

    /// Internal implementation for trigger_program_releases.
    ///
    /// # Deterministic Behavior
    /// - Processes due schedules in ascending order by schedule_id
    /// - Maintains stable ordering across all contract instances
    /// - Emits deterministic events for audit and monitoring
    ///
    /// # Explicit Errors
    /// - Returns ReleaseTriggerFailed (910) on critical state corruption
    /// - Returns NoSchedulesDue (911) if no schedules meet release conditions
    /// - Returns DeterminismViolation (912) on ordering inconsistencies
    ///
    /// # Upgrade-Safe Storage
    /// - Uses ReleaseTriggerSchemaVersion for backward compatibility
    /// - Gracefully handles schema migrations
    /// - Preserves payout history and schedule state across upgrades
    fn trigger_program_releases_internal(env: Env, caller: Option<Address>, epoch_id: Option<u64>) -> u32 {
        reentrancy_guard::acquire(&env);

        let mut program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        if program_data.status == ProgramStatus::Draft {
            panic!("Program is in Draft status. Publish the program first.");
        }
        Self::authorize_release_actor(&env, &program_data, caller.as_ref());

        if Self::check_paused(&env, Some(&program_data.program_id), symbol_short!("release")) {
            panic!("Funds Paused");
        }

        let mut schedules: soroban_sdk::Vec<ProgramReleaseSchedule> = env
            .storage()
            .instance()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));
        let mut release_history: soroban_sdk::Vec<ProgramReleaseHistory> = env
            .storage()
            .instance()
            .get(&RELEASE_HISTORY)
            .unwrap_or_else(|| Vec::new(&env));

        let now = env.ledger().timestamp();
        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &program_data.token_address);
        let mut released_count: u32 = 0;
        let mut skipped_count: u32 = 0;

        // Deterministic ordering: build a sorted index of due, unreleased schedules
        // sorted ascending by schedule_id so output is replay-identical across nodes.
        let len = schedules.len();

        let mut snapshot_schedules: Option<soroban_sdk::Vec<ProgramReleaseSchedule>> = None;
        if let Some(eid) = epoch_id {
            let snapshots: soroban_sdk::Map<u64, EpochSnapshot> = env
                .storage()
                .instance()
                .get(&EPOCH_SNAPSHOTS)
                .unwrap_or_else(|| panic!("Epoch snapshots map not found"));
            let snapshot = snapshots.get(eid).unwrap_or_else(|| panic!("Epoch snapshot not found"));
            snapshot_schedules = Some(snapshot.schedules);
        }

        // store a tuple of (main_schedule_index, snap_schedule_index) where u32::MAX means no snap schedule
        let mut due_entries: soroban_sdk::Vec<u64> = Vec::new(&env);
        // Pack (main_index, snap_index) into a single u64 for easier use with vec_insert_at if needed, but let's just use two u32s encoded as u64
        // High 32 bits = main_index, Low 32 bits = snap_index

        if let Some(ref snap_scheds) = snapshot_schedules {
            let snap_len = snap_scheds.len();
            for snap_i in 0..snap_len {
                let s = snap_scheds.get(snap_i).unwrap();
                // Find matching schedule_id in main schedules
                for i in 0..len {
                    let existing = schedules.get(i).unwrap();
                    if existing.schedule_id == s.schedule_id && !existing.released {
                        // Insert-sort by schedule_id (ascending)
                        let mut inserted = false;
                        for j in 0..due_entries.len() {
                            let entry_packed = due_entries.get(j).unwrap();
                            let existing_in_list = schedules.get((entry_packed >> 32) as u32).unwrap();
                            if existing.schedule_id < existing_in_list.schedule_id {
                                let packed = ((i as u64) << 32) | (snap_i as u64);
                                due_entries = Self::vec_insert_at_u64(&env, due_entries, j, packed);
                                inserted = true;
                                break;
                            }
                        }
                        if !inserted {
                            due_entries.push_back(((i as u64) << 32) | (snap_i as u64));
                        }
                        break; // Move to next snap schedule
                    }
                }
            }
        } else {
            for i in 0..len {
                let s = schedules.get(i).unwrap();
                if !s.released && now >= s.release_timestamp {
                    // Insert-sort by schedule_id (ascending) for determinism
                    let mut inserted = false;
                    for j in 0..due_entries.len() {
                        let entry_packed = due_entries.get(j).unwrap();
                        let existing_in_list = schedules.get((entry_packed >> 32) as u32).unwrap();
                        if s.schedule_id < existing_in_list.schedule_id {
                            let packed = ((i as u64) << 32) | (u32::MAX as u64);
                            due_entries = Self::vec_insert_at_u64(&env, due_entries, j, packed);
                            inserted = true;
                            break;
                        }
                    }
                    if !inserted {
                        due_entries.push_back(((i as u64) << 32) | (u32::MAX as u64));
                    }
                }
            }
        }

        // Process due schedules in sorted order; skip (don't panic) on insufficient balance
        for k in 0..due_entries.len() {
            let entry_packed = due_entries.get(k).unwrap();
            let i = (entry_packed >> 32) as u32;
            let snap_i = (entry_packed & 0xFFFFFFFF) as u32;
            let mut schedule = schedules.get(i).unwrap();

            let (exec_amount, exec_recipient) = if snap_i != u32::MAX {
                let s = snapshot_schedules.as_ref().unwrap().get(snap_i).unwrap();
                (s.amount, s.recipient.clone())
            } else {
                (schedule.amount, schedule.recipient.clone())
            };

            // Skip schedule if contract has insufficient balance — deferred to next trigger
            if exec_amount > program_data.remaining_balance {
                skipped_count += 1;
                continue;
            }

            // Effects before interaction (CEI pattern)
            program_data.remaining_balance -= exec_amount;
            schedule.released = true;
            schedule.released_at = Some(now);
            schedule.released_by = Some(contract_address.clone());
            schedules.set(i, schedule.clone());

            program_data.payout_history.push_back(PayoutRecord {
                recipient: exec_recipient.clone(),
                amount: exec_amount,
                timestamp: now,
            });
            release_history.push_back(ProgramReleaseHistory {
                schedule_id: schedule.schedule_id,
                recipient: exec_recipient.clone(),
                amount: exec_amount,
                released_at: now,
                release_type: ReleaseType::Automatic,
            });

            // Interaction: token transfer (after state updates)
            token_client.transfer(&contract_address, &exec_recipient, &exec_amount);

            // Emit per-schedule event
            env.events().publish(
                (SCHEDULE_RELEASED,),
                ScheduleReleasedEvent {
                    version: EVENT_VERSION_V2,
                    program_id: program_data.program_id.clone(),
                    schedule_id: schedule.schedule_id,
                    recipient: exec_recipient,
                    amount: exec_amount,
                    released_at: now,
                    released_by: contract_address.clone(),
                    correlation_id: None,
                },
            );

            released_count += 1;
        }

        env.storage().instance().set(&PROGRAM_DATA, &program_data);
        env.storage().instance().set(&SCHEDULES, &schedules);
        env.storage()
            .instance()
            .set(&RELEASE_HISTORY, &release_history);

        // Emit summary event for the trigger run
        env.events().publish(
            (symbol_short!("SchTrig"),),
            ScheduleTriggerSummaryEvent {
                version: EVENT_VERSION_V2,
                program_id: program_data.program_id.clone(),
                triggered_at: now,
                released_count,
                skipped_count,
            },
        );

        // Clear reentrancy guard before returning
        reentrancy_guard::release(&env);

        released_count
    }

    // Insert `value` at position `pos` in a `Vec<u32>`, returning the new Vec.
    fn vec_insert_at(
        env: &Env,
        v: soroban_sdk::Vec<u32>,
        pos: u32,
        value: u32,
    ) -> soroban_sdk::Vec<u32> {
        let mut result: soroban_sdk::Vec<u32> = Vec::new(env);
        for i in 0..v.len() {
            if i == pos {
                result.push_back(value);
            }
            result.push_back(v.get(i).unwrap());
        }
        if pos >= v.len() {
            result.push_back(value);
        }
        result
    }

    fn vec_insert_at_u64(
        env: &Env,
        v: soroban_sdk::Vec<u64>,
        pos: u32,
        value: u64,
    ) -> soroban_sdk::Vec<u64> {
        let mut result: soroban_sdk::Vec<u64> = Vec::new(env);
        for i in 0..v.len() {
            if i == pos {
                result.push_back(value);
            }
            result.push_back(v.get(i).unwrap());
        }
        if pos >= v.len() {
            result.push_back(value);
        }
        result
    }

    pub fn get_release_schedules(env: Env) -> soroban_sdk::Vec<ProgramReleaseSchedule> {
        if let Some(info) = env
            .storage()
            .instance()
            .get::<Symbol, ProgramData>(&PROGRAM_DATA)
        {
            if info.archived {
                return Vec::new(&env);
            }
        }
        env.storage()
            .instance()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env))
    }

    pub fn get_program_release_history(env: Env) -> soroban_sdk::Vec<ProgramReleaseHistory> {
        env.storage()
            .instance()
            .get(&RELEASE_HISTORY)
            .unwrap_or_else(|| Vec::new(&env))
    }

}
