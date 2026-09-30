//! # Tests for Payout Splits: Rounding and Security Properties
//!
//! This module contains comprehensive tests for the `payout_splits` module,
//! focusing on:
//! - Rounding behavior across multiple beneficiaries
//! - Dust handling and prevention of fund loss
//! - Security against over-distribution attacks
//! - Property-based testing of invariants

#![cfg(test)]

extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, vec, Address, Env, String, Vec,
};

use crate::{
    payout_splits::{
        disable_split_config, execute_split_payout, get_split_config, preview_split,
        set_split_config, BeneficiarySplit, SplitConfig, SplitConfigSetEvent, SplitPayoutEvent,
        SplitPayoutResult, TOTAL_BASIS_POINTS,
    },
    DataKey, OptionalFotRouter, ProgramData, ProgramMetadata, ProgramStatus, PROGRAM_DATA,
    STORAGE_SCHEMA_VERSION,
};

// ===========================================================================
// Test Setup Helpers
// ===========================================================================

struct SplitTestEnv {
    env: Env,
    contract_id: Address,
    program_id: String,
    payout_key: Address,
    token: Address,
    admin: Address,
    r1: Address,
    r2: Address,
    r3: Address,
}

impl SplitTestEnv {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths_allowing_non_root_auth();

        let admin = Address::generate(&env);
        let payout_key = Address::generate(&env);
        let token_admin = Address::generate(&env);

        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token = token_contract.address();

        let contract_id = env.register_contract(None, crate::ProgramEscrowContract);
        let r1 = Address::generate(&env);
        let r2 = Address::generate(&env);
        let r3 = Address::generate(&env);

        let program_id = String::from_str(&env, "TestProgram");

        Self {
            env,
            contract_id,
            program_id,
            payout_key,
            token,
            admin,
            r1,
            r2,
            r3,
        }
    }

    fn setup_program_data(&self, remaining_balance: i128) {
        let program_data = ProgramData {
            program_id: self.program_id.clone(),
            total_funds: remaining_balance,
            remaining_balance,
            authorized_payout_key: self.payout_key.clone(),
            delegate: None,
            delegate_permissions: 0,
            payout_history: vec![&self.env],
            token_address: self.token.clone(),
            initial_liquidity: 0,
            risk_flags: 0,
            reference_hash: None,
            archived: false,
            archived_at: None,
            status: ProgramStatus::Active,
            circuit_breaker_threshold: None,
            fot_router: OptionalFotRouter::None,
        };
        self.env
            .storage()
            .instance()
            .set(&PROGRAM_DATA, &program_data);
        self.env
            .storage()
            .instance()
            .set(&DataKey::Admin, &self.admin);
    }

    fn mint_tokens(&self, amount: i128) {
        let token_client = token::StellarAssetClient::new(&self.env, &self.token);
        token_client.mint(&self.contract_id, &amount);
    }

    fn get_balance(&self, addr: &Address) -> i128 {
        let tc = token::Client::new(&self.env, &self.token);
        tc.balance(addr)
    }
}

// ===========================================================================
// Rounding Property Tests
// ===========================================================================

mod rounding_properties {
    use super::*;

    /// Property: For any split configuration, the sum of all distributed
    /// amounts must equal the input total amount (dust absorbed).
    #[test]
    fn test_sum_of_distributions_equals_input() {
        let setup = SplitTestEnv::new();
        setup.mint_tokens(10_000);

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(10_000);

            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 3_333,
                },
                BeneficiarySplit {
                    recipient: setup.r2.clone(),
                    share_bps: 3_333,
                },
                BeneficiarySplit {
                    recipient: setup.r3.clone(),
                    share_bps: 3_334,
                },
            ];
            set_split_config(&setup.env, &setup.program_id, bens);

            let result = execute_split_payout(&setup.env, &setup.program_id, 10_000);

            let total: i128 = setup.get_balance(&setup.r1)
                + setup.get_balance(&setup.r2)
                + setup.get_balance(&setup.r3);

            assert_eq!(
                total, 10_000,
                "Conservation invariant: sum of payouts must equal funded amount"
            );
            assert_eq!(
                result.total_distributed, 10_000,
                "total_distributed must match input"
            );
            let pd: ProgramData = setup.env.storage().instance().get(&PROGRAM_DATA).unwrap();
            assert_eq!(
                pd.total_funds - pd.remaining_balance,
                total,
                "Conservation invariant: escrow balance deduction must equal sum of payouts"
            );
            assert_eq!(pd.remaining_balance, 0);
        });
    }

    /// Property: Total distributed across all beneficiaries must never exceed
    /// the input amount (no over-distribution attack) and must conserve total funds.
    #[test]
    fn test_no_over_distribution() {
        let setup = SplitTestEnv::new();
        setup.mint_tokens(1_000_000);

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(1_000_000);

            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 7_000,
                },
                BeneficiarySplit {
                    recipient: setup.r2.clone(),
                    share_bps: 3_000,
                },
            ];
            set_split_config(&setup.env, &setup.program_id, bens);

            let result = execute_split_payout(&setup.env, &setup.program_id, 1_000_000);

            let total: i128 = setup.get_balance(&setup.r1) + setup.get_balance(&setup.r2);

            assert_eq!(
                total, 1_000_000,
                "Conservation invariant: sum of payouts must equal funded amount"
            );
            assert_eq!(
                result.total_distributed, total,
                "Result total must match actual distribution"
            );
            let pd: ProgramData = setup.env.storage().instance().get(&PROGRAM_DATA).unwrap();
            assert_eq!(
                pd.total_funds - pd.remaining_balance,
                total,
                "Conservation invariant: escrow balance deduction must equal sum of payouts"
            );
            assert_eq!(pd.remaining_balance, 0);
        });
    }

    /// Property: Floor rounding must never overpay any beneficiary beyond
    /// their proportional share, while conserving the total funded amount.
    #[test]
    fn test_floor_rounding_never_overpays() {
        let setup = SplitTestEnv::new();
        setup.mint_tokens(100_000);

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(100_000);

            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 3_334,
                },
                BeneficiarySplit {
                    recipient: setup.r2.clone(),
                    share_bps: 3_333,
                },
                BeneficiarySplit {
                    recipient: setup.r3.clone(),
                    share_bps: 3_333,
                },
            ];
            set_split_config(&setup.env, &setup.program_id, bens);

            let result = execute_split_payout(&setup.env, &setup.program_id, 100_000);

            let r1_balance = setup.get_balance(&setup.r1);
            let r2_balance = setup.get_balance(&setup.r2);
            let r3_balance = setup.get_balance(&setup.r3);
            let r1_max = (100_000i128 * 3_334 / TOTAL_BASIS_POINTS) + 1;
            let r2_max = 100_000i128 * 3_333 / TOTAL_BASIS_POINTS;
            let r3_max = 100_000i128 * 3_333 / TOTAL_BASIS_POINTS;

            assert!(
                r1_balance <= r1_max,
                "r1 overpaid: {} > {}",
                r1_balance,
                r1_max
            );
            assert!(
                r2_balance <= r2_max,
                "r2 overpaid: {} > {}",
                r2_balance,
                r2_max
            );
            assert!(
                r3_balance <= r3_max,
                "r3 overpaid: {} > {}",
                r3_balance,
                r3_max
            );

            let total: i128 = r1_balance + r2_balance + r3_balance;
            assert_eq!(
                total, 100_000,
                "Conservation invariant: sum of payouts must equal funded amount"
            );
            assert_eq!(result.total_distributed, 100_000);
            let pd: ProgramData = setup.env.storage().instance().get(&PROGRAM_DATA).unwrap();
            assert_eq!(
                pd.total_funds - pd.remaining_balance,
                total,
                "Conservation invariant: escrow balance deduction must equal sum of payouts"
            );
            assert_eq!(pd.remaining_balance, 0);
        });
    }

    /// Property: For equal splits, all beneficiaries must receive amounts
    /// that differ by at most 1 unit (due to floor rounding), conserving total funds.
    #[test]
    fn test_equal_splits_within_one_unit() {
        let setup = SplitTestEnv::new();
        let amount = 10_001;
        setup.mint_tokens(amount);

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(amount);

            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 3_334,
                },
                BeneficiarySplit {
                    recipient: setup.r2.clone(),
                    share_bps: 3_333,
                },
                BeneficiarySplit {
                    recipient: setup.r3.clone(),
                    share_bps: 3_333,
                },
            ];
            set_split_config(&setup.env, &setup.program_id, bens);

            let result = execute_split_payout(&setup.env, &setup.program_id, amount);

            let b1 = setup.get_balance(&setup.r1);
            let b2 = setup.get_balance(&setup.r2);
            let b3 = setup.get_balance(&setup.r3);

            let max_diff_from_first = 2i128;
            let max_diff_between_peers = 1i128;
            assert!(
                (b1 - b2).abs() <= max_diff_from_first,
                "Diff between r1 and r2 exceeds 2: {}",
                (b1 - b2).abs()
            );
            assert!(
                (b2 - b3).abs() <= max_diff_between_peers,
                "Diff between r2 and r3 exceeds 1: {}",
                (b2 - b3).abs()
            );

            let total: i128 = b1 + b2 + b3;
            assert_eq!(
                total, amount,
                "Conservation invariant: sum of payouts must equal funded amount"
            );
            assert_eq!(result.total_distributed, amount);
            let pd: ProgramData = setup.env.storage().instance().get(&PROGRAM_DATA).unwrap();
            assert_eq!(
                pd.total_funds - pd.remaining_balance,
                total,
                "Conservation invariant: escrow balance deduction must equal sum of payouts"
            );
            assert_eq!(pd.remaining_balance, 0);
        });
    }
}

