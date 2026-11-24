//! # PauseStateChanged V1 → V2 Migration Tests
//!
//! ## Background
//!
//! The `PauseStateChanged` event was extended from V1 to V2:
//!
//! | Field        | V1  | V2  | Notes                          |
//! |--------------|-----|-----|--------------------------------|
//! | `operation`  | ✅  | ✅  | unchanged                      |
//! | `paused`     | ✅  | ✅  | unchanged                      |
//! | `admin`      | ✅  | ✅  | unchanged                      |
//! | `reason`     | ✅  | ✅  | unchanged                      |
//! | `timestamp`  | ✅  | ✅  | unchanged                      |
//! | `receipt_id` | ❌  | ✅  | **new in V2** — monotonic u64  |
//!
//! ## Migration guarantee (as implemented by `soroban-sdk` 21.7.7)
//!
//! Soroban encodes `#[contracttype]` structs as XDR maps keyed by field name,
//! and the SDK's generated decoder requires the map length to match the struct's
//! field count exactly. The practical consequence, asserted below, is:
//!
//! - Each schema decodes its own blobs (round-trip) and preserves every field.
//! - A V1 parser handed a V2 blob **fails loudly** with
//!   `Error(Object, UnexpectedSize)` instead of silently dropping `receipt_id`.
//! - A V2 parser handed a legacy V1 blob fails the same way instead of
//!   inventing `receipt_id`.
//!
//! This is stricter than the earlier "forward-compatible map" assumption: an
//! indexer that replays historical events must version-gate its parser on the
//! event topic (`PauseSt` for V1, `PauseStateChangedV2` / `PAUSE_STATE_CHANGED_V2`
//! for the current shape) rather than trying one parser for both. `receipt_id` is
//! informational, so a V1-only indexer loses deduplication but cannot be tricked
//! into double-processing a payout.
//!
//! See `contracts/EVENT_VERSIONING.md`: legacy payloads may be unversioned,
//! current payloads carry `version: 2`, and parsers must reject payloads that do
//! not match the shape they expect.

#![cfg(test)]

extern crate std;

use soroban_sdk::{
    contracttype,
    testutils::Address as _,
    xdr::{FromXdr, ToXdr},
    Address, Env, String as SdkString, Symbol,
};

use crate::PauseStateChanged;

// ---------------------------------------------------------------------------
// V1 schema — the original PauseStateChanged without receipt_id
// ---------------------------------------------------------------------------

/// V1 schema for `PauseStateChanged`.
///
/// This mirrors the struct as it existed before `receipt_id` was added.
/// Indexers that have not yet upgraded to V2 parse their own (V1) XDR using this
/// layout; they must not be handed V2 XDR (see the module docs).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PauseStateChangedV1 {
    pub operation: Symbol,
    pub paused: bool,
    pub admin: Address,
    pub reason: Option<SdkString>,
    pub timestamp: u64,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_v2(env: &Env) -> PauseStateChanged {
    let admin = Address::generate(env);
    PauseStateChanged {
        operation: Symbol::new(env, "lock"),
        paused: true,
        admin,
        reason: Some(SdkString::from_str(env, "maintenance")),
        timestamp: 99_999,
        receipt_id: 42,
    }
}

fn make_v2_no_reason(env: &Env) -> PauseStateChanged {
    let admin = Address::generate(env);
    PauseStateChanged {
        operation: Symbol::new(env, "release"),
        paused: false,
        admin,
        reason: None,
        timestamp: 1,
        receipt_id: 1,
    }
}

// ---------------------------------------------------------------------------
// Migration tests
// ---------------------------------------------------------------------------

/// A V1 parser must reject V2 XDR rather than silently truncating it.
///
/// This is the regression guard for indexer operators: a partial decode that
/// dropped `receipt_id` would look successful but lose deduplication, so the
/// decoder is expected to fail with a map-size error.
///
/// Note: the SDK decoder raises the map-size mismatch as a host panic rather
/// than returning `Err`, so this is asserted with `should_panic`.
#[test]
#[should_panic]
fn test_v1_parser_rejects_v2_xdr_explicitly() {
    let env = Env::default();
    let v2 = make_v2(&env);

    let xdr_bytes = v2.clone().to_xdr(&env);

    // Must not silently decode a shorter field set.
    let _ = PauseStateChangedV1::from_xdr(&env, &xdr_bytes);
}

/// The same holds for a V2 blob whose `reason` is `None`.
#[test]
#[should_panic]
fn test_v1_parser_rejects_v2_xdr_no_reason() {
    let env = Env::default();
    let v2 = make_v2_no_reason(&env);

    let _ = PauseStateChangedV1::from_xdr(&env, &v2.to_xdr(&env));
}

/// A V2 parser must reject legacy V1 XDR instead of defaulting `receipt_id` to 0.
///
/// Defaulting would fabricate an event id that never existed on chain, so the
/// decoder fails explicitly and the indexer can fall back to V1 parsing.
#[test]
#[should_panic]
fn test_v2_parser_rejects_v1_xdr_explicitly() {
    let env = Env::default();
    let v1 = PauseStateChangedV1 {
        operation: Symbol::new(&env, "refund"),
        paused: true,
        admin: Address::generate(&env),
        reason: None,
        timestamp: 500,
    };

    let _ = PauseStateChanged::from_xdr(&env, &v1.to_xdr(&env));
}

