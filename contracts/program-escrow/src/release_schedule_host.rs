//! Host-side (std) simulation of the O(1) `release_schedule` optimization.
//!
//! Compiled only for `cargo test` so the WASM `cdylib` stays `no_std`.

#![cfg(test)]

extern crate std;

use std::collections::HashMap;
use std::string::{String, ToString};
use std::vec;
use std::vec::Vec;

// Host-side Program Escrow release_schedule simulation (std HashMap storage).

// ─── Storage key constants ────────────────────────────────────────────────────

/// Storage key for the full release schedule / history vector.
pub const RELEASE_HISTORY: &str = "RELEASE_HISTORY";

/// Storage key for the locked escrow balance.
pub const ESCROW_BALANCE: &str = "ESCROW_BALANCE";

/// Storage key for the circuit-breaker pause flag.
pub const PROGRAM_PAUSED: &str = "PROGRAM_PAUSED";

/// Storage key for the authorized payout public key.
pub const AUTHORIZED_KEY: &str = "AUTHORIZED_KEY";

// ─── Domain types ─────────────────────────────────────────────────────────────

/// A single scheduled release entry.
///
/// Entries are stored in a flat `Vec<ReleaseEntry>` under `RELEASE_HISTORY`.
/// The optimized [`release_schedule`] loads this vec **once**, processes all
/// due entries in memory, then writes it back **once**.
#[derive(Debug, Clone, PartialEq)]
pub struct ReleaseEntry {
    /// Unique identifier for this milestone / schedule entry.
    pub id: u64,
    /// Ledger timestamp at which this entry becomes eligible for release.
    pub due_ledger: u64,
    /// Amount (in the escrow token's smallest unit) to release.
    pub amount: u64,
    /// Recipient wallet address.
    pub recipient: String,
    /// Whether this entry has already been released.
    pub released: bool,
    /// Optional dependency: this entry cannot be released until the entry
    /// with this `id` has already been released.
    pub depends_on: Option<u64>,
}

/// Result returned by [`release_schedule`].
#[derive(Debug, PartialEq)]
pub struct ReleaseResult {
    /// Number of entries that were newly released in this call.
    pub entries_released: usize,
    /// Total amount disbursed across all newly released entries.
    pub total_disbursed: u64,
    /// IDs of entries that were released.
    pub released_ids: Vec<u64>,
}

// ─── Error type ───────────────────────────────────────────────────────────────

/// Errors that can be returned by contract functions.
#[derive(Debug, PartialEq)]
pub enum EscrowError {
    /// The program has been paused by the circuit breaker.
    ProgramPaused,
    /// The caller is not the authorized payout key.
    Unauthorized,
    /// The escrow balance is insufficient to cover the requested payout.
    InsufficientBalance,
    /// Integer overflow occurred during amount accumulation.
    Overflow,
    /// A dependency milestone has not yet been released.
    DependencyNotMet(u64),
}

// ─── Simulated storage (used in tests and embedded logic) ─────────────────────

/// Minimal in-process key-value store used to simulate ledger storage.
///
/// In a real Soroban contract this would be replaced by `env.storage()`.
/// Keeping it as a plain `std::collections::HashMap` lets us run the full
/// logic under `cargo test` without a Soroban test harness.
pub struct Storage {
    inner: std::collections::HashMap<String, StorageValue>,
}

/// Union of storable value types.
#[derive(Clone)]
pub enum StorageValue {
    Entries(Vec<ReleaseEntry>),
    Balance(u64),
    Paused(bool),
    Key(String),
}

impl Storage {
    pub fn new() -> Self {
        Self {
            inner: std::collections::HashMap::new(),
        }
    }

    pub fn get_entries(&self, key: &str) -> Vec<ReleaseEntry> {
        match self.inner.get(key) {
            Some(StorageValue::Entries(v)) => v.clone(),
            _ => Vec::new(),
        }
    }

    pub fn set_entries(&mut self, key: &str, entries: Vec<ReleaseEntry>) {
        self.inner.insert(key.to_string(), StorageValue::Entries(entries));
    }

