//! # Feature-Flag Matrix & Facade Dependency Assertions (Issue #1885)
//!
//! These tests verify that:
//! 1. The feature-flag model is documented in `contracts/grainlify-core/FEATURE_FLAGS.md`.
//! 2. `grainlify-core/Cargo.toml` declares the seven expected cargo features.
//! 3. Downstream contracts and facades (`program-escrow`, `view-facade`, `escrow-view-facade`)
//!    explicitly enforce `default-features = false` when depending on `grainlify-core` and
//!    `program-escrow` to avoid duplicate contract entrypoint link collisions.
//! 4. `grainlify-core/src/lib.rs` contains fail-fast compile-time assertions for unsupported
//!    feature combinations.
//! 5. Unsupported combinations fail with clear compile-time error messages rather than
//!    unresolved link errors.
//! 6. The CI validation script `contracts/scripts/check-feature-matrix.sh` exists and exercises
//!    all supported combinations.

use std::fs;
use std::path::{Path, PathBuf};

/// Expected seven cargo features declared by `grainlify-core`.
const EXPECTED_FEATURES: &[&str] = &[
    "default",
    "contract",
    "strict-mode",
    "testutils",
    "upgrade_rollback_tests",
    "governance_contract_tests",
    "wasm_tests",
];

/// Returns the absolute path to the repository root.
fn repo_root() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir)
        .parent()
        .expect("contracts/grainlify-core should have parent (contracts/)")
        .parent()
        .expect("contracts/ should have parent (repo root)")
        .to_path_buf()
}

fn read_file(rel: &str) -> String {
    let path = repo_root().join(rel);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("could not read file at {}: {e}", path.display()))
}

// ---------------------------------------------------------------------------
// Test 1: Documentation of Feature-Flag Model & Supported Combinations
// ---------------------------------------------------------------------------

#[test]
fn feature_flags_doc_exists_and_documents_model() {
    let doc = read_file("contracts/grainlify-core/FEATURE_FLAGS.md");

    // Must document each of the seven cargo features
    for feature in EXPECTED_FEATURES {
        assert!(
            doc.contains(&format!("`{feature}`")),
            "FEATURE_FLAGS.md must document feature `{feature}`"
        );
    }

    // Must document supported combinations table and what each is for
    assert!(
        doc.contains("Supported Feature Combinations"),
        "FEATURE_FLAGS.md must contain a 'Supported Feature Combinations' section"
    );
    assert!(
        doc.contains("--no-default-features"),
        "FEATURE_FLAGS.md must explain library mode (--no-default-features)"
    );
    assert!(
        doc.contains("default-features = false"),
        "FEATURE_FLAGS.md must document the mandatory facade 'default-features = false' requirement"
    );
    assert!(
        doc.contains("Unsupported Combinations"),
        "FEATURE_FLAGS.md must document unsupported feature combinations"
    );
}

// ---------------------------------------------------------------------------
// Test 2: Cargo.toml declares the exact seven features and links documentation
// ---------------------------------------------------------------------------

#[test]
fn grainlify_core_declares_exact_seven_features() {
    let manifest = read_file("contracts/grainlify-core/Cargo.toml");

    for feature in EXPECTED_FEATURES {
        assert!(
            manifest.contains(&format!("{feature} = ")),
            "contracts/grainlify-core/Cargo.toml must declare feature `{feature}`"
        );
    }

    // Default must include "contract"
    assert!(
        manifest.contains("default = [\"contract\"]"),
        "contracts/grainlify-core/Cargo.toml default must be [\"contract\"]"
    );

    // Manifest must cross-reference FEATURE_FLAGS.md
    assert!(
        manifest.contains("FEATURE_FLAGS.md"),
        "contracts/grainlify-core/Cargo.toml should reference FEATURE_FLAGS.md in comments"
    );
}

// ---------------------------------------------------------------------------
// Test 3: Facades and downstream contracts MUST declare default-features = false
// ---------------------------------------------------------------------------

