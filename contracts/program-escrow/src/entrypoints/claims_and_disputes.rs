#[cfg(feature = "contract")]
#[contractimpl]
impl ProgramEscrowContract {
    pub fn get_all_prog_release_schedules(env: Env) -> soroban_sdk::Vec<ProgramReleaseSchedule> {
        Self::get_release_schedules(env)
    }

    pub fn get_pending_program_schedules(env: Env) -> soroban_sdk::Vec<ProgramReleaseSchedule> {
        Self::get_pending_schedules(env)
    }

    pub fn get_due_program_schedules(env: Env) -> soroban_sdk::Vec<ProgramReleaseSchedule> {
        Self::get_due_schedules(env)
    }

    pub fn release_program_schedule_manual(env: Env, schedule_id: u64) {
        Self::release_program_schedule_manual_internal(env, None, schedule_id)
    }

    pub fn release_prog_schedule_manual_by(env: Env, caller: Address, schedule_id: u64) {
        Self::release_program_schedule_manual_internal(env, Some(caller), schedule_id)
    }

    fn release_program_schedule_manual_internal(
        env: Env,
        caller: Option<Address>,
        schedule_id: u64,
    ) {
        let mut schedules = Self::get_release_schedules(env.clone());
        let program_data = Self::get_program_info(env.clone());

        if program_data.status == ProgramStatus::Draft {
            panic!("Program is in Draft status. Publish the program first.");
        }

        let caller = Self::authorize_release_actor(&env, &program_data, caller.as_ref());
        let now = env.ledger().timestamp();
        let mut released_schedule: Option<ProgramReleaseSchedule> = None;

        let mut found = false;
        for i in 0..schedules.len() {
            let mut s = schedules.get(i).unwrap();
            if s.schedule_id == schedule_id {
                if s.released {
                    panic!("Already released");
                }

                // Per-window spending limit check before transfer
                Self::enforce_spending_window(&env, &program_data.program_id, s.amount);

                // Transfer funds
                let token_client = token::Client::new(&env, &program_data.token_address);
                token_client.transfer(&env.current_contract_address(), &s.recipient, &s.amount);

                s.released = true;
                s.released_at = Some(now);
                s.released_by = Some(caller.clone());
                released_schedule = Some(s.clone());
                schedules.set(i, s);
                found = true;
                break;
            }
        }

        if !found {
            panic!("Schedule not found");
        }

        env.storage().instance().set(&SCHEDULES, &schedules);

        // Write to release history
        if let Some(s) = released_schedule {
            let mut updated_program_data = program_data.clone();
            updated_program_data.remaining_balance -= s.amount;
            env.storage()
                .instance()
                .set(&PROGRAM_DATA, &updated_program_data);

            let mut history: soroban_sdk::Vec<ProgramReleaseHistory> = env
                .storage()
                .instance()
                .get(&RELEASE_HISTORY)
                .unwrap_or_else(|| Vec::new(&env));
            history.push_back(ProgramReleaseHistory {
                schedule_id: s.schedule_id,
                recipient: s.recipient,
                amount: s.amount,
                released_at: now,
                release_type: ReleaseType::Manual,
            });
            env.storage().instance().set(&RELEASE_HISTORY, &history);
        }
    }

    pub fn release_prog_schedule_automatic(env: Env, schedule_id: u64) {
        let mut schedules = Self::get_release_schedules(env.clone());
        let program_data = Self::get_program_info(env.clone());
        let now = env.ledger().timestamp();
        let mut released_schedule: Option<ProgramReleaseSchedule> = None;

        let mut found = false;
        for i in 0..schedules.len() {
            let mut s = schedules.get(i).unwrap();
            if s.schedule_id == schedule_id {
                if s.released {
                    panic!("Already released");
                }
                if now < s.release_timestamp {
                    panic!("Not yet due");
                }

                // Per-window spending limit check before transfer
                Self::enforce_spending_window(&env, &program_data.program_id, s.amount);

                // Transfer funds
                let token_client = token::Client::new(&env, &program_data.token_address);
                token_client.transfer(&env.current_contract_address(), &s.recipient, &s.amount);

                s.released = true;
                s.released_at = Some(now);
                s.released_by = Some(env.current_contract_address());
                released_schedule = Some(s.clone());
                schedules.set(i, s);
                found = true;
                break;
            }
        }

        if !found {
            panic!("Schedule not found");
        }

        env.storage().instance().set(&SCHEDULES, &schedules);

        // Write to release history
        if let Some(s) = released_schedule {
            let mut updated_program_data = program_data.clone();
            updated_program_data.remaining_balance -= s.amount;
            env.storage()
                .instance()
                .set(&PROGRAM_DATA, &updated_program_data);

            let mut history: soroban_sdk::Vec<ProgramReleaseHistory> = env
                .storage()
                .instance()
                .get(&RELEASE_HISTORY)
                .unwrap_or_else(|| Vec::new(&env));
            history.push_back(ProgramReleaseHistory {
                schedule_id: s.schedule_id,
                recipient: s.recipient,
                amount: s.amount,
                released_at: now,
                release_type: ReleaseType::Automatic,
            });
            env.storage().instance().set(&RELEASE_HISTORY, &history);
        }
    }

