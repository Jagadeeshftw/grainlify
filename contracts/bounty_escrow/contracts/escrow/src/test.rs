use super::*;
use soroban_sdk::{
    // Added Ledger as _ to put the trait in scope for set_timestamp
    testutils::{Address as _, Ledger as _},
    token,
    vec,
    Address,
    Env,
    Vec,
};

fn create_token_contract<'a>(
    e: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    // Updated to v2 to remove deprecation warning
    let contract = e.register_stellar_asset_contract_v2(admin.clone());
    let contract_address = contract.address();
    (
        token::Client::new(e, &contract_address),
        token::StellarAssetClient::new(e, &contract_address),
    )
}

fn create_escrow_contract<'a>(e: &Env) -> BountyEscrowContractClient<'a> {
    let contract_id = e.register_contract(None, BountyEscrowContract);
    BountyEscrowContractClient::new(e, &contract_id)
}

// ==================== NETWORK CONFIGURATION TESTS ====================

#[test]
fn test_network_initialization() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let chain_id = soroban_sdk::String::from_str(&env, "stellar");
    let network_id = soroban_sdk::String::from_str(&env, "testnet");

    client.init_with_network(
        &admin,
        &asset::AssetId::Stellar(token),
        &chain_id,
        &network_id,
    );

    // Verify network configuration
    let retrieved_chain = client.get_chain_id();
    let retrieved_network = client.get_network_id();

    assert_eq!(retrieved_chain, Some(chain_id));
    assert_eq!(retrieved_network, Some(network_id));
}

#[test]
fn test_network_info_getter() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let chain_id = soroban_sdk::String::from_str(&env, "ethereum");
    let network_id = soroban_sdk::String::from_str(&env, "mainnet");

    client.init_with_network(
        &admin,
        &asset::AssetId::Stellar(token),
        &chain_id,
        &network_id,
    );

    // Test tuple getter
    let (chain, network) = client.get_network_info();
    assert_eq!(chain, Some(chain_id));
    assert_eq!(network, Some(network_id));
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")] // AlreadyInitialized error
fn test_cannot_reinitialize_network_config() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);
    let token = Address::generate(&env);
    let chain_id = soroban_sdk::String::from_str(&env, "stellar");
    let network_id = soroban_sdk::String::from_str(&env, "testnet");

    // First initialization should succeed
    client.init_with_network(
        &admin1,
        &asset::AssetId::Stellar(token.clone()),
        &chain_id,
        &network_id,
    );

    // Second initialization should panic
    client.init_with_network(
        &admin2,
        &asset::AssetId::Stellar(token),
        &chain_id,
        &network_id,
    );
}

#[test]
fn test_legacy_init_still_works() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    // Legacy init should still work (without network config)
    client.init(&admin, &asset::AssetId::Stellar(token));

    // Network info should be None for legacy initialization
    assert_eq!(client.get_chain_id(), None);
    assert_eq!(client.get_network_id(), None);
    let (chain, network) = client.get_network_info();
    assert_eq!(chain, None);
    assert_eq!(network, None);
}

// ==================== END NETWORK CONFIGURATION TESTS ====================

struct TestSetup<'a> {
    env: Env,
    _admin: Address, // Prefixed with underscore to clear "never read" warning
    depositor: Address,
    contributor: Address,
    token: token::Client<'a>,
    token_admin: token::StellarAssetClient<'a>,
    escrow: BountyEscrowContractClient<'a>,
}

impl<'a> TestSetup<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);

        let (token, token_admin) = create_token_contract(&env, &admin);
        let escrow = create_escrow_contract(&env);

        escrow.init(&admin, &token.address);

        // Mint tokens to depositor
        token_admin.mint(&depositor, &1_000_000);

        Self {
            env,
            _admin: admin,
            depositor,
            contributor,
            token,
            token_admin,
            escrow,
        }
    }
}

// =============================================================================
// Existing core tests
// =============================================================================

#[test]
fn test_lock_funds_success() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    let stored_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(stored_escrow.depositor, setup.depositor);
    assert_eq!(stored_escrow.amount, amount);
    assert_eq!(stored_escrow.remaining_amount, amount);
    assert_eq!(stored_escrow.status, EscrowStatus::Locked);
    assert_eq!(stored_escrow.deadline, deadline);

    assert_eq!(setup.token.balance(&setup.escrow.address), amount);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_lock_funds_duplicate() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
}

#[test]
#[should_panic]
fn test_lock_funds_negative_amount() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = -100;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
}

#[test]
fn test_get_escrow_info() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.amount, amount);
    assert_eq!(escrow.remaining_amount, amount);
    assert_eq!(escrow.deadline, deadline);
    assert_eq!(escrow.depositor, setup.depositor);
    assert_eq!(escrow.status, EscrowStatus::Locked);
}

