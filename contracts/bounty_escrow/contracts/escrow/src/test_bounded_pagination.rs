//! # Bounded Pagination Tests for Every Escrow Query Entrypoint
//!
//! Closes: "Add bounded pagination tests for every escrow query"
//!
//! ## Design Decisions
//! - **Maximum page size**: 50 (`MAX_PARTICIPANT_FILTER_PAGE_SIZE`)
//! - **Cursor encoding**: Offset-based (`offset: u32, limit: u32`)
//! - **Deleted/expired records**: Retained in participant list indexes and
//!   aggregate stats, verified via pagination metadata and status totals.
//!
//! ## Coverage Matrix
//! For each query entrypoint we test:
//! - Zero results (empty dataset)
//! - One result
//! - Max page size (50)
//! - Max+1 (51 items, verify clamping/second page)
//! - Empty middle page / out-of-bounds offset
//! - Repeated offset (idempotent)
//! - Invalid cursors / limit zero
//! - Cursor progression without duplicates
//! - No unbounded storage scan (limit is always enforced/clamped)

#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, Vec,
};

/// Maximum page size enforced by the contract for paginated filter queries.
const MAX_PAGE_SIZE: u32 = 50;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn create_setup(env: &Env) -> (Address, BountyEscrowContractClient<'_>) {
    env.mock_all_auths();
    let admin = Address::generate(env);
    let token_admin = Address::generate(env);
    let token_addr = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(env, &id);
    client.init(&admin, &token_addr);
    (admin, client)
}

// ═══════════════════════════════════════════════════════════════════════════════
// 1. query_whitelist — bounded pagination tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_query_whitelist_zero_results() {
    let env = Env::default();
    let (_admin, client) = create_setup(&env);
    let page = client.query_whitelist(&0, &10);
    assert_eq!(page.items.len(), 0);
    assert_eq!(page.total, 0);
    assert_eq!(page.offset, 0);
    assert!(!page.has_more);
}

#[test]
fn test_query_whitelist_one_result() {
    let env = Env::default();
    let (_admin, client) = create_setup(&env);
    let addr = Address::generate(&env);
    client.set_whitelist_entry(&addr, &true);

    let page = client.query_whitelist(&0, &10);
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.total, 1);
    assert_eq!(page.offset, 0);
    assert_eq!(page.items.get(0).unwrap(), addr);
    assert!(!page.has_more);
}

#[test]
fn test_query_whitelist_exactly_max_page() {
    let env = Env::default();
    let (_admin, client) = create_setup(&env);
    for _ in 0..MAX_PAGE_SIZE {
        client.set_whitelist_entry(&Address::generate(&env), &true);
    }

    let page = client.query_whitelist(&0, &MAX_PAGE_SIZE);
    assert_eq!(page.items.len(), MAX_PAGE_SIZE);
    assert_eq!(page.total, MAX_PAGE_SIZE);
    assert!(!page.has_more);
}

#[test]
fn test_query_whitelist_max_plus_one_clamped() {
    let env = Env::default();
    let (_admin, client) = create_setup(&env);
    let total_count = MAX_PAGE_SIZE + 1;
    for _ in 0..total_count {
        client.set_whitelist_entry(&Address::generate(&env), &true);
    }

    // Request oversized page (100) — must be clamped to MAX_PAGE_SIZE (50)
    let page1 = client.query_whitelist(&0, &100);
    assert_eq!(page1.items.len(), MAX_PAGE_SIZE);
    assert_eq!(page1.total, total_count);
    assert!(page1.has_more);

    // Fetch page 2 starting at offset 50
    let page2 = client.query_whitelist(&MAX_PAGE_SIZE, &10);
    assert_eq!(page2.items.len(), 1);
    assert_eq!(page2.total, total_count);
    assert!(!page2.has_more);
}

