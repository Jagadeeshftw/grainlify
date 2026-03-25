#![cfg(test)]
//! Pagination and criteria-filtering tests for `get_programs`.

use super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{token, vec, Address, Env, String};

/// Sets up a test environment with contract, token, admin, and program_admin.
macro_rules! setup_search {
    ($env:ident, $client:ident, $contract_id:ident, $admin:ident,
     $program_admin:ident, $token_client:ident, $token_admin:ident,
     $initial_balance:expr) => {
        let $env = Env::default();
        $env.mock_all_auths();

        let $contract_id = $env.register(ProgramEscrowContract, ());
        let $client = ProgramEscrowContractClient::new(&$env, &$contract_id);

        let $admin = Address::generate(&$env);
        let $program_admin = Address::generate(&$env);

        let token_contract = $env.register_stellar_asset_contract_v2($admin.clone());
        let token_addr = token_contract.address();
        let $token_client = token::Client::new(&$env, &token_addr);
        let $token_admin = token::StellarAssetClient::new(&$env, &token_addr);

        let _ = $client.init(&$admin, &token_addr);
        $token_admin.mint(&$program_admin, &$initial_balance);
    };
}

// ==================== EMPTY STATE ====================

#[test]
fn test_search_empty_contract() {
    setup_search!(
        env, client, _contract_id, _admin, _program_admin,
        _token_client, _token_admin, 0i128
    );

    let criteria = ProgramSearchCriteria {
        status_filter: 0,
        admin: None,
    };
    let page = client.get_programs(&criteria, &None, &10);
    assert_eq!(page.records.len(), 0);
    assert_eq!(page.next_cursor, None);
    assert!(!page.has_more);
    assert_eq!(client.get_program_count(), 0);
}

// ==================== BASIC LISTING ====================

#[test]
fn test_search_lists_all_programs() {
    setup_search!(
        env, client, _contract_id, _admin, program_admin,
        _token_client, _token_admin, 100_000i128
    );

    for id in 1..=5u64 {
        client.register_program(
            &id,
            &program_admin,
            &String::from_str(&env, "Program"),
            &1_000,
        );
    }

    let criteria = ProgramSearchCriteria {
        status_filter: 0,
        admin: None,
    };
    let page = client.get_programs(&criteria, &None, &10);
    assert_eq!(page.records.len(), 5);
    assert!(!page.has_more);
    assert_eq!(page.next_cursor, None);
    assert_eq!(client.get_program_count(), 5);
}

// ==================== PAGINATION ====================

#[test]
fn test_search_pagination_basic() {
    setup_search!(
        env, client, _contract_id, _admin, program_admin,
        _token_client, _token_admin, 100_000i128
    );

    for id in 1..=5u64 {
        client.register_program(
            &id,
            &program_admin,
            &String::from_str(&env, "Program"),
            &1_000,
        );
    }

    let criteria = ProgramSearchCriteria {
        status_filter: 0,
        admin: None,
    };

    // First page: limit 2
    let page1 = client.get_programs(&criteria, &None, &2);
    assert_eq!(page1.records.len(), 2);
    assert!(page1.has_more);
    assert!(page1.next_cursor.is_some());
    assert_eq!(page1.records.get(0).unwrap().program_id, 1);
    assert_eq!(page1.records.get(1).unwrap().program_id, 2);

    // Second page
    let page2 = client.get_programs(&criteria, &page1.next_cursor, &2);
    assert_eq!(page2.records.len(), 2);
    assert!(page2.has_more);
    assert_eq!(page2.records.get(0).unwrap().program_id, 3);
    assert_eq!(page2.records.get(1).unwrap().program_id, 4);

    // Third page: last result
    let page3 = client.get_programs(&criteria, &page2.next_cursor, &2);
    assert_eq!(page3.records.len(), 1);
    assert!(!page3.has_more);
    assert_eq!(page3.next_cursor, None);
    assert_eq!(page3.records.get(0).unwrap().program_id, 5);
}

// ==================== FILTER BY STATUS ====================

#[test]
fn test_search_filter_by_status() {
    setup_search!(
        env, client, _contract_id, _admin, program_admin,
        _token_client, _token_admin, 100_000i128
    );

    // Register 3 programs — all Active by default
    for id in 1..=3u64 {
        client.register_program(
            &id,
            &program_admin,
            &String::from_str(&env, "Program"),
            &1_000,
        );
    }

    // All should be Active (status_filter = 1)
    let active_criteria = ProgramSearchCriteria {
        status_filter: 1,
        admin: None,
    };
    let page = client.get_programs(&active_criteria, &None, &10);
    assert_eq!(page.records.len(), 3);

    // None should be Completed (status_filter = 2)
    let completed_criteria = ProgramSearchCriteria {
        status_filter: 2,
        admin: None,
    };
    let page = client.get_programs(&completed_criteria, &None, &10);
    assert_eq!(page.records.len(), 0);
}

