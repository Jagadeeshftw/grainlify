//! # Recipient Payout Index Tests
//!
//! Validates the lazy-initialized inverted index introduced in issue #1384:
//! `DataKey::RecipientPayoutIndex(program_id, recipient)` → `Vec<PayoutRecord>`.
//!
//! ## Coverage
//! - Index populated by `single_payout` (unit + integration)
//! - Index populated by `batch_payout` (unit + integration)
//! - Lazy init: unknown recipient returns empty vec, not a panic
//! - Index is scoped per program_id (no cross-program leakage)
//! - Multiple payouts to same recipient accumulate in insertion order
//! - `query_recipient_history` vs legacy `query_payouts_by_recipient` return same records
//! - Idempotent replay does not duplicate index entries
//! - Pagination: index returns all records regardless of offset/limit boundaries

use crate::{DataKey, PayoutRecord, ProgramEscrowContract, ProgramEscrowContractClient};
use soroban_sdk::{testutils::Address as _, token, Address, Env, String, Vec};

// ─── setup helper ────────────────────────────────────────────────────────────

fn setup(env: &Env, balance: i128) -> (ProgramEscrowContractClient<'static>, Address) {
    env.mock_all_auths();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let token_admin = Address::generate(env);
    let sac = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = sac.address();
    let token_admin_client = token::StellarAssetClient::new(env, &token_id);

    let program_id = String::from_str(env, "prog-1");
    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    client.publish_program(&program_id, &admin);

    token_admin_client.mint(&client.address, &balance);
    client.lock_program_funds(&balance);

    (client, admin)
}

