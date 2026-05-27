#![cfg(test)]

extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Events},
    vec, Address, BytesN, Env, Symbol, TryFromVal,
};

use crate::{
    BuildInfoEvent, GovernanceConfig, GrainlifyContract, GrainlifyContractClient,
    MigrationCommittedEvent, MigrationEvent, ReadOnlyModeEvent, UpgradeEvent,
    VotingScheme, EVENT_SCHEMA_VERSION,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn default_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

fn register(env: &Env) -> GrainlifyContractClient {
    let id = env.register_contract(None, GrainlifyContract);
    GrainlifyContractClient::new(env, &id)
}

fn governance_config(env: &Env) -> GovernanceConfig {
    GovernanceConfig {
        voting_period: 86_400,
        execution_delay: 3_600,
        quorum_percentage: 5_000,
        approval_threshold: 6_000,
        min_proposal_stake: 1,
        voting_scheme: VotingScheme::OnePersonOneVote,
        governance_token: Address::generate(env),
    }
}

fn find_events_by_topic(env: &Env, t0: &str, t1: &str) -> std::vec::Vec<soroban_sdk::Val> {
    env.events()
        .all()
        .iter()
        .filter(|e| {
            if e.1.len() < 2 {
                return false;
            }
            let a = Symbol::try_from_val(env, &e.1.get(0).unwrap()).ok();
            let b = Symbol::try_from_val(env, &e.1.get(1).unwrap()).ok();
            a == Some(Symbol::new(env, t0)) && b == Some(Symbol::new(env, t1))
        })
        .map(|e| e.2)
        .collect()
}

// ---------------------------------------------------------------------------
// EVENT_SCHEMA_VERSION constant
// ---------------------------------------------------------------------------

#[test]
fn event_schema_version_is_one() {
    assert_eq!(EVENT_SCHEMA_VERSION, 1);
}

// ---------------------------------------------------------------------------
// is_compatible_event_version
// ---------------------------------------------------------------------------

#[test]
fn compatible_version_returns_true_for_current() {
    assert!(crate::is_compatible_event_version(EVENT_SCHEMA_VERSION));
}

#[test]
fn compatible_version_returns_false_for_zero() {
    assert!(!crate::is_compatible_event_version(0));
}

#[test]
fn compatible_version_returns_false_for_future() {
    assert!(!crate::is_compatible_event_version(EVENT_SCHEMA_VERSION + 1));
}

#[test]
fn compatible_version_returns_false_for_large_value() {
    assert!(!crate::is_compatible_event_version(u32::MAX));
}

// ---------------------------------------------------------------------------
// BuildInfoEvent — event_version field
// ---------------------------------------------------------------------------

#[test]
fn build_info_event_has_event_version_field() {
    let env = default_env();
    let client = register(&env);
    let admin = Address::generate(&env);

    client.init_admin(&admin);

    let payloads = find_events_by_topic(&env, "init", "build");
    assert!(!payloads.is_empty(), "Expected at least one init/build event");

    let ev = BuildInfoEvent::try_from_val(&env, &payloads[0])
        .expect("Failed to deserialize BuildInfoEvent");
    assert_eq!(ev.event_version, EVENT_SCHEMA_VERSION);
}

#[test]
fn build_info_event_version_via_multisig_init() {
    let env = default_env();
    let client = register(&env);
    let signers = vec![&env, Address::generate(&env), Address::generate(&env)];

    client.init(&signers, &2);

    let payloads = find_events_by_topic(&env, "init", "build");
    assert!(!payloads.is_empty());

    let ev = BuildInfoEvent::try_from_val(&env, &payloads[0])
        .expect("Failed to deserialize BuildInfoEvent");
    assert_eq!(ev.event_version, EVENT_SCHEMA_VERSION);
}

#[test]
fn build_info_event_version_via_network_init() {
    let env = default_env();
    let client = register(&env);
    let admin = Address::generate(&env);

    client.init_with_network(
        &admin,
        &soroban_sdk::String::from_str(&env, "stellar-main"),
        &soroban_sdk::String::from_str(&env, "mainnet"),
    );

    let payloads = find_events_by_topic(&env, "init", "build");
    assert!(!payloads.is_empty());

    let ev = BuildInfoEvent::try_from_val(&env, &payloads[0])
        .expect("Failed to deserialize BuildInfoEvent");
    assert_eq!(ev.event_version, EVENT_SCHEMA_VERSION);
}

#[test]
fn build_info_event_version_via_governance_init() {
    let env = default_env();
    let client = register(&env);
    let admin = Address::generate(&env);

    client.init_governance(&admin, &governance_config(&env));

    let payloads = find_events_by_topic(&env, "init", "build");
    assert!(!payloads.is_empty());

    let ev = BuildInfoEvent::try_from_val(&env, &payloads[0])
        .expect("Failed to deserialize BuildInfoEvent");
    assert_eq!(ev.event_version, EVENT_SCHEMA_VERSION);
}

// ---------------------------------------------------------------------------
// ReadOnlyModeEvent — event_version field
// ---------------------------------------------------------------------------

#[test]
fn read_only_mode_event_has_event_version_field() {
    let env = default_env();
    let client = register(&env);
    let admin = Address::generate(&env);
    client.init_admin(&admin);

    client.set_read_only_mode(&true);

    let payloads: std::vec::Vec<soroban_sdk::Val> = env
        .events()
        .all()
        .iter()
        .filter(|e| {
            e.1.len() >= 1
                && Symbol::try_from_val(&env, &e.1.get(0).unwrap())
                    == Ok(Symbol::new(&env, "ROModeChg"))
        })
        .map(|e| e.2)
        .collect();

    assert!(!payloads.is_empty(), "Expected ROModeChg event");

    let ev = ReadOnlyModeEvent::try_from_val(&env, &payloads[0])
        .expect("Failed to deserialize ReadOnlyModeEvent");
    assert_eq!(ev.event_version, EVENT_SCHEMA_VERSION);
    assert!(ev.enabled);
}

#[test]
fn read_only_mode_disable_event_has_event_version() {
    let env = default_env();
    let client = register(&env);
    let admin = Address::generate(&env);
    client.init_admin(&admin);

    client.set_read_only_mode(&true);
    client.set_read_only_mode(&false);

    let payloads: std::vec::Vec<soroban_sdk::Val> = env
        .events()
        .all()
        .iter()
        .filter(|e| {
            e.1.len() >= 1
                && Symbol::try_from_val(&env, &e.1.get(0).unwrap())
                    == Ok(Symbol::new(&env, "ROModeChg"))
        })
        .map(|e| e.2)
        .collect();

    assert!(payloads.len() >= 2, "Expected two ROModeChg events");

    for payload in &payloads {
        let ev = ReadOnlyModeEvent::try_from_val(&env, payload)
            .expect("Failed to deserialize ReadOnlyModeEvent");
        assert_eq!(ev.event_version, EVENT_SCHEMA_VERSION);
    }
}

// ---------------------------------------------------------------------------
// Struct field existence — compile-time proof that event_version is present
// ---------------------------------------------------------------------------

#[test]
fn upgrade_event_struct_has_event_version_field() {
    let env = Env::default();
    let hash = BytesN::from_array(&env, &[0u8; 32]);
    let ev = UpgradeEvent {
        new_wasm_hash: hash,
        previous_version: 1,
        timestamp: 0,
        event_version: EVENT_SCHEMA_VERSION,
    };
    assert_eq!(ev.event_version, EVENT_SCHEMA_VERSION);
}

#[test]
fn read_only_mode_event_struct_has_event_version_field() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let ev = ReadOnlyModeEvent {
        enabled: true,
        admin,
        timestamp: 0,
        event_version: EVENT_SCHEMA_VERSION,
    };
    assert_eq!(ev.event_version, EVENT_SCHEMA_VERSION);
}

