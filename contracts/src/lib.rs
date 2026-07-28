//! # Grainlify Contracts Library
//!
//! This crate provides shared utilities and storage key management for Grainlify smart contracts.
//! It includes namespace protection, collision detection, and common constants.

pub mod storage_key_audit;

//! # View-Facade Contract
//!
//! Exposes **read-only** queries over `ProgramData` and `FeeConfig` so
//! that wallets, UIs, and off-chain indexers can inspect live state without
//! paying gas for a mutating transaction.
//!
//! ## Entrypoints
//!
//! | Function | Kind | Description |
//! |----------|------|-------------|
//! | [`ViewFacade::get_program`] | view | Returns the full `ProgramData` for a program ID |
//! | [`ViewFacade::get_fee_config`] | view | Returns the active `FeeConfig` |
//! | [`ViewFacade::is_circuit_open`] | view | Returns the circuit-breaker state |
//! | [`ViewFacade::simulate_payout`] | **view** | Computes net amounts, fees, and warnings without writing state |
//!
//! ## Security model
//!
//! All functions in this contract are **read-only**: they never call
//! `storage.set`, `storage.remove`, or any function that transfers tokens.
//! The circuit-breaker check inside `simulate_payout` only *reports* the
//! breaker state as a warning — it does not abort the simulation, because
//! the purpose is to let the UI show a preview even when payouts are
//! currently paused.
//!
//! No authentication is required. All entrypoints are permissionless.

// ─── Storage key constants ────────────────────────────────────────────────────

pub const KEY_PROGRAM_DATA:  &str = "PROGRAM_DATA";
pub const KEY_FEE_CONFIG:    &str = "FEE_CONFIG";
pub const KEY_CIRCUIT_OPEN:  &str = "CIRCUIT_OPEN";

// ─── Domain types ─────────────────────────────────────────────────────────────

/// An address is represented as a `String` in this standalone implementation.
/// In a real Soroban contract this would be `soroban_sdk::Address`.
pub type Address = String;

/// Represents a single recipient and their gross payout amount.
#[derive(Debug, Clone, PartialEq)]
pub struct Recipient {
    pub address: Address,
    /// Gross amount (before fee deduction), in the escrow token's smallest unit.
    pub gross_amount: u128,
}

/// A single entry in the simulation result's `net_amounts` list.
#[derive(Debug, Clone, PartialEq)]
pub struct NetEntry {
    pub address: Address,
    /// Amount the recipient will actually receive after fee deduction.
    pub net_amount: u128,
    /// Fee deducted from this recipient's gross amount.
    pub fee: u128,
}

/// Bracket weighting tier for graduated fee schedules.
///
/// Each bracket defines a gross-amount threshold and the fee rate
/// (in basis points) applied to amounts within that bracket.
///
/// Example:
/// ```text
/// tier 1: up to 10,000  → 100 bp (1 %)
/// tier 2: up to 50,000  → 200 bp (2 %)
/// tier 3: unlimited     → 300 bp (3 %)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct FeeBracket {
    /// Upper bound of this bracket (inclusive).  `None` = no upper bound.
    pub ceiling: Option<u128>,
    /// Fee rate in basis points for this bracket.
    pub rate_bp: u32,
}

/// Fee configuration for a program.
///
/// If `brackets` is empty, `default_rate_bp` is applied uniformly.
/// Otherwise brackets are evaluated in order; the first bracket whose
/// `ceiling >= gross_amount` (or whose ceiling is `None`) wins.
#[derive(Debug, Clone, PartialEq)]
pub struct FeeConfig {
    /// Default fee rate in basis points (0–10,000).
    /// Applied when no brackets are defined or no bracket matches.
    pub default_rate_bp: u32,
    /// Optional graduated bracket schedule.
    pub brackets: Vec<FeeBracket>,
}

