#[cfg(feature = "contract")]
#[contractimpl]
impl ProgramEscrowContract {
    pub fn configure_dynamic_pricing(
        env: Env,
        config: DynamicPricingConfig,
    ) {
        let admin = Self::require_admin(&env);

        // Validate configuration parameters
        if config.base_fee_bps < 0 || config.base_fee_bps > 10000 {
            panic!("Invalid base fee rate");
        }
        if config.max_fee_bps < config.min_fee_bps {
            panic!("Max fee must be >= min fee");
        }
        if config.max_change_bps < 0 || config.max_change_bps > 10000 {
            panic!("Invalid max change rate");
        }
        if config.smoothing_alpha_bps < 0 || config.smoothing_alpha_bps > 10000 {
            panic!("Invalid smoothing alpha");
        }
        if config.min_update_interval == 0 {
            panic!("Min update interval must be > 0");
        }

        // Initialize pricing state if not exists
        if !env.storage().instance().has(&DataKey::PricingState) {
            let initial_state = PricingState::initial(&env, config.base_fee_bps);
            env.storage().instance().set(&DataKey::PricingState, &initial_state);
        }

        env.storage().instance().set(&DataKey::DynamicPricingConfig, &config);

        env.events().publish(
            (DYNAMIC_PRICING_CONFIG_UPDATED,),
            (
                config.enabled,
                config.base_fee_bps,
                config.max_fee_bps,
                config.min_fee_bps,
                config.max_change_bps,
                config.smoothing_alpha_bps,
                config.min_update_interval,
                admin,
                env.ledger().timestamp(),
            ),
        );
    }

    /// Get current dynamic pricing configuration.
    pub fn get_dynamic_pricing_config(env: Env) -> Option<DynamicPricingConfig> {
        env.storage().instance().get(&DataKey::DynamicPricingConfig)
    }

    /// Get current pricing state.
    pub fn get_pricing_state(env: Env) -> Option<PricingState> {
        env.storage().instance().get(&DataKey::PricingState)
    }

    /// Update demand metrics for dynamic pricing. Admin-only.
    ///
    /// # Arguments
    /// * `tx_count` - Transaction count in current window
    /// * `total_volume` - Total volume in current window
    /// * `unique_users` - Number of unique users
    /// * `avg_tx_size` - Average transaction size
    /// * `growth_rate_bps` - Growth rate vs previous window in basis points
    pub fn update_demand_metrics(
        env: Env,
        tx_count: u64,
        total_volume: i128,
        unique_users: u64,
        avg_tx_size: i128,
        growth_rate_bps: i128,
    ) {
        Self::require_admin(&env);

        let metrics = DemandMetrics {
            tx_count,
            total_volume,
            unique_users,
            avg_tx_size,
            growth_rate_bps,
        };
        env.storage().instance().set(&DataKey::DemandMetrics, &metrics);
    }

    /// Update supply metrics for dynamic pricing. Admin-only.
    ///
    /// # Arguments
    /// * `total_liquidity` - Total liquidity available
    /// * `utilization_bps` - Utilization rate in basis points
    /// * `available_liquidity` - Available liquidity
    /// * `locked_liquidity` - Locked liquidity
    pub fn update_supply_metrics(
        env: Env,
        total_liquidity: i128,
        utilization_bps: i128,
        available_liquidity: i128,
        locked_liquidity: i128,
    ) {
        Self::require_admin(&env);

        let metrics = SupplyMetrics {
            total_liquidity,
            utilization_bps,
            available_liquidity,
            locked_liquidity,
        };
        env.storage().instance().set(&DataKey::SupplyMetrics, &metrics);
    }

    /// Update oracle data for dynamic pricing. Admin-only.
    ///
    /// # Arguments
    /// * `token_price` - Current token price
    /// * `volume_24h` - 24h trading volume
    /// * `market_cap` - Current market cap
    /// * `volatility_bps` - Volatility index in basis points
    /// * `timestamp` - Oracle data timestamp
    /// * `signature` - Optional oracle signature
    pub fn update_oracle_data(
        env: Env,
        token_price: i128,
        volume_24h: i128,
        market_cap: i128,
        volatility_bps: i128,
        timestamp: u64,
        signature: Option<Bytes>,
    ) {
        Self::require_admin(&env);

        let oracle_data = OracleMarketData {
            token_price,
            volume_24h,
            market_cap,
            volatility_bps,
            timestamp,
            signature,
        };

        // Validate oracle data
        PricingEngine::validate_oracle_data(&env, &oracle_data)
            .expect("Invalid oracle data");

        env.storage().instance().set(&DataKey::OracleData, &oracle_data);
    }

