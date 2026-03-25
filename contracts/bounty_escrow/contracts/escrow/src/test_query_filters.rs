use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

//  local helpers

fn create_token(
    env: &Env,
    admin: &Address,
) -> (token::Client<'static>, token::StellarAssetClient<'static>) {
    let addr = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    (
        token::Client::new(env, &addr),
        token::StellarAssetClient::new(env, &addr),
    )
}

fn create_escrow(env: &Env) -> BountyEscrowContractClient<'static> {
    let id = env.register_contract(None, BountyEscrowContract);
    BountyEscrowContractClient::new(env, &id)
}

struct Setup {
    env: Env,
    depositor: Address,
    contributor: Address,
    _token: token::Client<'static>, // Added underscore to silence 'never read' warning
    token_admin: token::StellarAssetClient<'static>,
    escrow: BountyEscrowContractClient<'static>,
}

impl Setup {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);
        let (token, token_admin) = create_token(&env, &admin);
        let escrow = create_escrow(&env);
        escrow.init(&admin, &token.address);
        token_admin.mint(&depositor, &10_000_000);
        Setup {
            env,
            depositor,
            contributor,
            _token: token, // Updated field name
            token_admin,
            escrow,
        }
    }
}
//  status filter tests

#[test]
fn test_query_by_status_locked_returns_only_locked() {
    let s = Setup::new();
    let dl = s.env.ledger().timestamp() + 1000;

    s.escrow.lock_funds(&s.depositor, &1, &100, &dl);
    s.escrow.lock_funds(&s.depositor, &2, &200, &dl);
    s.escrow.lock_funds(&s.depositor, &3, &300, &dl);
    s.escrow.release_funds(&2, &s.contributor);

    let results = s
        .escrow
        .query_escrows_by_status(&EscrowStatus::Locked, &0, &10);
    assert_eq!(results.len(), 2);
    for i in 0..results.len() {
        assert_eq!(results.get(i).unwrap().escrow.status, EscrowStatus::Locked);
    }
}

#[test]
fn test_query_by_status_released_returns_only_released() {
    let s = Setup::new();
    let dl = s.env.ledger().timestamp() + 1000;

    s.escrow.lock_funds(&s.depositor, &1, &100, &dl);
    s.escrow.lock_funds(&s.depositor, &2, &200, &dl);
    s.escrow.lock_funds(&s.depositor, &3, &300, &dl);
    s.escrow.release_funds(&1, &s.contributor);
    s.escrow.release_funds(&3, &s.contributor);

    let results = s
        .escrow
        .query_escrows_by_status(&EscrowStatus::Released, &0, &10);
    assert_eq!(results.len(), 2);
    for i in 0..results.len() {
        assert_eq!(
            results.get(i).unwrap().escrow.status,
            EscrowStatus::Released
        );
    }
}

#[test]
fn test_query_by_status_refunded_returns_only_refunded() {
    let s = Setup::new();
    let now = s.env.ledger().timestamp();
    let dl = now + 100;

    s.escrow.lock_funds(&s.depositor, &1, &100, &dl);
    s.escrow.lock_funds(&s.depositor, &2, &200, &dl);
    s.escrow.lock_funds(&s.depositor, &3, &300, &dl);
    s.escrow.release_funds(&1, &s.contributor);
    s.env.ledger().set_timestamp(dl + 1);
    s.escrow.refund(&2);
    s.escrow.refund(&3);

    let results = s
        .escrow
        .query_escrows_by_status(&EscrowStatus::Refunded, &0, &10);
    assert_eq!(results.len(), 2);
    for i in 0..results.len() {
        assert_eq!(
            results.get(i).unwrap().escrow.status,
            EscrowStatus::Refunded
        );
    }
}

#[test]
fn test_query_by_status_empty_when_no_match() {
    let s = Setup::new();
    let dl = s.env.ledger().timestamp() + 1000;

    s.escrow.lock_funds(&s.depositor, &1, &100, &dl);
    s.escrow.lock_funds(&s.depositor, &2, &200, &dl);

    let results = s
        .escrow
        .query_escrows_by_status(&EscrowStatus::Refunded, &0, &10);
    assert_eq!(results.len(), 0);
}

