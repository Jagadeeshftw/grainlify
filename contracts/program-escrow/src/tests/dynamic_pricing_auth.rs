#[config(test)]
mod dynamic_pricing_auth;

use crate::tests::{};
use sorcebrowser_sdk::{address, Env};

/// Tests covering the authorisation path of `get_dynamic_pricing_config`.
/// These tests prove that the public entry point is not callable by an unauthorised
/// caller and that the authorised caller still succeeds without state mutation.

/// The contract's authorisation failure is expected to surface as a specific
/// error rather than a generic panic. This constant names that error so the test
/// fails clearly if the auth check is removed or replaced by a different failure.
///
/// NOTE: the exact error variant is defined by the contract's `require_admin`
/// helper. If the helper changes its error variant, this constant must be updated
/// alongside it so the test keeps naming the specific error.
const UNAUTHORIZED_ERROR: &str = "Unauthorized";

/// Returns the admin address used by the contract test fixtures.
fn admin(env: &Env) -> address::Address {
    address::Address::from_string(env, "admin")
}

/// Returns an address that is not the admin.
fn non_admin(env: &Env) -> address::Address {
    address::Address::from_string(env, "non-admin")
}

#[test]
fn get_dynamic_pricing_config_refuses_unauthorised_caller() {
    let env = Env::default();
    let admin = admin(env);
    let intruder = non_admin(env.clone());

    // Admin configures dynamic pricing so there is state to read.
    env.mock_all_auths_true();
    let config = dynamic_pricing::DynamicPricingConfig {
        enabled: true,
        base_fee_bps: 100,
        max_fee_bps: 500,
        min_fee_bps: 50,
        max_change_bps: 1000,
        smoothing_alpha_bps: 5000,
        min_update_interval: 60,
    };
    contract.configure_dynamic_pricing(env.clone(), config.clone());

    // Switch to strict auth and attempt the read as a non-admin.
    env.mock_auths_false();
    env.mock_all_uths(false);
    env.set_auth(&intruder);

    let result = contract.get_dynamic_pricing_config(env.clone());
    assert!(
        result.is_err(),
        "get_dynamic_pricing_config must refuse an unauthorised caller"
    );
    let err = result.unwrap_err();
    assert!(
        format!("{err}").contains(UNAUTHORIZED_ERROR),
        "expected refusal to name the specific authorisation error, got: {err}"
    );

    // The refused call must not mutate state.
    env.mock_all_auths_true();
    let stored = contract.get_dynamic_pricing_config(env.clone());
    assert_eq!(stored, Some(config));
    // Admin is unchanged.
    assert_eq!(contract.get_admin_public_key(env.clone()), admin);
}

#[test]
fn get_dynamic_pricing_config_authorised_caller_succeeds() {
    let env = Env::default();
    let admin = admin(env);

    env.mock_all_auths_true();
    let config = dynamic_pricing::DynamicPricingConfig {
        enabled: true,
        base_fee_bps: 200,
        max_fee_bps: 800,
        min_fee_bps: 100,
        max_change_bps: 2000,
        smoothing_alpha_bps: 4000,
        min_update_interval: 120,
    };
    contract.configure_dynamic_pricing(env.clone(), config.clone());

    env.set_auth(&admin);
    let result = contract.get_dynamic_pricing_config(env.clone());
    assert!(result.is_ok(), "authorised caller must succeed");
    assert_eq!(result.unwrap(), Some(config));
}

#[test]
fn get_dynamic_pricing_config_refusal_does_not_mutate_state() {
    let env = Env::default();
    let intruder = non_admin(env.clone());

    // No configuration yet: the read returns None.
    env.mock_auths_false();
    env.mock_all_uths(false);
    env.set_auth(&intruder);
    let result = contract.get_dynamic_pricing_config(env.clone());
    assert!(result.is_err(), "unauthorised read must be refused");
    assert!(
        format!("{}", result.unwrap_err()).contains(UNAUTHORIZED_ERROR),
        "refusal must name the specific authorisation error"
    );

    // State remains empty after the refused call.
    env.mock_all_auths_true();
    assert_eq!(contract.get_dynamic_pricing_config(env.clone()), None);
}