#[test]
fn test_release_funds_success() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    assert_eq!(setup.token.balance(&setup.escrow.address), amount);
    assert_eq!(setup.token.balance(&setup.contributor), 0);

    setup.escrow.release_funds(&bounty_id, &setup.contributor);

    let stored_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(stored_escrow.status, EscrowStatus::Released);

    assert_eq!(setup.token.balance(&setup.escrow.address), 0);
    assert_eq!(setup.token.balance(&setup.contributor), amount);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_release_funds_already_released() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.release_funds(&bounty_id, &setup.contributor);

    setup.escrow.release_funds(&bounty_id, &setup.contributor);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_release_funds_not_found() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    setup.escrow.release_funds(&bounty_id, &setup.contributor);
}

#[test]
fn test_refund_success() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.env.ledger().set_timestamp(deadline + 1);

    let initial_depositor_balance = setup.token.balance(&setup.depositor);

    setup.escrow.refund(&bounty_id);

    let stored_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(stored_escrow.status, EscrowStatus::Refunded);

    assert_eq!(setup.token.balance(&setup.escrow.address), 0);
    assert_eq!(
        setup.token.balance(&setup.depositor),
        initial_depositor_balance + amount
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn test_refund_too_early() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.escrow.refund(&bounty_id);
}

#[test]
fn test_get_balance() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 500;
    let deadline = setup.env.ledger().timestamp() + 1000;

    assert_eq!(setup.escrow.get_balance(), 0);

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    assert_eq!(setup.escrow.get_balance(), amount);
}

// =============================================================================
// Partial Payout Rounding and Small Amount Tests (Issue #354)
// =============================================================================

#[test]
fn test_partial_release_single_minimum_unit() {
    let setup = TestSetup::new();
    let bounty_id = 42;
    let amount = 1000_i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    let payout = 1_i128;
    setup
        .escrow
        .partial_release(&bounty_id, &setup.contributor, &payout);

    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.remaining_amount, amount - payout);
    assert_eq!(escrow.status, EscrowStatus::Locked);
    assert_eq!(setup.token.balance(&setup.contributor), payout);
    assert_eq!(setup.token.balance(&setup.escrow.address), amount - payout);
}

#[test]
fn test_partial_release_leaves_tiny_remainder() {
    let setup = TestSetup::new();
    let bounty_id = 43;
    let amount = 1000_i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    let payout = amount - 1;
    setup
        .escrow
        .partial_release(&bounty_id, &setup.contributor, &payout);

    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.remaining_amount, 1);
    assert!(escrow.remaining_amount >= 0);
    assert_eq!(escrow.status, EscrowStatus::Locked);
    assert_eq!(setup.token.balance(&setup.contributor), payout);
}

#[test]
fn test_partial_release_multiple_sequential_small_amounts() {
    let setup = TestSetup::new();
    let bounty_id = 44;
    let amount = 100_i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    let payout_per_step = 10_i128;
    let steps = 10_i128;

    for i in 1..=steps {
        setup
            .escrow
            .partial_release(&bounty_id, &setup.contributor, &payout_per_step);

        let escrow = setup.escrow.get_escrow_info(&bounty_id);
        let expected_remaining = amount - (payout_per_step * i);

        assert_eq!(escrow.remaining_amount, expected_remaining);
        assert!(
            escrow.remaining_amount >= 0,
            "remaining_amount went negative at step {}: {}",
            i,
            escrow.remaining_amount
        );
    }

    let final_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(final_escrow.remaining_amount, 0);
    assert_eq!(final_escrow.status, EscrowStatus::Released);
    assert_eq!(setup.token.balance(&setup.contributor), amount);
    assert_eq!(setup.token.balance(&setup.escrow.address), 0);
}

