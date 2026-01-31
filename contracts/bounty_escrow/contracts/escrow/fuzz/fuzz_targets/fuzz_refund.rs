#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

mod escrow_fuzz_utils;
use escrow_fuzz_utils::*;

// Fuzz target for refund function
// Tests all refund modes: Full, Partial, Custom
fuzz_target!(|data: &[u8]| {
    if data.len() < 32 {
        return;
    }

    let setup = FuzzTestSetup::new();

    // Parse parameters from fuzz data
    let bounty_id = bytes_to_u64(data, 0);
    let total_amount = bytes_to_i128(data, 8).abs().max(100); // Ensure reasonable amount
    let refund_amount = bytes_to_i128(data, 24).abs() % total_amount; // Partial refund amount
    
    // Mode selector (0=Full, 1=Partial, 2=Custom)
    let mode_selector = data.get(40).copied().unwrap_or(0) % 3;
    
    // Deadline selector (0=before, 1=after, 2=exact)
    let deadline_selector = data.get(41).copied().unwrap_or(0) % 3;

    // Set up locked escrow
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    if setup.escrow.try_lock_funds(&setup.depositor, &bounty_id, &total_amount, &deadline).is_err() {
        return;
    }

    // Set time based on deadline selector
    match deadline_selector {
        0 => setup.set_time(deadline - 1), // Before deadline
        1 => setup.set_time(deadline + 1), // After deadline
        _ => setup.set_time(deadline),     // At deadline
    }

    // Generate custom recipient if needed
    let custom_recipient = if data.len() >= 72 {
        address_from_bytes(&setup.env, &data[48..80])
    } else {
        Address::generate(&setup.env)
    };

    // Record balances before refund
    let contract_balance_before = setup.token.balance(&setup.escrow.address);
    let depositor_balance_before = setup.token.balance(&setup.depositor);

    // Execute refund based on mode
    let result = match mode_selector {
        0 => {
            // Full refund
            setup.escrow.try_refund(
                &bounty_id,
                &None::<i128>,
                &None::<Address>,
                &bounty_escrow::RefundMode::Full,
            )
        }
        1 => {
            // Partial refund
            setup.escrow.try_refund(
                &bounty_id,
                &Some(refund_amount.max(1)),
                &None::<Address>,
                &bounty_escrow::RefundMode::Partial,
            )
        }
        _ => {
            // Custom refund
            setup.escrow.try_refund(
                &bounty_id,
                &Some(refund_amount.max(1)),
                &Some(custom_recipient.clone()),
                &bounty_escrow::RefundMode::Custom,
            )
        }
    };

    match result {
        Ok(Ok(())) => {
            // Success case: verify state
            let escrow_info = setup.escrow.get_escrow_info(&bounty_id);
            
            // Verify status is either Refunded or PartiallyRefunded
            assert!(
                escrow_info.status == bounty_escrow::EscrowStatus::Refunded
                    || escrow_info.status == bounty_escrow::EscrowStatus::PartiallyRefunded
            );

            // Verify refund history was updated
            let refund_history = setup.escrow.get_refund_history(&bounty_id);
            assert!(!refund_history.is_empty());

            // For full refund, remaining should be 0
            if mode_selector == 0 {
                assert_eq!(escrow_info.remaining_amount, 0);
                assert_eq!(escrow_info.status, bounty_escrow::EscrowStatus::Refunded);
            }
        }
        Ok(Err(_)) => {
            // Expected error cases:
            // - Deadline not passed (for Full/Partial)
            // - Refund not approved (for Custom before deadline)
            // - Invalid amount
        }
        Err(_) => {
            // Unexpected error
        }
    }

    // Test double-refund edge case for partial refunds
    if mode_selector == 1 && result.is_ok() {
        let escrow_info = setup.escrow.get_escrow_info(&bounty_id);
        if escrow_info.remaining_amount > 0 {
            // Should be able to refund remaining
            let remaining = escrow_info.remaining_amount;
            let second_refund = setup.escrow.try_refund(
                &bounty_id,
                &Some(remaining),
                &None::<Address>,
                &bounty_escrow::RefundMode::Partial,
            );
            
            if second_refund.is_ok() {
                let final_info = setup.escrow.get_escrow_info(&bounty_id);
                assert_eq!(final_info.remaining_amount, 0);
                assert_eq!(final_info.status, bounty_escrow::EscrowStatus::Refunded);
            }
        }
    }
});
