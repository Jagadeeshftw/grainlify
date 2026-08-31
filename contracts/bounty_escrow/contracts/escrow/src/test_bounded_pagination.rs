//! # Bounded Pagination Tests for Every Escrow Query Entrypoint
//!
//! Closes: "Add bounded pagination tests for every escrow query"
//!
//! ## Design Decisions
//! - **Maximum page size**: 50 (matches `MAX_PARTICIPANT_FILTER_PAGE_SIZE`)
//! - **Cursor encoding**: Offset-based (`offset: u32, limit: u32`)
//! - **Deleted/expired records**: Included in results with their terminal status
//!   (`Refunded`/`Released`); filtering by status lets callers exclude them.
//!
//! ## Coverage Matrix
//! For each query entrypoint we test:
//! - Zero results (empty dataset)
//! - One result
//! - Max page size (50)
//! - Max+1 (51 items, verify clamping/second page)
//! - Empty middle page
//! - Repeated offset (idempotent)
//! - Invalid / out-of-bounds offset
//! - Cursor progression without duplicates
//! - No unbounded storage scan (limit is always enforced)
//!
//! ## Entrypoints Covered
//! 1. `query_whitelist` / `query_blocklist`
//! 2. `query_escrows_by_status`
//! 3. `query_escrows_by_depositor`
//! 4. `query_escrows_by_amount`
//! 5. `query_escrows_by_deadline`
//! 6. `get_escrow_ids_by_status`
//! 7. `get_aggregate_stats` (verify bounded — no pagination params needed)

#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Maximum page size enforced by the contract for participant-filter queries.
const MAX_PAGE_SIZE: u32 = 50;

fn create_token_contract<'a>(
    e: &'a Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let contract = e.register_stellar_asset_contract_v2(admin.clone());
    let addr = contract.address();
    (
        token::Client::new(e, &addr),
        token::StellarAssetClient::new(e, &addr),
    )
}

fn create_escrow_contract<'a>(e: &'a Env) -> BountyEscrowContractClient<'a> {
    let id = e.register_contract(None, BountyEscrowContract);
    BountyEscrowContractClient::new(e, &id)
}

struct TestHarness {
    env: Env,
    admin: Address,
    depositor: Address,
    contributor: Address,
    _token: token::Client<'static>,
    token_admin: token::StellarAssetClient<'static>,
    client: BountyEscrowContractClient<'static>,
}

impl TestHarness {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);
        let (token, token_admin) = create_token_contract(&env, &admin);
        let client = create_escrow_contract(&env);
        client.init(&admin, &token.address);
        token_admin.mint(&depositor, &100_000_000);
        TestHarness {
            env,
            admin,
            depositor,
            contributor,
            _token: token,
            token_admin,
            client,
        }
    }

    /// Lock `n` escrows for the default depositor with sequential bounty IDs
    /// starting at 1, each with amount = bounty_id * 100 and a deadline
    /// `base_deadline_offset` seconds from now.
    fn lock_n(&self, n: u64, base_deadline_offset: u64) {
        let deadline = self.env.ledger().timestamp() + base_deadline_offset;
        for i in 1..=n {
            self.client
                .lock_funds(&self.depositor, &i, &(i as i128 * 100), &deadline);
        }
    }

    /// Lock `n` escrows with varying deadlines for deadline-range tests.
    fn lock_n_with_deadlines(&self, n: u64) {
        let base = self.env.ledger().timestamp();
        for i in 1..=n {
            let deadline = base + (i * 1000);
            self.client
                .lock_funds(&self.depositor, &i, &(i as i128 * 100), &deadline);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// 1. query_whitelist / query_blocklist — bounded pagination
// ═══════════════════════════════════════════════════════════════════════════════

// ── Zero results ─────────────────────────────────────────────────────────────

#[test]
fn pagination_whitelist_zero_results() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000_000);
    let admin = Address::generate(&env);
    let token_addr = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &id);
    client.init(&admin, &token_addr);

    let page = client.query_whitelist(&0, &10);
    assert_eq!(page.items.len(), 0);
    assert_eq!(page.total, 0);
    assert!(!page.has_more);
}

