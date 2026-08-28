#![cfg(test)]
//! # Query Adapter Regression Tests
//!
//! Regression tests for the split of the escrow-view-facade into a thin
//! entrypoint ([`crate::EscrowViewFacade`]) and the query-specific adapters in
//! [`crate::query`].
//!
//! These tests exercise the query adapters through the public entrypoint to
//! prove the refactor preserved behavior for:
//!
//! 1. **Missing records** — summary / batch / portfolio queries for bounty IDs
//!    that do not exist.
//! 2. **Pagination** — batch and portfolio queries over many bounty IDs.
//! 3. **Cross-contract error mapping** — delegate / payout queries degrade to
//!    empty results instead of trapping when the underlying query fails.

use crate::{EscrowStatus, EscrowViewFacade, EscrowViewFacadeClient};
use soroban_sdk::{
    testutils::Address as _,
    Address, Env, String, Vec,
};

// ── Mock escrow contract with configurable failure modes ──────────────────────

mod mock_escrow {
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec};

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

    #[contracttype]
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct ProgramDelegateInfo {
        pub program_id: String,
        pub delegate: Option<Address>,
        pub permissions: u32,
    }

    #[contracttype]
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct PayoutRecord {
        pub recipient: Address,
        pub amount: i128,
        pub timestamp: u64,
    }

    #[contract]
    pub struct MockEscrow;

    #[contractimpl]
    impl MockEscrow {
        /// Returns escrow info for `bounty_id % 3 != 0`; otherwise the record
        /// is missing (contract error).
        pub fn get_escrow_info(env: Env, bounty_id: u64) -> Result<Escrow, soroban_sdk::Error> {
            if bounty_id % 3 == 0 {
                return Err(soroban_sdk::Error::from_contract_error(4)); // BountyNotFound
            }
            Ok(Escrow {
                depositor: Address::generate(&env),
                amount: 1000 + bounty_id as i128,
                remaining_amount: 500 + bounty_id as i128,
                status: EscrowStatus::Locked,
                deadline: 123456789,
                schema_version: 1,
            })
        }

        pub fn get_metadata(
            env: Env,
            bounty_id: u64,
        ) -> Result<EscrowMetadata, soroban_sdk::Error> {
            Ok(EscrowMetadata {
                repo_id: 42,
                issue_id: bounty_id * 10,
                bounty_type: String::from_str(&env, "bug-bounty"),
                risk_flags: 0,
                notification_prefs: 0,
                reference_hash: None,
            })
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
            offset: u32,
            limit: u32,
        ) -> Vec<EscrowWithId> {
            let mut result = Vec::new(&env);
            for id in offset..offset + limit {
                if id % 3 == 0 {
                    continue; // missing records are skipped
                }
                result.push_back(EscrowWithId {
                    bounty_id: id,
                    escrow: Escrow {
                        depositor: _depositor.clone(),
                        amount: 1000 + id as i128,
                        remaining_amount: 500 + id as i128,
                        status: EscrowStatus::Released,
                        deadline: 987654321,
                        schema_version: 1,
                    },
                });
            }
            result
        }

        pub fn query_escrows_by_beneficiary(
            env: Env,
            beneficiary: Address,
            offset: u32,
            limit: u32,
        ) -> Vec<EscrowWithId> {
            let mut result = Vec::new(&env);
            for id in (offset + 100)..(offset + 100 + limit) {
                if id % 3 == 0 {
                    continue;
                }
                result.push_back(EscrowWithId {
                    bounty_id: id,
                    escrow: Escrow {
                        depositor: beneficiary.clone(),
                        amount: 2000 + id as i128,
                        remaining_amount: 1000 + id as i128,
                        status: EscrowStatus::PartiallyRefunded,
                        deadline: 555,
                        schema_version: 1,
                    },
                });
            }
            result
        }

        /// Panics for `program_id == "missing"` (error-mapping path, since the
        /// binding returns a plain `Vec`); otherwise a single delegate record.
        pub fn query_all_delegates(env: Env, program_id: String) -> Vec<ProgramDelegateInfo> {
            if program_id == String::from_str(&env, "missing") {
                panic!("delegate query failed");
            }
            let mut result = Vec::new(&env);
            let delegate = Address::generate(&env);
            result.push_back(ProgramDelegateInfo {
                program_id,
                delegate: Some(delegate),
                permissions: 0x3,
            });
            result
        }

        /// Panics for `program_id == "missing"` (error-mapping path, since the
        /// binding returns a plain `Vec`); otherwise a single payout record.
        pub fn query_recipient_history(
            env: Env,
            program_id: String,
            recipient: Address,
        ) -> Vec<PayoutRecord> {
            if program_id == String::from_str(&env, "missing") {
                panic!("recipient history query failed");
            }
            let mut result = Vec::new(&env);
            result.push_back(PayoutRecord {
                recipient,
                amount: 777,
                timestamp: 111,
            });
            result
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

// ── Missing records ───────────────────────────────────────────────────────────

/// A bounty ID that does not exist returns `None`, not a panic.
#[test]
fn test_missing_escrow_summary_returns_none() {
    let env = Env::default();
    env.mock_all_auths();
    let (facade, escrow_id) = setup(&env);

    let result = facade.get_escrow_summary(&escrow_id, &3u64); // % 3 == 0 → missing
    assert!(result.is_none());
}

/// Batch query skips missing records and preserves order of the survivors.
#[test]
fn test_batch_query_skips_missing_records() {
    let env = Env::default();
    env.mock_all_auths();
    let (facade, escrow_id) = setup(&env);

    let mut ids = Vec::new(&env);
    ids.push_back(1u64);
    ids.push_back(2u64);
    ids.push_back(3u64); // missing
    ids.push_back(4u64);
    ids.push_back(5u64);

    let summaries = facade.get_escrow_summaries(&escrow_id, &ids);
    assert_eq!(summaries.len(), 4);
    assert_eq!(summaries.get(0).unwrap().bounty_id, 1);
    assert_eq!(summaries.get(1).unwrap().bounty_id, 2);
    assert_eq!(summaries.get(2).unwrap().bounty_id, 4);
    assert_eq!(summaries.get(3).unwrap().bounty_id, 5);
}

// ── Pagination ────────────────────────────────────────────────────────────────

/// Portfolio pagination: the query adapters forward offset/limit to the
/// underlying contract and only map the records that exist.
#[test]
fn test_portfolio_paginated_results() {
    let env = Env::default();
    env.mock_all_auths();
    let (facade, escrow_id) = setup(&env);

    let user = Address::generate(&env);
    let portfolio = facade.get_user_portfolio(&escrow_id, &user);

    // query_escrows_by_depositor(offset=0, limit=100) returns ids 0..100,
    // skipping every 3rd (missing) → 66 records.
    assert_eq!(portfolio.as_depositor.len(), 66);
    assert_eq!(portfolio.as_depositor.get(0).unwrap().bounty_id, 1);

    // query_escrows_by_beneficiary(offset=0, limit=100) returns ids 100..200,
    // skipping every 3rd → 67 records (100 numbers, 33 skipped).
    assert_eq!(portfolio.as_beneficiary.len(), 67);
    assert_eq!(portfolio.as_beneficiary.get(0).unwrap().bounty_id, 100);
}

// ── Cross-contract error mapping ──────────────────────────────────────────────

/// `query_all_delegates` degrades to an empty vec when the underlying query
/// fails, rather than trapping.
#[test]
fn test_query_all_delegates_error_maps_to_empty() {
    let env = Env::default();
    env.mock_all_auths();
    let (facade, escrow_id) = setup(&env);

    let missing = String::from_str(&env, "missing");
    let delegates = facade.query_all_delegates(&escrow_id, &missing);
    assert_eq!(delegates.len(), 0);
}

/// `query_recipient_history` degrades to an empty vec when the underlying
/// query fails, rather than trapping.
#[test]
fn test_query_recipient_history_error_maps_to_empty() {
    let env = Env::default();
    env.mock_all_auths();
    let (facade, escrow_id) = setup(&env);

    let missing = String::from_str(&env, "missing");
    let recipient = Address::generate(&env);
    let records = facade.query_recipient_history(&escrow_id, &missing, &recipient);
    assert_eq!(records.len(), 0);
}

/// `query_all_delegates` still returns data on the happy path.
#[test]
fn test_query_all_delegates_happy_path() {
    let env = Env::default();
    env.mock_all_auths();
    let (facade, escrow_id) = setup(&env);

    let program_id = String::from_str(&env, "program-audit");
    let delegates = facade.query_all_delegates(&escrow_id, &program_id);
    assert_eq!(delegates.len(), 1);
    assert_eq!(delegates.get(0).unwrap().program_id, program_id);
}

/// `query_recipient_history` still returns data on the happy path.
#[test]
fn test_query_recipient_history_happy_path() {
    let env = Env::default();
    env.mock_all_auths();
    let (facade, escrow_id) = setup(&env);

    let program_id = String::from_str(&env, "program-payouts");
    let recipient = Address::generate(&env);
    let records = facade.query_recipient_history(&escrow_id, &program_id, &recipient);
    assert_eq!(records.len(), 1);
    assert_eq!(records.get(0).unwrap().recipient, recipient);
}

/// Summary status mapping is unchanged after the split.
#[test]
fn test_summary_status_mapping_unchanged() {
    let env = Env::default();
    env.mock_all_auths();
    let (facade, escrow_id) = setup(&env);

    let summary = facade.get_escrow_summary(&escrow_id, &1u64).unwrap();
    assert_eq!(summary.status, EscrowStatus::Locked);
    assert_eq!(summary.bounty_type, String::from_str(&env, "bug-bounty"));
    assert!(!summary.is_paused);
}