#[test]
fn test_partial_release_full_amount_in_one_shot_marks_released() {
    let setup = TestSetup::new();
    let bounty_id = 45;
    let amount = 500_i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup
        .escrow
        .partial_release(&bounty_id, &setup.contributor, &amount);

    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.remaining_amount, 0);
    assert_eq!(escrow.status, EscrowStatus::Released);
    assert_eq!(setup.token.balance(&setup.contributor), amount);
    assert_eq!(setup.token.balance(&setup.escrow.address), 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #16)")]
fn test_partial_release_overpayment_panics() {
    let setup = TestSetup::new();
    let bounty_id = 46;
    let amount = 100_i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup
        .escrow
        .partial_release(&bounty_id, &setup.contributor, &90_i128);

    setup
        .escrow
        .partial_release(&bounty_id, &setup.contributor, &11_i128);
}

#[test]
fn test_partial_release_exact_remaining_after_prior_release() {
    let setup = TestSetup::new();
    let bounty_id = 49;
    let amount = 100_i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup
        .escrow
        .partial_release(&bounty_id, &setup.contributor, &60_i128);

    let mid_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(mid_escrow.remaining_amount, 40);

    setup
        .escrow
        .partial_release(&bounty_id, &setup.contributor, &40_i128);

    let final_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(final_escrow.remaining_amount, 0);
    assert_eq!(final_escrow.status, EscrowStatus::Released);
    assert_eq!(setup.token.balance(&setup.contributor), amount);
}

#[test]
#[should_panic(expected = "Error(Contract, #13)")]
fn test_partial_release_zero_amount_rejected() {
    let setup = TestSetup::new();
    let bounty_id = 47;
    let amount = 1000_i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup
        .escrow
        .partial_release(&bounty_id, &setup.contributor, &0_i128);
}

#[test]
fn test_partial_release_remaining_amount_never_goes_negative() {
    let setup = TestSetup::new();
    let bounty_id = 48;
    let amount = 7_i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    for payout in [3_i128, 3_i128, 1_i128] {
        setup
            .escrow
            .partial_release(&bounty_id, &setup.contributor, &payout);

        let escrow = setup.escrow.get_escrow_info(&bounty_id);
        assert!(
            escrow.remaining_amount >= 0,
            "remaining_amount went negative: {}",
            escrow.remaining_amount
        );
    }

    let final_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(final_escrow.remaining_amount, 0);
    assert_eq!(final_escrow.status, EscrowStatus::Released);
    assert_eq!(setup.token.balance(&setup.contributor), amount);
    assert_eq!(setup.token.balance(&setup.escrow.address), 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_partial_release_bounty_not_found() {
    let setup = TestSetup::new();
    setup
        .escrow
        .partial_release(&999_u64, &setup.contributor, &100_i128);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_partial_release_on_already_released_bounty_panics() {
    let setup = TestSetup::new();
    let bounty_id = 50;
    let amount = 200_i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.escrow.release_funds(&bounty_id, &setup.contributor);

    setup
        .escrow
        .partial_release(&bounty_id, &setup.contributor, &1_i128);
}

#[test]
fn test_refund_after_partial_release_returns_only_remainder() {
    let setup = TestSetup::new();
    let bounty_id = 51;
    let amount = 1000_i128;
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup
        .escrow
        .partial_release(&bounty_id, &setup.contributor, &300_i128);

    setup.env.ledger().set_timestamp(deadline + 1);

    let depositor_balance_before = setup.token.balance(&setup.depositor);

    setup.escrow.refund(&bounty_id);

    let stored_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(stored_escrow.status, EscrowStatus::Refunded);

    assert_eq!(setup.token.balance(&setup.contributor), 300);
    assert_eq!(
        setup.token.balance(&setup.depositor),
        depositor_balance_before + 700
    );
    assert_eq!(setup.token.balance(&setup.escrow.address), 0);
}

#[test]
fn test_claim_within_window_transfers_funds() {
    let setup = TestSetup::new();
    let bounty_id = 100_u64;
    let amount = 1_000_i128;
    let deadline = setup.env.ledger().timestamp() + 10_000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.escrow.set_claim_window(&500_u64);

    setup
        .escrow
        .authorize_claim(&bounty_id, &setup.contributor, &DisputeReason::Other);
    let pending = setup.escrow.get_pending_claim(&bounty_id);
    assert_eq!(pending.recipient, setup.contributor);
    assert_eq!(pending.amount, amount);
    assert!(!pending.claimed);
    assert!(pending.expires_at > setup.env.ledger().timestamp());

    let before = setup.token.balance(&setup.contributor);
    setup.escrow.claim(&bounty_id);
    assert_eq!(setup.token.balance(&setup.contributor), before + amount);
    assert_eq!(setup.token.balance(&setup.escrow.address), 0);
    let escrow_info = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow_info.status, EscrowStatus::Released);
    let claim_after = setup.escrow.get_pending_claim(&bounty_id);
    assert!(claim_after.claimed);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn test_claim_after_window_expires_panics() {
    let setup = TestSetup::new();
    let bounty_id = 101_u64;
    let amount = 1_000_i128;
    let deadline = setup.env.ledger().timestamp() + 10_000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.escrow.set_claim_window(&200_u64);
    setup
        .escrow
        .authorize_claim(&bounty_id, &setup.contributor, &DisputeReason::Other);

    let now = setup.env.ledger().timestamp();
    setup.env.ledger().set_timestamp(now + 201);

    setup.escrow.claim(&bounty_id);
}

#[test]
fn test_cancel_pending_claim_restores_escrow() {
    let setup = TestSetup::new();
    let bounty_id = 102_u64;
    let amount = 2_000_i128;
    let deadline = setup.env.ledger().timestamp() + 10_000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.escrow.set_claim_window(&300_u64);
    setup
        .escrow
        .authorize_claim(&bounty_id, &setup.contributor, &DisputeReason::Other);
    let pending = setup.escrow.get_pending_claim(&bounty_id);
    assert_eq!(pending.amount, amount);
    setup
        .escrow
        .cancel_pending_claim(&bounty_id, &DisputeOutcome::CancelledByAdmin);
    let result = setup.escrow.try_get_pending_claim(&bounty_id);
    assert!(
        result.is_err(),
        "PendingClaim should be removed after cancel"
    );
    let escrow_info = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow_info.status, EscrowStatus::Locked);
    assert_eq!(setup.token.balance(&setup.escrow.address), amount);
    assert_eq!(setup.token.balance(&setup.contributor), 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_cancel_pending_claim_not_found() {
    let setup = TestSetup::new();
    setup
        .escrow
        .cancel_pending_claim(&999_u64, &DisputeOutcome::CancelledByAdmin);
}

#[test]
fn test_cancel_expired_claim_then_authorize_new_one() {
    let setup = TestSetup::new();
    let bounty_id = 103_u64;
    let amount = 1_500_i128;
    let deadline = setup.env.ledger().timestamp() + 10_000;
    let new_contributor = Address::generate(&setup.env);

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.set_claim_window(&100_u64);
    setup
        .escrow
        .authorize_claim(&bounty_id, &setup.contributor, &DisputeReason::Other);
    let now = setup.env.ledger().timestamp();
    setup.env.ledger().set_timestamp(now + 101);
    setup
        .escrow
        .cancel_pending_claim(&bounty_id, &DisputeOutcome::CancelledByAdmin);
    setup.escrow.set_claim_window(&1_000_u64);
    setup
        .escrow
        .authorize_claim(&bounty_id, &new_contributor, &DisputeReason::Other);

    let new_pending = setup.escrow.get_pending_claim(&bounty_id);
    assert_eq!(new_pending.recipient, new_contributor);
    assert!(!new_pending.claimed);

    setup.escrow.claim(&bounty_id);

    assert_eq!(setup.token.balance(&new_contributor), amount);
    assert_eq!(setup.token.balance(&setup.escrow.address), 0);

    let escrow_info = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow_info.status, EscrowStatus::Released);
}

#[test]
fn test_cancel_claim_then_use_release_funds_normally() {
    let setup = TestSetup::new();
    let bounty_id = 104_u64;
    let amount = 800_i128;
    let deadline = setup.env.ledger().timestamp() + 10_000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.escrow.set_claim_window(&300_u64);
    setup
        .escrow
        .authorize_claim(&bounty_id, &setup.contributor, &DisputeReason::Other);

    setup
        .escrow
        .cancel_pending_claim(&bounty_id, &DisputeOutcome::CancelledByAdmin);

    setup.escrow.release_funds(&bounty_id, &setup.contributor);

    assert_eq!(setup.token.balance(&setup.contributor), amount);
    let escrow_info = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow_info.status, EscrowStatus::Released);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_claim_twice_panics() {
    let setup = TestSetup::new();
    let bounty_id = 105_u64;
    let amount = 500_i128;
    let deadline = setup.env.ledger().timestamp() + 10_000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.escrow.set_claim_window(&500_u64);
    setup
        .escrow
        .authorize_claim(&bounty_id, &setup.contributor, &DisputeReason::Other);

    setup.escrow.claim(&bounty_id);

    setup.escrow.claim(&bounty_id);
}

#[test]
fn test_claim_does_not_affect_other_bounties() {
    let setup = TestSetup::new();
    let bounty_a = 106_u64;
    let bounty_b = 107_u64;
    let amount = 1_000_i128;
    let deadline = setup.env.ledger().timestamp() + 10_000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_a, &amount, &deadline);
    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_b, &amount, &deadline);

    setup.escrow.set_claim_window(&500_u64);
    setup
        .escrow
        .authorize_claim(&bounty_a, &setup.contributor, &DisputeReason::Other);

    setup.escrow.claim(&bounty_a);

    let escrow_b = setup.escrow.get_escrow_info(&bounty_b);
    assert_eq!(escrow_b.status, EscrowStatus::Locked);
    assert_eq!(escrow_b.remaining_amount, amount);

    assert_eq!(setup.token.balance(&setup.escrow.address), amount);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn test_authorize_claim_zero_window_expires_immediately() {
    let setup = TestSetup::new();
    let bounty_id = 108_u64;
    let amount = 1_000_i128;
    let deadline = setup.env.ledger().timestamp() + 10_000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup
        .escrow
        .authorize_claim(&bounty_id, &setup.contributor, &DisputeReason::Other);

    let now = setup.env.ledger().timestamp();
    setup.env.ledger().set_timestamp(now + 1);

    setup.escrow.claim(&bounty_id);
}

#[test]
fn test_claim_at_exact_window_boundary_succeeds() {
    let setup = TestSetup::new();
    let bounty_id = 109_u64;
    let amount = 1_000_i128;
    let now = setup.env.ledger().timestamp();
    let deadline = now + 10_000;
    let window = 300_u64;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.escrow.set_claim_window(&window);
    setup
        .escrow
        .authorize_claim(&bounty_id, &setup.contributor, &DisputeReason::Other);

    let pending = setup.escrow.get_pending_claim(&bounty_id);
    setup.env.ledger().set_timestamp(pending.expires_at);
    setup.escrow.claim(&bounty_id);

    assert_eq!(setup.token.balance(&setup.contributor), amount);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_authorize_claim_on_nonexistent_bounty() {
    let setup = TestSetup::new();
    setup
        .escrow
        .authorize_claim(&999_u64, &setup.contributor, &DisputeReason::Other);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_authorize_claim_on_released_bounty() {
    let setup = TestSetup::new();
    let bounty_id = 110_u64;
    let amount = 1_000_i128;
    let deadline = setup.env.ledger().timestamp() + 10_000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.release_funds(&bounty_id, &setup.contributor);
    setup
        .escrow
        .authorize_claim(&bounty_id, &setup.contributor, &DisputeReason::Other);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_authorize_claim_on_refunded_bounty() {
    let setup = TestSetup::new();
    let bounty_id = 111_u64;
    let amount = 1_000_i128;
    let now = setup.env.ledger().timestamp();
    let deadline = now + 500;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.env.ledger().set_timestamp(deadline + 1);
    setup.escrow.refund(&bounty_id);
    setup
        .escrow
        .authorize_claim(&bounty_id, &setup.contributor, &DisputeReason::Other);
}

#[test]
fn test_authorize_claim_default_window_used_when_not_set() {
    let setup = TestSetup::new();
    let bounty_id = 112_u64;
    let amount = 1_000_i128;
    let deadline = setup.env.ledger().timestamp() + 10_000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    let auth_time = setup.env.ledger().timestamp();
    setup
        .escrow
        .authorize_claim(&bounty_id, &setup.contributor, &DisputeReason::Other);

    let pending = setup.escrow.get_pending_claim(&bounty_id);
    assert_eq!(pending.expires_at, auth_time);
}

#[test]
fn test_set_claim_window_success() {
    let setup = TestSetup::new();
    let bounty_id = 113_u64;
    let amount = 1_000_i128;
    let deadline = setup.env.ledger().timestamp() + 10_000;
    let window = 600_u64;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.escrow.set_claim_window(&window);

    let auth_time = setup.env.ledger().timestamp();
    setup
        .escrow
        .authorize_claim(&bounty_id, &setup.contributor, &DisputeReason::Other);

    let pending = setup.escrow.get_pending_claim(&bounty_id);
    assert_eq!(pending.expires_at, auth_time + window);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_get_pending_claim_not_found() {
    let setup = TestSetup::new();
    setup.escrow.get_pending_claim(&999_u64);
}

#[test]
fn test_authorize_claim_creates_pending_claim() {
    let setup = TestSetup::new();
    let bounty_id = 114_u64;
    let amount = 3_000_i128;
    let deadline = setup.env.ledger().timestamp() + 10_000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.escrow.set_claim_window(&400_u64);
    setup
        .escrow
        .authorize_claim(&bounty_id, &setup.contributor, &DisputeReason::Other);

    let pending = setup.escrow.get_pending_claim(&bounty_id);
    assert_eq!(pending.bounty_id, bounty_id);
    assert_eq!(pending.amount, amount);
    assert_eq!(pending.recipient, setup.contributor);
    assert!(!pending.claimed);
}

// ============================================================================
// BATCH LOCK AND RELEASE FAILURE MODE TESTS
// ============================================================================

#[test]
fn test_batch_lock_funds_success() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    let items = vec![
        &setup.env,
        LockFundsItem {
            bounty_id: 1,
            depositor: setup.depositor.clone(),
            amount: 1000,
            deadline,
        },
        LockFundsItem {
            bounty_id: 2,
            depositor: setup.depositor.clone(),
            amount: 2000,
            deadline,
        },
        LockFundsItem {
            bounty_id: 3,
            depositor: setup.depositor.clone(),
            amount: 3000,
            deadline,
        },
    ];

    setup.token_admin.mint(&setup.depositor, &10_000);

    let count = setup.escrow.batch_lock_funds(&items);
    assert_eq!(count, 3);

    for i in 1..=3 {
        let escrow = setup.escrow.get_escrow_info(&i);
        assert_eq!(escrow.status, EscrowStatus::Locked);
    }

    assert_eq!(setup.escrow.get_balance(), 6000);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_batch_lock_funds_empty() {
    let setup = TestSetup::new();
    let items: Vec<LockFundsItem> = Vec::new(&setup.env);
    setup.escrow.batch_lock_funds(&items);
}

#[test]
fn test_batch_lock_funds_single_item() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    let items = vec![
        &setup.env,
        LockFundsItem {
            bounty_id: 1,
            depositor: setup.depositor.clone(),
            amount: 1000,
            deadline,
        },
    ];

    setup.token_admin.mint(&setup.depositor, &1000);
    let count = setup.escrow.batch_lock_funds(&items);
    assert_eq!(count, 1);

    let escrow = setup.escrow.get_escrow_info(&1);
    assert_eq!(escrow.status, EscrowStatus::Locked);
    assert_eq!(escrow.amount, 1000);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_batch_lock_funds_exceeds_max_batch_size() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    let mut items = Vec::new(&setup.env);
    for i in 1..=21 {
        items.push_back(LockFundsItem {
            bounty_id: i,
            depositor: setup.depositor.clone(),
            amount: 100,
            deadline,
        });
    }

    setup.token_admin.mint(&setup.depositor, &10_000);
    setup.escrow.batch_lock_funds(&items);
}

#[test]
fn test_batch_lock_funds_at_max_batch_size() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    let mut items = Vec::new(&setup.env);
    for i in 1..=20 {
        items.push_back(LockFundsItem {
            bounty_id: i,
            depositor: setup.depositor.clone(),
            amount: 100,
            deadline,
        });
    }

    setup.token_admin.mint(&setup.depositor, &10_000);
    let count = setup.escrow.batch_lock_funds(&items);
    assert_eq!(count, 20);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_batch_lock_funds_duplicate_bounty_id() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &1, &1000, &deadline);

    let items = vec![
        &setup.env,
        LockFundsItem {
            bounty_id: 1,
            depositor: setup.depositor.clone(),
            amount: 2000,
            deadline,
        },
        LockFundsItem {
            bounty_id: 2,
            depositor: setup.depositor.clone(),
            amount: 3000,
            deadline,
        },
    ];

    setup.escrow.batch_lock_funds(&items);
}

#[test]
#[should_panic(expected = "Error(Contract, #12)")]
fn test_batch_lock_funds_duplicate_in_batch() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    let items = vec![
        &setup.env,
        LockFundsItem {
            bounty_id: 1,
            depositor: setup.depositor.clone(),
            amount: 1000,
            deadline,
        },
        LockFundsItem {
            bounty_id: 1,
            depositor: setup.depositor.clone(),
            amount: 2000,
            deadline,
        },
    ];

    setup.escrow.batch_lock_funds(&items);
}

#[test]
#[should_panic(expected = "Error(Contract, #12)")]
fn test_batch_lock_funds_triple_duplicate_in_batch() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    let items = vec![
        &setup.env,
        LockFundsItem {
            bounty_id: 1,
            depositor: setup.depositor.clone(),
            amount: 1000,
            deadline,
        },
        LockFundsItem {
            bounty_id: 1,
            depositor: setup.depositor.clone(),
            amount: 2000,
            deadline,
        },
        LockFundsItem {
            bounty_id: 1,
            depositor: setup.depositor.clone(),
            amount: 3000,
            deadline,
        },
    ];

    setup.token_admin.mint(&setup.depositor, &10000);
    setup.escrow.batch_lock_funds(&items);
}

#[test]
#[should_panic(expected = "Error(Contract, #12)")]
fn test_batch_lock_funds_non_adjacent_duplicates() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    let items = vec![
        &setup.env,
        LockFundsItem {
            bounty_id: 1,
            depositor: setup.depositor.clone(),
            amount: 1000,
            deadline,
        },
        LockFundsItem {
            bounty_id: 2,
            depositor: setup.depositor.clone(),
            amount: 2000,
            deadline,
        },
        LockFundsItem {
            bounty_id: 1,
            depositor: setup.depositor.clone(),
            amount: 3000,
            deadline,
        },
    ];

    setup.token_admin.mint(&setup.depositor, &10000);
    setup.escrow.batch_lock_funds(&items);
}

#[test]
#[should_panic(expected = "Error(Contract, #13)")]
fn test_batch_lock_funds_zero_amount() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    let items = vec![
        &setup.env,
        LockFundsItem {
            bounty_id: 1,
            depositor: setup.depositor.clone(),
            amount: 0,
            deadline,
        },
    ];

    setup.escrow.batch_lock_funds(&items);
}

