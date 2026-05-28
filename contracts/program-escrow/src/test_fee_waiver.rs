//! # Per-PayoutType Fee Waiver Tests
//!
//! Verifies the fee waiver feature introduced in [`FeeConfig::fee_waivers`].
//!
//! ## What is tested
//!
//! | # | Test | Property verified |
//! |---|------|-------------------|
//! | 1 | `single_waiver_delivers_full_amount` | `PayoutType::Single` waiver → recipient receives gross amount |
//! | 2 | `batch_waiver_delivers_full_amounts` | `PayoutType::Batch` waiver → every recipient receives gross amount |
//! | 3 | `single_waiver_does_not_affect_batch` | Single waiver only — batch payout still deducts fee |
//! | 4 | `batch_waiver_does_not_affect_single` | Batch waiver only — single payout still deducts fee |
//! | 5 | `clearing_single_waiver_restores_fee` | Waiver removed → fee resumes on next payout |
//! | 6 | `set_fee_waiver_requires_admin` | Non-admin call panics |
//! | 7 | `get_fee_config_reflects_waiver_bitmask` | `get_fee_config` exposes correct `fee_waivers` value |
//! | 8 | `both_types_waived_simultaneously` | Both bits set → neither Single nor Batch charges fee |
//! | 9 | `fee_recipient_balance_is_zero_when_waived` | Fee recipient receives nothing when waiver active |
//! | 10 | `fee_waiver_event_emitted_on_set` | `FeeWaiverUpdatedEvent` emitted with correct fields |

#![cfg(test)]
extern crate std;

use soroban_sdk::{testutils::Address as _, token, Address, Env, String};

use crate::{
    DataKey, FeeWaiverUpdatedEvent, PayoutType, ProgramData, ProgramEscrowContract,
    ProgramStatus, FEE_WAIVER_BATCH, FEE_WAIVER_SINGLE, PROGRAM_DATA,
};

// ============================================================================
// Test Environment
// ============================================================================
//
// Design:
//   - `FeeWaiverTestEnv::new(balance)` registers the contract, injects
//     `Admin` + `ProgramData` directly into storage, and mints `balance`
//     tokens to the contract address.  No `init_program` is called — this
//     keeps the setup minimal and independent of init-path complexity.
//   - `enable_fee(rate_bps)` configures `update_fee_config` via the public
//     client so that payout fees are charged at `rate_bps` basis points.
//   - `balance_of(addr)` reads the token balance via the SAC client.
//
// Time: O(1) per helper.  Space: O(1) per instance.

struct FeeWaiverTestEnv {
    env: Env,
    contract_id: Address,
    token: Address,
    admin: Address,
    fee_recipient: Address,
}

impl FeeWaiverTestEnv {
    fn new(balance: i128) -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let fee_recipient = Address::generate(&env);
        let payout_key = Address::generate(&env);
        let token_admin = Address::generate(&env);

        let sac = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token = sac.address();

        let contract_id = env.register_contract(None, ProgramEscrowContract);

        // Inject admin and program state directly — avoids coupling to init_program.
        env.as_contract(&contract_id, || {
            env.storage().instance().set(&DataKey::Admin, &admin);
            let program_data = ProgramData {
                program_id: String::from_str(&env, "TestProg"),
                total_funds: balance,
                remaining_balance: balance,
                authorized_payout_key: payout_key,
                delegate: None,
                delegate_permissions: 0,
                payout_history: soroban_sdk::vec![&env],
                token_address: token.clone(),
                initial_liquidity: 0,
                risk_flags: 0,
                reference_hash: None,
                archived: false,
                archived_at: None,
                status: ProgramStatus::Active,
            };
            env.storage().instance().set(&PROGRAM_DATA, &program_data);
        });

        // Fund the contract with tokens.
        token::StellarAssetClient::new(&env, &token).mint(&contract_id, &balance);

        Self {
            env,
            contract_id,
            token,
            admin,
            fee_recipient,
        }
    }

    fn client(&self) -> crate::ProgramEscrowContractClient {
        crate::ProgramEscrowContractClient::new(&self.env, &self.contract_id)
    }

    /// Enable payout fee at `rate_bps` basis points and set `fee_recipient`.
    fn enable_payout_fee(&self, rate_bps: i128) {
        self.client().update_fee_config(
            &None,
            &Some(rate_bps),
            &None,
            &None,
            &Some(self.fee_recipient.clone()),
            &Some(true),
        );
    }

    fn token_balance(&self, addr: &Address) -> i128 {
        token::Client::new(&self.env, &self.token).balance(addr)
    }
}

// ============================================================================
// Tests
// ============================================================================

