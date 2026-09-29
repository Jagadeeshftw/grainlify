#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BatchItemStatus {
    /// Item has not been processed yet
    Pending,
    /// Item was processed successfully
    Success,
    /// Item processing failed
    Failed,
    /// Item was rolled back after initial success
    RolledBack,
}

/// Status of an individual item within a batch, including amount and recipient.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchItem {
    /// Index in the original batch
    pub index: u32,
    /// Recipient address
    pub recipient: Address,
    /// Amount to transfer
    pub amount: i128,
    /// Current status of this item
    pub status: BatchItemStatus,
    /// Error code if failed (0 if no error)
    pub error_code: u32,
    /// Number of retry attempts
    pub retry_count: u32,
}

/// State of a batch operation for recovery purposes.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchRecoveryState {
    /// Unique identifier for this batch operation
    pub batch_id: u64,
    /// Program ID this batch belongs to
    pub program_id: String,
    /// Original balance before batch started
    pub original_balance: i128,
    /// Total amount attempted in batch
    pub total_amount: i128,
    /// Amount successfully transferred
    pub successful_amount: i128,
    /// Number of items in batch
    pub item_count: u32,
    /// Items and their statuses
    pub items: soroban_sdk::Vec<BatchItem>,
    /// Timestamp when batch was initiated
    pub started_at: u64,
    /// Timestamp when batch completed (success or failure)
    pub completed_at: Option<u64>,
    /// Whether this batch is pending recovery
    pub pending_recovery: bool,
    /// Authorized key that initiated the batch
    pub authorized_key: Address,
}

/// Configuration for batch recovery behavior.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchRecoveryConfig {
    /// Maximum number of retry attempts for failed items
    pub max_retries: u32,
    /// Whether to automatically retry on failure
    pub auto_retry: bool,
    /// Maximum items per batch
    pub max_batch_size: u32,
    /// Enable rollback capability
    pub rollback_enabled: bool,
    /// Timeout in seconds after which pending recovery expires
    pub recovery_timeout_secs: u64,
}

impl BatchRecoveryConfig {
    /// Default configuration with sensible limits.
    pub fn default() -> Self {
        BatchRecoveryConfig {
            max_retries: 3,
            auto_retry: false,
            max_batch_size: 100,
            rollback_enabled: true,
            recovery_timeout_secs: 86400, // 24 hours
        }
    }

    /// Validate configuration values.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.max_retries > 10 {
            return Err("Max retries cannot exceed 10");
        }
        if self.max_batch_size == 0 || self.max_batch_size > 500 {
            return Err("Batch size must be between 1 and 500");
        }
        if self.recovery_timeout_secs < 3600 || self.recovery_timeout_secs > 604800 {
            return Err("Recovery timeout must be between 1 hour and 7 days");
        }
        Ok(())
    }
}

/// Storage keys for batch recovery data.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BatchRecoveryKey {
    /// Current batch recovery configuration
    Config,
    /// Next batch ID counter
    NextBatchId,
    /// Active batch recovery state by batch_id
    ActiveBatch(u64),
    /// List of pending recovery batch IDs
    PendingRecoveries,
    /// Recovery history (last N batches)
    RecoveryHistory,
}

/// Error codes for batch recovery operations.
pub const ERR_BATCH_NOT_FOUND: u32 = 3001;
pub const ERR_BATCH_ALREADY_COMPLETE: u32 = 3002;
pub const ERR_BATCH_NOT_RECOVERABLE: u32 = 3003;
pub const ERR_UNAUTHORIZED_RECOVERY: u32 = 3004;
pub const ERR_BATCH_SIZE_EXCEEDED: u32 = 3005;
pub const ERR_RECOVERY_EXPIRED: u32 = 3006;
pub const ERR_ROLLBACK_DISABLED: u32 = 3007;
pub const ERR_NO_FAILED_ITEMS: u32 = 3008;
pub const ERR_NO_SUCCESSFUL_ITEMS: u32 = 3009;
pub const ERR_INVALID_BATCH_CONFIG: u32 = 3010;
/// The stored batch state failed an invariant check. This is a caller-visible
/// error, not a host trap, so clients can retry or escalate safely.
pub const ERR_BATCH_INTEGRITY: u32 = 3011;