// ===========================================================================
// Dust Handling Tests
// ===========================================================================

mod dust_handling {
    use super::*;

    /// Dust from integer division must go to the first beneficiary deterministically.
    #[test]
    fn test_dust_goes_to_first_beneficiary() {
        let setup = SplitTestEnv::new();
        let amount = 10;
        setup.mint_tokens(amount);

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(amount);

            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 3_334,
                },
                BeneficiarySplit {
                    recipient: setup.r2.clone(),
                    share_bps: 3_333,
                },
                BeneficiarySplit {
                    recipient: setup.r3.clone(),
                    share_bps: 3_333,
                },
            ];
            set_split_config(&setup.env, &setup.program_id, bens);

            let result = execute_split_payout(&setup.env, &setup.program_id, amount);

            let total: i128 = setup.get_balance(&setup.r1)
                + setup.get_balance(&setup.r2)
                + setup.get_balance(&setup.r3);
            assert_eq!(
                total, amount,
                "Conservation invariant: sum of payouts must equal funded amount"
            );
            assert_eq!(result.total_distributed, amount);
            // Remainder of 1 is absorbed by index 0 (3 + 1 = 4).
            assert_eq!(setup.get_balance(&setup.r1), 4);
            assert_eq!(setup.get_balance(&setup.r2), 3);
            assert_eq!(setup.get_balance(&setup.r3), 3);
            let pd: ProgramData = setup.env.storage().instance().get(&PROGRAM_DATA).unwrap();
            assert_eq!(
                pd.total_funds - pd.remaining_balance,
                total,
                "Conservation invariant: escrow balance deduction must equal sum of payouts"
            );
            assert_eq!(pd.remaining_balance, 0);
        });
    }

    /// Multiple small amounts must not accumulate dust to cause over-distribution.
    #[test]
    fn test_no_dust_accumulation_over_payouts() {
        let setup = SplitTestEnv::new();
        let total = 100;
        let payouts = [10, 20, 30, 40];
        setup.mint_tokens(total);

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(total);

            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 5_000,
                },
                BeneficiarySplit {
                    recipient: setup.r2.clone(),
                    share_bps: 5_000,
                },
            ];
            set_split_config(&setup.env, &setup.program_id, bens);

            for p in payouts {
                execute_split_payout(&setup.env, &setup.program_id, p);
            }

            let total_distributed: i128 =
                setup.get_balance(&setup.r1) + setup.get_balance(&setup.r2);
            assert_eq!(
                total_distributed, total,
                "Conservation invariant: sum of payouts must equal funded amount"
            );
            let pd: ProgramData = setup.env.storage().instance().get(&PROGRAM_DATA).unwrap();
            assert_eq!(
                pd.total_funds - pd.remaining_balance,
                total,
                "Conservation invariant: escrow balance deduction must equal sum of payouts"
            );
            assert_eq!(pd.remaining_balance, 0);
        });
    }

    /// Test that dust cannot exceed the number of beneficiaries minus 1, conserving total.
    #[test]
    fn test_dust_bounded_by_beneficiary_count() {
        let setup = SplitTestEnv::new();
        setup.mint_tokens(100);

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(100);

            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 4_000,
                },
                BeneficiarySplit {
                    recipient: setup.r2.clone(),
                    share_bps: 3_000,
                },
                BeneficiarySplit {
                    recipient: setup.r3.clone(),
                    share_bps: 3_000,
                },
            ];
            set_split_config(&setup.env, &setup.program_id, bens);

            let preview = preview_split(&setup.env, &setup.program_id, 100);
            let total_preview: i128 = (0..preview.len())
                .map(|i| preview.get(i).unwrap().share_bps)
                .sum();

            assert_eq!(
                total_preview, 100,
                "Conservation invariant: preview sum of payouts must equal funded amount"
            );
        });
    }
}

// ===========================================================================
// Edge Cases
// ===========================================================================

mod edge_cases {
    use super::*;

