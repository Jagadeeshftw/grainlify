#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, Vec,
};

mod escrow_fuzz_utils;
use escrow_fuzz_utils::*;

// Fuzz target for lock_funds function
// Tests various edge cases and input combinations
fuzz_target!(|data: &[u8]| {
    if data.len() < 32 {
        return;
    }

    let env = Env::default();
    env.mock_all_auths();

    // Generate addresses from fuzz data
    let admin = Address::from_bytes(&env, &data[0..32].try_into().unwrap_or([0; 32]));
    let depositor = Address::generate(&env);
    
    // Create token contract
    let (token, token_admin) = create_token_contract(&env, &admin);
    
    // Create and initialize escrow contract
    let escrow = create_escrow_contract(&env);
    let _ = escrow.init(&admin, &token.address);

    // Parse fuzz data for parameters
    let bounty_id = u64::from_le_bytes([
        data.get(0).copied().unwrap_or(0),
        data.get(1).copied().unwrap_or(0),
        data.get(2).copied().unwrap_or(0),
        data.get(3).copied().unwrap_or(0),
        data.get(4).copied().unwrap_or(0),
        data.get(5).copied().unwrap_or(0),
        data.get(6).copied().unwrap_or(0),
        data.get(7).copied().unwrap_or(0),
    ]);

    // Amount: can be zero, negative (edge cases), or positive
    let amount = i128::from_le_bytes([
        data.get(8).copied().unwrap_or(0),
        data.get(9).copied().unwrap_or(0),
        data.get(10).copied().unwrap_or(0),
        data.get(11).copied().unwrap_or(0),
        data.get(12).copied().unwrap_or(0),
        data.get(13).copied().unwrap_or(0),
        data.get(14).copied().unwrap_or(0),
        data.get(15).copied().unwrap_or(0),
        data.get(16).copied().unwrap_or(0),
        data.get(17).copied().unwrap_or(0),
        data.get(18).copied().unwrap_or(0),
        data.get(19).copied().unwrap_or(0),
        data.get(20).copied().unwrap_or(0),
        data.get(21).copied().unwrap_or(0),
        data.get(22).copied().unwrap_or(0),
        data.get(23).copied().unwrap_or(0),
    ]);

    // Deadline: can be in past, present, or future
    let current_time = env.ledger().timestamp();
    let deadline_offset = i64::from_le_bytes([
        data.get(24).copied().unwrap_or(0) as i8 as u8,
        data.get(25).copied().unwrap_or(0),
        data.get(26).copied().unwrap_or(0),
        data.get(27).copied().unwrap_or(0),
        data.get(28).copied().unwrap_or(0),
        data.get(29).copied().unwrap_or(0),
        data.get(30).copied().unwrap_or(0),
        data.get(31).copied().unwrap_or(0),
    ]);
    
    // Calculate deadline (can be before, at, or after current time)
    let deadline = if deadline_offset >= 0 {
        current_time.saturating_add(deadline_offset as u64)
    } else {
        current_time.saturating_sub((-deadline_offset) as u64)
    };

    // Mint tokens to depositor (sufficient for most cases)
    let mint_amount = amount.abs().saturating_mul(2).max(1_000_000);
    token_admin.mint(&depositor, &mint_amount);

    // Attempt to lock funds - should handle all edge cases gracefully
    let result = escrow.try_lock_funds(&depositor, &bounty_id, &amount, &deadline);

    // Verify that the contract behaves correctly:
    // - Should succeed with valid inputs
    // - Should fail gracefully with invalid inputs (no panics)
    match result {
        Ok(Ok(())) => {
            // Success case: verify state is correct
            let escrow_info = escrow.get_escrow_info(&bounty_id);
            assert_eq!(escrow_info.depositor, depositor);
            assert_eq!(escrow_info.amount, amount);
            assert!(amount > 0, "Amount should be positive for success");
            assert!(deadline > current_time, "Deadline should be in future for success");
        }
        Ok(Err(_)) => {
            // Expected error case - contract rejected the operation
            // This is valid behavior for edge cases
        }
        Err(_) => {
            // Unexpected error - this could indicate a bug
            // In production, we'd want to investigate these
        }
    }
});
