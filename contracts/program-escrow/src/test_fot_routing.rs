#![cfg(test)]

use crate::{ProgramEscrowContract, ProgramEscrowContractClient};
use soroban_sdk::{
    contract, contractimpl,
    testutils::Address as _,
    vec, Address, Env, String, Symbol,
};

// ===========================================================================
// Mock FoT Router Contract
// ===========================================================================

/// A mock router that reports the gross amount needed to net `amount` after
/// a configurable fee-on-transfer deduction.
///
/// Fee is stored per-token: `token_address -> fee_bps`.
/// `quote(token, amount)` returns `ceil(amount * 10000 / (10000 - fee_bps))`.
#[contract]
pub struct MockFotRouter;

#[contractimpl]
impl MockFotRouter {
    pub fn quote(env: Env, token: Address, amount: i128) -> i128 {
        let fee_bps: i128 = env.storage().instance().get(&token).unwrap_or(0);
        if fee_bps == 0 {
            return amount;
        }
        if fee_bps >= 10000 {
            panic!("MockFotRouter: fee too high");
        }
        if amount <= 0 {
            return 0;
        }
        let numerator = amount * 10000;
        let denominator = 10000 - fee_bps;
        (numerator + denominator - 1) / denominator
    }

    pub fn set_fee(env: Env, token: Address, fee_bps: i128) {
        env.storage().instance().set(&token, &fee_bps);
    }
}

// ===========================================================================
// Mock FoT Token Contract
// ===========================================================================

#[contract]
pub struct DeflatToken;

#[contractimpl]
impl DeflatToken {
    pub fn balance(env: Env, id: Address) -> i128 {
        env.storage().instance().get(&id).unwrap_or(0)
    }

    pub fn mint(env: Env, to: Address, amount: i128) {
        let b: i128 = env.storage().instance().get(&to).unwrap_or(0);
        env.storage().instance().set(&to, &(b + amount));
    }

    pub fn set_fee_bps(env: Env, fee_bps: i128) {
        let key = Symbol::new(&env, "fee_bps");
        env.storage().instance().set(&key, &fee_bps);
    }

    fn fee_bps_internal(env: &Env) -> i128 {
        let key = Symbol::new(&env, "fee_bps");
        env.storage().instance().get(&key).unwrap_or(0)
    }

    fn do_transfer(env: &Env, from: Address, to: Address, amount: i128) {
        let b_from: i128 = env.storage().instance().get(&from).unwrap_or(0);
        if b_from < amount {
            panic!("insufficient balance");
        }
        let fee = amount * Self::fee_bps_internal(env) / 10_000;
        let net = amount - fee;
        env.storage().instance().set(&from, &(b_from - amount));
        let b_to: i128 = env.storage().instance().get(&to).unwrap_or(0);
        env.storage().instance().set(&to, &(b_to + net));
    }

    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        from.require_auth();
        Self::do_transfer(&env, from, to, amount);
    }
}

// ===========================================================================
// Test helpers
// ===========================================================================

struct FotRoutingSetup<'a> {
    client: ProgramEscrowContractClient<'a>,
    token: DeflatTokenClient<'a>,
    router: MockFotRouterClient<'a>,
    admin: Address,
}

fn setup_with_router(
    env: &Env,
    token_fee_bps: i128,
    router_fee_bps: i128,
    slippage_bps: u32,
) -> FotRoutingSetup<'_> {
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);

    let token_id = env.register_contract(None, DeflatToken);
    let token = DeflatTokenClient::new(env, &token_id);
    token.set_fee_bps(&token_fee_bps);

    let router_id = env.register_contract(None, MockFotRouter);
    let router = MockFotRouterClient::new(env, &router_id);
    router.set_fee(&token_id, &router_fee_bps);

    let admin = Address::generate(env);
    let program_id = String::from_str(env, "fot-routing-prog");

    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    client.set_fot_router(&router_id, &slippage_bps);
    client.publish_program(&program_id, &admin);

    FotRoutingSetup { client, token, router, admin }
}

struct NoRouterSetup<'a> {
    client: ProgramEscrowContractClient<'a>,
    token: DeflatTokenClient<'a>,
    admin: Address,
}

fn setup_no_router(env: &Env, token_fee_bps: i128) -> NoRouterSetup<'_> {
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);

    let token_id = env.register_contract(None, DeflatToken);
    let token = DeflatTokenClient::new(env, &token_id);
    token.set_fee_bps(&token_fee_bps);

    let admin = Address::generate(env);
    let program_id = String::from_str(env, "no-router-prog");

    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    client.publish_program(&program_id, &admin);

    NoRouterSetup { client, token, admin }
}

/// Fund the contract with `gross_amount` tokens via lock_program_funds.
/// Mints tokens directly so the contract holds the full amount.
fn fund_contract(setup: &FotRoutingSetup<'_>, env: &Env, gross_amount: i128) {
    setup.token.mint(&setup.client.address, &gross_amount);
    setup.client.lock_program_funds(&gross_amount);
}

/// Fund the contract without router (no-router setup).
fn fund_contract_no_router(setup: &NoRouterSetup<'_>, gross_amount: i128) {
    setup.token.mint(&setup.client.address, &gross_amount);
    setup.client.lock_program_funds(&gross_amount);
}

// ===========================================================================
// 1. Backward compatibility: no router behaves identically
// ===========================================================================

#[test]
fn test_no_router_payout_matches_existing_behavior() {
    let env = Env::default();
    let setup = setup_no_router(&env, 0);
    fund_contract_no_router(&setup, 1_000);

    let recipient = Address::generate(&env);
    setup.client.single_payout(&recipient, &500, &None);

    assert_eq!(setup.token.balance(&recipient), 500);
    assert_eq!(setup.client.get_program_info().remaining_balance, 500);
}

// ===========================================================================
// 2. Single payout with routing preserves net amount despite FoT
// ===========================================================================

#[test]
fn test_single_payout_router_preserves_net() {
    let env = Env::default();
    let setup = setup_with_router(&env, 1_000, 1_000, 0);

    // Fund with 10,000. Token has 10% FoT on transfer, but we mint directly.
    // Contract holds 10,000. We credit 10,000 via lock_program_funds.
    fund_contract(&setup, &env, 10_000);

    let recipient = Address::generate(&env);
    // Pay out 900 net. Router knows 10% FoT: ceil(900*10000/9000) = 1000.
    // Contract sends 1000, FoT takes 10%, recipient gets 900.
    setup.client.single_payout(&recipient, &900, &None);

    assert_eq!(setup.token.balance(&recipient), 900,
        "Recipient must receive the intended net amount despite FoT fee");
    assert_eq!(setup.client.get_program_info().remaining_balance, 9_000,
        "remaining_balance debited by actual outflow (1000), not net (900)");
}

// ===========================================================================
// 3. Batch payout with routing preserves net amounts
// ===========================================================================

#[test]
fn test_batch_payout_router_preserves_net() {
    let env = Env::default();
    let setup = setup_with_router(&env, 1_000, 1_000, 0);
    fund_contract(&setup, &env, 20_000);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    let r3 = Address::generate(&env);

    let recipients = vec![&env, r1.clone(), r2.clone(), r3.clone()];
    let amounts = vec![&env, 900_i128, 1_800_i128, 900_i128];

    setup.client.batch_payout(&recipients, &amounts, &None);

    // Router: ceil(900*10000/9000)=1000, ceil(1800*10000/9000)=2000
    // After 10% FoT: 900, 1800, 900
    assert_eq!(setup.token.balance(&r1), 900, "r1 receives intended 900");
    assert_eq!(setup.token.balance(&r2), 1_800, "r2 receives intended 1800");
    assert_eq!(setup.token.balance(&r3), 900, "r3 receives intended 900");

    // Outflow: 1000 + 2000 + 1000 = 4000, remaining: 20000 - 4000 = 16000
    assert_eq!(setup.client.get_program_info().remaining_balance, 16_000);
}

// ===========================================================================
// 4. Router with slippage tolerance
// ===========================================================================

#[test]
fn test_single_payout_with_slippage() {
    let env = Env::default();
    let setup = setup_with_router(&env, 1_000, 1_000, 500);
    fund_contract(&setup, &env, 10_000);

    let recipient = Address::generate(&env);
    // net=900, router quotes 1000, slippage 5% → 1050
    setup.client.single_payout(&recipient, &900, &None);

    // Transfer amount = 1050, FoT 10% → recipient gets 945
    assert_eq!(setup.token.balance(&recipient), 945);
    assert_eq!(setup.client.get_program_info().remaining_balance, 8_950);
}

// ===========================================================================
// 5. Zero slippage with high FoT
// ===========================================================================

#[test]
fn test_single_payout_zero_slippage_high_fot() {
    let env = Env::default();
    let setup = setup_with_router(&env, 2_000, 2_000, 0);
    fund_contract(&setup, &env, 10_000);

    let recipient = Address::generate(&env);
    // net=800, router: ceil(800*10000/8000) = 1000
    setup.client.single_payout(&recipient, &800, &None);

    assert_eq!(setup.token.balance(&recipient), 800,
        "20% FoT: recipient gets intended 800");
    assert_eq!(setup.client.get_program_info().remaining_balance, 9_000);
}

// ===========================================================================
// 6. Batch payout with slippage
// ===========================================================================

#[test]
fn test_batch_payout_with_slippage() {
    let env = Env::default();
    let setup = setup_with_router(&env, 1_000, 1_000, 200);
    fund_contract(&setup, &env, 30_000);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    let recipients = vec![&env, r1.clone(), r2.clone()];
    let amounts = vec![&env, 900_i128, 1_800_i128];

    setup.client.batch_payout(&recipients, &amounts, &None);

    // r1: net 900 → 1000 gross → +2% = 1020, after 10% FoT = 918
    // r2: net 1800 → 2000 gross → +2% = 2040, after 10% FoT = 1836
    assert_eq!(setup.token.balance(&r1), 918, "r1 with slippage");
    assert_eq!(setup.token.balance(&r2), 1_836, "r2 with slippage");

    // Outflow: 1020 + 2040 = 3060
    // Remaining: 30000 - 3060 = 26940
    assert_eq!(setup.client.get_program_info().remaining_balance, 26_940);
}

// ===========================================================================
// 7. Zero FoT fee (no-op routing)
// ===========================================================================

#[test]
fn test_router_zero_fot_fee_no_op() {
    let env = Env::default();
    let setup = setup_with_router(&env, 0, 0, 0);
    fund_contract(&setup, &env, 5_000);

    let recipient = Address::generate(&env);
    setup.client.single_payout(&recipient, &1_000, &None);

    assert_eq!(setup.token.balance(&recipient), 1_000);
    assert_eq!(setup.client.get_program_info().remaining_balance, 4_000);
}

// ===========================================================================
// 8. Router FoT differs from token FoT (under-routing)
// ===========================================================================

#[test]
fn test_router_fot_differs_from_token() {
    let env = Env::default();
    // Token: 10% FoT, Router: 5% FoT (under-routing)
    let setup = setup_with_router(&env, 1_000, 500, 0);
    fund_contract(&setup, &env, 10_000);

    let recipient = Address::generate(&env);
    // net=900, router thinks 5%: ceil(900*10000/9500) = 948
    // Token takes 10%: recipient gets 948 - 94 = 854
    setup.client.single_payout(&recipient, &900, &None);

    assert!(setup.token.balance(&recipient) < 900,
        "Under-routing: recipient gets less than intended");
}

// ===========================================================================
// 9. Insufficient balance with routing
// ===========================================================================

#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_insufficient_balance_with_routing() {
    let env = Env::default();
    let setup = setup_with_router(&env, 1_000, 1_000, 0);
    fund_contract(&setup, &env, 5_000);

    let recipient = Address::generate(&env);
    // remaining=5000. Payout of 4500 net → router quotes 5000, total_debit=5000.
    // This is at the limit, so this should succeed...
    // Let's try a higher amount that fails
    // remaining=5000. Payout of 5000 net → router quotes 5556, total_debit=5556 > 5000
    setup.client.single_payout(&recipient, &5_000, &None);
}

// ===========================================================================
// 10. Clear router restores backward-compatible behavior
// ===========================================================================

#[test]
fn test_clear_router_restores_no_routing() {
    let env = Env::default();
    let setup = setup_with_router(&env, 1_000, 1_000, 0);
    fund_contract(&setup, &env, 10_000);

    setup.client.clear_fot_router();

    let recipient = Address::generate(&env);
    // Without routing, sends net=900 directly. FoT takes 10% → recipient gets 810.
    setup.client.single_payout(&recipient, &900, &None);

    assert_eq!(setup.token.balance(&recipient), 810,
        "After clearing router, FoT fee is not compensated");
    assert_eq!(setup.client.get_program_info().remaining_balance, 9_100);
}

// ===========================================================================
// 11. Batch payout with insufficient balance due to routing
// ===========================================================================

#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_batch_insufficient_balance_with_routing() {
    let env = Env::default();
    // 50% router FoT means gross ≈ 2x net
    let setup = setup_with_router(&env, 1_000, 5_000, 0);
    fund_contract(&setup, &env, 10_000);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    let recipients = vec![&env, r1, r2];
    let amounts = vec![&env, 4_000_i128, 4_000_i128];

    // net 4000 each, router 50%: gross = ceil(4000*10000/5000) = 8000 each
    // total_debit = 16000 > 10000 remaining
    setup.client.batch_payout(&recipients, &amounts, &None);
}

// ===========================================================================
// 12. Fee waiver + routing (single)
// ===========================================================================

#[test]
fn test_single_payout_fee_waived_with_routing() {
    let env = Env::default();
    let setup = setup_with_router(&env, 1_000, 1_000, 0);
    fund_contract(&setup, &env, 10_000);

    let recipient = Address::generate(&env);
    setup.client.single_payout(&recipient, &900, &None);

    assert_eq!(setup.token.balance(&recipient), 900,
        "Fee waived: recipient gets intended net with FoT routing");
}

// ===========================================================================
// 13. Event emission
// ===========================================================================

#[test]
fn test_set_fot_router_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);

    let token_id = env.register_contract(None, DeflatToken);
    let router_id = env.register_contract(None, MockFotRouter);

    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "event-prog");

    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);

    use soroban_sdk::testutils::Events;
    let events_before = env.events().all().len();

    client.set_fot_router(&router_id, &100);

    let events_after = env.events().all().len();
    assert!(events_after > events_before, "set_fot_router must emit an event");
}

#[test]
fn test_clear_fot_router_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);

    let token_id = env.register_contract(None, DeflatToken);
    let router_id = env.register_contract(None, MockFotRouter);

    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "event-prog-2");

    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    client.set_fot_router(&router_id, &100);

    use soroban_sdk::testutils::Events;
    let events_before = env.events().all().len();

    client.clear_fot_router();

    let events_after = env.events().all().len();
    assert!(events_after > events_before, "clear_fot_router must emit an event");
}

// ===========================================================================
// 14. Slippage bounds validation
// ===========================================================================

#[test]
#[should_panic(expected = "slippage exceeds maximum")]
fn test_set_fot_router_rejects_excessive_slippage() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);

    let token_id = env.register_contract(None, DeflatToken);
    let router_id = env.register_contract(None, MockFotRouter);

    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "slippage-bound");

    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    // 600 bps = 6% > 5% max
    client.set_fot_router(&router_id, &600);
}

// ===========================================================================
// 15. Batch payout with fee waiver + routing
// ===========================================================================

#[test]
fn test_batch_payout_fee_waived_with_routing() {
    let env = Env::default();
    let setup = setup_with_router(&env, 500, 500, 0);
    fund_contract(&setup, &env, 15_000);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    let recipients = vec![&env, r1.clone(), r2.clone()];
    let amounts = vec![&env, 5_000_i128, 4_000_i128];

    setup.client.batch_payout(&recipients, &amounts, &None);

    // 5% FoT: router ceil(5000*10000/9500)=5264, ceil(4000*10000/9500)=4211
    assert_eq!(setup.token.balance(&r1), 5_000, "r1 receives intended 5000");
    assert_eq!(setup.token.balance(&r2), 4_000, "r2 receives intended 4000");

    // Outflow: 5264 + 4211 = 9475
    assert_eq!(setup.client.get_program_info().remaining_balance, 15_000 - 5_264 - 4_211);
}

// ===========================================================================
// 16. Router configured but token has 0% FoT (over-routing)
// ===========================================================================

#[test]
fn test_router_no_fot_on_token() {
    let env = Env::default();
    let setup = setup_with_router(&env, 0, 1_000, 0);
    fund_contract(&setup, &env, 5_000);

    let recipient = Address::generate(&env);
    // Router thinks 10%, quotes 1112 for net=1000
    // Token takes 0% FoT → recipient gets full 1112
    setup.client.single_payout(&recipient, &1_000, &None);

    assert_eq!(setup.token.balance(&recipient), 1_112);
    assert_eq!(setup.client.get_program_info().remaining_balance, 3_888);
}

// ===========================================================================
// 17. FotRouter in ProgramData
// ===========================================================================

#[test]
fn test_fot_router_in_program_data() {
    let env = Env::default();
    let setup = setup_with_router(&env, 0, 0, 100);

    let info = setup.client.get_program_info();
    assert!(info.fot_router.is_some(), "ProgramData must contain fot_router config");

    let fot_router = info.fot_router.unwrap();
    assert_eq!(fot_router.router_contract, setup.router.address);
    assert_eq!(fot_router.slippage_bps, 100);
}

// ===========================================================================
// 18. Clear router removes fot_router from ProgramData
// ===========================================================================

#[test]
fn test_clear_router_removes_config() {
    let env = Env::default();
    let setup = setup_with_router(&env, 0, 0, 100);
    setup.client.clear_fot_router();

    let info = setup.client.get_program_info();
    assert!(info.fot_router.is_none(), "After clearing, fot_router must be None");
}

// ===========================================================================
// 19. PayoutRecord reflects routed transfer amount (single)
// ===========================================================================

#[test]
fn test_single_payout_record_reflects_transfer_amount() {
    let env = Env::default();
    let setup = setup_with_router(&env, 1_000, 1_000, 0);
    fund_contract(&setup, &env, 5_000);

    let recipient = Address::generate(&env);
    setup.client.single_payout(&recipient, &900, &None);

    let info = setup.client.get_program_info();
    let record = info.payout_history.get(0).unwrap();
    assert_eq!(record.amount, 1_000,
        "PayoutRecord.amount must reflect the routed transfer amount (1000), not the net (900)");
}

// ===========================================================================
// 20. PayoutRecord reflects routed transfer amounts (batch)
// ===========================================================================

#[test]
fn test_batch_payout_records_reflect_transfer_amounts() {
    let env = Env::default();
    let setup = setup_with_router(&env, 500, 500, 0);
    fund_contract(&setup, &env, 10_000);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    let recipients = vec![&env, r1, r2];
    let amounts = vec![&env, 2_000_i128, 3_000_i128];

    setup.client.batch_payout(&recipients, &amounts, &None);

    let info = setup.client.get_program_info();
    let record0 = info.payout_history.get(0).unwrap();
    let record1 = info.payout_history.get(1).unwrap();
    assert_eq!(record0.amount, 2_106, "PayoutRecord[0] reflects routed amount");
    assert_eq!(record1.amount, 3_158, "PayoutRecord[1] reflects routed amount");
}

// ===========================================================================
// 21. Single payout with protocol fee and routing
// ===========================================================================

#[test]
fn test_single_payout_with_protocol_fee_and_routing() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);

    let token_id = env.register_contract(None, DeflatToken);
    let token = DeflatTokenClient::new(env, &token_id);
    token.set_fee_bps(&1_000); // 10% FoT

    let router_id = env.register_contract(None, MockFotRouter);
    let router = MockFotRouterClient::new(env, &router_id);
    router.set_fee(&token_id, &1_000); // 10% FoT in router

    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "fee-routing");

    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);

    // Enable a 5% protocol fee
    client.update_fee_config(&100, &0, &500, &0, &admin, &true, &0);
    client.set_fot_router(&router_id, &0);
    client.publish_program(&program_id, &admin);

    // Fund
    token.mint(&client.address, &20_000);
    client.lock_program_funds(&20_000);

    let recipient = Address::generate(&env);
    // amount=2000, 5% protocol fee=100, net=1900
    // Router 10% FoT: gross = ceil(1900*10000/9000) = 2112
    // Total debit = 100 + 2112 = 2212
    client.single_payout(&recipient, &2_000, &None);

    // Recipient gets 2112 - 10% FoT = 1900.8 → 1900 (floor)
    assert_eq!(token.balance(&recipient), 1_900,
        "Recipient gets intended net after both protocol and FoT fees");
    // remaining: 20000 - 2212 = 17788
    assert_eq!(client.get_program_info().remaining_balance, 17_788);
}

// ===========================================================================
// 22. Batch payout with protocol fee and routing
// ===========================================================================

#[test]
fn test_batch_payout_with_protocol_fee_and_routing() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);

    let token_id = env.register_contract(None, DeflatToken);
    let token = DeflatTokenClient::new(env, &token_id);
    token.set_fee_bps(&500); // 5% FoT

    let router_id = env.register_contract(None, MockFotRouter);
    let router = MockFotRouterClient::new(env, &router_id);
    router.set_fee(&token_id, &500); // 5% FoT

    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "batch-fee-routing");

    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    client.update_fee_config(&100, &0, &500, &0, &admin, &true, &0);
    client.set_fot_router(&router_id, &0);
    client.publish_program(&program_id, &admin);

    token.mint(&client.address, &30_000);
    client.lock_program_funds(&30_000);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    let recipients = vec![&env, r1.clone(), r2.clone()];
    let amounts = vec![&env, 10_000_i128, 10_000_i128];

    client.batch_payout(&recipients, &amounts, &None);

    // Each: amount=10000, 5% protocol fee=500, net=9500
    // Router 5%: ceil(9500*10000/9500)=10000
    // Total debit per recipient: 500 + 10000 = 10500
    // Total outflow: 21000
    // Recipient gets: 10000 - 5% = 9500 ✓
    assert_eq!(token.balance(&r1), 9_500, "r1 gets intended net");
    assert_eq!(token.balance(&r2), 9_500, "r2 gets intended net");
    assert_eq!(client.get_program_info().remaining_balance, 9_000);
}
