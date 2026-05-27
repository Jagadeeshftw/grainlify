#![cfg(test)]

//! # RBAC Tests — Payout Key Rotation
//!
//! Verifies the role-based access control rules for `rotate_payout_key`:
//!
//! | Caller                  | Allowed? |
//! |-------------------------|----------|
//! | Current payout key      | ✅ Yes   |
//! | Contract admin          | ✅ Yes   |
//! | Arbitrary third party   | ❌ No    |
//! | Old key after rotation  | ❌ No    |
//! | Delegate                | ❌ No    |
//!
//! Security assumptions validated here:
//! - A hijacked (old) key cannot re-rotate after being replaced.
//! - A delegate with full permissions cannot rotate the key.
//! - An unauthorized address cannot rotate even with a correct nonce.

use super::*;
use soroban_sdk::{testutils::Address as _, token, Address, Env, String};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_client(env: &Env) -> (ProgramEscrowContractClient<'static>, Address) {
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);
    (client, contract_id)
}

fn fund_contract(env: &Env, contract_id: &Address, amount: i128) -> Address {
    let token_admin = Address::generate(env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = token_contract.address();
    let sac = token::StellarAssetClient::new(env, &token_id);
    if amount > 0 {
        sac.mint(contract_id, &amount);
    }
    token_id
}

/// Set up a program with a distinct admin and payout key.
fn setup(
    env: &Env,
) -> (
    ProgramEscrowContractClient<'static>,
    String,   // program_id
    Address,  // payout_key
    Address,  // admin
) {
    env.mock_all_auths();
    let (client, contract_id) = make_client(env);
    let token_id = fund_contract(env, &contract_id, 0);
    let admin = Address::generate(env);
    let payout_key = Address::generate(env);
    let program_id = String::from_str(env, "rbac-prog");
    client.initialize_contract(&admin);
    client.init_program(&program_id, &payout_key, &token_id, &payout_key, &None, &None);
    (client, program_id, payout_key, admin)
}

// ---------------------------------------------------------------------------
// Positive cases
// ---------------------------------------------------------------------------

/// Current payout key is authorized to rotate.
#[test]
fn test_rbac_payout_key_can_rotate() {
    let env = Env::default();
    let (client, program_id, payout_key, _admin) = setup(&env);
    let new_key = Address::generate(&env);
    let nonce = client.get_rotation_nonce(&program_id);
    let data = client.rotate_payout_key(&program_id, &payout_key, &new_key, &nonce);
    assert_eq!(data.authorized_payout_key, new_key);
}

/// Contract admin is authorized to rotate.
#[test]
fn test_rbac_admin_can_rotate() {
    let env = Env::default();
    let (client, program_id, _payout_key, admin) = setup(&env);
    let new_key = Address::generate(&env);
    let nonce = client.get_rotation_nonce(&program_id);
    let data = client.rotate_payout_key(&program_id, &admin, &new_key, &nonce);
    assert_eq!(data.authorized_payout_key, new_key);
}

// ---------------------------------------------------------------------------
// Negative cases
// ---------------------------------------------------------------------------

/// An arbitrary third party cannot rotate the key.
#[test]
#[should_panic(expected = "Unauthorized")]
fn test_rbac_unauthorized_caller_rejected() {
    let env = Env::default();
    let (client, program_id, _payout_key, _admin) = setup(&env);
    let attacker = Address::generate(&env);
    let new_key = Address::generate(&env);
    let nonce = client.get_rotation_nonce(&program_id);
    client.rotate_payout_key(&program_id, &attacker, &new_key, &nonce);
}

/// After rotation the old key is immediately invalidated and cannot rotate again.
#[test]
#[should_panic(expected = "Unauthorized")]
fn test_rbac_old_key_cannot_rotate_after_replacement() {
    let env = Env::default();
    let (client, program_id, old_key, _admin) = setup(&env);
    let new_key = Address::generate(&env);
    let key3 = Address::generate(&env);

    // Successful rotation: old_key → new_key.
    let nonce0 = client.get_rotation_nonce(&program_id);
    client.rotate_payout_key(&program_id, &old_key, &new_key, &nonce0);

    // old_key is now invalid; attempting another rotation must fail.
    let nonce1 = client.get_rotation_nonce(&program_id);
    client.rotate_payout_key(&program_id, &old_key, &key3, &nonce1);
}

/// A delegate with all permissions cannot rotate the payout key.
///
/// Key rotation is a privileged operation reserved for the payout key itself
/// or the contract admin — delegates are explicitly excluded.
#[test]
#[should_panic(expected = "Unauthorized")]
fn test_rbac_delegate_cannot_rotate() {
    let env = Env::default();
    let (client, program_id, payout_key, _admin) = setup(&env);
    let delegate = Address::generate(&env);
    let new_key = Address::generate(&env);

    // Grant delegate all permissions.
    client.set_program_delegate(
        &program_id,
        &payout_key,
        &delegate,
        &(DELEGATE_PERMISSION_RELEASE | DELEGATE_PERMISSION_REFUND | DELEGATE_PERMISSION_UPDATE_META),
    );

    let nonce = client.get_rotation_nonce(&program_id);
    // Delegate must not be able to rotate.
    client.rotate_payout_key(&program_id, &delegate, &new_key, &nonce);
}

/// Rotation on a non-existent program must panic.
#[test]
#[should_panic(expected = "Program not found")]
fn test_rbac_rotation_on_missing_program_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _contract_id) = make_client(&env);
    let admin = Address::generate(&env);
    client.initialize_contract(&admin);

    let ghost_id = String::from_str(&env, "ghost-prog");
    let caller = Address::generate(&env);
    let new_key = Address::generate(&env);
    client.rotate_payout_key(&ghost_id, &caller, &new_key, &0);
}

