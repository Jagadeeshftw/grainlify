use super::*;
use soroban_sdk::{
    contract, contractimpl, testutils::Address as _, token, Address, Env, Vec, IntoVal,
};

#[contract]
pub struct MockRouter;

#[contractimpl]
impl MockRouter {
    pub fn set_rates(env: Env, quote_rate: i128, swap_rate: i128) {
        env.storage().instance().set(&soroban_sdk::symbol_short!("q_rate"), &quote_rate);
        env.storage().instance().set(&soroban_sdk::symbol_short!("s_rate"), &swap_rate);
    }

    pub fn get_rates(env: Env) -> (i128, i128) {
        let q = env.storage().instance().get(&soroban_sdk::symbol_short!("q_rate")).unwrap_or(2);
        let s = env.storage().instance().get(&soroban_sdk::symbol_short!("s_rate")).unwrap_or(2);
        (q, s)
    }

    pub fn swap_exact_tokens_for_tokens(
        env: Env,
        amount_in: i128,
        _amount_out_min: i128,
        path: Vec<Address>,
        to: Address,
        _deadline: u64,
    ) -> Vec<i128> {
        let src = path.get(0).unwrap();
        let dest = path.get(path.len() - 1).unwrap();
        let (_, s_rate) = Self::get_rates(env.clone());
        
        let actual_out = amount_in * s_rate;
        
        // Transfer source tokens to mock router
        let src_client = token::Client::new(&env, &src);
        src_client.transfer(&env.current_contract_address(), &dest, &amount_in);
        
        // Transfer destination tokens to contributor
        let dest_client = token::Client::new(&env, &dest);
        dest_client.transfer(&env.current_contract_address(), &to, &actual_out);
        
        let mut result = Vec::new(&env);
        result.push_back(amount_in);
        result.push_back(actual_out);
        result
    }

    pub fn get_amounts_out(env: Env, amount_in: i128, _path: Vec<Address>) -> Vec<i128> {
        let (q_rate, _) = Self::get_rates(env.clone());
        let mut result = Vec::new(&env);
        result.push_back(amount_in);
        result.push_back(amount_in * q_rate);
        result
    }
}

fn create_token_contract<'a>(
    e: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
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

struct TestSetup<'a> {
    env: Env,
    admin: Address,
    depositor: Address,
    contributor: Address,
    token_a: token::Client<'a>,
    token_a_admin: token::StellarAssetClient<'a>,
    token_b: token::Client<'a>,
    token_b_admin: token::StellarAssetClient<'a>,
    escrow: BountyEscrowContractClient<'a>,
    router_address: Address,
    router: MockRouterClient<'a>,
}

impl<'a> TestSetup<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);

        let (token_a, token_a_admin) = create_token_contract(&env, &admin);
        let (token_b, token_b_admin) = create_token_contract(&env, &admin);

        let escrow = create_escrow_contract(&env);
        escrow.init(&admin, &token_a.address);

        let router_id = env.register_contract(None, MockRouter);
        let router = MockRouterClient::new(&env, &router_id);

        token_a_admin.mint(&depositor, &1_000_000);
        // Fund the router with token_b for conversions
        token_b_admin.mint(&router_id, &2_000_000);

        Self {
            env,
            admin,
            depositor,
            contributor,
            token_a,
            token_a_admin,
            token_b,
            token_b_admin,
            escrow,
            router_address: router_id,
            router,
        }
    }
}

#[test]
fn test_release_with_conversion_happy_path() {
    let s = TestSetup::new();
    let bounty_id = 1u64;
    let amount = 1000i128;
    let deadline = s.env.ledger().timestamp() + 3600;

    // Lock funds first
    s.escrow.lock_funds(&s.depositor, &bounty_id, &amount, &deadline, &None, &None);

    // Set the router
    s.escrow.set_router(&s.router_address);
    assert_eq!(s.escrow.get_router(), Some(s.router_address.clone()));

    // Create the path: token_a -> token_b
    let mut path = Vec::new(&s.env);
    path.push_back(s.token_a.address.clone());
    path.push_back(s.token_b.address.clone());

    // Release with conversion (max slippage 500 bps = 5%)
    let max_slippage_bps = 500u32;
    s.escrow.release_with_conversion(
        &bounty_id,
        &s.contributor,
        &s.token_b.address,
        &path,
        &max_slippage_bps,
    );

    // Verify escrow status is Released and remaining_amount is 0
    let info = s.escrow.get_escrow_info(&bounty_id);
    assert_eq!(info.status, EscrowStatus::Released);
    assert_eq!(info.remaining_amount, 0);

    // Contributor should have received dest tokens at mock rate of 2x
    assert_eq!(s.token_b.balance(&s.contributor), 2000i128);
}

#[test]
#[should_panic(expected = "Error(Contract, #58)")]
fn test_release_with_conversion_router_not_configured() {
    let s = TestSetup::new();
    let bounty_id = 2u64;
    let amount = 1000i128;
    let deadline = s.env.ledger().timestamp() + 3600;

    s.escrow.lock_funds(&s.depositor, &bounty_id, &amount, &deadline, &None, &None);

    let mut path = Vec::new(&s.env);
    path.push_back(s.token_a.address.clone());
    path.push_back(s.token_b.address.clone());

    // Call without setting router
    s.escrow.release_with_conversion(
        &bounty_id,
        &s.contributor,
        &s.token_b.address,
        &path,
        &500u32,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #59)")]
fn test_release_with_conversion_slippage_exceeded() {
    let s = TestSetup::new();
    let bounty_id = 3u64;
    let amount = 1000i128;
    let deadline = s.env.ledger().timestamp() + 3600;

    s.escrow.lock_funds(&s.depositor, &bounty_id, &amount, &deadline, &None, &None);
    s.escrow.set_router(&s.router_address);

    // Set MockRouter quote rate to 2, but actual swap rate to 1 (50% drop, exceeding 5% slippage)
    s.router.set_rates(&2, &1);

    let mut path = Vec::new(&s.env);
    path.push_back(s.token_a.address.clone());
    path.push_back(s.token_b.address.clone());

    // Slippage 500 bps = 5% (min expected is 1900, but actual is 1000, so it must revert)
    s.escrow.release_with_conversion(
        &bounty_id,
        &s.contributor,
        &s.token_b.address,
        &path,
        &500u32,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")]
fn test_set_router_unauthorized() {
    let s = TestSetup::new();
    let non_admin = Address::generate(&s.env);

    s.env.mock_all_auths();
    // Try to set router as non-admin (by changing authorization context)
    s.env.mock_auths(&[Env::default().mock_auths(&[]).get(0).cloned().unwrap_or(non_admin.clone())]); // or simple require_auth mock
    
    // We will clear all mock auths and only mock non_admin
    s.env.mock_auths(&[]);
    let auths = soroban_sdk::vec![
        &s.env,
        soroban_sdk::testutils::MockAuth {
            address: non_admin.clone(),
            invoke: soroban_sdk::testutils::MockAuthInvoke {
                contract: s.escrow.address.clone(),
                function: soroban_sdk::Symbol::new(&s.env, "set_router"),
                args: soroban_sdk::vec![&s.env, s.router_address.clone().into_val(&s.env)],
                sub_invokes: soroban_sdk::vec![&s.env],
            },
        }
    ];
    s.env.mock_auths(&auths);
    
    s.escrow.set_router(&s.router_address);
}