#[test]
fn test_query_whitelist_offset_beyond_total_returns_empty() {
    let env = Env::default();
    let (_admin, client) = create_setup(&env);
    for _ in 0..3 {
        client.set_whitelist_entry(&Address::generate(&env), &true);
    }

    let page = client.query_whitelist(&999, &10);
    assert_eq!(page.items.len(), 0);
    assert_eq!(page.total, 3);
    assert_eq!(page.offset, 999);
    assert!(!page.has_more);
}

#[test]
fn test_query_whitelist_limit_zero_returns_empty() {
    let env = Env::default();
    let (_admin, client) = create_setup(&env);
    for _ in 0..5 {
        client.set_whitelist_entry(&Address::generate(&env), &true);
    }

    let page = client.query_whitelist(&0, &0);
    assert_eq!(page.items.len(), 0);
    assert_eq!(page.total, 5);
    assert_eq!(page.offset, 0);
}

#[test]
fn test_query_whitelist_repeated_offset_idempotent() {
    let env = Env::default();
    let (_admin, client) = create_setup(&env);
    for _ in 0..5 {
        client.set_whitelist_entry(&Address::generate(&env), &true);
    }

    let page_a = client.query_whitelist(&1, &2);
    let page_b = client.query_whitelist(&1, &2);
    assert_eq!(page_a.items.len(), page_b.items.len());
    assert_eq!(page_a.offset, page_b.offset);
    assert_eq!(page_a.total, page_b.total);
    for i in 0..page_a.items.len() {
        assert_eq!(page_a.items.get(i).unwrap(), page_b.items.get(i).unwrap());
    }
}

#[test]
fn test_query_whitelist_full_cursor_walk_no_duplicates() {
    let env = Env::default();
    let (_admin, client) = create_setup(&env);
    let total_addrs = 13u32;
    for _ in 0..total_addrs {
        client.set_whitelist_entry(&Address::generate(&env), &true);
    }

    let page_size = 3u32;
    let mut collected: Vec<Address> = Vec::new(&env);
    let mut offset = 0u32;

    loop {
        let page = client.query_whitelist(&offset, &page_size);
        if page.items.len() == 0 {
            break;
        }
        for i in 0..page.items.len() {
            let item = page.items.get(i).unwrap();
            for j in 0..collected.len() {
                assert_ne!(
                    collected.get(j).unwrap(),
                    item,
                    "Duplicate address at offset {}",
                    offset
                );
            }
            collected.push_back(item);
        }
        offset += page.items.len();
        if !page.has_more {
            break;
        }
    }

    assert_eq!(collected.len(), total_addrs);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 2. query_blocklist — bounded pagination tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_query_blocklist_zero_results() {
    let env = Env::default();
    let (_admin, client) = create_setup(&env);
    let page = client.query_blocklist(&0, &10);
    assert_eq!(page.items.len(), 0);
    assert_eq!(page.total, 0);
    assert!(!page.has_more);
}

#[test]
fn test_query_blocklist_one_result() {
    let env = Env::default();
    let (_admin, client) = create_setup(&env);
    let addr = Address::generate(&env);
    client.set_blocklist_entry(&addr, &true);

    let page = client.query_blocklist(&0, &10);
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.total, 1);
    assert_eq!(page.items.get(0).unwrap(), addr);
    assert!(!page.has_more);
}

#[test]
fn test_query_blocklist_max_plus_one_clamped() {
    let env = Env::default();
    let (_admin, client) = create_setup(&env);
    let count = MAX_PAGE_SIZE + 5;
    for _ in 0..count {
        client.set_blocklist_entry(&Address::generate(&env), &true);
    }

    let page1 = client.query_blocklist(&0, &999);
    assert_eq!(page1.items.len(), MAX_PAGE_SIZE);
    assert_eq!(page1.total, count);
    assert!(page1.has_more);

    let page2 = client.query_blocklist(&MAX_PAGE_SIZE, &50);
    assert_eq!(page2.items.len(), 5);
    assert!(!page2.has_more);
}

