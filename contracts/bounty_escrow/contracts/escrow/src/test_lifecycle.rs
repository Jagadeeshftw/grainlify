//! # Lifecycle Tests — Initialization & Events (Issue #757)
#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger, LedgerInfo, MockAuth, MockAuthInvoke},
    token::{Client as TokenClient, StellarAssetClient},
    Address, Env, IntoVal, Symbol, TryIntoVal,
};

fn create_token_contract<'a>(
    e: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let contract_address = e
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    (
        token::Client::new(e, &contract_address),
        token::StellarAssetClient::new(e, &contract_address),
    )
}

fn create_escrow_contract<'a>(e: &Env) -> BountyEscrowContractClient<'a> {
    let contract_id = e.register_contract(None, BountyEscrowContract);
    BountyEscrowContractClient::new(e, &contract_id)
}

// ─── Shared helpers & constants ────────────────────────────────────────────

/// A timestamp well in the past — used as "current" time for locked escrows.
const BASE_TS: u64 = 1_000_000;
/// A deadline sufficiently far in the future relative to BASE_TS.
const FUTURE_DL: u64 = BASE_TS + 86_400;
/// Default token amount used across lifecycle tests.
const DEFAULT_AMOUNT: i128 = 10_000;

/// Test context carrying all live handles for a single test scenario.
struct Ctx {
    env: Env,
    client: BountyEscrowContractClient<'static>,
    token_id: Address,
    admin: Address,
    depositor: Address,
    contributor: Address,
}

/// Construct a fresh Env + contract + token without calling `init`.
fn setup() -> Ctx {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set(LedgerInfo {
        timestamp: BASE_TS,
        ..Default::default()
    });

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);

    let token_ref = env.register_stellar_asset_contract_v2(Address::generate(&env));
    let token_id = token_ref.address();
    let sac: token::StellarAssetClient<'static> =
        unsafe { core::mem::transmute(token::StellarAssetClient::new(&env, &token_id)) };
    sac.mint(&depositor, &1_000_000);

    let cid = env.register_contract(None, BountyEscrowContract);
    let client: BountyEscrowContractClient<'static> =
        unsafe { core::mem::transmute(BountyEscrowContractClient::new(&env, &cid)) };

    Ctx {
        env,
        client,
        token_id,
        admin,
        depositor,
        contributor,
    }
}

fn setup_init() -> Ctx {
    let ctx = setup();
    ctx.client.init(&ctx.admin, &ctx.token_id);
    ctx
}

fn lock(ctx: &Ctx, bounty_id: u64, amount: i128) {
    ctx.client
        .lock_funds(&ctx.depositor, &bounty_id, &amount, &FUTURE_DL);
}

/// Returns true if any event in `events` has the given topic symbol.
fn has_topic(
    _env: &Env,
    events: &soroban_sdk::Vec<(
        soroban_sdk::Address,
        soroban_sdk::Vec<soroban_sdk::Val>,
        soroban_sdk::Val,
    )>,
    topic: Symbol,
) -> bool {
    for (_contract, topics, _data) in events.iter() {
        for t in topics.iter() {
            if let Ok(s) =
                <Symbol as soroban_sdk::TryFromVal<Env, soroban_sdk::Val>>::try_from_val(_env, &t)
            {
                if s == topic {
                    return true;
                }
            }
        }
    }
    false
}

/// Find the data payload of the first event that has the given topic symbol.
fn find_data(
    _env: &Env,
    events: &soroban_sdk::Vec<(
        soroban_sdk::Address,
        soroban_sdk::Vec<soroban_sdk::Val>,
        soroban_sdk::Val,
    )>,
    topic: Symbol,
) -> Option<soroban_sdk::Val> {
    for (_contract, topics, data) in events.iter() {
        for t in topics.iter() {
            if let Ok(s) =
                <Symbol as soroban_sdk::TryFromVal<Env, soroban_sdk::Val>>::try_from_val(_env, &t)
            {
                if s == topic {
                    return Some(data);
                }
            }
        }
    }
    None
}