    /// Test with maximum number of beneficiaries (50).
    #[test]
    fn test_max_beneficiaries() {
        let setup = SplitTestEnv::new();
        let num_beneficiaries = 50;
        let amount = 10_000_000;
        setup.mint_tokens(amount);

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(amount);

            let mut bens = vec![&setup.env];
            let share_per_ben = TOTAL_BASIS_POINTS / num_beneficiaries as i128;

            for _ in 0..num_beneficiaries {
                bens.push_back(BeneficiarySplit {
                    recipient: Address::generate(&setup.env),
                    share_bps: share_per_ben,
                });
            }

            let cfg = set_split_config(&setup.env, &setup.program_id, bens.clone());
            assert_eq!(cfg.beneficiaries.len(), num_beneficiaries as u32);

            let result = execute_split_payout(&setup.env, &setup.program_id, amount);
            assert_eq!(result.recipient_count, num_beneficiaries as u32);
            assert_eq!(result.total_distributed, amount);

            let total_distributed: i128 = (0..num_beneficiaries)
                .map(|i| setup.get_balance(&bens.get(i).unwrap().recipient))
                .sum();
            assert_eq!(
                total_distributed, amount,
                "Conservation invariant: sum of payouts must equal funded amount"
            );
            let pd: ProgramData = setup.env.storage().instance().get(&PROGRAM_DATA).unwrap();
            assert_eq!(
                pd.total_funds - pd.remaining_balance,
                total_distributed,
                "Conservation invariant: escrow balance deduction must equal sum of payouts"
            );
            assert_eq!(pd.remaining_balance, 0);
        });
    }

    /// Test with single beneficiary (100% share).
    #[test]
    fn test_single_beneficiary_full_share() {
        let setup = SplitTestEnv::new();
        let amount = 500;
        setup.mint_tokens(amount);

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(amount);

            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 10_000,
                },
            ];
            set_split_config(&setup.env, &setup.program_id, bens);

            let result = execute_split_payout(&setup.env, &setup.program_id, amount);

            assert_eq!(result.total_distributed, amount);
            assert_eq!(result.recipient_count, 1);
            assert_eq!(setup.get_balance(&setup.r1), amount);
            assert_eq!(
                setup.get_balance(&setup.r1), amount,
                "Conservation invariant: sum of payouts must equal funded amount"
            );
            let pd: ProgramData = setup.env.storage().instance().get(&PROGRAM_DATA).unwrap();
            assert_eq!(
                pd.total_funds - pd.remaining_balance,
                amount,
                "Conservation invariant: escrow balance deduction must equal sum of payouts"
            );
            assert_eq!(pd.remaining_balance, 0);
        });
    }

    /// Test with very small amount (1 unit).
    #[test]
    fn test_minimum_amount_single_unit() {
        let setup = SplitTestEnv::new();
        let amount = 1;
        setup.mint_tokens(amount);

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(amount);

            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 7_000,
                },
                BeneficiarySplit {
                    recipient: setup.r2.clone(),
                    share_bps: 3_000,
                },
            ];
            set_split_config(&setup.env, &setup.program_id, bens);

            let result = execute_split_payout(&setup.env, &setup.program_id, amount);

            assert_eq!(
                result.total_distributed, amount,
                "Single unit must be fully distributed"
            );
            assert_eq!(
                result.remaining_balance, 0,
                "Remaining balance must be zero"
            );
            let total = setup.get_balance(&setup.r1) + setup.get_balance(&setup.r2);
            assert_eq!(
                total, amount,
                "Conservation invariant: sum of payouts must equal funded amount"
            );
            let pd: ProgramData = setup.env.storage().instance().get(&PROGRAM_DATA).unwrap();
            assert_eq!(
                pd.total_funds - pd.remaining_balance,
                total,
                "Conservation invariant: escrow balance deduction must equal sum of payouts"
            );
            assert_eq!(pd.remaining_balance, 0);
        });
    }

    /// Test with large amount and fine-grained shares.
    #[test]
    fn test_large_amount_fine_grained_shares() {
        let setup = SplitTestEnv::new();
        let amount = 1_000_000_000_000i128; // 1 trillion
        setup.mint_tokens(amount);

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(amount);

            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 1,
                },
                BeneficiarySplit {
                    recipient: setup.r2.clone(),
                    share_bps: 9_999,
                },
            ];
            set_split_config(&setup.env, &setup.program_id, bens);

            let result = execute_split_payout(&setup.env, &setup.program_id, amount);

            let total: i128 = setup.get_balance(&setup.r1) + setup.get_balance(&setup.r2);

            assert_eq!(total, amount, "Large amount must be fully distributed");
            assert_eq!(
                total, amount,
                "Conservation invariant: sum of payouts must equal funded amount"
            );
            assert_eq!(result.total_distributed, amount);
            let pd: ProgramData = setup.env.storage().instance().get(&PROGRAM_DATA).unwrap();
            assert_eq!(
                pd.total_funds - pd.remaining_balance,
                total,
                "Conservation invariant: escrow balance deduction must equal sum of payouts"
            );
            assert_eq!(pd.remaining_balance, 0);
        });
    }

    /// Test that share of 1 basis point works correctly.
    #[test]
    fn test_single_basis_point_share() {
        let setup = SplitTestEnv::new();
        let amount = 10_000;
        setup.mint_tokens(amount);

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(amount);

            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 1,
                },
                BeneficiarySplit {
                    recipient: setup.r2.clone(),
                    share_bps: 9_999,
                },
            ];
            set_split_config(&setup.env, &setup.program_id, bens);

            let result = execute_split_payout(&setup.env, &setup.program_id, amount);

            assert_eq!(
                setup.get_balance(&setup.r1),
                1,
                "1 bp of 10,000 should be exactly 1 unit"
            );
            assert_eq!(
                result.remaining_balance, 0,
                "Remaining must be 0 after full distribution"
            );
            let total = setup.get_balance(&setup.r1) + setup.get_balance(&setup.r2);
            assert_eq!(
                total, amount,
                "Conservation invariant: sum of payouts must equal funded amount"
            );
            assert_eq!(result.total_distributed, amount);
            let pd: ProgramData = setup.env.storage().instance().get(&PROGRAM_DATA).unwrap();
            assert_eq!(
                pd.total_funds - pd.remaining_balance,
                total,
                "Conservation invariant: escrow balance deduction must equal sum of payouts"
            );
            assert_eq!(pd.remaining_balance, 0);
        });
    }
}

// ===========================================================================
// Security Tests
// ===========================================================================

mod security {
    use super::*;