// ─────────────────────────────────────────────────────────
// Batch Recovery Configuration
// ─────────────────────────────────────────────────────────

/// Get the current batch recovery configuration.
pub fn get_batch_recovery_config(env: &Env) -> BatchRecoveryConfig {
    env.storage()
        .persistent()
        .get(&BatchRecoveryKey::Config)
        .unwrap_or(BatchRecoveryConfig::default())
}

/// Set the batch recovery configuration.
///
/// # Security
/// Caller must enforce admin authorization before calling this function.
pub fn set_batch_recovery_config(env: &Env, config: BatchRecoveryConfig) -> Result<(), u32> {
    config.validate().map_err(|_| ERR_INVALID_BATCH_CONFIG)?;
    env.storage()
        .persistent()
        .set(&BatchRecoveryKey::Config, &config);
    emit_batch_event(env, symbol_short!("br_cfg"), 0, 0);
    Ok(())
}

/// Get the next batch ID and increment the counter.
fn get_next_batch_id(env: &Env) -> u64 {
    let next_id: u64 = env
        .storage()
        .persistent()
        .get(&BatchRecoveryKey::NextBatchId)
        .unwrap_or(1);
    env.storage()
        .persistent()
        .set(&BatchRecoveryKey::NextBatchId, &(next_id + 1));
    next_id
}

// ─────────────────────────────────────────────────────────
// Batch State Storage
// ─────────────────────────────────────────────────────────

/// Store batch state before execution begins.
///
/// Creates a checkpoint that can be used for recovery if the batch
/// fails partway through execution.
///
/// # Arguments
/// * `env` - Soroban environment
/// * `program_id` - Program identifier
/// * `recipients` - Vector of recipient addresses
/// * `amounts` - Vector of amounts to transfer
/// * `original_balance` - Balance before batch starts
/// * `authorized_key` - Address authorized to perform recovery
///
/// # Returns
/// The batch ID assigned to this operation
///
/// # Security
/// Caller must verify authorization before storing batch state.
pub fn store_batch_state(
    env: &Env,
    program_id: String,
    recipients: soroban_sdk::Vec<Address>,
    amounts: soroban_sdk::Vec<i128>,
    original_balance: i128,
    authorized_key: Address,
) -> Result<u64, u32> {
    let config = get_batch_recovery_config(env);

    // Validate batch size
    if recipients.len() > config.max_batch_size {
        return Err(ERR_BATCH_SIZE_EXCEEDED);
    }

    let batch_id = get_next_batch_id(env);
    let now = env.ledger().timestamp();

    // Calculate total amount
    let mut total_amount: i128 = 0;
    let mut items = soroban_sdk::Vec::new(env);

    for i in 0..recipients.len() {
        let recipient = recipients.get(i).unwrap();
        let amount = amounts.get(i).unwrap();
        total_amount = crate::token_math::safe_add(total_amount, amount);

        items.push_back(BatchItem {
            index: i as u32,
            recipient,
            amount,
            status: BatchItemStatus::Pending,
            error_code: 0,
            retry_count: 0,
        });
    }

    let state = BatchRecoveryState {
        batch_id,
        program_id,
        original_balance,
        total_amount,
        successful_amount: 0,
        item_count: recipients.len() as u32,
        items,
        started_at: now,
        completed_at: None,
        pending_recovery: false,
        authorized_key,
    };

    // Store the state
    env.storage()
        .persistent()
        .set(&BatchRecoveryKey::ActiveBatch(batch_id), &state);

    // Add to pending recoveries list
    add_to_pending_recoveries(env, batch_id);

    emit_batch_event(
        env,
        symbol_short!("br_start"),
        batch_id,
        recipients.len() as u32,
    );

    Ok(batch_id)
}

