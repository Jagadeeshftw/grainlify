//! # ABI Stability Matrix Doc-Link Tests
//!
//! These tests verify that:
//! 1. `docs/abi-stability-matrix.md` exists at the repository root.
//! 2. Each of the five Grainlify Soroban contract `lib.rs` files contains a reference to
//!    the ABI stability matrix, ensuring the cross-link stays in place as contracts evolve.
//!
//! These are host-side tests (not WASM); they run with `cargo test -p grainlify-core`.
//!
//! ## Security Note
//! These tests act as a static guardrail: if someone removes the stability matrix reference
//! from a contract's crate-level docs, the corresponding assertion here will fail, making
//! the omission visible during CI.
//!
//! See: [`docs/abi-stability-matrix.md`](../../../../docs/abi-stability-matrix.md)

use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Helper: resolve a path relative to the workspace root by traversing up from
// the current `CARGO_MANIFEST_DIR` (which points to the crate directory).
// ---------------------------------------------------------------------------

/// Returns the absolute path to the repository root, derived from `CARGO_MANIFEST_DIR`.
///
/// The grainlify-core crate lives at `contracts/grainlify-core`, so we walk up two levels
/// from the manifest directory to reach the repo root.
fn repo_root() -> std::path::PathBuf {
    // CARGO_MANIFEST_DIR is set by Cargo at compile time; it resolves to the directory
    // containing the `Cargo.toml` of the crate under test.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    // contracts/grainlify-core  →  contracts  →  repo root
    Path::new(manifest_dir)
        .parent()
        .expect("contracts/grainlify-core should have a parent (contracts/)")
        .parent()
        .expect("contracts/ should have a parent (repo root)")
        .to_path_buf()
}

// ---------------------------------------------------------------------------
// Test 1 — Matrix file exists
// ---------------------------------------------------------------------------

/// Asserts that `docs/abi-stability-matrix.md` exists at the repository root.
///
/// This is the canonical cross-contract ABI stability reference. If the file is
/// renamed or removed, all five contract crate-level doc links become broken and
/// this test will surface the regression.
#[test]
fn abi_stability_matrix_file_exists() {
    let matrix_path = repo_root().join("docs").join("abi-stability-matrix.md");
    assert!(
        matrix_path.exists(),
        "docs/abi-stability-matrix.md must exist at the repository root. \
         Path checked: {}",
        matrix_path.display()
    );
    assert!(
        matrix_path.is_file(),
        "docs/abi-stability-matrix.md must be a regular file, not a directory. \
         Path checked: {}",
        matrix_path.display()
    );
}

/// Asserts that `docs/abi-stability-matrix.md` is non-empty and contains the
/// canonical section headers that make it a useful reference document.
#[test]
fn abi_stability_matrix_file_is_complete() {
    let matrix_path = repo_root().join("docs").join("abi-stability-matrix.md");
    let content = fs::read_to_string(&matrix_path).unwrap_or_else(|e| {
        panic!(
            "Failed to read docs/abi-stability-matrix.md: {}\nPath: {}",
            e,
            matrix_path.display()
        )
    });

    assert!(
        !content.trim().is_empty(),
        "docs/abi-stability-matrix.md must not be empty"
    );

    // Verify the document contains the key structural sections expected by integrators.
    let required_sections = [
        "Stability Classifications",
        "Breaking vs Additive Changes",
        "Synchronization-Risk Types",
        "program-escrow",
        "bounty-escrow",
        "grainlify-core",
        "view-facade",
        "escrow-view-facade",
    ];

    for section in &required_sections {
        assert!(
            content.contains(section),
            "docs/abi-stability-matrix.md is missing required section: '{}'",
            section
        );
    }
}

// ---------------------------------------------------------------------------
// Test 2 — Each contract's lib.rs references the matrix
// ---------------------------------------------------------------------------

/// Checks that a given `lib.rs` file contains a reference to `abi-stability-matrix`,
/// proving the crate-level doc comment cross-link is present.
fn assert_lib_rs_references_matrix(relative_path: &str) {
    let lib_path = repo_root().join(relative_path);
    assert!(
        lib_path.exists(),
        "Contract lib.rs not found at expected path: {}",
        lib_path.display()
    );

    let content = fs::read_to_string(&lib_path).unwrap_or_else(|e| {
        panic!(
            "Failed to read {}: {}\nPath: {}",
            relative_path,
            e,
            lib_path.display()
        )
    });

    assert!(
        content.contains("abi-stability-matrix"),
        "lib.rs at '{}' does not reference `abi-stability-matrix`. \
         Every Grainlify contract crate must include a doc comment linking to \
         `docs/abi-stability-matrix.md`.",
        relative_path
    );
}

