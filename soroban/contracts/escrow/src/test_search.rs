#![cfg(test)]
//! Pagination and criteria-filtering tests for `get_escrows`.

use super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{token, Address, Env};

fn create_token<'a>(
    env: &'a Env,
    admin: &Address,
) -> (Address, token::Client<'a>, token::StellarAssetClient<'a>) {
    let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
    let addr = token_contract.address();
    let client = token::Client::new(env, &addr);
    let admin_client = token::StellarAssetClient::new(env, &addr);
    (addr, client, admin_client)
}

fn setup_search<'a>(
    env: &'a Env,
    initial_balance: i128,
) -> (
    EscrowContractClient<'a>,
    Address, // contract_id
    Address, // admin
    Address, // depositor
    Address, // contributor
    token::Client<'a>,
) {
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(env, &contract_id);

    let admin = Address::generate(env);
    let depositor = Address::generate(env);
    let contributor = Address::generate(env);
    let (token_addr, token_client, token_admin) = create_token(env, &admin);

    client.init(&admin, &token_addr);
    token_admin.mint(&depositor, &initial_balance);

    (
        client,
        contract_id,
        admin,
        depositor,
        contributor,
        token_client,
    )
}

// ==================== EMPTY STATE ====================

#[test]
fn test_search_empty_contract() {
    let env = Env::default();
    let (client, _cid, _admin, _depositor, _contributor, _tc) = setup_search(&env, 0);

    // status_filter 0 = match any
    let criteria = EscrowSearchCriteria {
        status_filter: 0,
        depositor: None,
    };
    let page = client.get_escrows(&criteria, &None, &10);
    assert_eq!(page.records.len(), 0);
    assert_eq!(page.next_cursor, None);
    assert!(!page.has_more);
    assert_eq!(client.get_escrow_count(), 0);
}

// ==================== BASIC LISTING ====================

#[test]
fn test_search_lists_all_escrows() {
    let env = Env::default();
    let (client, _cid, _admin, depositor, _contributor, _tc) = setup_search(&env, 100_000);

    let deadline = env.ledger().timestamp() + 1000;
    for id in 1..=5u64 {
        client.lock_funds(&depositor, &id, &1_000, &deadline);
    }

    let criteria = EscrowSearchCriteria {
        status_filter: 0,
        depositor: None,
    };
    let page = client.get_escrows(&criteria, &None, &10);
    assert_eq!(page.records.len(), 5);
    assert!(!page.has_more);
    assert_eq!(page.next_cursor, None);
    assert_eq!(client.get_escrow_count(), 5);
}

// ==================== PAGINATION ====================

#[test]
fn test_search_pagination_basic() {
    let env = Env::default();
    let (client, _cid, _admin, depositor, _contributor, _tc) = setup_search(&env, 100_000);

    let deadline = env.ledger().timestamp() + 1000;
    for id in 1..=5u64 {
        client.lock_funds(&depositor, &id, &1_000, &deadline);
    }

    let criteria = EscrowSearchCriteria {
        status_filter: 0,
        depositor: None,
    };

    // First page: limit 2
    let page1 = client.get_escrows(&criteria, &None, &2);
    assert_eq!(page1.records.len(), 2);
    assert!(page1.has_more);
    assert!(page1.next_cursor.is_some());
    assert_eq!(page1.records.get(0).unwrap().bounty_id, 1);
    assert_eq!(page1.records.get(1).unwrap().bounty_id, 2);

    // Second page: start after cursor
    let page2 = client.get_escrows(&criteria, &page1.next_cursor, &2);
    assert_eq!(page2.records.len(), 2);
    assert!(page2.has_more);
    assert_eq!(page2.records.get(0).unwrap().bounty_id, 3);
    assert_eq!(page2.records.get(1).unwrap().bounty_id, 4);

    // Third page: last result
    let page3 = client.get_escrows(&criteria, &page2.next_cursor, &2);
    assert_eq!(page3.records.len(), 1);
    assert!(!page3.has_more);
    assert_eq!(page3.next_cursor, None);
    assert_eq!(page3.records.get(0).unwrap().bounty_id, 5);
}

// ==================== FILTER BY STATUS ====================