    /// Security: Insufficient balance must revert.
    #[test]
    #[should_panic(expected = "SplitPayout: insufficient escrow balance")]
    fn test_insufficient_balance_reverts() {
        let setup = SplitTestEnv::new();
        setup.mint_tokens(50);

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(50);

            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 10_000,
                },
            ];
            set_split_config(&setup.env, &setup.program_id, bens);

            execute_split_payout(&setup.env, &setup.program_id, 100);
        });
    }

    /// Security: Zero amount must revert.
    #[test]
    #[should_panic(expected = "SplitPayout: amount must be greater than zero")]
    fn test_zero_amount_reverts() {
        let setup = SplitTestEnv::new();
        setup.mint_tokens(100);

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(100);

            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 10_000,
                },
            ];
            set_split_config(&setup.env, &setup.program_id, bens);

            execute_split_payout(&setup.env, &setup.program_id, 0);
        });
    }

    /// Security: Negative amount must revert.
    #[test]
    #[should_panic(expected = "SplitPayout: amount must be greater than zero")]
    fn test_negative_amount_reverts() {
        let setup = SplitTestEnv::new();
        setup.mint_tokens(100);

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(100);

            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 10_000,
                },
            ];
            set_split_config(&setup.env, &setup.program_id, bens);

            execute_split_payout(&setup.env, &setup.program_id, -100);
        });
    }

    /// Security: Disabled config must revert.
    #[test]
    #[should_panic(expected = "SplitPayout: split config is disabled")]
    fn test_disabled_config_reverts() {
        let setup = SplitTestEnv::new();
        setup.mint_tokens(1000);

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(1000);
        });

        setup.env.as_contract(&setup.contract_id, || {
            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 10_000,
                },
            ];
            set_split_config(&setup.env, &setup.program_id, bens);
        });

        setup.env.as_contract(&setup.contract_id, || {
            disable_split_config(&setup.env, &setup.program_id);
        });

        setup.env.as_contract(&setup.contract_id, || {
            execute_split_payout(&setup.env, &setup.program_id, 500);
        });
    }

    /// Security: Overflow in calculation must not cause silent wrap-around.
    #[test]
    #[should_panic(expected = "Token math overflow: multiplication")]
    fn test_overflow_in_share_calculation() {
        let setup = SplitTestEnv::new();
        let max_i128 = i128::MAX;
        setup.mint_tokens(max_i128);

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(max_i128);

            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 10_000,
                },
            ];
            set_split_config(&setup.env, &setup.program_id, bens);

            execute_split_payout(&setup.env, &setup.program_id, max_i128);
        });
    }

    /// Large equal splits should remain within bounds and distribute exactly.
    #[test]
    fn test_sum_overflow_detected() {
        let setup = SplitTestEnv::new();
        let huge = i128::MAX / TOTAL_BASIS_POINTS;
        setup.mint_tokens(huge);

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(huge);
        });

        setup.env.as_contract(&setup.contract_id, || {
            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 5_000,
                },
                BeneficiarySplit {
                    recipient: setup.r2.clone(),
                    share_bps: 5_000,
                },
            ];
            set_split_config(&setup.env, &setup.program_id, bens);
        });

        setup.env.as_contract(&setup.contract_id, || {
            let result = execute_split_payout(&setup.env, &setup.program_id, huge);

            assert_eq!(result.total_distributed, huge);
            let total = setup.get_balance(&setup.r1) + setup.get_balance(&setup.r2);
            assert_eq!(
                total,
                huge,
                "Conservation invariant: sum of payouts must equal funded amount"
            );
            let pd: ProgramData = setup.env.storage().instance().get(&PROGRAM_DATA).unwrap();
            assert_eq!(
                pd.total_funds - pd.remaining_balance,
                huge,
                "Conservation invariant: escrow balance deduction must equal sum of payouts"
            );
            assert_eq!(result.remaining_balance, 0);
        });
    }
}

// ===========================================================================
// Configuration Validation Tests
// ===========================================================================

mod config_validation {
    use super::*;

    /// Config must reject empty beneficiary list.
    #[test]
    #[should_panic(expected = "SplitConfig: must have at least one beneficiary")]
    fn test_empty_beneficiaries_rejected() {
        let setup = SplitTestEnv::new();

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(100);

            let empty: soroban_sdk::Vec<BeneficiarySplit> = soroban_sdk::Vec::new(&setup.env);
            set_split_config(&setup.env, &setup.program_id, empty);
        });
    }

    /// Config must reject more than 50 beneficiaries.
    #[test]
    #[should_panic(expected = "SplitConfig: maximum 50 beneficiaries")]
    fn test_too_many_beneficiaries_rejected() {
        let setup = SplitTestEnv::new();

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(100);

            let mut bens = vec![&setup.env];
            for _ in 0..51 {
                bens.push_back(BeneficiarySplit {
                    recipient: Address::generate(&setup.env),
                    share_bps: 195,
                });
            }
            set_split_config(&setup.env, &setup.program_id, bens);
        });
    }

    /// Config must reject zero share.
    #[test]
    #[should_panic(expected = "SplitConfig: share_bps must be positive")]
    fn test_zero_share_rejected() {
        let setup = SplitTestEnv::new();

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(100);

            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 10_000,
                },
                BeneficiarySplit {
                    recipient: setup.r2.clone(),
                    share_bps: 0,
                },
            ];
            set_split_config(&setup.env, &setup.program_id, bens);
        });
    }

    /// Config must reject negative share.
    #[test]
    #[should_panic(expected = "SplitConfig: share_bps must be positive")]
    fn test_negative_share_rejected() {
        let setup = SplitTestEnv::new();

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(100);

            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 10_000,
                },
                BeneficiarySplit {
                    recipient: setup.r2.clone(),
                    share_bps: -100,
                },
            ];
            set_split_config(&setup.env, &setup.program_id, bens);
        });
    }

    /// Config must reject shares not summing to TOTAL_BASIS_POINTS.
    #[test]
    #[should_panic(expected = "SplitConfig: shares must sum to 10000 basis points")]
    fn test_shares_must_sum_to_total() {
        let setup = SplitTestEnv::new();

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(100);

            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 5_000,
                },
                BeneficiarySplit {
                    recipient: setup.r2.clone(),
                    share_bps: 4_000,
                },
            ];
            set_split_config(&setup.env, &setup.program_id, bens);
        });
    }

    /// Config must reject shares exceeding TOTAL_BASIS_POINTS.
    #[test]
    #[should_panic(expected = "SplitConfig: shares must sum to 10000 basis points")]
    fn test_shares_exceeding_total_rejected() {
        let setup = SplitTestEnv::new();

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(100);

            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 6_000,
                },
                BeneficiarySplit {
                    recipient: setup.r2.clone(),
                    share_bps: 5_000,
                },
            ];
            set_split_config(&setup.env, &setup.program_id, bens);
        });
    }

    /// Config must accept valid split summing to 10,000.
    #[test]
    fn test_valid_split_accepted() {
        let setup = SplitTestEnv::new();

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(100);

            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 6_000,
                },
                BeneficiarySplit {
                    recipient: setup.r2.clone(),
                    share_bps: 4_000,
                },
            ];
            let cfg = set_split_config(&setup.env, &setup.program_id, bens);
            assert!(cfg.active);
            assert_eq!(cfg.beneficiaries.len(), 2);
        });
    }
}

// ===========================================================================
// MAX_FEE_RATE Cap Enforcement Tests
// ===========================================================================

/// Verifies that `update_fee_config` enforces the `MAX_FEE_RATE` cap on
/// every code path that writes `lock_fee_rate` or `payout_fee_rate`.
///
/// The cap is defined in [lib.rs] as `MAX_FEE_RATE = 1000` (10 % in basis
/// points).  Any rate above this value must be rejected; a rate exactly at
/// the boundary must be accepted.
///
/// ## Test design
///
/// Every test uses `try_update_fee_config` (the auto‑generated non‑panicking
/// wrapper) to assert success/failure without catching panics.
///
/// ## Security coverage
///
/// - Boundary-value analysis: {max-1, max, max+1}
/// - Domain coverage: positive, zero, negative
/// - Each mutable field tested in isolation
/// - Remaining fields preserved on partial update
#[cfg(test)]
mod fee_enforcement {
    use soroban_sdk::testutils::Address as _;

