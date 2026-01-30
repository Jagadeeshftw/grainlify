#[cfg(test)]
mod pause_tests {
    use crate::{BountyEscrowContract, BountyEscrowContractClient};
    use soroban_sdk::{testutils::Address as _, token, Address, Env};

    fn create_token<'a>(env: &Env, admin: &Address) -> token::Client<'a> {
        let addr = env.register_stellar_asset_contract(admin.clone());
        token::Client::new(env, &addr)
    }

    #[test]
    fn test_pause() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let token = create_token(&env, &admin);

        client.init(&admin, &token.address);
        client.pause();
        assert!(client.is_paused());
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #11)")]
    fn test_lock_blocked_when_paused() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let token = create_token(&env, &admin);

        client.init(&admin, &token.address);
        client.pause();
        client.lock_funds(&admin, &1, &1000, &9999);
    }

    #[test]
    fn test_unpause() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let token = create_token(&env, &admin);

        client.init(&admin, &token.address);
        client.pause();
        client.unpause();
        assert!(!client.is_paused());
    }

    #[test]
    fn test_emergency_withdraw() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let token = create_token(&env, &admin);
        let recipient = Address::generate(&env);

        client.init(&admin, &token.address);
        client.pause();
        client.emergency_withdraw(&recipient);
    }

    #[test]
    fn test_pause_state_persists() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let token = create_token(&env, &admin);

        client.init(&admin, &token.address);
        client.pause();
        assert!(client.is_paused());
        assert!(client.is_paused());
    }
}
