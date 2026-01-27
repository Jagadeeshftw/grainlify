//! Tests for event indexing and query functionality

#[cfg(test)]
mod event_query_tests {
    use crate::{
        BountyEscrowContract, BountyEscrowContractClient, EventFilter, EventType,
    };
    use soroban_sdk::{
        testutils::Address as _,
        token::{StellarAssetClient, TokenClient},
        vec, Address, Env, String,
    };

    fn create_token_contract<'a>(env: &Env, admin: &Address) -> (TokenClient<'a>, StellarAssetClient<'a>) {
        let contract = env.register_stellar_asset_contract_v2(admin.clone());
        let token_address = contract.address();
        (
            TokenClient::new(env, &token_address),
            StellarAssetClient::new(env, &token_address),
        )
    }

    #[test]
    fn test_event_indexing_on_lock() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let depositor = Address::generate(&env);

        let (token_client, asset_client) = create_token_contract(&env, &token_admin);
        asset_client.mint(&depositor, &10_000_000_000);

        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);

        // Initialize contract
        client.init(&admin, &token_client.address);

        // Lock funds
        let bounty_id = 1u64;
        let amount = 1_000_000_000i128;
        let deadline = env.ledger().timestamp() + 1000;

        client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

        // Query events
        let event_count = client.get_event_count();
        assert_eq!(event_count, 2); // Init + Lock

        // Query by bounty
        let history = client.get_bounty_history(&bounty_id);
        assert_eq!(history.len(), 1);
        assert_eq!(history.get(0).unwrap().event_type, EventType::Locked);
        assert_eq!(history.get(0).unwrap().amount, amount);
    }

    #[test]
    fn test_query_by_address() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let depositor = Address::generate(&env);

        let (token_client, asset_client) = create_token_contract(&env, &token_admin);
        asset_client.mint(&depositor, &20_000_000_000);

        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);

        client.init(&admin, &token_client.address);

        // Lock multiple bounties
        client.lock_funds(&depositor, &1, &1_000_000_000, &(env.ledger().timestamp() + 1000));
        client.lock_funds(&depositor, &2, &2_000_000_000, &(env.ledger().timestamp() + 1000));

        // Query by depositor address
        let result = client.get_address_history(&depositor, &10, &None);
        assert_eq!(result.total_count, 2);
    }

    #[test]
    fn test_query_with_filters() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);

        let (token_client, asset_client) = create_token_contract(&env, &token_admin);
        asset_client.mint(&depositor, &10_000_000_000);

        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);

        client.init(&admin, &token_client.address);

        // Lock and release
        let bounty_id = 1u64;
        let amount = 1_000_000_000i128;
        client.lock_funds(&depositor, &bounty_id, &amount, &(env.ledger().timestamp() + 1000));
        client.release_funds(&bounty_id, &contributor);

        // Query only release events
        let event_types = vec![&env, EventType::Released];
        let result = client.get_events_by_type(&event_types, &10, &None);
        
        assert_eq!(result.total_count, 1);
        assert_eq!(result.events.get(0).unwrap().event_type, EventType::Released);
    }

    #[test]
    fn test_pagination() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let depositor = Address::generate(&env);

        let (token_client, asset_client) = create_token_contract(&env, &token_admin);
        asset_client.mint(&depositor, &100_000_000_000);

        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);

        client.init(&admin, &token_client.address);

        // Lock 10 bounties
        for i in 1..=10 {
            client.lock_funds(
                &depositor,
                &i,
                &(i as i128 * 100_000_000),
                &(env.ledger().timestamp() + 1000),
            );
        }

        // First page
        let page1 = client.get_address_history(&depositor, &5, &None);
        assert_eq!(page1.total_count, 5);
        assert!(page1.has_more);

        // Second page
        let page2 = client.get_address_history(&depositor, &5, &page1.next_cursor);
        assert_eq!(page2.total_count, 5);
    }

    #[test]
    fn test_query_by_amount_range() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let depositor = Address::generate(&env);

        let (token_client, asset_client) = create_token_contract(&env, &token_admin);
        asset_client.mint(&depositor, &100_000_000_000);

        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);

        client.init(&admin, &token_client.address);

        // Lock bounties with different amounts
        client.lock_funds(&depositor, &1, &500_000_000, &(env.ledger().timestamp() + 1000));
        client.lock_funds(&depositor, &2, &1_500_000_000, &(env.ledger().timestamp() + 1000));
        client.lock_funds(&depositor, &3, &2_500_000_000, &(env.ledger().timestamp() + 1000));

        // Query amounts between 1-2 billion
        let result = client.get_events_by_amount(
            &Some(1_000_000_000),
            &Some(2_000_000_000),
            &10,
            &None,
        );

        assert_eq!(result.total_count, 1);
        assert_eq!(result.events.get(0).unwrap().amount, 1_500_000_000);
    }

    #[test]
    fn test_query_by_timerange() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let depositor = Address::generate(&env);

        let (token_client, asset_client) = create_token_contract(&env, &token_admin);
        asset_client.mint(&depositor, &10_000_000_000);

        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);

        client.init(&admin, &token_client.address);

        let start_time = env.ledger().timestamp();
        
        // Lock funds
        client.lock_funds(&depositor, &1, &1_000_000_000, &(start_time + 1000));

        let end_time = env.ledger().timestamp();

        // Query time range
        let result = client.get_events_by_timerange(&start_time, &end_time, &10, &None);
        
        assert!(result.total_count >= 1);
    }
}
