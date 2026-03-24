extern crate std;

use soroban_sdk::{
    xdr::{FromXdr, Hash, ScAddress, ToXdr},
    Address, Env, IntoVal, String as SdkString, Symbol, TryFromVal, Val,
};

use crate::events::*;
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
// 1) Run: `GRAINLIFY_PRINT_SERIALIZATION_GOLDENS=1 cargo test -p bounty-escrow --lib serialization_compatibility_public_types_and_events -- --nocapture > /tmp/bounty_goldens.txt`
// 2) Regenerate `serialization_goldens.rs` from the printed EXPECTED block.

#[test]
fn serialization_compatibility_public_types_and_events() {
    let env = Env::default();

    // Deterministic addresses (avoid randomness in goldens).
    let admin = contract_address(&env, 0x01);
    let token = contract_address(&env, 0x02);
    let depositor = contract_address(&env, 0x03);
    let contributor = contract_address(&env, 0x04);
    let fee_recipient = contract_address(&env, 0x05);
    let recipient = contract_address(&env, 0x06);
    let holder = contract_address(&env, 0x07);

    let bounty_id = 42u64;
    let repo_id = 1001u64;
    let issue_id = 561u64;
    let amount = 1_234_567_i128;
    let deadline = 1_700_000_000u64;

    let bounty_type = SdkString::from_str(&env, "bugfix");
    let pause_reason = Some(SdkString::from_str(&env, "maintenance"));

    let refund_record_full = RefundRecord {
        amount: 11,
        recipient: depositor.clone(),
        timestamp: 111,
        mode: RefundMode::Full,
    };
    let escrow = Escrow {
        depositor: depositor.clone(),
        amount,
        remaining_amount: amount - 33,
        status: EscrowStatus::Locked,
        deadline,
        // Keep nested vectors minimal in goldens to avoid huge outputs.
        refund_history: soroban_sdk::vec![&env],
    };

    let samples: &[(&str, Val)] = &[
        (
            "EscrowMetadata",
            EscrowMetadata {
                repo_id,
                issue_id,
                bounty_type: bounty_type.clone(),
                risk_flags: 0,
                reference_hash: None,
            }
            .into_val(&env),
        ),
        ("EscrowStatus::Locked", EscrowStatus::Locked.into_val(&env)),
        ("Escrow", escrow.clone().into_val(&env)),
        (
            "EscrowWithId",
            EscrowWithId {
                bounty_id,
                escrow: escrow.clone(),
            }
            .into_val(&env),
        ),
        (
            "PauseFlags",
            PauseFlags {
                lock_paused: true,
                release_paused: false,
                refund_paused: true,
                pause_reason: pause_reason.clone(),
                paused_at: 999,
            }
            .into_val(&env),
        ),
        (
            "AggregateStats",
            AggregateStats {
                total_locked: 10,
                total_released: 20,
                total_refunded: 30,
                count_locked: 1,
                count_released: 2,
                count_refunded: 3,
            }
            .into_val(&env),
        ),
        (
            "PauseStateChanged",
            PauseStateChanged {
                operation: Symbol::new(&env, "lock"),
                paused: true,
                admin: admin.clone(),
                reason: pause_reason.clone(),
                timestamp: 123,
            }
            .into_val(&env),
        ),
        (
            "AntiAbuseConfigView",
            AntiAbuseConfigView {
                window_size: 60,
                max_operations: 10,
                cooldown_period: 5,
            }
            .into_val(&env),
        ),
        (
            "FeeConfig",
            FeeConfig {
                lock_fee_rate: 100,
                release_fee_rate: 200,
                fee_recipient: fee_recipient.clone(),
                fee_enabled: true,
            }
            .into_val(&env),
        ),
        (
            "MultisigConfig",
            MultisigConfig {
                threshold_amount: 500,
                signers: soroban_sdk::vec![&env, admin.clone(), depositor.clone()],
                required_signatures: 2,
            }
            .into_val(&env),
        ),
        (
            "ReleaseApproval",
            ReleaseApproval {
                bounty_id,
                contributor: contributor.clone(),
                approvals: soroban_sdk::vec![&env, admin.clone()],
            }
            .into_val(&env),
        ),
        (
            "ClaimRecord",
            ClaimRecord {
                bounty_id,
                recipient: recipient.clone(),
                amount: 1234,
                expires_at: 555,
                claimed: false,
                reason: DisputeReason::Other,
            }
            .into_val(&env),
        ),
        (
            "CapabilityAction::Claim",
            CapabilityAction::Claim.into_val(&env),
        ),
        (
            "Capability",
            Capability {
                owner: admin.clone(),
                holder: holder.clone(),
                action: CapabilityAction::Release,
                bounty_id,
                amount_limit: 999,
                remaining_amount: 888,
                expiry: 777,
                remaining_uses: 3,
                revoked: false,
            }
            .into_val(&env),
        ),
        ("RefundMode::Full", RefundMode::Full.into_val(&env)),
        (
            "RefundApproval",
            RefundApproval {
                bounty_id,
                amount: 444,
                recipient: depositor.clone(),
                mode: RefundMode::Partial,
                approved_by: admin.clone(),
                approved_at: 9999,
            }
            .into_val(&env),
        ),
        ("RefundRecord", refund_record_full.into_val(&env)),
        (
            "LockFundsItem",
            LockFundsItem {
                bounty_id,
                depositor: depositor.clone(),
                amount: 123,
                deadline: 456,
            }
            .into_val(&env),
        ),
        (
            "ReleaseFundsItem",
            ReleaseFundsItem {
                bounty_id,
                contributor: contributor.clone(),
            }
            .into_val(&env),
        ),
        // Event payloads
        (
            "BountyEscrowInitialized",
            BountyEscrowInitialized {
                version: EVENT_VERSION_V2,
                admin: admin.clone(),
                token: token.clone(),
                timestamp: 1,
            }
            .into_val(&env),
        ),
        (
            "FundsLocked",
            FundsLocked {
                version: EVENT_VERSION_V2,
                bounty_id,
                amount,
                depositor: depositor.clone(),
                deadline,
            }
            .into_val(&env),
        ),
        (
            "FundsReleased",
            FundsReleased {
                version: EVENT_VERSION_V2,
                bounty_id,
                amount: 123,
                recipient: contributor.clone(),
                timestamp: 456,
            }
            .into_val(&env),
        ),
        (
            "FundsRefunded",
            FundsRefunded {
                version: EVENT_VERSION_V2,
                bounty_id,
                amount: 100,
                refund_to: depositor.clone(),
                timestamp: 200,
            }
            .into_val(&env),
        ),
        (
            "FeeOperationType::Lock",
            FeeOperationType::Lock.into_val(&env),
        ),
        (
            "FeeCollected",
            FeeCollected {
                operation_type: FeeOperationType::Release,
                amount: 456,
                fee_rate: 123,
                recipient: fee_recipient.clone(),
                timestamp: 999,
            }
            .into_val(&env),
        ),
        (
            "BatchFundsLocked",
            BatchFundsLocked {
                count: 2,
                total_amount: 999,
                timestamp: 1,
            }
            .into_val(&env),
        ),
        (
            "FeeConfigUpdated",
            FeeConfigUpdated {
                lock_fee_rate: 10,
                release_fee_rate: 20,
                fee_recipient: fee_recipient.clone(),
                fee_enabled: true,
                timestamp: 2,
            }
            .into_val(&env),
        ),
        (
            "BatchFundsReleased",
            BatchFundsReleased {
                count: 1,
                total_amount: 333,
                timestamp: 3,
            }
            .into_val(&env),
        ),
        (
            "ApprovalAdded",
            ApprovalAdded {
                bounty_id,
                contributor: contributor.clone(),
                approver: admin.clone(),
                timestamp: 4,
            }
            .into_val(&env),
        ),
        (
            "ClaimCreated",
            ClaimCreated {
                bounty_id,
                recipient: recipient.clone(),
                amount: 100,
                expires_at: 200,
            }
            .into_val(&env),
        ),
        (
            "ClaimExecuted",
            ClaimExecuted {
                bounty_id,
                recipient: recipient.clone(),
                amount: 100,
                claimed_at: 300,
            }
            .into_val(&env),
        ),
        (
            "ClaimCancelled",
            ClaimCancelled {
                bounty_id,
                recipient: recipient.clone(),
                amount: 100,
                cancelled_at: 400,
                cancelled_by: admin.clone(),
            }
            .into_val(&env),
        ),
        (
            "EmergencyWithdrawEvent",
            EmergencyWithdrawEvent {
                admin: admin.clone(),
                recipient: depositor.clone(),
                amount: 1000,
                timestamp: 500,
            }
            .into_val(&env),
        ),
        (
            "CapabilityIssued",
            CapabilityIssued {
                capability_id: 7,
                owner: admin.clone(),
                holder: holder.clone(),
                action: CapabilityAction::Refund,
                bounty_id,
                amount_limit: 123,
                expires_at: 456,
                max_uses: 2,
                timestamp: 789,
            }
            .into_val(&env),
        ),
        (
            "CapabilityUsed",
            CapabilityUsed {
                capability_id: 7,
                holder: holder.clone(),
                action: CapabilityAction::Refund,
                bounty_id,
                amount_used: 11,
                remaining_amount: 22,
                remaining_uses: 1,
                used_at: 999,
            }
            .into_val(&env),
        ),
        (
            "CapabilityRevoked",
            CapabilityRevoked {
                capability_id: 7,
                owner: admin.clone(),
                revoked_at: 111,
            }
            .into_val(&env),
        ),
    ];

    // Validate round-trips for a representative subset (full list is validated by golden checks).
    assert_roundtrip(&env, &escrow);
    assert_roundtrip(&env, &refund_record_full);
    assert_roundtrip(&env, &RefundMode::Partial);

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
