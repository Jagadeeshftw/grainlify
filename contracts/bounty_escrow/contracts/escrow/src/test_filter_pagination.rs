//! Tests for `query_whitelist` / `query_blocklist` pagination semantics,
//! `get_whitelist_count` / `get_blocklist_count`, `has_more` accuracy,
//! page-size cap (`MAX_PARTICIPANT_FILTER_PAGE_SIZE`), and audit events.

#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token, Address, Env, IntoVal, Symbol, TryIntoVal,
};

// ── helpers ──────────────────────────────────────────────────────────────────

fn create_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000_000);
    env
}

fn setup(env: &Env) -> BountyEscrowContractClient<'_> {
    let admin = Address::generate(env);
    let token_admin = Address::generate(env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(env, &contract_id);
    client.init(&admin, &token_address);
    client
}

fn add_n_to_whitelist(env: &Env, client: &BountyEscrowContractClient<'_>, n: u32) -> Vec<Address> {
    let mut addrs = Vec::new(env);
    for _ in 0..n {
        let a = Address::generate(env);
        client.set_whitelist_entry(&a, &true);
        addrs.push_back(a);
    }
    addrs
}

fn add_n_to_blocklist(env: &Env, client: &BountyEscrowContractClient<'_>, n: u32) -> Vec<Address> {
    let mut addrs = Vec::new(env);
    for _ in 0..n {
        let a = Address::generate(env);
        client.set_blocklist_entry(&a, &true);
        addrs.push_back(a);
    }
    addrs
}

fn last_pf_query_event(env: &Env) -> events::ParticipantFilterQueried {
    use soroban_sdk::symbol_short;
    let all = env.events().all();
    for event in all.iter().rev() {
        let topics = event.1;
        if topics.len() < 1 {
            continue;
        }
        let tag: Symbol = topics.get(0).unwrap().into_val(env);
        if tag != symbol_short!("pf_query") {
            continue;
        }
        return event.2.try_into_val(env).expect("pf_query event");
    }
    panic!("expected a ParticipantFilterQueried event");
}

// ── count functions ───────────────────────────────────────────────────────────

#[test]
fn test_whitelist_count_zero_initially() {
    let env = create_env();
    let client = setup(&env);
    assert_eq!(client.get_whitelist_count(), 0);
}

#[test]
fn test_blocklist_count_zero_initially() {
    let env = create_env();
    let client = setup(&env);
    assert_eq!(client.get_blocklist_count(), 0);
}

#[test]
fn test_whitelist_count_increments_on_add() {
    let env = create_env();
    let client = setup(&env);
    add_n_to_whitelist(&env, &client, 3);
    assert_eq!(client.get_whitelist_count(), 3);
}

#[test]
fn test_blocklist_count_increments_on_add() {
    let env = create_env();
    let client = setup(&env);
    add_n_to_blocklist(&env, &client, 5);
    assert_eq!(client.get_blocklist_count(), 5);
}

#[test]
fn test_whitelist_count_decrements_on_remove() {
    let env = create_env();
    let client = setup(&env);
    let a = Address::generate(&env);
    let b = Address::generate(&env);
    client.set_whitelist_entry(&a, &true);
    client.set_whitelist_entry(&b, &true);
    assert_eq!(client.get_whitelist_count(), 2);
    client.set_whitelist_entry(&a, &false);
    assert_eq!(client.get_whitelist_count(), 1);
}

#[test]
fn test_blocklist_count_decrements_on_remove() {
    let env = create_env();
    let client = setup(&env);
    let a = Address::generate(&env);
    client.set_blocklist_entry(&a, &true);
    assert_eq!(client.get_blocklist_count(), 1);
    client.set_blocklist_entry(&a, &false);
    assert_eq!(client.get_blocklist_count(), 0);
}

#[test]
fn test_count_not_affected_by_duplicate_add() {
    let env = create_env();
    let client = setup(&env);
    let a = Address::generate(&env);
    client.set_whitelist_entry(&a, &true);
    client.set_whitelist_entry(&a, &true);
    assert_eq!(client.get_whitelist_count(), 1);
}

// ── ParticipantListPage fields ────────────────────────────────────────────────

#[test]
fn test_query_whitelist_empty_list_returns_zero_total() {
    let env = create_env();
    let client = setup(&env);
    let page = client.query_whitelist(&0, &10);
    assert_eq!(page.items.len(), 0);
    assert_eq!(page.total, 0);
    assert_eq!(page.offset, 0);
    assert!(!page.has_more);
}

#[test]
fn test_query_blocklist_empty_list_returns_zero_total() {
    let env = create_env();
    let client = setup(&env);
    let page = client.query_blocklist(&0, &10);
    assert_eq!(page.items.len(), 0);
    assert_eq!(page.total, 0);
    assert!(!page.has_more);
}

