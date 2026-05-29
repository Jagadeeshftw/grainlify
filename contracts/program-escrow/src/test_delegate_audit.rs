#![cfg(test)]

use crate::{ProgramEscrowContract, ProgramEscrowContractClient, DELEGATE_PERMISSION_RELEASE, DELEGATE_PERMISSION_REFUND};
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