/// Wrong nonce is rejected even when caller is authorized.
#[test]
#[should_panic(expected = "Invalid nonce")]
fn test_rbac_wrong_nonce_rejected_for_authorized_caller() {
    let env = Env::default();
    let (client, program_id, payout_key, _admin) = setup(&env);
    let new_key = Address::generate(&env);
    // Supply nonce=99 when stored nonce is 0.
    client.rotate_payout_key(&program_id, &payout_key, &new_key, &99);
}

// ---------------------------------------------------------------------------
// Multisig Threshold Tests
// ---------------------------------------------------------------------------

use crate::{AdminOpKind, MultisigThresholdConfig, ADMIN_OP_EXPIRY_LEDGERS};

fn advance_seq(env: &Env, n: u32) {
    env.ledger().with_mut(|li| li.sequence_number += n);
}

fn make_payload(env: &Env, tag: &str) -> soroban_sdk::Bytes {
    soroban_sdk::Bytes::from_slice(env, tag.as_bytes())
}

/// Set up a contract with a 2-of-3 multisig threshold config.
fn setup_multisig(
    env: &Env,
) -> (
    ProgramEscrowContractClient<'static>,
    Address, // admin
    Address, // signer1
    Address, // signer2
    Address, // signer3
) {
    env.mock_all_auths();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize_contract(&admin);

    let s1 = Address::generate(env);
    let s2 = Address::generate(env);
    let s3 = Address::generate(env);
    let signers = soroban_sdk::vec![env, s1.clone(), s2.clone(), s3.clone()];
    client.set_multisig_threshold_config(&signers, &2, &1_000_000i128);

    (client, admin, s1, s2, s3)
}

// --- set_multisig_threshold_config ---

#[test]
fn test_set_multisig_config_stores_correctly() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize_contract(&admin);

    let s1 = Address::generate(&env);
    let s2 = Address::generate(&env);
    client.set_multisig_threshold_config(
        &soroban_sdk::vec![&env, s1.clone(), s2.clone()],
        &2,
        &500_000i128,
    );

    let cfg = client.get_multisig_threshold_config().unwrap();
    assert_eq!(cfg.required_approvals, 2);
    assert_eq!(cfg.high_value_threshold, 500_000);
    assert_eq!(cfg.signers.len(), 2);
}

#[test]
#[should_panic(expected = "InvalidMultisigConfig")]
fn test_set_multisig_config_required_gt_signers_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize_contract(&admin);
    let s1 = Address::generate(&env);
    // required=3 but only 1 signer
    client.set_multisig_threshold_config(&soroban_sdk::vec![&env, s1], &3, &1000i128);
}

