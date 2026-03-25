extern crate std;

use soroban_sdk::{
    xdr::{FromXdr, Hash, ScAddress, ToXdr},
    Address, Env, IntoVal, String as SdkString, Symbol, TryFromVal, Val,
};

use crate::claim_period::*;
use crate::error_recovery::*;
use crate::*;

mod serialization_goldens {
    include!("serialization_goldens.rs");
}
use serialization_goldens::EXPECTED;

fn contract_address(env: &Env, tag: u8) -> Address {
    Address::try_from_val(env, &ScAddress::Contract(Hash([tag; 32]))).unwrap()
}

fn hex_encode(bytes: &[u8]) -> std::string::String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = std::string::String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

fn xdr_hex<T>(env: &Env, value: &T) -> std::string::String
where
    T: IntoVal<Env, Val> + Clone,
{
    let xdr = value.clone().to_xdr(env);
    let len = xdr.len() as usize;
    let mut buf = std::vec![0u8; len];
    xdr.copy_into_slice(&mut buf);
    hex_encode(&buf)
}

fn assert_roundtrip<T>(env: &Env, value: &T)
where
    T: IntoVal<Env, Val> + TryFromVal<Env, Val> + Clone + Eq + core::fmt::Debug,
{
    let bytes = value.clone().to_xdr(env);
    let roundtrip = T::from_xdr(env, &bytes).expect("from_xdr should succeed");
    assert_eq!(roundtrip, *value);
}

