#[cfg(test)]
mod pause_tests {
    use crate::{ProgramEscrowContract, ProgramEscrowContractClient};
    use soroban_sdk::{testutils::Address as _, token, Address, Env, String};

    fn create_token<'a>(env: &Env, admin: &Address) -> token::Client<'a> {
        let addr = env.register_stellar_asset_contract(admin.clone());
        token::Client::new(env, &addr)
    }

    #[test]
    fn test_pause() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        client.pause();
        assert!(client.is_paused());
    }

    #[test]
    #[should_panic(expected = "Contract is paused")]
    fn test_lock_blocked_when_paused() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let token = create_token(&env, &admin);
        let prog_id = String::from_str(&env, "Test");

        client.initialize_program(&prog_id, &admin, &token.address);
        client.pause();
        client.lock_program_funds(&prog_id, &1000);
    }

    #[test]
    fn test_unpause() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        client.pause();
        client.unpause();
        assert!(!client.is_paused());
    }

    #[test]
    fn test_emergency_withdraw() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let token = create_token(&env, &admin);
        let recipient = Address::generate(&env);
        let prog_id = String::from_str(&env, "Test");

        client.initialize_program(&prog_id, &admin, &token.address);
        client.pause();
        client.emergency_withdraw(&prog_id, &recipient);
    }

    #[test]
    fn test_pause_state_persists() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        client.pause();
        assert!(client.is_paused());
        assert!(client.is_paused());
    }
}
