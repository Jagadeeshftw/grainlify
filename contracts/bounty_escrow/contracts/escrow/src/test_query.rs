extern crate std;
use crate::{
    BountyEscrowContract, BountyEscrowContractClient, EscrowFilter, EscrowStatus, Pagination,
};
use soroban_sdk::{testutils::Address as _, token, Address, Env};

fn create_token_contract<'a>(
    e: &'a Env,
    admin: &Address,
) -> (Address, token::Client<'a>, token::StellarAssetClient<'a>) {
    let token_id = e.register_stellar_asset_contract_v2(admin.clone());
    let token = token_id.address();
    let token_client = token::Client::new(e, &token);
    let token_admin_client = token::StellarAssetClient::new(e, &token);
    (token, token_client, token_admin_client)
}

fn create_test_env(
    env: &Env,
) -> (
    BountyEscrowContractClient<'_>,
    Address,
    Address,
    token::Client<'_>,
    token::StellarAssetClient<'_>,
) {
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(env, &contract_id);

    let admin = Address::generate(env);
    let (token, token_client, token_admin) = create_token_contract(env, &admin);

    // Initialize
    client.init(&admin, &token);

    (client, admin, token, token_client, token_admin)
}

#[test]
fn test_get_bounties_filtering() {
    let env = Env::default();
    let (client, _admin, _token, _token_client, token_admin) = create_test_env(&env);

    let depositor1 = Address::generate(&env);
    let depositor2 = Address::generate(&env);

    // Mint tokens
    token_admin.mint(&depositor1, &10000);
    token_admin.mint(&depositor2, &10000);

    let now = env.ledger().timestamp();
    let deadline1 = now + 1000;
    let deadline2 = now + 2000;

    // Create 3 bounties
    // 1. Depositor 1, 100 amount, deadline1
    client.lock_funds(&depositor1, &1, &100, &deadline1);

    // 2. Depositor 1, 200 amount, deadline2
    client.lock_funds(&depositor1, &2, &200, &deadline2);

    // 3. Depositor 2, 300 amount, deadline2
    client.lock_funds(&depositor2, &3, &300, &deadline2);

    // Filter by Depositor 1
    let filter_dep1 = EscrowFilter {
        status: None,
        depositor: Some(depositor1.clone()),
        min_amount: None,
        max_amount: None,
        start_time: None,
        end_time: None,
    };
    let bounds = Pagination {
        start_index: 0,
        limit: 10,
    };
    let bounties_dep1 = client.get_bounties(&filter_dep1, &bounds);
    assert_eq!(bounties_dep1.len(), 2);
    assert_eq!(bounties_dep1.get(0).unwrap().0, 1);
    assert_eq!(bounties_dep1.get(1).unwrap().0, 2);

    // Filter by Min Amount 250
    let filter_amt = EscrowFilter {
        status: None,
        depositor: None,
        min_amount: Some(250),
        max_amount: None,
        start_time: None,
        end_time: None,
    };
    let bounties_amt = client.get_bounties(&filter_amt, &bounds);
    assert_eq!(bounties_amt.len(), 1);
    assert_eq!(bounties_amt.get(0).unwrap().0, 3);

    // Filter by Time (Deadline > 1500)
    let filter_time = EscrowFilter {
        status: None,
        depositor: None,
        min_amount: None,
        max_amount: None,
        start_time: Some(deadline1 + 100), // > deadline1
        end_time: None,
    };
    let bounties_time = client.get_bounties(&filter_time, &bounds);
    assert_eq!(bounties_time.len(), 2); // Bounty 2 and 3

    // Filter by Status (Locked is default)
    let filter_status = EscrowFilter {
        status: Some(EscrowStatus::Locked as u32),
        depositor: None,
        min_amount: None,
        max_amount: None,
        start_time: None,
        end_time: None,
    };
    let bounties_status = client.get_bounties(&filter_status, &bounds);
    assert_eq!(bounties_status.len(), 3);
}