/// Retrieve batch state for recovery.
///
/// # Arguments
/// * `env` - Soroban environment
/// * `batch_id` - The batch identifier
///
/// # Returns
/// The batch recovery state, or None if not found
pub fn get_batch_state(env: &Env, batch_id: u64) -> Option<BatchRecoveryState> {
    env.storage()
        .persistent()
        .get(&BatchRecoveryKey::ActiveBatch(batch_id))
}

/// Update batch state during execution.
///
/// # Security
/// Should only be called during batch execution or recovery.
pub fn update_batch_state(env: &Env, state: &BatchRecoveryState) {
    env.storage()
        .persistent()
        .set(&BatchRecoveryKey::ActiveBatch(state.batch_id), state);
}

/// Clear batch state after successful completion.
///
/// Removes the batch from active storage and pending list.
///
/// # Security
/// Caller must verify authorization before clearing state.
pub fn clear_batch_state(env: &Env, batch_id: u64) {
    // Remove from active batches
    env.storage()
        .persistent()
        .remove(&BatchRecoveryKey::ActiveBatch(batch_id));

    // Remove from pending recoveries
    remove_from_pending_recoveries(env, batch_id);

    emit_batch_event(env, symbol_short!("br_clear"), batch_id, 0);
}

/// Add batch ID to pending recoveries list.
fn add_to_pending_recoveries(env: &Env, batch_id: u64) {
    let mut pending: soroban_sdk::Vec<u64> = env
        .storage()
        .persistent()
        .get(&BatchRecoveryKey::PendingRecoveries)
        .unwrap_or(soroban_sdk::Vec::new(env));

    pending.push_back(batch_id);
    env.storage()
        .persistent()
        .set(&BatchRecoveryKey::PendingRecoveries, &pending);
}

/// Remove batch ID from pending recoveries list.
fn remove_from_pending_recoveries(env: &Env, batch_id: u64) {
    let pending: soroban_sdk::Vec<u64> = env
        .storage()
        .persistent()
        .get(&BatchRecoveryKey::PendingRecoveries)
        .unwrap_or(soroban_sdk::Vec::new(env));

    let mut new_pending = soroban_sdk::Vec::new(env);
    for id in pending.iter() {
        if id != batch_id {
            new_pending.push_back(id);
        }
    }
    env.storage()
        .persistent()
        .set(&BatchRecoveryKey::PendingRecoveries, &new_pending);
}

/// Get all pending recovery batch IDs.
pub fn get_pending_recoveries(env: &Env) -> soroban_sdk::Vec<u64> {
    env.storage()
        .persistent()
        .get(&BatchRecoveryKey::PendingRecoveries)
        .unwrap_or(soroban_sdk::Vec::new(env))
}

// ─────────────────────────────────────────────────────────
// Batch Item Status Updates
// ─────────────────────────────────────────────────────────

/// Mark a batch item as successful.
///
/// Updates the item status and increments the successful amount.
pub fn mark_item_success(env: &Env, batch_id: u64, item_index: u32) -> Result<(), u32> {
    let mut state = get_batch_state(env, batch_id).ok_or(ERR_BATCH_NOT_FOUND)?;

    if item_index >= state.item_count {
        return Err(ERR_BATCH_NOT_FOUND);
    }

    let mut item = state.items.get(item_index).unwrap();
    item.status = BatchItemStatus::Success;
    state.successful_amount = crate::token_math::safe_add(state.successful_amount, item.amount);
    state.items.set(item_index, item);

    update_batch_state(env, &state);
    Ok(())
}

/// Mark a batch item as failed.
///
/// Updates the item status with error code.
pub fn mark_item_failed(
    env: &Env,
    batch_id: u64,
    item_index: u32,
    error_code: u32,
) -> Result<(), u32> {
    let mut state = get_batch_state(env, batch_id).ok_or(ERR_BATCH_NOT_FOUND)?;

    if item_index >= state.item_count {
        return Err(ERR_BATCH_NOT_FOUND);
    }

    let mut item = state.items.get(item_index).unwrap();
    item.status = BatchItemStatus::Failed;
    item.error_code = error_code;
    state.items.set(item_index, item);
    state.pending_recovery = true;

    update_batch_state(env, &state);
    emit_batch_event(env, symbol_short!("br_fail"), batch_id, item_index);
    Ok(())
}