#[test]
fn build_info_event_struct_has_event_version_field() {
    let env = Env::default();
    let ev = BuildInfoEvent {
        init_path: Symbol::new(&env, "test"),
        admin: None,
        signer_count: 0,
        threshold: 0,
        version: 1,
        timestamp: 0,
        event_version: EVENT_SCHEMA_VERSION,
    };
    assert_eq!(ev.event_version, EVENT_SCHEMA_VERSION);
}

#[test]
fn migration_event_struct_has_event_version_field() {
    let env = Env::default();
    let hash = BytesN::from_array(&env, &[0u8; 32]);
    let ev = MigrationEvent {
        from_version: 1,
        to_version: 2,
        timestamp: 0,
        migration_hash: hash,
        success: true,
        error_message: None,
        event_version: EVENT_SCHEMA_VERSION,
    };
    assert_eq!(ev.event_version, EVENT_SCHEMA_VERSION);
}

#[test]
fn migration_committed_event_struct_has_event_version_field() {
    let env = Env::default();
    let hash = BytesN::from_array(&env, &[0u8; 32]);
    let ev = MigrationCommittedEvent {
        target_version: 2,
        hash,
        committed_at: 0,
        expires_at: 0,
        event_version: EVENT_SCHEMA_VERSION,
    };
    assert_eq!(ev.event_version, EVENT_SCHEMA_VERSION);
}