// How to update goldens:
// 1) Run:
//    `GRAINLIFY_PRINT_SERIALIZATION_GOLDENS=1 cargo test --lib serialization_compatibility_public_types_and_events -- --nocapture > /tmp/program_escrow_goldens.txt`
// 2) Regenerate `serialization_goldens.rs` from the printed EXPECTED block.
//
// Note: This test intentionally excludes internal storage keys (`DataKey`,
// `CircuitBreakerKey`) to avoid pinning internal layouts.
#[test]
fn serialization_compatibility_public_types_and_events() {
    let env = Env::default();

    let authorized = contract_address(&env, 0x01);
    let token = contract_address(&env, 0x02);
    let recipient = contract_address(&env, 0x03);
    let fee_recipient = contract_address(&env, 0x04);
    let admin = contract_address(&env, 0x05);

    let program_id = SdkString::from_str(&env, "Hackathon2026");

    let payout_record = PayoutRecord {
        recipient: recipient.clone(),
        amount: 123,
        timestamp: 10,
    };

    let payout_history = soroban_sdk::vec![&env, payout_record.clone()];

    let program_data = ProgramData {
        program_id: program_id.clone(),
        total_funds: 10_000,
        remaining_balance: 9_000,
        authorized_payout_key: authorized.clone(),
        payout_history: payout_history.clone(),
        token_address: token.clone(),
        initial_liquidity: 500,
        risk_flags: 0,
        reference_hash: None,
    };

    let program_initialized = ProgramInitializedEvent {
        version: EVENT_VERSION_V2,
        program_id: program_id.clone(),
        authorized_payout_key: authorized.clone(),
        token_address: token.clone(),
        total_funds: 10_000,
    };

    let claim_record = ClaimRecord {
        claim_id: 7,
        program_id: program_id.clone(),
        recipient: recipient.clone(),
        amount: 123,
        claim_deadline: 999,
        created_at: 111,
        status: ClaimStatus::Pending,
    };

    let error_entry = ErrorEntry {
        operation: Symbol::new(&env, "payout"),
        program_id: program_id.clone(),
        error_code: ERR_TRANSFER_FAILED,
        timestamp: 12,
        failure_count_at_time: 1,
    };

    let circuit_status = CircuitBreakerStatus {
        state: CircuitState::HalfOpen,
        failure_count: 2,
        success_count: 1,
        last_failure_timestamp: 100,
        opened_at: 200,
        failure_threshold: 3,
        success_threshold: 1,
    };

    let samples: &[(&str, Val)] = &[
        ("PayoutRecord", payout_record.clone().into_val(&env)),
        (
            "FeeConfig",
            FeeConfig {
                lock_fee_rate: 100,
                payout_fee_rate: 200,
                fee_recipient: fee_recipient.clone(),
                fee_enabled: true,
            }
            .into_val(&env),
        ),
        (
            "ProgramInitializedEvent",
            program_initialized.clone().into_val(&env),
        ),
        (
            "FundsLockedEvent",
            FundsLockedEvent {
                version: EVENT_VERSION_V2,
                program_id: program_id.clone(),
                amount: 1000,
                remaining_balance: 9000,
            }
            .into_val(&env),
        ),
        (
            "BatchPayoutEvent",
            BatchPayoutEvent {
                version: EVENT_VERSION_V2,
                program_id: program_id.clone(),
                recipient_count: 2,
                total_amount: 500,
                remaining_balance: 8500,
            }
            .into_val(&env),
        ),
        (
            "PayoutEvent",
            PayoutEvent {
                version: EVENT_VERSION_V2,
                program_id: program_id.clone(),
                recipient: recipient.clone(),
                amount: 200,
                remaining_balance: 8800,
            }
            .into_val(&env),
        ),
        ("ProgramData", program_data.clone().into_val(&env)),
        (
            "PauseFlags",
            PauseFlags {
                lock_paused: true,
                release_paused: false,
                refund_paused: true,
                pause_reason: Some(SdkString::from_str(&env, "maintenance")),
                paused_at: 1,
            }
            .into_val(&env),
        ),
        (
            "PauseStateChanged",
            PauseStateChanged {
                operation: Symbol::new(&env, "lock"),
                paused: true,
                admin: admin.clone(),
                reason: None,
                timestamp: 12345,
                receipt_id: 1,
            }
            .into_val(&env),
        ),
        (
            "RateLimitConfig",
            RateLimitConfig {
                window_size: 60,
                max_operations: 10,
                cooldown_period: 5,
            }
            .into_val(&env),
        ),
        (
            "Analytics",
            Analytics {
                total_locked: 10,
                total_released: 5,
                total_payouts: 2,
                active_programs: 1,
                operation_count: 7,
            }
            .into_val(&env),
        ),
        (
            "ProgramReleaseSchedule",
            ProgramReleaseSchedule {
                schedule_id: 1,
                recipient: recipient.clone(),
                amount: 123,
                release_timestamp: 500,
                released: false,
                released_at: None,
                released_by: None,
                approval: PayoutApprovalRecord {
                    approver: Address::generate(&env),
                    approved_at: 123,
                },
            }
            .into_val(&env),
        ),
        ("ReleaseType::Manual", ReleaseType::Manual.into_val(&env)),
        (
            "ProgramReleaseHistory",
            ProgramReleaseHistory {
                schedule_id: 1,
                recipient: recipient.clone(),
                amount: 123,
                released_at: 501,
                release_type: ReleaseType::Automatic,
            }
            .into_val(&env),
        ),
        (
            "ProgramAggregateStats",
            ProgramAggregateStats {
                total_funds: 10_000,
                remaining_balance: 9_000,
                total_paid_out: 1_000,
                authorized_payout_key: authorized.clone(),
                payout_history: payout_history.clone(),
                token_address: token.clone(),
                payout_count: 1,
                scheduled_count: 2,
                released_count: 0,
            }
            .into_val(&env),
        ),
        (
            "ProgramInitItem",
            ProgramInitItem {
                program_id: program_id.clone(),
                authorized_payout_key: authorized.clone(),
                token_address: token.clone(),
                reference_hash: None,
            }
            .into_val(&env),
        ),
        (
            "MultisigConfig",
            MultisigConfig {
                threshold_amount: 1000,
                signers: soroban_sdk::vec![&env, admin.clone(), authorized.clone()],
                required_signatures: 2,
            }
            .into_val(&env),
        ),
        (
            "PayoutApproval",
            PayoutApproval {
                program_id: program_id.clone(),
                recipient: recipient.clone(),
                amount: 123,
                approvals: soroban_sdk::vec![&env, admin.clone()],
            }
            .into_val(&env),
        ),
        ("ClaimStatus::Pending", ClaimStatus::Pending.into_val(&env)),
        ("ClaimRecord", claim_record.clone().into_val(&env)),
        (
            "CircuitState::HalfOpen",
            CircuitState::HalfOpen.into_val(&env),
        ),
        (
            "CircuitBreakerConfig",
            CircuitBreakerConfig {
                failure_threshold: 3,
                success_threshold: 1,
                max_error_log: 10,
            }
            .into_val(&env),
        ),
        ("ErrorEntry", error_entry.clone().into_val(&env)),
        (
            "CircuitBreakerStatus",
            circuit_status.clone().into_val(&env),
        ),
        (
            "RetryConfig",
            RetryConfig {
                max_attempts: 3,
                initial_backoff: 0,
                backoff_multiplier: 1,
                max_backoff: 0,
            }
            .into_val(&env),
        ),
        (
            "RetryResult",
            RetryResult {
                succeeded: false,
                attempts: 2,
                final_error: ERR_CIRCUIT_OPEN,
                total_delay: 10,
            }
            .into_val(&env),
        ),
    ];

    assert_roundtrip(&env, &ClaimStatus::Pending);
    assert_roundtrip(&env, &CircuitState::HalfOpen);

    let mut computed: std::vec::Vec<(&str, std::string::String)> = std::vec::Vec::new();
    for (name, val) in samples {
        computed.push((name, xdr_hex(&env, val)));
    }

    if std::env::var("GRAINLIFY_PRINT_SERIALIZATION_GOLDENS").is_ok() {
        std::eprintln!("const EXPECTED: &[(&str, &str)] = &[");
        for (name, hex) in &computed {
            std::eprintln!("  (\"{name}\", \"{hex}\"),");
        }
        std::eprintln!("];");
        return;
    }

    for (name, hex) in computed {
        let expected = EXPECTED
            .iter()
            .find(|(k, _)| *k == name)
            .map(|(_, v)| *v)
            .unwrap_or_else(|| panic!("Missing golden for {name}. Re-run with GRAINLIFY_PRINT_SERIALIZATION_GOLDENS=1"));
        assert_eq!(hex, expected, "XDR encoding changed for {name}");
    }
}