    pub fn get_balance(&self) -> u64 {
        match self.inner.get(ESCROW_BALANCE) {
            Some(StorageValue::Balance(b)) => *b,
            _ => 0,
        }
    }

    pub fn set_balance(&mut self, balance: u64) {
        self.inner.insert(ESCROW_BALANCE.to_string(), StorageValue::Balance(balance));
    }

    pub fn is_paused(&self) -> bool {
        match self.inner.get(PROGRAM_PAUSED) {
            Some(StorageValue::Paused(p)) => *p,
            _ => false,
        }
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.inner.insert(PROGRAM_PAUSED.to_string(), StorageValue::Paused(paused));
    }

    pub fn get_authorized_key(&self) -> Option<String> {
        match self.inner.get(AUTHORIZED_KEY) {
            Some(StorageValue::Key(k)) => Some(k.clone()),
            _ => None,
        }
    }

    pub fn set_authorized_key(&mut self, key: &str) {
        self.inner.insert(
            AUTHORIZED_KEY.to_string(),
            StorageValue::Key(key.to_string()),
        );
    }
}

// ─── Core contract function ───────────────────────────────────────────────────

/// Processes all due milestone entries and releases their escrowed funds.
///
/// # Optimization — O(1) storage reads for `RELEASE_HISTORY`
///
/// ## Before (O(N) pattern — **do not use**)
///
/// The naive implementation read and wrote `RELEASE_HISTORY` on every loop
/// iteration, producing N ledger-entry accesses for N due entries:
///
/// ```text
/// for entry in schedule_ids {
///     let mut history = storage.get(RELEASE_HISTORY);   // ← read #1..N
///     history[entry].released = true;
///     storage.set(RELEASE_HISTORY, history);             // ← write #1..N
/// }
/// ```
///
/// Each ledger-entry read/write incurs a fee on Stellar/Soroban.  For a
/// program with 50 milestones becoming due simultaneously this was 100
/// ledger-entry operations instead of 2.
///
/// ## After (O(1) pattern — **this implementation**)
///
/// ```text
/// let mut history = storage.get(RELEASE_HISTORY);       // ← single read
/// let mut dirty = false;
/// for entry in &mut history {
///     if is_due && !entry.released {
///         entry.released = true;                         // mutate in memory
///         dirty = true;
///     }
/// }
/// if dirty {
///     storage.set(RELEASE_HISTORY, history);            // ← single write
/// }
/// ```
///
/// Storage accesses are now constant (1 read + at most 1 write) regardless
/// of how many entries are processed.
///
/// # Arguments
///
/// * `storage`         — Mutable reference to the contract storage.
/// * `current_ledger`  — The current ledger sequence number (acts as timestamp).
/// * `caller`          — Public key of the transaction submitter.
///
/// # Returns
///
/// * `Ok(ReleaseResult)` — Summary of what was released.
/// * `Err(EscrowError)`  — If the program is paused, caller is unauthorized,
///                         balance is insufficient, or arithmetic overflows.
///
/// # Security
///
/// - Authorization check runs **before** any state mutation.
/// - Circuit breaker (pause flag) check runs **before** authorization.
/// - Overflow is checked with `checked_add`; the function returns
///   [`EscrowError::Overflow`] rather than panicking.
/// - Dependency constraints are enforced: an entry whose `depends_on` points
///   to an unreleased entry is skipped in the current call rather than
///   returning an error, allowing independent milestones to proceed.
/// - The balance is decremented atomically after accumulation; the escrow
///   write happens only if at least one entry was processed.
/// - Already-released entries are idempotently skipped (no double-spend).
///
/// # Examples
///
/// ```rust
/// use program_escrow::{Storage, ReleaseEntry, release_schedule};
///
/// let mut storage = Storage::new();
/// storage.set_authorized_key("alice");
/// storage.set_balance(1_000);
/// storage.set_entries("RELEASE_HISTORY", vec![
///     ReleaseEntry {
///         id: 1, due_ledger: 100, amount: 500,
///         recipient: "bob".into(), released: false, depends_on: None,
///     },
/// ]);
///
/// let result = release_schedule(&mut storage, 100, "alice").unwrap();
/// assert_eq!(result.entries_released, 1);
/// assert_eq!(result.total_disbursed, 500);
/// ```
pub fn release_schedule(
    storage: &mut Storage,
    current_ledger: u64,
    caller: &str,
) -> Result<ReleaseResult, EscrowError> {
    // ── 1. Circuit breaker ────────────────────────────────────────────────────
    if storage.is_paused() {
        return Err(EscrowError::ProgramPaused);
    }

    // ── 2. Authorization ──────────────────────────────────────────────────────
    let authorized = storage.get_authorized_key();
    if authorized.as_deref() != Some(caller) {
        return Err(EscrowError::Unauthorized);
    }

    // ── 3. Load RELEASE_HISTORY exactly once ──────────────────────────────────
    //
    // OPTIMIZATION: This is the single storage read for the entire function.
    // All subsequent processing operates on `history` in memory.
    let mut history = storage.get_entries(RELEASE_HISTORY);

    // ── 4. Build a set of already-released IDs for dependency checking ────────
    //
    // We collect these up-front so dependency resolution uses the state
    // *before* this call, preventing intra-call cascading releases which
    // could introduce ordering ambiguity.
    let already_released: std::collections::HashSet<u64> = history
        .iter()
        .filter(|e| e.released)
        .map(|e| e.id)
        .collect();

    // ── 5. Process all due entries in memory ──────────────────────────────────
    let mut total_disbursed: u64 = 0;
    let mut released_ids: Vec<u64> = Vec::new();
    let mut dirty = false;

    for entry in history.iter_mut() {
        // Skip already-released entries (idempotency / double-spend guard).
        if entry.released {
            continue;
        }

        // Skip entries not yet due.
        if entry.due_ledger > current_ledger {
            continue;
        }

        // Enforce dependency constraints.  If the dependency hasn't been
        // released *before* this call, skip rather than error so that
        // independent milestones can still be paid out.
        if let Some(dep_id) = entry.depends_on {
            if !already_released.contains(&dep_id) {
                continue;
            }
        }

        // Accumulate disbursement amount — overflow-safe.
        total_disbursed = total_disbursed
            .checked_add(entry.amount)
            .ok_or(EscrowError::Overflow)?;

        // Mark released in memory — NO storage write yet.
        entry.released = true;
        released_ids.push(entry.id);
        dirty = true;
    }

    // ── 6. Early exit if nothing was processed ────────────────────────────────
    if !dirty {
        return Ok(ReleaseResult {
            entries_released: 0,
            total_disbursed: 0,
            released_ids: Vec::new(),
        });
    }

    // ── 7. Balance check and deduction ────────────────────────────────────────
    let balance = storage.get_balance();
    if balance < total_disbursed {
        return Err(EscrowError::InsufficientBalance);
    }
    storage.set_balance(balance - total_disbursed);

    // ── 8. Write RELEASE_HISTORY exactly once ─────────────────────────────────
    //
    // OPTIMIZATION: This is the single storage write for the entire function.
    // `dirty` guarantees we only pay the write fee when something changed.
    storage.set_entries(RELEASE_HISTORY, history);

    Ok(ReleaseResult {
        entries_released: released_ids.len(),
        total_disbursed,
        released_ids,
    })
}

// ─── Helper constructors (used in tests) ─────────────────────────────────────

impl ReleaseEntry {
    /// Constructs a basic entry with no dependency.
    pub fn new(id: u64, due_ledger: u64, amount: u64, recipient: &str) -> Self {
        Self {
            id,
            due_ledger,
            amount,
            recipient: recipient.to_string(),
            released: false,
            depends_on: None,
        }
    }

    /// Constructs an entry with a dependency on another entry's `id`.
    pub fn with_dependency(mut self, dep_id: u64) -> Self {
        self.depends_on = Some(dep_id);
        self
    }

    /// Marks this entry as already released (for seeding pre-released state).
    pub fn already_released(mut self) -> Self {
        self.released = true;
        self
    }
}
