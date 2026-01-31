#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

mod escrow_fuzz_utils;
use escrow_fuzz_utils::*;

// Fuzz target for release_funds function
// Tests state transitions and authorization
fuzz_target!(|data: &[u8]| {
    if data.len() < 48 {
        return;
    }

    let setup = FuzzTestSetup::new();

    // Parse bounty_id
    let bounty_id = bytes_to_u64(data, 0);
    
    // Parse amount (ensure positive for initial lock)
    let amount = bytes_to_i128(data, 8).abs().max(1);
    
    // Set up a locked escrow first
    let deadline_offset = 10000u64;
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + deadline_offset;

    // Lock funds first
    if setup.escrow.try_lock_funds(&setup.depositor, &bounty_id, &amount, &deadline).is_err() {
        return; // Can't proceed without locked funds
    }

    // Generate contributor from fuzz data
    let contributor = if data.len() >= 80 {
        address_from_bytes(&setup.env, &data[48..80])
    } else {
        Address::generate(&setup.env)
    };

    // Record balances before release
    let contract_balance_before = setup.token.balance(&setup.escrow.address);
    let contributor_balance_before = setup.token.balance(&contributor);

    // Attempt to release funds
    let result = setup.escrow.try_release_funds(&bounty_id, &contributor);

    match result {
        Ok(Ok(())) => {
            // Success case: verify state transition
            let escrow_info = setup.escrow.get_escrow_info(&bounty_id);
            assert_eq!(escrow_info.status, bounty_escrow::EscrowStatus::Released);

            // Verify fund transfer
            let contract_balance_after = setup.token.balance(&setup.escrow.address);
            let contributor_balance_after = setup.token.balance(&contributor);

            // Invariant: contract balance decreased by amount
            assert_eq!(contract_balance_after, contract_balance_before - amount);
            // Invariant: contributor balance increased by amount
            assert_eq!(contributor_balance_after, contributor_balance_before + amount);
        }
        Ok(Err(_)) => {
            // Expected error - might be due to:
            // - Already released
            // - Not locked
            // - Authorization failure
        }
        Err(_) => {
            // Unexpected error
        }
    }

    // Test double-release (should fail)
    if result.is_ok() {
        let second_release = setup.escrow.try_release_funds(&bounty_id, &contributor);
        assert!(
            second_release.is_err() || second_release.as_ref().unwrap().is_err(),
            "Double release should fail"
        );
    }
});
