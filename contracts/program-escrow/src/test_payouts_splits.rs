// ============================================================
// FILE: contracts/program-escrow/src/test_payout_splits.rs
//
// Tests for multi-beneficiary payout splits (Issue #[issue_id]).
// ============================================================

#![cfg(test)]

extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, vec, Address, Env, String,
};

use crate::{
    payout_splits::{
        BeneficiarySplit, SplitConfig, TOTAL_BASIS_POINTS,
        disable_split_config, execute_split_payout, get_split_config, preview_split, set_split_config,
    },
    DataKey, ProgramData, PROGRAM_DATA,
};

// ── Helpers ──────────────────────────────────────────────────────────────────

struct TestSetup {
    env: Env,
    program_id: String,
    payout_key: Address,
    token: Address,
    admin: Address,
}

fn setup() -> TestSetup {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let payout_key = Address::generate(&env);
    let token_admin = Address::generate(&env);

    // Deploy a SAC token for testing
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token = token_contract.address();

    // Mint 1_000_000 units to a funder
    let funder = Address::generate(&env);
    let token_client = token::StellarAssetClient::new(&env, &token);
    token_client.mint(&funder, &1_000_000i128);

    // Register the escrow contract
    let contract_id = env.register_contract(None, crate::ProgramEscrowContract);

    // Bootstrap ProgramData manually (simulate init_program having been called)
    let program_id = String::from_str(&env, "TestProgram");
    let program_data = ProgramData {
        program_id: program_id.clone(),
        total_funds: 100_000,
        remaining_balance: 100_000,
        authorized_payout_key: payout_key.clone(),
        payout_history: vec![&env],
        token_address: token.clone(),
        initial_liquidity: 0,
    };

    // Fund the contract address so token transfers succeed
    token_client.mint(&contract_id, &100_000i128);

    env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .set(&PROGRAM_DATA, &program_data);
        env.storage()
            .instance()
            .set(&DataKey::Admin, &admin);
    });

    TestSetup {
        env,
        program_id,
        payout_key,
        token,
        admin,
    }
}

// ── set_split_config ─────────────────────────────────────────────────────────

#[test]
fn test_set_split_config_success_two_beneficiaries() {
    let s = setup();
    let env = &s.env;
    let a = Address::generate(env);
    let b = Address::generate(env);

    let beneficiaries = vec![
        env,
        BeneficiarySplit { recipient: a.clone(), share_bps: 6_000 },
        BeneficiarySplit { recipient: b.clone(), share_bps: 4_000 },
    ];

    let contract_id = env.register_contract(None, crate::ProgramEscrowContract);
    env.as_contract(&contract_id, || {
        // re-seed program data
        let program_data = ProgramData {
            program_id: s.program_id.clone(),
            total_funds: 100_000,
            remaining_balance: 100_000,
            authorized_payout_key: s.payout_key.clone(),
            payout_history: vec![env],
            token_address: s.token.clone(),
            initial_liquidity: 0,
        };
        env.storage().instance().set(&PROGRAM_DATA, &program_data);

        let cfg = set_split_config(env, &s.program_id, beneficiaries);
        assert!(cfg.active);
        assert_eq!(cfg.beneficiaries.len(), 2);
    });
}

#[test]
#[should_panic(expected = "SplitConfig: shares must sum to 10000 basis points")]
fn test_set_split_config_rejects_wrong_sum() {
    let s = setup();
    let env = &s.env;
    let contract_id = env.register_contract(None, crate::ProgramEscrowContract);
    let a = Address::generate(env);
    let b = Address::generate(env);

    let bad = vec![
        env,
        BeneficiarySplit { recipient: a, share_bps: 5_000 },
        BeneficiarySplit { recipient: b, share_bps: 4_000 }, // sum = 9_000 ≠ 10_000
    ];

    env.as_contract(&contract_id, || {
        let program_data = ProgramData {
            program_id: s.program_id.clone(),
            total_funds: 0,
            remaining_balance: 0,
            authorized_payout_key: s.payout_key.clone(),
            payout_history: vec![env],
            token_address: s.token.clone(),
            initial_liquidity: 0,
        };
        env.storage().instance().set(&PROGRAM_DATA, &program_data);
        set_split_config(env, &s.program_id, bad);
    });
}

