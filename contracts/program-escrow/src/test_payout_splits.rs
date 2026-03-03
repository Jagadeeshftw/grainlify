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
            };
        env.storage().instance().set(&PROGRAM_DATA, &program_data);
        set_split_config(env, &s.program_id, bad);
    });
}

// ── execute_split_payout ──────────────────────────────────────────────────────
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
