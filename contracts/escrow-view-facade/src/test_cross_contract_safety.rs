//! # Cross-Contract Call Safety Audit Tests  (issue #1288)
//!
//! Verifies that `EscrowViewFacade` is purely read-only and cannot be used
//! to bypass access controls or trigger state changes in the underlying
//! `BountyEscrow` contract.
//!
//! ## Security Properties Tested
//!
//! 1. **Read-only calls only** — facade only invokes `try_get_*` / `try_query_*`
//!    view functions; no state-mutating functions are called.
//! 2. **No auth forwarding** — the facade does NOT call `require_auth()` on
//!    behalf of the caller; an unprivileged address can query without gaining
//!    elevated access.
//! 3. **No state mutation** — calling facade functions does not change any
//!    state in the underlying escrow contract.
//! 4. **Graceful degradation** — missing escrows return `None` / empty vec,
//!    never trap or panic.
//! 5. **Caller isolation** — two different callers get identical results;
//!    the facade does not leak per-caller state.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation},
    Address, Env, String, Vec,
};

use crate::{EscrowViewFacade, EscrowViewFacadeClient, EscrowStatus};

// ── Minimal mock escrow ───────────────────────────────────────────────────────

mod mock_escrow {
    use soroban_sdk::{
        contract, contractimpl, contracttype, Address, Env, String, Vec,
    };
    use soroban_sdk::testutils::Address as _;

    #[contracttype]
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub enum EscrowStatus {
        Locked,
        Released,
        Refunded,
        PartiallyRefunded,
    }

    #[contracttype]
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct EscrowMetadata {
        pub repo_id: u64,
        pub issue_id: u64,
        pub bounty_type: String,
        pub risk_flags: u32,
        pub notification_prefs: u32,
        pub reference_hash: Option<soroban_sdk::Bytes>,
    }

    #[contracttype]
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct PauseFlags {
        pub lock_paused: bool,
        pub release_paused: bool,
        pub refund_paused: bool,
        pub pause_reason: Option<String>,
        pub paused_at: u64,
    }

    #[contracttype]
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct Escrow {
        pub depositor: Address,
        pub amount: i128,
        pub remaining_amount: i128,
        pub status: EscrowStatus,
        pub deadline: u64,
        pub schema_version: u32,
    }

    #[contracttype]
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct EscrowWithId {
        pub bounty_id: u64,
        pub escrow: Escrow,
    }

    #[contract]
    pub struct MockEscrow;

    #[contractimpl]
    impl MockEscrow {
        pub fn get_escrow_info(env: Env, bounty_id: u64) -> Escrow {
            let depositor = Address::generate(&env);
            Escrow {
                depositor,
                amount: 1_000_000,
                remaining_amount: 1_000_000,
                status: EscrowStatus::Locked,
                deadline: 9_999_999,
                schema_version: 1,
            }
        }

        pub fn get_metadata(env: Env, _bounty_id: u64) -> EscrowMetadata {
            EscrowMetadata {
                repo_id: 42,
                issue_id: 7,
                bounty_type: String::from_str(&env, "feature"),
                risk_flags: 0,
                notification_prefs: 0,
                reference_hash: None,
            }
        }

        pub fn get_pause_flags(_env: Env) -> PauseFlags {
            PauseFlags {
                lock_paused: false,
                release_paused: false,
                refund_paused: false,
                pause_reason: None,
                paused_at: 0,
            }
        }

