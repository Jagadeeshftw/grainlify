//! # View Facade ⇄ Program Escrow conformance tests (issue #1870)
//!
//! The view-facade is a read-only aggregation layer whose query adapters mirror
//! data types from the canonical `program-escrow` contract. `program-escrow` is
//! depended on with `default-features = false` so its force-exported server
//! entrypoints do not collide with the facade ABI at link time — which also
//! means the mirrored types are *copies* that nothing else forces to stay in
//! step with the contract.
//!
//! This target is the missing guardrail. It is an integration test so it
//! compiles the facade as a library (the crate's `#[cfg(test)]` unit modules
//! are unrelated to conformance and are not built here):
//!
//! 1. Every mirrored type is built with an **exhaustive struct literal** here.
//!    Adding, removing, renaming, or retyping a field in `program-escrow` makes
//!    this file fail to compile, so the conformance gate fails until the mirror
//!    (and this expectation) are updated in the same pull request.
//! 2. The canonical value is converted through the same `Val` representation
//!    the facade uses for cross-contract returns, and must decode back into the
//!    facade's mirror with identical field values.
//! 3. A reference `program-escrow` implementation is registered in the test
//!    `Env`, so the facade's real cross-contract decode path runs against
//!    canonical state and its output is compared field-by-field.

use soroban_sdk::{
    testutils::Address as _,
    Address, Env, IntoVal, String as SdkString, TryFromVal, Val, Vec,
};
use view_facade::query::PayoutRecord as FacadePayoutRecord;
use view_facade::{ViewFacade, ViewFacadeClient};

/// A minimal `program-escrow` implementation whose return types are the
/// **canonical** `program_escrow` types (not the facade's mirrors), so the
/// facade has to decode them exactly as it would in production.
mod reference_program_escrow {
    use super::*;
    use soroban_sdk::{contract, contractimpl};

    #[contract]
    pub struct ReferenceProgramEscrow;

    #[contractimpl]
    impl ReferenceProgramEscrow {
        pub fn query_recipient_history(
            env: Env,
            _program_id: SdkString,
            recipient: Address,
        ) -> Vec<program_escrow::PayoutRecord> {
            let mut out = Vec::new(&env);
            out.push_back(program_escrow::PayoutRecord {
                recipient,
                amount: 777,
                timestamp: 111,
            });
            out
        }

        pub fn get_fee_config(env: Env) -> program_escrow::FeeConfig {
            program_escrow::FeeConfig {
                lock_fee_rate: 10,
                payout_fee_rate: 20,
                lock_fixed_fee: 100,
                payout_fixed_fee: 200,
                // The facade must see exactly the address the underlying
                // contract holds — the reference contract's own address.
                fee_recipient: env.current_contract_address(),
                fee_enabled: true,
                fee_waivers: 0b11,
                insurance_reserve_bps: 250,
            }
        }
    }
}

use reference_program_escrow::ReferenceProgramEscrow;

fn setup(env: &Env) -> (ViewFacadeClient<'_>, Address) {
    env.mock_all_auths();
    let facade_id = env.register_contract(None, ViewFacade);
    (ViewFacadeClient::new(env, &facade_id), facade_id)
}

// ── 1. Mirrored type shape ────────────────────────────────────────────────────

/// The facade's local `PayoutRecord` must decode a canonical `PayoutRecord`
/// `Val` into identical field values.
///
/// The canonical literal is exhaustive: a new field on
/// `program_escrow::PayoutRecord` breaks this file at compile time.
#[test]
fn payout_record_mirror_decodes_canonical_program_escrow_value() {
    let env = Env::default();
    let recipient = Address::generate(&env);

    let canonical = program_escrow::PayoutRecord {
        recipient: recipient.clone(),
        amount: 1_234_i128,
        timestamp: 99_u64,
    };

    let canonical_val: Val = canonical.into_val(&env);
    let decoded = FacadePayoutRecord::try_from_val(&env, &canonical_val).expect(
        "the facade's PayoutRecord mirror must decode a canonical program-escrow PayoutRecord",
    );

    assert_eq!(decoded.recipient, recipient);
    assert_eq!(decoded.amount, 1_234_i128);
    assert_eq!(decoded.timestamp, 99_u64);
}

// ── 2. Facade output vs. underlying contract state ────────────────────────────

/// `query_recipient_history` must return exactly what the underlying
/// `program-escrow` contract returned for the same state.
#[test]
fn query_recipient_history_output_matches_underlying_program_escrow() {
    let env = Env::default();
    let (facade, _) = setup(&env);
    let escrow_id = env.register_contract(None, ReferenceProgramEscrow);

    let program_id = SdkString::from_str(&env, "program-conformance");
    let recipient = Address::generate(&env);

    let records = facade.query_recipient_history(&escrow_id, &program_id, &recipient);

    assert_eq!(records.len(), 1, "facade must surface the one payout record");
    let record = records.get(0).unwrap();
    assert_eq!(record.recipient, recipient);
    assert_eq!(record.amount, 777_i128);
    assert_eq!(record.timestamp, 111_u64);
}

/// `query_fee_config_cached` must return exactly the `FeeConfig` the underlying
/// `program-escrow` contract holds, including the `fee_recipient` address.
#[test]
fn query_fee_config_output_matches_underlying_program_escrow() {
    let env = Env::default();
    let (facade, _) = setup(&env);
    let escrow_id = env.register_contract(None, ReferenceProgramEscrow);

    let config = facade.query_fee_config_cached(&escrow_id);

    let expected = program_escrow::FeeConfig {
        lock_fee_rate: 10_i128,
        payout_fee_rate: 20_i128,
        lock_fixed_fee: 100_i128,
        payout_fixed_fee: 200_i128,
        fee_recipient: escrow_id.clone(),
        fee_enabled: true,
        fee_waivers: 0b11_u32,
        insurance_reserve_bps: 250_u32,
    };

    assert_eq!(config, expected, "facade FeeConfig must match the contract's");
}
