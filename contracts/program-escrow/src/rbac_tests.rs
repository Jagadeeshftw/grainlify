#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    Address, Env, IntoVal, String,
};

struct RbacSetup<'a> {
    env: Env,
    contract_id: Address,
    admin: Address,
    operator: Address,
    pauser: Address,
    outsider: Address,
    client: ProgramEscrowContractClient<'a>,
}

impl<'a> RbacSetup<'a> {
    fn new() -> Self {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let operator = Address::generate(&env);
        let pauser = Address::generate(&env);
        let outsider = Address::generate(&env);

        let token_admin = Address::generate(&env);
        let token_id = env
            .register_stellar_asset_contract_v2(token_admin.clone())
            .address();

        let program_id = String::from_str(&env, "RBAC-Test");

        // Initialize contract with admin
        client.initialize_contract(&admin);

        // Initialize program with operator
        // Note: Currently init_program doesn't have auth, so we can just call it
        client.init_program(&program_id, &operator, &token_id);

        // Initialize circuit breaker with pauser
        // caller is None for first setting
        client.set_circuit_admin(&pauser, &None);

        Self {
            env,
            contract_id,
            admin,
            operator,
            pauser,
            outsider,
            client,
        }
    }
}

#[test]
fn test_admin_can_set_pause_flags() {
    let setup = RbacSetup::new();
    setup.env.mock_auths(&[MockAuth {
        address: &setup.admin,
        invoke: &MockAuthInvoke {
            contract: &setup.contract_id,
            fn_name: "set_paused",
            args: (
                Some(true),
                Option::<bool>::None,
                Option::<bool>::None,
                Option::<String>::None,
            )
                .into_val(&setup.env),
            sub_invokes: &[],
        },
    }]);

    setup.client.set_paused(&Some(true), &None, &None, &None);
    assert!(setup.client.get_pause_flags().lock_paused);
}

#[test]
#[should_panic]
fn test_non_admin_cannot_set_pause_flags() {
    let setup = RbacSetup::new();

    setup.env.mock_auths(&[MockAuth {
        address: &setup.outsider,
        invoke: &MockAuthInvoke {
            contract: &setup.contract_id,
            fn_name: "set_paused",
            args: (
                Some(true),
                Option::<bool>::None,
                Option::<bool>::None,
                Option::<String>::None,
            )
                .into_val(&setup.env),
            sub_invokes: &[],
        },
    }]);

    setup.client.set_paused(&Some(true), &None, &None, &None);
}

#[test]
fn test_operator_can_trigger_program_releases() {
    let setup = RbacSetup::new();
    setup.env.mock_auths(&[MockAuth {
        address: &setup.operator,
        invoke: &MockAuthInvoke {
            contract: &setup.contract_id,
            fn_name: "trigger_program_releases",
            args: ().into_val(&setup.env),
            sub_invokes: &[],
        },
    }]);

    assert_eq!(setup.client.trigger_program_releases(), 0);
}

#[test]
#[should_panic]
fn test_admin_cannot_trigger_releases() {
    let setup = RbacSetup::new();
    setup.env.mock_auths(&[MockAuth {
        address: &setup.admin,
        invoke: &MockAuthInvoke {
            contract: &setup.contract_id,
            fn_name: "trigger_program_releases",
            args: ().into_val(&setup.env),
            sub_invokes: &[],
        },
    }]);

    setup.client.trigger_program_releases();
}

#[test]
fn test_pauser_can_reset_and_configure_circuit_breaker() {
    let setup = RbacSetup::new();
    setup.env.mock_all_auths();
    setup.client.reset_circuit_breaker(&setup.pauser);

    setup
        .client
        .configure_circuit_breaker(&setup.pauser, &5, &2, &20);
}

#[test]
#[should_panic]
fn test_admin_cannot_reset_circuit() {
    let setup = RbacSetup::new();
    setup.env.mock_auths(&[MockAuth {
        address: &setup.admin,
        invoke: &MockAuthInvoke {
            contract: &setup.contract_id,
            fn_name: "reset_circuit_breaker",
            args: (setup.admin.clone(),).into_val(&setup.env),
            sub_invokes: &[],
        },
    }]);

    setup.client.reset_circuit_breaker(&setup.admin);
}

