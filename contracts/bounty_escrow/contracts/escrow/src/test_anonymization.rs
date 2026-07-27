//! Regression tests for privacy leaks on the *read* paths of anonymous escrows.
//!
//! `lock_funds_anonymous` is meant to shield the depositor's identity until
//! `refund_resolved` is driven by the configured `AnonymousResolver`. These tests
//! guard the query surface (`get_escrow_info`, `get_metadata`, and the raw
//! storage layout) against ever exposing that identity before resolution.
//!
//! This suite deliberately does not touch `set_anonymous_resolver` /
//! `refund_resolved` correctness itself (tracked separately) — it only asserts
//! that the *query* paths stay blind to the depositor pre-resolution, and that
//! the resolver-driven path is the sole way identity becomes visible.

use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, token, BytesN, Env};

fn create_token<'a>(e: &Env, admin: &Address) -> token::StellarAssetClient<'a> {
    let contract = e.register_stellar_asset_contract_v2(admin.clone());
    token::StellarAssetClient::new(e, &contract.address())
}

/// Spins up an initialized contract plus a funded depositor ready to call
/// `lock_funds_anonymous`.
fn setup(env: &Env) -> (BountyEscrowContractClient<'_>, Address, Address, Address) {
    env.mock_all_auths();
    let admin = Address::generate(env);
    let depositor = Address::generate(env);
    let recipient = Address::generate(env);
    let token_admin = Address::generate(env);
    let token_client = create_token(env, &token_admin);
    let token_admin_address = token_client.address.clone();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(env, &contract_id);
    client.init(&admin, &token_admin_address);
    token_client.mint(&depositor, &10_000_000);

    (client, admin, depositor, recipient)
}

/// `get_escrow_info` must never return the depositor for an anonymously-locked
/// bounty prior to resolution: the anon record lives under `DataKey::EscrowAnon`,
/// which `get_escrow_info` does not consult, so the call must fail closed
/// (`BountyNotFound`) rather than fall back to any address-bearing record.
#[test]
fn test_get_escrow_info_does_not_leak_depositor_for_anonymous_lock() {
    let env = Env::default();
    let (client, _admin, depositor, _recipient) = setup(&env);

    let bounty_id = 1u64;
    let commitment = BytesN::from_array(&env, &[7u8; 32]);
    let deadline = env.ledger().timestamp() + 1_000;
    client.lock_funds_anonymous(&depositor, &commitment, &bounty_id, &1_000i128, &deadline);

    let result = client.try_get_escrow_info(&bounty_id);
    assert!(
        result.is_err(),
        "get_escrow_info must not resolve an anonymous bounty before refund_resolved"
    );
}

/// `get_metadata` carries no depositor-identifying field at all; confirm it
/// keeps returning the plain default record for an anonymously-locked bounty
/// that never had `update_metadata` called on it, i.e. no identity data is
/// implicitly attached to metadata storage during an anonymous lock.
#[test]
fn test_get_metadata_has_no_identity_for_anonymous_lock() {
    let env = Env::default();
    let (client, _admin, depositor, _recipient) = setup(&env);

    let bounty_id = 2u64;
    let commitment = BytesN::from_array(&env, &[9u8; 32]);
    let deadline = env.ledger().timestamp() + 1_000;
    client.lock_funds_anonymous(&depositor, &commitment, &bounty_id, &1_000i128, &deadline);

    let meta = client.get_metadata(&bounty_id);
    assert_eq!(meta.repo_id, 0);
    assert_eq!(meta.issue_id, 0);
    assert_eq!(meta.bounty_type, soroban_sdk::String::from_str(&env, ""));
    assert_eq!(meta.reference_hash, None);
}

/// Storage-layout guard: `lock_funds_anonymous` must persist only the
/// `EscrowAnon` variant (32-byte commitment) and must never also create a
/// `DataKey::Escrow` entry (which stores a plain `Address`) or add the
/// depositor to `DataKey::DepositorIndex` — the index that a future
/// `query_escrows_by_depositor` implementation would read. Either would leak
/// the depositor through a path this suite doesn't otherwise exercise.
#[test]
fn test_anonymous_lock_writes_only_commitment_no_address_indexes() {
    let env = Env::default();
    let (client, _admin, depositor, _recipient) = setup(&env);

    let bounty_id = 3u64;
    let commitment = BytesN::from_array(&env, &[3u8; 32]);
    let deadline = env.ledger().timestamp() + 1_000;
    client.lock_funds_anonymous(&depositor, &commitment, &bounty_id, &1_000i128, &deadline);

    env.as_contract(&client.address, || {
        assert!(
            !env.storage().persistent().has(&DataKey::Escrow(bounty_id)),
            "anonymous lock must not create an address-bearing Escrow record"
        );
        assert!(
            env.storage()
                .persistent()
                .has(&DataKey::EscrowAnon(bounty_id)),
            "anonymous lock must create the commitment-only EscrowAnon record"
        );
        let depositor_index: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::DepositorIndex(depositor.clone()))
            .unwrap_or(Vec::new(&env));
        assert!(
            !depositor_index.contains(&bounty_id),
            "anonymous lock must not link the depositor to the bounty via DepositorIndex"
        );
    });
}

/// End-to-end: identity stays hidden from `get_escrow_info` through the whole
/// pre-resolution lifetime, and only the resolver-driven `refund_resolved`
/// call (post-deadline) can move funds to a chosen recipient. That recipient
/// is the intended point where identity becomes visible — not any query call.
#[test]
fn test_identity_hidden_until_resolver_driven_refund() {
    let env = Env::default();
    let (client, admin, depositor, recipient) = setup(&env);

    let bounty_id = 4u64;
    let commitment = BytesN::from_array(&env, &[5u8; 32]);
    let deadline = env.ledger().timestamp() + 1_000;
    client.lock_funds_anonymous(&depositor, &commitment, &bounty_id, &1_000i128, &deadline);

    // Pre-resolution: still hidden.
    assert!(client.try_get_escrow_info(&bounty_id).is_err());

    // Resolver is configured by the admin, then drives the refund.
    client.set_anonymous_resolver(&Some(admin.clone()));
    env.ledger().set_timestamp(deadline + 1);
    client.refund_resolved(&bounty_id, &recipient);

    // Identity became visible only through the recipient of this call, not
    // through get_escrow_info, which still has nothing to return for this id.
    assert!(client.try_get_escrow_info(&bounty_id).is_err());
}
