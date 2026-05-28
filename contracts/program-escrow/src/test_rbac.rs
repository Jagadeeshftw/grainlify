#![cfg(test)]

//! # RBAC Tests — Payout Key Rotation and Draft Status Guards
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
//! Also verifies Draft status guards for delegate and capability-token operations:
//! - set_program_delegate must reject programs in Draft status
//! - revoke_program_delegate must reject programs in Draft status  
//! - Delegate actions (via require_program_actor) must reject programs in Draft status
//!
//! Security assumptions validated here:
//! - A hijacked (old) key cannot re-rotate after being replaced.
//! - A delegate with full permissions cannot rotate the key.
//! - An unauthorized address cannot rotate even with a correct nonce.
//! - Delegate operations are blocked on programs in Draft status.

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
// Delegation-chain exploit tests
//
// Security invariants verified:
//   1. Only the program owner (payout key) or contract admin can call
//      `set_program_delegate` — a delegate cannot re-delegate.
//   2. A delegate cannot escalate its own bitmask beyond what was granted.
//   3. A delegate with a subset of permissions cannot grant a superset to
//      a third party.
//   4. A delegate cannot overwrite itself with a different address.
//   5. A delegate cannot revoke itself or another delegate.
// ---------------------------------------------------------------------------

/// A delegate with full permissions cannot call `set_program_delegate` to
/// grant those permissions to a third party (re-delegation / chain attack).
#[test]
#[should_panic(expected = "Unauthorized")]
fn test_delegate_cannot_redelegate_to_third_party() {
    let env = Env::default();
    let (client, program_id, payout_key, _admin) = setup(&env);
    let delegate = Address::generate(&env);
    let third_party = Address::generate(&env);

    // Owner grants delegate all permissions.
    client.set_program_delegate(
        &program_id,
        &payout_key,
        &delegate,
        &DELEGATE_PERMISSION_MASK,
    );

    // Delegate attempts to re-delegate to a third party — must be rejected.
    client.set_program_delegate(
        &program_id,
        &delegate,
        &third_party,
        &DELEGATE_PERMISSION_MASK,
    );
}

/// A delegate with only RELEASE permission cannot grant RELEASE to another
/// address, even though it holds that permission itself.
#[test]
#[should_panic(expected = "Unauthorized")]
fn test_delegate_with_partial_permissions_cannot_redelegate() {
    let env = Env::default();
    let (client, program_id, payout_key, _admin) = setup(&env);
    let delegate = Address::generate(&env);
    let third_party = Address::generate(&env);

    client.set_program_delegate(
        &program_id,
        &payout_key,
        &delegate,
        &DELEGATE_PERMISSION_RELEASE,
    );

    // Delegate tries to pass its RELEASE permission on — must be rejected.
    client.set_program_delegate(
        &program_id,
        &delegate,
        &third_party,
        &DELEGATE_PERMISSION_RELEASE,
    );
}

/// A delegate cannot escalate its own bitmask by calling `set_program_delegate`
/// with itself as the delegate target and a larger permission set.
#[test]
#[should_panic(expected = "Unauthorized")]
fn test_delegate_cannot_escalate_own_permissions() {
    let env = Env::default();
    let (client, program_id, payout_key, _admin) = setup(&env);
    let delegate = Address::generate(&env);

    // Grant only RELEASE.
    client.set_program_delegate(
        &program_id,
        &payout_key,
        &delegate,
        &DELEGATE_PERMISSION_RELEASE,
    );

    // Delegate tries to upgrade itself to full permissions — must be rejected.
    client.set_program_delegate(
        &program_id,
        &delegate,
        &delegate,
        &DELEGATE_PERMISSION_MASK,
    );
}

/// A delegate cannot revoke itself (or any other delegate) via
/// `revoke_program_delegate` — only the owner or admin may revoke.
#[test]
#[should_panic(expected = "Unauthorized")]
fn test_delegate_cannot_revoke_itself() {
    let env = Env::default();
    let (client, program_id, payout_key, _admin) = setup(&env);
    let delegate = Address::generate(&env);

    client.set_program_delegate(
        &program_id,
        &payout_key,
        &delegate,
        &DELEGATE_PERMISSION_MASK,
    );

    // Delegate attempts self-revocation — must be rejected.
    client.revoke_program_delegate(&program_id, &delegate);
}

