#![cfg(test)]

use crate::{ProgramEscrowContract, ProgramEscrowContractClient, DELEGATE_PERMISSION_RELEASE, DELEGATE_PERMISSION_REFUND, ROTATION_TIMELOCK_DELAY};
use soroban_sdk::{Address, Env, String};

#[test]
fn test_query_program_delegates_returns_expected() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize_contract(&admin);

    let payout_key = Address::generate(&env);
    let token = Address::generate(&env);
    let prog1 = String::from_str(&env, "prog-1");
    let prog2 = String::from_str(&env, "prog-2");

    client.init_program(&prog1, &payout_key, &token, &payout_key, &None, &None);
    client.init_program(&prog2, &payout_key, &token, &payout_key, &None, &None);

    let delegate1 = Address::generate(&env);
    let delegate2 = Address::generate(&env);

    client.set_program_delegate(&prog1, &payout_key, &delegate1, &DELEGATE_PERMISSION_RELEASE);
    client.set_program_delegate(&prog2, &payout_key, &delegate2, &DELEGATE_PERMISSION_REFUND);

    let delegates = ProgramEscrowContract::query_program_delegates(env.clone(), Some(0u32), Some(10u32));

    assert_eq!(delegates.len(), 2);

    let mut found1 = false;
    let mut found2 = false;
    for d in delegates.iter() {
        if d.program_id == prog1 {
            assert_eq!(d.delegate.unwrap(), delegate1);
            assert_eq!(d.permissions, DELEGATE_PERMISSION_RELEASE);
            found1 = true;
        } else if d.program_id == prog2 {
            assert_eq!(d.delegate.unwrap(), delegate2);
            assert_eq!(d.permissions, DELEGATE_PERMISSION_REFUND);
            found2 = true;
        }
    }
    assert!(found1 && found2);
}

#[test]
fn test_delegate_reassignment_during_in_flight_rotation() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize_contract(&admin);

    let original_controller = Address::generate(&env);
    let proposed_controller = Address::generate(&env);
    let delegate = Address::generate(&env);
    let token = Address::generate(&env);
    let program_id = String::from_str(&env, "prog-rotation");

    // Init program
    client.init_program(&program_id, &original_controller, &token, &original_controller, &None, &None);

    env.ledger().with_mut(|li| li.timestamp = 1_000_000);

    // Propose new controller
    client.propose_controller(&program_id, &original_controller, &proposed_controller);

    // Set delegate while rotation is in-flight
    client.set_program_delegate(&program_id, &original_controller, &delegate, &DELEGATE_PERMISSION_RELEASE);

    // Assert delegate is set properly
    let info = client.get_program_data_by_id(&program_id);
    assert_eq!(info.delegate.unwrap(), delegate);
    assert_eq!(info.delegate_permissions, DELEGATE_PERMISSION_RELEASE);

    // Accept rotation should still succeed (rotation was not invalidated)
    env.ledger().with_mut(|li| li.timestamp = 1_000_000 + ROTATION_TIMELOCK_DELAY);
    client.accept_controller(&program_id);

    let info_after = client.get_program_data_by_id(&program_id);
    assert_eq!(info_after.authorized_payout_key, proposed_controller);
}

#[test]
fn test_delegate_carries_over_after_rotation_accepted() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize_contract(&admin);

    let original_controller = Address::generate(&env);
    let proposed_controller = Address::generate(&env);
    let delegate = Address::generate(&env);
    let token = Address::generate(&env);
    let program_id = String::from_str(&env, "prog-carryover");

    client.init_program(&program_id, &original_controller, &token, &original_controller, &None, &None);

    // Set delegate first
    client.set_program_delegate(&program_id, &original_controller, &delegate, &DELEGATE_PERMISSION_RELEASE);

    env.ledger().with_mut(|li| li.timestamp = 2_000_000);

    // Propose and accept rotation
    client.propose_controller(&program_id, &original_controller, &proposed_controller);
    
    env.ledger().with_mut(|li| li.timestamp = 2_000_000 + ROTATION_TIMELOCK_DELAY);
    client.accept_controller(&program_id);

    // Assert that the delegate carried over to the new controller
    let info = client.get_program_data_by_id(&program_id);
    assert_eq!(info.authorized_payout_key, proposed_controller);
    assert_eq!(info.delegate.unwrap(), delegate);
    assert_eq!(info.delegate_permissions, DELEGATE_PERMISSION_RELEASE);
}
