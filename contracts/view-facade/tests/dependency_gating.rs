//! # ABI-collision build gating guardrails (issue #1870)
//!
//! `view-facade` depends on `program-escrow` and `grainlify-core` with
//! `default-features = false`. That is load-bearing: both crates gate their
//! `#[contractimpl]` server behind a `contract` feature, so dropping the
//! `default-features = false` would force-export their entrypoints into the
//! facade and collide with the facade's own ABI at link time.
//!
//! These host-side tests pin that arrangement so an edit that silently drops
//! the gating fails the facade build instead of only failing a wasm link step.

use std::fs;
use std::path::{Path, PathBuf};

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("could not read {}: {e}", path.display()))
}

/// Returns the `[dependencies]` entry line for `name`.
fn dependency_line(manifest: &str, name: &str) -> String {
    manifest
        .lines()
        .find(|line| line.trim_start().starts_with(&format!("{name} ")))
        .unwrap_or_else(|| panic!("view-facade must depend on `{name}`"))
        .trim()
        .to_string()
}

#[test]
fn program_escrow_is_pulled_in_without_its_server() {
    let dep = dependency_line(&read("Cargo.toml"), "program-escrow");
    assert!(
        dep.contains("default-features = false"),
        "program-escrow must be depended on with `default-features = false` so its \
         server entrypoints do not collide with the facade ABI: `{dep}`"
    );
    assert!(
        !dep.contains("features") || !dep.contains("contract"),
        "program-escrow's `contract` feature must never be re-enabled by the facade: `{dep}`"
    );
}

#[test]
fn grainlify_core_is_pulled_in_without_its_server() {
    let dep = dependency_line(&read("Cargo.toml"), "grainlify-core");
    assert!(
        dep.contains("default-features = false"),
        "grainlify-core must be depended on with `default-features = false`: `{dep}`"
    );
    assert!(
        !dep.contains("features") || !dep.contains("contract"),
        "grainlify-core's `contract` feature must never be re-enabled by the facade: `{dep}`"
    );
}

#[test]
fn program_escrow_gates_its_server_behind_the_contract_feature() {
    let manifest = read("../program-escrow/Cargo.toml");
    assert!(
        manifest.contains("default = [\"contract\"]"),
        "program-escrow must keep `contract` as a default feature; otherwise \
         `default-features = false` in the facades no longer disables the server"
    );

    let lib = read("../program-escrow/src/lib.rs");
    assert!(
        lib.contains("#[cfg(feature = \"contract\")]"),
        "program-escrow's `#[contractimpl]` must stay behind `#[cfg(feature = \"contract\")]`"
    );
    assert!(
        lib.contains("impl ProgramEscrowContract"),
        "program-escrow must still declare the `ProgramEscrowContract` server impl"
    );
}

/// `CARGO_MANIFEST_DIR` is the facade crate dir; sanity-check the sibling path
/// used by the other assertions so a moved crate fails loudly here too.
#[test]
fn facade_lives_beside_the_contracts_it_models() {
    assert!(crate_dir().join("Cargo.toml").is_file());
    assert!(crate_dir().join("../program-escrow/Cargo.toml").is_file());
    assert!(crate_dir().join("../grainlify-core/Cargo.toml").is_file());
}
