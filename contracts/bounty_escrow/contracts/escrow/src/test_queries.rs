use super::*;
use crate::test::TestSetup;
use soroban_sdk::{testutils::{Address as _, Ledger}, Address, Vec};

#[test]
fn test_get_bounties_filtering() {
    let setup = TestSetup::new();
    let bounty_id1 = 1;
    let bounty_id2 = 2;
    let bounty_id3 = 3;
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    // Create bounties with different properties
    setup.escrow.lock_funds(&setup.depositor, &bounty_id1, &1000, &deadline); // Locked
    
    setup.token_admin.mint(&setup.depositor, &5000);
    setup.escrow.lock_funds(&setup.depositor, &bounty_id2, &2000, &deadline); // Will release
    
    setup.escrow.lock_funds(&setup.depositor, &bounty_id3, &3000, &deadline); // Locked

    // Release bounty 2
    let contributor = Address::generate(&setup.env);
    setup.escrow.release_funds(&bounty_id2, &contributor);

    // Test 1: Filter by Status (Locked)
    // Note: EscrowStatus::Locked = 1
    let filter_locked = EscrowFilter {
        status: Some(1), // Locked
        min_amount: None,
        max_amount: None,
        min_deadline: None,
        max_deadline: None,
        depositor: None,
    };
    let pagination = Pagination { limit: 10, offset: 0 };
    
    let results_locked = setup.escrow.get_bounties(&filter_locked, &pagination);
    assert_eq!(results_locked.len(), 2);
    assert_eq!(results_locked.get(0).unwrap().bounty_id, bounty_id1);
    assert_eq!(results_locked.get(1).unwrap().bounty_id, bounty_id3);

    // Test 2: Filter by Status (Released)
    // Note: EscrowStatus::Released = 2
     let filter_released = EscrowFilter {
        status: Some(2), // Released
        min_amount: None,
        max_amount: None,
        min_deadline: None,
        max_deadline: None,
        depositor: None,
    };
    let results_released = setup.escrow.get_bounties(&filter_released, &pagination);
    assert_eq!(results_released.len(), 1);
    assert_eq!(results_released.get(0).unwrap().bounty_id, bounty_id2);

    // Test 3: Filter by Amount
    let filter_amount = EscrowFilter {
        status: None,
        min_amount: Some(1500),
        max_amount: None,
        min_deadline: None,
        max_deadline: None,
        depositor: None,
    };
    let results_amount = setup.escrow.get_bounties(&filter_amount, &pagination);
    assert_eq!(results_amount.len(), 2); // 2000 and 3000
    assert_eq!(results_amount.get(0).unwrap().bounty_id, bounty_id2);
    assert_eq!(results_amount.get(1).unwrap().bounty_id, bounty_id3);
}

#[test]
fn test_get_bounties_pagination() {
    let setup = TestSetup::new();
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    // Create 5 bounties
    setup.token_admin.mint(&setup.depositor, &10000);
    for i in 1..=5 {
        setup.escrow.lock_funds(&setup.depositor, &i, &1000, &deadline);
    }

    let filter_all = EscrowFilter {
        status: None,
        min_amount: None,
        max_amount: None,
        min_deadline: None,
        max_deadline: None,
        depositor: None,
    };

    // Page 1: Limit 2, Offset 0
    let page1 = setup.escrow.get_bounties(&filter_all, &Pagination { limit: 2, offset: 0 });
    assert_eq!(page1.len(), 2);
    assert_eq!(page1.get(0).unwrap().bounty_id, 1);
    assert_eq!(page1.get(1).unwrap().bounty_id, 2);

    // Page 2: Limit 2, Offset 2
    let page2 = setup.escrow.get_bounties(&filter_all, &Pagination { limit: 2, offset: 2 });
    assert_eq!(page2.len(), 2);
    assert_eq!(page2.get(0).unwrap().bounty_id, 3);
    assert_eq!(page2.get(1).unwrap().bounty_id, 4);

    // Page 3: Limit 2, Offset 4
    let page3 = setup.escrow.get_bounties(&filter_all, &Pagination { limit: 2, offset: 4 });
    assert_eq!(page3.len(), 1);
    assert_eq!(page3.get(0).unwrap().bounty_id, 5);
}

#[test]
fn test_get_contract_stats() {
    let setup = TestSetup::new();
    let bounty_id1 = 1;
    let bounty_id2 = 2;
    let bounty_id3 = 3;
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup.escrow.lock_funds(
        &setup.depositor, 
        &bounty_id1, 
        &100, 
        &deadline
    );

    setup.token_admin.mint(&setup.depositor, &5000);
    setup.escrow.lock_funds(
        &setup.depositor, 
        &bounty_id2, 
        &200, 
        &deadline
    );

    // Initial check
    let stats = setup.escrow.get_contract_stats();
    assert_eq!(stats.total_bounties, 2);
    assert_eq!(stats.total_locked_funds, 300);
    assert_eq!(stats.total_released_funds, 0);

    // Release one
    let contributor = Address::generate(&setup.env);
    setup.escrow.release_funds(
        &bounty_id1, 
        &contributor
    );

    let stats_after = setup.escrow.get_contract_stats();
    assert_eq!(stats_after.total_bounties, 2);
    assert_eq!(stats_after.total_locked_funds, 200); // Only bounty_id2 is locked
    assert_eq!(stats_after.total_released_funds, 100); // bounty_id1 released

    // Add another
    setup.escrow.lock_funds(
        &setup.depositor, 
        &bounty_id3, 
        &50, 
        &deadline
    );

    let stats_final = setup.escrow.get_contract_stats();
    assert_eq!(stats_final.total_bounties, 3);
    assert_eq!(stats_final.total_locked_funds, 250); // 200 + 50
    assert_eq!(stats_final.total_released_funds, 100);
}

#[test]
fn test_large_dataset_pagination_and_stats() {
    let setup = TestSetup::new();
    let start_time = setup.env.ledger().timestamp();
    // Set deadline far in future (1 year) to allow for time jumps
    let deadline = start_time + 31_536_000; 
    
    // Mint enough tokens for 50 bounties of 100 each
    setup.token_admin.mint(&setup.depositor, &10000);

    // Create 50 bounties
    for i in 1..=50 {
        setup.escrow.lock_funds(
            &setup.depositor, 
            &i, 
            &100, 
            &deadline
        );
        // Advance time by 1 day (86400 seconds) to strictly avoid any rate limits
        setup.env.ledger().set_timestamp(setup.env.ledger().timestamp() + 86400);
    }

    // Verify Stats
    let stats = setup.escrow.get_contract_stats();
    assert_eq!(stats.total_bounties, 50);
    assert_eq!(stats.total_locked_funds, 5000); // 50 * 100

    // Verify Pagination (5 pages of 10)
    let filter = EscrowFilter {
        status: None,
        min_amount: None,
        max_amount: None,
        min_deadline: None,
        max_deadline: None,
        depositor: None,
    };

    let mut total_fetched = 0;
    for page in 0..5 {
        let results = setup.escrow.get_bounties(
            &filter, 
            &Pagination { limit: 10, offset: (page * 10) as u64 }
        );
        assert_eq!(results.len(), 10, "Page {} should have 10 items", page);
        total_fetched += results.len();
    }
    
    assert_eq!(total_fetched, 50);
}
