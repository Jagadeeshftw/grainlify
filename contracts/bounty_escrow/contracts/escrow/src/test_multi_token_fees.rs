// test_multi_token_fees.rs
//
// Tests for Issue #386: Multi-Token Fee and Cross-Token Scenarios
//
// ARCHITECTURE NOTE:
// The bounty_escrow contract is initialized with a single token per instance
// (via `init(admin, token)`). Multi-token scenarios are modeled by deploying
// two separate contract instances — one per token. This mirrors production usage.
//
// Fee accounting note: `FeeConfig` is stored per-instance and is independent
// across contract instances. The `lock_funds` function transfers the full `amount`
// to the contract, and `release_funds` transfers the full stored `escrow.amount`
// to the contributor. Fee configuration is a per-instance setting that governs
// which rate applies when fee collection is triggered.
//
// The tests here verify:
//   1. Fee configuration is independent per contract instance.
//   2. Operations on one contract instance do NOT affect token balances of another.
//   3. Correct balance accounting per-token when operating on two instances.
//   4. Fee rate differences across instances are correctly stored and isolated.
//   5. Refunding on one instance does not affect the other token's balances.

#[cfg(test)]
mod test_multi_token_fees {
    use crate::{BountyEscrowContract, BountyEscrowContractClient, RefundMode};
    use soroban_sdk::{testutils::Address as _, token, Address, Env};

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

    // ─── 1. Fee config is independent per contract instance ─────────────────

    /// Each escrow instance stores its own FeeConfig. Setting fees on one
    /// must not change the fee config of another.
    #[test]
    fn test_fee_config_is_independent_per_contract_instance() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let fee_recipient_a = Address::generate(&env);
        let fee_recipient_b = Address::generate(&env);

        let (token_a, _ta_client, _ta_minter) = make_token(&env, &token_admin);
        let (token_b, _tb_client, _tb_minter) = make_token(&env, &token_admin);

        let client_a = make_escrow_instance(&env, &admin, &token_a);
        let client_b = make_escrow_instance(&env, &admin, &token_b);

        // Configure fee on contract_a only
        client_a.update_fee_config(
            &Some(500),
            &Some(300),
            &Some(fee_recipient_a.clone()),
            &Some(true),
        );

        // contract_b gets a different config
        client_b.update_fee_config(
            &Some(100),
            &Some(200),
            &Some(fee_recipient_b.clone()),
            &Some(false),
        );

        let config_a = client_a.get_fee_config();
        let config_b = client_b.get_fee_config();

        // Each instance has its own isolated fee configuration
        assert_eq!(
            config_a.lock_fee_rate, 500,
            "contract_a should have 5% lock fee"
        );
        assert_eq!(
            config_a.release_fee_rate, 300,
            "contract_a should have 3% release fee"
        );
        assert!(config_a.fee_enabled, "contract_a fees should be enabled");
        assert_eq!(config_a.fee_recipient, fee_recipient_a);

        assert_eq!(
            config_b.lock_fee_rate, 100,
            "contract_b should have 1% lock fee"
        );
        assert_eq!(
            config_b.release_fee_rate, 200,
            "contract_b should have 2% release fee"
        );
        assert!(!config_b.fee_enabled, "contract_b fees should be disabled");
        assert_eq!(config_b.fee_recipient, fee_recipient_b);

