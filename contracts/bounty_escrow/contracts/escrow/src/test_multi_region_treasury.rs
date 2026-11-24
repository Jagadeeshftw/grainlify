// test_multi_region_treasury.rs
//
// Tests for Issue #597: Multi-Region Treasury Distribution
//
// This module tests the multi-region treasury distribution feature that allows
// configuring multiple treasury destinations with weights for automatic distribution
// of fees across regions or business units.
//
// Architecture:
// - TreasuryDestination: contains address, weight, and region identifier
// - FeeConfig: extended to include treasury_destinations and distribution_enabled
// - Distribution uses weighted allocation based on configured weights
//
// Key invariants tested:
//   1. Treasury destinations can be configured with weights
//   2. Fees are distributed proportionally based on weights
//   3. Distribution can be enabled/disabled
//   4. Events are emitted for each distribution
//   5. Backward compatibility with single recipient mode

#[cfg(test)]
mod test_multi_region_treasury {
    use crate::{BountyEscrowContract, BountyEscrowContractClient, TreasuryDestination};
    use soroban_sdk::{testutils::Address as _, token, Address, Env, String, Vec};

    // ─── Helpers ────────────────────────────────────────────────────────────

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

    fn make_escrow_instance<'a>(
        env: &'a Env,
        admin: &Address,
        token: &Address,
    ) -> BountyEscrowContractClient<'a> {
        let id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(env, &id);
        client.init(admin, token);
        client
    }

    // ─── 1. Treasury destinations configuration tests ───────────────────────

    /// Test that treasury destinations can be configured with weights
    #[test]
    fn test_treasury_destinations_can_be_configured() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);

        let (token_addr, _token_client, token_minter) = make_token(&env, &token_admin);
        let client = make_escrow_instance(&env, &admin, &token_addr);

        // Create treasury destinations for different regions
        let na_address = Address::generate(&env);
        let eu_address = Address::generate(&env);
        let apac_address = Address::generate(&env);

        let mut destinations = Vec::new(&env);
        destinations.push_back(TreasuryDestination {
            address: na_address,
            weight: 5000, // 50%
            region: String::from_str(&env, "north_america"),
        });
        destinations.push_back(TreasuryDestination {
            address: eu_address,
            weight: 3000, // 30%
            region: String::from_str(&env, "europe"),
        });
        destinations.push_back(TreasuryDestination {
            address: apac_address,
            weight: 2000, // 20%
            region: String::from_str(&env, "asia_pacific"),
        });

        // Configure treasury distribution
        client.set_treasury_distributions(&destinations, &true);

        // Verify configuration
        let (retrieved_destinations, distribution_enabled) = client.get_treasury_distributions();
        assert!(distribution_enabled);
        assert_eq!(retrieved_destinations.len(), 3);
    }

    /// Test that treasury distribution can be disabled
    #[test]
    fn test_treasury_distribution_can_be_disabled() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);

        let (token_addr, _token_client, _token_minter) = make_token(&env, &token_admin);
        let client = make_escrow_instance(&env, &admin, &token_addr);

        // Create treasury destinations
        let treasury_address = Address::generate(&env);
        let mut destinations = Vec::new(&env);
        destinations.push_back(TreasuryDestination {
            address: treasury_address,
            weight: 10000,
            region: String::from_str(&env, "global"),
        });

        // Configure with distribution disabled
        client.set_treasury_distributions(&destinations, &false);

        // Verify distribution is disabled
        let (_destinations, distribution_enabled) = client.get_treasury_distributions();
        assert!(!distribution_enabled);
    }

    /// Test that treasury distribution fails when enabled but no destinations
    #[test]
    fn test_treasury_distribution_fails_with_no_destinations() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);

        let (token_addr, _token_client, _token_minter) = make_token(&env, &token_admin);
        let client = make_escrow_instance(&env, &admin, &token_addr);

        // Try to enable distribution with empty destinations - should fail
        let empty_destinations = Vec::new(&env);
        let result = client.try_set_treasury_distributions(&empty_destinations, &true);
        assert!(result.is_err());
    }

    /// Test that treasury distribution fails when enabled but total weight is zero
    #[test]
    fn test_treasury_distribution_fails_with_zero_weight() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);

        let (token_addr, _token_client, _token_minter) = make_token(&env, &token_admin);
        let client = make_escrow_instance(&env, &admin, &token_addr);

        // Create destination with zero weight
        let treasury_address = Address::generate(&env);
        let mut destinations = Vec::new(&env);
        destinations.push_back(TreasuryDestination {
            address: treasury_address,
            weight: 0,
            region: String::from_str(&env, "global"),
        });

        // Try to enable distribution with zero weight - should fail
        let result = client.try_set_treasury_distributions(&destinations, &true);
        assert!(result.is_err());
    }

    // ─── 2. Fee distribution with treasury destinations ─────────────────────

    /// Test that fees are distributed proportionally when treasury distribution is enabled
    #[test]
    fn test_fees_distributed_proportionally() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let depositor = Address::generate(&env);

        let (token_addr, token_client, token_minter) = make_token(&env, &token_admin);
        let client = make_escrow_instance(&env, &admin, &token_addr);

        // Setup treasury destinations with specific weights
        let na_address = Address::generate(&env);
        let eu_address = Address::generate(&env);

        let mut destinations = Vec::new(&env);
        destinations.push_back(TreasuryDestination {
            address: na_address.clone(),
            weight: 6000, // 60%
            region: String::from_str(&env, "north_america"),
        });
        destinations.push_back(TreasuryDestination {
            address: eu_address.clone(),
            weight: 4000, // 40%
            region: String::from_str(&env, "europe"),
        });

        // Configure fee and treasury distribution
        client.set_treasury_distributions(&destinations, &true);
        client.update_fee_config(&Some(1000), &Some(500), &None, &None, &None, &Some(true)); // 10% lock, 5% release

        // Mint tokens to depositor
        token_minter.mint(&depositor, &1000);

        // Lock funds - should incur 10% lock fee = 100 tokens
        // 60 goes to NA, 40 goes to EU
        let deadline = env.ledger().timestamp() + 1000;
        client.lock_funds(&depositor, &1u64, &1000i128, &deadline);

        // Verify contract balance (net amount after fee)
        let contract_balance = token_client.balance(&client.address);
        assert_eq!(contract_balance, 900); // 1000 - 100 fee

        // Verify treasury destinations received fees
        let na_balance = token_client.balance(&na_address);
        let eu_balance = token_client.balance(&eu_address);
        assert_eq!(na_balance, 60); // 100 * 60%
        assert_eq!(eu_balance, 40); // 100 * 40%
    }

    /// Test backward compatibility: single recipient mode still works
    #[test]
    fn test_single_recipient_mode_backward_compatible() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let fee_recipient = Address::generate(&env);

        let (token_addr, token_client, token_minter) = make_token(&env, &token_admin);
        let client = make_escrow_instance(&env, &admin, &token_addr);

        // Configure fee with single recipient (no treasury distribution)
        client.update_fee_config(
            &Some(1000),
            &Some(500),
            &None,
            &None,
            &Some(fee_recipient.clone()),
            &Some(true),
        );

        // Mint tokens to depositor
        token_minter.mint(&depositor, &1000);

        // Lock funds
        let deadline = env.ledger().timestamp() + 1000;
        client.lock_funds(&depositor, &1u64, &1000i128, &deadline);

        // Verify fee went to single recipient
        let recipient_balance = token_client.balance(&fee_recipient);
        assert_eq!(recipient_balance, 100); // 10% of 1000
    }

    // ─── 3. Edge cases and invariants ─────────────────────────────────────

    /// Test with single treasury destination (100% weight)
    #[test]
    fn test_single_treasury_destination_full_weight() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let depositor = Address::generate(&env);

        let (token_addr, token_client, token_minter) = make_token(&env, &token_admin);
        let client = make_escrow_instance(&env, &admin, &token_addr);

        // Single treasury destination with 100% weight
        let treasury_address = Address::generate(&env);
        let mut destinations = Vec::new(&env);
        destinations.push_back(TreasuryDestination {
            address: treasury_address.clone(),
            weight: 10000, // 100%
            region: String::from_str(&env, "global"),
        });

        client.set_treasury_distributions(&destinations, &true);
        client.update_fee_config(&Some(1000), &None, &None, &None, &None, &Some(true));

        // Mint and lock
        token_minter.mint(&depositor, &1000);
        let deadline = env.ledger().timestamp() + 1000;
        client.lock_funds(&depositor, &1u64, &1000i128, &deadline);

        // All fee should go to single destination
        let treasury_balance = token_client.balance(&treasury_address);
        assert_eq!(treasury_balance, 100); // 10% of 1000
    }

    /// Test that fee distribution works with release fees
    #[test]
    fn test_treasury_distribution_with_release_fees() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);

        let (token_addr, token_client, token_minter) = make_token(&env, &token_admin);
        let client = make_escrow_instance(&env, &admin, &token_addr);

        // Setup treasury destinations
        let na_address = Address::generate(&env);
        let eu_address = Address::generate(&env);

        let mut destinations = Vec::new(&env);
        destinations.push_back(TreasuryDestination {
            address: na_address.clone(),
            weight: 7000, // 70%
            region: String::from_str(&env, "north_america"),
        });
        destinations.push_back(TreasuryDestination {
            address: eu_address.clone(),
            weight: 3000, // 30%
            region: String::from_str(&env, "europe"),
        });

        client.set_treasury_distributions(&destinations, &true);
        // No lock fee, but 5% release fee
        client.update_fee_config(&Some(0), &Some(500), &None, &None, &None, &Some(true));

        // Mint and lock (no lock fee)
        token_minter.mint(&depositor, &1000);
        let deadline = env.ledger().timestamp() + 1000;
        client.lock_funds(&depositor, &1u64, &1000i128, &deadline);

        // Contract has 1000
        let contract_balance = token_client.balance(&client.address);
        assert_eq!(contract_balance, 1000);

        // Release funds - 5% release fee = 50 tokens
        // 35 to NA, 15 to EU
        client.release_funds(&1u64, &contributor);

        // Contributor gets net amount
        let contributor_balance = token_client.balance(&contributor);
        assert_eq!(contributor_balance, 950); // 1000 - 50

        // Treasury destinations get release fee
        let na_balance = token_client.balance(&na_address);
        let eu_balance = token_client.balance(&eu_address);
        assert_eq!(na_balance, 35); // 50 * 70%
        assert_eq!(eu_balance, 15); // 50 * 30%
    }

    /// Test get_fee_config returns full configuration including treasury
    #[test]
    fn test_get_fee_config_includes_treasury() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);

        let (token_addr, _token_client, _token_minter) = make_token(&env, &token_admin);
        let client = make_escrow_instance(&env, &admin, &token_addr);

        // Configure treasury destinations
        let treasury_address = Address::generate(&env);
        let mut destinations = Vec::new(&env);
        destinations.push_back(TreasuryDestination {
            address: treasury_address,
            weight: 10000,
            region: String::from_str(&env, "global"),
        });

        client.set_treasury_distributions(&destinations, &true);
        client.update_fee_config(&Some(500), &Some(300), &None, &None, &None, &Some(true));

        // Verify FeeConfig includes treasury configuration
        let fee_config = client.get_fee_config();
        assert_eq!(fee_config.lock_fee_rate, 500);
        assert_eq!(fee_config.release_fee_rate, 300);
        assert!(fee_config.fee_enabled);
        assert!(fee_config.distribution_enabled);
        assert_eq!(fee_config.treasury_destinations.len(), 1);
    }

    /// Test that rounding remainder from integer division is assigned to the last destination
    /// and that total distributed strictly equals total fee.
    #[test]
    fn test_weighted_distribution_rounding_remainder_goes_to_last_destination() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let depositor = Address::generate(&env);

        let (token_addr, token_client, token_minter) = make_token(&env, &token_admin);
        let client = make_escrow_instance(&env, &admin, &token_addr);

        let dest_a = Address::generate(&env);
        let dest_b = Address::generate(&env);
        let dest_c = Address::generate(&env);

        let mut destinations = Vec::new(&env);
        destinations.push_back(TreasuryDestination {
            address: dest_a.clone(),
            weight: 1,
            region: String::from_str(&env, "north_america"),
        });
        destinations.push_back(TreasuryDestination {
            address: dest_b.clone(),
            weight: 1,
            region: String::from_str(&env, "europe"),
        });
        destinations.push_back(TreasuryDestination {
            address: dest_c.clone(),
            weight: 1,
            region: String::from_str(&env, "asia_pacific"),
        });

        client.set_treasury_distributions(&destinations, &true);
        client.update_fee_config(&Some(1000), &Some(0), &None, &None, &None, &Some(true)); // 10% lock fee

        token_minter.mint(&depositor, &1000);
        let deadline = env.ledger().timestamp() + 1000;
        client.lock_funds(&depositor, &1u64, &1000i128, &deadline);

        // Total lock fee is 100; 100 / 3 = 33 remainder 1 -> last destination absorbs remainder.
        let bal_a = token_client.balance(&dest_a);
        let bal_b = token_client.balance(&dest_b);
        let bal_c = token_client.balance(&dest_c);

        assert_eq!(bal_a, 33);
        assert_eq!(bal_b, 33);
        assert_eq!(bal_c, 34);
        assert_eq!(bal_a + bal_b + bal_c, 100);
        assert_eq!(token_client.balance(&client.address), 900);
    }
}