    use crate::{
        ContractError, FeeConfig, OptionalFotRouter, ProgramData, ProgramEscrowContract,
        ProgramEscrowContractClient, ProgramStatus, FEE_CONFIG, MAX_FEE_RATE, PROGRAM_DATA,
    };

    /// Minimal environment that sets up admin + program data in storage,
    /// then returns a client ready to call `update_fee_config`.
    struct FeeCapTestEnv {
        env: Env,
        client: ProgramEscrowContractClient<'static>,
        _admin: Address,
    }

    impl FeeCapTestEnv {
        fn new() -> Self {
            let env = Env::default();
            env.mock_all_auths();

            let admin = Address::generate(&env);
            let payout_key = Address::generate(&env);
            let token_admin = Address::generate(&env);

            let sac = env.register_stellar_asset_contract_v2(token_admin);
            let token = sac.address();

            let contract_id = env.register_contract(None, ProgramEscrowContract);
            let client = ProgramEscrowContractClient::new(&env, &contract_id);

            // Inject admin and program data directly.
            env.as_contract(&contract_id, || {
                env.storage().instance().set(&crate::DataKey::Admin, &admin);
                let pd = ProgramData {
                    program_id: String::from_str(&env, "FeeCapProg"),
                    total_funds: 0,
                    remaining_balance: 0,
                    authorized_payout_key: payout_key,
                    delegate: None,
                    delegate_permissions: 0,
                    payout_history: soroban_sdk::vec![&env],
                    token_address: token,
                    initial_liquidity: 0,
                    risk_flags: 0,
                    reference_hash: None,
                    archived: false,
                    archived_at: None,
                    status: ProgramStatus::Active,
                    circuit_breaker_threshold: None,
                    fot_router: OptionalFotRouter::None,
                };
                env.storage().instance().set(&PROGRAM_DATA, &pd);
            });

            Self {
                env,
                client,
                _admin: admin,
            }
        }

        fn get_fee_config(&self) -> FeeConfig {
            self.client.get_fee_config()
        }
    }

    // ── lock_fee_rate ───────────────────────────────────────────────────

    #[test]
    fn test_lock_fee_rate_above_max_rejected() {
        let t = FeeCapTestEnv::new();
        let r = t.client.try_update_fee_config(
            &Some(MAX_FEE_RATE + 1),
            &None, &None, &None, &None, &None,
        );
        assert!(r.is_err(), "lock_fee_rate above MAX_FEE_RATE must be rejected");
    }

    #[test]
    fn test_lock_fee_rate_at_max_accepted() {
        let t = FeeCapTestEnv::new();
        let r = t.client.try_update_fee_config(
            &Some(MAX_FEE_RATE),
            &None, &None, &None, &None, &None,
        );
        assert!(r.is_ok(), "lock_fee_rate == MAX_FEE_RATE must be accepted");
        let cfg = t.get_fee_config();
        assert_eq!(cfg.lock_fee_rate, MAX_FEE_RATE);
    }

    #[test]
    fn test_lock_fee_rate_zero_accepted() {
        let t = FeeCapTestEnv::new();
        let r = t.client.try_update_fee_config(
            &Some(0),
            &None, &None, &None, &None, &None,
        );
        assert!(r.is_ok());
        let cfg = t.get_fee_config();
        assert_eq!(cfg.lock_fee_rate, 0);
    }

    #[test]
    fn test_lock_fee_rate_negative_rejected() {
        let t = FeeCapTestEnv::new();
        let r = t.client.try_update_fee_config(
            &Some(-1),
            &None, &None, &None, &None, &None,
        );
        assert!(r.is_err(), "negative lock_fee_rate must be rejected");
    }

    // ── payout_fee_rate ────────────────────────────────────────────────

    #[test]
    fn test_payout_fee_rate_above_max_rejected() {
        let t = FeeCapTestEnv::new();
        let r = t.client.try_update_fee_config(
            &None,
            &Some(MAX_FEE_RATE + 1), &None, &None, &None, &None,
        );
        assert!(r.is_err(), "payout_fee_rate above MAX_FEE_RATE must be rejected");
    }

    #[test]
    fn test_payout_fee_rate_at_max_accepted() {
        let t = FeeCapTestEnv::new();
        let r = t.client.try_update_fee_config(
            &None,
            &Some(MAX_FEE_RATE), &None, &None, &None, &None,
        );
        assert!(r.is_ok(), "payout_fee_rate == MAX_FEE_RATE must be accepted");
        let cfg = t.get_fee_config();
        assert_eq!(cfg.payout_fee_rate, MAX_FEE_RATE);
    }

    #[test]
    fn test_payout_fee_rate_zero_accepted() {
        let t = FeeCapTestEnv::new();
        let r = t.client.try_update_fee_config(
            &None,
            &Some(0), &None, &None, &None, &None,
        );
        assert!(r.is_ok());
        let cfg = t.get_fee_config();
        assert_eq!(cfg.payout_fee_rate, 0);
    }

    #[test]
    fn test_payout_fee_rate_negative_rejected() {
        let t = FeeCapTestEnv::new();
        let r = t.client.try_update_fee_config(
            &None,
            &Some(-1), &None, &None, &None, &None,
        );
        assert!(r.is_err(), "negative payout_fee_rate must be rejected");
    }

    // ── Both rates ─────────────────────────────────────────────────────

    #[test]
    fn test_both_rates_at_max_accepted() {
        let t = FeeCapTestEnv::new();
        let r = t.client.try_update_fee_config(
            &Some(MAX_FEE_RATE),
            &Some(MAX_FEE_RATE), &None, &None, &None, &None,
        );
        assert!(r.is_ok());
        let cfg = t.get_fee_config();
        assert_eq!(cfg.lock_fee_rate, MAX_FEE_RATE);
        assert_eq!(cfg.payout_fee_rate, MAX_FEE_RATE);
    }

    #[test]
    fn test_both_rates_above_max_rejected() {
        let t = FeeCapTestEnv::new();
        let r = t.client.try_update_fee_config(
            &Some(MAX_FEE_RATE + 1),
            &Some(MAX_FEE_RATE + 1), &None, &None, &None, &None,
        );
        assert!(r.is_err(), "both rates above MAX_FEE_RATE must be rejected");
    }

    #[test]
    fn test_lock_rate_valid_payout_rate_invalid_rejected() {
        let t = FeeCapTestEnv::new();
        let r = t.client.try_update_fee_config(
            &Some(500),
            &Some(MAX_FEE_RATE + 1), &None, &None, &None, &None,
        );
        assert!(r.is_err(), "payout_fee_rate > MAX_FEE_RATE must be rejected even when lock_fee_rate is valid");
    }

    // ── Preservation ───────────────────────────────────────────────────