/// program-escrow references the ABI stability matrix in its crate-level doc comment.
#[test]
fn program_escrow_lib_rs_references_abi_matrix() {
    assert_lib_rs_references_matrix("contracts/program-escrow/src/lib.rs");
}

/// bounty-escrow references the ABI stability matrix in its crate-level doc comment.
#[test]
fn bounty_escrow_lib_rs_references_abi_matrix() {
    assert_lib_rs_references_matrix("contracts/bounty_escrow/contracts/escrow/src/lib.rs");
}

/// grainlify-core references the ABI stability matrix in its crate-level doc comment.
#[test]
fn grainlify_core_lib_rs_references_abi_matrix() {
    assert_lib_rs_references_matrix("contracts/grainlify-core/src/lib.rs");
}

/// view-facade references the ABI stability matrix in its crate-level doc comment.
#[test]
fn view_facade_lib_rs_references_abi_matrix() {
    assert_lib_rs_references_matrix("contracts/view-facade/src/lib.rs");
}

/// escrow-view-facade references the ABI stability matrix in its crate-level doc comment.
#[test]
fn escrow_view_facade_lib_rs_references_abi_matrix() {
    assert_lib_rs_references_matrix("contracts/escrow-view-facade/src/lib.rs");
}

// ---------------------------------------------------------------------------
// Test 3 — Matrix content consistency: sync-risk types are documented
// ---------------------------------------------------------------------------

/// Asserts that the matrix explicitly documents `PayoutRecord` as a synchronization risk.
/// This type drifts between program-escrow and view-facade and must remain flagged.
#[test]
fn matrix_documents_payout_record_sync_risk() {
    let matrix_path = repo_root().join("docs").join("abi-stability-matrix.md");
    let content = fs::read_to_string(&matrix_path)
        .expect("docs/abi-stability-matrix.md should be readable");

    assert!(
        content.contains("PayoutRecord"),
        "docs/abi-stability-matrix.md must document `PayoutRecord` as a synchronization-risk type \
         (it is mirrored with field drift between program-escrow and view-facade)"
    );
}

/// Asserts that the matrix explicitly documents `ProgramDelegateInfo` as a sync risk.
#[test]
fn matrix_documents_program_delegate_info_sync_risk() {
    let matrix_path = repo_root().join("docs").join("abi-stability-matrix.md");
    let content = fs::read_to_string(&matrix_path)
        .expect("docs/abi-stability-matrix.md should be readable");

    assert!(
        content.contains("ProgramDelegateInfo"),
        "docs/abi-stability-matrix.md must document `ProgramDelegateInfo` as a synchronization-risk \
         type (it is mirrored in escrow-view-facade/src/program_escrow_bindings.rs)"
    );
}

/// Asserts that the matrix explicitly documents `EscrowStatus` as a sync risk.
#[test]
fn matrix_documents_escrow_status_sync_risk() {
    let matrix_path = repo_root().join("docs").join("abi-stability-matrix.md");
    let content = fs::read_to_string(&matrix_path)
        .expect("docs/abi-stability-matrix.md should be readable");

    assert!(
        content.contains("EscrowStatus"),
        "docs/abi-stability-matrix.md must document `EscrowStatus` as a synchronization-risk \
         type (it is exhaustively matched across multiple facade files)"
    );
}

/// Asserts that the matrix defines what counts as a breaking change.
#[test]
fn matrix_defines_breaking_changes() {
    let matrix_path = repo_root().join("docs").join("abi-stability-matrix.md");
    let content = fs::read_to_string(&matrix_path)
        .expect("docs/abi-stability-matrix.md should be readable");

    // Verify the core breaking-change concepts are described.
    let breaking_concepts = [
        "Breaking Change",
        "Removing a field",
        "Reordering fields",
    ];
    for concept in &breaking_concepts {
        assert!(
            content.contains(concept),
            "docs/abi-stability-matrix.md must describe breaking changes, including: '{}'",
            concept
        );
    }
}

/// Asserts that the matrix defines what counts as an additive (non-breaking) change.
#[test]
fn matrix_defines_additive_changes() {
    let matrix_path = repo_root().join("docs").join("abi-stability-matrix.md");
    let content = fs::read_to_string(&matrix_path)
        .expect("docs/abi-stability-matrix.md should be readable");

    assert!(
        content.contains("Additive"),
        "docs/abi-stability-matrix.md must describe additive (non-breaking) changes"
    );
}
