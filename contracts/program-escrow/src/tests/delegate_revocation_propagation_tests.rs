#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Events},
    token, vec, Address, Env, String, IntoVal,
};
use crate::{
    DELEGATE_PERMISSION_PAYOUT, ProgramEscrowContract, ProgramEscrowContractClient, ProgramDelegateInfo, ProgramDelegateRevokedEvent
};

pub struct Ctx<'a> {
    pub env: Env,
    pub client: ProgramEscrowContractClient<'a>,
    pub token_id: Address,
    pub admin: Address,
    pub payout_key: Address,
    pub delegate: Address,
}

pub fn setup() -> Ctx<'static> {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract(token_admin.clone());

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    client.initialize_contract(&admin);

    let payout_key = Address::generate(&env);
    let delegate = Address::generate(&env);

    let prog_id = String::from_str(&env, "PROG-REVOKE");
    
    token::StellarAssetClient::new(&env, &token_id).mint(&payout_key, &1000);
    
    client.init_program(
        &prog_id,
        &payout_key, // authorized_payout_key
        &token_id,
        &payout_key, // owner/creator
        &Some(1000),
        &None,
    );
    
    client.publish_program(&prog_id, &payout_key);

    Ctx {
        env,
        client,
        token_id,
        admin,
        payout_key,
        delegate,
    }
}

#[test]
fn test_emergency_revoke_propagates_immediately() {
    let ctx = setup();
    let prog_id = String::from_str(&ctx.env, "PROG-REVOKE");
    
    // Set delegate with payout permissions
    ctx.client.set_program_delegate(
        &prog_id,
        &ctx.payout_key,
        &ctx.delegate,
        &DELEGATE_PERMISSION_PAYOUT,
    );
    
    // Ensure delegate is present in query
    let delegates = ProgramEscrowContract::query_all_delegates(ctx.env.clone(), prog_id.clone());
    assert_eq!(delegates.len(), 1);
    assert_eq!(delegates.get(0).unwrap().delegate.unwrap(), ctx.delegate);
    
    // Emergency revoke
    ctx.client.emergency_revoke_delegate(&prog_id, &ctx.delegate);
    
    // Same-transaction check: delegate should immediately disappear
    let delegates_after = ProgramEscrowContract::query_all_delegates(ctx.env.clone(), prog_id.clone());
    assert_eq!(delegates_after.len(), 0, "Delegate must be immediately absent from queries");
    
    // Verify in-flight payout calls fail
    let recipient = Address::generate(&ctx.env);
    let res = ctx.client.try_single_payout_by(
        &ctx.delegate,
        &prog_id,
        &recipient,
        &100,
        &String::from_str(&ctx.env, "payout1"),
    );
    assert!(res.is_err(), "In-flight payout must fail after revocation");
    
    let recipients = vec![&ctx.env, recipient];
    let amounts = vec![&ctx.env, 100_i128];
    let res2 = ctx.client.try_batch_payout_by(
        &ctx.delegate,
        &prog_id,
        &recipients,
        &amounts,
    );
    assert!(res2.is_err(), "In-flight batch payout must fail after revocation");
}

pub mod test_facade {
    use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};
    use crate::{ProgramDelegateInfo, ProgramEscrowContractClient};

    #[contract]
    pub struct TestFacade;

    #[contractimpl]
    impl TestFacade {
        pub fn query_all_delegates(env: Env, program_contract: Address, program_id: String) -> Vec<ProgramDelegateInfo> {
            let client = ProgramEscrowContractClient::new(&env, &program_contract);
            client.query_all_delegates(&program_id)
        }
    }
}

use test_facade::{TestFacade, TestFacadeClient};

#[test]
fn test_emergency_revoke_propagates_to_facade_atomically() {
    let ctx = setup();
    let prog_id = String::from_str(&ctx.env, "PROG-REVOKE");
    
    // Deploy facade
    let facade_id = ctx.env.register_contract(None, TestFacade);
    let facade_client = TestFacadeClient::new(&ctx.env, &facade_id);
    
    // Set delegate
    ctx.client.set_program_delegate(
        &prog_id,
        &ctx.payout_key,
        &ctx.delegate,
        &DELEGATE_PERMISSION_PAYOUT,
    );
    
    // Check facade query
    let delegates_before = facade_client.query_all_delegates(&ctx.client.address, &prog_id);
    assert_eq!(delegates_before.len(), 1);
    
    // Revoke
    ctx.client.emergency_revoke_delegate(&prog_id, &ctx.delegate);
    
    // Facade should immediately see empty list
    let delegates_after = facade_client.query_all_delegates(&ctx.client.address, &prog_id);
    assert_eq!(delegates_after.len(), 0, "Facade must reflect revocation immediately");
}
