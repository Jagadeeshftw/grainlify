use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    vec, Address, BytesN, Env, String,
};

#[test]
fn test_batch_payout_with_receipt() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = sac.address();
    let token_client = token::Client::new(&env, &token_id);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_id);

    let program_id = String::from_str(&env, "hack-merkle-06");
    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    client.publish_program(&program_id);

    token_admin_client.mint(&client.address, &10_000_000);
    client.lock_program_funds_v2(&program_id, &10_000_000);

    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);

    let recipients = vec![&env, recipient1.clone(), recipient2.clone()];
    let amounts = vec![&env, 1000, 2000];
    let merkle_root = BytesN::from_array(&env, &[1u8; 32]);

    let receipt = client.batch_payout_with_receipt(&recipients, &amounts, &merkle_root);

    assert_eq!(receipt.batch_id, 0);
    assert_eq!(receipt.total_amount, 3000);
    assert_eq!(receipt.recipient_count, 2);
    assert_eq!(receipt.merkle_root, merkle_root);

    let stored_receipt = client.get_batch_receipt(&0);
    assert_eq!(stored_receipt, receipt);

    let balance1 = token_client.balance(&recipient1);
    let balance2 = token_client.balance(&recipient2);
    assert_eq!(balance1, 1000);
    assert_eq!(balance2, 2000);
}