#[test]
fn test_search_filter_by_status_and_admin_together() {
    setup_search!(
        env, client, _contract_id, _admin, program_admin,
        _token_client, token_admin, 100_000i128
    );

    let other_admin = Address::generate(&env);
    token_admin.mint(&other_admin, &100_000);

    client.register_program(
        &1,
        &program_admin,
        &String::from_str(&env, "Mine Active A"),
        &1_000,
    );
    client.register_program(
        &2,
        &other_admin,
        &String::from_str(&env, "Other Active"),
        &1_000,
    );
    client.register_program(
        &3,
        &program_admin,
        &String::from_str(&env, "Mine Active B"),
        &1_000,
    );

    let criteria = ProgramSearchCriteria {
        status_filter: 1,
        admin: Some(program_admin.clone()),
    };

    let page = client.get_programs(&criteria, &None, &10);
    assert_eq!(page.records.len(), 2);
    assert_eq!(page.records.get(0).unwrap().program_id, 1);
    assert_eq!(page.records.get(1).unwrap().program_id, 3);
}

// ==================== FILTER BY ADMIN ====================

#[test]
fn test_search_filter_by_admin() {
    setup_search!(
        env, client, _contract_id, _admin, program_admin,
        _token_client, token_admin, 100_000i128
    );

    let other_admin = Address::generate(&env);
    token_admin.mint(&other_admin, &100_000);

    // Register 2 by program_admin, 1 by other_admin
    client.register_program(
        &1,
        &program_admin,
        &String::from_str(&env, "Admin1 Program"),
        &1_000,
    );
    client.register_program(
        &2,
        &other_admin,
        &String::from_str(&env, "Admin2 Program"),
        &1_000,
    );
    client.register_program(
        &3,
        &program_admin,
        &String::from_str(&env, "Admin1 Program 2"),
        &1_000,
    );

    // Filter by program_admin
    let criteria = ProgramSearchCriteria {
        status_filter: 0,
        admin: Some(program_admin.clone()),
    };
    let page = client.get_programs(&criteria, &None, &10);
    assert_eq!(page.records.len(), 2);
    assert_eq!(page.records.get(0).unwrap().program_id, 1);
    assert_eq!(page.records.get(1).unwrap().program_id, 3);

    // Filter by other_admin
    let criteria_other = ProgramSearchCriteria {
        status_filter: 0,
        admin: Some(other_admin.clone()),
    };
    let page_other = client.get_programs(&criteria_other, &None, &10);
    assert_eq!(page_other.records.len(), 1);
    assert_eq!(page_other.records.get(0).unwrap().program_id, 2);
}

// ==================== PAGE SIZE CAP ====================

#[test]
fn test_search_page_size_cap() {
    setup_search!(
        env, client, _contract_id, _admin, program_admin,
        _token_client, _token_admin, 1_000_000i128
    );

    // Create 25 programs
    for id in 1..=25u64 {
        client.register_program(
            &id,
            &program_admin,
            &String::from_str(&env, "Program"),
            &100,
        );
    }

    let criteria = ProgramSearchCriteria {
        status_filter: 0,
        admin: None,
    };

    // Request 100 (exceeds cap), should return 20
    let page = client.get_programs(&criteria, &None, &100);
    assert_eq!(page.records.len(), 20);
    assert!(page.has_more);
    assert!(page.next_cursor.is_some());
}

#[test]
fn test_search_zero_limit_defaults_to_max_page_size() {
    setup_search!(
        env, client, _contract_id, _admin, program_admin,
        _token_client, _token_admin, 1_000_000i128
    );

    for id in 1..=25u64 {
        client.register_program(
            &id,
            &program_admin,
            &String::from_str(&env, "Program"),
            &100,
        );
    }

    let criteria = ProgramSearchCriteria {
        status_filter: 0,
        admin: None,
    };

    let page = client.get_programs(&criteria, &None, &0);
    assert_eq!(page.records.len(), 20);
    assert!(page.has_more);
    assert_eq!(page.records.get(0).unwrap().program_id, 1);
}

