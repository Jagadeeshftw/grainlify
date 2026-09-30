// ============================================================================
// Tests for Multi-Token Balance Invariants (Issue #960)
//
// Covers:
//   INV-1  Per-Escrow Sanity
//   INV-2  Aggregate-to-Ledger
//   INV-4  Refund Consistency
//   INV-5  Index Completeness
//   Property-style fund conservation tests
// ============================================================================

use crate::{
    multitoken_invariants::{
        check_all_invariants, check_anon_escrow_sanity, check_anon_refund_consistency,
        check_escrow_sanity, check_refund_consistency, count_orphaned_index_entries,
        get_contract_token_balance, sum_active_escrow_balances,
    },
    AnonymousEscrow, BountyEscrowContract, BountyEscrowContractClient, DataKey, Escrow,
    EscrowStatus, RefundMode, RefundRecord,
};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, vec, Address, BytesN, Env, Vec,
};

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

struct TestEnv {
    env: Env,
    contract_id: Address,
    client: BountyEscrowContractClient<'static>,
    admin: Address,
    token_id: Address,
}

impl TestEnv {
    fn setup() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, BountyEscrowContract);

        // SAFETY: we extend the lifetime to 'static so the client can live
        // inside the struct alongside `env`.  This is safe because both are
        // owned by the same `TestEnv` and are dropped together.
        let client = BountyEscrowContractClient::new(
            unsafe { &*(&env as *const Env) },
            &contract_id,
        );

        let admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract(admin.clone());

        client.init(&admin, &token_id);

        TestEnv {
            env,
            contract_id,
            client,
            admin,
            token_id,
        }
    }

    /// Mint `amount` tokens to `to`.
    fn mint(&self, to: &Address, amount: i128) {
        let token_admin =
            token::StellarAssetClient::new(&self.env, &self.token_id);
        token_admin.mint(to, &amount);
    }

    /// Advance ledger timestamp by `delta` seconds.
    fn advance_time(&self, delta: u64) {
        let now = self.env.ledger().timestamp();
        self.env.ledger().set_timestamp(now + delta);
    }

    /// Return actual contract token balance.
    fn contract_balance(&self) -> i128 {
        get_contract_token_balance(&self.env)
    }

    /// Return sum of all active escrow remaining amounts.
    fn sum_remaining(&self) -> i128 {
        sum_active_escrow_balances(&self.env)
    }

    /// Deadline safely in the future.
    fn future_deadline(&self) -> u64 {
        self.env.ledger().timestamp() + 86_400
    }

    /// Disable the InvOff guard so we can write bad state manually in tests.
    fn disable_invariant_guard(&self) {
        self.env
            .storage()
            .instance()
            .set(&soroban_sdk::Symbol::new(&self.env, "InvOff"), &true);
    }
}

// ============================================================
// INV-1: Per-Escrow Sanity — check_escrow_sanity
// ============================================================

#[test]
fn test_inv1_valid_locked_escrow_passes() {
    let env = Env::default();
    let escrow = Escrow {
        depositor: Address::generate(&env),
        amount: 1_000,
        remaining_amount: 1_000,
        status: EscrowStatus::Locked,
        deadline: 999,
        refund_history: soroban_sdk::Vec::new(&Env::default()),
        creation_timestamp: 0,
        expiry: 0,
        archived: false,
        archived_at: None,
    };
    assert!(check_escrow_sanity(&escrow));
}

#[test]
fn test_inv1_negative_amount_fails() {
    let env = Env::default();
    let escrow = Escrow {
        depositor: Address::generate(&env),
        amount: -1,
        remaining_amount: 0,
        status: EscrowStatus::Locked,
        deadline: 999,
        refund_history: soroban_sdk::Vec::new(&Env::default()),
        creation_timestamp: 0,
        expiry: 0,
        archived: false,
        archived_at: None,
    };
    assert!(!check_escrow_sanity(&escrow));
}