#[test]
fn test_full_bounty_lifecycle_with_refund() {
    let env = Env::default();
    // env.mock_all_auths();

    // 1. Setup participants
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let _bystander = Address::generate(&env);

    // 2. Setup token and contract
    let (token_client, token_admin) = create_token_contract(&env, &admin);
    let escrow_client = create_escrow_contract(&env);

    // 3. Initialize contract
    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &escrow_client.address,
            fn_name: "init",
            args: (&admin, &token_client.address).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    escrow_client.init(&admin, &token_client.address);
    assert_eq!(escrow_client.get_balance(), 0);

    // 4.  Mint tokens to depositor
    env.mock_auths(&[MockAuth {
        address: &admin, // token admin is the admin
        invoke: &MockAuthInvoke {
            contract: &token_client.address,
            fn_name: "mint",
            args: (depositor.clone(), 10000i128).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    token_admin.mint(&depositor, &10000);

    // 5. Lock funds for a bounty
    let bounty_id = 101u64;
    let initial_amount = 5000i128;
    let deadline = env.ledger().timestamp() + 86400; // 1 day

    env.mock_auths(&[MockAuth {
        address: &depositor,
        invoke: &MockAuthInvoke {
            contract: &escrow_client.address,
            fn_name: "lock_funds",
            args: (depositor.clone(), bounty_id, initial_amount, deadline).into_val(&env),
            sub_invokes: &[MockAuthInvoke {
                contract: &token_client.address,
                fn_name: "transfer",
                args: (
                    depositor.clone(),
                    escrow_client.address.clone(),
                    initial_amount,
                )
                    .into_val(&env),
                sub_invokes: &[],
            }],
        },
    }]);
    assert_eq!(token_client.balance(&depositor), 10000);

    escrow_client.lock_funds(&depositor, &bounty_id, &initial_amount, &deadline);

    // Verify Locked state
    let info = escrow_client.get_escrow(&bounty_id);
    assert_eq!(info.status, EscrowStatus::Locked);
    assert_eq!(info.amount, initial_amount);
    assert_eq!(info.remaining_amount, initial_amount);
    assert_eq!(escrow_client.get_balance(), initial_amount);
    assert_eq!(token_client.balance(&depositor), 5000);

    // 6. Test authorization failure for non-admin trying to release funds
    // Attempt to release funds as non-admin (should fail)
    let non_admin_result = escrow_client.try_release_funds(&bounty_id, &contributor);
    assert!(
        non_admin_result.is_err(),
        "Non-admin should not be able to release funds"
    );

    // Verify the error is Unauthorized (error code 7)
    match non_admin_result {
        Err(_e) => {
            // Expected: non-admin cannot release
        }
        Ok(_) => panic!("expected release to fail for non-admin"),
    }

    // 7. Continue with the refund flow (Administrative action: Approve a partial refund)
    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &escrow_client.address,
            fn_name: "approve_refund",
            args: (bounty_id, 2000i128, depositor.clone(), RefundMode::Partial).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    // Approve a partial refund
    let refund_amount = 2000;
    escrow_client.approve_refund(&bounty_id, &refund_amount, &depositor, &RefundMode::Partial);

    // Verify eligibility
    let (can_refund, deadline_passed, remaining, approval) =
        escrow_client.get_refund_eligibility(&bounty_id);
    assert!(can_refund);
    assert!(!deadline_passed);
    assert_eq!(remaining, initial_amount);
    assert!(approval.is_some());

    // 8. Execute partial refund payout
    env.mock_auths(&[
        MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &escrow_client.address,
                fn_name: "refund",
                args: (bounty_id,).into_val(&env),
                sub_invokes: &[],
            },
        },
        MockAuth {
            address: &depositor,
            invoke: &MockAuthInvoke {
                contract: &escrow_client.address,
                fn_name: "refund",
                args: (bounty_id,).into_val(&env),
                sub_invokes: &[MockAuthInvoke {
                    contract: &token_client.address,
                    fn_name: "transfer",
                    args: (
                        escrow_client.address.clone(),
                        depositor.clone(),
                        refund_amount,
                    )
                        .into_val(&env),
                    sub_invokes: &[],
                }],
            },
        },
    ]);
    escrow_client.refund(&bounty_id);

    // Verify partially refunded state
    let info = escrow_client.get_escrow(&bounty_id);
    assert_eq!(info.status, EscrowStatus::PartiallyRefunded);
    assert_eq!(info.remaining_amount, initial_amount - refund_amount);
    assert_eq!(token_client.balance(&depositor), 5000 + refund_amount);
    assert_eq!(escrow_client.get_balance(), initial_amount - refund_amount);

    // Verify history
    let history = escrow_client.get_refund_history(&bounty_id);
    assert_eq!(history.len(), 1);
    assert_eq!(history.get(0).unwrap().amount, refund_amount);
    assert_eq!(history.get(0).unwrap().mode, RefundMode::Partial);

    // 9. Approve and execute final full refund payout
    let final_amount = info.remaining_amount;

    // Set auth for final approval
    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &escrow_client.address,
            fn_name: "approve_refund",
            args: (bounty_id, final_amount, depositor.clone(), RefundMode::Full).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    escrow_client.approve_refund(&bounty_id, &final_amount, &depositor, &RefundMode::Full);

    // Set auth for final refund with nested token transfer
    env.mock_auths(&[
        MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &escrow_client.address,
                fn_name: "refund",
                args: (bounty_id,).into_val(&env),
                sub_invokes: &[],
            },
        },
        MockAuth {
            address: &depositor,
            invoke: &MockAuthInvoke {
                contract: &escrow_client.address,
                fn_name: "refund",
                args: (bounty_id,).into_val(&env),
                sub_invokes: &[MockAuthInvoke {
                    contract: &token_client.address,
                    fn_name: "transfer",
                    args: (
                        escrow_client.address.clone(),
                        depositor.clone(),
                        final_amount,
                    )
                        .into_val(&env),
                    sub_invokes: &[],
                }],
            },
        },
    ]);

    escrow_client.refund(&bounty_id);

    // Verify final state
    let final_info = escrow_client.get_escrow(&bounty_id);
    assert_eq!(final_info.status, EscrowStatus::Refunded);
    assert_eq!(final_info.remaining_amount, 0);
    assert_eq!(token_client.balance(&depositor), 10000);
    assert_eq!(escrow_client.get_balance(), 0);

    // Verify full history
    let full_history = escrow_client.get_refund_history(&bounty_id);
    assert_eq!(full_history.len(), 2);
    assert_eq!(full_history.get(1).unwrap().amount, final_amount);
    assert_eq!(full_history.get(1).unwrap().mode, RefundMode::Full);
}