#[test]
#[should_panic(expected = "Error(Contract, #13)")]
fn test_batch_lock_funds_negative_amount() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    let items = vec![
        &setup.env,
        LockFundsItem {
            bounty_id: 1,
            depositor: setup.depositor.clone(),
            amount: -100,
            deadline,
        },
    ];

    setup.escrow.batch_lock_funds(&items);
}

#[test]
#[should_panic(expected = "Error(Contract, #13)")]
fn test_batch_lock_funds_mixed_valid_invalid_amounts() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    let items = vec![
        &setup.env,
        LockFundsItem {
            bounty_id: 1,
            depositor: setup.depositor.clone(),
            amount: 1000,
            deadline,
        },
        LockFundsItem {
            bounty_id: 2,
            depositor: setup.depositor.clone(),
            amount: 0,
            deadline,
        },
    ];

    setup.token_admin.mint(&setup.depositor, &2000);
    setup.escrow.batch_lock_funds(&items);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_batch_lock_funds_first_valid_second_exists() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &2, &1000, &deadline);

    let items = vec![
        &setup.env,
        LockFundsItem {
            bounty_id: 1,
            depositor: setup.depositor.clone(),
            amount: 1000,
            deadline,
        },
        LockFundsItem {
            bounty_id: 2,
            depositor: setup.depositor.clone(),
            amount: 2000,
            deadline,
        },
    ];

    setup.token_admin.mint(&setup.depositor, &5000);
    setup.escrow.batch_lock_funds(&items);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_batch_operations_atomicity() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &1, &1000, &deadline);

    let items = vec![
        &setup.env,
        LockFundsItem {
            bounty_id: 2,
            depositor: setup.depositor.clone(),
            amount: 2000,
            deadline,
        },
        LockFundsItem {
            bounty_id: 1,
            depositor: setup.depositor.clone(),
            amount: 3000,
            deadline,
        },
    ];

    setup.escrow.batch_lock_funds(&items);
}