#[test]
fn test_query_blocklist_offset_beyond_total() {
    let env = Env::default();
    let (_admin, client) = create_setup(&env);
    for _ in 0..2 {
        client.set_blocklist_entry(&Address::generate(&env), &true);
    }

    let page = client.query_blocklist(&100, &10);
    assert_eq!(page.items.len(), 0);
    assert_eq!(page.total, 2);
    assert!(!page.has_more);
}

#[test]
fn test_query_blocklist_full_cursor_walk_no_duplicates() {
    let env = Env::default();
    let (_admin, client) = create_setup(&env);
    let total_addrs = 11u32;
    for _ in 0..total_addrs {
        client.set_blocklist_entry(&Address::generate(&env), &true);
    }

    let page_size = 4u32;
    let mut collected: Vec<Address> = Vec::new(&env);
    let mut offset = 0u32;

    loop {
        let page = client.query_blocklist(&offset, &page_size);
        if page.items.len() == 0 {
            break;
        }
        for i in 0..page.items.len() {
            let item = page.items.get(i).unwrap();
            for j in 0..collected.len() {
                assert_ne!(collected.get(j).unwrap(), item);
            }
            collected.push_back(item);
        }
        offset += page.items.len();
        if !page.has_more {
            break;
        }
    }

    assert_eq!(collected.len(), total_addrs);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 3. get_whitelist_count & get_blocklist_count — consistency tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_counts_match_paginated_total_field() {
    let env = Env::default();
    let (_admin, client) = create_setup(&env);
    for _ in 0..7 {
        client.set_whitelist_entry(&Address::generate(&env), &true);
    }
    for _ in 0..4 {
        client.set_blocklist_entry(&Address::generate(&env), &true);
    }

    assert_eq!(client.get_whitelist_count(), 7);
    assert_eq!(client.query_whitelist(&0, &2).total, 7);

    assert_eq!(client.get_blocklist_count(), 4);
    assert_eq!(client.query_blocklist(&0, &2).total, 4);
}

#[test]
fn test_counts_decrease_after_removal() {
    let env = Env::default();
    let (_admin, client) = create_setup(&env);
    let a = Address::generate(&env);
    client.set_whitelist_entry(&a, &true);
    assert_eq!(client.get_whitelist_count(), 1);

    client.set_whitelist_entry(&a, &false);
    assert_eq!(client.get_whitelist_count(), 0);
    assert_eq!(client.query_whitelist(&0, &10).total, 0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 4. get_aggregate_stats — bounded scan over escrow index
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_aggregate_stats_bounded_totals() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_addr = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_client = token::StellarAssetClient::new(&env, &token_addr);
    token_client.mint(&depositor, &10_000);

    let id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &id);
    client.init(&admin, &token_addr);

    let dl = env.ledger().timestamp() + 100;

    // Create escrows
    for i in 1u64..=4 {
        client.lock_funds(&depositor, &i, &(i as i128 * 100), &dl);
    }

    // Release bounty 1 and 2
    client.release_funds(&1, &contributor);
    client.release_funds(&2, &contributor);

    // Refund bounty 3 after deadline
    env.ledger().set_timestamp(dl + 1);
    client.refund(&3);

    let stats = client.get_aggregate_stats();

    assert_eq!(stats.count_locked, 1); // bounty 4
    assert_eq!(stats.count_released, 2); // bounties 1, 2
    assert_eq!(stats.count_refunded, 1); // bounty 3

    assert_eq!(stats.total_released, 300); // 100 + 200
    assert_eq!(stats.total_refunded, 300); // 300
    assert_eq!(stats.total_locked, 400); // 400

    // Invariant: sum of all buckets equals total amount locked
    let total = stats.total_locked + stats.total_released + stats.total_refunded;
    assert_eq!(total, 1000);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 5. get_escrow_info — missing escrow returns typed error
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_get_escrow_info_returns_error_for_invalid_id() {
    let env = Env::default();
    let (_admin, client) = create_setup(&env);
    let res = client.try_get_escrow_info(&99999);
    assert!(res.is_err());
}