/// Admin `release_funds` after `lock_funds` moves the full balance from the escrow SAC to the contributor.
#[test]
fn test_lock_to_release_sac_transfers() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let (token_client, token_admin) = create_token_contract(&env, &admin);
    let escrow_client = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);
    let amount = 7_500i128;
    token_admin.mint(&depositor, &amount);

    let bounty_id = 303u64;
    let deadline = env.ledger().timestamp() + 86_400;
    assert_eq!(token_client.balance(&depositor), amount);
    assert_eq!(token_client.balance(&escrow_client.address), 0);

    escrow_client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

    assert_eq!(token_client.balance(&depositor), 0);
    assert_eq!(token_client.balance(&escrow_client.address), amount);
    let info = escrow_client.get_escrow(&bounty_id);
    assert_eq!(info.status, EscrowStatus::Locked);

    escrow_client.release_funds(&bounty_id, &contributor);

    assert_eq!(token_client.balance(&contributor), amount);
    assert_eq!(token_client.balance(&escrow_client.address), 0);
    let info = escrow_client.get_escrow(&bounty_id);
    assert_eq!(info.status, EscrowStatus::Released);
    assert_eq!(info.remaining_amount, 0);
}

/// Illegal transition: cannot release twice after full release.
#[test]
fn test_double_release_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let (token_client, token_admin) = create_token_contract(&env, &admin);
    let escrow_client = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);
    token_admin.mint(&depositor, &500i128);
    let bounty_id = 404u64;
    let deadline = env.ledger().timestamp() + 3600;
    escrow_client.lock_funds(&depositor, &bounty_id, &500i128, &deadline);
    escrow_client.release_funds(&bounty_id, &contributor);

    let second = escrow_client.try_release_funds(&bounty_id, &contributor);
    assert!(second.is_err());
}