#[test]
#[should_panic(expected = "InvalidMultisigConfig")]
fn test_set_multisig_config_zero_required_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize_contract(&admin);
    let s1 = Address::generate(&env);
    client.set_multisig_threshold_config(&soroban_sdk::vec![&env, s1], &0, &1000i128);
}

// --- propose_admin_op ---

#[test]
fn test_propose_admin_op_stores_pending_op() {
    let env = Env::default();
    let (client, _admin, _s1, _s2, _s3) = setup_multisig(&env);
    let payload = make_payload(&env, "fee-update-v1");

    let op = client.propose_admin_op(&AdminOpKind::UpdateFeeConfig, &0i128, &payload);
    assert_eq!(op.kind, AdminOpKind::UpdateFeeConfig);
    assert_eq!(op.approvals.len(), 1); // proposer auto-approves
}

#[test]
#[should_panic(expected = "PendingOpExists")]
fn test_propose_second_op_while_first_pending_rejected() {
    let env = Env::default();
    let (client, _admin, _s1, _s2, _s3) = setup_multisig(&env);
    let payload = make_payload(&env, "op1");
    client.propose_admin_op(&AdminOpKind::UpdateFeeConfig, &0i128, &payload);
    // Second proposal while first is still pending
    client.propose_admin_op(&AdminOpKind::EmergencyWithdraw, &5_000_000i128, &payload);
}

#[test]
fn test_propose_replaces_expired_op() {
    let env = Env::default();
    let (client, _admin, _s1, _s2, _s3) = setup_multisig(&env);
    let payload = make_payload(&env, "op1");
    client.propose_admin_op(&AdminOpKind::UpdateFeeConfig, &0i128, &payload);

    // Advance past expiry
    advance_seq(&env, ADMIN_OP_EXPIRY_LEDGERS + 1);

    // New proposal should succeed
    let payload2 = make_payload(&env, "op2");
    let op = client.propose_admin_op(&AdminOpKind::EmergencyWithdraw, &0i128, &payload2);
    assert_eq!(op.kind, AdminOpKind::EmergencyWithdraw);
}

// --- approve_admin_op ---

#[test]
fn test_approve_increments_approval_count() {
    let env = Env::default();
    let (client, _admin, s1, _s2, _s3) = setup_multisig(&env);
    let payload = make_payload(&env, "fee-update");
    client.propose_admin_op(&AdminOpKind::UpdateFeeConfig, &0i128, &payload);

    let op = client.approve_admin_op(&s1);
    assert_eq!(op.approvals.len(), 2); // proposer + s1
}

#[test]
#[should_panic(expected = "NotASigner")]
fn test_non_signer_cannot_approve() {
    let env = Env::default();
    let (client, _admin, _s1, _s2, _s3) = setup_multisig(&env);
    let payload = make_payload(&env, "fee-update");
    client.propose_admin_op(&AdminOpKind::UpdateFeeConfig, &0i128, &payload);

    let outsider = Address::generate(&env);
    client.approve_admin_op(&outsider);
}

#[test]
#[should_panic(expected = "AlreadyApproved")]
fn test_double_approve_rejected() {
    let env = Env::default();
    let (client, _admin, s1, _s2, _s3) = setup_multisig(&env);
    let payload = make_payload(&env, "fee-update");
    client.propose_admin_op(&AdminOpKind::UpdateFeeConfig, &0i128, &payload);
    client.approve_admin_op(&s1);
    client.approve_admin_op(&s1); // second approval from same signer
}

#[test]
#[should_panic(expected = "PendingOpExpired")]
fn test_approve_expired_op_rejected() {
    let env = Env::default();
    let (client, _admin, s1, _s2, _s3) = setup_multisig(&env);
    let payload = make_payload(&env, "fee-update");
    client.propose_admin_op(&AdminOpKind::UpdateFeeConfig, &0i128, &payload);
    advance_seq(&env, ADMIN_OP_EXPIRY_LEDGERS + 1);
    client.approve_admin_op(&s1);
}

// --- execute_admin_op ---

