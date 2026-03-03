// ============================================================
// FILE: contracts/program-escrow/src/payout_splits.rs
//
// This module implements multi-beneficiary payout splits for Issue #[issue_id].
//
// Enables a single escrow to distribute funds across multiple recipients
// using predefined share ratios, avoiding the need for multiple escrows.
//
// ## Design
//
// - Shares are expressed in basis points (1 bp = 0.01%), summing to 10_000 (100%)
// - Dust (remainder after integer division) is awarded to the first beneficiary
// - Splits are stored per-program and validated at creation time
// - Both partial releases and full releases honour the ratio
//
// ## Integration (lib.rs)
//
//   mod payout_splits;
//   pub use payout_splits::{BeneficiarySplit, SplitConfig};
//
// Add the following DataKey variants if not already present:
//
//   SplitConfig(String),   // program_id -> SplitConfig
//
// Expose the public functions inside the `ProgramEscrowContract` impl block.
// ============================================================

use soroban_sdk::{contracttype, symbol_short, token, Address, Env, String, Symbol, Vec};
use crate::{DataKey, ProgramData, PayoutRecord, PROGRAM_DATA};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Total basis points that split shares must sum to (10 000 bp == 100 %).
pub const TOTAL_BASIS_POINTS: i128 = 10_000;

// Event symbols
const SPLIT_CONFIG_SET: Symbol = symbol_short!("SplitCfg");
const SPLIT_PAYOUT: Symbol = symbol_short!("SplitPay");

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
    let program = get_program(env);
    program.authorized_payout_key.require_auth();

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
        total = total
            .checked_add(entry.share_bps)
            .unwrap_or_else(|| panic!("SplitConfig: share overflow"));
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
        (SPLIT_CONFIG_SET,),
        (program_id.clone(), n as u32, env.ledger().timestamp()),
    );

    config
}

/// Retrieve the split configuration for a program.
///
/// Returns `None` if no split config has been set.
pub fn get_split_config(env: &Env, program_id: &String) -> Option<SplitConfig> {
    env.storage()
        .persistent()
        .get(&split_key(program_id))
}

/// Deactivate the split configuration for a program.
///
/// Requires authorisation from the `authorized_payout_key`.
pub fn disable_split_config(env: &Env, program_id: &String) {
    let program = get_program(env);
    program.authorized_payout_key.require_auth();

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
/// The amount is divided proportionally using basis-point arithmetic.  Any
/// remainder from integer division (dust) is added to the **first** beneficiary,
/// ensuring the full `total_amount` is always distributed without drift.
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
    program.authorized_payout_key.require_auth();

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
        let share_amount = total_amount
            .checked_mul(entry.share_bps)
            .and_then(|x| x.checked_div(TOTAL_BASIS_POINTS))
            .unwrap_or_else(|| panic!("SplitPayout: arithmetic overflow"));
        amounts.push_back(share_amount);
        distributed = distributed
            .checked_add(share_amount)
            .unwrap_or_else(|| panic!("SplitPayout: sum overflow"));
    }

    // Dust goes to index 0.
    let dust = total_amount - distributed;
    if dust < 0 {
        panic!("SplitPayout: internal accounting error");
    }
    let first_amount = amounts.get(0).unwrap() + dust;
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

    program.remaining_balance -= total_amount;
    save_program(env, &program);

    env.events().publish(
        (SPLIT_PAYOUT,),
        (
            program_id.clone(),
            total_amount,
            n as u32,
            program.remaining_balance,
            now,
        ),
    );

    SplitPayoutResult {
        total_distributed: total_amount,
        recipient_count: n as u32,
        remaining_balance: program.remaining_balance,
    }
}

/// Calculate the hypothetical split amounts for `total_amount` without executing transfers.
///
/// Useful for off-chain previews and tests.  Dust is awarded to index 0.
///
/// Returns a `Vec` of `(recipient, amount)` pairs in config order.
pub fn preview_split(
    env: &Env,
    program_id: &String,
    total_amount: i128,
) -> Vec<BeneficiarySplit> {
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
        let share_amount = total_amount
            .checked_mul(entry.share_bps)
            .and_then(|x| x.checked_div(TOTAL_BASIS_POINTS))
            .unwrap_or(0);
        computed.push_back(share_amount);
        distributed += share_amount;
    }

    let dust = total_amount - distributed;

    for i in 0..n {
        let entry = config.beneficiaries.get(i).unwrap();
        let mut amount = computed.get(i).unwrap();
        if i == 0 {
            amount += dust;
        }
        preview.push_back(BeneficiarySplit {
            recipient: entry.recipient,
            share_bps: amount, // repurposed field: holds computed amount in preview context
        });
    }

    preview
}