#[test]
fn test_search_filter_by_status() {
    let env = Env::default();
    let (client, _cid, _admin, depositor, contributor, _tc) = setup_search(&env, 100_000);

    let deadline = env.ledger().timestamp() + 1000;
    // Create 3 escrows: release #2, leave #1 and #3 locked
    client.lock_funds(&depositor, &1, &1_000, &deadline);
    client.lock_funds(&depositor, &2, &1_000, &deadline);
    client.lock_funds(&depositor, &3, &1_000, &deadline);
    client.release_funds(&2, &contributor);

    // Search for Locked only (status_filter = 1)
    let locked_criteria = EscrowSearchCriteria {
        status_filter: 1,
        depositor: None,
    };
    let page = client.get_escrows(&locked_criteria, &None, &10);
    assert_eq!(page.records.len(), 2);
    assert_eq!(page.records.get(0).unwrap().bounty_id, 1);
    assert_eq!(page.records.get(1).unwrap().bounty_id, 3);

    // Search for Released only (status_filter = 2)
    let released_criteria = EscrowSearchCriteria {
        status_filter: 2,
        depositor: None,
    };
    let page = client.get_escrows(&released_criteria, &None, &10);
    assert_eq!(page.records.len(), 1);
    assert_eq!(page.records.get(0).unwrap().bounty_id, 2);
}

// ==================== FILTER BY DEPOSITOR ====================

#[test]
fn test_search_filter_by_depositor() {
    let env = Env::default();
    let (client, _cid, _admin, depositor, _contributor, _tc) = setup_search(&env, 100_000);

    let depositor2 = Address::generate(&env);

    let deadline = env.ledger().timestamp() + 1000;
    // All escrows by original depositor
    client.lock_funds(&depositor, &1, &1_000, &deadline);
    client.lock_funds(&depositor, &2, &1_000, &deadline);
    client.lock_funds(&depositor, &3, &1_000, &deadline);

    // Filter by depositor
    let criteria = EscrowSearchCriteria {
        status_filter: 0,
        depositor: Some(depositor.clone()),
    };
    let page = client.get_escrows(&criteria, &None, &10);
    assert_eq!(page.records.len(), 3);

    // Filter by a non-existent depositor returns empty
    let criteria_other = EscrowSearchCriteria {
        status_filter: 0,
        depositor: Some(depositor2.clone()),
    };
    let page_other = client.get_escrows(&criteria_other, &None, &10);
    assert_eq!(page_other.records.len(), 0);
}

// ==================== PAGE SIZE CAP ====================

#[test]
fn test_search_page_size_cap() {
    let env = Env::default();
    let (client, _cid, _admin, depositor, _contributor, _tc) = setup_search(&env, 1_000_000);

    let deadline = env.ledger().timestamp() + 1000;
    // Create 25 escrows (more than MAX_PAGE_SIZE=20)
    for id in 1..=25u64 {
        client.lock_funds(&depositor, &id, &100, &deadline);
    }

    let criteria = EscrowSearchCriteria {
        status_filter: 0,
        depositor: None,
    };

    // Request 100 (exceeds cap), should return 20
    let page = client.get_escrows(&criteria, &None, &100);
    assert_eq!(page.records.len(), 20);
    assert!(page.has_more);
    assert!(page.next_cursor.is_some());
}

// ==================== COMBINED CRITERIA WITH PAGINATION ====================

#[test]
fn test_search_combined_criteria_pagination() {
    let env = Env::default();
    let (client, _cid, _admin, depositor, contributor, _tc) = setup_search(&env, 100_000);

    let deadline = env.ledger().timestamp() + 1000;
    // Create 6 escrows, release even ones
    for id in 1..=6u64 {
        client.lock_funds(&depositor, &id, &1_000, &deadline);
    }
    client.release_funds(&2, &contributor);
    client.release_funds(&4, &contributor);
    client.release_funds(&6, &contributor);

    // Search for Locked (1), by depositor, limit 2
    let criteria = EscrowSearchCriteria {
        status_filter: 1,
        depositor: Some(depositor.clone()),
    };

    let page1 = client.get_escrows(&criteria, &None, &2);
    assert_eq!(page1.records.len(), 2);
    assert!(page1.has_more);
    assert_eq!(page1.records.get(0).unwrap().bounty_id, 1);
    assert_eq!(page1.records.get(1).unwrap().bounty_id, 3);

    let page2 = client.get_escrows(&criteria, &page1.next_cursor, &2);
    assert_eq!(page2.records.len(), 1);
    assert!(!page2.has_more);
    assert_eq!(page2.records.get(0).unwrap().bounty_id, 5);
}