#[test]
fn test_get_stats() {
    let env = Env::default();
    let (client, _admin, _token, _token_client, token_admin) = create_test_env(&env);
    let depositor = Address::generate(&env);
    token_admin.mint(&depositor, &10000);

    let now = env.ledger().timestamp();

    client.lock_funds(&depositor, &1, &100, &(now + 1000));
    client.lock_funds(&depositor, &2, &200, &(now + 2000));

    let stats = client.get_stats();
    assert_eq!(stats.total_bounties, 2);
    assert_eq!(stats.total_locked_amount, 300);
    assert_eq!(stats.total_released_amount, 0);

    // Release one
    client.release_funds(&1, &Address::generate(&env));

    let stats_after = client.get_stats();
    assert_eq!(stats_after.total_locked_amount, 200);
    assert_eq!(stats_after.total_released_amount, 100);
}

#[test]
fn test_pagination() {
    let env = Env::default();
    let (client, _admin, _token, _token_client, token_admin) = create_test_env(&env);
    let depositor = Address::generate(&env);
    token_admin.mint(&depositor, &10000);

    let now = env.ledger().timestamp();

    for i in 1..=5 {
        client.lock_funds(&depositor, &i, &100, &(now + 1000));
    }

    let filter_none = EscrowFilter {
        status: None,
        depositor: None,
        min_amount: None,
        max_amount: None,
        start_time: None,
        end_time: None,
    };

    // Page 1: 2 items
    let page1 = client.get_bounties(
        &filter_none,
        &Pagination {
            start_index: 0,
            limit: 2,
        },
    );
    assert_eq!(page1.len(), 2);
    assert_eq!(page1.get(0).unwrap().0, 1);
    assert_eq!(page1.get(1).unwrap().0, 2);

    // Page 2: 2 items (skip 2)
    let page2 = client.get_bounties(
        &filter_none,
        &Pagination {
            start_index: 2,
            limit: 2,
        },
    );
    assert_eq!(page2.len(), 2);
    assert_eq!(page2.get(0).unwrap().0, 3);
    assert_eq!(page2.get(1).unwrap().0, 4);

    // Page 3: 1 item (skip 4)
    let page3 = client.get_bounties(
        &filter_none,
        &Pagination {
            start_index: 4,
            limit: 2,
        },
    );
    assert_eq!(page3.len(), 1);
    assert_eq!(page3.get(0).unwrap().0, 5);
}

#[test]
fn test_large_dataset_pagination() {
    let env = Env::default();
    let (client, _admin, _token, _token_client, token_admin) = create_test_env(&env);
    let depositor = Address::generate(&env);
    token_admin.mint(&depositor, &100000);

    let now = env.ledger().timestamp();

    // Create 10 bounties
    for i in 1..=10 {
        client.lock_funds(&depositor, &i, &100, &(now + 1000));
    }

    // Query middle page (items 4-6)
    let filter_none = EscrowFilter {
        status: None,
        depositor: None,
        min_amount: None,
        max_amount: None,
        start_time: None,
        end_time: None,
    };

    let page = client.get_bounties(
        &filter_none,
        &Pagination {
            start_index: 3,
            limit: 3,
        },
    );
    assert_eq!(page.len(), 3);
    assert_eq!(page.get(0).unwrap().0, 4);
    assert_eq!(page.get(2).unwrap().0, 6);

    // Query end of list
    let last_page = client.get_bounties(
        &filter_none,
        &Pagination {
            start_index: 8,
            limit: 5,
        },
    );
    assert_eq!(last_page.len(), 2); // 9, 10
    assert_eq!(last_page.get(0).unwrap().0, 9);

    // Verify aggregation still works
    let stats = client.get_stats();
    assert_eq!(stats.total_bounties, 10);
    assert_eq!(stats.total_locked_amount, 1000);
}
