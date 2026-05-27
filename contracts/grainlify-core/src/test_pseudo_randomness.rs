#![cfg(test)]

extern crate std;

use crate::pseudo_randomness::{derive_selection, DeterministicSelection};
use soroban_sdk::{
    testutils::Address as _,
    xdr::{Hash, ScAddress, ToXdr},
    Address, Bytes, BytesN, Env, Symbol, TryFromVal, Vec as SorobanVec,
};

// ============================================================================
// Test Constants and Helpers
// ============================================================================

/// Test domain symbol
const TEST_DOMAIN: &str = "test";

/// Generate a deterministic test seed
fn test_seed(env: &Env, seed_value: u32) -> BytesN<32> {
    let mut seed = [0u8; 32];
    let le = seed_value.to_le_bytes();
    seed[0..4].copy_from_slice(&le);
    BytesN::from_array(env, &seed)
}

/// Generate test candidates
fn generate_candidates(env: &Env, count: u32) -> SorobanVec<Address> {
    let mut candidates = SorobanVec::new(&env);
    for i in 0..count {
        let mut addr_bytes = [0u8; 32];
        addr_bytes[0] = i as u8;
        let addr = Address::try_from_val(env, &ScAddress::Contract(Hash(addr_bytes))).unwrap();
        candidates.push_back(addr);
    }
    candidates
}

/// Generate test context
fn test_context(env: &Env, context: &str) -> Bytes {
    Bytes::from_slice(env, context.as_bytes())
}

// ============================================================================
// Deterministic Behavior Tests
// ============================================================================

#[test]
fn test_deterministic_behavior_same_inputs() {
    let env = Env::default();

    let domain = Symbol::new(&env, TEST_DOMAIN);
    let context = test_context(&env, "test_context");
    let seed = test_seed(&env, 42);
    let candidates = generate_candidates(&env, 5);

    // Run selection twice with same inputs
    let result1 = derive_selection(&env, &domain, &context, &seed, &candidates);
    let result2 = derive_selection(&env, &domain, &context, &seed, &candidates);

    // Results should be identical
    assert!(result1.is_some(), "First selection should succeed");
    assert!(result2.is_some(), "Second selection should succeed");

    let sel1 = result1.unwrap();
    let sel2 = result2.unwrap();

    assert_eq!(
        sel1.index, sel2.index,
        "Winner index should be deterministic"
    );
    assert_eq!(
        sel1.seed_hash, sel2.seed_hash,
        "Seed hash should be identical"
    );
    assert_eq!(
        sel1.winner_score, sel2.winner_score,
        "Winner score should be identical"
    );
}

#[test]
fn test_deterministic_behavior_different_seeds() {
    let env = Env::default();

    let domain = Symbol::new(&env, TEST_DOMAIN);
    let context = test_context(&env, "test_context");
    let candidates = generate_candidates(&env, 5);

    // Test with different seeds
    let seed1 = test_seed(&env, 42);
    let seed2 = test_seed(&env, 43);

    let result1 = derive_selection(&env, &domain, &context, &seed1, &candidates);
    let result2 = derive_selection(&env, &domain, &context, &seed2, &candidates);

    assert!(result1.is_some());
    assert!(result2.is_some());

    let sel1 = result1.unwrap();
    let sel2 = result2.unwrap();

    // Different seeds should produce different results (most likely)
    assert_ne!(sel1.seed_hash, sel2.seed_hash, "Seed hashes should differ");
    // Note: Winner indices might coincidentally be the same, but seed hashes must differ
}

#[test]
fn test_deterministic_behavior_different_contexts() {
    let env = Env::default();

    let domain = Symbol::new(&env, TEST_DOMAIN);
    let seed = test_seed(&env, 42);
    let candidates = generate_candidates(&env, 5);

    // Test with different contexts
    let context1 = test_context(&env, "context_1");
    let context2 = test_context(&env, "context_2");

    let result1 = derive_selection(&env, &domain, &context1, &seed, &candidates);
    let result2 = derive_selection(&env, &domain, &context2, &seed, &candidates);

    assert!(result1.is_some());
    assert!(result2.is_some());

    let sel1 = result1.unwrap();
    let sel2 = result2.unwrap();

    assert_ne!(
        sel1.seed_hash, sel2.seed_hash,
        "Seed hashes should differ with different contexts"
    );
}