#[test]
fn facades_assert_default_features_false_requirement() {
    // 1. Check contracts/program-escrow/Cargo.toml
    let program_escrow_toml = read_file("contracts/program-escrow/Cargo.toml");
    assert!(
        program_escrow_toml.contains("grainlify-core = {"),
        "program-escrow must declare grainlify-core dependency"
    );
    assert!(
        program_escrow_toml.contains("grainlify-core = { path = \"../grainlify-core\", default-features = false }"),
        "program-escrow/Cargo.toml must specify `default-features = false` for grainlify-core"
    );

    // 2. Check contracts/view-facade/Cargo.toml
    let view_facade_toml = read_file("contracts/view-facade/Cargo.toml");
    assert!(
        view_facade_toml.contains("grainlify-core = {"),
        "view-facade must declare grainlify-core dependency"
    );
    assert!(
        view_facade_toml.contains("grainlify-core = { path = \"../grainlify-core\", default-features = false }"),
        "view-facade/Cargo.toml must specify `default-features = false` for grainlify-core"
    );
    assert!(
        view_facade_toml.contains("program-escrow = { path = \"../program-escrow\", default-features = false }"),
        "view-facade/Cargo.toml must specify `default-features = false` for program-escrow"
    );

    // 3. Check contracts/escrow-view-facade/Cargo.toml
    let escrow_view_facade_toml = read_file("contracts/escrow-view-facade/Cargo.toml");
    assert!(
        escrow_view_facade_toml.contains("program-escrow = {"),
        "escrow-view-facade must declare program-escrow dependency"
    );
    assert!(
        escrow_view_facade_toml.contains("program-escrow = { path = \"../program-escrow\", default-features = false }"),
        "escrow-view-facade/Cargo.toml must specify `default-features = false` for program-escrow"
    );
}

// ---------------------------------------------------------------------------
// Test 4: lib.rs contains compile_error checks for unsupported combinations
// ---------------------------------------------------------------------------

#[test]
fn lib_rs_guards_unsupported_combinations() {
    let lib_rs = read_file("contracts/grainlify-core/src/lib.rs");

    assert!(
        lib_rs.contains("FEATURE_FLAGS.md"),
        "contracts/grainlify-core/src/lib.rs must reference FEATURE_FLAGS.md"
    );
    assert!(
        lib_rs.contains("compile_error!"),
        "contracts/grainlify-core/src/lib.rs must define compile_error! guards"
    );
    assert!(
        lib_rs.contains("wasm_tests"),
        "contracts/grainlify-core/src/lib.rs must guard wasm_tests requiring contract"
    );
    assert!(
        lib_rs.contains("upgrade_rollback_tests"),
        "contracts/grainlify-core/src/lib.rs must guard upgrade_rollback_tests requiring contract"
    );
    assert!(
        lib_rs.contains("governance_contract_tests"),
        "contracts/grainlify-core/src/lib.rs must guard governance_contract_tests requiring contract"
    );
    assert!(
        lib_rs.contains("target_arch = \"wasm32\"") && lib_rs.contains("feature = \"testutils\""),
        "contracts/grainlify-core/src/lib.rs must guard testutils on wasm32"
    );
}

// ---------------------------------------------------------------------------
// Test 5: Unsupported combination fails with a clear message rather than link error
// ---------------------------------------------------------------------------

#[test]
fn unsupported_combination_fails_with_clear_message() {
    let manifest_path = repo_root().join("contracts/grainlify-core/Cargo.toml");
    let mut cmd = std::process::Command::new("cargo");
    cmd.args([
        "check",
        "--manifest-path",
        manifest_path.to_str().unwrap(),
        "--no-default-features",
        "--features",
        "wasm_tests",
    ]);

    // Forward CARGO_TARGET_DIR if set
    if let Ok(target_dir) = std::env::var("CARGO_TARGET_DIR") {
        cmd.env("CARGO_TARGET_DIR", target_dir);
    }

    let output = cmd.output().expect("cargo check command should run");

    assert!(
        !output.status.success(),
        "unsupported combination (--no-default-features --features wasm_tests) must fail compilation"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Feature combination error: 'wasm_tests' requires the 'contract' feature"),
        "stderr must contain clear compile_error message, got:\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// Test 6: CI script check-feature-matrix.sh exists and covers all combinations
// ---------------------------------------------------------------------------

#[test]
fn ci_script_exists_and_covers_matrix() {
    let script = read_file("contracts/scripts/check-feature-matrix.sh");

    for feature in EXPECTED_FEATURES {
        if *feature != "default" {
            assert!(
                script.contains(&format!("--features {feature}")),
                "check-feature-matrix.sh must build combination with feature `{feature}`"
            );
        }
    }

    assert!(
        script.contains("--no-default-features"),
        "check-feature-matrix.sh must build library-only combination (--no-default-features)"
    );
    assert!(
        script.contains("--all-features"),
        "check-feature-matrix.sh must build all-features combination"
    );
    assert!(
        script.contains("wasm32-unknown-unknown"),
        "check-feature-matrix.sh must build wasm32 combinations"
    );
}