    /// Reserve funds for a recipient-controlled claim.
    ///
    /// This is treated as part of the release path because it authorizes
    /// a payout claim against escrowed program funds.
    pub fn create_pending_claim(
        env: Env,
        program_id: String,
        recipient: Address,
        amount: i128,
        claim_deadline: u64,
    ) -> u64 {
        if Self::check_paused(&env, Some(&program_id), symbol_short!("release")) {
            panic!("Funds Paused");
        }
        claim_period::create_pending_claim(&env, &program_id, &recipient, amount, claim_deadline)
    }

    /// Execute a previously approved claim and transfer its reserved funds.
    ///
    /// Claims are part of the release path, so `release_paused` blocks them.
    pub fn execute_claim(env: Env, program_id: String, claim_id: u64, recipient: Address) {
        if Self::check_paused(&env, Some(&program_id), symbol_short!("release")) {
            panic!("Funds Paused");
        }
        claim_period::execute_claim(&env, &program_id, claim_id, &recipient)
    }

    /// Cancel a pending claim and return its reserved amount to escrow.
    ///
    /// Claim cancellation is a refund-path operation, so `refund_paused`
    /// blocks it independently of lock and release operations.
    pub fn cancel_claim(env: Env, program_id: String, claim_id: u64, admin: Address) {
        if Self::check_paused(&env, Some(&program_id), symbol_short!("refund")) {
            panic!("Funds Paused");
        }
        claim_period::cancel_claim(&env, &program_id, claim_id, &admin)
    }

    /// Retrieve a stored claim record by program and claim id.
    pub fn get_claim(env: Env, program_id: String, claim_id: u64) -> claim_period::ClaimRecord {
        claim_period::get_claim(&env, &program_id, claim_id)
    }

    /// Set the default claim window used by off-chain workflows.
    pub fn set_claim_window(env: Env, admin: Address, window_seconds: u64) {
        claim_period::set_claim_window(&env, &admin, window_seconds)
    }

    /// Return the configured default claim window duration in seconds.
    pub fn get_claim_window(env: Env) -> u64 {
        claim_period::get_claim_window(&env)
    }

    // ========================================================================
    // Dispute Resolution
    // ========================================================================



    /// Open a dispute on the program, blocking all payouts until resolved.
    ///
    /// # Authorization
    /// Caller must be the contract admin.
    ///
    /// # Errors
    /// Panics if:
    /// - Contract is not initialized (no admin set).
    /// - A dispute is already open (`DisputeState::Open`).
    ///
    /// # Events
    /// Emits `DspOpen` with [`DisputeOpenedEvent`].
    pub fn open_dispute(env: Env, reason: String) -> DisputeRecord {
        let admin = Self::require_admin(&env);

        // Only one active dispute at a time
        if Self::dispute_state(&env) == DisputeState::Open {
            panic!("Dispute already open");
        }

        let now = env.ledger().timestamp();
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        let record = DisputeRecord {
            raised_by: admin.clone(),
            reason: reason.clone(),
            opened_at: now,
            state: DisputeState::Open,
            resolved_by: None,
            resolved_at: None,
            resolution_notes: None,
        };

        env.storage().instance().set(&DataKey::Dispute, &record);

        env.events().publish(
            (DISPUTE_OPENED,),
            DisputeOpenedEvent {
                version: EVENT_VERSION_V2,
                program_id: program_data.program_id,
                raised_by: admin,
                reason,
                opened_at: now,
            },
        );

        record
    }

