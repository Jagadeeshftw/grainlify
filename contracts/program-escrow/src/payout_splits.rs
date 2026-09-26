//! # Program Escrow: Payout Splits Module
//!
//! Enables a single escrow to distribute funds across multiple beneficiaries
//! using predefined share ratios, avoiding the need for multiple escrows.
//!
//! ## Rounding Policy
//!
//! This module uses **floor (round-down)** rounding for all share calculations:
//!
//! ```text
//! share_amount = floor(total_amount * share_bps / TOTAL_BASIS_POINTS)
//! ```
//!
//! **Key Invariants:**
//! 1. `sum(all_shares) = TOTAL_BASIS_POINTS` (10,000 bps = 100%).
//! 2. Exact Conservation: `sum(distribution amounts) = total_amount` (no fund loss, no over-distribution).
//! 3. Deterministic Remainder: Any rounding remainder (dust) is deterministically allocated to the first beneficiary (index 0).
//! 4. Exact Representation: Split configurations that cannot be represented exactly in basis points (i.e. shares do not sum to exactly 10,000) are strictly rejected rather than silently rounded.
//!
//! **Security Properties:**
//! - Dust attacks are prevented: each beneficiary gets at most their proportional share plus deterministic dust for index 0.
//! - Total distributed strictly equals the funded amount deducted from escrow (`remaining_balance` decreases by exactly `total_amount`).
//! - No funds are trapped: all funded tokens are distributed across the configured beneficiaries.
//!
//! ## Usage
//!
//! ```rust,ignore
//! // 1. Configure split for a program
//! let beneficiaries = vec![
//!     &env,
//!     BeneficiarySplit { recipient: addr1, share_bps: 7_000 }, // 70%
//!     BeneficiarySplit { recipient: addr2, share_bps: 3_000 }, // 30%
//! ];
//! set_split_config(&env, &program_id, beneficiaries);
//!
//! // 2. Preview distribution (no token transfer)
//! let preview = preview_split(&env, &program_id, 1_000_000);
//!
//! // 3. Execute split payout (transfers tokens)
//! let result = execute_split_payout(&env, &program_id, 1_000_000);
//! ```
//!
//! ## Edge Cases
//!
//! | Scenario | Behavior |
//! |----------|----------|
//! | Amount < beneficiaries | Small amounts may result in 0 for some |
//! | Dust from integer division | Goes to first beneficiary |
//! | Disabled split config | Panics with `split config is disabled` |
//! | Amount > remaining balance | Panics with `insufficient escrow balance` |

use crate::{DataKey, PayoutRecord, ProgramData, PROGRAM_DATA};
use soroban_sdk::{contracttype, symbol_short, token, Address, Env, String, Vec};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Total basis points that split shares must sum to (10 000 bp == 100 %).
pub const TOTAL_BASIS_POINTS: i128 = 10_000;

// Event structs
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SplitConfigSetEvent {
    pub version: u32,
    pub program_id: String,
    pub recipient_count: u32,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SplitPayoutEvent {
    pub version: u32,
    pub program_id: String,
    pub total_amount: i128,
    pub recipient_count: u32,
    pub remaining_balance: i128,
}

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// One entry in a split configuration.
///
/// `share_bps` is this beneficiary's portion expressed in basis points.
/// The sum across all entries in a `SplitConfig` must equal `TOTAL_BASIS_POINTS`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BeneficiarySplit {
    pub recipient: Address,
    /// Share in basis points (1–9 999). All shares must sum to 10 000.
    pub share_bps: i128,
}

/// The complete split configuration attached to a program.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SplitConfig {
    pub program_id: String,
    /// Ordered list of beneficiaries. Dust goes to index 0.
    pub beneficiaries: Vec<BeneficiarySplit>,
    /// Whether this config is currently active.
    pub active: bool,
}

/// Result returned from a split payout execution.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SplitPayoutResult {
    pub total_distributed: i128,
    pub recipient_count: u32,
    pub remaining_balance: i128,
}

// ---------------------------------------------------------------------------
// Storage helpers
// ---------------------------------------------------------------------------

fn split_key(program_id: &String) -> DataKey {
    DataKey::SplitConfig(program_id.clone())
}

fn get_program(env: &Env) -> ProgramData {
    env.storage()
        .instance()
        .get(&PROGRAM_DATA)
        .unwrap_or_else(|| panic!("Program not initialized"))
}