#[test]
fn test_query_by_status_pagination_offset_and_limit() {
    let s = Setup::new();
    let dl = s.env.ledger().timestamp() + 1000;

    for i in 1u64..=5 {
        s.escrow
            .lock_funds(&s.depositor, &i, &(i as i128 * 100), &dl);
    }

    let page1 = s
        .escrow
        .query_escrows_by_status(&EscrowStatus::Locked, &0, &2);
    assert_eq!(page1.len(), 2);

    let page2 = s
        .escrow
        .query_escrows_by_status(&EscrowStatus::Locked, &2, &2);
    assert_eq!(page2.len(), 2);

    let page3 = s
        .escrow
        .query_escrows_by_status(&EscrowStatus::Locked, &4, &2);
    assert_eq!(page3.len(), 1);

    // No overlap between pages
    assert_ne!(
        page1.get(0).unwrap().bounty_id,
        page2.get(0).unwrap().bounty_id
    );
    assert_ne!(
        page2.get(0).unwrap().bounty_id,
        page3.get(0).unwrap().bounty_id
    );
}

// amount filter tests
#[test]
fn test_query_by_amount_range_returns_matching_escrows() {
    let s = Setup::new();
    let dl = s.env.ledger().timestamp() + 1000;

    s.escrow.lock_funds(&s.depositor, &1, &100, &dl);
    s.escrow.lock_funds(&s.depositor, &2, &500, &dl);
    s.escrow.lock_funds(&s.depositor, &3, &1000, &dl);
    s.escrow.lock_funds(&s.depositor, &4, &5000, &dl);

    let results = s.escrow.query_escrows_by_amount(&400, &1100, &0, &10);
    assert_eq!(results.len(), 2);
    for i in 0..results.len() {
        let amt = results.get(i).unwrap().escrow.amount;
        assert!((400..=1100).contains(&amt));
    }
}

#[test]
fn test_query_by_amount_exact_boundaries_included() {
    let s = Setup::new();
    let dl = s.env.ledger().timestamp() + 1000;

    s.escrow.lock_funds(&s.depositor, &1, &100, &dl);
    s.escrow.lock_funds(&s.depositor, &2, &200, &dl);
    s.escrow.lock_funds(&s.depositor, &3, &300, &dl);

    let results = s.escrow.query_escrows_by_amount(&100, &300, &0, &10);
    assert_eq!(results.len(), 3);
}

#[test]
fn test_query_by_amount_no_results_outside_range() {
    let s = Setup::new();
    let dl = s.env.ledger().timestamp() + 1000;

    s.escrow.lock_funds(&s.depositor, &1, &100, &dl);
    s.escrow.lock_funds(&s.depositor, &2, &200, &dl);

    let results = s.escrow.query_escrows_by_amount(&5000, &9999, &0, &10);
    assert_eq!(results.len(), 0);
}

// deadline filter tests

#[test]
fn test_query_by_deadline_range_filters_correctly() {
    let s = Setup::new();
    let base = s.env.ledger().timestamp();

    s.escrow.lock_funds(&s.depositor, &1, &100, &(base + 100));
    s.escrow.lock_funds(&s.depositor, &2, &200, &(base + 500));
    s.escrow.lock_funds(&s.depositor, &3, &300, &(base + 1000));
    s.escrow.lock_funds(&s.depositor, &4, &400, &(base + 9999));

    let results = s
        .escrow
        .query_escrows_by_deadline(&(base + 400), &(base + 1500), &0, &10);
    assert_eq!(results.len(), 2);
    for i in 0..results.len() {
        let dl = results.get(i).unwrap().escrow.deadline;
        assert!(dl >= base + 400 && dl <= base + 1500);
    }
}

#[test]
fn test_query_by_deadline_exact_boundary_included() {
    let s = Setup::new();
    let base = s.env.ledger().timestamp();

    s.escrow.lock_funds(&s.depositor, &1, &100, &(base + 200));
    s.escrow.lock_funds(&s.depositor, &2, &200, &(base + 500));
    s.escrow.lock_funds(&s.depositor, &3, &300, &(base + 800));

    let results = s
        .escrow
        .query_escrows_by_deadline(&(base + 200), &(base + 500), &0, &10);
    assert_eq!(results.len(), 2);
}

// depositor filter tests

#[test]
fn test_query_by_depositor_returns_only_that_depositors_escrows() {
    let s = Setup::new();
    let dl = s.env.ledger().timestamp() + 1000;
    let depositor2 = Address::generate(&s.env);
    s.token_admin.mint(&depositor2, &10_000);

    s.escrow.lock_funds(&s.depositor, &1, &100, &dl);
    s.escrow.lock_funds(&s.depositor, &2, &200, &dl);
    s.escrow.lock_funds(&depositor2, &3, &300, &dl);

    let r1 = s.escrow.query_escrows_by_depositor(&s.depositor, &0, &10);
    assert_eq!(r1.len(), 2);
    for i in 0..r1.len() {
        assert_eq!(r1.get(i).unwrap().escrow.depositor, s.depositor);
    }

    let r2 = s.escrow.query_escrows_by_depositor(&depositor2, &0, &10);
    assert_eq!(r2.len(), 1);
    assert_eq!(r2.get(0).unwrap().escrow.depositor, depositor2);
}