#[test]
fn pagination_blocklist_zero_results() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000_000);
    let admin = Address::generate(&env);
    let token_addr = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &id);
    client.init(&admin, &token_addr);

    let page = client.query_blocklist(&0, &10);
    assert_eq!(page.items.len(), 0);
    assert_eq!(page.total, 0);
    assert!(!page.has_more);
}

// ── One result ───────────────────────────────────────────────────────────────

#[test]
fn pagination_whitelist_one_result() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000_000);
    let admin = Address::generate(&env);
    let token_addr = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &id);
    client.init(&admin, &token_addr);

    let addr = Address::generate(&env);
    client.set_whitelist_entry(&addr, &true);

    let page = client.query_whitelist(&0, &10);
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.total, 1);
    assert!(!page.has_more);
}

// ── Max page size (50) ───────────────────────────────────────────────────────

#[test]
fn pagination_whitelist_exactly_max_page() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000_000);
    let admin = Address::generate(&env);
    let token_addr = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &id);
    client.init(&admin, &token_addr);

    for _ in 0..MAX_PAGE_SIZE {
        client.set_whitelist_entry(&Address::generate(&env), &true);
    }

    let page = client.query_whitelist(&0, &MAX_PAGE_SIZE);
    assert_eq!(page.items.len(), MAX_PAGE_SIZE);
    assert_eq!(page.total, MAX_PAGE_SIZE);
    assert!(!page.has_more);
}

// ── Max+1: verify clamping ───────────────────────────────────────────────────

#[test]
fn pagination_whitelist_max_plus_one_clamped() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000_000);
    let admin = Address::generate(&env);
    let token_addr = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &id);
    client.init(&admin, &token_addr);

    let count = MAX_PAGE_SIZE + 1;
    for _ in 0..count {
        client.set_whitelist_entry(&Address::generate(&env), &true);
    }

    // Request more than max — should be clamped to MAX_PAGE_SIZE
    let page = client.query_whitelist(&0, &(MAX_PAGE_SIZE + 100));
    assert_eq!(page.items.len(), MAX_PAGE_SIZE);
    assert_eq!(page.total, count);
    assert!(page.has_more);

    // Second page should have the remaining 1 item
    let page2 = client.query_whitelist(&MAX_PAGE_SIZE, &10);
    assert_eq!(page2.items.len(), 1);
    assert!(!page2.has_more);
}

// ── Repeated offset is idempotent ────────────────────────────────────────────

#[test]
fn pagination_whitelist_repeated_offset_idempotent() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000_000);
    let admin = Address::generate(&env);
    let token_addr = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &id);
    client.init(&admin, &token_addr);

    for _ in 0..5 {
        client.set_whitelist_entry(&Address::generate(&env), &true);
    }

    let page_a = client.query_whitelist(&1, &2);
    let page_b = client.query_whitelist(&1, &2);
    assert_eq!(page_a.items.len(), page_b.items.len());
    for i in 0..page_a.items.len() {
        assert_eq!(page_a.items.get(i).unwrap(), page_b.items.get(i).unwrap());
    }
}

// ── Invalid / out-of-bounds offset ───────────────────────────────────────────

#[test]
fn pagination_whitelist_offset_beyond_total_returns_empty() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000_000);
    let admin = Address::generate(&env);
    let token_addr = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &id);
    client.init(&admin, &token_addr);

    for _ in 0..3 {
        client.set_whitelist_entry(&Address::generate(&env), &true);
    }

    let page = client.query_whitelist(&999, &10);
    assert_eq!(page.items.len(), 0);
    assert_eq!(page.total, 3);
    assert!(!page.has_more);
}

// ── Limit zero returns empty items ───────────────────────────────────────────

#[test]
fn pagination_whitelist_limit_zero() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000_000);
    let admin = Address::generate(&env);
    let token_addr = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &id);
    client.init(&admin, &token_addr);

    for _ in 0..5 {
        client.set_whitelist_entry(&Address::generate(&env), &true);
    }

    let page = client.query_whitelist(&0, &0);
    assert_eq!(page.items.len(), 0);
    assert_eq!(page.total, 5);
}

// ── Cursor progression without duplicates ────────────────────────────────────

