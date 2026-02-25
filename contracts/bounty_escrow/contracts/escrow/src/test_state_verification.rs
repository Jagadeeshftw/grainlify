#[cfg(test)]
mod test {
    use crate::{BountyEscrowContract, BountyEscrowContractClient, DataKey, EscrowStatus};
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{token, Address, Env};

    fn setup_bounty(
        env: &Env,
    ) -> (
        BountyEscrowContractClient<'static>,
        Address,
        Address,
        Address,
        Address,
    ) {
        env.mock_all_auths();
        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(env, &contract_id);

        let admin = Address::generate(env);
        let depositor = Address::generate(env);
        let token_admin = Address::generate(env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_id.address();
        let token_admin_client = token::StellarAssetClient::new(env, &token_address);

        client.init(&admin, &token_address);
        token_admin_client.mint(&depositor, &10_000);
        (client, contract_id, admin, depositor, token_address)
    }

    #[test]
    fn test_bounty_healthy_state_passes() {
        let env = Env::default();
        let (client, _contract_id, _admin, depositor, _token_id) = setup_bounty(&env);

        let bounty_id = 1u64;
        let amount = 1000i128;
        let deadline = env.ledger().timestamp() + 100;

        client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

        assert!(client.verify_state(&bounty_id));
    }

    #[test]
    fn test_bounty_tamper_amount_drift() {
        let env = Env::default();
        let (client, contract_id, _admin, depositor, _token_id) = setup_bounty(&env);

        let bounty_id = 1u64;
        client.lock_funds(&depositor, &bounty_id, &1000i128, &100);

        // TAMPER: Manually make remaining_amount > amount (must run in contract context)
        let mut escrow = client.get_escrow_info(&bounty_id);
        escrow.remaining_amount = 2000;
        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&DataKey::Escrow(bounty_id), &escrow);
        });

        assert!(
            !client.verify_state(&bounty_id),
            "Should fail when remaining > total"
        );
    }

    #[test]
    fn test_bounty_tamper_negative_amount() {
        let env = Env::default();
        let (client, contract_id, _admin, depositor, _token_id) = setup_bounty(&env);

        let bounty_id = 1u64;
        client.lock_funds(&depositor, &bounty_id, &1000i128, &100);

        // TAMPER: Manually set negative amount (must run in contract context)
        let mut escrow = client.get_escrow_info(&bounty_id);
        escrow.amount = -1;
        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&DataKey::Escrow(bounty_id), &escrow);
        });

        assert!(
            !client.verify_state(&bounty_id),
            "Should fail with negative amount"
        );
    }

    #[test]
    fn test_bounty_tamper_released_with_balance() {
        let env = Env::default();
        let (client, contract_id, _admin, depositor, _token_id) = setup_bounty(&env);

        let bounty_id = 1u64;
        client.lock_funds(&depositor, &bounty_id, &1000i128, &100);

        // TAMPER: Mark as Released but keep remaining_amount > 0 (must run in contract context)
        let mut escrow = client.get_escrow_info(&bounty_id);
        escrow.status = EscrowStatus::Released;
        escrow.remaining_amount = 100;
        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&DataKey::Escrow(bounty_id), &escrow);
        });

        assert!(
            !client.verify_state(&bounty_id),
            "Should fail if released with remaining balance"
        );
    }
}