    /// Resolve an open dispute, unblocking payouts.
    ///
    /// # Authorization
    /// Caller must be the contract admin.
    ///
    /// # Errors
    /// Panics if:
    /// - Contract is not initialized (no admin set).
    /// - No dispute is currently open.
    ///
    /// # Events
    /// Emits `DspRslv` with [`DisputeResolvedEvent`].
    pub fn resolve_dispute(env: Env, resolution_notes: String) -> DisputeRecord {
        let admin = Self::require_admin(&env);

        let mut record: DisputeRecord = env
            .storage()
            .instance()
            .get(&DataKey::Dispute)
            .unwrap_or_else(|| panic!("No dispute found"));

        if record.state != DisputeState::Open {
            panic!("No open dispute to resolve");
        }

        let now = env.ledger().timestamp();
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        record.state = DisputeState::Resolved;
        record.resolved_by = Some(admin.clone());
        record.resolved_at = Some(now);
        record.resolution_notes = Some(resolution_notes.clone());

        env.storage().instance().set(&DataKey::Dispute, &record);

        env.events().publish(
            (DISPUTE_RESOLVED,),
            DisputeResolvedEvent {
                version: EVENT_VERSION_V2,
                program_id: program_data.program_id,
                resolved_by: admin,
                resolution_notes,
                resolved_at: now,
            },
        );

        record
    }

    /// Return the current dispute record, if any.
    ///
    /// Returns `None` when no dispute has ever been opened.
    pub fn get_dispute(env: Env) -> Option<DisputeRecord> {
        env.storage().instance().get(&DataKey::Dispute)
    }

    /// Get reputation metrics for the current program.
    ///
    /// Computes reputation from schedules, payout **amounts**, and locked funds.
    /// `overall_score_bps` is value-weighted; dust-sized payouts cannot cheaply max the score
    /// while most funds stay locked. See `docs/program-escrow-reputation-gaming.md`.
    /// Returns zero `overall_score_bps` if any releases are overdue (missed milestone penalty).
    pub fn get_program_reputation(env: Env) -> ProgramReputation {
        let program_data: Option<ProgramData> = env.storage().instance().get(&PROGRAM_DATA);

        if program_data.is_none() {
            // Return zero reputation for uninitialized program
            return ProgramReputation {
                total_payouts: 0,
                qualified_payout_count: 0,
                total_scheduled: 0,
                completed_releases: 0,
                pending_releases: 0,
                overdue_releases: 0,
                dispute_count: 0,
                refund_count: 0,
                total_funds_locked: 0,
                total_funds_distributed: 0,
                completion_rate_bps: 10_000,
                payout_fulfillment_rate_bps: 10_000,
                overall_score_bps: 10_000,
            };
        }

        let program_data = program_data.unwrap();
        let schedules: soroban_sdk::Vec<ProgramReleaseSchedule> = env
            .storage()
            .instance()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));

        let now = env.ledger().timestamp();

        // Count schedule states
        let mut total_scheduled: u32 = 0;
        let mut completed_releases: u32 = 0;
        let mut pending_releases: u32 = 0;
        let mut overdue_releases: u32 = 0;

        for schedule in schedules.iter() {
            total_scheduled = total_scheduled.saturating_add(1);
            if schedule.released {
                completed_releases = completed_releases.saturating_add(1);
            } else {
                // Not yet released
                pending_releases = pending_releases.saturating_add(1);
                // Check if also overdue (past deadline but not released)
                if schedule.release_timestamp <= now {
                    overdue_releases = overdue_releases.saturating_add(1);
                }
            }
        }

        // Compute distributed funds and qualifying activity from payout history
        let mut total_funds_distributed: i128 = 0;
        let mut qualified_payout_count: u32 = 0;
        for payout in program_data.payout_history.iter() {
            total_funds_distributed =
                total_funds_distributed.saturating_add(payout.amount);
            if payout.amount >= reputation::REPUTATION_MIN_QUALIFYING_PAYOUT_AMOUNT {
                qualified_payout_count = qualified_payout_count.saturating_add(1);
            }
        }

        let total_payouts = program_data.payout_history.len() as u32;
        let total_funds_locked = program_data.total_funds;

        let completion_rate_bps =
            reputation::completion_rate_bps(completed_releases, total_scheduled);

        let payout_fulfillment_rate_bps =
            reputation::payout_fulfillment_rate_bps(total_funds_distributed, total_funds_locked);

        let overall_score_bps = reputation::overall_score_bps(
            completion_rate_bps,
            payout_fulfillment_rate_bps,
            overdue_releases,
        );

        ProgramReputation {
            total_payouts,
            qualified_payout_count,
            total_scheduled,
            completed_releases,
            pending_releases,
            overdue_releases,
            dispute_count: 0,
            refund_count: 0,
            total_funds_locked,
            total_funds_distributed,
            completion_rate_bps,
            payout_fulfillment_rate_bps,
            overall_score_bps,
        }
    }

}
