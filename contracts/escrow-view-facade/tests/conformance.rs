//! # Escrow View Facade ⇄ Program Escrow conformance tests (issue #1870)
//!
//! The escrow-view-facade proxies program-escrow queries for its
//! `query_all_delegates` / `query_recipient_history` entrypoints. `program-escrow`
//! is depended on with `default-features = false` so its force-exported server
//! entrypoints do not collide with the facade ABI at link time — which also
//! means nothing else forces the facade's view of the contract to stay in step.
//!
//! This target is the missing guardrail. It is an integration test so it builds
//! the facade as a library, independent of the crate's `#[cfg(test)]` unit
//! modules:
//!
//! 1. Every mirrored type is built with an **exhaustive struct literal** here.
//!    Adding, removing, renaming, or retyping a field in `program-escrow` makes
//!    this file fail to compile, so the conformance gate fails until the facade
//!    and this expectation are updated in the same pull request.
//! 2. A reference `program-escrow` implementation is registered in the test
//!    `Env`, so the facade's real cross-contract decode path runs against
//!    canonical state and its output is compared field-by-field.

use escrow_view_facade::{EscrowViewFacade, EscrowViewFacadeClient};
use soroban_sdk::{testutils::Address as _, Address, Env, String as SdkString, Vec};

/// A minimal `program-escrow` implementation whose return types are the
/// **canonical** `program_escrow` types, so the facade has to decode them
/// exactly as it would in production.
mod reference_program_escrow {
    use super::*;
    use soroban_sdk::{contract, contractimpl};

    #[contract]
    pub struct ReferenceProgramEscrow;

    #[contractimpl]
    impl ReferenceProgramEscrow {
        pub fn query_all_delegates(
            env: Env,
            program_id: SdkString,
        ) -> Vec<program_escrow::ProgramDelegateInfo> {
            let mut out = Vec::new(&env);
            out.push_back(program_escrow::ProgramDelegateInfo {
                program_id,
                // The facade must see exactly the delegate the underlying
                // contract stored — the reference contract's own address.
                delegate: Some(env.current_contract_address()),
                permissions: 0b011,
            });
            out
        }

        pub fn query_recipient_history(
            env: Env,
            _program_id: SdkString,
            recipient: Address,
        ) -> Vec<program_escrow::PayoutRecord> {
            let mut out = Vec::new(&env);
            out.push_back(program_escrow::PayoutRecord {
                recipient,
                amount: 4_242,
                timestamp: 7,
            });
            out
        }
    }
}

use reference_program_escrow::ReferenceProgramEscrow;

fn setup(env: &Env) -> (EscrowViewFacadeClient<'_>, Address) {
    env.mock_all_auths();
    let facade_id = env.register_contract(None, EscrowViewFacade);
    (EscrowViewFacadeClient::new(env, &facade_id), facade_id)
}

// ── 1. Mirrored type shapes ───────────────────────────────────────────────────

/// The facade must surface the canonical `ProgramDelegateInfo`, decoded from the
/// underlying program-escrow contract's return value.
///
/// The canonical literal is exhaustive: a new field on
/// `program_escrow::ProgramDelegateInfo` breaks this file at compile time.
#[test]
fn query_all_delegates_output_matches_underlying_program_escrow() {
    let env = Env::default();
    let (facade, _) = setup(&env);
    let program_id = SdkString::from_str(&env, "program-conformance");

    let program_contract = env.register_contract(None, ReferenceProgramEscrow);
    let delegates = facade.query_all_delegates(&program_contract, &program_id);

    assert_eq!(delegates.len(), 1, "facade must surface the one delegate");
    let delegate = delegates.get(0).unwrap();
    assert_eq!(delegate.program_id, program_id);
    assert_eq!(delegate.delegate, Some(program_contract.clone()));
    assert_eq!(delegate.permissions, 0b011_u32);
}

/// The facade must surface the canonical `PayoutRecord`, decoded from the
/// underlying program-escrow contract's return value.
#[test]
fn query_recipient_history_output_matches_underlying_program_escrow() {
    let env = Env::default();
    let (facade, _) = setup(&env);
    let program_id = SdkString::from_str(&env, "program-conformance");
    let recipient = Address::generate(&env);

    let program_contract = env.register_contract(None, ReferenceProgramEscrow);
    let records = facade.query_recipient_history(&program_contract, &program_id, &recipient);

    assert_eq!(records.len(), 1, "facade must surface the one payout record");
    let record = records.get(0).unwrap();
    assert_eq!(record.recipient, recipient);
    assert_eq!(record.amount, 4_242_i128);
    assert_eq!(record.timestamp, 7_u64);
}
