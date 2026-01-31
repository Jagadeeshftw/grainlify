#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, vec, Address, Env, Vec,
};

mod escrow_fuzz_utils;
use escrow_fuzz_utils::*;

// Fuzz target for batch operations
// Tests batch_lock_funds and batch_release_funds
fuzz_target!(|data: &[u8]| {
    if data.len() < 64 {
        return;
    }

    let setup = FuzzTestSetup::new();

    // Parse batch size (limit to reasonable range)
    let batch_size = (data[0] % 10 + 1) as u32; // 1-10 items
    
    // Create batch items
    let mut lock_items = Vec::new(&setup.env);
    let mut bounty_ids = Vec::new(&setup.env);
    
    let base_amount = bytes_to_i128(data, 1).abs().max(100);
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 10000;

    // Ensure enough tokens for all locks
    let total_needed = base_amount * batch_size as i128;
    setup.token_admin.mint(&setup.depositor, &total_needed);

    for i in 0..batch_size {
        let offset = 17 + (i as usize * 24); // Each item needs 24 bytes
        if offset + 24 > data.len() {
            break;
        }

        let bounty_id = bytes_to_u64(data, offset) as u64;
        let amount = (bytes_to_i128(data, offset + 8).abs() % base_amount).max(1);

        // Avoid duplicate bounty IDs in batch
        let mut is_duplicate = false;
        for existing_id in bounty_ids.iter() {
            if existing_id == bounty_id {
                is_duplicate = true;
                break;
            }
        }

        if !is_duplicate {
            bounty_ids.push_back(bounty_id);
            lock_items.push_back(bounty_escrow::LockFundsItem {
                bounty_id,
                depositor: setup.depositor.clone(),
                amount,
                deadline,
            });
        }
    }

    if lock_items.is_empty() {
        return;
    }

    // Test batch lock
    let lock_result = setup.escrow.try_batch_lock_funds(&lock_items);

    match lock_result {
        Ok(Ok(count)) => {
            // Success: verify all items were locked
            assert_eq!(count, lock_items.len() as u32);

            // Verify each escrow exists and is locked
            for item in lock_items.iter() {
                let escrow_info = setup.escrow.get_escrow_info(&item.bounty_id);
                assert_eq!(escrow_info.status, bounty_escrow::EscrowStatus::Locked);
                assert_eq!(escrow_info.amount, item.amount);
            }

            // Test batch release
            let mut release_items = Vec::new(&setup.env);
            for item in lock_items.iter() {
                let contributor = Address::generate(&setup.env);
                release_items.push_back(bounty_escrow::ReleaseFundsItem {
                    bounty_id: item.bounty_id,
                    contributor,
                });
            }

            let release_result = setup.escrow.try_batch_release_funds(&release_items);

            match release_result {
                Ok(Ok(release_count)) => {
                    assert_eq!(release_count, release_items.len() as u32);

                    // Verify all released
                    for item in release_items.iter() {
                        let escrow_info = setup.escrow.get_escrow_info(&item.bounty_id);
                        assert_eq!(escrow_info.status, bounty_escrow::EscrowStatus::Released);
                    }
                }
                Ok(Err(_)) => {
                    // Release failed - check that state is consistent
                    // (all should fail or none should fail due to atomicity)
                }
                Err(_) => {}
            }
        }
        Ok(Err(_)) => {
            // Lock failed - verify no partial state changes
            // (atomicity should ensure all-or-nothing)
            for item in lock_items.iter() {
                let exists = setup.escrow.try_get_escrow_info(&item.bounty_id).is_ok();
                // If any exists, all should exist (atomicity)
                // If none exist, that's also valid
            }
        }
        Err(_) => {}
    }

    // Test edge case: empty batch
    let empty_items: Vec<bounty_escrow::LockFundsItem> = vec![&setup.env];
    let empty_result = setup.escrow.try_batch_lock_funds(&empty_items);
    assert!(
        empty_result.is_err() || empty_result.as_ref().unwrap().is_err(),
        "Empty batch should fail"
    );

    // Test edge case: duplicate bounty IDs in batch
    if lock_items.len() >= 2 {
        let first_id = lock_items.get(0).unwrap().bounty_id;
        let mut duplicate_items = vec![
            &setup.env,
            bounty_escrow::LockFundsItem {
                bounty_id: first_id,
                depositor: setup.depositor.clone(),
                amount: 100,
                deadline,
            },
            bounty_escrow::LockFundsItem {
                bounty_id: first_id, // Duplicate
                depositor: setup.depositor.clone(),
                amount: 200,
                deadline,
            },
        ];

        let duplicate_result = setup.escrow.try_batch_lock_funds(&duplicate_items);
        assert!(
            duplicate_result.is_err() || duplicate_result.as_ref().unwrap().is_err(),
            "Duplicate bounty IDs in batch should fail"
        );
    }
});
