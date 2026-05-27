//! # Pseudo-Randomness Helpers for On-Chain Selection
//!
//! **⚠️ CRITICAL SECURITY WARNING**: This module provides **deterministic pseudo-randomness**,
//! **NOT** true cryptographic randomness. It is **NOT suitable** for:
//! - Gambling applications
//! - High-value lottery systems  
//! - Security-critical random number generation
//! - Any use case where economic incentives exist to manipulate outcomes
//!
//! ## Overview
//!
//! This module implements deterministic selection algorithms that are:
//! - **Fully deterministic** given the same inputs
//! - **Replayable** from public blockchain data
//! - **Resistant to order-manipulation** attacks
//! - **Predictable** if all inputs are known
//!
//! ## Security Model & Limitations
//!
//! ### 🚨 Major Security Limitations
//!
//! 1. **No VRF Guarantees**: This is **NOT** a Verifiable Random Function (VRF)
//! 2. **Predictable Outcomes**: Anyone with all inputs can compute the result
//! 3. **Seed Grinding**: Attackers can try many seeds to influence outcomes
//! 4. **Timing Attacks**: Ledger metadata can be influenced by validators
//! 5. **Candidate Stuffing**: Adding sybil candidates affects probabilities
//!
//! ### 🎯 Acceptable Use Cases
//!
//! ✅ **Safe for**:
//! - Low-stakes selection (e.g., randomized order processing)
//! - Fair ordering when economic incentives are minimal
//! - Tie-breaking in deterministic systems
//! - Demonstrations and testing environments
//!
//! ❌ **Unsafe for**:
//! - Gambling or betting systems
//! - High-value lotteries or raffles
//! - Security-critical random number generation
//! - Any system with significant economic manipulation incentives
//!
//! ## Attack Vectors & Mitigations
//!
//! ### 1. Seed Grinding Attacks
//! **Attack**: Attacker tries many external seeds off-chain to get desired outcome
//! **Mitigation**:
//! - Use unpredictable external seeds (commit-reveal schemes)
//! - Include time-delayed reveals
//! - Add multiple independent entropy sources
//!
//! ### 2. Timing Manipulation  
//! **Attack**: Attacker submits transactions when ledger metadata favors them
//! **Mitigation**:
//! - Use ledger-independent entropy sources
//! - Include commit-reveal data from multiple parties
//! - Add time-locked reveal periods
//!
//! ### 3. Candidate Stuffing
//! **Attack**: Attacker adds many sybil candidates to increase win probability
//! **Mitigation**:
//! - Validate candidate eligibility
//! - Limit candidate pool size
//! - Use stake-weighted selection if applicable
//!
//! ## Implementation Details
//!
//! ### Deterministic Selection Algorithm
//!
//! The selection process uses a **scoring-based approach** rather than modulo selection:
//!
//! ```text
//! 1. Create seed_hash = SHA256(domain || context || external_seed)
//! 2. For each candidate:
//!    candidate_score = SHA256(seed_hash || candidate_address)
//! 3. Select candidate with highest candidate_score
//! ```
//!
//! This prevents **order-manipulation attacks** that affect `hash % n` approaches.
//!
//! ### Input Parameters
//!
//! - **`domain`**: Context identifier to prevent cross-domain collisions
//! - **`context`**: Additional entropy (e.g., previous state, commit data)
//! - **`external_seed`**: 32-byte external entropy source
//! - **`candidates`**: Address vector for selection
//!
//! ## Security Best Practices
//!
//! ### 🛡️ Production Deployment
//!
//! 1. **Multiple Entropy Sources**: Combine ledger data, commit-reveals, oracle data
//! 2. **Time Delays**: Use delayed reveals to prevent last-minute manipulation
//! 3. **Candidate Validation**: Verify candidate eligibility before selection
//! 4. **Audit Trail**: Log all selection parameters for transparency
//!
//! ### 🔍 Testing & Validation
//!
//! 1. **Deterministic Tests**: Verify same inputs produce same outputs
//! 2. **Statistical Analysis**: Check uniform distribution under random inputs
//! 3. **Adversarial Testing**: Simulate grinding and timing attacks
//! 4. **Edge Case Testing**: Empty candidates, single candidate, etc.
//!
//! ## Alternative Solutions
//!
//! For high-security applications requiring true randomness, consider:
//!
//! - **Chainlink VRF**: Verifiable Random Function with cryptographic guarantees
//! - **Stellar-native randomness**: Future protocol-level randomness features
//! - **Multi-party computation**: Threshold signature-based randomness
//! - **Oracle-based randomness**: Trusted external entropy providers
//!
//! ## Example Usage
//!
//! ```rust
//! use soroban_sdk::{symbol_short, Bytes, BytesN, Env, Vec};
//! use grainlify_core::pseudo_randomness::derive_selection;
//!
//! let env = Env::default();
//! let domain = symbol_short!("lottery");
//! let context = Bytes::from_slice(&env, b"round_42");
//! let external_seed = BytesN::from_array(&env, &[0x01; 32]);
//! let candidates = Vec::new(&env);
//! // ... add candidate addresses ...
//!
//! let selection = derive_selection(&env, &domain, &context, &external_seed, &candidates);
//! ```
//!
//! ## Version History
//!
//! - **v1.0**: Initial deterministic selection implementation
//! - **v1.1**: Enhanced security documentation and attack mitigations
//! - **v1.2**: Added comprehensive test suite and statistical analysis