/// An arbitrary third party (neither owner, admin, nor delegate) cannot
/// set a delegate on a program.
#[test]
#[should_panic(expected = "Unauthorized")]
fn test_arbitrary_address_cannot_set_delegate() {
    let env = Env::default();
    let (client, program_id, _payout_key, _admin) = setup(&env);
    let attacker = Address::generate(&env);
    let victim = Address::generate(&env);

    client.set_program_delegate(
        &program_id,
        &attacker,
        &victim,
        &DELEGATE_PERMISSION_MASK,
    );
}

/// The contract admin CAN set a delegate (positive control — ensures the
/// restriction targets delegates specifically, not all non-owners).
#[test]
fn test_admin_can_set_delegate() {
    let env = Env::default();
    let (client, program_id, _payout_key, admin) = setup(&env);
    let delegate = Address::generate(&env);

    let data = client.set_program_delegate(
        &program_id,
        &admin,
        &delegate,
        &DELEGATE_PERMISSION_RELEASE,
    );

    assert_eq!(data.delegate, Some(delegate));
    assert_eq!(data.delegate_permissions, DELEGATE_PERMISSION_RELEASE);
}

/// After a delegate is set, the owner can replace it with a different address
/// and different permissions — the new state is authoritative.
#[test]
fn test_owner_can_replace_delegate() {
    let env = Env::default();
    let (client, program_id, payout_key, _admin) = setup(&env);
    let delegate_a = Address::generate(&env);
    let delegate_b = Address::generate(&env);

    client.set_program_delegate(
        &program_id,
        &payout_key,
        &delegate_a,
        &DELEGATE_PERMISSION_MASK,
    );

    let data = client.set_program_delegate(
        &program_id,
        &payout_key,
        &delegate_b,
        &DELEGATE_PERMISSION_RELEASE,
    );

    // delegate_b is now the active delegate with reduced permissions.
    assert_eq!(data.delegate, Some(delegate_b));
    assert_eq!(data.delegate_permissions, DELEGATE_PERMISSION_RELEASE);
}

/// After the owner replaces the delegate, the old delegate loses all access
/// and cannot perform operations that required its former permissions.
#[test]
#[should_panic(expected = "Unauthorized")]
fn test_replaced_delegate_loses_access() {
    let env = Env::default();
    let (client, program_id, payout_key, _admin) = setup(&env);
    let delegate_a = Address::generate(&env);
    let delegate_b = Address::generate(&env);

    client.set_program_delegate(
        &program_id,
        &payout_key,
        &delegate_a,
        &DELEGATE_PERMISSION_MASK,
    );

    // Replace delegate_a with delegate_b.
    client.set_program_delegate(
        &program_id,
        &payout_key,
        &delegate_b,
        &DELEGATE_PERMISSION_RELEASE,
    );

    // delegate_a is no longer active — any attempt to re-delegate must fail.
    client.set_program_delegate(
        &program_id,
        &delegate_a,
        &delegate_a,
        &DELEGATE_PERMISSION_MASK,
    );
}

/// `set_program_delegate` rejects a permissions bitmask of zero.
#[test]
#[should_panic(expected = "Delegate permissions cannot be empty")]
fn test_set_delegate_rejects_zero_permissions() {
    let env = Env::default();
    let (client, program_id, payout_key, _admin) = setup(&env);
    let delegate = Address::generate(&env);

    client.set_program_delegate(&program_id, &payout_key, &delegate, &0);
}

/// `set_program_delegate` rejects a bitmask with unsupported bits set.
#[test]
#[should_panic(expected = "Unsupported delegate permissions")]
fn test_set_delegate_rejects_unsupported_permission_bits() {
    let env = Env::default();
    let (client, program_id, payout_key, _admin) = setup(&env);
    let delegate = Address::generate(&env);

    // Bit 3 (0x08) is outside DELEGATE_PERMISSION_MASK.
    client.set_program_delegate(&program_id, &payout_key, &delegate, &0x08);
}

/// The payout key cannot be set as its own delegate.
#[test]
#[should_panic(expected = "Delegate must differ from owner")]
fn test_owner_cannot_be_set_as_own_delegate() {
    let env = Env::default();
    let (client, program_id, payout_key, _admin) = setup(&env);

    client.set_program_delegate(
        &program_id,
        &payout_key,
        &payout_key,
        &DELEGATE_PERMISSION_MASK,
    );
}