/// Funding and metadata for a grant program.
#[derive(Debug, Clone, PartialEq)]
pub struct ProgramData {
    /// Human-readable program identifier (e.g. "Stellar Q1 OSS Program").
    pub program_id: String,
    /// Total funds locked in escrow at program creation.
    pub total_locked: u128,
    /// Remaining undisbursed balance.
    pub remaining_balance: u128,
    /// Whether the program is currently active.
    pub is_active: bool,
    /// Free-form metadata (off-chain CID, description hash, etc.).
    pub metadata: String,
}

// ─── Warning type ─────────────────────────────────────────────────────────────

/// A non-fatal advisory message produced by `simulate_payout`.
///
/// Warnings do not prevent the simulation from completing; they surface
/// conditions the UI should bring to the user's attention before submitting
/// a real payout transaction.
#[derive(Debug, Clone, PartialEq)]
pub enum Warning {
    /// The circuit breaker is open; real payouts are currently blocked.
    CircuitBreakerOpen,
    /// The program is not active; real payouts would be rejected.
    ProgramInactive { program_id: String },
    /// The total gross payout exceeds the program's remaining balance.
    InsufficientBalance {
        required: u128,
        available: u128,
    },
    /// A recipient's gross amount is zero; they would receive nothing.
    ZeroAmountRecipient { address: Address },
    /// Computed net amount for a recipient rounded down to zero due to fees.
    NetAmountZero { address: Address },
    /// The recipients list is empty; no simulation was performed.
    EmptyRecipientList,
    /// Duplicate address detected; the later entry overwrites the earlier one
    /// in a real payout (surfaced here for transparency).
    DuplicateAddress { address: Address },
}

impl std::fmt::Display for Warning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CircuitBreakerOpen =>
                write!(f, "CIRCUIT_BREAKER_OPEN: real payouts are currently suspended"),
            Self::ProgramInactive { program_id } =>
                write!(f, "PROGRAM_INACTIVE: program '{}' is not active", program_id),
            Self::InsufficientBalance { required, available } =>
                write!(f, "INSUFFICIENT_BALANCE: required {} but only {} available", required, available),
            Self::ZeroAmountRecipient { address } =>
                write!(f, "ZERO_AMOUNT: recipient '{}' has gross amount of 0", address),
            Self::NetAmountZero { address } =>
                write!(f, "NET_ZERO: fee consumed entire payout for '{}'", address),
            Self::EmptyRecipientList =>
                write!(f, "EMPTY_RECIPIENTS: no recipients provided"),
            Self::DuplicateAddress { address } =>
                write!(f, "DUPLICATE_ADDRESS: '{}' appears more than once", address),
        }
    }
}

// ─── Simulation result ────────────────────────────────────────────────────────

/// Output of [`ViewFacade::simulate_payout`].
///
/// # Invariants
///
/// - `net_amounts[i].net_amount + net_amounts[i].fee == gross_amount[i]`
///   (conservation — no dust is lost per recipient).
/// - `total_fees == sum of net_amounts[i].fee`
/// - `total_net  == sum of net_amounts[i].net_amount`
/// - `total_gross == total_fees + total_net`
///
/// These invariants hold even when warnings are present.
#[derive(Debug, Clone, PartialEq)]
pub struct SimulationResult {
    /// Per-recipient net amounts after fee deduction.
    pub net_amounts: Vec<NetEntry>,
    /// Sum of all fees across all recipients.
    pub total_fees: u128,
    /// Sum of all net payouts across all recipients.
    pub total_net: u128,
    /// Effective fee rate applied (for display purposes only).
    /// If different brackets applied to different recipients this
    /// is the weighted average in basis points.
    pub effective_rate_bp: u32,
    /// Non-fatal advisories. Empty = no issues detected.
    pub warnings: Vec<Warning>,
}

// ─── Simulated storage ────────────────────────────────────────────────────────

/// Minimal in-process key-value store used to simulate ledger storage.
/// Mirrors the pattern from `program-escrow`'s `Storage` type.
pub struct Storage {
    programs:     std::collections::HashMap<String, ProgramData>,
    fee_config:   Option<FeeConfig>,
    circuit_open: bool,
}

