#[cfg(test)]
mod network_config_tests {
    use soroban_sdk::{testutils::Address as _, Address, Env, String};
    use grainlify_core::{GrainlifyContract, GrainlifyContractClient};

    #[test]
    fn test_network_initialization() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let chain_id = String::from_str(&env, "stellar");
        let network_id = String::from_str(&env, "testnet");
        
        client.init_with_network(&admin, &chain_id, &network_id);

        // Verify initialization
        assert_eq!(client.get_version(), 2);
        
        // Verify network configuration
        let retrieved_chain = client.get_chain_id();
        let retrieved_network = client.get_network_id();
        
        assert_eq!(retrieved_chain, Some(chain_id));
        assert_eq!(retrieved_network, Some(network_id));
    }

    #[test]
    fn test_network_info_getter() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let chain_id = String::from_str(&env, "ethereum");
        let network_id = String::from_str(&env, "mainnet");
        
        client.init_with_network(&admin, &chain_id, &network_id);

        // Test tuple getter
        let (chain, network) = client.get_network_info();
        assert_eq!(chain, Some(chain_id));
        assert_eq!(network, Some(network_id));
    }

    #[test]
    #[should_panic(expected = "Already initialized")]
    fn test_cannot_reinitialize_network_config() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin1 = Address::generate(&env);
        let admin2 = Address::generate(&env);
        let chain_id = String::from_str(&env, "stellar");
        let network_id = String::from_str(&env, "testnet");

        // First initialization should succeed
        client.init_with_network(&admin1, &chain_id, &network_id);
        
        // Second initialization should panic
        client.init_with_network(&admin2, &chain_id, &network_id);
    }

    #[test]
    fn test_legacy_init_still_works() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        
        // Legacy init should still work (without network config)
        client.init_admin(&admin);
        
        // Network info should be None for legacy initialization
        assert_eq!(client.get_chain_id(), None);
        assert_eq!(client.get_network_id(), None);
        let (chain, network) = client.get_network_info();
        assert_eq!(chain, None);
        assert_eq!(network, None);
    }
}