/// Mark a batch item as rolled back.
pub fn mark_item_rolled_back(env: &Env, batch_id: u64, item_index: u32) -> Result<(), u32> {
    let mut state = get_batch_state(env, batch_id).ok_or(ERR_BATCH_NOT_FOUND)?;

    if item_index >= state.item_count {
        return Err(ERR_BATCH_NOT_FOUND);
    }

    let mut item = state.items.get(item_index).unwrap();
    let was_success = item.status == BatchItemStatus::Success;
    item.status = BatchItemStatus::RolledBack;

    if was_success {
        state.successful_amount = crate::token_math::safe_sub(state.successful_amount, item.amount);
    }

    state.items.set(item_index, item);
    update_batch_state(env, &state);
    Ok(())
}

// ─────────────────────────────────────────────────────────
// Recovery Functions
// ─────────────────────────────────────────────────────────

/// Result of a batch recovery operation.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchRecoveryResult {
    /// Batch ID that was recovered
    pub batch_id: u64,
    /// Number of items retried
    pub items_retried: u32,
    /// Number of items that succeeded on retry
    pub items_succeeded: u32,
    /// Number of items that still failed
    pub items_failed: u32,
    /// Total amount recovered
    pub amount_recovered: i128,
    /// Whether the batch is now complete
    pub complete: bool,
}

/// Get failed items from a batch for retry.
///
/// Returns the indices of all failed items.
pub fn get_failed_items(env: &Env, batch_id: u64) -> Result<soroban_sdk::Vec<u32>, u32> {
    let state = get_batch_state(env, batch_id).ok_or(ERR_BATCH_NOT_FOUND)?;

    let mut failed_indices = soroban_sdk::Vec::new(env);
    for i in 0..state.items.len() {
        let item = state.items.get(i).unwrap();
        if item.status == BatchItemStatus::Failed {
            failed_indices.push_back(item.index);
        }
    }

    Ok(failed_indices)
}

/// Get successful items from a batch for potential rollback.
///
/// Returns the indices of all successful items.
pub fn get_successful_items(env: &Env, batch_id: u64) -> Result<soroban_sdk::Vec<u32>, u32> {
    let state = get_batch_state(env, batch_id).ok_or(ERR_BATCH_NOT_FOUND)?;

    let mut success_indices = soroban_sdk::Vec::new(env);
    for i in 0..state.items.len() {
        let item = state.items.get(i).unwrap();
        if item.status == BatchItemStatus::Success {
            success_indices.push_back(item.index);
        }
    }

    Ok(success_indices)
}

/// Check if a batch recovery has expired.
///
/// A recovery is expired if the recovery timeout has passed since
/// the batch was started.
pub fn is_recovery_expired(env: &Env, batch_id: u64) -> bool {
    let state = match get_batch_state(env, batch_id) {
        Some(s) => s,
        None => return true, // Non-existent batches are considered expired
    };

    let config = get_batch_recovery_config(env);
    let now = env.ledger().timestamp();
    now > state.started_at + config.recovery_timeout_secs
}

/// Increment retry count for an item.
///
/// Returns an error if max retries exceeded.
pub fn increment_retry_count(env: &Env, batch_id: u64, item_index: u32) -> Result<(), u32> {
    let config = get_batch_recovery_config(env);
    let mut state = get_batch_state(env, batch_id).ok_or(ERR_BATCH_NOT_FOUND)?;

    let mut item = state.items.get(item_index).unwrap();

    if item.retry_count >= config.max_retries {
        return Err(ERR_BATCH_NOT_RECOVERABLE);
    }

    item.retry_count += 1;
    item.status = BatchItemStatus::Pending; // Reset to pending for retry
    state.items.set(item_index, item);

    update_batch_state(env, &state);
    Ok(())
}