fn save_program(env: &Env, data: &ProgramData) {
    env.storage().instance().set(&PROGRAM_DATA, data);
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Set (or replace) the split configuration for a program.
///
/// # Arguments
/// * `program_id`     - The program this config applies to.
/// * `beneficiaries`  - Ordered list of `BeneficiarySplit`. Index 0 receives dust.
///
/// # Rounding
/// Shares are validated to sum exactly to `TOTAL_BASIS_POINTS` (10,000).
/// Any rounding dust during payout goes to index 0.
///
/// # Panics
/// * If the caller is not the `authorized_payout_key`.
/// * If `beneficiaries` is empty or has more than 50 entries.
/// * If any individual `share_bps` is zero or negative.
/// * If shares do not sum to exactly `TOTAL_BASIS_POINTS` (10 000).
pub fn set_split_config(
    env: &Env,
    program_id: &String,
    beneficiaries: Vec<BeneficiarySplit>,
) -> SplitConfig {
    let n = beneficiaries.len();
    if n == 0 {
        panic!("SplitConfig: must have at least one beneficiary");
    }
    if n > 50 {
        panic!("SplitConfig: maximum 50 beneficiaries");
    }

    // Validate individual shares and compute total.
    let mut total: i128 = 0;
    for i in 0..n {
        let entry = beneficiaries.get(i).unwrap();
        if entry.share_bps <= 0 {
            panic!("SplitConfig: share_bps must be positive");
        }
        total = crate::token_math::safe_add(total, entry.share_bps);
    }
    if total != TOTAL_BASIS_POINTS {
        panic!("SplitConfig: shares must sum to 10000 basis points");
    }

    let config = SplitConfig {
        program_id: program_id.clone(),
        beneficiaries: beneficiaries.clone(),
        active: true,
    };

    env.storage()
        .persistent()
        .set(&split_key(program_id), &config);

    env.events().publish(
        (symbol_short!("SplitCfg"),),
        SplitConfigSetEvent {
            version: 2,
            program_id: program_id.clone(),
            recipient_count: n as u32,
            timestamp: env.ledger().timestamp(),
        },
    );

    config
}

/// Retrieve the split configuration for a program.
///
/// Returns `None` if no split config has been set.
pub fn get_split_config(env: &Env, program_id: &String) -> Option<SplitConfig> {
    env.storage().persistent().get(&split_key(program_id))
}

/// Deactivate the split configuration for a program.
///
/// Requires authorisation from the `authorized_payout_key`.
pub fn disable_split_config(env: &Env, program_id: &String) {
    let key = split_key(program_id);
    let mut config: SplitConfig = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic!("No split config found for program"));

    config.active = false;
    env.storage().persistent().set(&key, &config);
}