#[test]
fn test_total_reflects_full_list_size_regardless_of_page() {
    let env = create_env();
    let client = setup(&env);
    add_n_to_whitelist(&env, &client, 7);

    let p1 = client.query_whitelist(&0, &3);
    assert_eq!(p1.total, 7);

    let p2 = client.query_whitelist(&3, &3);
    assert_eq!(p2.total, 7);

    let p3 = client.query_whitelist(&6, &3);
    assert_eq!(p3.total, 7);
}

#[test]
fn test_has_more_true_when_more_items_follow() {
    let env = create_env();
    let client = setup(&env);
    add_n_to_whitelist(&env, &client, 5);

    let page = client.query_whitelist(&0, &3);
    assert_eq!(page.items.len(), 3);
    assert!(page.has_more);
}

#[test]
fn test_has_more_false_on_last_page() {
    let env = create_env();
    let client = setup(&env);
    add_n_to_whitelist(&env, &client, 5);

    let page = client.query_whitelist(&3, &5);
    assert_eq!(page.items.len(), 2);
    assert!(!page.has_more);
}

#[test]
fn test_has_more_false_when_exact_page_boundary() {
    let env = create_env();
    let client = setup(&env);
    add_n_to_whitelist(&env, &client, 4);

    let page = client.query_whitelist(&0, &4);
    assert_eq!(page.items.len(), 4);
    assert!(!page.has_more);
}

#[test]
fn test_offset_field_echoed_in_page() {
    let env = create_env();
    let client = setup(&env);
    add_n_to_whitelist(&env, &client, 5);

    let page = client.query_whitelist(&2, &2);
    assert_eq!(page.offset, 2);
}

// ── pagination correctness ────────────────────────────────────────────────────

#[test]
fn test_pages_are_contiguous_and_non_overlapping() {
    let env = create_env();
    let client = setup(&env);
    add_n_to_whitelist(&env, &client, 6);

    let p1 = client.query_whitelist(&0, &2);
    let p2 = client.query_whitelist(&2, &2);
    let p3 = client.query_whitelist(&4, &2);

    assert_eq!(p1.items.len(), 2);
    assert_eq!(p2.items.len(), 2);
    assert_eq!(p3.items.len(), 2);

    for i in 0..p1.items.len() {
        let a = p1.items.get(i).unwrap();
        for j in 0..p2.items.len() {
            assert_ne!(a, p2.items.get(j).unwrap());
        }
        for j in 0..p3.items.len() {
            assert_ne!(a, p3.items.get(j).unwrap());
        }
    }
}

#[test]
fn test_offset_beyond_total_returns_empty() {
    let env = create_env();
    let client = setup(&env);
    add_n_to_whitelist(&env, &client, 3);

    let page = client.query_whitelist(&100, &10);
    assert_eq!(page.items.len(), 0);
    assert_eq!(page.total, 3);
    assert!(!page.has_more);
}

#[test]
fn test_limit_zero_returns_empty_items() {
    let env = create_env();
    let client = setup(&env);
    add_n_to_whitelist(&env, &client, 5);

    let page = client.query_whitelist(&0, &0);
    assert_eq!(page.items.len(), 0);
    assert_eq!(page.total, 5);
}

#[test]
fn test_last_partial_page_has_correct_count() {
    let env = create_env();
    let client = setup(&env);
    add_n_to_whitelist(&env, &client, 5);

    let page = client.query_whitelist(&4, &10);
    assert_eq!(page.items.len(), 1);
    assert!(!page.has_more);
}

#[test]
fn test_blocklist_pagination_mirrors_whitelist_semantics() {
    let env = create_env();
    let client = setup(&env);
    add_n_to_blocklist(&env, &client, 5);

    let p1 = client.query_blocklist(&0, &2);
    assert_eq!(p1.items.len(), 2);
    assert_eq!(p1.total, 5);
    assert!(p1.has_more);

    let p2 = client.query_blocklist(&4, &2);
    assert_eq!(p2.items.len(), 1);
    assert!(!p2.has_more);
}

// ── page size cap ─────────────────────────────────────────────────────────────

#[test]
fn test_limit_capped_at_max_page_size() {
    let env = create_env();
    let client = setup(&env);
    // Add more than the cap (50) to the whitelist
    add_n_to_whitelist(&env, &client, 60);

    let page = client.query_whitelist(&0, &200);
    assert_eq!(page.items.len(), 50);
    assert_eq!(page.total, 60);
    assert!(page.has_more);
}

#[test]
fn test_limit_exactly_at_cap_returns_cap_items() {
    let env = create_env();
    let client = setup(&env);
    add_n_to_whitelist(&env, &client, 50);

    let page = client.query_whitelist(&0, &50);
    assert_eq!(page.items.len(), 50);
    assert!(!page.has_more);
}