impl Storage {
    pub fn new() -> Self {
        Self {
            programs:     std::collections::HashMap::new(),
            fee_config:   None,
            circuit_open: false,
        }
    }

    pub fn set_program(&mut self, data: ProgramData) {
        self.programs.insert(data.program_id.clone(), data);
    }

    pub fn get_program(&self, id: &str) -> Option<&ProgramData> {
        self.programs.get(id)
    }

    pub fn set_fee_config(&mut self, cfg: FeeConfig) {
        self.fee_config = Some(cfg);
    }

    pub fn get_fee_config(&self) -> Option<&FeeConfig> {
        self.fee_config.as_ref()
    }

    pub fn set_circuit_open(&mut self, open: bool) {
        self.circuit_open = open;
    }

    pub fn is_circuit_open(&self) -> bool {
        self.circuit_open
    }
}

// ─── Fee arithmetic ───────────────────────────────────────────────────────────

/// Basis-point denominator.
const BASIS_POINTS: u128 = 10_000;

/// Maximum allowed fee rate: 10 % (1,000 bp).
pub const MAX_FEE_RATE_BP: u32 = 1_000;

/// Resolves the applicable fee rate for `gross_amount` given `config`.
///
/// Bracket resolution order:
/// 1. If `brackets` is empty → use `default_rate_bp`.
/// 2. Walk brackets in order; use the first bracket where
///    `ceiling.is_none() || ceiling >= gross_amount`.
/// 3. If no bracket matches → fall back to `default_rate_bp`.
///
/// # Security
///
/// The rate is capped at `MAX_FEE_RATE_BP` regardless of what is stored
/// in `FeeConfig`. This prevents a corrupted config from extracting > 10 %.
pub fn resolve_fee_rate(config: &FeeConfig, gross_amount: u128) -> u32 {
    let rate = if config.brackets.is_empty() {
        config.default_rate_bp
    } else {
        config.brackets
            .iter()
            .find(|b| b.ceiling.map_or(true, |c| gross_amount <= c))
            .map(|b| b.rate_bp)
            .unwrap_or(config.default_rate_bp)
    };
    rate.min(MAX_FEE_RATE_BP)
}

/// Computes the fee for `gross_amount` at `rate_bp` using floor division.
///
/// Uses the split-division algorithm to avoid intermediate `u128` overflow:
///
/// ```text
/// q   = gross_amount / 10_000
/// r   = gross_amount % 10_000
/// fee = q * rate_bp + r * rate_bp / 10_000
/// ```
///
/// # Overflow proof
///
/// - `q * rate_bp` ≤ `(u128::MAX / 10_000) * 1_000` = `u128::MAX / 10` ✓
/// - `r * rate_bp` ≤ `9_999 * 1_000` = `9_999_000` ✓
pub fn compute_fee(gross_amount: u128, rate_bp: u32) -> u128 {
    if rate_bp == 0 || gross_amount == 0 { return 0; }
    let rate = rate_bp as u128;
    let q = gross_amount / BASIS_POINTS;
    let r = gross_amount % BASIS_POINTS;
    q * rate + r * rate / BASIS_POINTS
}

// ─── ViewFacade contract ──────────────────────────────────────────────────────

/// The view-facade contract.
///
/// In a real Soroban deployment this struct would carry an `Env` reference.
/// Here it wraps a `Storage` reference so the same logic can be unit-tested
/// without a full SDK harness.
pub struct ViewFacade<'a> {
    storage: &'a Storage,
}

impl<'a> ViewFacade<'a> {
    pub fn new(storage: &'a Storage) -> Self {
        Self { storage }
    }

    // ── get_program ───────────────────────────────────────────────────────────