#[test]
fn test_inv1_remaining_exceeds_amount_fails() {
    let env = Env::default();
    let escrow = Escrow {
        depositor: Address::generate(&env),
        amount: 500,
        remaining_amount: 501,
        status: EscrowStatus::Locked,
        deadline: 999,
        refund_history: soroban_sdk::Vec::new(&Env::default()),
        creation_timestamp: 0,
        expiry: 0,
        archived: false,
        archived_at: None,
    };
    assert!(!check_escrow_sanity(&escrow));
}

#[test]
fn test_inv1_released_with_nonzero_remaining_fails() {
    let env = Env::default();
    let escrow = Escrow {
        depositor: Address::generate(&env),
        amount: 1_000,
        remaining_amount: 1,
        status: EscrowStatus::Released,
        deadline: 999,
        refund_history: soroban_sdk::Vec::new(&Env::default()),
        creation_timestamp: 0,
        expiry: 0,
        archived: false,
        archived_at: None,
    };
    assert!(!check_escrow_sanity(&escrow));
}

#[test]
fn test_inv1_refunded_with_nonzero_remaining_fails() {
    let env = Env::default();
    let escrow = Escrow {
        depositor: Address::generate(&env),
        amount: 1_000,
        remaining_amount: 500,
        status: EscrowStatus::Refunded,
        deadline: 999,
        refund_history: soroban_sdk::Vec::new(&Env::default()),
        creation_timestamp: 0,
        expiry: 0,
        archived: false,
        archived_at: None,
    };
    assert!(!check_escrow_sanity(&escrow));
}

#[test]
fn test_inv1_released_with_zero_remaining_passes() {
    let env = Env::default();
    let escrow = Escrow {
        depositor: Address::generate(&env),
        amount: 1_000,
        remaining_amount: 0,
        status: EscrowStatus::Released,
        deadline: 9_999_999,
        refund_history: vec![&env],
        archived: false,
        archived_at: None,
    };
    assert!(check_escrow_sanity(&escrow));
}

#[test]
fn test_inv1_refunded_with_zero_remaining_passes() {
    let env = Env::default();
    let escrow = Escrow {
        depositor: Address::generate(&env),
        amount: 1_000,
        remaining_amount: 0,
        status: EscrowStatus::Refunded,
        deadline: 9_999_999,
        refund_history: vec![&env],
        archived: false,
        archived_at: None,
    };
    assert!(check_escrow_sanity(&escrow));
}

#[test]
fn test_inv1_partially_refunded_with_partial_remaining_passes() {
    let env = Env::default();
    let escrow = Escrow {
        depositor: Address::generate(&env),
        amount: 1_000,
        remaining_amount: 400,
        status: EscrowStatus::PartiallyRefunded,
        deadline: 9_999_999,
        refund_history: vec![&env],
        archived: false,
        archived_at: None,
    };
    assert!(check_escrow_sanity(&escrow));
}

// --- Anonymous escrow INV-1 ---

#[test]
fn test_inv1_anon_valid_passes() {
    let env = Env::default();
    let anon = AnonymousEscrow {
        depositor_commitment: BytesN::from_array(&env, &[0u8; 32]),
        amount: 1_000,
        remaining_amount: 1_000,
        status: EscrowStatus::Locked,
        deadline: 9_999_999,
        refund_history: vec![&env],
        archived: false,
        archived_at: None,
    };
    assert!(check_anon_escrow_sanity(&anon));
}

#[test]
fn test_inv1_anon_negative_amount_fails() {
    let env = Env::default();
    let anon = AnonymousEscrow {
        depositor_commitment: BytesN::from_array(&env, &[0u8; 32]),
        amount: -100,
        remaining_amount: 0,
        status: EscrowStatus::Locked,
        deadline: 9_999_999,
        refund_history: vec![&env],
        archived: false,
        archived_at: None,
    };
    assert!(!check_anon_escrow_sanity(&anon));
}

