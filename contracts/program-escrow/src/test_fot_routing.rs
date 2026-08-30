#![cfg(test)]

use crate::errors::ContractError;
use crate::{
    FeeCollectedEvent, FundsLockedEvent, PayoutEvent, ProgramEscrowContract,
    ProgramEscrowContractClient,
};
use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, Events},
    vec, Address, Env, IntoVal, String, Symbol, TryFromVal, Val, Vec,
};

// ===========================================================================
// Mock contracts
// ---------------------------------------------------------------------------
// Each `#[contract]` mock lives in its own module so the name-generating
// `#[contractimpl]` machinery (e.g. the private `__quote` / `__SPEC_XDR_FN_*`
// shims) is scoped per module and does not collide when two mock contracts
// expose functions with the same name (as `MockFotRouter` and
// `InflatedFotRouter` both expose `quote`).
// ===========================================================================

pub mod fot_mocks {
    // ── Mock FoT token ───────────────────────────────────────────────
    pub mod deflat_token {
        use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

        /// A fee-on-transfer token. Every transfer burns `fee_bps` (out of
        /// 10_000) and credits the recipient with the remainder. The fee is
        /// configurable per test so different fee scenarios can be exercised
        /// against the same escrow contract.
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
    }

    // ── Mock FoT router ──────────────────────────────────────────────
    pub mod router {
        use soroban_sdk::{contract, contractimpl, Address, Env};

        /// A mock router that reports the gross amount needed to net `amount`
        /// after a configurable fee-on-transfer deduction.
        ///
        /// Fee is stored per-token: `token_address -> fee_bps`.
        /// `quote(token, amount)` returns `ceil(amount * 10000 / (10000 - fee_bps))`.
        ///
        /// A fee >= 100% (>= 10_000 bps) is an *impossible configuration*: the
        /// gross-up divisor is zero or negative. The router signals this by
        /// returning `0`, and the escrow's `apply_fot_router` turns it into the
        /// typed `ContractError::FotRoutingFailed`.
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
                    return 0;
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
    }

    // ── Mock malicious / inflated router ─────────────────────────────
    pub mod inflated_router {
        use soroban_sdk::{contract, contractimpl, Address, Env};

        /// A mock router that simulates a compromised quote source by inflating
        /// the gross amount far above any plausible fee-on-transfer compensation.
        #[contract]
        pub struct InflatedFotRouter;

        #[contractimpl]
        impl InflatedFotRouter {
            pub fn quote(_env: Env, _token: Address, amount: i128) -> i128 {
                // Return 100x the intended net amount; this should be rejected by
                // the upper-bound check before any tokens are transferred.
                amount.checked_mul(100).expect("Mock inflated amount overflow")
            }
        }
    }
}

use fot_mocks::deflat_token::{DeflatToken, DeflatTokenClient};
use fot_mocks::inflated_router::InflatedFotRouter;
use fot_mocks::router::{MockFotRouter, MockFotRouterClient};

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
    // Use a 5x upper-bound multiplier so high-FoT router tests (up to 50% fee) pass.
    client.publish_program(&program_id, &admin);
    client.set_fot_router(&router_id, &slippage_bps, &50_000);

    FotRoutingSetup { client, token, router, admin }
}

struct NoRouterSetup<'a> {
    client: ProgramEscrowContractClient<'a>,
    token: DeflatTokenClient<'a>,
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

    NoRouterSetup { client, token }
}

/// Fund the contract with `gross_amount` tokens via lock_program_funds.
/// Mints tokens directly so the contract holds the full amount.
fn fund_contract(setup: &FotRoutingSetup<'_>, _env: &Env, gross_amount: i128) {
    setup.token.mint(&setup.client.address, &gross_amount);
    setup.client.lock_program_funds(&gross_amount);
}

/// Fund the contract without router (no-router setup).
fn fund_contract_no_router(setup: &NoRouterSetup<'_>, gross_amount: i128) {
    setup.token.mint(&setup.client.address, &gross_amount);
    setup.client.lock_program_funds(&gross_amount);
}

/// Gross-up formula used by `MockFotRouter` (ceiling division).
fn gross_up(net: i128, fee_bps: i128) -> i128 {
    let numerator = net * 10_000;
    let denominator = 10_000 - fee_bps;
    (numerator + denominator - 1) / denominator
}

/// The liability invariant (issue #1721):
/// `remaining_balance + insurance_reserve == on_chain_token_balance`.
/// Protocol fee carve-outs that are booked to the insurance reserve stay on the
/// escrow wallet until withdrawn, so the reserve must be added back on the
/// liability side before comparing to the on-chain balance.
fn assert_liability_invariant(
    client: &ProgramEscrowContractClient<'_>,
    token: &DeflatTokenClient<'_>,
    reserve: i128,
) {
    let info = client.get_program_info();
    let on_chain = token.balance(&client.address);
    assert_eq!(
        info.remaining_balance + reserve,
        on_chain,
        "liability invariant violated: remaining_balance({}) + reserve({}) != on_chain({})",
        info.remaining_balance,
        reserve,
        on_chain,
    );
}

fn event_data(env: &Env, topic: Symbol) -> Option<Val> {
    env.events()
        .all()
        .iter()
        .find_map(|(_, topics, data)| {
            let first = topics.get(0)?;
            if Symbol::try_from_val(env, &first).map(|t| t == topic).unwrap_or(false) {
                Some(data.clone())
            } else {
                None
            }
        })
}

fn payout_events(env: &Env) -> Vec<PayoutEvent> {
    let topic = Symbol::new(env, "Payout");
    let mut out: Vec<PayoutEvent> = Vec::new(env);
    for (_, topics, data) in env.events().all() {
        let first = topics.get(0).unwrap_or_default();
        if Symbol::try_from_val(env, &first).map(|t| t == topic).unwrap_or(false) {
            out.push_back(PayoutEvent::try_from_val(env, &data).expect("valid PayoutEvent"));
        }
    }
    out
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

    setup.client.batch_payout(&recipients, &amounts);

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

    // Transfer amount = 1050, FoT 10% → recipient gets 945 (= 1050 - 105)
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

    setup.client.batch_payout(&recipients, &amounts);

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
    setup.client.batch_payout(&recipients, &amounts);
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
    assert_eq!(setup.client.get_program_info().remaining_balance, 9_000);
}

// ===========================================================================
// 13. Event emission
// ===========================================================================

#[test]
fn test_set_fot_router_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let token_id = env.register_contract(None, DeflatToken);
    let router_id = env.register_contract(None, MockFotRouter);

    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "event-prog");

    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);

    let events_before = env.events().all().len();

    client.set_fot_router(&router_id, &100, &15_000);

    let events_after = env.events().all().len();
    assert!(events_after > events_before, "set_fot_router must emit an event");
}

#[test]
fn test_clear_fot_router_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let token_id = env.register_contract(None, DeflatToken);
    let router_id = env.register_contract(None, MockFotRouter);

    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "event-prog-2");

    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    client.set_fot_router(&router_id, &100, &15_000);

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
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let token_id = env.register_contract(None, DeflatToken);
    let router_id = env.register_contract(None, MockFotRouter);

    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "slippage-bound");

    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    // 600 bps = 6% > 5% max
    client.set_fot_router(&router_id, &600, &15_000);
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

    setup.client.batch_payout(&recipients, &amounts);

    // 5% FoT: router ceil(5000*10000/9500)=5264, ceil(4000*10000/9500)=4211
    // Token takes 5%: r1 net = 5264 - floor(5264*5%) = 5264 - 263 = 5001
    //                 r2 net = 4211 - floor(4211*5%) = 4211 - 210 = 4001
    assert_eq!(setup.token.balance(&r1), 5_001, "r1 receives intended (~5000) net");
    assert_eq!(setup.token.balance(&r2), 4_001, "r2 receives intended (~4000) net");

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
    let fot_router = match info.fot_router {
        crate::OptionalFotRouter::Some(r) => r,
        crate::OptionalFotRouter::None => panic!("ProgramData must contain fot_router config"),
    };
    assert_eq!(fot_router.router_contract, setup.router.address);
    assert_eq!(fot_router.slippage_bps, 100);
    assert_eq!(fot_router.max_fot_multiplier_bps, 50_000,
        "max_fot_multiplier_bps default used by the helper must be persisted");
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
    assert!(
        matches!(info.fot_router, crate::OptionalFotRouter::None),
        "After clearing, fot_router must be None"
    );
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

    setup.client.batch_payout(&recipients, &amounts);

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
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let token_id = env.register_contract(None, DeflatToken);
    let token = DeflatTokenClient::new(&env, &token_id);
    token.set_fee_bps(&1_000); // 10% FoT

    let router_id = env.register_contract(None, MockFotRouter);
    let router = MockFotRouterClient::new(&env, &router_id);
    router.set_fee(&token_id, &1_000); // 10% FoT in router

    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "fee-routing");

    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);

    // Enable a 5% protocol payout fee
    client.update_fee_config(&Some(0), &Some(500), &None, &None, &Some(admin.clone()), &Some(true), &Some(0));
    client.publish_program(&program_id, &admin);
    client.set_fot_router(&router_id, &0, &15_000);

    // Fund
    token.mint(&client.address, &20_000);
    client.lock_program_funds(&20_000);

    let recipient = Address::generate(&env);
    // amount=2000, 5% protocol fee (ceiling): fee=100, net=1900
    // Router 10% FoT: gross = ceil(1900*10000/9000) = 2112
    // Total debit = 100 + 2112 = 2212
    client.single_payout(&recipient, &2_000, &None);

    // Recipient gets 2112 - floor(2112*10%) = 2112 - 211 = 1901
    // (one token above the intended 1900 net due to the router's ceiling
    //  rounding — routing still guarantees the recipient is never short)
    assert_eq!(token.balance(&recipient), 1_901);
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
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let token_id = env.register_contract(None, DeflatToken);
    let token = DeflatTokenClient::new(&env, &token_id);
    token.set_fee_bps(&500); // 5% FoT

    let router_id = env.register_contract(None, MockFotRouter);
    let router = MockFotRouterClient::new(&env, &router_id);
    router.set_fee(&token_id, &500); // 5% FoT

    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "batch-fee-routing");

    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    client.update_fee_config(&Some(0), &Some(500), &None, &None, &Some(admin.clone()), &Some(true), &Some(0));
    client.publish_program(&program_id, &admin);
    client.set_fot_router(&router_id, &0, &15_000);

    token.mint(&client.address, &30_000);
    client.lock_program_funds(&30_000);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    let recipients = vec![&env, r1.clone(), r2.clone()];
    let amounts = vec![&env, 10_000_i128, 10_000_i128];

    client.batch_payout(&recipients, &amounts);

    // Each: amount=10000, 5% protocol fee=500, net=9500
    // Router 5%: ceil(9500*10000/9500)=10000
    // Total debit per recipient: 500 + 10000 = 10500
    // Total outflow: 21000
    // Recipient gets: 10000 - floor(10000*5%) = 9500 ✓
    assert_eq!(token.balance(&r1), 9_500, "r1 gets intended net");
    assert_eq!(token.balance(&r2), 9_500, "r2 gets intended net");
    assert_eq!(client.get_program_info().remaining_balance, 9_000);
}

// ===========================================================================
// 23. Malicious router with an inflated quote is rejected
// ===========================================================================

#[test]
#[should_panic(expected = "HostError: Error(Contract, #1211)")]
fn test_malicious_router_inflated_quote_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let token_id = env.register_contract(None, DeflatToken);
    let token = DeflatTokenClient::new(&env, &token_id);
    token.set_fee_bps(&0);

    let router_id = env.register_contract(None, InflatedFotRouter);

    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "malicious-router-prog");

    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    // Cap the gross quote at 1.5x net. The malicious router returns 100x.
    client.publish_program(&program_id, &admin);
    client.set_fot_router(&router_id, &0, &15_000);

    // Fund the program
    token.mint(&client.address, &10_000);
    client.lock_program_funds(&10_000);

    let recipient = Address::generate(&env);
    // This must abort because the router's 100x gross quote exceeds the 1.5x cap.
    // No tokens should be transferred to the recipient or debited beyond the lock.
    client.single_payout(&recipient, &100, &None);
}

// ===========================================================================
// 24 (NEW). End-to-end liability invariant across deposit → payout → refund
// ===========================================================================
//
// Mock token charges 10% on every transfer. The router grosses up payouts so
// the beneficiary receives the intended net. This test proves the liability
// invariant `remaining_balance == on_chain_token_balance` after every leg and
// asserts the contract balance, beneficiary amount, and emitted amounts.
//
//   Deposit: funder mints 1250 and transfers → escrow receives 1125 (10% burn).
//            funder locks 1125 = the RECEIVED value  ⇒ remaining == on_chain.
//   Payout:  net 450 → gross 500, recipient nets 450 ⇒ remaining == on_chain.
//   Refund:  net 450 → gross 500, funder nets 450    ⇒ remaining == on_chain.
//
// FoT burn: deposit 125 + payout 50 + refund 50 = 225 (fully outside liability).

#[test]
fn test_end_to_end_liability_invariant_deposit_payout_refund() {
    let env = Env::default();
    let setup = setup_with_router(&env, 1_000, 1_000, 0);

    let funder = Address::generate(&env);
    let recipient = Address::generate(&env);

    let minted = 1_250_i128;

    // ── DEPOSIT leg ────────────────────────────────────────────────────
    // Funder transfers 1250 through the 10%-fee token; the escrow is credited
    // with only what it actually received (1125), so liability can never exceed
    // the on-chain balance.
    setup.token.mint(&funder, &minted);
    setup
        .token
        .transfer(&funder, &setup.client.address, &minted);

    setup.client.lock_program_funds(&1_125);
    assert_eq!(setup.token.balance(&setup.client.address), 1_125, "contract balance after deposit");
    assert_eq!(setup.client.get_program_info().remaining_balance, 1_125);
    assert_liability_invariant(&setup.client, &setup.token, 0);

    // FundsLocked event credits the received amount, not the gross minted.
    let funds_locked: FundsLockedEvent = event_data(&env, Symbol::new(&env, "FndsLock"))
        .map(|data| FundsLockedEvent::try_from_val(&env, &data).expect("valid FundsLockedEvent"))
        .expect("FundsLocked event must be emitted");
    assert_eq!(funds_locked.amount, 1_125, "FundsLocked.amount must be the received value");
    assert_eq!(funds_locked.remaining_balance, 1_125);

    // ── PAYOUT leg ─────────────────────────────────────────────────────
    // net 450 → router gross 500 → escrow transfers 500, recipient nets 450.
    setup.client.single_payout(&recipient, &450, &None);
    assert_eq!(setup.token.balance(&recipient), 450, "beneficiary receives intended net");
    assert_eq!(setup.token.balance(&setup.client.address), 625, "contract balance after payout");
    assert_eq!(setup.client.get_program_info().remaining_balance, 625);
    assert_liability_invariant(&setup.client, &setup.token, 0);

    // Payout event (payout leg) reflects the routed transfer amount.
    let payouts = payout_events(&env);
    assert_eq!(payouts.len(), 1);
    assert_eq!(payouts.get(0).unwrap().amount, 500, "Payout.amount must be the routed gross (500)");
    assert_eq!(payouts.get(0).unwrap().remaining_balance, 625);

    // ── REFUND leg ─────────────────────────────────────────────────────
    // The outstanding liability (625) is (partially) returned to the funder as
    // a payout-type transfer. net 450 → gross 500, funder nets 450.
    setup.client.single_payout(&funder, &450, &None);
    assert_eq!(setup.token.balance(&funder), 450, "funder refund nets the intended amount");
    assert_eq!(setup.token.balance(&setup.client.address), 125, "contract balance after refund");
    assert_eq!(setup.client.get_program_info().remaining_balance, 125);
    assert_liability_invariant(&setup.client, &setup.token, 0);

    let payouts = payout_events(&env);
    assert_eq!(payouts.len(), 2);
    assert_eq!(payouts.get(1).unwrap().amount, 500, "refund Payout.amount must be the routed gross (500)");
    assert_eq!(payouts.get(1).unwrap().remaining_balance, 125);

    // ── Conservation ───────────────────────────────────────────────────
    // Every minted token is accounted for: beneficiaries + escrow + FoT burns.
    // 1250 = 450 (recipient) + 450 (funder) + 125 (escrow dust) + 225 (burns)
    let burns = minted
        - setup.token.balance(&recipient)
        - setup.token.balance(&funder)
        - setup.token.balance(&setup.client.address);
    assert_eq!(burns, 225, "FoT burns: deposit 125 + payout 50 + refund 50 = 225");
}

// ===========================================================================
// 25 (NEW). End-to-end liability invariant with insurance reserve carve-out
// ===========================================================================
//
// Same lifecycle but with a 5% protocol payout fee and 5% of each fee carved
// into the on-chain insurance reserve (insurance_reserve_bps = 500, capped at
// 1_000 bps). On every fee collection, `ceil(fee * 500 / 10_000)` is booked to
// the reserve (which stays in the escrow wallet) and the remainder goes to the
// fee recipient. The invariant therefore becomes
// `remaining_balance + insurance_reserve == on_chain_token_balance`.

#[test]
fn test_end_to_end_liability_invariant_with_insurance_reserve() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let token_id = env.register_contract(None, DeflatToken);
    let token = DeflatTokenClient::new(&env, &token_id);
    token.set_fee_bps(&1_000); // 10% FoT

    let router_id = env.register_contract(None, MockFotRouter);
    let router = MockFotRouterClient::new(&env, &router_id);
    router.set_fee(&token_id, &1_000);

    let admin = Address::generate(&env);
    let fee_recipient = Address::generate(&env);
    let funder = Address::generate(&env);
    let recipient = Address::generate(&env);

    let program_id = String::from_str(&env, "reserve-prog");
    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    // 5% protocol payout fee, 5% of each fee carved to the insurance reserve.
    client.update_fee_config(
        &Some(0),
        &Some(500),
        &None,
        &None,
        &Some(fee_recipient.clone()),
        &Some(true),
        &Some(500),
    );
    client.publish_program(&program_id, &admin);
    client.set_fot_router(&router_id, &0, &15_000);

    // ── DEPOSIT leg (received-value crediting) ────────────────────────
    let minted = 12_500_i128;
    token.mint(&funder, &minted);
    token.transfer(&funder, &client.address, &minted); // 10% burn → 11 250 received
    client.lock_program_funds(&11_250);
    assert_eq!(token.balance(&client.address), 11_250);
    assert_eq!(client.get_program_info().remaining_balance, 11_250);
    assert_eq!(client.get_insurance_reserve_balance(), 0);
    assert_liability_invariant(&client, &token, 0);

    // ── PAYOUT leg ─────────────────────────────────────────────────────
    // amount=2000 → protocol fee=100 (ceil), net=1900, router gross=2112.
    // fee: reserve_share=ceil(100*500/10000)=5, recipient_share=95.
    // The 95 transfer to the fee recipient also crosses the 10% FoT, so they
    // net 86 (9 burned is outside liability). total_debit = 2112+100 = 2212.
    client.single_payout(&recipient, &2_000, &None);

    assert_eq!(token.balance(&recipient), 1_901, "beneficiary nets intended ~1900");
    assert_eq!(token.balance(&fee_recipient), 86, "fee recipient nets 86 after FoT on the 95 transfer");
    assert_eq!(client.get_insurance_reserve_balance(), 5, "reserve booked 5");
    assert_eq!(client.get_program_info().remaining_balance, 11_250 - 2_212);
    // on_chain = 11_250 - 2112 (transfer) - 95 (recipient share gross) = 9_043
    assert_eq!(token.balance(&client.address), 9_043);
    assert_liability_invariant(&client, &token, 5);

    // FeeCollected event carries the protocol fee amount.
    let fee_event: FeeCollectedEvent = event_data(&env, Symbol::new(&env, "FeeCol"))
        .map(|data| FeeCollectedEvent::try_from_val(&env, &data).expect("valid FeeCollectedEvent"))
        .expect("FeeCollected event must be emitted");
    assert_eq!(fee_event.fee_amount, 100, "FeeCollected.fee_amount must be 100");

    // ── REFUND leg ─────────────────────────────────────────────────────
    // amount=400 → protocol fee=20 (ceil), net=380, router gross=423.
    // fee: reserve_share=ceil(20*500/10000)=1, recipient_share=19.
    // The 19 transfer to the fee recipient nets 18 after the 10% FoT (1 burned).
    // total_debit = 423+20 = 443.
    client.single_payout(&funder, &400, &None);

    assert_eq!(token.balance(&funder), 381, "funder refund nets ~380");
    assert_eq!(token.balance(&fee_recipient), 104, "fee recipient total 86+18 = 104");
    assert_eq!(client.get_insurance_reserve_balance(), 6, "reserve total 6");
    assert_eq!(client.get_program_info().remaining_balance, 9_038 - 443);
    // on_chain = 9_043 - 423 (transfer) - 19 (recipient share gross) = 8_601
    assert_eq!(token.balance(&client.address), 8_601);
    assert_liability_invariant(&client, &token, 6);

    // Both payouts (payout + refund) carry the routed transfer amounts.
    let payouts = payout_events(&env);
    assert_eq!(payouts.len(), 2);
    assert_eq!(payouts.get(0).unwrap().amount, 2_112, "payout leg Payout.amount = routed gross");
    assert_eq!(payouts.get(1).unwrap().amount, 423, "refund leg Payout.amount = routed gross");
}

// ===========================================================================
// 26 (NEW). Every supported fee scenario preserves the liability invariant
// ===========================================================================
//
// A fee is "supported" iff the router can gross it up (fee < 10_000 bps) and
// the configured max multiplier bounds it. For each supported fee rate the
// contract balance and `remaining_balance` are debited by the same routed
// gross, so the invariant `remaining_balance == on_chain` holds and the
// beneficiary always receives at least the intended net.