    /// Returns the `ProgramData` for `program_id`, or `None` if not found.
    ///
    /// # Security
    ///
    /// Read-only. No authentication required.
    pub fn get_program(&self, program_id: &str) -> Option<ProgramData> {
        self.storage.get_program(program_id).cloned()
    }

    // ── get_fee_config ────────────────────────────────────────────────────────

    /// Returns the active `FeeConfig`, or `None` if not initialised.
    ///
    /// # Security
    ///
    /// Read-only. No authentication required.
    pub fn get_fee_config(&self) -> Option<FeeConfig> {
        self.storage.get_fee_config().cloned()
    }

    // ── is_circuit_open ───────────────────────────────────────────────────────

    /// Returns `true` when the circuit breaker is open (payouts suspended).
    ///
    /// # Security
    ///
    /// Read-only. No authentication required.
    pub fn is_circuit_open(&self) -> bool {
        self.storage.is_circuit_open()
    }

    // ── simulate_payout ───────────────────────────────────────────────────────

    /// Simulates a batch payout for `program_id` against `recipients`
    /// **without mutating any on-chain state**.
    ///
    /// This is the primary entrypoint of the view-facade for wallet previews
    /// and UI dry-runs.  It is safe to call at any time, even when the
    /// circuit breaker is open.
    ///
    /// # What is computed
    ///
    /// For each `(address, gross_amount)` in `recipients`:
    ///
    /// 1. Resolve the applicable `FeeConfig` for `program_id`.
    /// 2. Determine the bracket-weighted fee rate for `gross_amount`.
    /// 3. Compute `fee = floor(gross_amount × rate / 10_000)` (overflow-safe).
    /// 4. Compute `net = gross_amount − fee`.
    /// 5. Accumulate `total_fees` and `total_net`.
    ///
    /// After processing all recipients, warnings are emitted if:
    /// - The circuit breaker is open.
    /// - The program is not active.
    /// - Total gross > program's remaining balance.
    /// - Any recipient has a zero gross amount.
    /// - Any recipient's fee consumed their entire gross (net == 0).
    /// - The recipient list is empty.
    /// - Duplicate addresses are present.
    ///
    /// # Returns
    ///
    /// A [`SimulationResult`] with per-recipient net amounts, total fees,
    /// total net, effective rate, and all warnings.  The result is always
    /// returned regardless of warnings — warnings are advisory only.
    ///
    /// # Security assumptions
    ///
    /// - **No state mutation**: this function never calls any storage write.
    /// - **No token transfer**: no `transfer`, `approve`, or escrow call is made.
    /// - **No authentication required**: any caller may invoke this function.
    /// - **Fee cap enforced**: the fee rate is capped at `MAX_FEE_RATE_BP`
    ///   even if the stored config contains a higher value.
    /// - **Overflow-safe arithmetic**: the split-division algorithm is used for
    ///   all fee calculations (see [`compute_fee`]).
    /// - **Conservation**: `fee + net == gross_amount` for every recipient.
    ///
    /// # Example
    ///
    /// ```rust
    /// use view_facade::{ViewFacade, Storage, ProgramData, FeeConfig, Recipient};
    ///
    /// let mut storage = Storage::new();
    /// storage.set_program(ProgramData {
    ///     program_id: "prog-1".into(),
    ///     total_locked: 100_000,
    ///     remaining_balance: 100_000,
    ///     is_active: true,
    ///     metadata: String::new(),
    /// });
    /// storage.set_fee_config(FeeConfig { default_rate_bp: 500, brackets: vec![] });
    ///
    /// let facade = ViewFacade::new(&storage);
    /// let result = facade.simulate_payout("prog-1", vec![
    ///     Recipient { address: "GABC".into(), gross_amount: 10_000 },
    /// ]);
    ///
    /// assert_eq!(result.net_amounts[0].net_amount, 9_500);
    /// assert_eq!(result.total_fees, 500);
    /// assert!(result.warnings.is_empty());
    /// ```
    pub fn simulate_payout(
        &self,
        program_id: &str,
        recipients: Vec<Recipient>,
    ) -> SimulationResult {
        let mut warnings: Vec<Warning> = Vec::new();
        let mut net_amounts: Vec<NetEntry> = Vec::new();
        let mut total_fees: u128 = 0;
        let mut total_net: u128 = 0;
        let mut total_gross: u128 = 0;
        let mut total_rate_bp_weighted: u128 = 0; // for effective rate calc

        // ── 1. Empty recipient list check ─────────────────────────────────────
        if recipients.is_empty() {
            warnings.push(Warning::EmptyRecipientList);
            return SimulationResult {
                net_amounts: vec![],
                total_fees: 0,
                total_net: 0,
                effective_rate_bp: 0,
                warnings,
            };
        }

        // ── 2. Circuit breaker check (advisory — does not abort simulation) ───
        if self.storage.is_circuit_open() {
            warnings.push(Warning::CircuitBreakerOpen);
        }

        // ── 3. Program state checks ───────────────────────────────────────────
        let program = self.storage.get_program(program_id);
        if let Some(prog) = &program {
            if !prog.is_active {
                warnings.push(Warning::ProgramInactive {
                    program_id: program_id.to_string(),
                });
            }
        }
        // Note: if program doesn't exist we still simulate using the fee config.

        // ── 4. Resolve fee config ─────────────────────────────────────────────
        // Fall back to a zero-fee config if none is stored.
        let fee_config = self.storage.get_fee_config().cloned().unwrap_or(FeeConfig {
            default_rate_bp: 0,
            brackets: vec![],
        });

        // ── 5. Duplicate address detection ────────────────────────────────────
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for r in &recipients {
            if !seen.insert(r.address.clone()) {
                warnings.push(Warning::DuplicateAddress {
                    address: r.address.clone(),
                });
            }
        }

        // ── 6. Per-recipient computation ──────────────────────────────────────
        for recipient in &recipients {
            // Zero gross amount warning
            if recipient.gross_amount == 0 {
                warnings.push(Warning::ZeroAmountRecipient {
                    address: recipient.address.clone(),
                });
                net_amounts.push(NetEntry {
                    address: recipient.address.clone(),
                    net_amount: 0,
                    fee: 0,
                });
                continue;
            }

            // Bracket-weighted fee rate
            let rate_bp = resolve_fee_rate(&fee_config, recipient.gross_amount);

            // Overflow-safe fee computation
            let fee = compute_fee(recipient.gross_amount, rate_bp);

            // Conservation: net = gross - fee (never underflows because fee <= gross)
            let net = recipient.gross_amount - fee;

            // Net-zero warning
            if net == 0 {
                warnings.push(Warning::NetAmountZero {
                    address: recipient.address.clone(),
                });
            }

            // Accumulate for weighted effective rate.
            // Use saturating_add to avoid overflow when amounts approach u128::MAX.
            total_rate_bp_weighted = total_rate_bp_weighted.saturating_add(
                (rate_bp as u128).saturating_mul(recipient.gross_amount),
            );
            total_gross += recipient.gross_amount;
            total_fees += fee;
            total_net += net;

            net_amounts.push(NetEntry {
                address: recipient.address.clone(),
                net_amount: net,
                fee,
            });
        }

        // ── 7. Balance sufficiency check ──────────────────────────────────────
        if let Some(prog) = &program {
            if total_gross > prog.remaining_balance {
                warnings.push(Warning::InsufficientBalance {
                    required:  total_gross,
                    available: prog.remaining_balance,
                });
            }
        }

        // ── 8. Weighted effective rate ────────────────────────────────────────
        let effective_rate_bp = if total_gross == 0 {
            0u32
        } else {
            (total_rate_bp_weighted / total_gross) as u32
        };

        SimulationResult {
            net_amounts,
            total_fees,
            total_net,
            effective_rate_bp,
            warnings,
        }
    }
}

// ─── Tests module ─────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests;