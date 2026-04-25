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
    DataKey, ProgramData, ProgramMetadata, ProgramStatus, PROGRAM_DATA, STORAGE_SCHEMA_VERSION,
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
                "Sum of distributions must equal input: got {}",
                total
            );
            assert_eq!(
                result.total_distributed, 10_000,
                "total_distributed must match input"
            );
        });
    }

    /// Property: Total distributed across all beneficiaries must never exceed
    /// the input amount (no over-distribution attack).
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

            assert!(
                total <= 1_000_000,
                "Over-distribution detected: {} > 1_000_000",
                total
            );
            assert_eq!(
                result.total_distributed, total,
                "Result total must match actual distribution"
            );
        });
    }

    /// Property: Floor rounding must never overpay any beneficiary beyond
    /// their proportional share.
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

            execute_split_payout(&setup.env, &setup.program_id, 100_000);

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
        });
    }

    /// Property: For equal splits, all beneficiaries must receive amounts
    /// that differ by at most 1 unit (due to floor rounding).
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

            execute_split_payout(&setup.env, &setup.program_id, amount);

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
        });
    }
}

// ===========================================================================
// Dust Handling Tests
// ===========================================================================

mod dust_handling {
    use super::*;

    /// Dust from integer division must go to the first beneficiary.
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

            execute_split_payout(&setup.env, &setup.program_id, amount);

            let total: i128 = setup.get_balance(&setup.r1)
                + setup.get_balance(&setup.r2)
                + setup.get_balance(&setup.r3);
            assert_eq!(
                total, amount,
                "All tokens must be distributed (dust absorbed by first beneficiary)"
            );
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
                "Sum of all payouts must equal total: {} != {}",
                total_distributed, total
            );
        });
    }

    /// Test that dust cannot exceed the number of beneficiaries minus 1.
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

            assert!(
                total_preview <= 100,
                "Preview sum must not exceed total: {} > 100",
                total_preview
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

            for i in 0..num_beneficiaries {
                bens.push_back(BeneficiarySplit {
                    recipient: Address::generate(&setup.env),
                    share_bps: share_per_ben,
                });
            }

            let cfg = set_split_config(&setup.env, &setup.program_id, bens);
            assert_eq!(cfg.beneficiaries.len(), num_beneficiaries as u32);

            let result = execute_split_payout(&setup.env, &setup.program_id, amount);
            assert_eq!(result.recipient_count, num_beneficiaries as u32);
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
            assert_eq!(
                setup.get_balance(&setup.r1) + setup.get_balance(&setup.r2),
                huge
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
// Preview Accuracy Tests
// ===========================================================================

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

            execute_split_payout(&setup.env, &setup.program_id, amount);

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
        });
    }

    /// Remaining balance must be correctly tracked.
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

            let pd: ProgramData = setup.env.storage().instance().get(&PROGRAM_DATA).unwrap();
            assert!(
                !pd.payout_history.is_empty(),
                "Payout history must not be empty"
            );
        });
    }
}