#[test]
fn test_supported_fee_scenarios_preserve_invariant() {
    for fee_bps in [0_i128, 100, 500, 1_000, 2_000, 5_000] {
        let env = Env::default();
        let setup = setup_with_router(&env, fee_bps, fee_bps, 0);
        fund_contract(&setup, &env, 10_000);

        let recipient = Address::generate(&env);
        setup.client.single_payout(&recipient, &1_000, &None);

        let gross = gross_up(1_000, fee_bps);
        // The mock token floors its own fee: fee = gross * fee_bps / 10_000.
        let expected_net = gross - gross * fee_bps / 10_000;
        assert!(
            expected_net >= 1_000,
            "supported scenario must never short the beneficiary (fee={fee_bps}bps, net={expected_net})"
        );
        assert_eq!(
            setup.token.balance(&recipient),
            expected_net,
            "beneficiary nets the intended amount at fee={fee_bps}bps"
        );
        assert_eq!(
            setup.client.get_program_info().remaining_balance,
            10_000 - gross,
            "remaining debited by routed gross at fee={fee_bps}bps"
        );
        assert_liability_invariant(&setup.client, &setup.token, 0);
    }
}

// ===========================================================================
// 27 (NEW). Impossible fee configurations return a typed error
// ===========================================================================
//
// A fee >= 10_000 bps (100%) cannot be routed: the gross-up divisor is zero or
// negative, so the mock router returns a 0 quote and `apply_fot_router` must
// abort with the typed `ContractError::FotRoutingFailed` (1210) instead of a
// generic panic. Both the 100% boundary and an impossible >100% rate are
// exercised.

#[test]
fn test_impossible_fot_fee_returns_typed_error() {
    for fee_bps in [10_000_i128, 12_000] {
        let env = Env::default();
        let setup = setup_with_router(&env, fee_bps, fee_bps, 0);
        fund_contract(&setup, &env, 10_000);

        let recipient = Address::generate(&env);
        let res = setup.client.try_single_payout(&recipient, &1_000, &None);

        assert!(
            matches!(
                res,
                Err(Ok(err)) if err.get_code() == ContractError::FotRoutingFailed as u32
            ),
            "fee={fee_bps}bps must surface the typed FotRoutingFailed error, got {res:?}"
        );
    }
}

// ===========================================================================
// 28 (NEW). A misconfigured router that cannot quote returns a typed error
// ===========================================================================
//
// Even when the token has no fee, a router misconfigured with an impossible
// fee (>100%) cannot produce a gross. The escrow must surface it as the typed
// `ContractError::FotRoutingFailed` rather than leaking a generic panic from
// the router contract.

#[test]
fn test_router_zero_quote_returns_typed_error() {
    let env = Env::default();
    // Token: 0% FoT (fine). Router: 100% (impossible) → quote 0.
    let setup = setup_with_router(&env, 0, 10_000, 0);
    fund_contract(&setup, &env, 10_000);

    let recipient = Address::generate(&env);
    let res = setup.client.try_single_payout(&recipient, &1_000, &None);

    assert!(
        matches!(
            res,
            Err(Ok(err)) if err.get_code() == ContractError::FotRoutingFailed as u32
        ),
        "misconfigured router must surface FotRoutingFailed, got {res:?}"
    );
}

// ===========================================================================
// 29 (NEW). Malicious inflated quote surfaces the typed FotRouterQuoteExceeded
// ===========================================================================

#[test]
fn test_malicious_router_inflated_quote_typed_error() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let token_id = env.register_contract(None, DeflatToken);
    let token = DeflatTokenClient::new(&env, &token_id);
    token.set_fee_bps(&0);

    let router_id = env.register_contract(None, InflatedFotRouter);

    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "malicious-typed-prog");

    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    client.publish_program(&program_id, &admin);
    client.set_fot_router(&router_id, &0, &15_000);

    token.mint(&client.address, &10_000);
    client.lock_program_funds(&10_000);

    let recipient = Address::generate(&env);
    let res = client.try_single_payout(&recipient, &100, &None);

    assert!(
        matches!(
            res,
            Err(Ok(err)) if err.get_code() == ContractError::FotRouterQuoteExceeded as u32
        ),
        "inflated quote must surface FotRouterQuoteExceeded, got {res:?}"
    );
}