#[test]
fn test_inv1_anon_released_nonzero_remaining_fails() {
    let env = Env::default();
    let anon = AnonymousEscrow {
        depositor_commitment: BytesN::from_array(&env, &[0u8; 32]),
        amount: 1_000,
        remaining_amount: 100,
        status: EscrowStatus::Released,
        deadline: 9_999_999,
        refund_history: vec![&env],
        archived: false,
        archived_at: None,
    };
    assert!(!check_anon_escrow_sanity(&anon));
}

#[test]
fn test_inv1_anon_refunded_nonzero_remaining_fails() {
    let env = Env::default();
    let anon = AnonymousEscrow {
        depositor_commitment: BytesN::from_array(&env, &[0u8; 32]),
        amount: 1_000,
        remaining_amount: 1,
        status: EscrowStatus::Refunded,
        deadline: 9_999_999,
        refund_history: vec![&env],
        archived: false,
        archived_at: None,
    };
    assert!(!check_anon_escrow_sanity(&anon));
}

// ============================================================
// INV-2: Aggregate-to-Ledger
// ============================================================

#[test]
fn test_inv2_single_lock_invariant_holds() {
    let t = TestEnv::setup();
    let depositor = Address::generate(&t.env);
    t.mint(&depositor, 5_000);

    t.client
        .lock_funds(&depositor, &1u64, &5_000, &t.future_deadline());

    assert_eq!(t.sum_remaining(), t.contract_balance());
}

#[test]
fn test_inv2_multiple_locks_invariant_holds() {
    let t = TestEnv::setup();

    for i in 1u64..=5 {
        let depositor = Address::generate(&t.env);
        t.mint(&depositor, 1_000);
        t.client
            .lock_funds(&depositor, &i, &1_000, &t.future_deadline());
    }

    assert_eq!(t.sum_remaining(), t.contract_balance());
    assert_eq!(t.contract_balance(), 5_000);
}

#[test]
fn test_inv2_lock_then_publish_invariant_holds() {
    let t = TestEnv::setup();
    let depositor = Address::generate(&t.env);
    t.mint(&depositor, 2_000);

    t.client
        .lock_funds(&depositor, &1u64, &2_000, &t.future_deadline());
    t.client.publish(&1u64);

    assert_eq!(t.sum_remaining(), t.contract_balance());
}

#[test]
fn test_inv2_lock_then_release_invariant_holds() {
    let t = TestEnv::setup();
    let depositor = Address::generate(&t.env);
    let contributor = Address::generate(&t.env);
    t.mint(&depositor, 3_000);

    t.client
        .lock_funds(&depositor, &1u64, &3_000, &t.future_deadline());
    t.client.publish(&1u64);
    t.client.release_funds(&1u64, &contributor);

    // After release the escrow is no longer active; sum should be 0.
    assert_eq!(t.sum_remaining(), 0);
    assert_eq!(t.contract_balance(), 0);
    assert_eq!(t.sum_remaining(), t.contract_balance());
}

#[test]
fn test_inv2_lock_then_refund_invariant_holds() {
    let t = TestEnv::setup();
    let depositor = Address::generate(&t.env);
    t.mint(&depositor, 4_000);

    let deadline = t.env.ledger().timestamp() + 100;
    t.client
        .lock_funds(&depositor, &1u64, &4_000, &deadline);
    t.client.publish(&1u64);

    // Advance past deadline so refund is eligible.
    t.advance_time(200);

    t.client.refund(&1u64);

    assert_eq!(t.sum_remaining(), 0);
    assert_eq!(t.contract_balance(), 0);
    assert_eq!(t.sum_remaining(), t.contract_balance());
}

#[test]
fn test_inv2_partial_release_invariant_holds() {
    let t = TestEnv::setup();
    let depositor = Address::generate(&t.env);
    let contributor = Address::generate(&t.env);
    t.mint(&depositor, 6_000);

    t.client
        .lock_funds(&depositor, &1u64, &6_000, &t.future_deadline());
    t.client.publish(&1u64);
    t.client.partial_release(&1u64, &contributor, &2_000);

    assert_eq!(t.sum_remaining(), t.contract_balance());
    assert_eq!(t.contract_balance(), 4_000);
}