/// **1. Single waiver delivers full gross amount to recipient.**
///
/// With a 10% payout fee and the Single waiver active, `single_payout`
/// must transfer the full 1_000 to the recipient and zero to fee_recipient.
#[test]
fn test_single_waiver_delivers_full_amount() {
    let t = FeeWaiverTestEnv::new(10_000);
    let recipient = Address::generate(&t.env);

    // 10% payout fee — without waiver recipient would get 900.
    t.enable_payout_fee(1_000);
    t.client()
        .set_fee_waiver(&PayoutType::Single, &true);

    t.client().single_payout(&recipient, &1_000, &None);

    assert_eq!(t.token_balance(&recipient), 1_000, "waived: full amount to recipient");
    assert_eq!(t.token_balance(&t.fee_recipient), 0, "waived: fee recipient gets nothing");
}

/// **2. Batch waiver delivers full gross amount to every recipient.**
///
/// With a 5% fee and the Batch waiver set, `batch_payout` transfers the
/// declared gross to each recipient — no fee deducted from any entry.
#[test]
fn test_batch_waiver_delivers_full_amounts() {
    let t = FeeWaiverTestEnv::new(10_000);
    let r1 = Address::generate(&t.env);
    let r2 = Address::generate(&t.env);

    t.enable_payout_fee(500); // 5%
    t.client()
        .set_fee_waiver(&PayoutType::Batch(0), &true);

    let recipients = soroban_sdk::vec![&t.env, r1.clone(), r2.clone()];
    let amounts = soroban_sdk::vec![&t.env, 2_000i128, 3_000i128];
    t.client().batch_payout(&recipients, &amounts, &None);

    assert_eq!(t.token_balance(&r1), 2_000, "r1: full amount, no fee");
    assert_eq!(t.token_balance(&r2), 3_000, "r2: full amount, no fee");
    assert_eq!(t.token_balance(&t.fee_recipient), 0, "fee recipient gets nothing");
}

/// **3. Single waiver does not affect batch payouts.**
///
/// Waiving `PayoutType::Single` must leave batch fee deduction unchanged.
#[test]
fn test_single_waiver_does_not_affect_batch() {
    let t = FeeWaiverTestEnv::new(10_000);
    let r1 = Address::generate(&t.env);

    t.enable_payout_fee(1_000); // 10%
    t.client()
        .set_fee_waiver(&PayoutType::Single, &true);

    // Batch payout: 1_000 gross → fee = 100 → net = 900.
    let recipients = soroban_sdk::vec![&t.env, r1.clone()];
    let amounts = soroban_sdk::vec![&t.env, 1_000i128];
    t.client().batch_payout(&recipients, &amounts, &None);

    assert_eq!(t.token_balance(&r1), 900, "batch: fee still charged");
    assert_eq!(t.token_balance(&t.fee_recipient), 100, "batch: fee recipient receives fee");
}

/// **4. Batch waiver does not affect single payouts.**
#[test]
fn test_batch_waiver_does_not_affect_single() {
    let t = FeeWaiverTestEnv::new(10_000);
    let recipient = Address::generate(&t.env);

    t.enable_payout_fee(1_000); // 10%
    t.client()
        .set_fee_waiver(&PayoutType::Batch(0), &true);

    // Single payout: 1_000 gross → fee = 100 → net = 900.
    t.client().single_payout(&recipient, &1_000, &None);

    assert_eq!(t.token_balance(&recipient), 900, "single: fee still charged");
    assert_eq!(t.token_balance(&t.fee_recipient), 100, "single: fee recipient receives fee");
}

/// **5. Clearing the single waiver restores fee deduction.**
///
/// After the waiver is removed via `set_fee_waiver(..., false)`, the next
/// `single_payout` must deduct the fee again.
#[test]
fn test_clearing_single_waiver_restores_fee() {
    let t = FeeWaiverTestEnv::new(10_000);
    let r1 = Address::generate(&t.env);
    let r2 = Address::generate(&t.env);

    t.enable_payout_fee(1_000); // 10%

    // First payout: waiver ON → no fee.
    t.client().set_fee_waiver(&PayoutType::Single, &true);
    t.client().single_payout(&r1, &1_000, &None);
    assert_eq!(t.token_balance(&r1), 1_000, "waiver on: full amount");
    assert_eq!(t.token_balance(&t.fee_recipient), 0);

    // Disable waiver.
    t.client().set_fee_waiver(&PayoutType::Single, &false);

    // Second payout: fee resumes.
    t.client().single_payout(&r2, &1_000, &None);
    assert_eq!(t.token_balance(&r2), 900, "waiver off: fee charged again");
    assert_eq!(t.token_balance(&t.fee_recipient), 100);
}