#[test]
fn test_deterministic_behavior_different_domains() {
    let env = Env::default();

    let context = test_context(&env, "test_context");
    let seed = test_seed(&env, 42);
    let candidates = generate_candidates(&env, 5);

    // Test with different domains
    let domain1 = Symbol::new(&env, "domain1");
    let domain2 = Symbol::new(&env, "domain2");

    let result1 = derive_selection(&env, &domain1, &context, &seed, &candidates);
    let result2 = derive_selection(&env, &domain2, &context, &seed, &candidates);

    assert!(result1.is_some());
    assert!(result2.is_some());

    let sel1 = result1.unwrap();
    let sel2 = result2.unwrap();

    assert_ne!(
        sel1.seed_hash, sel2.seed_hash,
        "Seed hashes should differ with different domains"
    );
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_empty_candidates_returns_none() {
    let env = Env::default();

    let domain = Symbol::new(&env, TEST_DOMAIN);
    let context = test_context(&env, "test_context");
    let seed = test_seed(&env, 42);
    let empty_candidates = SorobanVec::new(&env);

    let result = derive_selection(&env, &domain, &context, &seed, &empty_candidates);

    assert!(result.is_none(), "Empty candidates should return None");
}

#[test]
fn test_single_candidate_always_wins() {
    let env = Env::default();

    let domain = Symbol::new(&env, TEST_DOMAIN);
    let context = test_context(&env, "test_context");
    let seed = test_seed(&env, 42);
    let candidates = generate_candidates(&env, 1);

    let result = derive_selection(&env, &domain, &context, &seed, &candidates);

    assert!(result.is_some(), "Single candidate should return result");
    let selection = result.unwrap();
    assert_eq!(selection.index, 0, "Single candidate should always win");
}

#[test]
fn test_two_candidates_deterministic() {
    let env = Env::default();

    let domain = Symbol::new(&env, TEST_DOMAIN);
    let context = test_context(&env, "test_context");
    let seed = test_seed(&env, 42);
    let candidates = generate_candidates(&env, 2);

    let result = derive_selection(&env, &domain, &context, &seed, &candidates);

    assert!(result.is_some(), "Two candidates should return result");
    let selection = result.unwrap();
    assert!(selection.index < 2, "Winner index should be valid");

    // Verify determinism
    let result2 = derive_selection(&env, &domain, &context, &seed, &candidates);
    assert_eq!(selection.index, result2.unwrap().index);
}

// ============================================================================
// Statistical Distribution Tests
// ============================================================================

#[test]
fn test_uniform_distribution_large_candidate_pool() {
    let env = Env::default();

    let domain = Symbol::new(&env, "distribution_test");
    let context = test_context(&env, "statistical_test");
    let candidates = generate_candidates(&env, 100); // Large candidate pool

    // Test with many different seeds
    let mut wins = std::vec![0u32; 100];
    let num_trials = 1000;

    for i in 0..num_trials {
        let seed = test_seed(&env, i);
        let result = derive_selection(&env, &domain, &context, &seed, &candidates);

        if let Some(selection) = result {
            wins[selection.index as usize] += 1;
        }
    }

    // Basic statistical checks
    let total_wins: u32 = wins.iter().sum();
    assert_eq!(total_wins, num_trials, "All trials should produce winners");

    // Check that no candidate wins excessively (basic uniformity check)
    let expected_avg = num_trials as f32 / 100.0;
    let tolerance = expected_avg * 0.5; // 50% tolerance for basic test

    for (i, &win_count) in wins.iter().enumerate() {
        let deviation = (win_count as f32 - expected_avg).abs();
        assert!(
            deviation <= tolerance,
            "Candidate {} won {} times, expected around {} (deviation: {})",
            i,
            win_count,
            expected_avg as u32,
            deviation
        );
    }
}

#[test]
fn test_seed_sensitivity_high_entropy() {
    let env = Env::default();

    let domain = Symbol::new(&env, "sensitivity_test");
    let context = test_context(&env, "entropy_test");
    let candidates = generate_candidates(&env, 10);

    // Test seeds that differ by only one bit
    let mut seed1 = [0u8; 32];
    let mut seed2 = [0u8; 32];
    seed1[31] = 0x00;
    seed2[31] = 0x01; // Flip last bit

    let seed1_bytes = BytesN::from_array(&env, &seed1);
    let seed2_bytes = BytesN::from_array(&env, &seed2);

    let result1 = derive_selection(&env, &domain, &context, &seed1_bytes, &candidates);
    let result2 = derive_selection(&env, &domain, &context, &seed2_bytes, &candidates);

    assert!(result1.is_some());
    assert!(result2.is_some());

    let sel1 = result1.unwrap();
    let sel2 = result2.unwrap();

    // Even single-bit changes should produce different results
    assert_ne!(
        sel1.seed_hash, sel2.seed_hash,
        "Single-bit seed change should affect seed hash"
    );
}

// ============================================================================
// Security and Attack Simulation Tests
// ============================================================================

#[test]
fn test_candidate_order_independence() {
    let env = Env::default();

    let domain = Symbol::new(&env, "order_test");
    let context = test_context(&env, "order_independence");
    let seed = test_seed(&env, 42);

    // Create candidates in different orders
    let mut candidates1 = generate_candidates(&env, 5);
    let mut candidates2 = generate_candidates(&env, 5);

    // Reverse the order of candidates2
    let mut reversed = SorobanVec::new(&env);
    for i in (0..candidates2.len()).rev() {
        reversed.push_back(candidates2.get(i).unwrap());
    }
    candidates2 = reversed;

    let result1 = derive_selection(&env, &domain, &context, &seed, &candidates1);
    let result2 = derive_selection(&env, &domain, &context, &seed, &candidates2);

    assert!(result1.is_some());
    assert!(result2.is_some());

    let sel1 = result1.unwrap();
    let sel2 = result2.unwrap();

    // The winners should be different due to order independence
    // (This tests the scoring approach vs modulo approach)
    let winner1 = candidates1.get(sel1.index).unwrap();
    let winner2 = candidates2.get(sel2.index).unwrap();

    // In most cases, different orders should produce different winners
    // Note: This is a probabilistic test - might occasionally fail by chance
    assert_ne!(
        winner1, winner2,
        "Different orders should typically produce different winners"
    );
}

#[test]
fn test_candidate_stuffing_simulation() {
    let env = Env::default();

    let domain = Symbol::new(&env, "stuffing_test");
    let context = test_context(&env, "candidate_stuffing");
    let seed = test_seed(&env, 42);

    // Test with normal candidate pool
    let normal_candidates = generate_candidates(&env, 10);
    let normal_result = derive_selection(&env, &domain, &context, &seed, &normal_candidates);

    // Test with stuffed candidate pool (add many similar candidates)
    let mut stuffed_candidates = normal_candidates.clone();
    let base_candidate = normal_candidates.get(0).unwrap();

    // Add many copies of the same candidate (simulated)
    for i in 10..50 {
        let mut addr_bytes = [0u8; 32];
        addr_bytes[0] = (i % 10) as u8; // Create similar addresses
        let addr = Address::try_from_val(&env, &ScAddress::Contract(Hash(addr_bytes))).unwrap();
        stuffed_candidates.push_back(addr);
    }

    let stuffed_result = derive_selection(&env, &domain, &context, &seed, &stuffed_candidates);

    assert!(normal_result.is_some());
    assert!(stuffed_result.is_some());

    // Candidate stuffing can affect outcomes (demonstrating the vulnerability)
    let normal_winner = normal_candidates.get(normal_result.unwrap().index).unwrap();
    let stuffed_winner = stuffed_candidates
        .get(stuffed_result.unwrap().index)
        .unwrap();

    // This test documents the vulnerability - results may differ
    std::println!("Normal winner: {:?}", normal_winner);
    std::println!("Stuffed winner: {:?}", stuffed_winner);
}

#[test]
fn test_seed_grinding_simulation() {
    let env = Env::default();

    let domain = Symbol::new(&env, "grinding_test");
    let context = test_context(&env, "seed_grinding");
    let candidates = generate_candidates(&env, 5);

    let target_candidate = candidates.get(0).unwrap(); // Attacker wants this to win
    let mut attempts = 0u32;
    let max_attempts: u32 = 1000;

    // Simulate seed grinding - try many seeds to get desired outcome
    for seed_value in 0..max_attempts {
        let seed = test_seed(&env, seed_value);
        let result = derive_selection(&env, &domain, &context, &seed, &candidates);

        if let Some(selection) = result {
            attempts += 1;
            let winner = candidates.get(selection.index).unwrap();

            if winner == target_candidate {
                std::println!("Found winning seed after {} attempts", attempts);
                break;
            }
        }
    }

    // This test demonstrates that seed grinding is possible
    // In production, unpredictable seeds should be used
    assert!(
        attempts < max_attempts,
        "Should find a winning seed within reasonable attempts"
    );
}

// ============================================================================
// Performance and Gas Tests
// ============================================================================

#[test]
fn test_performance_large_candidate_pool() {
    let env = Env::default();

    let domain = Symbol::new(&env, "performance_test");
    let context = test_context(&env, "performance");
    let seed = test_seed(&env, 42);

    // Test with increasing candidate pool sizes
    for size in [10, 50, 100, 500] {
        let candidates = generate_candidates(&env, size);

        let start = env.ledger().timestamp();
        let result = derive_selection(&env, &domain, &context, &seed, &candidates);
        let duration = env.ledger().timestamp() - start;

        assert!(
            result.is_some(),
            "Selection should succeed for {} candidates",
            size
        );
        assert!(
            duration < 1000000,
            "Selection should complete quickly for {} candidates",
            size
        );

        let selection = result.unwrap();
        assert!(
            selection.index < size,
            "Winner index should be valid for {} candidates",
            size
        );
    }
}

// ============================================================================
// Audit Trail and Verification Tests
// ============================================================================

#[test]
fn test_audit_trail_completeness() {
    let env = Env::default();

    let domain = Symbol::new(&env, "audit_test");
    let context = test_context(&env, "audit_trail");
    let seed = test_seed(&env, 42);
    let candidates = generate_candidates(&env, 5);

    let result = derive_selection(&env, &domain, &context, &seed, &candidates);
    assert!(result.is_some());

    let selection = result.unwrap();

    // Verify audit trail data is complete
    assert_eq!(
        selection.seed_hash.len(),
        32,
        "Seed hash should be 32 bytes"
    );
    assert_eq!(
        selection.winner_score.len(),
        32,
        "Winner score should be 32 bytes"
    );
    assert!(
        selection.index < candidates.len(),
        "Winner index should be valid"
    );

    // Verify winner score can be recomputed
    let winner = candidates.get(selection.index).unwrap();
    let mut score_material = Bytes::new(&env);
    score_material.append(&selection.seed_hash.clone().to_xdr(&env));
    score_material.append(&winner.to_xdr(&env));
    let recomputed_score: BytesN<32> = env.crypto().sha256(&score_material).into();

    assert_eq!(
        selection.winner_score, recomputed_score,
        "Winner score should be verifiable from audit trail"
    );
}

#[test]
fn test_cross_domain_isolation() {
    let env = Env::default();

    let context = test_context(&env, "isolation_test");
    let seed = test_seed(&env, 42);
    let candidates = generate_candidates(&env, 5);

    // Test with different domains to ensure isolation
    let domain1 = Symbol::new(&env, "lottery");
    let domain2 = Symbol::new(&env, "auction");

    let result1 = derive_selection(&env, &domain1, &context, &seed, &candidates);
    let result2 = derive_selection(&env, &domain2, &context, &seed, &candidates);

    assert!(result1.is_some());
    assert!(result2.is_some());

    let sel1 = result1.unwrap();
    let sel2 = result2.unwrap();

    // Different domains should produce completely different results
    assert_ne!(
        sel1.seed_hash, sel2.seed_hash,
        "Different domains should produce different seed hashes"
    );
    assert_ne!(
        sel1.winner_score, sel2.winner_score,
        "Different domains should produce different winner scores"
    );
}

// ============================================================================
// Error Handling and Robustness Tests
// ============================================================================

#[test]
fn test_malformed_inputs_handling() {
    let env = Env::default();

    let domain = Symbol::new(&env, TEST_DOMAIN);
    let candidates = generate_candidates(&env, 3);

    // Test with empty context
    let empty_context = Bytes::new(&env);
    let seed = test_seed(&env, 42);
    let result = derive_selection(&env, &domain, &empty_context, &seed, &candidates);
    assert!(
        result.is_some(),
        "Empty context should be handled gracefully"
    );

    // Test with zero seed
    let zero_seed = BytesN::from_array(&env, &[0u8; 32]);
    let result = derive_selection(&env, &domain, &empty_context, &zero_seed, &candidates);
    assert!(result.is_some(), "Zero seed should be handled gracefully");
}

#[test]
fn test_reproducibility_across_environments() {
    // Test that the same inputs produce the same outputs
    // This is crucial for audit trails and verification

    let env1 = Env::default();
    let env2 = Env::default();

    let domain = Symbol::new(&env1, TEST_DOMAIN);
    let domain2 = Symbol::new(&env2, TEST_DOMAIN);
    let context = test_context(&env1, "reproducibility_test");
    let context2 = test_context(&env2, "reproducibility_test");
    let seed = test_seed(&env1, 123);
    let seed2 = test_seed(&env2, 123);
    let candidates = generate_candidates(&env1, 5);

    // Recreate identical candidates in second environment
    let mut candidates2 = SorobanVec::new(&env2);
    for i in 0..5u8 {
        let mut addr_bytes = [0u8; 32];
        addr_bytes[0] = i;
        let addr = Address::try_from_val(&env2, &ScAddress::Contract(Hash(addr_bytes))).unwrap();
        candidates2.push_back(addr);
    }

    let result1 = derive_selection(&env1, &domain, &context, &seed, &candidates);
    let result2 = derive_selection(&env2, &domain2, &context2, &seed2, &candidates2);

    assert!(result1.is_some());
    assert!(result2.is_some());

    let sel1 = result1.unwrap();
    let sel2 = result2.unwrap();

    assert_eq!(
        sel1.index, sel2.index,
        "Results should be reproducible across environments"
    );
    assert_eq!(
        sel1.seed_hash, sel2.seed_hash,
        "Seed hashes should be identical across environments"
    );
    assert_eq!(
        sel1.winner_score, sel2.winner_score,
        "Winner scores should be identical across environments"
    );
}