#[test]
fn test_inv2_multiple_partial_releases_invariant_holds() {
    let t = TestEnv::setup();
    let depositor = Address::generate(&t.env);
    let contributor = Address::generate(&t.env);
    t.mint(&depositor, 9_000);

    t.client
        .lock_funds(&depositor, &1u64, &9_000, &t.future_deadline());
    t.client.publish(&1u64);

    for _ in 0..3 {
        t.client.partial_release(&1u64, &contributor, &1_000);
        assert_eq!(t.sum_remaining(), t.contract_balance());
    }
}

#[test]
fn test_inv2_all_released_contract_empty() {
    let t = TestEnv::setup();

    for i in 1u64..=3 {
        let depositor = Address::generate(&t.env);
        let contributor = Address::generate(&t.env);
        t.mint(&depositor, 1_000);

        t.client
            .lock_funds(&depositor, &i, &1_000, &t.future_deadline());
        t.client.publish(&i);
        t.client.release_funds(&i, &contributor);
    }

    assert_eq!(t.sum_remaining(), 0);
    assert_eq!(t.contract_balance(), 0);
}

#[test]
fn test_inv2_mixed_active_and_released_escrows() {
    let t = TestEnv::setup();
    let contributor = Address::generate(&t.env);

    // Lock three escrows.
    for i in 1u64..=3 {
        let depositor = Address::generate(&t.env);
        t.mint(&depositor, 1_000);
        t.client
            .lock_funds(&depositor, &i, &1_000, &t.future_deadline());
        t.client.publish(&i);
    }

    // Release only the first.
    t.client.release_funds(&1u64, &contributor);

    // Sum of active (2 + 3) should match balance (2_000).
    assert_eq!(t.sum_remaining(), 2_000);
    assert_eq!(t.contract_balance(), 2_000);
    assert_eq!(t.sum_remaining(), t.contract_balance());
}

#[test]
fn test_inv2_batch_lock_invariant_holds() {
    use crate::LockFundsItem;

    let t = TestEnv::setup();
    let deadline = t.future_deadline();
    let mut items = Vec::new(&t.env);

    for i in 1u64..=5 {
        let depositor = Address::generate(&t.env);
        t.mint(&depositor, 500);
        items.push_back(LockFundsItem {
            bounty_id: i,
            depositor,
            amount: 500,
            deadline,
        });
    }

    t.client.batch_lock_funds(&items);

    assert_eq!(t.sum_remaining(), t.contract_balance());
    assert_eq!(t.contract_balance(), 2_500);
}

#[test]
fn test_inv2_batch_release_invariant_holds() {
    use crate::{LockFundsItem, ReleaseFundsItem};

    let t = TestEnv::setup();
    let deadline = t.future_deadline();
    let mut lock_items = Vec::new(&t.env);

    for i in 1u64..=3 {
        let depositor = Address::generate(&t.env);
        t.mint(&depositor, 1_000);
        lock_items.push_back(LockFundsItem {
            bounty_id: i,
            depositor,
            amount: 1_000,
            deadline,
        });
    }
    t.client.batch_lock_funds(&lock_items);

    // Publish all.
    for i in 1u64..=3 {
        t.client.publish(&i);
    }

    let contributor = Address::generate(&t.env);
    let mut release_items = Vec::new(&t.env);
    for i in 1u64..=3 {
        release_items.push_back(ReleaseFundsItem {
            bounty_id: i,
            contributor: contributor.clone(),
        });
    }
    t.client.batch_release_funds(&release_items);

    assert_eq!(t.sum_remaining(), 0);
    assert_eq!(t.contract_balance(), 0);
}