// ---------------------------------------------------------------------------
// Compatibility: version mismatches are detected
// ---------------------------------------------------------------------------

#[test]
fn version_mismatch_detected_for_stale_version() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let ev = ReadOnlyModeEvent {
        enabled: false,
        admin,
        timestamp: 1_000,
        event_version: 0, // stale — pre-versioning
    };
    assert!(!crate::is_compatible_event_version(ev.event_version));
}

#[test]
fn version_mismatch_detected_for_future_version() {
    let env = Env::default();
    let hash = BytesN::from_array(&env, &[1u8; 32]);
    let ev = UpgradeEvent {
        new_wasm_hash: hash,
        previous_version: 2,
        timestamp: 5_000,
        event_version: EVENT_SCHEMA_VERSION + 99, // from a future contract version
    };
    assert!(!crate::is_compatible_event_version(ev.event_version));
}

#[test]
fn current_version_events_are_always_compatible() {
    let env = Env::default();
    let hash = BytesN::from_array(&env, &[2u8; 32]);

    let upgrade_ev = UpgradeEvent {
        new_wasm_hash: hash.clone(),
        previous_version: 1,
        timestamp: 0,
        event_version: EVENT_SCHEMA_VERSION,
    };
    assert!(crate::is_compatible_event_version(upgrade_ev.event_version));

    let ro_ev = ReadOnlyModeEvent {
        enabled: true,
        admin: Address::generate(&env),
        timestamp: 0,
        event_version: EVENT_SCHEMA_VERSION,
    };
    assert!(crate::is_compatible_event_version(ro_ev.event_version));

    let bi_ev = BuildInfoEvent {
        init_path: Symbol::new(&env, "adm_init"),
        admin: None,
        signer_count: 0,
        threshold: 0,
        version: 2,
        timestamp: 0,
        event_version: EVENT_SCHEMA_VERSION,
    };
    assert!(crate::is_compatible_event_version(bi_ev.event_version));

    let mig_ev = MigrationEvent {
        from_version: 1,
        to_version: 2,
        timestamp: 0,
        migration_hash: hash.clone(),
        success: true,
        error_message: None,
        event_version: EVENT_SCHEMA_VERSION,
    };
    assert!(crate::is_compatible_event_version(mig_ev.event_version));

    let commit_ev = MigrationCommittedEvent {
        target_version: 2,
        hash,
        committed_at: 0,
        expires_at: 9_999,
        event_version: EVENT_SCHEMA_VERSION,
    };
    assert!(crate::is_compatible_event_version(commit_ev.event_version));
}

// ---------------------------------------------------------------------------
// Build info event carries all expected metadata + version
// ---------------------------------------------------------------------------

#[test]
fn build_info_event_records_contract_version_and_admin() {
    let env = default_env();
    let client = register(&env);
    let admin = Address::generate(&env);

    client.init_admin(&admin);

    let payloads = find_events_by_topic(&env, "init", "build");
    let ev = BuildInfoEvent::try_from_val(&env, &payloads[0]).unwrap();

    assert_eq!(ev.event_version, EVENT_SCHEMA_VERSION);
    assert_eq!(ev.version, crate::VERSION);
    assert_eq!(ev.admin, Some(admin));
    assert_eq!(ev.signer_count, 0);
    assert_eq!(ev.threshold, 0);
}

#[test]
fn build_info_event_multisig_path_has_no_admin() {
    let env = default_env();
    let client = register(&env);
    let signers = vec![&env, Address::generate(&env), Address::generate(&env)];

    client.init(&signers, &2);

    let payloads = find_events_by_topic(&env, "init", "build");
    let ev = BuildInfoEvent::try_from_val(&env, &payloads[0]).unwrap();

    assert_eq!(ev.event_version, EVENT_SCHEMA_VERSION);
    assert!(ev.admin.is_none());
    assert_eq!(ev.signer_count, 2);
    assert_eq!(ev.threshold, 2);
}