/// Cancel a batch recovery and mark all items as non-recoverable.
///
/// # Security
/// Caller must verify that `caller` is the authorized key for this batch.
pub fn cancel_batch_recovery(env: &Env, batch_id: u64, caller: &Address) -> Result<(), u32> {
    let mut state = get_batch_state(env, batch_id).ok_or(ERR_BATCH_NOT_FOUND)?;

    // Verify authorization
    if state.authorized_key != *caller {
        return Err(ERR_UNAUTHORIZED_RECOVERY);
    }

    // Check if already complete
    if state.completed_at.is_some() {
        return Err(ERR_BATCH_ALREADY_COMPLETE);
    }

    state.pending_recovery = false;
    state.completed_at = Some(env.ledger().timestamp());

    update_batch_state(env, &state);
    remove_from_pending_recoveries(env, batch_id);

    emit_batch_event(env, symbol_short!("br_cancel"), batch_id, 0);
    Ok(())
}

// ─────────────────────────────────────────────────────────
// Rollback Mechanism
// ─────────────────────────────────────────────────────────

/// Calculate the total amount that would be rolled back.
///
/// This is the sum of all successfully transferred items.
pub fn calculate_rollback_amount(env: &Env, batch_id: u64) -> Result<i128, u32> {
    let state = get_batch_state(env, batch_id).ok_or(ERR_BATCH_NOT_FOUND)?;

    let config = get_batch_recovery_config(env);
    if !config.rollback_enabled {
        return Err(ERR_ROLLBACK_DISABLED);
    }

    let mut total: i128 = 0;
    for i in 0..state.items.len() {
        let item = state.items.get(i).unwrap();
        if item.status == BatchItemStatus::Success {
            total = crate::token_math::safe_add(total, item.amount);
        }
    }

    Ok(total)
}

/// Result of a rollback operation.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RollbackResult {
    /// Batch ID that was rolled back
    pub batch_id: u64,
    /// Number of items rolled back
    pub items_rolled_back: u32,
    /// Total amount returned
    pub amount_returned: i128,
    /// Recipients that were rolled back
    pub affected_recipients: soroban_sdk::Vec<Address>,
}

/// Prepare a rollback for a batch.
///
/// Returns the items that need to be rolled back and the total amount.
/// Does not execute the rollback - caller must handle actual token transfers.
///
/// # Security
/// Caller must verify authorization before executing rollback.
pub fn prepare_rollback(env: &Env, batch_id: u64, caller: &Address) -> Result<RollbackResult, u32> {
    let state = get_batch_state(env, batch_id).ok_or(ERR_BATCH_NOT_FOUND)?;

    // Verify authorization
    if state.authorized_key != *caller {
        return Err(ERR_UNAUTHORIZED_RECOVERY);
    }

    let config = get_batch_recovery_config(env);
    if !config.rollback_enabled {
        return Err(ERR_ROLLBACK_DISABLED);
    }

    let mut total_amount: i128 = 0;
    let mut count: u32 = 0;
    let mut affected = soroban_sdk::Vec::new(env);

    for i in 0..state.items.len() {
        let item = state.items.get(i).unwrap();
        if item.status == BatchItemStatus::Success {
            total_amount = crate::token_math::safe_add(total_amount, item.amount);
            count += 1;
            affected.push_back(item.recipient.clone());
        }
    }

    if count == 0 {
        return Err(ERR_NO_SUCCESSFUL_ITEMS);
    }

    Ok(RollbackResult {
        batch_id,
        items_rolled_back: count,
        amount_returned: total_amount,
        affected_recipients: affected,
    })
}

