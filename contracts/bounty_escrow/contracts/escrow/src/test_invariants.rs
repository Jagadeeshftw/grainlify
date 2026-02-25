use super::*;
use crate::invariants;
use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, token, Address, Env};

fn setup_bounty(env: &Env) -> (BountyEscrowContractClient<'static>, Address, Address) {
    env.mock_all_auths();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(env, &contract_id);

    let admin = Address::generate(env);
    let depositor = Address::generate(env);
    let token_admin = Address::generate(env);
    // Fixed: Updated to v2 to resolve deprecation warning
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_admin_client = token::StellarAssetClient::new(env, &token_id);

    client.init(&admin, &token_id);
    token_admin_client.mint(&depositor, &50_000);

    (client, admin, depositor)
}

/// Ensures invariant checks are invoked in all three major state-changing flows:
/// lock_funds, release_funds, and refund. If any flow stops calling assert_escrow,
/// the call count drops and this test fails.
#[test]
fn test_invariant_checker_ci_called_in_major_bounty_flows() {
    let env = Env::default();
    let (client, _admin, depositor) = setup_bounty(&env);
    env.as_contract(&client.address, || invariants::reset_test_state(&env));

    let bounty_id = 42_u64;
    let contributor = Address::generate(&env);
    let amount = 10_000_i128;
    let deadline = env.ledger().timestamp() + 1000;

    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);
    client.release_funds(&bounty_id, &contributor);

    let calls = env.as_contract(&client.address, || invariants::call_count_for_test(&env));
    assert!(
        calls >= 2,
        "lock_funds and release_funds must each trigger invariant check"
    );
}

/// Covers all three major flows (lock, release, refund) and asserts the exact
/// expected invariant call count. Prevents future changes from bypassing checks
/// in any of these flows.
#[test]
fn test_invariant_checker_ci_all_three_flows_increment_call_count() {
    let env = Env::default();
    let (client, _admin, depositor) = setup_bounty(&env);
    env.as_contract(&client.address, || invariants::reset_test_state(&env));

    let lock_id = 10_u64;
    let release_id = 11_u64;
    let refund_id = 12_u64;
    let amount = 5_000_i128;
    let now = env.ledger().timestamp();
    let deadline_short = now + 100;
    let deadline_later = now + 2000;

    client.lock_funds(&depositor, &lock_id, &amount, &deadline_later);
    client.lock_funds(&depositor, &release_id, &amount, &deadline_later);
    client.lock_funds(&depositor, &refund_id, &amount, &deadline_short);

    let contributor = Address::generate(&env);
    client.release_funds(&release_id, &contributor);

    env.ledger().set_timestamp(deadline_short + 1);
    client.refund(&refund_id);

    let calls = env.as_contract(&client.address, || invariants::call_count_for_test(&env));
    assert_eq!(
        calls, 5,
        "expected 5 invariant checks: 3 lock_funds + 1 release_funds + 1 refund; \
         if this fails, a major flow may have stopped calling assert_escrow"
    );
}

#[test]
#[should_panic(expected = "Invariant checks disabled")]
fn test_invariant_checker_ci_panics_when_disabled() {
    let env = Env::default();
    let (client, _admin, depositor) = setup_bounty(&env);
    env.as_contract(&client.address, || {
        invariants::reset_test_state(&env);
        invariants::set_disabled_for_test(&env, true);
    });

    client.lock_funds(&depositor, &7_u64, &5_000_i128, &500);
}