#[test]
#[should_panic(expected = "SplitConfig: must have at least one beneficiary")]
fn test_set_split_config_rejects_empty() {
    let s = setup();
    let env = &s.env;
    let contract_id = env.register_contract(None, crate::ProgramEscrowContract);
    let empty: soroban_sdk::Vec<BeneficiarySplit> = soroban_sdk::Vec::new(env);

    env.as_contract(&contract_id, || {
        let program_data = ProgramData {
            program_id: s.program_id.clone(),
            total_funds: 0,
            remaining_balance: 0,
            authorized_payout_key: s.payout_key.clone(),
            payout_history: vec![env],
            token_address: s.token.clone(),
            initial_liquidity: 0,
        };
        env.storage().instance().set(&PROGRAM_DATA, &program_data);
        set_split_config(env, &s.program_id, empty);
    });
}

#[test]
#[should_panic(expected = "SplitConfig: share_bps must be positive")]
fn test_set_split_config_rejects_zero_share() {
    let s = setup();
    let env = &s.env;
    let contract_id = env.register_contract(None, crate::ProgramEscrowContract);
    let a = Address::generate(env);
    let b = Address::generate(env);

    let bad = vec![
        env,
        BeneficiarySplit { recipient: a, share_bps: 10_000 },
        BeneficiarySplit { recipient: b, share_bps: 0 },
    ];

    env.as_contract(&contract_id, || {
        let program_data = ProgramData {
            program_id: s.program_id.clone(),
            total_funds: 0,
            remaining_balance: 0,
            authorized_payout_key: s.payout_key.clone(),
            payout_history: vec![env],
            token_address: s.token.clone(),
            initial_liquidity: 0,
        };
        env.storage().instance().set(&PROGRAM_DATA, &program_data);
        set_split_config(env, &s.program_id, bad);
    });
}

// ── execute_split_payout ──────────────────────────────────────────────────────

#[test]
fn test_exact_split_three_beneficiaries() {
    let env = Env::default();
    env.mock_all_auths();

    let payout_key = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token = token_contract.address();
    let token_sac = token::StellarAssetClient::new(&env, &token);

    let contract_id = env.register_contract(None, crate::ProgramEscrowContract);
    token_sac.mint(&contract_id, &10_000i128);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    let r3 = Address::generate(&env);

    let program_id = String::from_str(&env, "P1");

    env.as_contract(&contract_id, || {
        let program_data = ProgramData {
            program_id: program_id.clone(),
            total_funds: 10_000,
            remaining_balance: 10_000,
            authorized_payout_key: payout_key.clone(),
            payout_history: vec![&env],
            token_address: token.clone(),
            initial_liquidity: 0,
        };
        env.storage().instance().set(&PROGRAM_DATA, &program_data);

        // 50% / 30% / 20%  of 10_000 → 5_000 / 3_000 / 2_000  (no dust)
        let bens = vec![
            &env,
            BeneficiarySplit { recipient: r1.clone(), share_bps: 5_000 },
            BeneficiarySplit { recipient: r2.clone(), share_bps: 3_000 },
            BeneficiarySplit { recipient: r3.clone(), share_bps: 2_000 },
        ];
        set_split_config(&env, &program_id, bens);

        let result = execute_split_payout(&env, &program_id, 10_000);
        assert_eq!(result.total_distributed, 10_000);
        assert_eq!(result.recipient_count, 3);
        assert_eq!(result.remaining_balance, 0);
    });

    let tc = token::Client::new(&env, &token);
    assert_eq!(tc.balance(&r1), 5_000);
    assert_eq!(tc.balance(&r2), 3_000);
    assert_eq!(tc.balance(&r3), 2_000);
}