#[test]
fn test_inv2_tampered_balance_detected_by_invariant() {
    let t = TestEnv::setup();
    let depositor = Address::generate(&t.env);
    t.mint(&depositor, 1_000);

    t.client
        .lock_funds(&depositor, &1u64, &1_000, &t.future_deadline());

    // Manually write a corrupt escrow with inflated remaining_amount.
    t.disable_invariant_guard();
    let mut escrow: Escrow = t
        .env
        .storage()
        .persistent()
        .get(&DataKey::Escrow(1u64))
        .unwrap();
    escrow.remaining_amount = 9_999; // doesn't match actual token balance
    crate::BountyEscrowContract::write_escrow(&t.env, 1u64, &escrow).unwrap();

    // INV-2 should now detect the mismatch.
    let report = check_all_invariants(&t.env);
    assert!(!report.healthy);
    assert_ne!(report.sum_remaining, report.token_balance);
}

// ============================================================
// INV-4: Refund Consistency
// ============================================================

#[test]
fn test_inv4_no_refund_history_is_consistent() {
    let env = Env::default();
    let escrow = Escrow {
        depositor: Address::generate(&env),
        amount: 1_000,
        remaining_amount: 1_000,
        status: EscrowStatus::Locked,
        deadline: 999,
        refund_history: soroban_sdk::Vec::new(&Env::default()),
        creation_timestamp: 0,
        expiry: 0,
        archived: false,
        archived_at: None,
    };
    assert!(check_refund_consistency(&escrow));
}

#[test]
fn test_inv4_refund_history_equals_consumed_passes() {
    let env = Env::default();
    let depositor = Address::generate(&env);
    let mut history = Vec::new(&env);
    history.push_back(RefundRecord {
        amount: 300,
        recipient: depositor.clone(),
        timestamp: 0,
        mode: RefundMode::Partial,
    });
    let escrow = Escrow {
        depositor: depositor.clone(),
        amount: 1_000,
        remaining_amount: 700, // consumed = 300
        status: EscrowStatus::PartiallyRefunded,
        deadline: 9_999_999,
        refund_history: history,
        archived: false,
        archived_at: None,
    };
    assert!(check_refund_consistency(&escrow));
}

#[test]
fn test_inv4_refund_history_less_than_consumed_passes() {
    // Partial consumed via release (not refund), so history < consumed.
    let env = Env::default();
    let depositor = Address::generate(&env);
    let mut history = Vec::new(&env);
    history.push_back(RefundRecord {
        amount: 100,
        recipient: depositor.clone(),
        timestamp: 0,
        mode: RefundMode::Partial,
    });
    let escrow = Escrow {
        depositor: depositor.clone(),
        amount: 1_000,
        remaining_amount: 500, // consumed = 500, refunded = 100 < 500
        status: EscrowStatus::PartiallyRefunded,
        deadline: 9_999_999,
        refund_history: history,
        archived: false,
        archived_at: None,
    };
    assert!(check_refund_consistency(&escrow));
}

#[test]
fn test_inv4_refund_history_exceeds_consumed_fails() {
    let env = Env::default();
    let depositor = Address::generate(&env);
    let mut history = Vec::new(&env);
    history.push_back(RefundRecord {
        amount: 800,
        recipient: depositor.clone(),
        timestamp: 0,
        mode: RefundMode::Full,
    });
    let escrow = Escrow {
        depositor: depositor.clone(),
        amount: 1_000,
        remaining_amount: 700, // consumed = 300, but history says 800
        status: EscrowStatus::PartiallyRefunded,
        deadline: 9_999_999,
        refund_history: history,
        archived: false,
        archived_at: None,
    };
    assert!(!check_refund_consistency(&escrow));
}

#[test]
fn test_inv4_negative_refund_record_fails() {
    let env = Env::default();
    let depositor = Address::generate(&env);
    let mut history = Vec::new(&env);
    history.push_back(RefundRecord {
        amount: -50,
        recipient: depositor.clone(),
        timestamp: 0,
        mode: RefundMode::Partial,
    });
    let escrow = Escrow {
        depositor: depositor.clone(),
        amount: 1_000,
        remaining_amount: 1_000,
        status: EscrowStatus::Locked,
        deadline: 9_999_999,
        refund_history: history,
        archived: false,
        archived_at: None,
    };
    assert!(!check_refund_consistency(&escrow));
}