#[test]
fn test_batch_release_funds_success() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &1, &1000, &deadline);
    setup
        .escrow
        .lock_funds(&setup.depositor, &2, &2000, &deadline);
    setup
        .escrow
        .lock_funds(&setup.depositor, &3, &3000, &deadline);

    let contributor1 = Address::generate(&setup.env);
    let contributor2 = Address::generate(&setup.env);
    let contributor3 = Address::generate(&setup.env);

    let items = vec![
        &setup.env,
        ReleaseFundsItem {
            bounty_id: 1,
            contributor: contributor1.clone(),
        },
        ReleaseFundsItem {
            bounty_id: 2,
            contributor: contributor2.clone(),
        },
        ReleaseFundsItem {
            bounty_id: 3,
            contributor: contributor3.clone(),
        },
    ];

    let count = setup.escrow.batch_release_funds(&items);
    assert_eq!(count, 3);

    for i in 1..=3 {
        let escrow = setup.escrow.get_escrow_info(&i);
        assert_eq!(escrow.status, EscrowStatus::Released);
    }

    assert_eq!(setup.token.balance(&contributor1), 1000);
    assert_eq!(setup.token.balance(&contributor2), 2000);
    assert_eq!(setup.token.balance(&contributor3), 3000);
    assert_eq!(setup.escrow.get_balance(), 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_batch_release_funds_empty() {
    let setup = TestSetup::new();
    let items: Vec<ReleaseFundsItem> = Vec::new(&setup.env);
    setup.escrow.batch_release_funds(&items);
}

#[test]
fn test_batch_release_funds_single_item() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &1, &1000, &deadline);

    let contributor = Address::generate(&setup.env);
    let items = vec![
        &setup.env,
        ReleaseFundsItem {
            bounty_id: 1,
            contributor: contributor.clone(),
        },
    ];

    let count = setup.escrow.batch_release_funds(&items);
    assert_eq!(count, 1);

    let escrow = setup.escrow.get_escrow_info(&1);
    assert_eq!(escrow.status, EscrowStatus::Released);
    assert_eq!(setup.token.balance(&contributor), 1000);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_batch_release_funds_exceeds_max_batch_size() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    let mut lock_items = Vec::new(&setup.env);
    for i in 1..=20 {
        lock_items.push_back(LockFundsItem {
            bounty_id: i,
            depositor: setup.depositor.clone(),
            amount: 100,
            deadline,
        });
    }
    setup.token_admin.mint(&setup.depositor, &10_000);
    setup.escrow.batch_lock_funds(&lock_items);

    let mut release_items = Vec::new(&setup.env);
    for i in 1..=21 {
        release_items.push_back(ReleaseFundsItem {
            bounty_id: i,
            contributor: Address::generate(&setup.env),
        });
    }

    setup.escrow.batch_release_funds(&release_items);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_batch_release_funds_not_found() {
    let setup = TestSetup::new();
    let contributor = Address::generate(&setup.env);

    let items = vec![
        &setup.env,
        ReleaseFundsItem {
            bounty_id: 999,
            contributor: contributor.clone(),
        },
    ];

    setup.escrow.batch_release_funds(&items);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_batch_release_funds_already_released() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &1, &1000, &deadline);
    setup.escrow.release_funds(&1, &setup.contributor);

    setup
        .escrow
        .lock_funds(&setup.depositor, &2, &2000, &deadline);

    let contributor2 = Address::generate(&setup.env);

    let items = vec![
        &setup.env,
        ReleaseFundsItem {
            bounty_id: 1,
            contributor: setup.contributor.clone(),
        },
        ReleaseFundsItem {
            bounty_id: 2,
            contributor: contributor2.clone(),
        },
    ];

    setup.escrow.batch_release_funds(&items);
}