#[test]
fn test_execute_succeeds_after_threshold_met() {
    let env = Env::default();
    let (client, _admin, s1, _s2, _s3) = setup_multisig(&env);
    let payload = make_payload(&env, "fee-update");
    client.propose_admin_op(&AdminOpKind::UpdateFeeConfig, &0i128, &payload);
    client.approve_admin_op(&s1); // now 2-of-3 met

    let kind = client.execute_admin_op(&payload);
    assert_eq!(kind, AdminOpKind::UpdateFeeConfig);

    // Pending op must be cleared
    assert!(client.get_pending_admin_op().is_none());
}

#[test]
#[should_panic(expected = "InsufficientApprovals")]
fn test_execute_before_threshold_rejected() {
    let env = Env::default();
    let (client, _admin, _s1, _s2, _s3) = setup_multisig(&env);
    let payload = make_payload(&env, "fee-update");
    client.propose_admin_op(&AdminOpKind::UpdateFeeConfig, &0i128, &payload);
    // Only 1 approval (proposer), need 2
    client.execute_admin_op(&payload);
}

#[test]
#[should_panic(expected = "PayloadMismatch")]
fn test_execute_wrong_payload_rejected() {
    let env = Env::default();
    let (client, _admin, s1, _s2, _s3) = setup_multisig(&env);
    let payload = make_payload(&env, "fee-update");
    client.propose_admin_op(&AdminOpKind::UpdateFeeConfig, &0i128, &payload);
    client.approve_admin_op(&s1);

    let wrong_payload = make_payload(&env, "different-payload");
    client.execute_admin_op(&wrong_payload);
}

#[test]
#[should_panic(expected = "PendingOpExpired")]
fn test_execute_expired_op_rejected() {
    let env = Env::default();
    let (client, _admin, s1, _s2, _s3) = setup_multisig(&env);
    let payload = make_payload(&env, "fee-update");
    client.propose_admin_op(&AdminOpKind::UpdateFeeConfig, &0i128, &payload);
    client.approve_admin_op(&s1);
    advance_seq(&env, ADMIN_OP_EXPIRY_LEDGERS + 1);
    client.execute_admin_op(&payload);
}

#[test]
#[should_panic(expected = "NoPendingOp")]
fn test_execute_with_no_pending_op_rejected() {
    let env = Env::default();
    let (client, _admin, _s1, _s2, _s3) = setup_multisig(&env);
    let payload = make_payload(&env, "fee-update");
    client.execute_admin_op(&payload);
}

// --- cancel_admin_op ---

#[test]
fn test_cancel_clears_pending_op() {
    let env = Env::default();
    let (client, _admin, _s1, _s2, _s3) = setup_multisig(&env);
    let payload = make_payload(&env, "fee-update");
    client.propose_admin_op(&AdminOpKind::UpdateFeeConfig, &0i128, &payload);
    assert!(client.get_pending_admin_op().is_some());

    client.cancel_admin_op();
    assert!(client.get_pending_admin_op().is_none());
}

// --- 1-of-1 (default / no multisig) ---

#[test]
fn test_single_approval_executes_immediately_after_propose_and_execute() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize_contract(&admin);

    // 1-of-1 config
    client.set_multisig_threshold_config(
        &soroban_sdk::vec![&env, admin.clone()],
        &1,
        &1_000_000i128,
    );

    let payload = make_payload(&env, "op");
    client.propose_admin_op(&AdminOpKind::UpdateFeeConfig, &0i128, &payload);
    // 1 approval already (proposer), threshold met — execute immediately
    let kind = client.execute_admin_op(&payload);
    assert_eq!(kind, AdminOpKind::UpdateFeeConfig);
}

// --- 3-of-3 full quorum ---

#[test]
fn test_full_quorum_3_of_3() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize_contract(&admin);

    let s1 = Address::generate(&env);
    let s2 = Address::generate(&env);
    let s3 = Address::generate(&env);
    client.set_multisig_threshold_config(
        &soroban_sdk::vec![&env, s1.clone(), s2.clone(), s3.clone()],
        &3,
        &0i128,
    );

    let payload = make_payload(&env, "full-quorum");
    client.propose_admin_op(&AdminOpKind::EmergencyWithdraw, &0i128, &payload);
    client.approve_admin_op(&s1);
    client.approve_admin_op(&s2);
    client.approve_admin_op(&s3);

    let kind = client.execute_admin_op(&payload);
    assert_eq!(kind, AdminOpKind::EmergencyWithdraw);
}