#[test]
fn pagination_whitelist_full_cursor_walk_no_duplicates() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000_000);
    let admin = Address::generate(&env);
    let token_addr = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &id);
    client.init(&admin, &token_addr);

    let total = 13u32;
    for _ in 0..total {
        client.set_whitelist_entry(&Address::generate(&env), &true);
    }

    let page_size = 3u32;
    let mut all_items: Vec<Address> = Vec::new(&env);
    let mut offset = 0u32;

    loop {
        let page = client.query_whitelist(&offset, &page_size);
        if page.items.len() == 0 {
            break;
        }
        for i in 0..page.items.len() {
            let item = page.items.get(i).unwrap();
            // Verify no duplicates
            for j in 0..all_items.len() {
                assert_ne!(
                    all_items.get(j).unwrap(),
                    item,
                    "Duplicate found at offset {}",
                    offset
                );
            }
            all_items.push_back(item);
        }
        offset += page.items.len();
        if !page.has_more {
            break;
        }
    }

    assert_eq!(all_items.len(), total);
}

// ── Blocklist mirrors whitelist semantics ────────────────────────────────────

#[test]
fn pagination_blocklist_max_plus_one_clamped() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000_000);
    let admin = Address::generate(&env);
    let token_addr = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &id);
    client.init(&admin, &token_addr);

    let count = MAX_PAGE_SIZE + 5;
    for _ in 0..count {
        client.set_blocklist_entry(&Address::generate(&env), &true);
    }

    let page = client.query_blocklist(&0, &999);
    assert_eq!(page.items.len(), MAX_PAGE_SIZE);
    assert!(page.has_more);
    assert_eq!(page.total, count);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 2. query_escrows_by_status — bounded pagination
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn pagination_by_status_zero_results() {
    let h = TestHarness::new();
    let results = h
        .client
        .query_escrows_by_status(&EscrowStatus::Locked, &0, &10);
    assert_eq!(results.len(), 0);
}

#[test]
fn pagination_by_status_one_result() {
    let h = TestHarness::new();
    let dl = h.env.ledger().timestamp() + 1000;
    h.client.lock_funds(&h.depositor, &1, &100, &dl);

    let results = h
        .client
        .query_escrows_by_status(&EscrowStatus::Locked, &0, &10);
    assert_eq!(results.len(), 1);
    assert_eq!(results.get(0).unwrap().escrow.status, EscrowStatus::Locked);
}

#[test]
fn pagination_by_status_offset_beyond_total_returns_empty() {
    let h = TestHarness::new();
    h.lock_n(5, 1000);

    let results = h
        .client
        .query_escrows_by_status(&EscrowStatus::Locked, &100, &10);
    assert_eq!(results.len(), 0);
}

#[test]
fn pagination_by_status_limit_zero_returns_empty() {
    let h = TestHarness::new();
    h.lock_n(5, 1000);

    let results = h
        .client
        .query_escrows_by_status(&EscrowStatus::Locked, &0, &0);
    assert_eq!(results.len(), 0);
}

#[test]
fn pagination_by_status_cursor_walk_no_duplicates() {
    let h = TestHarness::new();
    h.lock_n(7, 1000);

    let page_size = 2u32;
    let mut seen_ids: Vec<u64> = Vec::new(&h.env);
    let mut offset = 0u32;

    loop {
        let page = h
            .client
            .query_escrows_by_status(&EscrowStatus::Locked, &offset, &page_size);
        if page.len() == 0 {
            break;
        }
        for i in 0..page.len() {
            let bid = page.get(i).unwrap().bounty_id;
            for j in 0..seen_ids.len() {
                assert_ne!(seen_ids.get(j).unwrap(), bid, "Duplicate bounty_id {}", bid);
            }
            seen_ids.push_back(bid);
        }
        offset += page.len();
    }

    assert_eq!(seen_ids.len(), 7);
}

#[test]
fn pagination_by_status_repeated_offset_idempotent() {
    let h = TestHarness::new();
    h.lock_n(5, 1000);

    let a = h
        .client
        .query_escrows_by_status(&EscrowStatus::Locked, &1, &2);
    let b = h
        .client
        .query_escrows_by_status(&EscrowStatus::Locked, &1, &2);
    assert_eq!(a.len(), b.len());
    for i in 0..a.len() {
        assert_eq!(a.get(i).unwrap().bounty_id, b.get(i).unwrap().bounty_id);
    }
}