        // Crucially: contract_a's config is unchanged by contract_b's update
        assert_ne!(
            config_a.lock_fee_rate, config_b.lock_fee_rate,
            "fee rates should differ across instances"
        );
    }

    // ─── 2. Lock-and-release on contract_a does not touch token_b balances ──

    /// Operating on contract_a (token_a) must not move any token_b.
    #[test]
    fn test_no_cross_token_leakage_two_contracts() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let depositor_a = Address::generate(&env);
        let depositor_b = Address::generate(&env);
        let contributor_a = Address::generate(&env);

        let (token_a, _ta_client, ta_minter) = make_token(&env, &token_admin);
        let (token_b, tb_client, tb_minter) = make_token(&env, &token_admin);

        let client_a = make_escrow_instance(&env, &admin, &token_a);
        // Contract B exists but is idle
        let _client_b = make_escrow_instance(&env, &admin, &token_b);

        let amount: i128 = 10_000;
        let deadline = env.ledger().timestamp() + 1000;

        ta_minter.mint(&depositor_a, &amount);
        tb_minter.mint(&depositor_b, &amount);

        // Operate only on contract_a
        client_a.lock_funds(&depositor_a, &1, &amount, &deadline);
        client_a.release_funds(&1, &contributor_a);

        // token_b balances must be completely untouched
        assert_eq!(
            tb_client.balance(&depositor_b),
            amount,
            "depositor_b's token_b should be unchanged after contract_a operations"
        );
        assert_eq!(
            tb_client.balance(&contributor_a),
            0,
            "contributor_a should have zero token_b (no cross-token leakage)"
        );
    }

    // ─── 3. Per-token correct accounting when both contracts are active ──────

    /// Lock and release on two separate contracts independently.
    /// Each token's balances must reflect only their own contract's operations.
    #[test]
    fn test_per_token_correct_accounting_two_contracts() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let depositor_a = Address::generate(&env);
        let depositor_b = Address::generate(&env);
        let contributor_a = Address::generate(&env);
        let contributor_b = Address::generate(&env);

        let (token_a, ta_client, ta_minter) = make_token(&env, &token_admin);
        let (token_b, tb_client, tb_minter) = make_token(&env, &token_admin);

        let client_a = make_escrow_instance(&env, &admin, &token_a);
        let client_b = make_escrow_instance(&env, &admin, &token_b);

        let amount_a: i128 = 10_000;
        let amount_b: i128 = 5_000;
        let deadline = env.ledger().timestamp() + 1000;

        ta_minter.mint(&depositor_a, &amount_a);
        tb_minter.mint(&depositor_b, &amount_b);

        // Each contract operated independently
        client_a.lock_funds(&depositor_a, &1, &amount_a, &deadline);
        client_b.lock_funds(&depositor_b, &1, &amount_b, &deadline);

        client_a.release_funds(&1, &contributor_a);
        client_b.release_funds(&1, &contributor_b);

        // token_a: contributor_a gets full amount (no fee deduction in this flow)
        assert_eq!(
            ta_client.balance(&contributor_a),
            amount_a,
            "contributor_a should receive the full token_a amount"
        );
        assert_eq!(
            ta_client.balance(&depositor_a),
            0,
            "depositor_a token_a should be zero after lock"
        );

        // token_b: contributor_b gets full amount
        assert_eq!(
            tb_client.balance(&contributor_b),
            amount_b,
            "contributor_b should receive the full token_b amount"
        );
        assert_eq!(
            tb_client.balance(&depositor_b),
            0,
            "depositor_b token_b should be zero after lock"
        );

        // Verify no cross-contamination of tokens
        assert_eq!(
            ta_client.balance(&contributor_b),
            0,
            "contributor_b should have zero token_a"
        );
        assert_eq!(
            tb_client.balance(&contributor_a),
            0,
            "contributor_a should have zero token_b"
        );
    }

    // ─── 4. Fee config enabled on one instance, disabled on another ──────────

    /// Setting `fee_enabled = true` on contract_a must not affect contract_b's
    /// fee_enabled state (which defaults to false).
    #[test]
    fn test_fee_flag_enabled_on_one_contract_but_not_other() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let treasury = Address::generate(&env);

        let (token_a, _ta_client, _ta_minter) = make_token(&env, &token_admin);
        let (token_b, _tb_client, _tb_minter) = make_token(&env, &token_admin);

        let client_a = make_escrow_instance(&env, &admin, &token_a);
        let client_b = make_escrow_instance(&env, &admin, &token_b);

        // Enable fees on contract_a only
        client_a.update_fee_config(&None, &Some(500), &Some(treasury.clone()), &Some(true));

        let config_a = client_a.get_fee_config();
        let config_b = client_b.get_fee_config();

        assert!(config_a.fee_enabled, "contract_a fees should be enabled");
        assert!(
            !config_b.fee_enabled,
            "contract_b fees should remain disabled"
        );
        assert_eq!(
            config_a.release_fee_rate, 500,
            "contract_a should have 5% release fee rate"
        );
        assert_eq!(
            config_b.release_fee_rate, 0,
            "contract_b release fee rate should be zero (default)"
        );
    }

    // ─── 5. Different fee rates stored correctly per contract ─────────────────

    /// Each contract instance stores and retrieves its fee rates independently.
    #[test]
    fn test_different_fee_rates_stored_per_contract() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let treasury = Address::generate(&env);

        let (token_a, _ta_client, _ta_minter) = make_token(&env, &token_admin);
        let (token_b, _tb_client, _tb_minter) = make_token(&env, &token_admin);

        let client_a = make_escrow_instance(&env, &admin, &token_a);
        let client_b = make_escrow_instance(&env, &admin, &token_b);

        // Assign different lock fee rates to each
        client_a.update_fee_config(&Some(300), &None, &Some(treasury.clone()), &Some(true));
        client_b.update_fee_config(&Some(700), &None, &Some(treasury.clone()), &Some(true));

        let config_a = client_a.get_fee_config();
        let config_b = client_b.get_fee_config();

        assert_eq!(
            config_a.lock_fee_rate, 300,
            "contract_a should store 3% lock fee"
        );
        assert_eq!(
            config_b.lock_fee_rate, 700,
            "contract_b should store 7% lock fee"
        );

        // Verify they don't cross-contaminate
        assert_ne!(
            config_a.lock_fee_rate, config_b.lock_fee_rate,
            "each contract should independently store its own lock fee rate"
        );
    }

    // ─── 6. Release on contract_a sends token_a, not token_b ─────────────────

    /// After a lock+release cycle on contract_a, the contributor receives
    /// token_a. The contributor should have zero token_b.
    #[test]
    fn test_release_sends_correct_token_per_contract() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);

        let (token_a, ta_client, ta_minter) = make_token(&env, &token_admin);
        let (token_b, tb_client, tb_minter) = make_token(&env, &token_admin);

        let client_a = make_escrow_instance(&env, &admin, &token_a);
        // Contract B initialized but idle
        let _client_b = make_escrow_instance(&env, &admin, &token_b);
        // Mint both tokens to depositor
        ta_minter.mint(&depositor, &10_000);
        tb_minter.mint(&depositor, &10_000);

        client_a.lock_funds(&depositor, &1, &10_000, &(env.ledger().timestamp() + 1000));
        client_a.release_funds(&1, &contributor);

        assert_eq!(
            ta_client.balance(&contributor),
            10_000,
            "contributor should receive 10_000 token_a"
        );
        assert_eq!(
            tb_client.balance(&contributor),
            0,
            "contributor should have zero token_b from a contract_a release"
        );
    }

    // ─── 7. Lock amounts are tracked independently per escrow instance ────────

    /// Locking funds on contract_a stores the correct `escrow.amount`.
    /// The corresponding escrow on contract_b has its own independent amount.
    #[test]
    fn test_escrow_amounts_independent_per_contract() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let depositor_a = Address::generate(&env);
        let depositor_b = Address::generate(&env);

        let (token_a, _ta, ta_minter) = make_token(&env, &token_admin);
        let (token_b, _tb, tb_minter) = make_token(&env, &token_admin);

        let client_a = make_escrow_instance(&env, &admin, &token_a);
        let client_b = make_escrow_instance(&env, &admin, &token_b);

        let amount_a: i128 = 3_000;
        let amount_b: i128 = 7_000;
        let deadline = env.ledger().timestamp() + 1000;

        ta_minter.mint(&depositor_a, &amount_a);
        tb_minter.mint(&depositor_b, &amount_b);

        client_a.lock_funds(&depositor_a, &42, &amount_a, &deadline);
        client_b.lock_funds(&depositor_b, &42, &amount_b, &deadline);

        // Both contracts share bounty_id=42 but hold different amounts independently
        let escrow_a = client_a.get_escrow_info(&42);
        let escrow_b = client_b.get_escrow_info(&42);

        assert_eq!(escrow_a.amount, amount_a, "escrow_a should store amount_a");
        assert_eq!(escrow_b.amount, amount_b, "escrow_b should store amount_b");
        assert_ne!(
            escrow_a.amount, escrow_b.amount,
            "each contract escrow should track its own independent amount"
        );
    }

    // ─── 8. Refund on contract_a does not touch token_b balances ─────────────

    /// Approving and executing a refund on contract_a returns token_a to
    /// the depositor. token_b balances must remain completely unchanged.
    #[test]
    fn test_refund_with_fee_no_cross_leakage() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let depositor_a = Address::generate(&env);
        let depositor_b = Address::generate(&env);

        let (token_a, ta_client, ta_minter) = make_token(&env, &token_admin);
        let (token_b, tb_client, tb_minter) = make_token(&env, &token_admin);

        let client_a = make_escrow_instance(&env, &admin, &token_a);
        // Contract B holds a separate locked amount but receives no operations
        let client_b = make_escrow_instance(&env, &admin, &token_b);

        let amount_a: i128 = 10_000;
        let amount_b: i128 = 6_000;
        let deadline = env.ledger().timestamp() + 1000;

        ta_minter.mint(&depositor_a, &amount_a);
        tb_minter.mint(&depositor_b, &amount_b);

        // Lock on both contracts
        client_a.lock_funds(&depositor_a, &1, &amount_a, &deadline);
        client_b.lock_funds(&depositor_b, &1, &amount_b, &deadline);

        // Approve and execute refund only on contract_a
        client_a.approve_refund(&1, &amount_a, &depositor_a, &RefundMode::Full);
        client_a.refund(&1);

        // depositor_a should receive the full amount_a back in token_a
        assert_eq!(
            ta_client.balance(&depositor_a),
            amount_a,
            "depositor_a should get full token_a refund"
        );

        // token_b balances must be completely unchanged
        assert_eq!(
            tb_client.balance(&depositor_b),
            0,
            "depositor_b token_b should still be locked — refund on contract_a should not affect contract_b"
        );

        // Confirm contract_b's escrow is still locked (not accidentally refunded)
        let escrow_b = client_b.get_escrow_info(&1);
        assert_eq!(
            escrow_b.remaining_amount, amount_b,
            "contract_b's escrow should still have its full remaining_amount"
        );
    }
}
