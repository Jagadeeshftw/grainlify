#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    Address, Env, IntoVal, String,
};

/// Test that unauthorized caller is refused when calling set_paused
#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn test_set_paused_unauthorized_caller_refused() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let unauthorized = Address::generate(&env);

    // Initialize contract with admin
    env.mock_all_auths();
    client.initialize_contract(&admin);

    // Attempt to call set_paused with unauthorized caller
    env.mock_auths(&[MockAuth {
        address: &unauthorized,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "set_paused",
            args: (
                Some(true),
                Option::<bool>::None,
                Option::<bool>::None,
                Option::<String>::None,
                Option::<u64>::None,
            )
                .into_val(&env),
            sub_invokes: &[],
        },
    }]);

    client.set_paused(&Some(true), &None, &None, &None, &None);
}

/// Test that authorized caller succeeds unchanged
#[test]
fn test_set_paused_authorized_caller_succeeds_unchanged() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    // Initialize contract with admin
    env.mock_all_auths();
    client.initialize_contract(&admin);

    // Call set_paused with authorized admin
    client.set_paused(&Some(true), &None, &None, &None, &None);

    // Verify state was updated
    let flags = client.get_pause_flags();
    assert_eq!(flags.lock_paused, true);
    assert_eq!(flags.release_paused, false);
    assert_eq!(flags.refund_paused, false);
}