#[test]
fn test_refund_after_deadline_no_approval_needed() {
    let ctx = setup_init();
    lock(&ctx, 1, DEFAULT_AMOUNT);
    // Move time past the deadline
    ctx.env.ledger().set(LedgerInfo {
        timestamp: FUTURE_DL + 1,
        ..Default::default()
    });
    // Should succeed without any admin approval
    ctx.client.refund(&1u64);
    let info = ctx.client.get_escrow(&1u64);
    assert_eq!(info.status, EscrowStatus::Refunded);
    assert_eq!(info.remaining_amount, 0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// INIT — happy paths
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_init_happy_path() {
    let ctx = setup();
    assert!(ctx.client.try_init(&ctx.admin, &ctx.token_id).is_ok());
}

#[test]
fn test_init_emits_bounty_escrow_initialized() {
    let ctx = setup();
    ctx.client.init(&ctx.admin, &ctx.token_id);
    let all = ctx.env.events().all();
    assert!(
        has_topic(&ctx.env, &all, symbol_short!("init")),
        "BountyEscrowInitialized must be emitted"
    );
}

#[test]
fn test_init_event_carries_version_v2() {
    let ctx = setup();
    ctx.client.init(&ctx.admin, &ctx.token_id);
    let all = ctx.env.events().all();
    let data = find_data(&ctx.env, &all, symbol_short!("init")).expect("init event missing");
    let p: events::BountyEscrowInitialized = data.into_val(&ctx.env);
    assert_eq!(p.version, EVENT_VERSION_V2);
}

#[test]
fn test_init_event_fields_match_inputs() {
    let ctx = setup();
    ctx.client.init(&ctx.admin, &ctx.token_id);
    let all = ctx.env.events().all();
    let data = find_data(&ctx.env, &all, symbol_short!("init")).expect("init event missing");
    let p: events::BountyEscrowInitialized = data.into_val(&ctx.env);
    assert_eq!(p.version, EVENT_VERSION_V2);
    assert_eq!(p.admin, ctx.admin);
    assert_eq!(p.token, ctx.token_id);
    assert_eq!(p.timestamp, BASE_TS);
}

#[test]
fn test_balance_zero_after_init() {
    let ctx = setup_init();
    assert_eq!(ctx.client.get_balance(), 0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// INIT — error paths
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_init_already_initialized_error() {
    let ctx = setup_init();
    let r = ctx.client.try_init(&ctx.admin, &ctx.token_id);
    assert!(r.is_err());
    assert_eq!(r.unwrap_err().unwrap(), Error::AlreadyInitialized);
}

#[test]
fn test_init_admin_equals_token_rejected() {
    let ctx = setup();
    let dup = ctx.token_id.clone();
    let r = ctx.client.try_init(&dup, &ctx.token_id);
    assert!(r.is_err(), "admin == token must be rejected");
}

// ═══════════════════════════════════════════════════════════════════════════════
// INIT WITH NETWORK
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_init_with_network_happy_path() {
    let ctx = setup();
    let chain = soroban_sdk::String::from_str(&ctx.env, "stellar");
    let net = soroban_sdk::String::from_str(&ctx.env, "mainnet");
    assert!(ctx
        .client
        .try_init_with_network(&ctx.admin, &ctx.token_id, &chain, &net)
        .is_ok());
    assert!(ctx.client.get_chain_id().is_some());
    assert!(ctx.client.get_network_id().is_some());
}

#[test]
fn test_init_with_network_replay_rejected() {
    let ctx = setup();
    let chain = soroban_sdk::String::from_str(&ctx.env, "stellar");
    let net = soroban_sdk::String::from_str(&ctx.env, "testnet");
    ctx.client
        .init_with_network(&ctx.admin, &ctx.token_id, &chain, &net);
    let r = ctx
        .client
        .try_init_with_network(&ctx.admin, &ctx.token_id, &chain, &net);
    assert!(r.is_err());
    assert_eq!(r.unwrap_err().unwrap(), Error::AlreadyInitialized);
}

// ═══════════════════════════════════════════════════════════════════════════════
// LOCK FUNDS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_lock_funds_after_init() {
    let ctx = setup_init();
    lock(&ctx, 1, DEFAULT_AMOUNT);
    let info = ctx.client.get_escrow(&1u64);
    assert_eq!(info.status, EscrowStatus::Locked);
    assert_eq!(info.amount, DEFAULT_AMOUNT);
}

#[test]
fn test_lock_funds_emits_funds_locked() {
    let ctx = setup_init();
    lock(&ctx, 1, DEFAULT_AMOUNT);
    let all = ctx.env.events().all();
    assert!(
        has_topic(&ctx.env, &all, symbol_short!("f_lock")),
        "FundsLocked must be emitted"
    );
}

#[test]
fn test_lock_funds_event_fields() {
    let ctx = setup_init();
    lock(&ctx, 99, DEFAULT_AMOUNT);
    let all = ctx.env.events().all();
    let data = find_data(&ctx.env, &all, symbol_short!("f_lock")).expect("f_lock missing");
    let p: events::FundsLocked = data.into_val(&ctx.env);
    assert_eq!(p.version, EVENT_VERSION_V2);
    assert_eq!(p.bounty_id, 99u64);
    assert_eq!(p.amount, DEFAULT_AMOUNT);
    assert_eq!(p.depositor, ctx.depositor);
    assert_eq!(p.deadline, FUTURE_DL);
}

#[test]
fn test_get_balance_reflects_locked_funds() {
    let ctx = setup_init();
    assert_eq!(ctx.client.get_balance(), 0);
    lock(&ctx, 1, DEFAULT_AMOUNT);
    assert_eq!(ctx.client.get_balance(), DEFAULT_AMOUNT);
    // Advance time to bypass cooldown period (default 60 seconds)
    ctx.env.ledger().set(LedgerInfo {
        timestamp: BASE_TS + 61,
        ..Default::default()
    });
    lock(&ctx, 2, 5_000);
    assert_eq!(ctx.client.get_balance(), DEFAULT_AMOUNT + 5_000);
}

#[test]
fn test_lock_funds_before_init_fails() {
    let ctx = setup();
    let r = ctx
        .client
        .try_lock_funds(&ctx.depositor, &1u64, &DEFAULT_AMOUNT, &FUTURE_DL);
    assert!(r.is_err());
    assert_eq!(r.unwrap_err().unwrap(), Error::NotInitialized);
}

#[test]
fn test_lock_funds_zero_amount_fails() {
    let ctx = setup_init();
    let r = ctx
        .client
        .try_lock_funds(&ctx.depositor, &1u64, &0i128, &FUTURE_DL);
    assert!(r.is_err(), "zero amount must be rejected");
}

#[test]
fn test_lock_funds_duplicate_bounty_fails() {
    let ctx = setup_init();
    lock(&ctx, 7, DEFAULT_AMOUNT);
    // Advance time to bypass cooldown period (default 60 seconds)
    ctx.env.ledger().set(LedgerInfo {
        timestamp: BASE_TS + 61,
        ..Default::default()
    });
    let r = ctx
        .client
        .try_lock_funds(&ctx.depositor, &7u64, &DEFAULT_AMOUNT, &FUTURE_DL);
    assert!(r.is_err());
    assert_eq!(r.unwrap_err().unwrap(), Error::BountyExists);
}

// ═══════════════════════════════════════════════════════════════════════════════
// RELEASE FUNDS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_release_funds_happy_path() {
    let ctx = setup_init();
    lock(&ctx, 1, DEFAULT_AMOUNT);
    ctx.client.release_funds(&1u64, &ctx.contributor);
    let info = ctx.client.get_escrow(&1u64);
    assert_eq!(info.status, EscrowStatus::Released);
    assert_eq!(info.remaining_amount, 0);
}

#[test]
fn test_release_funds_emits_event() {
    let ctx = setup_init();
    lock(&ctx, 1, DEFAULT_AMOUNT);
    ctx.client.release_funds(&1u64, &ctx.contributor);
    let all = ctx.env.events().all();
    assert!(
        has_topic(&ctx.env, &all, symbol_short!("f_rel")),
        "FundsReleased must be emitted"
    );
}

#[test]
fn test_release_funds_event_fields() {
    let ctx = setup_init();
    lock(&ctx, 1, DEFAULT_AMOUNT);
    ctx.client.release_funds(&1u64, &ctx.contributor);
    let all = ctx.env.events().all();
    let data = find_data(&ctx.env, &all, symbol_short!("f_rel")).expect("f_rel missing");
    let p: events::FundsReleased = data.into_val(&ctx.env);
    assert_eq!(p.version, EVENT_VERSION_V2);
    assert_eq!(p.bounty_id, 1u64);
    assert_eq!(p.amount, DEFAULT_AMOUNT);
    assert_eq!(p.recipient, ctx.contributor);
}

#[test]
fn test_release_funds_bounty_not_found() {
    let ctx = setup_init();
    let r = ctx.client.try_release_funds(&99u64, &ctx.contributor);
    assert!(r.is_err());
    assert_eq!(r.unwrap_err().unwrap(), Error::BountyNotFound);
}

#[test]
fn test_release_funds_double_release_fails() {
    let ctx = setup_init();
    lock(&ctx, 1, DEFAULT_AMOUNT);
    ctx.client.release_funds(&1u64, &ctx.contributor);
    let r = ctx.client.try_release_funds(&1u64, &ctx.contributor);
    assert!(r.is_err());
    assert_eq!(r.unwrap_err().unwrap(), Error::FundsNotLocked);
}

// ═══════════════════════════════════════════════════════════════════════════════
// REFUND
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_refund_after_deadline_happy_path() {
    let ctx = setup_init();
    lock(&ctx, 1, DEFAULT_AMOUNT);
    ctx.env.ledger().set(LedgerInfo {
        timestamp: FUTURE_DL + 1,
        ..Default::default()
    });
    ctx.client.refund(&1u64);
    let info = ctx.client.get_escrow(&1u64);
    assert_eq!(info.status, EscrowStatus::Refunded);
    assert_eq!(info.remaining_amount, 0);
}

#[test]
fn test_refund_emits_event() {
    let ctx = setup_init();
    lock(&ctx, 1, DEFAULT_AMOUNT);
    ctx.env.ledger().set(LedgerInfo {
        timestamp: FUTURE_DL + 1,
        ..Default::default()
    });
    ctx.client.refund(&1u64);
    let all = ctx.env.events().all();
    assert!(
        has_topic(&ctx.env, &all, symbol_short!("f_ref")),
        "FundsRefunded must be emitted"
    );
}

#[test]
fn test_refund_event_fields() {
    let ctx = setup_init();
    lock(&ctx, 1, DEFAULT_AMOUNT);
    ctx.env.ledger().set(LedgerInfo {
        timestamp: FUTURE_DL + 1,
        ..Default::default()
    });
    ctx.client.refund(&1u64);
    let all = ctx.env.events().all();
    let data = find_data(&ctx.env, &all, symbol_short!("f_ref")).expect("f_ref missing");
    let p: events::FundsRefunded = data.into_val(&ctx.env);
    assert_eq!(p.version, EVENT_VERSION_V2);
    assert_eq!(p.bounty_id, 1u64);
    assert_eq!(p.amount, DEFAULT_AMOUNT);
    assert_eq!(p.refund_to, ctx.depositor);
}

#[test]
fn test_refund_before_deadline_no_approval_fails() {
    let ctx = setup_init();
    lock(&ctx, 1, DEFAULT_AMOUNT);
    let r = ctx.client.try_refund(&1u64);
    assert!(r.is_err());
    assert_eq!(r.unwrap_err().unwrap(), Error::DeadlineNotPassed);
}

#[test]
fn test_refund_already_released_fails() {
    let ctx = setup_init();
    lock(&ctx, 1, DEFAULT_AMOUNT);
    ctx.client.release_funds(&1u64, &ctx.contributor);
    ctx.env.ledger().set(LedgerInfo {
        timestamp: FUTURE_DL + 1,
        ..Default::default()
    });
    let r = ctx.client.try_refund(&1u64);
    assert!(r.is_err());
    assert_eq!(r.unwrap_err().unwrap(), Error::FundsNotLocked);
}

#[test]
fn test_early_refund_with_admin_approval() {
    let ctx = setup_init();
    lock(&ctx, 1, DEFAULT_AMOUNT);
    ctx.client
        .approve_refund(&1u64, &DEFAULT_AMOUNT, &ctx.depositor, &RefundMode::Full);
    ctx.client.refund(&1u64);
    assert_eq!(
        ctx.client.get_escrow(&1u64).status,
        EscrowStatus::Refunded
    );
}

#[test]
fn test_partial_refund_flow() {
    let ctx = setup_init();
    let partial = 3_000i128;
    lock(&ctx, 1, DEFAULT_AMOUNT);
    ctx.client
        .approve_refund(&1u64, &partial, &ctx.depositor, &RefundMode::Partial);
    ctx.client.refund(&1u64);
    let info = ctx.client.get_escrow(&1u64);
    assert_eq!(info.status, EscrowStatus::PartiallyRefunded);
    assert_eq!(info.remaining_amount, DEFAULT_AMOUNT - partial);
    assert_eq!(
        ctx.client.get_refund_history(&1u64).get(0).unwrap().amount,
        partial
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// PAUSE / DEPRECATION / MAINTENANCE
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_lock_paused_blocks_lock_funds() {
    let ctx = setup_init();
    ctx.client.set_paused(&Some(true), &None, &None, &None);
    let r = ctx
        .client
        .try_lock_funds(&ctx.depositor, &1u64, &DEFAULT_AMOUNT, &FUTURE_DL);
    assert!(r.is_err());
    assert_eq!(r.unwrap_err().unwrap(), Error::FundsPaused);
}

#[test]
fn test_release_paused_blocks_release() {
    let ctx = setup_init();
    lock(&ctx, 1, DEFAULT_AMOUNT);
    ctx.client.set_paused(&None, &Some(true), &None, &None);
    let r = ctx.client.try_release_funds(&1u64, &ctx.contributor);
    assert!(r.is_err());
    assert_eq!(r.unwrap_err().unwrap(), Error::FundsPaused);
}

#[test]
fn test_deprecated_blocks_lock_funds() {
    let ctx = setup_init();
    ctx.client.set_deprecated(&true, &None);
    let r = ctx
        .client
        .try_lock_funds(&ctx.depositor, &1u64, &DEFAULT_AMOUNT, &FUTURE_DL);
    assert!(r.is_err());
    assert_eq!(r.unwrap_err().unwrap(), Error::ContractDeprecated);
}

#[test]
fn test_maintenance_mode_blocks_lock() {
    let ctx = setup_init();
    ctx.client.set_maintenance_mode(&true, &None);
    let r = ctx
        .client
        .try_lock_funds(&ctx.depositor, &1u64, &DEFAULT_AMOUNT, &FUTURE_DL);
    assert!(r.is_err());
    assert_eq!(r.unwrap_err().unwrap(), Error::FundsPaused);
}

// ═══════════════════════════════════════════════════════════════════════════════
// EMERGENCY WITHDRAW
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_emergency_withdraw_requires_paused() {
    let ctx = setup_init();
    lock(&ctx, 1, DEFAULT_AMOUNT);
    let r = ctx.client.try_emergency_withdraw(&ctx.admin);
    assert!(r.is_err());
    assert_eq!(r.unwrap_err().unwrap(), Error::NotPaused);
}

#[test]
fn test_emergency_withdraw_happy_path() {
    let ctx = setup_init();
    lock(&ctx, 1, DEFAULT_AMOUNT);
    ctx.client.set_paused(&Some(true), &None, &None, &None);
    let target = Address::generate(&ctx.env);
    let tc = TokenClient::new(&ctx.env, &ctx.token_id);
    let before = tc.balance(&target);
    ctx.client.emergency_withdraw(&target);
    assert_eq!(tc.balance(&target) - before, DEFAULT_AMOUNT);
    assert_eq!(ctx.client.get_balance(), 0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// OPERATIONAL STATE EVENTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_deprecation_emits_event() {
    let ctx = setup_init();
    ctx.client.set_deprecated(&true, &None);
    let all = ctx.env.events().all();
    let data = find_data(&ctx.env, &all, symbol_short!("deprec")).expect("deprec event missing");
    let p: events::DeprecationStateChanged = data.into_val(&ctx.env);
    assert!(p.deprecated);
    assert_eq!(p.admin, ctx.admin);
}

#[test]
fn test_maintenance_mode_emits_event() {
    let ctx = setup_init();
    ctx.client.set_maintenance_mode(&true, &None);
    let all = ctx.env.events().all();
    let data = find_data(&ctx.env, &all, symbol_short!("maint")).expect("maint event missing");
    let p: events::MaintenanceModeChanged = data.into_val(&ctx.env);
    assert!(p.enabled);
    assert_eq!(p.admin, ctx.admin);
}

// ═══════════════════════════════════════════════════════════════════════════════
// ALL VERSIONED EVENTS CARRY EVENT_VERSION_V2
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_all_lifecycle_events_carry_v2_version() {
    let ctx = setup_init();
    lock(&ctx, 1, DEFAULT_AMOUNT);
    ctx.client.release_funds(&1u64, &ctx.contributor);

    let all = ctx.env.events().all();
    for i in 0..all.len() {
        let (_, topics, data) = all.get(i).unwrap();
        if topics.len() == 0 {
            continue;
        }
        let result: Result<Symbol, _> = topics.get(0).unwrap().try_into_val(&ctx.env);
        let Ok(sym) = result else { continue };
        {
            continue;
        };

        if sym == symbol_short!("init") {
            let p: events::BountyEscrowInitialized = data.into_val(&ctx.env);
            assert_eq!(p.version, EVENT_VERSION_V2, "init: wrong version");
        } else if sym == symbol_short!("f_lock") {
            let p: events::FundsLocked = data.into_val(&ctx.env);
            assert_eq!(p.version, EVENT_VERSION_V2, "f_lock: wrong version");
        } else if sym == symbol_short!("f_rel") {
            let p: events::FundsReleased = data.into_val(&ctx.env);
            assert_eq!(p.version, EVENT_VERSION_V2, "f_rel: wrong version");
        } else if sym == symbol_short!("f_ref") {
            let p: events::FundsRefunded = data.into_val(&ctx.env);
            assert_eq!(p.version, EVENT_VERSION_V2, "f_ref: wrong version");
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// NOT-FOUND GUARDS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_get_escrow_not_found() {
    let ctx = setup_init();
    let r = ctx.client.try_get_escrow(&9999u64);
    assert!(r.is_err());
    assert_eq!(r.unwrap_err().unwrap(), Error::BountyNotFound);
}

#[test]
fn test_refund_bounty_not_found() {
    let ctx = setup_init();
    let r = ctx.client.try_refund(&9999u64);
    assert!(r.is_err());
    assert_eq!(r.unwrap_err().unwrap(), Error::BountyNotFound);
}

/// Admin approves an early refund (before deadline) to a custom recipient.
/// Verifies that the approval is consumed after execution and the recipient
/// receives the funds rather than the depositor.
#[test]
fn test_admin_early_refund_to_custom_recipient() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (token_client, token_admin) = create_token_contract(&env, &admin);
    let escrow_client = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);
    token_admin.mint(&depositor, &5000);

    let bounty_id = 301u64;
    let deadline = env.ledger().timestamp() + 86400;
    escrow_client.lock_funds(&depositor, &bounty_id, &5000, &deadline);

    // Admin approves full refund to a custom recipient before deadline
    escrow_client.approve_refund(&bounty_id, &5000, &recipient, &RefundMode::Full);

    let (can_refund, deadline_passed, remaining, approval) =
        escrow_client.get_refund_eligibility(&bounty_id);
    assert!(can_refund);
    assert!(!deadline_passed);
    assert_eq!(remaining, 5000);
    assert!(approval.is_some());

    escrow_client.refund(&bounty_id);

    let info = escrow_client.get_escrow(&bounty_id);
    assert_eq!(info.status, EscrowStatus::Refunded);
    assert_eq!(info.remaining_amount, 0);
    // Funds went to the custom recipient, not the depositor
    assert_eq!(token_client.balance(&recipient), 5000);
    assert_eq!(token_client.balance(&depositor), 0);

    // Approval is consumed — a second refund attempt must fail
    let res = escrow_client.try_refund(&bounty_id);
    assert!(res.is_err());
}

/// Refund on a bounty that does not exist returns BountyNotFound.
#[test]
fn test_refund_nonexistent_bounty_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (token_client, _) = create_token_contract(&env, &admin);
    let escrow_client = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);

    let res = escrow_client.try_refund(&9999u64);
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().unwrap(), Error::BountyNotFound);
}

/// Refund on an already-refunded escrow returns FundsNotLocked.
#[test]
fn test_refund_already_refunded_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token_client, token_admin) = create_token_contract(&env, &admin);
    let escrow_client = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);
    token_admin.mint(&depositor, &1000);

    let bounty_id = 302u64;
    let deadline = env.ledger().timestamp() + 100;
    escrow_client.lock_funds(&depositor, &bounty_id, &1000, &deadline);

    env.ledger().set_timestamp(deadline + 1);
    escrow_client.refund(&bounty_id);

    // Second refund must fail
    let res = escrow_client.try_refund(&bounty_id);
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().unwrap(), Error::FundsNotLocked);
}

