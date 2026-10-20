#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env};

use crate::{
    errors::ContractError, DynamicPricingConfig, ProgramEscrowContract, ProgramEscrowContractClient,
};

fn setup_client(env: &Env) -> ProgramEscrowContractClient<'static> {
    env.mock_all_auths();
    let admin = Address::generate(env);
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);
    client.initialize_contract(&admin);
    client
}

fn config(
    base_fee_bps: i128,
    max_fee_bps: i128,
    min_fee_bps: i128,
    max_change_bps: i128,
    smoothing_alpha_bps: i128,
    min_update_interval: u64,
) -> DynamicPricingConfig {
    DynamicPricingConfig {
        enabled: true,
        base_fee_bps,
        max_fee_bps,
        min_fee_bps,
        max_change_bps,
        smoothing_alpha_bps,
        min_update_interval,
        oracle_address: None,
        use_demand_pricing: true,
        use_supply_pricing: true,
        use_time_decay: true,
    }
}

#[test]
fn get_dynamic_pricing_config_preserves_numeric_and_time_boundaries() {
    let env = Env::default();
    let client = setup_client(&env);

    let low_boundary = config(0, 1, 0, 0, 1, 1);
    client.configure_dynamic_pricing(&low_boundary);
    assert_eq!(
        client.get_dynamic_pricing_config(),
        Some(low_boundary),
        "zero and one values should round-trip"
    );

    let maximum_boundary = config(10_000, 10_000, 10_000, 10_000, 10_000, u64::MAX);
    client.configure_dynamic_pricing(&maximum_boundary);
    assert_eq!(
        client.get_dynamic_pricing_config(),
        Some(maximum_boundary),
        "maximum accepted values should round-trip"
    );
}

#[test]
fn configure_dynamic_pricing_rejects_one_past_max_with_typed_error() {
    let env = Env::default();
    let client = setup_client(&env);

    for invalid_config in [
        config(10_001, 10_000, 0, 0, 0, 1),
        config(0, 10_000, 0, 10_001, 0, 1),
        config(0, 10_000, 0, 0, 10_001, 1),
        config(0, 10_000, 0, 0, 0, 0),
    ] {
        let result = client.try_configure_dynamic_pricing(&invalid_config);
        assert!(
            matches!(
                result,
                Err(Ok(error))
                    if error.get_code() == ContractError::InvalidDynamicPricingConfig as u32
            ),
            "invalid config should return InvalidDynamicPricingConfig, got {result:?}"
        );
    }
}