#[test]
fn test_inv4_multiple_partial_refunds_consistent() {
    let env = Env::default();
    let depositor = Address::generate(&env);
    let mut history = Vec::new(&env);
    for _ in 0..3 {
        history.push_back(RefundRecord {
            amount: 100,
            recipient: depositor.clone(),
            timestamp: 0,
            mode: RefundMode::Partial,
        });
    }
    // total refunded = 300, consumed = 400, remaining = 600
    let escrow = Escrow {
        depositor: depositor.clone(),
        amount: 1_000,
        remaining_amount: 600,
        status: EscrowStatus::PartiallyRefunded,
        deadline: 9_999_999,
        refund_history: history,
        archived: false,
        archived_at: None,
    };
    assert!(check_refund_consistency(&escrow));
}

#[test]
fn test_inv4_refund_after_deadline_consistent() {
    let t = TestEnv::setup();
    let depositor = Address::generate(&t.env);
    t.mint(&depositor, 2_000);

    let deadline = t.env.ledger().timestamp() + 50;
    t.client
        .lock_funds(&depositor, &1u64, &2_000, &deadline);
    t.client.publish(&1u64);
    t.advance_time(100);
    t.client.refund(&1u64);

    let escrow: Escrow = t
        .env
        .storage()
        .persistent()
        .get(&DataKey::Escrow(1u64))
        .unwrap();

    assert!(check_refund_consistency(&escrow));
    assert_eq!(escrow.refund_history.len(), 1);
    assert_eq!(escrow.refund_history.get(0).unwrap().amount, 2_000);
}

#[test]
fn test_inv4_anon_no_refund_history_consistent() {
    let env = Env::default();
    let anon = AnonymousEscrow {
        depositor_commitment: BytesN::from_array(&env, &[1u8; 32]),
        amount: 500,
        remaining_amount: 500,
        status: EscrowStatus::Locked,
        deadline: 9_999_999,
        refund_history: vec![&env],
        archived: false,
        archived_at: None,
    };
    assert!(check_anon_refund_consistency(&anon));
}

#[test]
fn test_inv4_anon_refund_exceeds_consumed_fails() {
    let env = Env::default();
    let addr = Address::generate(&env);
    let mut history = Vec::new(&env);
    history.push_back(RefundRecord {
        amount: 999,
        recipient: addr.clone(),
        timestamp: 0,
        mode: RefundMode::Full,
    });
    let anon = AnonymousEscrow {
        depositor_commitment: BytesN::from_array(&env, &[1u8; 32]),
        amount: 500,
        remaining_amount: 400, // consumed = 100, history = 999
        status: EscrowStatus::PartiallyRefunded,
        deadline: 9_999_999,
        refund_history: history,
        archived: false,
        archived_at: None,
    };
    assert!(!check_anon_refund_consistency(&anon));
}

// ============================================================
// INV-5: Index Completeness
// ============================================================

#[test]
fn test_inv5_no_orphans_after_normal_flow() {
    let t = TestEnv::setup();
    let depositor = Address::generate(&t.env);
    t.mint(&depositor, 1_000);

    t.client
        .lock_funds(&depositor, &1u64, &1_000, &t.future_deadline());

    assert_eq!(count_orphaned_index_entries(&t.env), 0);
}

#[test]
fn test_inv5_no_orphans_multiple_escrows() {
    let t = TestEnv::setup();
    for i in 1u64..=5 {
        let depositor = Address::generate(&t.env);
        t.mint(&depositor, 500);
        t.client
            .lock_funds(&depositor, &i, &500, &t.future_deadline());
    }
    assert_eq!(count_orphaned_index_entries(&t.env), 0);
}

#[test]
fn test_inv5_tampered_index_detects_orphan() {
    let t = TestEnv::setup();
    let depositor = Address::generate(&t.env);
    t.mint(&depositor, 1_000);

    t.client
        .lock_funds(&depositor, &1u64, &1_000, &t.future_deadline());

    // Manually inject a ghost entry into the index.
    let mut index: Vec<u64> = t
        .env
        .storage()
        .persistent()
        .get(&DataKey::EscrowIndex)
        .unwrap();
    index.push_back(9_999u64); // no escrow record exists for this id
    t.env
        .storage()
        .persistent()
        .set(&DataKey::EscrowIndex, &index);

    assert_eq!(count_orphaned_index_entries(&t.env), 1);
}