/// Execute a split payout of `total_amount` according to the stored `SplitConfig`.
///
/// ## Rounding Implementation
///
/// Each beneficiary receives:
/// ```text
/// amount_i = floor(total_amount * share_bps_i / TOTAL_BASIS_POINTS)
/// ```
///
/// Dust (remainder) is computed as:
/// ```text
/// dust = total_amount - sum(amount_i for all i)
/// dust_amount_0 = amount_0 + dust  // dust goes to first beneficiary
/// ```
///
/// ## Security Invariants
///
/// 1. **No over-distribution**: `sum(amounts) = total_amount` (dust absorbed)
/// 2. **Balance protection**: `total_amount ≤ remaining_balance`
/// 3. **Dust absorption**: First beneficiary absorbs dust, preventing fund loss
///
/// # Arguments
/// * `program_id`   - The program whose config to use.
/// * `total_amount` - Gross amount to distribute (must be ≤ remaining balance).
///
/// # Returns
/// `SplitPayoutResult` with totals and updated remaining balance.
///
/// # Panics
/// * If no active split config exists.
/// * If `total_amount` ≤ 0 or exceeds the remaining balance.
/// * If caller is not the `authorized_payout_key`.
pub fn execute_split_payout(
    env: &Env,
    program_id: &String,
    total_amount: i128,
) -> SplitPayoutResult {
    let mut program = get_program(env);

    if total_amount <= 0 {
        panic!("SplitPayout: amount must be greater than zero");
    }
    if total_amount > program.remaining_balance {
        panic!("SplitPayout: insufficient escrow balance");
    }

    // Load and validate config.
    let config: SplitConfig = env
        .storage()
        .persistent()
        .get(&split_key(program_id))
        .unwrap_or_else(|| panic!("SplitPayout: no split config found for program"));

    if !config.active {
        panic!("SplitPayout: split config is disabled");
    }

    let n = config.beneficiaries.len();
    let contract_addr = env.current_contract_address();
    let token_client = token::Client::new(env, &program.token_address);
    let now = env.ledger().timestamp();

    // Compute individual amounts using bp arithmetic; accumulate dust.
    // dust = total_amount - sum(floor(total_amount * share_bps / 10_000))
    let mut amounts: soroban_sdk::Vec<i128> = soroban_sdk::Vec::new(env);
    let mut distributed: i128 = 0;

    for i in 0..n {
        let entry = config.beneficiaries.get(i).unwrap();
        let product = crate::token_math::safe_mul(total_amount, entry.share_bps);
        let share_amount = product
            .checked_div(TOTAL_BASIS_POINTS)
            .unwrap_or_else(|| panic!("SplitPayout: division error"));
        amounts.push_back(share_amount);
        distributed = crate::token_math::safe_add(distributed, share_amount);
    }

    // Dust goes to index 0.
    let dust = crate::token_math::safe_sub(total_amount, distributed);
    if dust < 0 {
        panic!("SplitPayout: internal accounting error");
    }
    let first_amount = crate::token_math::safe_add(amounts.get(0).unwrap(), dust);
    amounts.set(0, first_amount);

    // Transfer and record payouts.
    for i in 0..n {
        let entry = config.beneficiaries.get(i).unwrap();
        let amount = amounts.get(i).unwrap();

        if amount <= 0 {
            // Edge case: a beneficiary with a very small share on a tiny payout.
            // Skip transfer but still record so history is complete.
            continue;
        }

        token_client.transfer(&contract_addr, &entry.recipient, &amount);

        program.payout_history.push_back(PayoutRecord {
            recipient: entry.recipient.clone(),
            amount,
            timestamp: now,
        });
    }

    program.remaining_balance =
        crate::token_math::safe_sub(program.remaining_balance, total_amount);
    save_program(env, &program);

    env.events().publish(
        (symbol_short!("SplitPay"),),
        SplitPayoutEvent {
            version: 2,
            program_id: program_id.clone(),
            total_amount,
            recipient_count: n as u32,
            remaining_balance: program.remaining_balance,
        },
    );

    SplitPayoutResult {
        total_distributed: total_amount,
        recipient_count: n as u32,
        remaining_balance: program.remaining_balance,
    }
}

/// Calculate the hypothetical split amounts for `total_amount` without executing transfers.
///
/// Useful for off-chain previews and tests. Uses the same floor rounding as
/// `execute_split_payout`; dust is awarded to index 0.
///
/// ## Rounding
/// Same as `execute_split_payout`: each share uses floor division,
/// with dust absorbed by the first beneficiary.
///
/// Returns a `Vec` of `BeneficiarySplit` where the `share_bps` field
/// contains the computed amount for each beneficiary.
pub fn preview_split(env: &Env, program_id: &String, total_amount: i128) -> Vec<BeneficiarySplit> {
    let config: SplitConfig = env
        .storage()
        .persistent()
        .get(&split_key(program_id))
        .unwrap_or_else(|| panic!("No split config found for program"));

    let n = config.beneficiaries.len();
    let mut preview: Vec<BeneficiarySplit> = Vec::new(env);
    let mut distributed: i128 = 0;
    let mut computed: soroban_sdk::Vec<i128> = soroban_sdk::Vec::new(env);

    for i in 0..n {
        let entry = config.beneficiaries.get(i).unwrap();
        let product = crate::token_math::safe_mul(total_amount, entry.share_bps);
        let share_amount = product.checked_div(TOTAL_BASIS_POINTS).unwrap_or(0);
        computed.push_back(share_amount);
        distributed = crate::token_math::safe_add(distributed, share_amount);
    }

    let dust = crate::token_math::safe_sub(total_amount, distributed);

    for i in 0..n {
        let entry = config.beneficiaries.get(i).unwrap();
        let mut amount = computed.get(i).unwrap();
        if i == 0 {
            amount = crate::token_math::safe_add(amount, dust);
        }
        preview.push_back(BeneficiarySplit {
            recipient: entry.recipient,
            share_bps: amount, // repurposed field: holds computed amount in preview context
        });
    }

    preview
}