fn read_index(
    env: &Env,
    contract_id: &Address,
    program_id: &str,
    recipient: &Address,
) -> Vec<PayoutRecord> {
    env.as_contract(contract_id, || {
        let key =
            DataKey::RecipientPayoutIndex(String::from_str(env, program_id), recipient.clone());
        env.storage()
            .persistent()
            .get::<DataKey, Vec<PayoutRecord>>(&key)
            .unwrap_or(Vec::new(env))
    })
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod recipient_index_tests {
    use super::*;

    // ── 1. Lazy init: unknown recipient returns empty, no panic ───────────────

    #[test]
    fn test_unknown_recipient_returns_empty() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env, 100_000);

        let stranger = Address::generate(&env);
        let result = client.query_recipient_history(&String::from_str(&env, "prog-1"), &stranger);

        assert_eq!(result.len(), 0);
    }

    // ── 2. single_payout populates the index ─────────────────────────────────

    #[test]
    fn test_single_payout_writes_index() {
        let env = Env::default();
        let (client, _) = setup(&env, 100_000);

        let recipient = Address::generate(&env);
        client.single_payout(&recipient, &25_000_i128, &None);

        let index = client.query_recipient_history(&String::from_str(&env, "prog-1"), &recipient);

        assert_eq!(index.len(), 1);
        assert_eq!(index.get(0).unwrap().recipient, recipient);
        assert_eq!(index.get(0).unwrap().amount, 25_000);
    }

    // ── 3. Multiple single_payouts accumulate in order ────────────────────────

    #[test]
    fn test_single_payout_accumulates_in_order() {
        let env = Env::default();
        let (client, _) = setup(&env, 100_000);

        let recipient = Address::generate(&env);
        client.single_payout(&recipient, &10_000_i128, &None);
        client.single_payout(&recipient, &20_000_i128, &None);
        client.single_payout(&recipient, &30_000_i128, &None);

        let index = client.query_recipient_history(&String::from_str(&env, "prog-1"), &recipient);

        assert_eq!(index.len(), 3);
        assert_eq!(index.get(0).unwrap().amount, 10_000);
        assert_eq!(index.get(1).unwrap().amount, 20_000);
        assert_eq!(index.get(2).unwrap().amount, 30_000);
    }

    // ── 4. batch_payout populates index for each recipient ────────────────────

    #[test]
    fn test_batch_payout_writes_index_for_each_recipient() {
        let env = Env::default();
        let (client, _) = setup(&env, 100_000);

        let r1 = Address::generate(&env);
        let r2 = Address::generate(&env);
        let mut recipients = soroban_sdk::Vec::new(&env);
        let mut amounts = soroban_sdk::Vec::new(&env);
        recipients.push_back(r1.clone());
        recipients.push_back(r2.clone());
        amounts.push_back(15_000_i128);
        amounts.push_back(20_000_i128);

        client.batch_payout(&recipients, &amounts, &None);

        let prog = String::from_str(&env, "prog-1");

        let idx1 = client.query_recipient_history(&prog, &r1);
        assert_eq!(idx1.len(), 1);
        assert_eq!(idx1.get(0).unwrap().amount, 15_000);

        let idx2 = client.query_recipient_history(&prog, &r2);
        assert_eq!(idx2.len(), 1);
        assert_eq!(idx2.get(0).unwrap().amount, 20_000);
    }

    // ── 5. Mixed single + batch accumulate correctly ──────────────────────────

    #[test]
    fn test_single_and_batch_payout_accumulate() {
        let env = Env::default();
        let (client, _) = setup(&env, 100_000);

        let recipient = Address::generate(&env);

        client.single_payout(&recipient, &5_000_i128, &None);

        let mut recipients = soroban_sdk::Vec::new(&env);
        let mut amounts = soroban_sdk::Vec::new(&env);
        recipients.push_back(recipient.clone());
        amounts.push_back(7_000_i128);
        client.batch_payout(&recipients, &amounts, &None);

        let index = client.query_recipient_history(&String::from_str(&env, "prog-1"), &recipient);

        assert_eq!(index.len(), 2);
        assert_eq!(index.get(0).unwrap().amount, 5_000);
        assert_eq!(index.get(1).unwrap().amount, 7_000);
    }

    // ── 6. query_recipient_history agrees with query_payouts_by_recipient ─────

    #[test]
    fn test_index_matches_legacy_filtered_query() {
        let env = Env::default();
        let (client, _) = setup(&env, 100_000);

        let target = Address::generate(&env);
        let other = Address::generate(&env);

        client.single_payout(&target, &11_000_i128, &None);
        client.single_payout(&other, &22_000_i128, &None);
        client.single_payout(&target, &33_000_i128, &None);

        let from_index = client.query_recipient_history(&String::from_str(&env, "prog-1"), &target);
        let from_scan = client.query_payouts_by_recipient(&target, &0, &50).unwrap();

        assert_eq!(from_index.len(), from_scan.len());
        for i in 0..from_index.len() {
            assert_eq!(
                from_index.get(i).unwrap().amount,
                from_scan.get(i).unwrap().amount
            );
        }
    }

    // ── 7. Index is scoped per program_id — no cross-program leakage ──────────

    #[test]
    fn test_index_scoped_to_program_id() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let sac = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_id = sac.address();
        let sac_client = token::StellarAssetClient::new(&env, &token_id);

        for prog in ["prog-A", "prog-B"] {
            let pid = String::from_str(&env, prog);
            client.init_program(&pid, &admin, &token_id, &admin, &None, &None);
            client.publish_program(&pid, &admin);
        }

        let recipient = Address::generate(&env);

        sac_client.mint(&client.address, &100_000_i128);
        let pid_a = String::from_str(&env, "prog-A");
        let key_a = DataKey::RecipientPayoutIndex(pid_a.clone(), recipient.clone());
        let key_b =
            DataKey::RecipientPayoutIndex(String::from_str(&env, "prog-B"), recipient.clone());

        env.as_contract(&contract_id, || {
            let mut v: Vec<PayoutRecord> = Vec::new(&env);
            v.push_back(PayoutRecord {
                recipient: recipient.clone(),
                amount: 999,
                timestamp: 1,
            });
            env.storage().persistent().set(&key_a, &v);
        });

        let b_index: Vec<PayoutRecord> = env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .get::<DataKey, Vec<PayoutRecord>>(&key_b)
                .unwrap_or(Vec::new(&env))
        });

        assert_eq!(
            b_index.len(),
            0,
            "prog-B index must not be affected by prog-A write"
        );

        let a_index: Vec<PayoutRecord> = env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .get::<DataKey, Vec<PayoutRecord>>(&key_a)
                .unwrap_or(Vec::new(&env))
        });
        assert_eq!(a_index.len(), 1);
        assert_eq!(a_index.get(0).unwrap().amount, 999);
    }

    // ── 8. Other recipients not polluted by a payout ─────────────────────────

    #[test]
    fn test_unrelated_recipient_index_stays_empty() {
        let env = Env::default();
        let (client, _) = setup(&env, 50_000);

        let paid = Address::generate(&env);
        let unpaid = Address::generate(&env);

        client.single_payout(&paid, &10_000_i128, &None);

        let result = client.query_recipient_history(&String::from_str(&env, "prog-1"), &unpaid);
        assert_eq!(result.len(), 0);
    }

    // ── 9. Timestamps are recorded in index entries ───────────────────────────

    #[test]
    fn test_index_records_timestamp() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env, 50_000);

        env.ledger().set_timestamp(1_700_000_000);
        let recipient = Address::generate(&env);
        client.single_payout(&recipient, &5_000_i128, &None);

        let index = client.query_recipient_history(&String::from_str(&env, "prog-1"), &recipient);

        assert_eq!(index.get(0).unwrap().timestamp, 1_700_000_000);
    }

    // ── 10. Idempotent replay does not duplicate index entries ────────────────

    #[test]
    fn test_idempotent_replay_does_not_duplicate_index() {
        let env = Env::default();
        let (client, _) = setup(&env, 100_000);

        let recipient = Address::generate(&env);
        let key = String::from_str(&env, "idem-001");

        client.single_payout_idempotent(&recipient, &10_000_i128, &Some(key.clone()));
        let index_after_first =
            client.query_recipient_history(&String::from_str(&env, "prog-1"), &recipient);
        assert_eq!(index_after_first.len(), 1);

        client.single_payout_idempotent(&recipient, &10_000_i128, &Some(key.clone()));
        let index_after_replay =
            client.query_recipient_history(&String::from_str(&env, "prog-1"), &recipient);
        assert_eq!(
            index_after_replay.len(),
            1,
            "replay must not append duplicate entry"
        );
    }

    // ── 11. batch_payout_idempotent replay does not duplicate index ────────────

    #[test]
    fn test_batch_idempotent_replay_does_not_duplicate_index() {
        let env = Env::default();
        let (client, _) = setup(&env, 100_000);

        let recipient = Address::generate(&env);
        let key = String::from_str(&env, "batch-idem-001");
        let recipients = soroban_sdk::vec![&env, recipient.clone()];
        let amounts = soroban_sdk::vec![&env, 15_000_i128];

        client.batch_payout_idempotent(&key, &recipients, &amounts);
        let index_after_first =
            client.query_recipient_history(&String::from_str(&env, "prog-1"), &recipient);
        assert_eq!(index_after_first.len(), 1);

        client.batch_payout_idempotent(&key, &recipients, &amounts);
        let index_after_replay =
            client.query_recipient_history(&String::from_str(&env, "prog-1"), &recipient);
        assert_eq!(
            index_after_replay.len(),
            1,
            "batch replay must not append duplicate entry"
        );
    }

    // ── 12. Index returns ALL records regardless of pagination boundaries ─────

    #[test]
    fn test_index_returns_all_records_beyond_limit() {
        let env = Env::default();
        let (client, _) = setup(&env, 1_000_000);

        let recipient = Address::generate(&env);

        for i in 0..25 {
            client.single_payout(&recipient, &((i + 1) * 1000), &None);
        }

        let full_index =
            client.query_recipient_history(&String::from_str(&env, "prog-1"), &recipient);
        assert_eq!(full_index.len(), 25);

        let small_page = client
            .query_payouts_by_recipient(&recipient, &0, &10)
            .unwrap();
        assert_eq!(small_page.len(), 10);

        for i in 0..full_index.len() {
            let expected = ((i + 1) * 1000) as i128;
            assert_eq!(full_index.get(i).unwrap().amount, expected);
        }
    }

    // ── 13. Interleaved single + batch across multiple recipients ─────────────

    #[test]
    fn test_interleaved_payouts_across_recipients() {
        let env = Env::default();
        let (client, _) = setup(&env, 500_000);

        let r1 = Address::generate(&env);
        let r2 = Address::generate(&env);
        let r3 = Address::generate(&env);

        client.single_payout(&r1, &1000, &None);
        client.single_payout(&r2, &2000, &None);

        let batch_r = soroban_sdk::vec![&env, r1.clone(), r3.clone()];
        let batch_a = soroban_sdk::vec![&env, 3000_i128, 4000_i128];
        client.batch_payout(&batch_r, &batch_a, &None);

        client.single_payout(&r1, &5000, &None);

        let idx1 = client.query_recipient_history(&String::from_str(&env, "prog-1"), &r1);
        assert_eq!(idx1.len(), 3);
        assert_eq!(idx1.get(0).unwrap().amount, 1000);
        assert_eq!(idx1.get(1).unwrap().amount, 3000);
        assert_eq!(idx1.get(2).unwrap().amount, 5000);

        let idx2 = client.query_recipient_history(&String::from_str(&env, "prog-1"), &r2);
        assert_eq!(idx2.len(), 1);
        assert_eq!(idx2.get(0).unwrap().amount, 2000);

        let idx3 = client.query_recipient_history(&String::from_str(&env, "prog-1"), &r3);
        assert_eq!(idx3.len(), 1);
        assert_eq!(idx3.get(0).unwrap().amount, 4000);
    }

    // ── 14. Index survives multiple batch payouts to same recipient ───────────

    #[test]
    fn test_multiple_batches_to_same_recipient() {
        let env = Env::default();
        let (client, _) = setup(&env, 200_000);

        let recipient = Address::generate(&env);

        for batch_num in 0..4 {
            let recipients = soroban_sdk::vec![&env, recipient.clone()];
            let amounts = soroban_sdk::vec![&env, ((batch_num + 1) * 1000) as i128];
            client.batch_payout(&recipients, &amounts, &None);
        }

        let index = client.query_recipient_history(&String::from_str(&env, "prog-1"), &recipient);

        assert_eq!(index.len(), 4);
        assert_eq!(index.get(0).unwrap().amount, 1000);
        assert_eq!(index.get(1).unwrap().amount, 2000);
        assert_eq!(index.get(2).unwrap().amount, 3000);
        assert_eq!(index.get(3).unwrap().amount, 4000);
    }

    // ── 15. Interleave all four entrypoint types to same recipient ────────────

    #[test]
    fn test_all_entrypoints_interleaved_to_same_recipient() {
        let env = Env::default();
        let (client, _) = setup(&env, 500_000);

        let recipient = Address::generate(&env);
        let idem_a = String::from_str(&env, "idem-a");
        let idem_b = String::from_str(&env, "idem-b");

        // 1. single_payout
        client.single_payout(&recipient, &1000, &None);
        let index = client.query_recipient_history(&String::from_str(&env, "prog-1"), &recipient);
        assert_eq!(index.len(), 1);
        assert_eq!(index.get(0).unwrap().amount, 1000);

        // 2. batch_payout (same recipient)
        let batch_r = soroban_sdk::vec![&env, recipient.clone()];
        let batch_a = soroban_sdk::vec![&env, 2000_i128];
        client.batch_payout(&batch_r, &batch_a, &None);
        let index = client.query_recipient_history(&String::from_str(&env, "prog-1"), &recipient);
        assert_eq!(index.len(), 2);
        assert_eq!(index.get(0).unwrap().amount, 1000);
        assert_eq!(index.get(1).unwrap().amount, 2000);

        // 3. single_payout_idempotent (first use)
        client.single_payout_idempotent(&recipient, &3000, &Some(idem_a.clone()));
        let index = client.query_recipient_history(&String::from_str(&env, "prog-1"), &recipient);
        assert_eq!(index.len(), 3);
        assert_eq!(index.get(0).unwrap().amount, 1000);
        assert_eq!(index.get(1).unwrap().amount, 2000);
        assert_eq!(index.get(2).unwrap().amount, 3000);

        // 4. batch_payout_idempotent (first use)
        let batch_r = soroban_sdk::vec![&env, recipient.clone()];
        let batch_a = soroban_sdk::vec![&env, 4000_i128];
        client.batch_payout_idempotent(&idem_b, &batch_r, &batch_a);
        let index = client.query_recipient_history(&String::from_str(&env, "prog-1"), &recipient);
        assert_eq!(index.len(), 4);
        assert_eq!(index.get(0).unwrap().amount, 1000);
        assert_eq!(index.get(1).unwrap().amount, 2000);
        assert_eq!(index.get(2).unwrap().amount, 3000);
        assert_eq!(index.get(3).unwrap().amount, 4000);

        // 5. Replay single_payout_idempotent — must NOT add a duplicate
        client.single_payout_idempotent(&recipient, &3000, &Some(idem_a.clone()));
        let index = client.query_recipient_history(&String::from_str(&env, "prog-1"), &recipient);
        assert_eq!(index.len(), 4, "idempotent replay must not duplicate index");

        // 6. Replay batch_payout_idempotent — must NOT add a duplicate
        let batch_r = soroban_sdk::vec![&env, recipient.clone()];
        let batch_a = soroban_sdk::vec![&env, 4000_i128];
        client.batch_payout_idempotent(&idem_b, &batch_r, &batch_a);
        let index = client.query_recipient_history(&String::from_str(&env, "prog-1"), &recipient);
        assert_eq!(index.len(), 4, "batch idempotent replay must not duplicate index");

        // 7. Another fresh single_payout after idempotent replay
        client.single_payout(&recipient, &5000, &None);
        let index = client.query_recipient_history(&String::from_str(&env, "prog-1"), &recipient);
        assert_eq!(index.len(), 5);
        assert_eq!(index.get(4).unwrap().amount, 5000);
    }
}