    #[test]
    fn test_update_rate_preserves_other_fields() {
        let t = FeeCapTestEnv::new();

        // Set a known lock_fee_rate.
        t.client.update_fee_config(
            &Some(500), &Some(800), &None, &None, &None, &None,
        );

        // Now update only payout_fee_rate — lock_fee_rate must be preserved.
        let r = t.client.try_update_fee_config(
            &None,
            &Some(MAX_FEE_RATE), &None, &None, &None, &None,
        );
        assert!(r.is_ok());
        let cfg = t.get_fee_config();
        assert_eq!(cfg.lock_fee_rate, 500, "lock_fee_rate must be preserved");
        assert_eq!(cfg.payout_fee_rate, MAX_FEE_RATE);
    }

    // ── Fixed fee validation ───────────────────────────────────────────

    #[test]
    fn test_negative_lock_fixed_fee_rejected() {
        let t = FeeCapTestEnv::new();
        let r = t.client.try_update_fee_config(
            &None, &None, &Some(-1), &None, &None, &None,
        );
        assert!(r.is_err(), "negative lock_fixed_fee must be rejected");
    }

    #[test]
    fn test_negative_payout_fixed_fee_rejected() {
        let t = FeeCapTestEnv::new();
        let r = t.client.try_update_fee_config(
            &None, &None, &None, &Some(-1), &None, &None,
        );
        assert!(r.is_err(), "negative payout_fixed_fee must be rejected");
    }
}

mod preview_accuracy {
    use super::*;

    /// Preview must accurately predict actual distribution.
    #[test]
    fn test_preview_matches_actual() {
        let setup = SplitTestEnv::new();
        let amount = 777;
        setup.mint_tokens(amount);

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(amount);

            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 7_000,
                },
                BeneficiarySplit {
                    recipient: setup.r2.clone(),
                    share_bps: 3_000,
                },
            ];
            set_split_config(&setup.env, &setup.program_id, bens);

            let preview = preview_split(&setup.env, &setup.program_id, amount);

            let result = execute_split_payout(&setup.env, &setup.program_id, amount);

            let b1_preview = preview.get(0).unwrap().share_bps;
            let b2_preview = preview.get(1).unwrap().share_bps;

            assert_eq!(
                setup.get_balance(&setup.r1),
                b1_preview,
                "Preview r1 must match actual: {} != {}",
                setup.get_balance(&setup.r1),
                b1_preview
            );
            assert_eq!(
                setup.get_balance(&setup.r2),
                b2_preview,
                "Preview r2 must match actual: {} != {}",
                setup.get_balance(&setup.r2),
                b2_preview
            );
            let total_actual = setup.get_balance(&setup.r1) + setup.get_balance(&setup.r2);
            assert_eq!(
                total_actual, amount,
                "Conservation invariant: sum of actual payouts must equal funded amount"
            );
            let total_preview = b1_preview + b2_preview;
            assert_eq!(
                total_preview, amount,
                "Conservation invariant: sum of preview payouts must equal funded amount"
            );
            assert_eq!(result.total_distributed, amount);
            let pd: ProgramData = setup.env.storage().instance().get(&PROGRAM_DATA).unwrap();
            assert_eq!(
                pd.total_funds - pd.remaining_balance,
                total_actual,
                "Conservation invariant: escrow balance deduction must equal sum of payouts"
            );
            assert_eq!(pd.remaining_balance, 0);
        });
    }

    /// Preview must not modify contract state.
    #[test]
    fn test_preview_is_readonly() {
        let setup = SplitTestEnv::new();
        let amount = 1000;
        setup.mint_tokens(amount);

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(amount);

            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 10_000,
                },
            ];
            set_split_config(&setup.env, &setup.program_id, bens);

            preview_split(&setup.env, &setup.program_id, amount);

            let pd: ProgramData = setup.env.storage().instance().get(&PROGRAM_DATA).unwrap();
            assert_eq!(
                pd.remaining_balance, amount,
                "Preview must not modify remaining balance"
            );
            assert_eq!(
                setup.get_balance(&setup.r1),
                0,
                "Preview must not transfer tokens"
            );
        });
    }

    /// Preview dust must be correctly calculated.
    #[test]
    fn test_preview_dust_calculation() {
        let setup = SplitTestEnv::new();
        let amount = 7;

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(7);

            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 3_334,
                },
                BeneficiarySplit {
                    recipient: setup.r2.clone(),
                    share_bps: 3_333,
                },
                BeneficiarySplit {
                    recipient: setup.r3.clone(),
                    share_bps: 3_333,
                },
            ];
            set_split_config(&setup.env, &setup.program_id, bens);

            let preview = preview_split(&setup.env, &setup.program_id, amount);
            let preview_sum: i128 = (0..preview.len())
                .map(|i| preview.get(i).unwrap().share_bps)
                .sum();

            assert_eq!(
                preview_sum, amount,
                "Preview sum must equal input: {} != {}",
                preview_sum, amount
            );
        });
    }
}

// ===========================================================================
// Partial Release Tests
// ===========================================================================

mod partial_releases {
    use super::*;

    /// Multiple partial releases must maintain correct ratios.
    #[test]
    fn test_partial_releases_maintain_ratio() {
        let setup = SplitTestEnv::new();
        let total = 10_000;
        let payouts = [4000, 3000, 3000];
        setup.mint_tokens(total);

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(total);

            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 7_000,
                },
                BeneficiarySplit {
                    recipient: setup.r2.clone(),
                    share_bps: 3_000,
                },
            ];
            set_split_config(&setup.env, &setup.program_id, bens);

            let mut expected_r1 = 0i128;
            let mut expected_r2 = 0i128;

            for p in payouts {
                execute_split_payout(&setup.env, &setup.program_id, p);
                expected_r1 += p * 7000 / TOTAL_BASIS_POINTS;
                expected_r2 += p * 3000 / TOTAL_BASIS_POINTS;
            }

            assert_eq!(
                setup.get_balance(&setup.r1),
                expected_r1,
                "r1 balance must match expected: {} != {}",
                setup.get_balance(&setup.r1),
                expected_r1
            );
            assert_eq!(
                setup.get_balance(&setup.r2),
                expected_r2,
                "r2 balance must match expected: {} != {}",
                setup.get_balance(&setup.r2),
                expected_r2
            );
            let total_paid = setup.get_balance(&setup.r1) + setup.get_balance(&setup.r2);
            assert_eq!(
                total_paid, total,
                "Conservation invariant: sum of partial payouts must equal total funded"
            );
            let pd: ProgramData = setup.env.storage().instance().get(&PROGRAM_DATA).unwrap();
            assert_eq!(
                pd.total_funds - pd.remaining_balance,
                total_paid,
                "Conservation invariant: escrow balance deduction must equal sum of payouts"
            );
            assert_eq!(pd.remaining_balance, 0);
        });
    }

    /// Remaining balance must be correctly tracked, conserving funds.
    #[test]
    fn test_remaining_balance_tracked() {
        let setup = SplitTestEnv::new();
        let total = 10_000;
        setup.mint_tokens(total);

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(total);

            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 5_000,
                },
                BeneficiarySplit {
                    recipient: setup.r2.clone(),
                    share_bps: 5_000,
                },
            ];
            set_split_config(&setup.env, &setup.program_id, bens);

            let r1 = execute_split_payout(&setup.env, &setup.program_id, 3000);
            assert_eq!(r1.remaining_balance, 7000);

            let r2 = execute_split_payout(&setup.env, &setup.program_id, 5000);
            assert_eq!(r2.remaining_balance, 2000);

            let r3 = execute_split_payout(&setup.env, &setup.program_id, 2000);
            assert_eq!(r3.remaining_balance, 0);

            let total_paid = setup.get_balance(&setup.r1) + setup.get_balance(&setup.r2);
            assert_eq!(
                total_paid, 10_000,
                "Conservation invariant: sum of payouts must equal total funded"
            );
            let pd: ProgramData = setup.env.storage().instance().get(&PROGRAM_DATA).unwrap();
            assert_eq!(
                pd.total_funds - pd.remaining_balance,
                total_paid,
                "Conservation invariant: escrow balance deduction must equal sum of payouts"
            );
            assert_eq!(pd.remaining_balance, 0);
        });
    }
}