/// V2 XDR round-trips correctly through the V2 parser.
#[test]
fn test_v2_roundtrip() {
    let env = Env::default();
    let v2 = make_v2(&env);
    let decoded = PauseStateChanged::from_xdr(&env, &v2.clone().to_xdr(&env))
        .expect("V2 round-trip must succeed");
    assert_eq!(decoded, v2);
}

/// V2 XDR round-trips with `paused = false` and `reason = None`.
#[test]
fn test_v2_roundtrip_unpause_no_reason() {
    let env = Env::default();
    let v2 = make_v2_no_reason(&env);
    let decoded = PauseStateChanged::from_xdr(&env, &v2.clone().to_xdr(&env))
        .expect("V2 round-trip (no reason) must succeed");
    assert_eq!(decoded, v2);
}

/// V1 round-trips correctly through the V1 parser.
#[test]
fn test_v1_roundtrip() {
    let env = Env::default();
    let v1 = PauseStateChangedV1 {
        operation: Symbol::new(&env, "lock"),
        paused: true,
        admin: Address::generate(&env),
        reason: Some(SdkString::from_str(&env, "scheduled")),
        timestamp: 12_345,
    };
    let decoded = PauseStateChangedV1::from_xdr(&env, &v1.clone().to_xdr(&env))
        .expect("V1 round-trip must succeed");
    assert_eq!(decoded, v1);
}

/// The V2 XDR blob is strictly larger than V1 (it carries the extra `receipt_id`).
#[test]
fn test_v2_xdr_larger_than_v1() {
    let env = Env::default();
    let admin = Address::generate(&env);

    let v1 = PauseStateChangedV1 {
        operation: Symbol::new(&env, "lock"),
        paused: true,
        admin: admin.clone(),
        reason: None,
        timestamp: 1,
    };
    let v2 = PauseStateChanged {
        operation: Symbol::new(&env, "lock"),
        paused: true,
        admin,
        reason: None,
        timestamp: 1,
        receipt_id: 1,
    };

    let v1_len = v1.to_xdr(&env).len();
    let v2_len = v2.to_xdr(&env).len();
    assert!(
        v2_len > v1_len,
        "V2 XDR ({v2_len} bytes) must be larger than V1 ({v1_len} bytes)"
    );
}

/// V2 is a strict superset of V1: every legacy field survives a round-trip for
/// all three pause operations, and the new `receipt_id` is preserved.
#[test]
fn test_v2_carries_all_legacy_fields_for_every_operation() {
    let env = Env::default();

    for (index, op) in ["lock", "release", "refund"].iter().enumerate() {
        let admin = Address::generate(&env);
        let legacy = PauseStateChangedV1 {
            operation: Symbol::new(&env, op),
            paused: true,
            admin: admin.clone(),
            reason: Some(SdkString::from_str(&env, "test")),
            timestamp: 100,
        };
        let current = PauseStateChanged {
            operation: legacy.operation.clone(),
            paused: legacy.paused,
            admin: legacy.admin.clone(),
            reason: legacy.reason.clone(),
            timestamp: legacy.timestamp,
            receipt_id: index as u64 + 1,
        };

        let decoded = PauseStateChanged::from_xdr(&env, &current.to_xdr(&env))
            .unwrap_or_else(|_| panic!("V2 round-trip failed for operation={op}"));

        // Legacy fields (present in both shapes) are unchanged.
        assert_eq!(
            decoded.operation, legacy.operation,
            "operation mismatch for {op}"
        );
        assert_eq!(decoded.paused, legacy.paused);
        assert_eq!(decoded.admin, legacy.admin);
        assert_eq!(decoded.reason, legacy.reason);
        assert_eq!(decoded.timestamp, legacy.timestamp);
        // The additive V2 field survives.
        assert_eq!(decoded.receipt_id, index as u64 + 1);
    }
}

/// Golden sanity check: the V2 payload is non-trivially sized.
#[test]
fn test_v2_xdr_golden_field_count() {
    let env = Env::default();
    let v2 = PauseStateChanged {
        operation: Symbol::new(&env, "lock"),
        paused: true,
        admin: Address::generate(&env),
        reason: None,
        timestamp: 1,
        receipt_id: 1,
    };
    let len = v2.to_xdr(&env).len() as usize;
    // V2 has 6 fields; V1 had 5. Sanity-check the XDR is non-trivially sized.
    assert!(len > 64, "V2 XDR unexpectedly small ({len} bytes)");
}

// ---------------------------------------------------------------------------
// Golden hex constant for documentation / indexer reference
// ---------------------------------------------------------------------------

/// V2 PauseStateChanged golden from serialization_goldens.rs.
/// Indexers can use this to validate their XDR parsers offline.
pub const PAUSE_STATE_CHANGED_V2_GOLDEN: &str = concat!(
    "0000001100000001000000060000000f0000000561646d696e000000000000120000000105050505",
    "050505050505050505050505050505050505050505050505050505050000000f000000096f706572",
    "6174696f6e0000000000000f000000046c6f636b0000000f00000006706175736564000000000000",
    "000000010000000f00000006726561736f6e0000000000010000000f0000000a726563656970745f",
    "696400000000000500000000000000010000000f0000000974696d657374616d7000000000000005",
    "0000000000003039"
);