use core::cmp::Ordering;
use soroban_sdk::xdr::ToXdr;
use soroban_sdk::{Address, Bytes, BytesN, Env, Symbol, Vec};

/// Result of a deterministic selection process.
///
/// Contains the winning candidate index and the cryptographic values
/// used in the selection process for auditability and verification.
///
/// # Fields
/// * `index` - Index of the winning candidate in the input vector
/// * `seed_hash` - SHA256 hash of all input parameters (domain || context || external_seed)
/// * `winner_score` - SHA256 hash that determined this candidate as winner
///
/// # Security Notes
/// - All fields are publicly verifiable given the same inputs
/// - This data enables audit trails and reproducibility checks
/// - Winner score is computed as SHA256(seed_hash || winner_address)
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeterministicSelection {
    pub index: u32,
    pub seed_hash: BytesN<32>,
    pub winner_score: BytesN<32>,
}

fn cmp_hash(env: &Env, a: &BytesN<32>, b: &BytesN<32>) -> Ordering {
    let ax = a.clone().to_xdr(env);
    let bx = b.clone().to_xdr(env);
    let mut i: u32 = 0;
    while i < ax.len() && i < bx.len() {
        let av = ax.get(i).unwrap();
        let bv = bx.get(i).unwrap();
        if av < bv {
            return Ordering::Less;
        }
        if av > bv {
            return Ordering::Greater;
        }
        i += 1;
    }
    ax.len().cmp(&bx.len())
}

/// Build the base seed hash from domain, context, and external seed.
///
/// This function creates the foundational hash used for all candidate scoring.
/// The deterministic nature ensures reproducibility while the combination of
/// inputs provides sufficient entropy for low-stakes applications.
///
/// # Arguments
/// * `env` - Soroban environment for crypto operations
/// * `domain` - Symbol identifying the selection domain
/// * `context` - Additional entropy/context data
/// * `external_seed` - 32-byte external entropy source
///
/// # Returns
/// * `BytesN<32>` - SHA256 hash of concatenated inputs
///
/// # Security Notes
/// - The same inputs always produce the same output
/// - Domain separation prevents cross-domain collisions
/// - External seed should be unpredictable in production
fn build_seed_hash(
    env: &Env,
    domain: &Symbol,
    context: &Bytes,
    external_seed: &BytesN<32>,
) -> BytesN<32> {
    let mut seed_material = Bytes::new(env);
    seed_material.append(&domain.to_xdr(env));
    seed_material.append(context);
    seed_material.append(&external_seed.clone().to_xdr(env));
    env.crypto().sha256(&seed_material).into()
}