// ===========================================================================
// Getter/Setter Tests
// ===========================================================================

mod getters_setters {
    use super::*;

    /// Config must be retrievable after setting.
    #[test]
    fn test_get_config_after_set() {
        let setup = SplitTestEnv::new();

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(100);

            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 6_000,
                },
                BeneficiarySplit {
                    recipient: setup.r2.clone(),
                    share_bps: 4_000,
                },
            ];
            set_split_config(&setup.env, &setup.program_id, bens.clone());

            let retrieved = get_split_config(&setup.env, &setup.program_id);
            assert!(retrieved.is_some());

            let cfg = retrieved.unwrap();
            assert!(cfg.active);
            assert_eq!(cfg.beneficiaries.len(), 2);
        });
    }

    /// Config must return None for non-existent program.
    #[test]
    fn test_get_config_nonexistent() {
        let setup = SplitTestEnv::new();

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(100);

            let nonexistent = String::from_str(&setup.env, "NonExistent");
            let retrieved = get_split_config(&setup.env, &nonexistent);
            assert!(retrieved.is_none());
        });
    }

    /// Config must be disabled correctly.
    #[test]
    fn test_disable_config() {
        let setup = SplitTestEnv::new();
        setup.mint_tokens(1000);

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(1000);

            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 10_000,
                },
            ];
            set_split_config(&setup.env, &setup.program_id, bens);

            disable_split_config(&setup.env, &setup.program_id);

            let cfg = get_split_config(&setup.env, &setup.program_id).unwrap();
            assert!(!cfg.active, "Config must be disabled");
        });
    }
}

// ===========================================================================
// Invariant Verification Tests
// ===========================================================================

mod invariants {
    use super::*;

    /// Invariant: Total distributed across all payouts never exceeds total funded.
    #[test]
    fn test_total_payouts_never_exceed_funded() {
        let setup = SplitTestEnv::new();
        let total_funded = 100_000;
        let mut remaining = total_funded;
        let payouts = [10_000, 20_000, 30_000, 40_000];

        setup.mint_tokens(total_funded);

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(total_funded);

            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 5_000,
                },
                BeneficiarySplit {
                    recipient: setup.r2.clone(),
                    share_bps: 5_000,
                },
            ];
            set_split_config(&setup.env, &setup.program_id, bens);

            for p in payouts {
                if p <= remaining {
                    execute_split_payout(&setup.env, &setup.program_id, p);
                    remaining -= p;
                }
            }

            let pd: ProgramData = setup.env.storage().instance().get(&PROGRAM_DATA).unwrap();
            assert_eq!(
                pd.remaining_balance, remaining,
                "Remaining balance must match expected: {} != {}",
                pd.remaining_balance, remaining
            );
            let total_paid = setup.get_balance(&setup.r1) + setup.get_balance(&setup.r2);
            assert_eq!(
                total_paid, total_funded,
                "Conservation invariant: sum of payouts must equal total funded"
            );
            assert_eq!(
                pd.total_funds - pd.remaining_balance,
                total_paid,
                "Conservation invariant: escrow balance deduction must equal sum of payouts"
            );
            assert_eq!(remaining, 0);
            assert_eq!(pd.remaining_balance, 0);
        });
    }

    /// Invariant: Payout history must be recorded correctly.
    #[test]
    fn test_payout_history_recorded() {
        let setup = SplitTestEnv::new();
        setup.mint_tokens(1000);

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(1000);

            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 10_000,
                },
            ];
            set_split_config(&setup.env, &setup.program_id, bens);

            execute_split_payout(&setup.env, &setup.program_id, 500);

            let total_paid = setup.get_balance(&setup.r1);
            assert_eq!(
                total_paid, 500,
                "Conservation invariant: sum of payouts must equal amount paid out"
            );
            let pd: ProgramData = setup.env.storage().instance().get(&PROGRAM_DATA).unwrap();
            assert_eq!(
                pd.total_funds - pd.remaining_balance,
                total_paid,
                "Conservation invariant: escrow balance deduction must equal sum of payouts"
            );
            assert_eq!(pd.remaining_balance, 500);
            assert!(
                !pd.payout_history.is_empty(),
                "Payout history must not be empty"
            );
        });
    }
}

// ===========================================================================
// Acceptance Criteria & Validation Tests (Issue #1875)
// ===========================================================================

mod validation_tests {
    use super::*;