/// Refund is blocked when the refund operation is paused.
#[test]
fn test_refund_blocked_when_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token_client, token_admin) = create_token_contract(&env, &admin);
    let escrow_client = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);
    token_admin.mint(&depositor, &1000);

    let bounty_id = 303u64;
    let deadline = env.ledger().timestamp() + 100;
    escrow_client.lock_funds(&depositor, &bounty_id, &1000, &deadline);

    // Pause refunds
    escrow_client.set_paused(&None, &None, &Some(true), &None);

    env.ledger().set_timestamp(deadline + 1);

    let res = escrow_client.try_refund(&bounty_id);
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().unwrap(), Error::FundsPaused);

    // Unpause and verify refund works
    escrow_client.set_paused(&None, &None, &Some(false), &None);
    escrow_client.refund(&bounty_id);

    let info = escrow_client.get_escrow(&bounty_id);
    assert_eq!(info.status, EscrowStatus::Refunded);
}

/// Sequential partial refunds drain the escrow correctly and the final
/// refund transitions status to Refunded.
#[test]
fn test_sequential_partial_refunds_drain_escrow() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token_client, token_admin) = create_token_contract(&env, &admin);
    let escrow_client = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);
    token_admin.mint(&depositor, &3000);

    let bounty_id = 304u64;
    let deadline = env.ledger().timestamp() + 86400;
    escrow_client.lock_funds(&depositor, &bounty_id, &3000, &deadline);

    // First partial refund: 1000
    escrow_client.approve_refund(&bounty_id, &1000, &depositor, &RefundMode::Partial);
    escrow_client.refund(&bounty_id);

    let info = escrow_client.get_escrow(&bounty_id);
    assert_eq!(info.status, EscrowStatus::PartiallyRefunded);
    assert_eq!(info.remaining_amount, 2000);
    assert_eq!(token_client.balance(&depositor), 1000);

    // Second partial refund: 1000
    escrow_client.approve_refund(&bounty_id, &1000, &depositor, &RefundMode::Partial);
    escrow_client.refund(&bounty_id);

    let info = escrow_client.get_escrow(&bounty_id);
    assert_eq!(info.status, EscrowStatus::PartiallyRefunded);
    assert_eq!(info.remaining_amount, 1000);
    assert_eq!(token_client.balance(&depositor), 2000);

    // Final full refund: remaining 1000
    escrow_client.approve_refund(&bounty_id, &1000, &depositor, &RefundMode::Full);
    escrow_client.refund(&bounty_id);

    let info = escrow_client.get_escrow(&bounty_id);
    assert_eq!(info.status, EscrowStatus::Refunded);
    assert_eq!(info.remaining_amount, 0);
    assert_eq!(token_client.balance(&depositor), 3000);

    // History has three entries
    let history = escrow_client.get_refund_history(&bounty_id);
    assert_eq!(history.len(), 3);
}