/// Derive a deterministic winner from candidates using cryptographic scoring.
///
/// **⚠️ SECURITY WARNING**: This function provides **deterministic pseudo-randomness**,
/// **NOT** true cryptographic randomness. See module-level documentation for critical
/// security limitations and acceptable use cases.
///
/// # Algorithm
/// ```text
/// 1. seed_hash = SHA256(domain || context || external_seed)
/// 2. For each candidate i:
///    score_i = SHA256(seed_hash || candidate_i)
/// 3. Select candidate with highest score (lexicographic comparison)
/// ```
///
/// This scoring-based approach prevents order-manipulation attacks that affect
/// `hash % n` selection methods.
///
/// # Arguments
/// * `env` - Soroban environment for crypto operations
/// * `domain` - Symbol identifying the selection domain (prevents cross-domain collisions)
/// * `context` - Additional entropy/context data (e.g., round number, commit data)
/// * `external_seed` - 32-byte external entropy source (should be unpredictable)
/// * `candidates` - Vector of candidate addresses to select from
///
/// # Returns
/// * `Some(DeterministicSelection)` - Winner with audit trail data
/// * `None` - When candidates vector is empty
///
/// # Security Requirements
/// - `external_seed` should be unpredictable to prevent seed grinding attacks
/// - `context` should include relevant state data for additional entropy
/// - `domain` should be unique per application to prevent cross-contamination
///
/// # Deterministic Behavior
/// Given the same inputs, this function will always return the same result.
/// This enables:
/// - Audit trails and verification
/// - Test reproducibility
/// - State reconstruction from blockchain data
///
/// # Attack Resistance
/// - **Order manipulation**: Resistant due to scoring vs modulo approach
/// - **Candidate stuffing**: Still vulnerable - validate candidate eligibility
/// - **Seed grinding**: Vulnerable - use unpredictable external seeds
/// - **Timing attacks**: Vulnerable - use ledger-independent entropy
///
/// # Performance
/// - Time: O(n) where n = number of candidates
/// - Space: O(1) additional storage
/// - Gas: Proportional to candidate count
///
/// # Examples
/// ```text
/// (See tests and on-chain usage; doctests use testutils for addresses.)
/// ```
///
/// # Panics
/// This function does not panic under normal conditions. It returns `None` for
/// empty candidate vectors rather than panicking.
pub fn derive_selection(
    env: &Env,
    domain: &Symbol,
    context: &Bytes,
    external_seed: &BytesN<32>,
    candidates: &Vec<Address>,
) -> Option<DeterministicSelection> {
    if candidates.is_empty() {
        return None;
    }

    let seed_hash = build_seed_hash(env, domain, context, external_seed);

    let mut best_idx: u32 = 0;
    let mut best_score: Option<BytesN<32>> = None;
    let mut i: u32 = 0;

    while i < candidates.len() {
        let candidate = candidates.get(i).unwrap();
        let mut score_material = Bytes::new(env);
        score_material.append(&seed_hash.clone().to_xdr(env));
        score_material.append(&candidate.to_xdr(env));
        let score: BytesN<32> = env.crypto().sha256(&score_material).into();

        match &best_score {
            None => {
                best_score = Some(score);
                best_idx = i;
            }
            Some(current_best) => {
                if cmp_hash(env, &score, current_best) == Ordering::Greater {
                    best_score = Some(score);
                    best_idx = i;
                }
            }
        }
        i += 1;
    }

    Some(DeterministicSelection {
        index: best_idx,
        seed_hash,
        winner_score: best_score.unwrap(),
    })
}