#[test]
fn test_inv5_report_flags_orphan() {
    let t = TestEnv::setup();
    let depositor = Address::generate(&t.env);
    t.mint(&depositor, 1_000);
    t.client
        .lock_funds(&depositor, &1u64, &1_000, &t.future_deadline());

    let mut index: Vec<u64> = t
        .env
        .storage()
        .persistent()
        .get(&DataKey::EscrowIndex)
        .unwrap();
    index.push_back(8_888u64);
    t.env
        .storage()
        .persistent()
        .set(&DataKey::EscrowIndex, &index);

    let report = check_all_invariants(&t.env);
    assert!(!report.healthy);
    assert_eq!(report.orphaned_index_entries, 1);
}

// ============================================================
// check_all_invariants — full report
// ============================================================

#[test]
fn test_check_all_invariants_healthy_after_lock() {
    let t = TestEnv::setup();
    let depositor = Address::generate(&t.env);
    t.mint(&depositor, 1_000);

    t.client
        .lock_funds(&depositor, &1u64, &1_000, &t.future_deadline());

    let report = check_all_invariants(&t.env);
    assert!(report.healthy);
    assert_eq!(report.per_escrow_failures, 0);
    assert_eq!(report.orphaned_index_entries, 0);
    assert_eq!(report.refund_inconsistencies, 0);
    assert_eq!(report.sum_remaining, report.token_balance);
}

#[test]
fn test_check_all_invariants_healthy_after_release() {
    let t = TestEnv::setup();
    let depositor = Address::generate(&t.env);
    let contributor = Address::generate(&t.env);
    t.mint(&depositor, 1_000);

    t.client
        .lock_funds(&depositor, &1u64, &1_000, &t.future_deadline());
    t.client.publish(&1u64);
    t.client.release_funds(&1u64, &contributor);

    let report = check_all_invariants(&t.env);
    assert!(report.healthy);
    assert_eq!(report.sum_remaining, 0);
    assert_eq!(report.token_balance, 0);
}

#[test]
fn test_check_all_invariants_reports_inv2_violation() {
    let t = TestEnv::setup();
    let depositor = Address::generate(&t.env);
    t.mint(&depositor, 1_000);
    t.client
        .lock_funds(&depositor, &1u64, &1_000, &t.future_deadline());

    t.disable_invariant_guard();
    let mut escrow: Escrow = t
        .env
        .storage()
        .persistent()
        .get(&DataKey::Escrow(1u64))
        .unwrap();
    escrow.remaining_amount = 5_000; // inflate artificially
    crate::BountyEscrowContract::write_escrow(&t.env, 1u64, &escrow).unwrap();

    let report = check_all_invariants(&t.env);
    assert!(!report.healthy);
    assert_ne!(report.sum_remaining, report.token_balance);
}

#[test]
fn test_check_all_invariants_reports_inv4_violation() {
    let t = TestEnv::setup();
    let depositor = Address::generate(&t.env);
    t.mint(&depositor, 1_000);
    t.client
        .lock_funds(&depositor, &1u64, &1_000, &t.future_deadline());

    t.disable_invariant_guard();

    // Write an escrow with an inconsistent refund history.
    let mut history = Vec::new(&t.env);
    history.push_back(RefundRecord {
        amount: 999,
        recipient: depositor.clone(),
        timestamp: 0,
        mode: RefundMode::Full,
    });
    let bad_escrow = Escrow {
        depositor: depositor.clone(),
        amount: 1_000,
        remaining_amount: 900, // consumed = 100, but history claims 999
        status: EscrowStatus::PartiallyRefunded,
        deadline: 9_999_999,
        refund_history: history,
        archived: false,
        archived_at: None,
    };
    crate::BountyEscrowContract::write_escrow(&t.env, 1u64, &bad_escrow).unwrap();

    let report = check_all_invariants(&t.env);
    assert!(!report.healthy);
    assert!(report.refund_inconsistencies > 0);
}