#[test]
#[should_panic(expected = "Error(Contract, #12)")]
fn test_batch_release_funds_duplicate_in_batch() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &1, &1000, &deadline);

    let contributor = Address::generate(&setup.env);

    let items = vec![
        &setup.env,
        ReleaseFundsItem {
            bounty_id: 1,
            contributor: contributor.clone(),
        },
        ReleaseFundsItem {
            bounty_id: 1,
            contributor: contributor.clone(),
        },
    ];

    setup.escrow.batch_release_funds(&items);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_batch_release_funds_first_valid_second_not_found() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &1, &1000, &deadline);

    let contributor = Address::generate(&setup.env);
    let items = vec![
        &setup.env,
        ReleaseFundsItem {
            bounty_id: 1,
            contributor: contributor.clone(),
        },
        ReleaseFundsItem {
            bounty_id: 999,
            contributor: contributor.clone(),
        },
    ];

    setup.escrow.batch_release_funds(&items);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_batch_release_funds_mixed_locked_and_refunded() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 100;

    setup
        .escrow
        .lock_funds(&setup.depositor, &1, &1000, &deadline);
    setup
        .escrow
        .lock_funds(&setup.depositor, &2, &2000, &deadline);

    setup.env.ledger().set_timestamp(deadline + 1);
    setup.escrow.refund(&2);

    let contributor = Address::generate(&setup.env);
    let items = vec![
        &setup.env,
        ReleaseFundsItem {
            bounty_id: 1,
            contributor: contributor.clone(),
        },
        ReleaseFundsItem {
            bounty_id: 2,
            contributor: contributor.clone(),
        },
    ];

    setup.escrow.batch_release_funds(&items);
}