#[test]
fn test_dust_goes_to_first_beneficiary() {
    let env = Env::default();
    env.mock_all_auths();

    let payout_key = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token = token_contract.address();
    let token_sac = token::StellarAssetClient::new(&env, &token);

    let contract_id = env.register_contract(None, crate::ProgramEscrowContract);
    token_sac.mint(&contract_id, &10i128); // very small amount to force dust

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    let r3 = Address::generate(&env);

    let program_id = String::from_str(&env, "DustTest");

    env.as_contract(&contract_id, || {
        let program_data = ProgramData {
            program_id: program_id.clone(),
            total_funds: 10,
            remaining_balance: 10,
            authorized_payout_key: payout_key.clone(),
            payout_history: vec![&env],
            token_address: token.clone(),
            initial_liquidity: 0,
        };
        env.storage().instance().set(&PROGRAM_DATA, &program_data);

        // 3 equal shares of 10 → floor(10/3) = 3 each, dust = 1
        let bens = vec![
            &env,
            BeneficiarySplit { recipient: r1.clone(), share_bps: 3_334 },
            BeneficiarySplit { recipient: r2.clone(), share_bps: 3_333 },
            BeneficiarySplit { recipient: r3.clone(), share_bps: 3_333 },
        ];
        set_split_config(&env, &program_id, bens);

        let result = execute_split_payout(&env, &program_id, 10);
        assert_eq!(result.total_distributed, 10, "full amount must be distributed");
        assert_eq!(result.remaining_balance, 0);
    });

    let tc = token::Client::new(&env, &token);
    // All 10 tokens must be distributed (r1 gets dust)
    let total = tc.balance(&r1) + tc.balance(&r2) + tc.balance(&r3);
    assert_eq!(total, 10, "No tokens should be lost");
}

#[test]
fn test_partial_release_respects_ratio() {
    let env = Env::default();
    env.mock_all_auths();

    let payout_key = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token = token_contract.address();
    let token_sac = token::StellarAssetClient::new(&env, &token);

    let contract_id = env.register_contract(None, crate::ProgramEscrowContract);
    token_sac.mint(&contract_id, &1_000i128);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    let program_id = String::from_str(&env, "PartialTest");

    env.as_contract(&contract_id, || {
        let program_data = ProgramData {
            program_id: program_id.clone(),
            total_funds: 1_000,
            remaining_balance: 1_000,
            authorized_payout_key: payout_key.clone(),
            payout_history: vec![&env],
            token_address: token.clone(),
            initial_liquidity: 0,
        };
        env.storage().instance().set(&PROGRAM_DATA, &program_data);

        // 70/30 split
        let bens = vec![
            &env,
            BeneficiarySplit { recipient: r1.clone(), share_bps: 7_000 },
            BeneficiarySplit { recipient: r2.clone(), share_bps: 3_000 },
        ];
        set_split_config(&env, &program_id, bens);

        // First partial release: 400 tokens
        let res1 = execute_split_payout(&env, &program_id, 400);
        assert_eq!(res1.remaining_balance, 600);

        // Second partial release: 600 tokens
        let res2 = execute_split_payout(&env, &program_id, 600);
        assert_eq!(res2.remaining_balance, 0);
    });

    let tc = token::Client::new(&env, &token);
    // After 400: r1 = 280, r2 = 120
    // After 600: r1 += 420, r2 += 180
    // Totals: r1 = 700, r2 = 300
    assert_eq!(tc.balance(&r1), 700);
    assert_eq!(tc.balance(&r2), 300);
}

#[test]
#[should_panic(expected = "SplitPayout: insufficient escrow balance")]
fn test_execute_split_payout_insufficient_balance() {
    let env = Env::default();
    env.mock_all_auths();
    let payout_key = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token = token_contract.address();
    let contract_id = env.register_contract(None, crate::ProgramEscrowContract);
    let r1 = Address::generate(&env);
    let program_id = String::from_str(&env, "LowBal");

    env.as_contract(&contract_id, || {
        let program_data = ProgramData {
            program_id: program_id.clone(),
            total_funds: 100,
            remaining_balance: 50,
            authorized_payout_key: payout_key.clone(),
            payout_history: vec![&env],
            token_address: token.clone(),
            initial_liquidity: 0,
        };
        env.storage().instance().set(&PROGRAM_DATA, &program_data);

        let bens = vec![
            &env,
            BeneficiarySplit { recipient: r1.clone(), share_bps: 10_000 },
        ];
        set_split_config(&env, &program_id, bens);
        execute_split_payout(&env, &program_id, 100); // exceeds balance of 50
    });
}

