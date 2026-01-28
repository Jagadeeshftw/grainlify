use soroban_sdk::{Env, Address, BytesN};
use rand::Rng;

pub fn generate_random_bytes(env: &Env) -> BytesN<32> {
    let mut rng = rand::thread_rng();
    let mut bytes = [0u8; 32];
    rng.fill(&mut bytes);
    BytesN::from_array(env, &bytes)
}

pub fn generate_random_address(env: &Env) -> Address {
    Address::generate(env)
}