#[test]
#[should_panic(expected = "Error(Contract, #12)")]
fn test_batch_release_funds_non_adjacent_duplicates() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &1, &1000, &deadline);
    setup
        .escrow
        .lock_funds(&setup.depositor, &2, &2000, &deadline);

    let contributor = Address::generate(&setup.env);
    let items = vec![
        &setup.env,
        ReleaseFundsItem {
            bounty_id: 1,
            contributor: contributor.clone(),
        },
        ReleaseFundsItem {
            bounty_id: 2,
            contributor: contributor.clone(),
        },
        ReleaseFundsItem {
            bounty_id: 1,
            contributor: contributor.clone(),
        },
    ];

    setup.escrow.batch_release_funds(&items);
}

#[test]
fn test_batch_operations_large_batch() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    let mut items = Vec::new(&setup.env);
    for i in 1..=10 {
        items.push_back(LockFundsItem {
            bounty_id: i,
            depositor: setup.depositor.clone(),
            amount: (i * 100) as i128,
            deadline,
        });
    }

    setup.token_admin.mint(&setup.depositor, &10_000);

    let count = setup.escrow.batch_lock_funds(&items);
    assert_eq!(count, 10);

    for i in 1..=10 {
        let escrow = setup.escrow.get_escrow_info(&i);
        assert_eq!(escrow.status, EscrowStatus::Locked);
    }

    let mut release_items = Vec::new(&setup.env);
    for i in 1..=10 {
        release_items.push_back(ReleaseFundsItem {
            bounty_id: i,
            contributor: Address::generate(&setup.env),
        });
    }

    let release_count = setup.escrow.batch_release_funds(&release_items);
    assert_eq!(release_count, 10);
}