#[test]
#[should_panic]
fn test_operator_cannot_reset_circuit() {
    let setup = RbacSetup::new();
    setup.env.mock_auths(&[MockAuth {
        address: &setup.operator,
        invoke: &MockAuthInvoke {
            contract: &setup.contract_id,
            fn_name: "reset_circuit_breaker",
            args: (setup.operator.clone(),).into_val(&setup.env),
            sub_invokes: &[],
        },
    }]);

    setup.client.reset_circuit_breaker(&setup.operator);
}

#[test]
#[should_panic]
fn test_pauser_cannot_set_pause_flags() {
    let setup = RbacSetup::new();
    setup.env.mock_auths(&[MockAuth {
        address: &setup.pauser,
        invoke: &MockAuthInvoke {
            contract: &setup.contract_id,
            fn_name: "set_paused",
            args: (
                Some(true),
                Option::<bool>::None,
                Option::<bool>::None,
                Option::<String>::None,
            )
                .into_val(&setup.env),
            sub_invokes: &[],
        },
    }]);

    setup.client.set_paused(&Some(true), &None, &None, &None);
}

#[test]
fn test_circuit_admin_can_rotate_assignment() {
    let setup = RbacSetup::new();
    let new_pauser = Address::generate(&setup.env);

    setup.env.mock_auths(&[MockAuth {
        address: &setup.pauser,
        invoke: &MockAuthInvoke {
            contract: &setup.contract_id,
            fn_name: "set_circuit_admin",
            args: (new_pauser.clone(), Some(setup.pauser.clone())).into_val(&setup.env),
            sub_invokes: &[],
        },
    }]);

    setup
        .client
        .set_circuit_admin(&new_pauser, &Some(setup.pauser.clone()));

    assert_eq!(setup.client.get_circuit_admin(), Some(new_pauser));
}

#[test]
#[should_panic]
fn test_non_circuit_admin_cannot_rotate_assignment() {
    let setup = RbacSetup::new();
    let new_pauser = Address::generate(&setup.env);

    setup.env.mock_auths(&[MockAuth {
        address: &setup.outsider,
        invoke: &MockAuthInvoke {
            contract: &setup.contract_id,
            fn_name: "set_circuit_admin",
            args: (new_pauser.clone(), Some(setup.pauser.clone())).into_val(&setup.env),
            sub_invokes: &[],
        },
    }]);

    setup
        .client
        .set_circuit_admin(&new_pauser, &Some(setup.pauser.clone()));
}

#[test]
#[should_panic]
fn test_old_circuit_admin_cannot_reset_after_rotation() {
    let setup = RbacSetup::new();
    let new_pauser = Address::generate(&setup.env);

    setup.env.mock_auths(&[MockAuth {
        address: &setup.pauser,
        invoke: &MockAuthInvoke {
            contract: &setup.contract_id,
            fn_name: "set_circuit_admin",
            args: (new_pauser.clone(), Some(setup.pauser.clone())).into_val(&setup.env),
            sub_invokes: &[],
        },
    }]);
    setup
        .client
        .set_circuit_admin(&new_pauser, &Some(setup.pauser.clone()));

    setup.env.mock_auths(&[MockAuth {
        address: &setup.pauser,
        invoke: &MockAuthInvoke {
            contract: &setup.contract_id,
            fn_name: "reset_circuit_breaker",
            args: (setup.pauser.clone(),).into_val(&setup.env),
            sub_invokes: &[],
        },
    }]);
    setup.client.reset_circuit_breaker(&setup.pauser);
}

#[test]
fn test_new_circuit_admin_can_reset_after_rotation() {
    let setup = RbacSetup::new();
    let new_pauser = Address::generate(&setup.env);

    setup.env.mock_auths(&[MockAuth {
        address: &setup.pauser,
        invoke: &MockAuthInvoke {
            contract: &setup.contract_id,
            fn_name: "set_circuit_admin",
            args: (new_pauser.clone(), Some(setup.pauser.clone())).into_val(&setup.env),
            sub_invokes: &[],
        },
    }]);
    setup
        .client
        .set_circuit_admin(&new_pauser, &Some(setup.pauser.clone()));

    setup.env.mock_all_auths();
    setup.client.reset_circuit_breaker(&new_pauser);
}