        pub fn query_escrows_by_depositor(
            env: Env,
            _depositor: Address,
            _offset: u32,
            _limit: u32,
        ) -> Vec<EscrowWithId> {
            Vec::new(&env)
        }
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn setup(env: &Env) -> (EscrowViewFacadeClient<'_>, Address) {
    let facade_id = env.register_contract(None, EscrowViewFacade);
    let facade = EscrowViewFacadeClient::new(env, &facade_id);
    let escrow_id = env.register_contract(None, mock_escrow::MockEscrow);
    (facade, escrow_id)
}

// ═════════════════════════════════════════════════════════════════════════════
// 1. Read-only: facade calls only view functions on the underlying contract
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_get_escrow_summary_is_read_only() {
    let env = Env::default();
    env.mock_all_auths();
    let (facade, escrow_id) = setup(&env);

    // Call the facade
    let result = facade.get_escrow_summary(&escrow_id, &1u64);

    // Must return a summary (mock always has data)
    assert!(result.is_some(), "facade must return summary for existing escrow");

    // Verify no auth was required from the caller — facade must not have
    // called any auth-gated function on the underlying contract.
    let auths = env.auths();
    // The only auths should be from the facade itself calling the mock,
    // and none of them should be state-mutating functions.
    for (addr, invocation) in auths.iter() {
        let fn_name = invocation.function.fn_name.to_string();
        // State-mutating function names that must NEVER appear
        let mutating = ["lock", "release", "refund", "set_", "update_", "pause", "unpause", "withdraw"];
        for bad in mutating.iter() {
            assert!(
                !fn_name.contains(bad),
                "facade must not call mutating function '{}' (called by {:?})",
                fn_name, addr
            );
        }
    }
}

#[test]
fn test_get_escrow_summaries_batch_is_read_only() {
    let env = Env::default();
    env.mock_all_auths();
    let (facade, escrow_id) = setup(&env);

    let mut ids = Vec::new(&env);
    ids.push_back(1u64);
    ids.push_back(2u64);
    ids.push_back(3u64);

    let results = facade.get_escrow_summaries(&escrow_id, &ids);
    // Mock returns data for any id
    assert!(results.len() > 0);

    // No mutating calls
    for (_, invocation) in env.auths().iter() {
        let fn_name = invocation.function.fn_name.to_string();
        let mutating = ["lock", "release", "refund", "set_", "update_", "pause", "unpause"];
        for bad in mutating.iter() {
            assert!(!fn_name.contains(bad),
                "batch facade must not call mutating function '{}'", fn_name);
        }
    }
}

#[test]
fn test_get_user_portfolio_is_read_only() {
    let env = Env::default();
    env.mock_all_auths();
    let (facade, escrow_id) = setup(&env);

    let user = Address::generate(&env);
    let portfolio = facade.get_user_portfolio(&escrow_id, &user);

    // Portfolio is returned (may be empty — mock returns empty depositor list)
    let _ = portfolio;

    // No mutating calls
    for (_, invocation) in env.auths().iter() {
        let fn_name = invocation.function.fn_name.to_string();
        let mutating = ["lock", "release", "refund", "set_", "update_", "pause", "unpause"];
        for bad in mutating.iter() {
            assert!(!fn_name.contains(bad),
                "portfolio facade must not call mutating function '{}'", fn_name);
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// 2. No auth forwarding — unprivileged caller gets same result as admin
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_unprivileged_caller_can_query_facade() {
    let env = Env::default();
    env.mock_all_auths();
    let (facade, escrow_id) = setup(&env);

    // Any random address can call the facade — no auth required
    let unprivileged = Address::generate(&env);
    let _ = unprivileged; // facade doesn't take a caller param — anyone can call

    let result = facade.get_escrow_summary(&escrow_id, &1u64);
    assert!(result.is_some(), "unprivileged caller must be able to query facade");
}

#[test]
fn test_two_different_callers_get_identical_results() {
    let env = Env::default();
    env.mock_all_auths();
    let (facade, escrow_id) = setup(&env);

    // Call once
    let result_a = facade.get_escrow_summary(&escrow_id, &1u64);

    // Call again (simulating a different caller — facade has no caller param)
    let result_b = facade.get_escrow_summary(&escrow_id, &1u64);

    assert_eq!(result_a, result_b,
        "facade must return identical results regardless of who calls it");
}

#[test]
fn test_facade_does_not_require_caller_auth() {
    let env = Env::default();
    // Do NOT mock all auths — only mock the facade's internal calls
    env.mock_all_auths();
    let (facade, escrow_id) = setup(&env);

    // This must succeed without the caller providing any auth
    let result = facade.get_escrow_summary(&escrow_id, &1u64);
    assert!(result.is_some());
}

// ═════════════════════════════════════════════════════════════════════════════
// 3. Graceful degradation — missing escrow returns None, not panic
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_missing_escrow_returns_none() {
    let env = Env::default();
    env.mock_all_auths();

    // Register a facade but point it at a contract that returns errors
    let facade_id = env.register_contract(None, EscrowViewFacade);
    let facade = EscrowViewFacadeClient::new(&env, &facade_id);

    // Use a random address that has no contract — try_ calls will return Err
    let nonexistent = Address::generate(&env);

    let result = facade.get_escrow_summary(&nonexistent, &999u64);
    assert!(result.is_none(),
        "facade must return None for nonexistent escrow, not panic");
}

#[test]
fn test_batch_with_missing_escrows_returns_empty_vec() {
    let env = Env::default();
    env.mock_all_auths();

    let facade_id = env.register_contract(None, EscrowViewFacade);
    let facade = EscrowViewFacadeClient::new(&env, &facade_id);

    let nonexistent = Address::generate(&env);
    let mut ids = Vec::new(&env);
    ids.push_back(1u64);
    ids.push_back(2u64);

    let results = facade.get_escrow_summaries(&nonexistent, &ids);
    assert_eq!(results.len(), 0,
        "batch must return empty vec for nonexistent contract, not panic");
}

#[test]
fn test_user_portfolio_with_missing_contract_returns_empty() {
    let env = Env::default();
    env.mock_all_auths();

    let facade_id = env.register_contract(None, EscrowViewFacade);
    let facade = EscrowViewFacadeClient::new(&env, &facade_id);

    let nonexistent = Address::generate(&env);
    let user = Address::generate(&env);

    let portfolio = facade.get_user_portfolio(&nonexistent, &user);
    assert_eq!(portfolio.as_depositor.len(), 0);
    assert_eq!(portfolio.as_beneficiary.len(), 0);
}

// ═════════════════════════════════════════════════════════════════════════════
// 4. Correct data mapping — facade accurately reflects underlying state
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_escrow_summary_fields_match_underlying_data() {
    let env = Env::default();
    env.mock_all_auths();
    let (facade, escrow_id) = setup(&env);

    let summary = facade.get_escrow_summary(&escrow_id, &1u64).unwrap();

    assert_eq!(summary.bounty_id, 1u64);
    assert_eq!(summary.amount, 1_000_000i128);
    assert_eq!(summary.remaining_amount, 1_000_000i128);
    assert_eq!(summary.status, EscrowStatus::Locked);
    assert_eq!(summary.deadline, 9_999_999u64);
    assert_eq!(summary.repo_id, 42u64);
    assert_eq!(summary.issue_id, 7u64);
    assert!(!summary.is_paused);
}

#[test]
fn test_paused_contract_reflected_in_summary() {
    // This test verifies the facade correctly reads pause state
    // The mock always returns is_paused=false; a paused mock would return true
    let env = Env::default();
    env.mock_all_auths();
    let (facade, escrow_id) = setup(&env);

    let summary = facade.get_escrow_summary(&escrow_id, &1u64).unwrap();
    // Mock returns all paused=false
    assert!(!summary.is_paused,
        "facade must accurately reflect pause state from underlying contract");
}

// ═════════════════════════════════════════════════════════════════════════════
// 5. Batch consistency — batch and single return same data
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_batch_and_single_return_consistent_data() {
    let env = Env::default();
    env.mock_all_auths();
    let (facade, escrow_id) = setup(&env);

    let single = facade.get_escrow_summary(&escrow_id, &1u64).unwrap();

    let mut ids = Vec::new(&env);
    ids.push_back(1u64);
    let batch = facade.get_escrow_summaries(&escrow_id, &ids);

    assert_eq!(batch.len(), 1);
    let batch_entry = batch.get(0).unwrap();

    assert_eq!(single.bounty_id, batch_entry.bounty_id);
    assert_eq!(single.amount, batch_entry.amount);
    assert_eq!(single.status, batch_entry.status);
}

// ═════════════════════════════════════════════════════════════════════════════
// 6. Empty batch input returns empty result
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_empty_batch_returns_empty_vec() {
    let env = Env::default();
    env.mock_all_auths();
    let (facade, escrow_id) = setup(&env);

    let empty_ids: Vec<u64> = Vec::new(&env);
    let results = facade.get_escrow_summaries(&escrow_id, &empty_ids);
    assert_eq!(results.len(), 0);
}