/// Verify batch integrity after completion.
///
/// Ensures that:
/// 1. Total amounts balance correctly
/// 2. No funds are stranded
/// 3. All items have a terminal status
///
/// # Returns
/// `true` if integrity checks pass, `false` otherwise
pub fn verify_batch_integrity(env: &Env, batch_id: u64) -> bool {
    let state = match get_batch_state(env, batch_id) {
        Some(s) => s,
        None => return false,
    };

    // Calculate actual successful amount from items
    let mut calculated_successful: i128 = 0;
    let mut calculated_pending: i128 = 0;
    let mut calculated_failed: i128 = 0;

    for i in 0..state.items.len() {
        let item = state.items.get(i).unwrap();
        match item.status {
            BatchItemStatus::Success => {
                calculated_successful =
                    crate::token_math::safe_add(calculated_successful, item.amount);
            }
            BatchItemStatus::Pending => {
                calculated_pending = crate::token_math::safe_add(calculated_pending, item.amount);
            }
            BatchItemStatus::Failed | BatchItemStatus::RolledBack => {
                calculated_failed = crate::token_math::safe_add(calculated_failed, item.amount);
            }
        }
    }

    // INV-1: Successful amount must match tracked value
    if calculated_successful != state.successful_amount {
        return false;
    }

    // INV-2: Total must equal sum of all items
    let sum = calculated_successful
        .checked_add(calculated_pending)
        .and_then(|s| s.checked_add(calculated_failed));

    match sum {
        Some(s) if s == state.total_amount => {}
        _ => return false,
    }

    // INV-3: If batch is complete, no pending items should remain
    if state.completed_at.is_some() && calculated_pending > 0 {
        return false;
    }

    true
}

/// Finalize a batch after successful completion.
///
/// Marks the batch as complete and cleans up state.
pub fn finalize_batch(env: &Env, batch_id: u64) -> Result<(), u32> {
    let mut state = get_batch_state(env, batch_id).ok_or(ERR_BATCH_NOT_FOUND)?;

    // Verify integrity before finalizing
    if !verify_batch_integrity(env, batch_id) {
        return Err(ERR_BATCH_INTEGRITY);
    }

    state.completed_at = Some(env.ledger().timestamp());
    state.pending_recovery = false;

    update_batch_state(env, &state);

    // Archive to history
    archive_batch(env, &state);

    // Clear active state
    clear_batch_state(env, batch_id);

    emit_batch_event(env, symbol_short!("br_done"), batch_id, state.item_count);
    Ok(())
}

/// Archive a completed batch to history.
fn archive_batch(env: &Env, state: &BatchRecoveryState) {
    let mut history: soroban_sdk::Vec<BatchRecoveryState> = env
        .storage()
        .persistent()
        .get(&BatchRecoveryKey::RecoveryHistory)
        .unwrap_or(soroban_sdk::Vec::new(env));

    // Keep last 50 batches in history
    const MAX_HISTORY: u32 = 50;
    if history.len() >= MAX_HISTORY {
        history.remove(0);
    }
    history.push_back(state.clone());

    env.storage()
        .persistent()
        .set(&BatchRecoveryKey::RecoveryHistory, &history);
}

/// Get batch recovery history.
pub fn get_recovery_history(env: &Env) -> soroban_sdk::Vec<BatchRecoveryState> {
    env.storage()
        .persistent()
        .get(&BatchRecoveryKey::RecoveryHistory)
        .unwrap_or(soroban_sdk::Vec::new(env))
}

// ─────────────────────────────────────────────────────────
// Event Emission
// ─────────────────────────────────────────────────────────

/// Emit a batch recovery event.
fn emit_batch_event(env: &Env, event_type: soroban_sdk::Symbol, batch_id: u64, value: u32) {
    env.events().publish(
        (symbol_short!("batch"), event_type),
        (batch_id, value, env.ledger().timestamp()),
    );
}

// ─────────────────────────────────────────────────────────
// Batch Recovery Invariants
// ─────────────────────────────────────────────────────────

/// Verify all batch recovery invariants.
///
/// This should be called periodically to ensure system integrity.
pub fn verify_batch_recovery_invariants(env: &Env) -> bool {
    let pending = get_pending_recoveries(env);

    for batch_id in pending.iter() {
        if !verify_batch_integrity(env, batch_id) {
            return false;
        }

        // Check for expired recoveries
        if is_recovery_expired(env, batch_id) {
            // Could auto-cleanup here, but just report for now
            return false;
        }
    }

    true
}