#[test]
fn test_query_by_depositor_returns_empty_for_unknown_address() {
    let s = Setup::new();
    let unknown = Address::generate(&s.env);
    let results = s.escrow.query_escrows_by_depositor(&unknown, &0, &10);
    assert_eq!(results.len(), 0);
}

// get_escrow_ids_by_status tests

#[test]
fn test_get_escrow_ids_by_status_returns_correct_ids() {
    let s = Setup::new();
    let dl = s.env.ledger().timestamp() + 1000;

    s.escrow.lock_funds(&s.depositor, &10, &100, &dl);
    s.escrow.lock_funds(&s.depositor, &20, &200, &dl);
    s.escrow.lock_funds(&s.depositor, &30, &300, &dl);
    s.escrow.release_funds(&20, &s.contributor);

    let locked_ids = s
        .escrow
        .get_escrow_ids_by_status(&EscrowStatus::Locked, &0, &10);
    assert_eq!(locked_ids.len(), 2);
    for i in 0..locked_ids.len() {
        assert_ne!(locked_ids.get(i).unwrap(), 20u64);
    }

    let released_ids = s
        .escrow
        .get_escrow_ids_by_status(&EscrowStatus::Released, &0, &10);
    assert_eq!(released_ids.len(), 1);
    assert_eq!(released_ids.get(0).unwrap(), 20u64);
}

#[test]
fn test_get_escrow_ids_by_status_empty_when_no_match() {
    let s = Setup::new();
    let dl = s.env.ledger().timestamp() + 1000;
    s.escrow.lock_funds(&s.depositor, &1, &100, &dl);

    let ids = s
        .escrow
        .get_escrow_ids_by_status(&EscrowStatus::Released, &0, &10);
    assert_eq!(ids.len(), 0);
}

// combined filter test (manual composition)

#[test]
fn test_combined_status_and_amount_filter_via_manual_compose() {
    let s = Setup::new();
    let dl = s.env.ledger().timestamp() + 1000;

    s.escrow.lock_funds(&s.depositor, &1, &50, &dl);
    s.escrow.lock_funds(&s.depositor, &2, &500, &dl);
    s.escrow.lock_funds(&s.depositor, &3, &5000, &dl);
    s.escrow.release_funds(&2, &s.contributor);

    // Step 1: filter by status=Locked
    let locked = s
        .escrow
        .query_escrows_by_status(&EscrowStatus::Locked, &0, &10);

    // Step 2: among locked, find those with amount >= 1000
    let mut large_count = 0u32;
    let mut large_id = 0u64;
    for i in 0..locked.len() {
        let item = locked.get(i).unwrap();
        if item.escrow.amount >= 1000 {
            large_count += 1;
            large_id = item.bounty_id;
        }
    }
    assert_eq!(large_count, 1);
    assert_eq!(large_id, 3u64);
}

// aggregate stats test

#[test]
fn test_aggregate_stats_reflects_correct_counts_after_lifecycle() {
    let s = Setup::new();
    let now = s.env.ledger().timestamp();
    let dl = now + 100;

    s.escrow.lock_funds(&s.depositor, &1, &100, &dl);
    s.escrow.lock_funds(&s.depositor, &2, &200, &dl);
    s.escrow.lock_funds(&s.depositor, &3, &300, &dl);
    s.escrow.lock_funds(&s.depositor, &4, &400, &dl);

    s.escrow.release_funds(&1, &s.contributor);
    s.escrow.release_funds(&2, &s.contributor);

    s.env.ledger().set_timestamp(dl + 1);
    s.escrow.refund(&3);

    let stats = s.escrow.get_aggregate_stats();
    assert_eq!(stats.count_locked, 1);
    assert_eq!(stats.count_released, 2);
    assert_eq!(stats.count_refunded, 1);
    assert_eq!(stats.total_released, 300); // bounties 1+2
    assert_eq!(stats.total_refunded, 300); // bounty 3
    assert_eq!(stats.total_locked, 400); // bounty 4
}