#[test]
#[should_panic(expected = "SplitPayout: split config is disabled")]
fn test_execute_split_payout_disabled_config() {
    let env = Env::default();
    env.mock_all_auths();
    let payout_key = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token = token_contract.address();
    let admin = Address::generate(&env);
    let contract_id = env.register_contract(None, crate::ProgramEscrowContract);
    let r1 = Address::generate(&env);
    let program_id = String::from_str(&env, "Disabled");

    env.as_contract(&contract_id, || {
        let program_data = ProgramData {
            program_id: program_id.clone(),
            total_funds: 1_000,
            remaining_balance: 1_000,
            authorized_payout_key: payout_key.clone(),
            payout_history: vec![&env],
            token_address: token.clone(),
            initial_liquidity: 0,
        };
        env.storage().instance().set(&PROGRAM_DATA, &program_data);
        env.storage().instance().set(&DataKey::Admin, &admin);

        let bens = vec![
            &env,
            BeneficiarySplit { recipient: r1.clone(), share_bps: 10_000 },
        ];
        set_split_config(&env, &program_id, bens);
        disable_split_config(&env, &program_id);
        execute_split_payout(&env, &program_id, 500);
    });
}

// ── preview_split ─────────────────────────────────────────────────────────────

#[test]
fn test_preview_split_no_transfer() {
    let env = Env::default();
    env.mock_all_auths();
    let payout_key = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token = token_contract.address();
    let contract_id = env.register_contract(None, crate::ProgramEscrowContract);
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    let program_id = String::from_str(&env, "Preview");

    env.as_contract(&contract_id, || {
        let program_data = ProgramData {
            program_id: program_id.clone(),
            total_funds: 1_000,
            remaining_balance: 1_000,
            authorized_payout_key: payout_key.clone(),
            payout_history: vec![&env],
            token_address: token.clone(),
            initial_liquidity: 0,
        };
        env.storage().instance().set(&PROGRAM_DATA, &program_data);

        let bens = vec![
            &env,
            BeneficiarySplit { recipient: r1.clone(), share_bps: 8_000 },
            BeneficiarySplit { recipient: r2.clone(), share_bps: 2_000 },
        ];
        set_split_config(&env, &program_id, bens);

        let preview = preview_split(&env, &program_id, 1_000);
        // share_bps field repurposed to hold computed amount
        assert_eq!(preview.get(0).unwrap().share_bps, 800);
        assert_eq!(preview.get(1).unwrap().share_bps, 200);

        // Balance must be unchanged (no transfers)
        let pd: ProgramData = env.storage().instance().get(&PROGRAM_DATA).unwrap();
        assert_eq!(pd.remaining_balance, 1_000);
    });
}

// ── Single-beneficiary edge case ─────────────────────────────────────────────

#[test]
fn test_single_beneficiary_receives_full_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let payout_key = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token = token_contract.address();
    let token_sac = token::StellarAssetClient::new(&env, &token);
    let contract_id = env.register_contract(None, crate::ProgramEscrowContract);
    token_sac.mint(&contract_id, &500i128);

    let r1 = Address::generate(&env);
    let program_id = String::from_str(&env, "Solo");

    env.as_contract(&contract_id, || {
        let program_data = ProgramData {
            program_id: program_id.clone(),
            total_funds: 500,
            remaining_balance: 500,
            authorized_payout_key: payout_key.clone(),
            payout_history: vec![&env],
            token_address: token.clone(),
            initial_liquidity: 0,
        };
        env.storage().instance().set(&PROGRAM_DATA, &program_data);

        let bens = vec![
            &env,
            BeneficiarySplit { recipient: r1.clone(), share_bps: 10_000 },
        ];
        set_split_config(&env, &program_id, bens);
        let result = execute_split_payout(&env, &program_id, 500);
        assert_eq!(result.total_distributed, 500);
        assert_eq!(result.remaining_balance, 0);
    });

    let tc = token::Client::new(&env, &token);
    assert_eq!(tc.balance(&r1), 500);
}