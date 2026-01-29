use crate::mocks::MockContractFactory;
use soroban_sdk::{Address, Env};

pub struct TestEnv {
    pub env: Env,
    pub admin: Address,
}

impl TestEnv {
    pub fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        Self { env, admin }
    }

    pub fn with_contract(&self, wasm_hash: &soroban_sdk::BytesN<32>) -> Address {
        MockContractFactory::create_contract(&self.env, wasm_hash)
    }
}