// ============================================================
// Property-style tests
// ============================================================

#[test]
fn test_property_fund_conservation_after_mixed_operations() {
    let t = TestEnv::setup();
    let contributor = Address::generate(&t.env);

    // Lock five escrows.
    for i in 1u64..=5 {
        let depositor = Address::generate(&t.env);
        t.mint(&depositor, 1_000);
        t.client
            .lock_funds(&depositor, &i, &1_000, &t.future_deadline());
        t.client.publish(&i);
    }

    // Release two.
    t.client.release_funds(&1u64, &contributor);
    t.client.release_funds(&2u64, &contributor);

    // Partial release one.
    t.client.partial_release(&3u64, &contributor, &400);

    // Refund one (deadline must pass first).
    let depositor6 = Address::generate(&t.env);
    t.mint(&depositor6, 1_000);
    let short_deadline = t.env.ledger().timestamp() + 10;
    t.client
        .lock_funds(&depositor6, &6u64, &1_000, &short_deadline);
    t.client.publish(&6u64);
    t.advance_time(20);
    t.client.refund(&6u64);

    // After all operations, invariant must hold.
    assert_eq!(t.sum_remaining(), t.contract_balance());
    let report = check_all_invariants(&t.env);
    assert!(report.healthy);
}

#[test]
fn test_property_no_fund_creation() {
    let t = TestEnv::setup();
    let initial = t.contract_balance();
    assert_eq!(initial, 0);

    let depositor = Address::generate(&t.env);
    t.mint(&depositor, 3_000);
    t.client
        .lock_funds(&depositor, &1u64, &3_000, &t.future_deadline());

    // Contract balance can only increase by the deposited amount.
    assert_eq!(t.contract_balance(), 3_000);
    assert!(t.contract_balance() <= 3_000);
}

#[test]
fn test_property_no_fund_destruction() {
    let t = TestEnv::setup();
    let depositor = Address::generate(&t.env);
    let contributor = Address::generate(&t.env);
    t.mint(&depositor, 5_000);

    t.client
        .lock_funds(&depositor, &1u64, &5_000, &t.future_deadline());
    t.client.publish(&1u64);

    let before_release = t.contract_balance();
    t.client.partial_release(&1u64, &contributor, &1_000);
    let after_release = t.contract_balance();

    // Balance must decrease by exactly the payout amount.
    assert_eq!(before_release - after_release, 1_000);
    assert_eq!(t.sum_remaining(), t.contract_balance());
}

#[test]
fn test_property_refund_history_bounded() {
    let t = TestEnv::setup();
    let depositor = Address::generate(&t.env);
    t.mint(&depositor, 3_000);

    let deadline = t.env.ledger().timestamp() + 10;
    t.client
        .lock_funds(&depositor, &1u64, &3_000, &deadline);
    t.client.publish(&1u64);
    t.advance_time(20);
    t.client.refund(&1u64);

    let escrow: Escrow = t
        .env
        .storage()
        .persistent()
        .get(&DataKey::Escrow(1u64))
        .unwrap();

    assert!(check_refund_consistency(&escrow));

    let total_refunded: i128 = escrow
        .refund_history
        .iter()
        .map(|r| r.amount)
        .sum();
    let consumed = escrow.amount - escrow.remaining_amount;
    assert!(total_refunded <= consumed);
}

#[test]
fn test_property_invariants_hold_after_every_lock_step() {
    let t = TestEnv::setup();

    for i in 1u64..=10 {
        let depositor = Address::generate(&t.env);
        t.mint(&depositor, 500);
        t.client
            .lock_funds(&depositor, &i, &500, &t.future_deadline());

        // Assert INV-2 after each individual lock.
        assert_eq!(
            t.sum_remaining(),
            t.contract_balance(),
            "INV-2 failed after locking bounty {}",
            i
        );
    }
}