#[test]
fn test_search_unknown_cursor_returns_empty_page() {
    setup_search!(
        env, client, _contract_id, _admin, program_admin,
        _token_client, _token_admin, 100_000i128
    );

    for id in 1..=3u64 {
        client.register_program(
            &id,
            &program_admin,
            &String::from_str(&env, "Program"),
            &1_000,
        );
    }

    let criteria = ProgramSearchCriteria {
        status_filter: 0,
        admin: None,
    };

    let page = client.get_programs(&criteria, &Some(999), &10);
    assert_eq!(page.records.len(), 0);
    assert_eq!(page.next_cursor, None);
    assert!(!page.has_more);
}

#[test]
fn test_search_cursor_skips_non_matching_records() {
    setup_search!(
        env, client, _contract_id, _admin, program_admin,
        _token_client, token_admin, 100_000i128
    );

    let other_admin = Address::generate(&env);
    token_admin.mint(&other_admin, &100_000);

    client.register_program(
        &1,
        &other_admin,
        &String::from_str(&env, "Other A"),
        &1_000,
    );
    client.register_program(
        &2,
        &program_admin,
        &String::from_str(&env, "Mine A"),
        &1_000,
    );
    client.register_program(
        &3,
        &other_admin,
        &String::from_str(&env, "Other B"),
        &1_000,
    );
    client.register_program(
        &4,
        &program_admin,
        &String::from_str(&env, "Mine B"),
        &1_000,
    );

    let criteria = ProgramSearchCriteria {
        status_filter: 0,
        admin: Some(program_admin.clone()),
    };

    let first = client.get_programs(&criteria, &None, &1);
    assert_eq!(first.records.len(), 1);
    assert_eq!(first.records.get(0).unwrap().program_id, 2);
    assert!(first.has_more);
    assert_eq!(first.next_cursor, Some(2));

    let second = client.get_programs(&criteria, &first.next_cursor, &1);
    assert_eq!(second.records.len(), 1);
    assert_eq!(second.records.get(0).unwrap().program_id, 4);
    assert!(!second.has_more);
    assert_eq!(second.next_cursor, None);
}

// ==================== BATCH REGISTRATION INDEX TRACKING ====================

#[test]
fn test_search_batch_registered_programs() {
    setup_search!(
        env, client, _contract_id, _admin, program_admin,
        _token_client, _token_admin, 100_000i128
    );

    let items = vec![
        &env,
        ProgramRegistrationItem {
            program_id: 10,
            admin: program_admin.clone(),
            name: String::from_str(&env, "Batch A"),
            total_funding: 1_000,
        },
        ProgramRegistrationItem {
            program_id: 20,
            admin: program_admin.clone(),
            name: String::from_str(&env, "Batch B"),
            total_funding: 2_000,
        },
        ProgramRegistrationItem {
            program_id: 30,
            admin: program_admin.clone(),
            name: String::from_str(&env, "Batch C"),
            total_funding: 3_000,
        },
    ];

    client.batch_register_programs(&items);

    let criteria = ProgramSearchCriteria {
        status_filter: 0,
        admin: None,
    };
    let page = client.get_programs(&criteria, &None, &10);
    assert_eq!(page.records.len(), 3);
    assert_eq!(page.records.get(0).unwrap().program_id, 10);
    assert_eq!(page.records.get(1).unwrap().program_id, 20);
    assert_eq!(page.records.get(2).unwrap().program_id, 30);
    assert_eq!(client.get_program_count(), 3);
}

#[test]
fn test_search_includes_jurisdiction_enabled_programs() {
    setup_search!(
        env, client, _contract_id, _admin, program_admin,
        _token_client, _token_admin, 100_000i128
    );

    let jurisdiction = ProgramJurisdictionConfig {
        tag: Some(String::from_str(&env, "EU-only")),
        requires_kyc: true,
        max_funding: Some(10_000),
        registration_paused: false,
    };

    client.register_program_juris(
        &10,
        &program_admin,
        &String::from_str(&env, "Jurisdiction Program"),
        &5_000,
        &jurisdiction.tag.clone(),
        &jurisdiction.requires_kyc,
        &jurisdiction.max_funding.clone(),
        &jurisdiction.registration_paused,
        &OptionalJurisdiction::Some(jurisdiction.clone()),
        &Some(true),
    );

    let criteria = ProgramSearchCriteria {
        status_filter: 1,
        admin: Some(program_admin.clone()),
    };

    let page = client.get_programs(&criteria, &None, &10);
    assert_eq!(page.records.len(), 1);
    let record = page.records.get(0).unwrap();
    assert_eq!(record.program_id, 10);
    assert_eq!(record.name, String::from_str(&env, "Jurisdiction Program"));
    assert_eq!(client.get_program_jurisdiction(&10), Some(jurisdiction));
}
