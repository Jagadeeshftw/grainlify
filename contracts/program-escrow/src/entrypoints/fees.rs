#[cfg(feature = "contract")]
#[contractimpl]
impl ProgramEscrowContract {
    fn calculate_fee(amount: i128, fee_rate: i128) -> i128 {
        if fee_rate == 0 || amount == 0 {
            return 0;
        }
        let numerator = amount
            .checked_mul(fee_rate)
            .and_then(|n| n.checked_add(BASIS_POINTS - 1))
            .unwrap_or_else(|| panic!("Fee calculation overflow"));
        numerator / BASIS_POINTS
    }

    /// Percentage + fixed fee, capped to `amount`.
    fn combined_fee_amount(amount: i128, rate_bps: i128, fixed: i128, fee_enabled: bool) -> i128 {
        if !fee_enabled || amount <= 0 || fixed < 0 {
            return 0;
        }
        let pct = Self::calculate_fee(amount, rate_bps);
        pct.saturating_add(fixed).min(amount).max(0)
    }

    /// Return `true` if `payout_type` has a fee waiver set in `fee_waivers`.
    ///
    /// Matches on the PayoutType variant — `Batch(u32)` waives all batch payouts
    /// regardless of the batch-index payload.
    ///
    /// Time complexity: O(1).  Space complexity: O(1).
    fn is_fee_waived(fee_waivers: u32, payout_type: &PayoutType) -> bool {
        let bit = match payout_type {
            PayoutType::Single => FEE_WAIVER_SINGLE,
            PayoutType::Batch(_) => FEE_WAIVER_BATCH,
        };
        fee_waivers & bit != 0
    }

    fn emit_fee_collected(
        env: &Env,
        operation: Symbol,
        fee_amount: i128,
        fee_rate_bps: i128,
        fee_fixed: i128,
        recipient: Address,
    ) {
        if fee_amount <= 0 {
            return;
        }
        env.events().publish(
            (FEE_COLLECTED,),
            FeeCollectedEvent {
                version: EVENT_VERSION_V2,
                operation,
                fee_amount,
                fee_rate_bps,
                fee_fixed,
                recipient,
                timestamp: env.ledger().timestamp(),
            },
        );
    }

    // ── Insurance reserve helpers ────────────────────────────────────────────

    /// Split `total_fee` into `(reserve_share, recipient_share)` using ceiling
    /// division for the reserve so no dust is lost.
    ///
    /// Invariant: `reserve_share + recipient_share == total_fee`.
    fn split_fee_for_reserve(total_fee: i128, insurance_reserve_bps: u32) -> (i128, i128) {
        insurance_reserve::split_fee_for_reserve(total_fee, insurance_reserve_bps)
    }

    /// Accrue `amount` into the on-chain insurance reserve.
    fn accrue_insurance_reserve(env: &Env, amount: i128) {
        insurance_reserve::accrue_insurance_reserve(env, amount);
    }

    /// Read the current insurance reserve balance (token units).
    pub fn get_insurance_reserve_balance(env: Env) -> i128 {
        insurance_reserve::get_insurance_reserve_balance(&env)
    }

    /// Withdraw the full (or partial) insurance reserve to `target` (admin-only).
    ///
    /// Authorization level mirrors `emergency_withdraw`: the contract admin must
    /// sign.  The contract must **not** need to be paused — reserve withdrawals
    /// are an independent admin operation to avoid mixing operational and
    /// financial-hygiene concerns.
    ///
    /// Emits `InsuranceReserveWithdrawnEvent` for audit purposes.
    pub fn withdraw_insurance_reserve(env: Env, target: Address, amount: i128) {
        if !env.storage().instance().has(&DataKey::Admin) {
            panic!("Not initialized");
        }
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let (balance_before, balance_after) =
            match insurance_reserve::debit_insurance_reserve(&env, amount) {
                Ok(res) => res,
                Err(e) => panic_with_error!(&env, &e),
            };

        // Determine the token to use from the legacy PROGRAM_DATA or any registered program.
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));
        let token_client = token::Client::new(&env, &program_data.token_address);
        token_client.transfer(&env.current_contract_address(), &target, &amount);

        env.events().publish(
            (INSURANCE_RESERVE_WITHDRAWN,),
            InsuranceReserveWithdrawnEvent {
                version: EVENT_VERSION_V2,
                admin,
                target,
                amount,
                balance_before,
                balance_after,
                timestamp: env.ledger().timestamp(),
            },
        );
    }

    /// Get fee configuration (internal helper)
    fn get_fee_config_internal(env: &Env) -> FeeConfig {
        env.storage()
            .instance()
            .get(&FEE_CONFIG)
            .unwrap_or_else(|| FeeConfig {
                lock_fee_rate: 0,
                payout_fee_rate: 0,
                lock_fixed_fee: 0,
                payout_fixed_fee: 0,
                fee_recipient: env.current_contract_address(),
                fee_enabled: false,
                fee_waivers: 0,
                insurance_reserve_bps: 0,
            })
    }

    /// Read fee configuration (view).
    pub fn get_fee_config(env: Env) -> FeeConfig {
        Self::get_fee_config_internal(&env)
    }

    /// Update fee parameters (admin only). `None` leaves a field unchanged.
    ///
    /// # `insurance_reserve_bps`
    /// When set to a non-zero value, each subsequent fee collection will split the
    /// collected fee: a `insurance_reserve_bps / BASIS_POINTS` share is added to
    /// the on-chain `InsuranceReserve` balance (query via `get_insurance_reserve_balance`)
    /// and the remainder is forwarded to `fee_recipient` as before.
    ///
    /// Validation rules (checked after all `Some` fields are merged):
    /// - `insurance_reserve_bps` must not exceed `MAX_FEE_RATE` (1 000, i.e. 10 %).
    pub fn update_fee_config(
        env: Env,
        lock_fee_rate: Option<i128>,
        payout_fee_rate: Option<i128>,
        lock_fixed_fee: Option<i128>,
        payout_fixed_fee: Option<i128>,
        fee_recipient: Option<Address>,
        fee_enabled: Option<bool>,
        insurance_reserve_bps: Option<u32>,
    ) {
        Self::require_admin(&env);
        let mut cfg = Self::get_fee_config_internal(&env);

        if let Some(r) = lock_fee_rate {
            if r > MAX_FEE_RATE {
                panic_with_error!(&env, &ContractError::InvalidFeeRate);
            }
            cfg.lock_fee_rate = r;
        }
        if let Some(r) = payout_fee_rate {
            if r > MAX_FEE_RATE {
                panic_with_error!(&env, &ContractError::InvalidFeeRate);
            }
            cfg.payout_fee_rate = r;
        }
        if let Some(f) = lock_fixed_fee {
            if f < 0 {
                panic!("Invalid lock fixed fee");
            }
            cfg.lock_fixed_fee = f;
        }
        if let Some(f) = payout_fixed_fee {
            if f < 0 {
                panic!("Invalid payout fixed fee");
            }
            cfg.payout_fixed_fee = f;
        }
        if let Some(a) = fee_recipient {
            cfg.fee_recipient = a;
        }
        if let Some(e) = fee_enabled {
            cfg.fee_enabled = e;
        }
        if let Some(bps) = insurance_reserve_bps {
            if bps as i128 > MAX_FEE_RATE {
                panic_with_error!(&env, &ContractError::InvalidInsuranceReserveBps);
            }
            cfg.insurance_reserve_bps = bps;
        }
        env.storage().instance().set(&FEE_CONFIG, &cfg);
    }

}