    /// Trigger a dynamic price update. Admin-only.
    ///
    /// This function calculates a new fee based on current metrics and
    /// updates the pricing state if the change is within allowed limits.
    ///
    /// # Events
    /// Emits `PriceUpd` with price update details.
    pub fn update_dynamic_price(env: Env) {
        Self::require_admin(&env);

        let config: DynamicPricingConfig = env
            .storage()
            .instance()
            .get(&DataKey::DynamicPricingConfig)
            .expect("Dynamic pricing not configured");

        if !config.enabled {
            panic!("Dynamic pricing is not enabled");
        }

        let state: PricingState = env
            .storage()
            .instance()
            .get(&DataKey::PricingState)
            .expect("Pricing state not initialized");

        // Get metrics if available
        let demand_metrics = env.storage().instance().get(&DataKey::DemandMetrics);
        let supply_metrics = env.storage().instance().get(&DataKey::SupplyMetrics);
        let oracle_data = env.storage().instance().get(&DataKey::OracleData);

        // Calculate new fee
        let calculation = PricingEngine::calculate_fee(
            &env,
            &config,
            &state,
            demand_metrics.as_ref(),
            supply_metrics.as_ref(),
            oracle_data.as_ref(),
        ).expect("Fee calculation failed");

        let previous_fee = state.current_fee_bps;
        let new_fee = calculation.final_fee_bps;

        // Update pricing state
        let mut new_state = state.clone();
        new_state.previous_fee_bps = previous_fee;
        new_state.current_fee_bps = new_fee;
        new_state.ema_fee_bps = calculation.smoothed_fee_bps;
        new_state.last_update = env.ledger().timestamp();
        new_state.update_count += 1;

        if let Some(demand) = demand_metrics {
            new_state.demand_score = (demand.tx_count as i128 * 10).min(10000);
        }

        if let Some(supply) = supply_metrics {
            new_state.supply_score = supply.utilization_bps;
        }

        env.storage().instance().set(&DataKey::PricingState, &new_state);

        // Emit price update event
        env.events().publish(
            (PRICE_UPDATED,),
            PriceUpdateEvent {
                version: EVENT_VERSION_V2,
                previous_fee_bps: previous_fee,
                new_fee_bps: new_fee,
                demand_score: new_state.demand_score,
                supply_score: new_state.supply_score,
                time_decay_factor: new_state.time_decay_factor,
                oracle_price: oracle_data.map(|o| o.token_price),
                timestamp: env.ledger().timestamp(),
                reason: String::from_str(&env, "Scheduled price update"),
            },
        );
    }

    /// Get the current dynamic fee rate.
    ///
    /// Returns the current fee rate in basis points if dynamic pricing is enabled,
    /// otherwise returns None.
    pub fn get_dynamic_fee(env: Env) -> Option<i128> {
        let config: Option<DynamicPricingConfig> = env.storage().instance().get(&DataKey::DynamicPricingConfig);

        if let Some(cfg) = config {
            if cfg.enabled {
                let state: Option<PricingState> = env.storage().instance().get(&DataKey::PricingState);
                state.map(|s| s.current_fee_bps)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Get demand metrics.
    pub fn get_demand_metrics(env: Env) -> Option<DemandMetrics> {
        env.storage().instance().get(&DataKey::DemandMetrics)
    }

    /// Get supply metrics.
    pub fn get_supply_metrics(env: Env) -> Option<SupplyMetrics> {
        env.storage().instance().get(&DataKey::SupplyMetrics)
    }

    /// Get oracle data.
    pub fn get_oracle_data(env: Env) -> Option<OracleMarketData> {
        env.storage().instance().get(&DataKey::OracleData)
    }
}