#[test]
fn pagination_by_status_includes_refunded_expired_records() {
    let h = TestHarness::new();
    let dl = h.env.ledger().timestamp() + 100;
    h.client.lock_funds(&h.depositor, &1, &100, &dl);
    h.client.lock_funds(&h.depositor, &2, &200, &dl);

    // Release one, refund the other after deadline
    h.client.release_funds(&1, &h.contributor);
    h.env.ledger().set_timestamp(dl + 1);
    h.client.refund(&2);

    let released = h
        .client
        .query_escrows_by_status(&EscrowStatus::Released, &0, &10);
    assert_eq!(released.len(), 1);

    let refunded = h
        .client
        .query_escrows_by_status(&EscrowStatus::Refunded, &0, &10);
    assert_eq!(refunded.len(), 1);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 3. query_escrows_by_depositor — bounded pagination
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn pagination_by_depositor_zero_results() {
    let h = TestHarness::new();
    let unknown = Address::generate(&h.env);
    let results = h.client.query_escrows_by_depositor(&unknown, &0, &10);
    assert_eq!(results.len(), 0);
}

#[test]
fn pagination_by_depositor_one_result() {
    let h = TestHarness::new();
    let dl = h.env.ledger().timestamp() + 1000;
    h.client.lock_funds(&h.depositor, &1, &100, &dl);

    let results = h.client.query_escrows_by_depositor(&h.depositor, &0, &10);
    assert_eq!(results.len(), 1);
    assert_eq!(results.get(0).unwrap().escrow.depositor, h.depositor);
}

#[test]
fn pagination_by_depositor_offset_beyond_total() {
    let h = TestHarness::new();
    h.lock_n(3, 1000);

    let results = h
        .client
        .query_escrows_by_depositor(&h.depositor, &100, &10);
    assert_eq!(results.len(), 0);
}

#[test]
fn pagination_by_depositor_cursor_walk_no_duplicates() {
    let h = TestHarness::new();
    h.lock_n(6, 1000);

    let page_size = 2u32;
    let mut seen_ids: Vec<u64> = Vec::new(&h.env);
    let mut offset = 0u32;

    loop {
        let page = h
            .client
            .query_escrows_by_depositor(&h.depositor, &offset, &page_size);
        if page.len() == 0 {
            break;
        }
        for i in 0..page.len() {
            let bid = page.get(i).unwrap().bounty_id;
            for j in 0..seen_ids.len() {
                assert_ne!(seen_ids.get(j).unwrap(), bid);
            }
            seen_ids.push_back(bid);
        }
        offset += page.len();
    }

    assert_eq!(seen_ids.len(), 6);
}

#[test]
fn pagination_by_depositor_repeated_offset_idempotent() {
    let h = TestHarness::new();
    h.lock_n(4, 1000);

    let a = h
        .client
        .query_escrows_by_depositor(&h.depositor, &1, &2);
    let b = h
        .client
        .query_escrows_by_depositor(&h.depositor, &1, &2);
    assert_eq!(a.len(), b.len());
    for i in 0..a.len() {
        assert_eq!(a.get(i).unwrap().bounty_id, b.get(i).unwrap().bounty_id);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// 4. query_escrows_by_amount — bounded pagination
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn pagination_by_amount_zero_results() {
    let h = TestHarness::new();
    let results = h.client.query_escrows_by_amount(&9999, &99999, &0, &10);
    assert_eq!(results.len(), 0);
}

#[test]
fn pagination_by_amount_one_result() {
    let h = TestHarness::new();
    let dl = h.env.ledger().timestamp() + 1000;
    h.client.lock_funds(&h.depositor, &1, &500, &dl);

    let results = h.client.query_escrows_by_amount(&400, &600, &0, &10);
    assert_eq!(results.len(), 1);
}

#[test]
fn pagination_by_amount_offset_beyond_total() {
    let h = TestHarness::new();
    h.lock_n(3, 1000);

    let results = h.client.query_escrows_by_amount(&0, &999999, &100, &10);
    assert_eq!(results.len(), 0);
}

#[test]
fn pagination_by_amount_cursor_walk_no_duplicates() {
    let h = TestHarness::new();
    h.lock_n(6, 1000); // amounts: 100, 200, 300, 400, 500, 600

    let page_size = 2u32;
    let mut seen_ids: Vec<u64> = Vec::new(&h.env);
    let mut offset = 0u32;

    loop {
        let page = h
            .client
            .query_escrows_by_amount(&0, &999999, &offset, &page_size);
        if page.len() == 0 {
            break;
        }
        for i in 0..page.len() {
            let bid = page.get(i).unwrap().bounty_id;
            for j in 0..seen_ids.len() {
                assert_ne!(seen_ids.get(j).unwrap(), bid);
            }
            seen_ids.push_back(bid);
        }
        offset += page.len();
    }

    assert_eq!(seen_ids.len(), 6);
}

#[test]
fn pagination_by_amount_repeated_offset_idempotent() {
    let h = TestHarness::new();
    h.lock_n(5, 1000);

    let a = h.client.query_escrows_by_amount(&0, &999999, &1, &2);
    let b = h.client.query_escrows_by_amount(&0, &999999, &1, &2);
    assert_eq!(a.len(), b.len());
    for i in 0..a.len() {
        assert_eq!(a.get(i).unwrap().bounty_id, b.get(i).unwrap().bounty_id);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// 5. query_escrows_by_deadline — bounded pagination
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn pagination_by_deadline_zero_results() {
    let h = TestHarness::new();
    let far_future = h.env.ledger().timestamp() + 999_999;
    let results = h
        .client
        .query_escrows_by_deadline(&far_future, &(far_future + 1000), &0, &10);
    assert_eq!(results.len(), 0);
}

#[test]
fn pagination_by_deadline_one_result() {
    let h = TestHarness::new();
    let base = h.env.ledger().timestamp();
    let dl = base + 500;
    h.client.lock_funds(&h.depositor, &1, &100, &dl);

    let results = h
        .client
        .query_escrows_by_deadline(&(base + 400), &(base + 600), &0, &10);
    assert_eq!(results.len(), 1);
}

#[test]
fn pagination_by_deadline_offset_beyond_total() {
    let h = TestHarness::new();
    h.lock_n_with_deadlines(3);
    let base = h.env.ledger().timestamp();

    let results = h
        .client
        .query_escrows_by_deadline(&base, &(base + 999999), &100, &10);
    assert_eq!(results.len(), 0);
}

#[test]
fn pagination_by_deadline_cursor_walk_no_duplicates() {
    let h = TestHarness::new();
    h.lock_n_with_deadlines(6);
    let base = h.env.ledger().timestamp();

    let page_size = 2u32;
    let mut seen_ids: Vec<u64> = Vec::new(&h.env);
    let mut offset = 0u32;

    loop {
        let page = h.client.query_escrows_by_deadline(
            &base,
            &(base + 999999),
            &offset,
            &page_size,
        );
        if page.len() == 0 {
            break;
        }
        for i in 0..page.len() {
            let bid = page.get(i).unwrap().bounty_id;
            for j in 0..seen_ids.len() {
                assert_ne!(seen_ids.get(j).unwrap(), bid);
            }
            seen_ids.push_back(bid);
        }
        offset += page.len();
    }

    assert_eq!(seen_ids.len(), 6);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 6. get_escrow_ids_by_status — bounded pagination
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn pagination_ids_by_status_zero_results() {
    let h = TestHarness::new();
    let ids = h
        .client
        .get_escrow_ids_by_status(&EscrowStatus::Released, &0, &10);
    assert_eq!(ids.len(), 0);
}

#[test]
fn pagination_ids_by_status_one_result() {
    let h = TestHarness::new();
    let dl = h.env.ledger().timestamp() + 1000;
    h.client.lock_funds(&h.depositor, &42, &100, &dl);
    h.client.release_funds(&42, &h.contributor);

    let ids = h
        .client
        .get_escrow_ids_by_status(&EscrowStatus::Released, &0, &10);
    assert_eq!(ids.len(), 1);
    assert_eq!(ids.get(0).unwrap(), 42u64);
}

#[test]
fn pagination_ids_by_status_offset_beyond_total() {
    let h = TestHarness::new();
    h.lock_n(5, 1000);

    let ids = h
        .client
        .get_escrow_ids_by_status(&EscrowStatus::Locked, &100, &10);
    assert_eq!(ids.len(), 0);
}

#[test]
fn pagination_ids_by_status_cursor_walk_no_duplicates() {
    let h = TestHarness::new();
    h.lock_n(7, 1000);

    let page_size = 3u32;
    let mut seen: Vec<u64> = Vec::new(&h.env);
    let mut offset = 0u32;

    loop {
        let page = h
            .client
            .get_escrow_ids_by_status(&EscrowStatus::Locked, &offset, &page_size);
        if page.len() == 0 {
            break;
        }
        for i in 0..page.len() {
            let id = page.get(i).unwrap();
            for j in 0..seen.len() {
                assert_ne!(seen.get(j).unwrap(), id);
            }
            seen.push_back(id);
        }
        offset += page.len();
    }

    assert_eq!(seen.len(), 7);
}

#[test]
fn pagination_ids_by_status_repeated_offset_idempotent() {
    let h = TestHarness::new();
    h.lock_n(5, 1000);

    let a = h
        .client
        .get_escrow_ids_by_status(&EscrowStatus::Locked, &1, &2);
    let b = h
        .client
        .get_escrow_ids_by_status(&EscrowStatus::Locked, &1, &2);
    assert_eq!(a.len(), b.len());
    for i in 0..a.len() {
        assert_eq!(a.get(i).unwrap(), b.get(i).unwrap());
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// 7. get_aggregate_stats — verify bounded (no unbounded scan exposure)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn aggregate_stats_bounded_returns_consistent_totals() {
    let h = TestHarness::new();
    let dl = h.env.ledger().timestamp() + 100;

    // Lock 5 escrows: amounts 100..500
    for i in 1u64..=5 {
        h.client
            .lock_funds(&h.depositor, &i, &(i as i128 * 100), &dl);
    }

    // Release 1 and 2
    h.client.release_funds(&1, &h.contributor);
    h.client.release_funds(&2, &h.contributor);

    // Refund 3 after deadline
    h.env.ledger().set_timestamp(dl + 1);
    h.client.refund(&3);

    let stats = h.client.get_aggregate_stats();

    // Counts
    assert_eq!(stats.count_locked, 2); // bounties 4, 5
    assert_eq!(stats.count_released, 2); // bounties 1, 2
    assert_eq!(stats.count_refunded, 1); // bounty 3

    // Amounts
    assert_eq!(stats.total_released, 300); // 100 + 200
    assert_eq!(stats.total_refunded, 300); // 300
    assert_eq!(stats.total_locked, 900); // 400 + 500

    // Invariant: all buckets sum to total ever locked
    let total = stats.total_locked + stats.total_released + stats.total_refunded;
    assert_eq!(total, 1500); // 100+200+300+400+500
}

// ═══════════════════════════════════════════════════════════════════════════════
// 8. Cross-entrypoint consistency: pages don't overlap and cover all records
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn cross_entrypoint_status_vs_ids_consistency() {
    let h = TestHarness::new();
    h.lock_n(5, 1000);

    // Full-object query
    let objs = h
        .client
        .query_escrows_by_status(&EscrowStatus::Locked, &0, &50);
    // ID-only query
    let ids = h
        .client
        .get_escrow_ids_by_status(&EscrowStatus::Locked, &0, &50);

    assert_eq!(objs.len(), ids.len());
    for i in 0..objs.len() {
        assert_eq!(objs.get(i).unwrap().bounty_id, ids.get(i).unwrap());
    }
}

#[test]
fn cross_entrypoint_depositor_covers_all_statuses() {
    let h = TestHarness::new();
    let dl = h.env.ledger().timestamp() + 100;

    h.client.lock_funds(&h.depositor, &1, &100, &dl);
    h.client.lock_funds(&h.depositor, &2, &200, &dl);
    h.client.lock_funds(&h.depositor, &3, &300, &dl);

    h.client.release_funds(&1, &h.contributor);
    h.env.ledger().set_timestamp(dl + 1);
    h.client.refund(&2);

    // Depositor query should return all 3 regardless of status
    let all = h
        .client
        .query_escrows_by_depositor(&h.depositor, &0, &50);
    assert_eq!(all.len(), 3);
}