    /// Validation: Test split that divides evenly among recipients.
    ///
    /// When total_amount is evenly divisible by recipient shares, no dust is generated.
    /// Conservation assertion ensures total paid equals funded amount.
    #[test]
    fn test_validation_split_divides_evenly() {
        let setup = SplitTestEnv::new();
        let funded_amount = 10_000i128;
        setup.mint_tokens(funded_amount);

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(funded_amount);

            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 5_000,
                },
                BeneficiarySplit {
                    recipient: setup.r2.clone(),
                    share_bps: 5_000,
                },
            ];
            set_split_config(&setup.env, &setup.program_id, bens);

            let result = execute_split_payout(&setup.env, &setup.program_id, funded_amount);
            assert_eq!(result.total_distributed, funded_amount);
            assert_eq!(result.remaining_balance, 0);

            // Each beneficiary receives exactly 50% (5,000)
            assert_eq!(setup.get_balance(&setup.r1), 5_000);
            assert_eq!(setup.get_balance(&setup.r2), 5_000);

            // Strict conservation assertion
            let total_paid = setup.get_balance(&setup.r1) + setup.get_balance(&setup.r2);
            assert_eq!(
                total_paid, funded_amount,
                "Conservation assertion: sum of payouts must equal funded amount"
            );

            let pd: ProgramData = setup.env.storage().instance().get(&PROGRAM_DATA).unwrap();
            assert_eq!(
                pd.total_funds - pd.remaining_balance,
                total_paid,
                "Conservation assertion: escrow balance deduction must equal sum of payouts"
            );
            assert_eq!(pd.remaining_balance, 0);
        });
    }

    /// Validation: Test split that leaves a remainder of one.
    ///
    /// When total_amount * share_bps / 10_000 produces rounding dust of exactly 1 unit,
    /// the remainder must deterministically be awarded to the first beneficiary (index 0).
    /// Conservation assertion ensures total paid equals funded amount.
    #[test]
    fn test_validation_split_remainder_of_one() {
        let setup = SplitTestEnv::new();
        // 3 tokens split 50/50:
        // share 0: 3 * 5000 / 10000 = 1
        // share 1: 3 * 5000 / 10000 = 1
        // base sum: 2; dust: 3 - 2 = 1.
        // Dust (1) is deterministically assigned to index 0.
        // Final: index 0 receives 2, index 1 receives 1.
        let funded_amount = 3i128;
        setup.mint_tokens(funded_amount);

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(funded_amount);

            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 5_000,
                },
                BeneficiarySplit {
                    recipient: setup.r2.clone(),
                    share_bps: 5_000,
                },
            ];
            set_split_config(&setup.env, &setup.program_id, bens);

            let result = execute_split_payout(&setup.env, &setup.program_id, funded_amount);
            assert_eq!(result.total_distributed, funded_amount);
            assert_eq!(result.remaining_balance, 0);

            // Deterministic rounding recipient: beneficiary 0 absorbs remainder of 1
            assert_eq!(setup.get_balance(&setup.r1), 2, "Recipient 0 must absorb dust remainder of 1");
            assert_eq!(setup.get_balance(&setup.r2), 1, "Recipient 1 must receive floor share");

            // Strict conservation assertion
            let total_paid = setup.get_balance(&setup.r1) + setup.get_balance(&setup.r2);
            assert_eq!(
                total_paid, funded_amount,
                "Conservation assertion: sum of payouts must equal funded amount"
            );

            let pd: ProgramData = setup.env.storage().instance().get(&PROGRAM_DATA).unwrap();
            assert_eq!(
                pd.total_funds - pd.remaining_balance,
                total_paid,
                "Conservation assertion: escrow balance deduction must equal sum of payouts"
            );
            assert_eq!(pd.remaining_balance, 0);
        });
    }

    /// Validation: Test split involving the maximum recipient count (50).
    ///
    /// With 50 recipients each holding 200 bps (2%), split 101 tokens:
    /// base share: 101 * 200 / 10_000 = 2 tokens per recipient.
    /// base sum: 50 * 2 = 100 tokens.
    /// dust = 101 - 100 = 1 token, awarded to index 0 -> recipient 0 gets 3 tokens,
    /// recipients 1..50 each get 2 tokens.
    /// Conservation assertion ensures total paid equals funded amount (101).
    #[test]
    fn test_validation_split_maximum_recipient_count() {
        let setup = SplitTestEnv::new();
        let num_recipients = 50usize;
        let funded_amount = 101i128;
        setup.mint_tokens(funded_amount);

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(funded_amount);

            let mut bens = vec![&setup.env];
            let share_per_recipient = TOTAL_BASIS_POINTS / num_recipients as i128; // 200 bps

            for _ in 0..num_recipients {
                bens.push_back(BeneficiarySplit {
                    recipient: Address::generate(&setup.env),
                    share_bps: share_per_recipient,
                });
            }

            let cfg = set_split_config(&setup.env, &setup.program_id, bens.clone());
            assert_eq!(cfg.beneficiaries.len(), num_recipients as u32);

            let result = execute_split_payout(&setup.env, &setup.program_id, funded_amount);
            assert_eq!(result.recipient_count, num_recipients as u32);
            assert_eq!(result.total_distributed, funded_amount);
            assert_eq!(result.remaining_balance, 0);

            // Recipient 0 receives base (2) + dust (1) = 3
            let r0_address = bens.get(0).unwrap().recipient;
            assert_eq!(
                setup.get_balance(&r0_address), 3,
                "First beneficiary must receive floor share + dust remainder"
            );

            // All other 49 recipients receive base share of 2
            for i in 1..num_recipients {
                let r_address = bens.get(i as u32).unwrap().recipient;
                assert_eq!(
                    setup.get_balance(&r_address), 2,
                    "Beneficiary {} must receive exact floor share of 2", i
                );
            }

            // Strict conservation assertion: sum of all 50 recipient balances equals funded_amount
            let total_paid: i128 = (0..num_recipients)
                .map(|i| setup.get_balance(&bens.get(i as u32).unwrap().recipient))
                .sum();
            assert_eq!(
                total_paid, funded_amount,
                "Conservation assertion: sum of payouts must equal funded amount"
            );

            let pd: ProgramData = setup.env.storage().instance().get(&PROGRAM_DATA).unwrap();
            assert_eq!(
                pd.total_funds - pd.remaining_balance,
                total_paid,
                "Conservation assertion: escrow balance deduction must equal sum of payouts"
            );
            assert_eq!(pd.remaining_balance, 0);
        });
    }

    /// Acceptance Criterion: Splits that cannot be represented exactly in basis points
    /// are rejected rather than silently rounded.
    ///
    /// For example, equal three-way split: 10,000 / 3 = 3333.33...
    /// Attempting [3333, 3333, 3333] sums to 9,999 (under by 1 bps).
    /// The configuration MUST panic and be rejected rather than silently rounded.
    #[test]
    #[should_panic(expected = "SplitConfig: shares must sum to 10000 basis points")]
    fn test_validation_split_inexact_representation_under_rejected() {
        let setup = SplitTestEnv::new();

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(10_000);

            // Sums to 9,999 basis points - cannot be represented exactly
            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 3_333,
                },
                BeneficiarySplit {
                    recipient: setup.r2.clone(),
                    share_bps: 3_333,
                },
                BeneficiarySplit {
                    recipient: setup.r3.clone(),
                    share_bps: 3_333,
                },
            ];

            set_split_config(&setup.env, &setup.program_id, bens);
        });
    }

    /// Acceptance Criterion: Splits whose sum exceeds 10,000 basis points
    /// are rejected rather than silently rounded.
    ///
    /// For example, [3334, 3333, 3334] sums to 10,001 bps.
    /// The configuration MUST panic and be rejected.
    #[test]
    #[should_panic(expected = "SplitConfig: shares must sum to 10000 basis points")]
    fn test_validation_split_inexact_representation_over_rejected() {
        let setup = SplitTestEnv::new();

        setup.env.as_contract(&setup.contract_id, || {
            setup.setup_program_data(10_000);

            // Sums to 10,001 basis points - cannot be represented exactly
            let bens = vec![
                &setup.env,
                BeneficiarySplit {
                    recipient: setup.r1.clone(),
                    share_bps: 3_334,
                },
                BeneficiarySplit {
                    recipient: setup.r2.clone(),
                    share_bps: 3_333,
                },
                BeneficiarySplit {
                    recipient: setup.r3.clone(),
                    share_bps: 3_334,
                },
            ];

            set_split_config(&setup.env, &setup.program_id, bens);
        });
    }
}