/// Depositor query: pagination with offset > 0 correctly skips earlier records.
#[test]
fn test_query_by_depositor_pagination_offset_skips_correctly() {
    let s = Setup::new();
    let dl = s.env.ledger().timestamp() + 1000;

    // Lock 4 bounties for the same depositor
    for i in 1u64..=4 {
        s.escrow
            .lock_funds(&s.depositor, &i, &(i as i128 * 100), &dl);
    }

    // Page 1: first 2
    let page1 = s.escrow.query_escrows_by_depositor(&s.depositor, &0, &2);
    assert_eq!(page1.len(), 2);

    // Page 2: next 2
    let page2 = s.escrow.query_escrows_by_depositor(&s.depositor, &2, &2);
    assert_eq!(page2.len(), 2);

    // Page 3: nothing left
    let page3 = s.escrow.query_escrows_by_depositor(&s.depositor, &4, &2);
    assert_eq!(page3.len(), 0);

    // No overlap: ids from page1 and page2 must be disjoint
    assert_ne!(
        page1.get(0).unwrap().bounty_id,
        page2.get(0).unwrap().bounty_id
    );
    assert_ne!(
        page1.get(1).unwrap().bounty_id,
        page2.get(0).unwrap().bounty_id
    );
}

/// Deadline filter: when no escrow falls within the range, result is empty.
#[test]
fn test_query_by_deadline_no_results_outside_range() {
    let s = Setup::new();
    let base = s.env.ledger().timestamp();

    s.escrow.lock_funds(&s.depositor, &1, &100, &(base + 100));
    s.escrow.lock_funds(&s.depositor, &2, &200, &(base + 200));

    // Query a range that none of the deadlines fall into
    let results = s
        .escrow
        .query_escrows_by_deadline(&(base + 5000), &(base + 9999), &0, &10);
    assert_eq!(results.len(), 0);
}

/// get_escrow_ids_by_status: pagination with offset returns the correct slice.
#[test]
fn test_get_escrow_ids_by_status_pagination_offset_and_limit() {
    let s = Setup::new();
    let dl = s.env.ledger().timestamp() + 1000;

    for i in 1u64..=5 {
        s.escrow
            .lock_funds(&s.depositor, &i, &(i as i128 * 50), &dl);
    }

    // All 5 are Locked — paginate in slices of 2
    let page1 = s
        .escrow
        .get_escrow_ids_by_status(&EscrowStatus::Locked, &0, &2);
    assert_eq!(page1.len(), 2);

    let page2 = s
        .escrow
        .get_escrow_ids_by_status(&EscrowStatus::Locked, &2, &2);
    assert_eq!(page2.len(), 2);

    let page3 = s
        .escrow
        .get_escrow_ids_by_status(&EscrowStatus::Locked, &4, &2);
    assert_eq!(page3.len(), 1);

    // No ID should appear in more than one page
    assert_ne!(page1.get(0).unwrap(), page2.get(0).unwrap());
    assert_ne!(page2.get(0).unwrap(), page3.get(0).unwrap());
}

/// Aggregate stats: total_locked + total_released + total_refunded must equal
/// the sum of all original amounts locked, regardless of lifecycle state.
#[test]
fn test_aggregate_stats_amounts_invariant_sum_equals_total_locked() {
    let s = Setup::new();
    let now = s.env.ledger().timestamp();
    let dl = now + 100;

    // Lock 4 bounties with known amounts: 100 + 200 + 300 + 400 = 1000
    s.escrow.lock_funds(&s.depositor, &1, &100, &dl);
    s.escrow.lock_funds(&s.depositor, &2, &200, &dl);
    s.escrow.lock_funds(&s.depositor, &3, &300, &dl);
    s.escrow.lock_funds(&s.depositor, &4, &400, &dl);

    // Release bounty 1 and 2
    s.escrow.release_funds(&1, &s.contributor);
    s.escrow.release_funds(&2, &s.contributor);

    // Refund bounty 3 after deadline
    s.env.ledger().set_timestamp(dl + 1);
    s.escrow.refund(&3);

    // Bounty 4 stays Locked
    let stats = s.escrow.get_aggregate_stats();

    // Individual bucket checks
    assert_eq!(stats.total_released, 300); // 100 + 200
    assert_eq!(stats.total_refunded, 300); // 300
    assert_eq!(stats.total_locked, 400); // 400

    // Invariant: buckets sum to the total amount ever locked
    let total = stats.total_locked + stats.total_released + stats.total_refunded;
    assert_eq!(total, 1000);
}
