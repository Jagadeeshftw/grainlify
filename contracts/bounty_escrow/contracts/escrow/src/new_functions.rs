    // =========================================================================
    // CONDITIONAL REFUND TRIGGERS (Issue: oracle + time-based)
    // =========================================================================

    /// Configure the oracle address for oracle-attested refunds (admin only).
    ///
    /// When `enabled` is true the oracle may call [`oracle_refund`] to
    /// unilaterally refund any locked bounty without admin or depositor auth.
    /// Setting `enabled = false` disables the oracle path without clearing
    /// the stored address.
    ///
    /// Emits [`OracleConfigUpdated`].
    pub fn set_oracle(env: Env, oracle_address: Address, enabled: bool) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();

        let config = OracleConfig {
            oracle_address: oracle_address.clone(),
            enabled,
        };
        env.storage()
            .instance()
            .set(&DataKey::OracleConfig, &config);

        events::emit_oracle_config_updated(
            &env,
            events::OracleConfigUpdated {
                version: EVENT_VERSION_V2,
                oracle_address,
                enabled,
                admin,
                timestamp: env.ledger().timestamp(),
            },
        );
        Ok(())
    }

    /// Get the current oracle configuration, if one has been set.
    pub fn get_oracle_config(env: Env) -> Option<OracleConfig> {
        env.storage().instance().get(&DataKey::OracleConfig)
    }

    /// Oracle-attested refund — only the configured oracle address may call this.
    ///
    /// The oracle unilaterally authorises a refund when an off-chain dispute
    /// is resolved in the depositor's favour.  No admin or depositor auth is
    /// required; the oracle's signature is the sole authorization.
    ///
    /// # Mutual exclusion
    /// Fails with [`Error::FundsNotLocked`] if the escrow has already been
    /// released or refunded via any other path.
    ///
    /// # Double-refund prevention
    /// [`DataKey::OracleRefundUsed`] is set atomically with the state update,
    /// so a second oracle_refund call on the same bounty fails with
    /// [`Error::OracleRefundAlreadyProcessed`].
    ///
    /// # Errors
    /// * `OracleNotConfigured` — no oracle set or oracle disabled
    /// * `CallerNotOracle` — `oracle` arg does not match configured address
    /// * `OracleRefundAlreadyProcessed` — already oracle-refunded
    /// * `BountyNotFound` — bounty does not exist
    /// * `FundsNotLocked` — escrow not in Locked/PartiallyRefunded state
    /// * `ClaimPending` — a pending claim blocks the refund
    pub fn oracle_refund(env: Env, bounty_id: u64, oracle: Address) -> Result<(), Error> {
        // GUARD: acquire reentrancy lock
        reentrancy_guard::acquire(&env);

        if Self::check_paused(&env, symbol_short!("refund")) {
            reentrancy_guard::release(&env);
            return Err(Error::FundsPaused);
        }

        // Load and validate oracle config
        let oracle_cfg: OracleConfig = env
            .storage()
            .instance()
            .get(&DataKey::OracleConfig)
            .ok_or_else(|| {
                reentrancy_guard::release(&env);
                Error::OracleNotConfigured
            })?;

        if !oracle_cfg.enabled {
            reentrancy_guard::release(&env);
            return Err(Error::OracleNotConfigured);
        }

        if oracle != oracle_cfg.oracle_address {
            reentrancy_guard::release(&env);
            return Err(Error::CallerNotOracle);
        }

        // Oracle must authorize itself
        oracle.require_auth();

        // Double-refund prevention: check before any state changes
        if env
            .storage()
            .persistent()
            .has(&DataKey::OracleRefundUsed(bounty_id))
        {
            reentrancy_guard::release(&env);
            return Err(Error::OracleRefundAlreadyProcessed);
        }

        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            reentrancy_guard::release(&env);
            return Err(Error::BountyNotFound);
        }

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

        // Mutual exclusion: only refund if still locked
        if escrow.status != EscrowStatus::Locked
            && escrow.status != EscrowStatus::PartiallyRefunded
        {
            reentrancy_guard::release(&env);
            return Err(Error::FundsNotLocked);
        }

        // Block refund if there is a pending (unclaimed) claim
        if env
            .storage()
            .persistent()
            .has(&DataKey::PendingClaim(bounty_id))
        {
            let claim: ClaimRecord = env
                .storage()
                .persistent()
                .get(&DataKey::PendingClaim(bounty_id))
                .unwrap();
            if !claim.claimed {
                reentrancy_guard::release(&env);
                return Err(Error::ClaimPending);
            }
        }

        let now = env.ledger().timestamp();
        let refund_amount = escrow.remaining_amount;
        let refund_to = escrow.depositor.clone();

        // EFFECTS: update state before external calls (CEI)
        escrow.remaining_amount = 0;
        escrow.status = EscrowStatus::Refunded;
        escrow.refund_history.push_back(RefundRecord {
            amount: refund_amount,
            recipient: refund_to.clone(),
            timestamp: now,
            mode: RefundMode::Full,
            trigger_type: events::RefundTriggerType::OracleAttestation,
        });

        Self::write_escrow(&env, bounty_id, &escrow)?;

        // Mark oracle refund used — prevents double-refund
        env.storage()
            .persistent()
            .set(&DataKey::OracleRefundUsed(bounty_id), &true);

        // INTERACTION: external token transfer is last
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        client.transfer(&env.current_contract_address(), &refund_to, &refund_amount);

        emit_funds_refunded(
            &env,
            FundsRefunded {
                version: EVENT_VERSION_V2,
                bounty_id,
                amount: refund_amount,
                refund_to,
                timestamp: now,
                trigger_type: events::RefundTriggerType::OracleAttestation,
            },
        );

        Self::record_receipt(
            &env,
            CriticalOperationOutcome::Refunded,
            bounty_id,
            refund_amount,
            escrow.depositor,
        );

        multitoken_invariants::assert_after_disbursement(&env);

        // GUARD: release reentrancy lock
        reentrancy_guard::release(&env);
        Ok(())
    }

    /// Permissionless time-based auto-refund — anyone may call once the
    /// escrow deadline has passed.
    ///
    /// Unlike [`refund`], this function requires **no authorization** from
    /// the admin or depositor.  Any address (including a third-party keeper)
    /// may trigger the refund after `now >= deadline`.  Funds always return
    /// to the original depositor.
    ///
    /// # Mutual exclusion
    /// Fails with [`Error::FundsNotLocked`] if the escrow has already been
    /// released or refunded via any other path.
    ///
    /// # Errors
    /// * `FundsPaused` — refund operations are paused
    /// * `BountyNotFound` — bounty does not exist
    /// * `FundsNotLocked` — escrow not in Locked/PartiallyRefunded state
    /// * `DeadlineNotPassed` — called before the deadline
    /// * `ClaimPending` — a pending claim blocks the refund
    pub fn auto_refund(env: Env, bounty_id: u64) -> Result<(), Error> {
        // GUARD: acquire reentrancy lock
        reentrancy_guard::acquire(&env);

        if Self::check_paused(&env, symbol_short!("refund")) {
            reentrancy_guard::release(&env);
            return Err(Error::FundsPaused);
        }

        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            reentrancy_guard::release(&env);
            return Err(Error::BountyNotFound);
        }

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

        // Mutual exclusion: only refund if still locked
        if escrow.status != EscrowStatus::Locked
            && escrow.status != EscrowStatus::PartiallyRefunded
        {
            reentrancy_guard::release(&env);
            return Err(Error::FundsNotLocked);
        }

        let now = env.ledger().timestamp();

        // Deadline must have passed
        if now < escrow.deadline {
            reentrancy_guard::release(&env);
            return Err(Error::DeadlineNotPassed);
        }

        // Block refund if there is a pending (unclaimed) claim
        if env
            .storage()
            .persistent()
            .has(&DataKey::PendingClaim(bounty_id))
        {
            let claim: ClaimRecord = env
                .storage()
                .persistent()
                .get(&DataKey::PendingClaim(bounty_id))
                .unwrap();
            if !claim.claimed {
                reentrancy_guard::release(&env);
                return Err(Error::ClaimPending);
            }
        }

        let refund_amount = escrow.remaining_amount;
        let refund_to = escrow.depositor.clone();

        // EFFECTS: update state before external calls (CEI)
        escrow.remaining_amount = 0;
        escrow.status = EscrowStatus::Refunded;
        escrow.refund_history.push_back(RefundRecord {
            amount: refund_amount,
            recipient: refund_to.clone(),
            timestamp: now,
            mode: RefundMode::Full,
            trigger_type: events::RefundTriggerType::DeadlineExpired,
        });

        Self::write_escrow(&env, bounty_id, &escrow)?;

        // INTERACTION: external token transfer is last
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        client.transfer(&env.current_contract_address(), &refund_to, &refund_amount);

        emit_funds_refunded(
            &env,
            FundsRefunded {
                version: EVENT_VERSION_V2,
                bounty_id,
                amount: refund_amount,
                refund_to,
                timestamp: now,
                trigger_type: events::RefundTriggerType::DeadlineExpired,
            },
        );

        Self::record_receipt(
            &env,
            CriticalOperationOutcome::Refunded,
            bounty_id,
            refund_amount,
            escrow.depositor,
        );

        multitoken_invariants::assert_after_disbursement(&env);

        // GUARD: release reentrancy lock
        reentrancy_guard::release(&env);
        Ok(())
    }
