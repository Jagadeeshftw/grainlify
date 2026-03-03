//! Standalone reentrancy guard tests that can be compiled independently
//!
//! These tests verify the core reentrancy guard functionality without
//! depending on the full contract implementation.

#![cfg(test)]

use crate::reentrancy_guard::*;
use soroban_sdk::Env;

#[test]
fn test_guard_initially_not_set() {
    let env = Env::default();
    let contract_id = env.register_contract(None, crate::ProgramEscrowContract);
    env.as_contract(&contract_id, || {
    assert!(!is_entered(&env), "Guard should not be set initially");

    });}

#[test]
fn test_guard_can_be_set_and_cleared() {
    let env = Env::default();
    let contract_id = env.register_contract(None, crate::ProgramEscrowContract);
    env.as_contract(&contract_id, || {

    // Initially not set
    assert!(!is_entered(&env));

    // Set the guard
    set_entered(&env);
    assert!(is_entered(&env), "Guard should be set after set_entered");

    // Clear the guard
    clear_entered(&env);
    assert!(
        !is_entered(&env),
        "Guard should be cleared after clear_entered"
    );

    });}

#[test]
fn test_check_passes_when_not_entered() {
    let env = Env::default();
    let contract_id = env.register_contract(None, crate::ProgramEscrowContract);
    env.as_contract(&contract_id, || {

    // Should not panic
    check_not_entered(&env);

    });}

#[test]
#[should_panic(expected = "Reentrancy detected")]
fn test_check_panics_when_entered() {
    let env = Env::default();
    let contract_id = env.register_contract(None, crate::ProgramEscrowContract);
    env.as_contract(&contract_id, || {

    // Set the guard
    set_entered(&env);

    // This should panic
    check_not_entered(&env);

    });}

#[test]
fn test_multiple_set_clear_cycles() {
    let env = Env::default();
    let contract_id = env.register_contract(None, crate::ProgramEscrowContract);
    env.as_contract(&contract_id, || {

    for _ in 0..5 {
        // Check passes
        check_not_entered(&env);

        // Set guard
        set_entered(&env);
        assert!(is_entered(&env));

        // Clear guard
        clear_entered(&env);
        assert!(!is_entered(&env));
    }

    });}

#[test]
fn test_guard_state_persistence() {
    let env = Env::default();
    let contract_id = env.register_contract(None, crate::ProgramEscrowContract);
    env.as_contract(&contract_id, || {

    // Set guard
    set_entered(&env);

    // Verify it persists across multiple checks
    assert!(is_entered(&env));
    assert!(is_entered(&env));
    assert!(is_entered(&env));

    // Clear and verify
    clear_entered(&env);
    assert!(!is_entered(&env));
    assert!(!is_entered(&env));

    });}

#[test]
#[should_panic(expected = "Reentrancy detected")]
fn test_double_set_detected() {
    let env = Env::default();
    let contract_id = env.register_contract(None, crate::ProgramEscrowContract);
    env.as_contract(&contract_id, || {

    // First set
    set_entered(&env);

    // Check should fail
    check_not_entered(&env);

    });}

#[test]
fn test_clear_when_not_set_is_safe() {
    let env = Env::default();
    let contract_id = env.register_contract(None, crate::ProgramEscrowContract);
    env.as_contract(&contract_id, || {

    // Clearing when not set should be safe
    clear_entered(&env);
    assert!(!is_entered(&env));

    // Can still set after clearing
    set_entered(&env);
    assert!(is_entered(&env));

    });}

#[test]
fn test_guard_isolation_between_envs() {
    let env1 = Env::default();
    let env2 = Env::default();
    let c1 = env1.register_contract(None, crate::ProgramEscrowContract);
    let c2 = env2.register_contract(None, crate::ProgramEscrowContract);
    env1.as_contract(&c1, || {
    env2.as_contract(&c2, || {

    // Set guard in env1
    set_entered(&env1);

    // env2 should not be affected
    assert!(is_entered(&env1));
    assert!(!is_entered(&env2));

    // Set guard in env2
    set_entered(&env2);

    // Both should be set
    assert!(is_entered(&env1));
    assert!(is_entered(&env2));

    // Clear env1
    clear_entered(&env1);

    // Only env1 should be cleared
    assert!(!is_entered(&env1));
    assert!(is_entered(&env2));

    });
    })}

#[test]
fn test_sequential_protected_operations() {
    let env = Env::default();
    let contract_id = env.register_contract(None, crate::ProgramEscrowContract);
    env.as_contract(&contract_id, || {

    // Simulate 3 sequential protected operations
    for i in 0..3 {
        // Check guard is clear
        check_not_entered(&env);

        // Set guard (operation starts)
        set_entered(&env);

        // Verify guard is set
        assert!(
            is_entered(&env),
            "Guard should be set during operation {}",
            i
        );

        // Clear guard (operation completes)
        clear_entered(&env);

        // Verify guard is cleared
        assert!(
            !is_entered(&env),
            "Guard should be cleared after operation {}",
            i
        );
    }

    });}