#[test]
fn test_batch_operations_multiple_depositors() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    let depositor2 = Address::generate(&setup.env);

    let initial_depositor_balance = setup.token.balance(&setup.depositor);

    setup.token_admin.mint(&depositor2, &5000);

    let items = vec![
        &setup.env,
        LockFundsItem {
            bounty_id: 1,
            depositor: setup.depositor.clone(),
            amount: 1000,
            deadline,
        },
        LockFundsItem {
            bounty_id: 2,
            depositor: depositor2.clone(),
            amount: 2000,
            deadline,
        },
        LockFundsItem {
            bounty_id: 3,
            depositor: setup.depositor.clone(),
            amount: 3000,
            deadline,
        },
    ];

    let count = setup.escrow.batch_lock_funds(&items);
    assert_eq!(count, 3);

    let escrow1 = setup.escrow.get_escrow_info(&1);
    let escrow2 = setup.escrow.get_escrow_info(&2);
    let escrow3 = setup.escrow.get_escrow_info(&3);

    assert_eq!(escrow1.depositor, setup.depositor);
    assert_eq!(escrow2.depositor, depositor2);
    assert_eq!(escrow3.depositor, setup.depositor);

    assert_eq!(
        setup.token.balance(&setup.depositor),
        initial_depositor_balance - 4000
    );
    assert_eq!(setup.token.balance(&depositor2), 3000);
    assert_eq!(setup.escrow.get_balance(), 6000);
}

#[test]
fn test_batch_release_funds_to_multiple_contributors() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &1, &1000, &deadline);
    setup
        .escrow
        .lock_funds(&setup.depositor, &2, &2000, &deadline);
    setup
        .escrow
        .lock_funds(&setup.depositor, &3, &3000, &deadline);

    let contributor1 = Address::generate(&setup.env);
    let contributor2 = Address::generate(&setup.env);
    let contributor3 = Address::generate(&setup.env);

    let items = vec![
        &setup.env,
        ReleaseFundsItem {
            bounty_id: 1,
            contributor: contributor1.clone(),
        },
        ReleaseFundsItem {
            bounty_id: 2,
            contributor: contributor2.clone(),
        },
        ReleaseFundsItem {
            bounty_id: 3,
            contributor: contributor3.clone(),
        },
    ];

    let count = setup.escrow.batch_release_funds(&items);
    assert_eq!(count, 3);

    assert_eq!(setup.token.balance(&contributor1), 1000);
    assert_eq!(setup.token.balance(&contributor2), 2000);
    assert_eq!(setup.token.balance(&contributor3), 3000);
    assert_eq!(setup.escrow.get_balance(), 0);
}