/// dry_run_refund returns success=true after deadline and success=false before.
#[test]
fn test_dry_run_refund_reflects_eligibility() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token_client, token_admin) = create_token_contract(&env, &admin);
    let escrow_client = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);
    token_admin.mint(&depositor, &500);

    let bounty_id = 305u64;
    let deadline = env.ledger().timestamp() + 200;
    escrow_client.lock_funds(&depositor, &bounty_id, &500, &deadline);

    // Before deadline, no approval → dry run should fail
    let result = escrow_client.dry_run_refund(&bounty_id);
    assert!(!result.success);

    // After deadline → dry run should succeed
    env.ledger().set_timestamp(deadline + 1);
    let result = escrow_client.dry_run_refund(&bounty_id);
    assert!(result.success);
    assert_eq!(result.amount, 500);
    assert_eq!(result.resulting_status, EscrowStatus::Refunded);

    // Actual state is unchanged (dry run is read-only)
    let info = escrow_client.get_escrow(&bounty_id);
    assert_eq!(info.status, EscrowStatus::Locked);
}

/// Refund before deadline without admin approval returns DeadlineNotPassed.
#[test]
fn test_refund_before_deadline_without_approval_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token_client, token_admin) = create_token_contract(&env, &admin);
    let escrow_client = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);
    token_admin.mint(&depositor, &1000);

    let bounty_id = 306u64;
    let deadline = env.ledger().timestamp() + 500;
    escrow_client.lock_funds(&depositor, &bounty_id, &1000, &deadline);

    let res = escrow_client.try_refund(&bounty_id);
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().unwrap(), Error::DeadlineNotPassed);
}
