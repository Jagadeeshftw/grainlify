use soroban_sdk::{Address, BytesN, Env};

pub struct MockContractFactory;

impl MockContractFactory {
    pub fn create_contract(env: &Env, wasm_hash: &BytesN<32>) -> Address {
        env.deployer()
            .with_current_contract(*wasm_hash)
            .deploy(wasm_hash)
    }

    pub fn register_contract(env: &Env, contract_id: &Address) {
        // Registration logic if needed
    }
}