/// **6. `set_fee_waiver` requires admin authorisation.**
///
/// A call from a non-admin address must panic.
#[test]
#[should_panic]
fn test_set_fee_waiver_requires_admin() {
    let env = Env::default();
    // Do NOT mock all auths — force real auth.

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(token_admin);
    let token = sac.address();
    let payout_key = Address::generate(&env);

    let contract_id = env.register_contract(None, ProgramEscrowContract);

    env.as_contract(&contract_id, || {
        env.storage().instance().set(&DataKey::Admin, &admin);
        let program_data = ProgramData {
            program_id: String::from_str(&env, "TestProg"),
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
        };
        env.storage().instance().set(&PROGRAM_DATA, &program_data);
    });

    // Admin authorises only itself — not a non-admin caller.
    let non_admin = Address::generate(&env);
    non_admin.mock_auths(&[]);

    // This must panic because `admin.require_auth()` has no mock entry for non_admin.
    crate::ProgramEscrowContractClient::new(&env, &contract_id)
        .set_fee_waiver(&PayoutType::Single, &true);
}

/// **7. `get_fee_config` exposes the correct waiver bitmask.**
///
/// After toggling each waiver bit, `get_fee_config()` must return the
/// matching `fee_waivers` field.
#[test]
fn test_get_fee_config_reflects_waiver_bitmask() {
    let t = FeeWaiverTestEnv::new(0);
    let client = t.client();

    // Default: no waivers.
    assert_eq!(client.get_fee_config().fee_waivers, 0);

    // Set Single.
    client.set_fee_waiver(&PayoutType::Single, &true);
    assert_eq!(client.get_fee_config().fee_waivers, FEE_WAIVER_SINGLE);

    // Add Batch.
    client.set_fee_waiver(&PayoutType::Batch(0), &true);
    assert_eq!(
        client.get_fee_config().fee_waivers,
        FEE_WAIVER_SINGLE | FEE_WAIVER_BATCH
    );

    // Clear Single — Batch remains.
    client.set_fee_waiver(&PayoutType::Single, &false);
    assert_eq!(client.get_fee_config().fee_waivers, FEE_WAIVER_BATCH);

    // Clear Batch — back to zero.
    client.set_fee_waiver(&PayoutType::Batch(0), &false);
    assert_eq!(client.get_fee_config().fee_waivers, 0);
}

/// **8. Both types can be waived simultaneously.**
///
/// With both bits set, both `single_payout` and `batch_payout` deliver
/// full gross amounts.
#[test]
fn test_both_types_waived_simultaneously() {
    let t = FeeWaiverTestEnv::new(10_000);
    let r_single = Address::generate(&t.env);
    let r_batch = Address::generate(&t.env);

    t.enable_payout_fee(1_000); // 10%
    t.client().set_fee_waiver(&PayoutType::Single, &true);
    t.client().set_fee_waiver(&PayoutType::Batch(0), &true);

    t.client().single_payout(&r_single, &1_000, &None);

    let recipients = soroban_sdk::vec![&t.env, r_batch.clone()];
    let amounts = soroban_sdk::vec![&t.env, 2_000i128];
    t.client().batch_payout(&recipients, &amounts, &None);

    assert_eq!(t.token_balance(&r_single), 1_000, "single: no fee");
    assert_eq!(t.token_balance(&r_batch), 2_000, "batch: no fee");
    assert_eq!(t.token_balance(&t.fee_recipient), 0, "fee recipient: nothing");
}

/// **9. Fee recipient receives zero tokens when waiver is active.**
///
/// Confirms the token-transfer branch is not entered when the waiver bit is set.
#[test]
fn test_fee_recipient_receives_nothing_when_waived() {
    let t = FeeWaiverTestEnv::new(10_000);
    let recipient = Address::generate(&t.env);

    // High fee rate to make any unintended deduction obvious.
    t.enable_payout_fee(1_000); // 10%
    t.client().set_fee_waiver(&PayoutType::Single, &true);

    // Perform multiple payouts; fee_recipient balance must remain 0 throughout.
    for _ in 0..3 {
        let r = Address::generate(&t.env);
        t.client().single_payout(&r, &100, &None);
        assert_eq!(t.token_balance(&t.fee_recipient), 0);
    }
    let _ = recipient; // suppress unused warning
}

/// **10. `FeeWaiverUpdatedEvent` is emitted with correct fields.**
///
/// Verifies the audit event contains the right bit, waived flag, and admin address.
#[test]
fn test_fee_waiver_event_is_emitted() {
    let t = FeeWaiverTestEnv::new(0);

    t.client().set_fee_waiver(&PayoutType::Single, &true);

    let events = t.env.events().all();
    // Find the FeeWaiverUpdatedEvent in the event log.
    let mut found = false;
    for i in 0..events.len() {
        let (_contract, _topics, data): (Address, soroban_sdk::Vec<soroban_sdk::Val>, soroban_sdk::Val) =
            events.get(i).unwrap();
        if let Ok(ev) = FeeWaiverUpdatedEvent::try_from_val(&t.env, &data) {
            assert_eq!(ev.payout_type_bit, FEE_WAIVER_SINGLE, "correct bit");
            assert!(ev.waived, "waived = true");
            assert_eq!(ev.updated_by, t.admin, "admin is the updater");
            assert_eq!(ev.version, 2);
            found = true;
            break;
        }
    }
    assert!(found, "FeeWaiverUpdatedEvent must be emitted");
}
