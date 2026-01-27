//! Tests for program escrow event indexing and query functionality

#[cfg(test)]
mod program_event_query_tests {
    use crate::{EventFilter, EventType, ProgramEscrowContract, ProgramEscrowContractClient};
    use soroban_sdk::{
        testutils::Address as _,
        token, vec, Address, Env, String,
    };

    fn create_token_contract<'a>(env: &Env, admin: &Address) -> token::Client<'a> {
        let token_address = env.register_stellar_asset_contract(admin.clone());
        token::Client::new(env, &token_address)
    }

    #[test]
    fn test_program_event_indexing() {
        let env = Env::default();
        env.mock_all_auths();

        let backend = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let organizer = Address::generate(&env);

        let token_client = create_token_contract(&env, &token_admin);
        token_client.mint(&organizer, &100_000_000_000);

        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let program_id = String::from_str(&env, "Hackathon2024");

        // Initialize program
        client.initialize_program(&program_id, &backend, &token_client.address);

        // Query events
        let event_count = client.get_event_count();
        assert_eq!(event_count, 1); // Registration

        // Query program history
        let history = client.get_program_history(&program_id);
        assert_eq!(history.len(), 1);
        assert_eq!(history.get(0).unwrap().event_type, EventType::ProgramRegistered);
    }

    #[test]
    fn test_program_funds_locking_indexed() {
        let env = Env::default();
        env.mock_all_auths();

        let backend = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let organizer = Address::generate(&env);

        let token_client = create_token_contract(&env, &token_admin);
        token_client.mint(&organizer, &100_000_000_000);

        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let program_id = String::from_str(&env, "Hackathon2024");
        client.initialize_program(&program_id, &backend, &token_client.address);

        // Transfer and lock funds
        token_client.transfer(&organizer, &contract_id, &10_000_000_000);
        client.lock_program_funds(&program_id, &10_000_000_000);

        // Query history
        let history = client.get_program_history(&program_id);
        assert_eq!(history.len(), 2); // Register + Lock

        let lock_event = history.get(1).unwrap();
        assert_eq!(lock_event.event_type, EventType::FundsLocked);
        assert_eq!(lock_event.amount, 10_000_000_000);
    }

    #[test]
    fn test_payout_event_indexing() {
        let env = Env::default();
        env.mock_all_auths();

        let backend = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let organizer = Address::generate(&env);
        let winner = Address::generate(&env);

        let token_client = create_token_contract(&env, &token_admin);
        token_client.mint(&organizer, &100_000_000_000);

        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let program_id = String::from_str(&env, "Hackathon2024");
        client.initialize_program(&program_id, &backend, &token_client.address);

        // Lock and payout
        token_client.transfer(&organizer, &contract_id, &10_000_000_000);
        client.lock_program_funds(&program_id, &10_000_000_000);
        client.single_payout(&program_id, &winner, &1_000_000_000);

        // Query recipient history
        let winner_history = client.get_recipient_history(&winner, &10, &None);
        assert_eq!(winner_history.total_count, 1);
        
        let payout_event = winner_history.events.get(0).unwrap();
        assert_eq!(payout_event.event_type, EventType::Payout);
        assert_eq!(payout_event.amount, 1_000_000_000);
        assert_eq!(payout_event.address, winner);
    }

    #[test]
    fn test_batch_payout_indexing() {
        let env = Env::default();
        env.mock_all_auths();

        let backend = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let organizer = Address::generate(&env);

        let token_client = create_token_contract(&env, &token_admin);
        token_client.mint(&organizer, &100_000_000_000);

        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let program_id = String::from_str(&env, "Hackathon2024");
        client.initialize_program(&program_id, &backend, &token_client.address);

        // Lock funds
        token_client.transfer(&organizer, &contract_id, &10_000_000_000);
        client.lock_program_funds(&program_id, &10_000_000_000);

        // Batch payout
        let winners = vec![
            &env,
            Address::generate(&env),
            Address::generate(&env),
            Address::generate(&env),
        ];
        let amounts = vec![&env, 3_000_000_000i128, 2_000_000_000, 1_000_000_000];

        client.batch_payout(&program_id, &winners, &amounts);

        // Query by event type
        let payout_types = vec![&env, EventType::BatchPayout];
        let result = client.get_events_by_type(&payout_types, &10, &None);
        
        assert_eq!(result.total_count, 1);
        assert_eq!(result.events.get(0).unwrap().amount, 6_000_000_000);
    }

    #[test]
    fn test_query_pagination_program() {
        let env = Env::default();
        env.mock_all_auths();

        let backend = Address::generate(&env);
        let token_admin = Address::generate(&env);

        let token_client = create_token_contract(&env, &token_admin);
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        // Register multiple programs
        for i in 1..=10 {
            let program_id = String::from_str(&env, &format!("Program{}", i));
            client.initialize_program(&program_id, &backend, &token_client.address);
        }

        // Paginate through events
        let page1 = client.query_events(&None, &5, &None);
        assert_eq!(page1.total_count, 5);
        assert!(page1.has_more);

        let page2 = client.query_events(&None, &5, &page1.next_cursor);
        assert_eq!(page2.total_count, 5);
    }

    #[test]
    fn test_query_by_amount_program() {
        let env = Env::default();
        env.mock_all_auths();

        let backend = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let organizer = Address::generate(&env);

        let token_client = create_token_contract(&env, &token_admin);
        token_client.mint(&organizer, &100_000_000_000);

        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let program_id = String::from_str(&env, "Hackathon2024");
        client.initialize_program(&program_id, &backend, &token_client.address);

        // Multiple payouts with different amounts
        token_client.transfer(&organizer, &contract_id, &10_000_000_000);
        client.lock_program_funds(&program_id, &10_000_000_000);

        let winner1 = Address::generate(&env);
        let winner2 = Address::generate(&env);
        let winner3 = Address::generate(&env);

        client.single_payout(&program_id, &winner1, &500_000_000);
        client.single_payout(&program_id, &winner2, &1_500_000_000);
        client.single_payout(&program_id, &winner3, &2_500_000_000);

        // Query payouts between 1-2 billion
        let result = client.get_events_by_amount(
            &Some(1_000_000_000),
            &Some(2_000_000_000),
            &10,
            &None,
        );

        assert_eq!(result.total_count, 1);
    }

    #[test]
    fn test_multiple_programs_isolation() {
        let env = Env::default();
        env.mock_all_auths();

        let backend = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let organizer = Address::generate(&env);

        let token_client = create_token_contract(&env, &token_admin);
        token_client.mint(&organizer, &100_000_000_000);

        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let program1 = String::from_str(&env, "Hackathon1");
        let program2 = String::from_str(&env, "Hackathon2");

        client.initialize_program(&program1, &backend, &token_client.address);
        client.initialize_program(&program2, &backend, &token_client.address);

        // Query each program separately
        let history1 = client.get_program_history(&program1);
        let history2 = client.get_program_history(&program2);

        assert_eq!(history1.len(), 1);
        assert_eq!(history2.len(), 1);
        assert_eq!(history1.get(0).unwrap().program_id, program1);
        assert_eq!(history2.get(0).unwrap().program_id, program2);
    }
}
