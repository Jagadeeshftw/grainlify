#[cfg(test)]
mod test_fee_routing {
    use crate::{BountyEscrowContract, BountyEscrowContractClient};
    use soroban_sdk::{testutils::Address as _, token, Address, Env};

    fn make_token<'a>(
        env: &'a Env,
        admin: &Address,
    ) -> (Address, token::Client<'a>, token::StellarAssetClient<'a>) {
        let sac = env.register_stellar_asset_contract_v2(admin.clone());
        let addr = sac.address();
        let client = token::Client::new(env, &addr);
        let admin_client = token::StellarAssetClient::new(env, &addr);
        (addr, client, admin_client)
    }

    fn make_setup<'a>(
        env: &'a Env,
    ) -> (
        BountyEscrowContractClient<'a>,
        token::Client<'a>,
        token::StellarAssetClient<'a>,
        Address,
        Address,
    ) {
        let admin = Address::generate(env);
        let token_admin = Address::generate(env);
        let (token_addr, token_client, token_admin_client) = make_token(env, &token_admin);
        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(env, &contract_id);
        client.init(&admin, &token_addr);
        (client, token_client, token_admin_client, admin, contract_id)
    }

    #[test]
    fn lock_fee_routes_to_treasury_and_partner_split() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, token_client, token_admin, _admin, contract_id) = make_setup(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);
        let treasury = Address::generate(&env);
        let partner = Address::generate(&env);

        token_admin.mint(&depositor, &1_000);
        client.update_fee_config(&Some(1000), &Some(0), &Some(treasury.clone()), &Some(true));
        client.set_fee_routing(&1, &treasury, &7000, &Some(partner.clone()), &3000);

        client.lock_funds(&depositor, &1, &1_000, &(env.ledger().timestamp() + 1_000));
        client.release_funds(&1, &contributor);

        // lock fee = 100, split 70/30, escrow stores and releases 900
        assert_eq!(token_client.balance(&treasury), 70);
        assert_eq!(token_client.balance(&partner), 30);
        assert_eq!(token_client.balance(&contributor), 900);
        assert_eq!(token_client.balance(&contract_id), 0);
    }

    #[test]
    fn release_fee_split_is_deterministic_with_remainder_to_partner() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, token_client, token_admin, _admin, contract_id) = make_setup(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);
        let treasury = Address::generate(&env);
        let partner = Address::generate(&env);

        token_admin.mint(&depositor, &1_000);
        client.update_fee_config(&Some(0), &Some(333), &Some(treasury.clone()), &Some(true));
        client.set_fee_routing(&2, &treasury, &5000, &Some(partner.clone()), &5000);

        client.lock_funds(&depositor, &2, &1_000, &(env.ledger().timestamp() + 1_000));
        client.release_funds(&2, &contributor);

        // release fee = floor(1000 * 333 / 10000) = 33
        // split 50/50 => treasury 16, partner 17 (deterministic remainder)
        assert_eq!(token_client.balance(&treasury), 16);
        assert_eq!(token_client.balance(&partner), 17);
        assert_eq!(token_client.balance(&contributor), 967);
        assert_eq!(token_client.balance(&contract_id), 0);
    }

    #[test]
    fn default_routing_uses_fee_recipient_when_no_bounty_override() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, token_client, token_admin, _admin, contract_id) = make_setup(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);
        let treasury = Address::generate(&env);

        token_admin.mint(&depositor, &1_000);
        client.update_fee_config(&Some(0), &Some(500), &Some(treasury.clone()), &Some(true));

        client.lock_funds(&depositor, &3, &1_000, &(env.ledger().timestamp() + 1_000));
        client.release_funds(&3, &contributor);

        assert_eq!(token_client.balance(&treasury), 50);
        assert_eq!(token_client.balance(&contributor), 950);
        assert_eq!(token_client.balance(&contract_id), 0);
    }

    #[test]
    fn reject_invalid_fee_routing_basis_points() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _token_client, _token_admin, _admin, _contract_id) = make_setup(&env);
        let treasury = Address::generate(&env);
        let partner = Address::generate(&env);

        let result = client.try_set_fee_routing(&9, &treasury, &9000, &Some(partner), &500);
        assert!(result.is_err());
    }
}
