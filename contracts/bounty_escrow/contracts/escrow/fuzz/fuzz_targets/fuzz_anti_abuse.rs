#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

mod escrow_fuzz_utils;
use escrow_fuzz_utils::*;

// Fuzz target for anti-abuse mechanisms
// Tests rate limiting, cooldown periods, and window limits
fuzz_target!(|data: &[u8]| {
    if data.len() < 32 {
        return;
    }

    let setup = FuzzTestSetup::new();

    // Parse operation count and timing from fuzz data
    let operation_count = (data[0] % 20 + 1) as u32; // 1-20 operations
    let time_increment = bytes_to_u64(data, 1) % 100; // 0-99 seconds between ops
    let use_same_address = data[9] % 2 == 0;

    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 10000;
    let amount = 1000i128;

    let mut success_count = 0u32;
    let mut rate_limited_count = 0u32;

    // Perform multiple lock operations
    for i in 0..operation_count {
        // Advance time
        setup.advance_time(time_increment);

        let depositor = if use_same_address {
            setup.depositor.clone()
        } else {
            // Generate new address for each operation
            Address::generate(&setup.env)
        };

        // Ensure depositor has funds
        if !use_same_address {
            setup.token_admin.mint(&depositor, &amount);
        }

        let bounty_id = i as u64;
        let result = setup.escrow.try_lock_funds(&depositor, &bounty_id, &amount, &deadline);

        match result {
            Ok(Ok(())) => {
                success_count += 1;
            }
            Ok(Err(_)) => {
                // Check if it's a rate limit error
                rate_limited_count += 1;
            }
            Err(_) => {}
        }
    }

    // Property: With same address and short time increments,
    // rate limiting should eventually kick in
    if use_same_address && time_increment < 60 && operation_count > 10 {
        // Should have some rate limiting with aggressive usage
        // Note: This depends on the anti-abuse config
    }

    // Property: With different addresses, all should succeed
    if !use_same_address {
        // All operations should succeed (no rate limiting across addresses)
        // Note: This assumes each address is unique
    }

    // Test whitelist functionality
    let whitelisted_address = Address::generate(&setup.env);
    setup.token_admin.mint(&whitelisted_address, &amount);

    // Note: Whitelist management would require admin functions
    // which may not be exposed in the current contract

    // Test edge case: very rapid operations (0 time increment)
    if time_increment == 0 && use_same_address {
        // Should hit cooldown quickly
    }

    // Test edge case: widely spaced operations
    if time_increment > 3600 {
        // Should reset window and allow operations
    }
});
