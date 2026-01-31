//! Example test file demonstrating usage of test-utils library.
//!
//! This file shows how to use the test utilities to write cleaner, more maintainable tests.

#![cfg(test)]

use test_utils::*;
use bounty_escrow::EscrowStatus;

#[test]
fn example_basic_test() {
    // Create a test setup with all components initialized
    let setup = TestSetup::new();
    
    // Use generators for test data
    let bounty_id = generate_bounty_id(None);
    let amount = standard_amount();
    let deadline = future_deadline(&setup.env, Some(3600));
    
    // Lock funds using convenience method
    setup.lock_funds(bounty_id, amount, deadline);
    
    // Use assertion utilities
    assert_escrow_status(&setup.escrow, bounty_id, EscrowStatus::Locked);
    assert_escrow_amount(&setup.escrow, bounty_id, amount);
    assert_escrow_depositor(&setup.escrow, bounty_id, &setup.depositor);
}

#[test]
fn example_balance_verification() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = future_deadline(&setup.env, None);
    
    // Get initial balances
    let initial_contributor = get_initial_balance(&setup.token, &setup.contributor);
    let initial_escrow = get_initial_balance(&setup.token, &setup.escrow_address);
    
    // Lock funds
    setup.lock_funds(bounty_id, amount, deadline);
    
    // Verify escrow received funds
    verify_balance_change(&setup.token, &setup.escrow_address, initial_escrow, amount);
    
    // Release funds
    setup.release_funds(bounty_id, None);
    
    // Verify contributor received funds
    verify_balance_change(&setup.token, &setup.contributor, initial_contributor, amount);
}

#[test]
fn example_multiple_contributors() {
    let (setup, contributors) = TestSetup::with_contributors(3);
    let bounty_id = 1;
    let total_amount = 3000;
    let deadline = future_deadline(&setup.env, None);
    
    setup.lock_funds(bounty_id, total_amount, deadline);
    
    // Release to each contributor
    for contributor in contributors.iter() {
        setup.escrow.release_funds(&bounty_id, contributor);
    }
}

#[test]
fn example_time_manipulation() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    
    // Create a deadline in the past
    let past_deadline = past_deadline(&setup.env, Some(100));
    setup.lock_funds(bounty_id, amount, past_deadline);
    
    // Advance time
    advance_time(&setup.env, 200);
    
    // Now refund should be allowed
    // (This would require refund functionality to be tested)
}

#[test]
fn example_custom_mint_amount() {
    let setup = TestSetup::with_mint_amount(5_000_000);
    
    // Depositor now has 5,000,000 tokens
    assert_balance(&setup.token, &setup.depositor, 5_000_000);
}
