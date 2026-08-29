//! # Soroban SDK Pin Consistency Tests (issue #1743)
//!
//! These tests verify that:
//! 1. Every Grainlify manifest pins soroban-sdk to an exact version.
//! 2. The pins match the versions documented in `contracts/SDK_COMPATIBILITY.md`
//!    and the table inside `scripts/check_sdk_versions.sh`.
//! 3. This crate's committed `Cargo.lock` resolves exactly one soroban-sdk.
//!
//! These are host-side tests (not WASM); they run with `cargo test -p grainlify-core`.
//!
//! ## Security Note
//! A dependency edit that loosens a pin (say `"=21.7.7"` -> `"21"`) or drifts
//! one manifest away from the others would otherwise land silently; the mixed
//! 21.7.7/23.4.1 graph that used to live in the `soroban` workspace is exactly
//! the failure mode these assertions make visible. The full resolved-graph
//! check runs in CI via `scripts/check_sdk_versions.sh`; this test is the
//! fast local guardrail over the manifests themselves.
//!
//! See: [`contracts/SDK_COMPATIBILITY.md`](../../SDK_COMPATIBILITY.md)

use std::fs;
use std::path::Path;

/// The exact soroban-sdk pin shared by every crate under `contracts/`.
const CONTRACTS_PIN: &str = "=21.7.7";

/// The exact soroban-sdk pin of the `soroban` workspace.
const SOROBAN_WS_PIN: &str = "=23.4.1";

/// Returns the absolute path to the repository root, derived from `CARGO_MANIFEST_DIR`.
///
/// The grainlify-core crate lives at `contracts/grainlify-core`, so we walk up two
/// levels from the manifest directory to reach the repo root.
fn repo_root() -> std::path::PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir)
        .parent()
        .expect("contracts/grainlify-core should have a parent (contracts/)")
        .parent()
        .expect("contracts/ should have a parent (repo root)")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    let path = repo_root().join(rel);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("could not read {}: {e}", path.display()))
}

/// Every soroban-sdk requirement in a manifest must be `workspace = true` or an
/// exact `=`-pin equal to `expected`.
fn assert_manifest_pins(rel: &str, expected: &str) {
    let manifest = read(rel);
    let mut seen = 0;
    for line in manifest.lines() {
        if !line.trim_start().starts_with("soroban-sdk") {
            continue;
        }
        seen += 1;
        if line.contains("workspace = true") {
            continue;
        }
        let wanted = format!("\"{expected}\"");
        assert!(
            line.contains(&wanted),
            "{rel}: soroban-sdk requirement is not the exact pin {expected}: `{line}`"
        );
    }
    assert!(seen > 0, "{rel}: expected at least one soroban-sdk requirement");
}

// ---------------------------------------------------------------------------
// Test 1 - every contracts/ manifest carries the shared exact pin
// ---------------------------------------------------------------------------

#[test]
fn contracts_manifests_pin_exact_sdk() {
    for rel in [
        "contracts/Cargo.toml",
        "contracts/grainlify-core/Cargo.toml",
        "contracts/program-escrow/Cargo.toml",
        "contracts/view-facade/Cargo.toml",
        "contracts/escrow-view-facade/Cargo.toml",
        "contracts/bounty_escrow/Cargo.toml",
        "contracts/bounty_escrow/contracts/escrow/Cargo.toml",
    ] {
        assert_manifest_pins(rel, CONTRACTS_PIN);
    }
}

// ---------------------------------------------------------------------------
// Test 2 - the soroban workspace carries its own exact pin
// ---------------------------------------------------------------------------

#[test]
fn soroban_workspace_pins_exact_sdk() {
    assert_manifest_pins("soroban/Cargo.toml", SOROBAN_WS_PIN);
    for rel in [
        "soroban/contracts/escrow/Cargo.toml",
        "soroban/contracts/program-escrow/Cargo.toml",
        "soroban/contracts/stream/Cargo.toml",
    ] {
        assert_manifest_pins(rel, SOROBAN_WS_PIN);
    }
}

// ---------------------------------------------------------------------------
// Test 3 - this crate's lockfile resolves exactly one soroban-sdk
// ---------------------------------------------------------------------------

#[test]
fn own_lockfile_resolves_single_sdk() {
    let lock = read("contracts/grainlify-core/Cargo.lock");
    let entries: Vec<&str> = lock
        .split("[[package]]")
        .filter(|block| block.contains("name = \"soroban-sdk\""))
        .collect();
    assert_eq!(
        entries.len(),
        1,
        "contracts/grainlify-core/Cargo.lock must resolve exactly one soroban-sdk, found {}",
        entries.len()
    );
    let expected = format!("version = \"{}\"", CONTRACTS_PIN.trim_start_matches('='));
    assert!(
        entries[0].contains(&expected),
        "locked soroban-sdk version does not match the {CONTRACTS_PIN} pin"
    );
}

// ---------------------------------------------------------------------------
// Test 4 - the policy doc and the CI gate script agree with the pins here
// ---------------------------------------------------------------------------

#[test]
fn compatibility_doc_documents_the_pins() {
    let doc = read("contracts/SDK_COMPATIBILITY.md");
    for needle in [CONTRACTS_PIN, SOROBAN_WS_PIN, "scripts/check_sdk_versions.sh"] {
        assert!(
            doc.contains(needle),
            "contracts/SDK_COMPATIBILITY.md must mention `{needle}`; update the doc \
             and this test together when the pin moves"
        );
    }
}

/// The pinned SDK must actually run at the protocol the policy documents for
/// it: the =21.7.7 tree targets protocol 21.
#[test]
fn sdk_runs_at_the_documented_target_protocol() {
    use soroban_sdk::testutils::Ledger as _;

    let env = soroban_sdk::Env::default();
    assert_eq!(
        env.ledger().get().protocol_version,
        21,
        "soroban-sdk {CONTRACTS_PIN} should target protocol 21; if this moved, \
         update contracts/SDK_COMPATIBILITY.md and the drifted test \
         protocol_version values it lists"
    );
}

#[test]
fn gate_script_table_matches_the_pins() {
    let script = read("scripts/check_sdk_versions.sh");
    for needle in [
        &format!("contracts/grainlify-core|{}", CONTRACTS_PIN.trim_start_matches('=')),
        &format!("soroban|{}", SOROBAN_WS_PIN.trim_start_matches('=')),
    ] {
        assert!(
            script.contains(needle.as_str()),
            "scripts/check_sdk_versions.sh pin table must contain `{needle}`"
        );
    }
}