#[test]
fn test_blocklist_cap_enforced_same_as_whitelist() {
    let env = create_env();
    let client = setup(&env);
    add_n_to_blocklist(&env, &client, 55);

    let page = client.query_blocklist(&0, &999);
    assert_eq!(page.items.len(), 50);
    assert!(page.has_more);
}

// ── audit events ──────────────────────────────────────────────────────────────

#[test]
fn test_query_whitelist_emits_audit_event() {
    let env = create_env();
    let client = setup(&env);
    add_n_to_whitelist(&env, &client, 3);

    client.query_whitelist(&0, &2);

    let ev = last_pf_query_event(&env);
    assert_eq!(ev.list_type, events::ParticipantFilterListType::Allowlist);
    assert_eq!(ev.offset, 0);
    assert_eq!(ev.limit, 2);
    assert_eq!(ev.result_count, 2);
    assert_eq!(ev.total, 3);
    assert_eq!(ev.timestamp, 1_000_000);
}

#[test]
fn test_query_blocklist_emits_audit_event() {
    let env = create_env();
    let client = setup(&env);
    add_n_to_blocklist(&env, &client, 4);

    client.query_blocklist(&2, &5);

    let ev = last_pf_query_event(&env);
    assert_eq!(ev.list_type, events::ParticipantFilterListType::Blocklist);
    assert_eq!(ev.offset, 2);
    assert_eq!(ev.result_count, 2);
    assert_eq!(ev.total, 4);
}

#[test]
fn test_audit_event_reflects_capped_limit() {
    let env = create_env();
    let client = setup(&env);
    add_n_to_whitelist(&env, &client, 10);

    client.query_whitelist(&0, &999);

    let ev = last_pf_query_event(&env);
    assert_eq!(ev.limit, 50);
}

#[test]
fn test_audit_event_empty_query_has_zero_result_count() {
    let env = create_env();
    let client = setup(&env);

    client.query_whitelist(&0, &10);

    let ev = last_pf_query_event(&env);
    assert_eq!(ev.result_count, 0);
    assert_eq!(ev.total, 0);
}

// ── count + query consistency ─────────────────────────────────────────────────

#[test]
fn test_count_equals_total_field_in_page() {
    let env = create_env();
    let client = setup(&env);
    add_n_to_whitelist(&env, &client, 7);

    let count = client.get_whitelist_count();
    let page = client.query_whitelist(&0, &3);
    assert_eq!(count, page.total);
}

#[test]
fn test_blocklist_count_equals_total_field_in_page() {
    let env = create_env();
    let client = setup(&env);
    add_n_to_blocklist(&env, &client, 4);

    let count = client.get_blocklist_count();
    let page = client.query_blocklist(&0, &2);
    assert_eq!(count, page.total);
}

#[test]
fn test_total_decreases_after_removal() {
    let env = create_env();
    let client = setup(&env);
    let a = Address::generate(&env);
    client.set_whitelist_entry(&a, &true);
    add_n_to_whitelist(&env, &client, 2);

    assert_eq!(client.query_whitelist(&0, &10).total, 3);

    client.set_whitelist_entry(&a, &false);
    assert_eq!(client.query_whitelist(&0, &10).total, 2);
    assert_eq!(client.get_whitelist_count(), 2);
}

// ── independent list isolation ────────────────────────────────────────────────

#[test]
fn test_whitelist_and_blocklist_are_independent_indexes() {
    let env = create_env();
    let client = setup(&env);
    let a = Address::generate(&env);
    let b = Address::generate(&env);

    client.set_whitelist_entry(&a, &true);
    client.set_blocklist_entry(&b, &true);

    assert_eq!(client.get_whitelist_count(), 1);
    assert_eq!(client.get_blocklist_count(), 1);

    let wp = client.query_whitelist(&0, &10);
    let bp = client.query_blocklist(&0, &10);

    assert_eq!(wp.items.get(0).unwrap(), a);
    assert_eq!(bp.items.get(0).unwrap(), b);
}

#[test]
fn test_address_on_both_lists_counted_independently() {
    let env = create_env();
    let client = setup(&env);
    let a = Address::generate(&env);

    client.set_whitelist_entry(&a, &true);
    client.set_blocklist_entry(&a, &true);

    assert_eq!(client.get_whitelist_count(), 1);
    assert_eq!(client.get_blocklist_count(), 1);
}

// ── schema version written on init ───────────────────────────────────────────

#[test]
fn test_participant_list_schema_version_written_on_init() {
    let env = create_env();
    let client = setup(&env);
    // Schema version must be readable — verified indirectly by ensuring
    // paginated queries work correctly from the first call after init.
    let page = client.query_whitelist(&0, &5);
    assert_eq!(page.total, 0);
    assert_eq!(page.items.len(), 0);
}
