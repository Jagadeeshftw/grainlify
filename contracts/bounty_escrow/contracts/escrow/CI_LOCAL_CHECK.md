# Run CI steps locally (Format, Build, Test, Stellar Build)

From repo root, run the same steps as `.github/workflows/contracts-ci.yml`:

```bash
# 1. Format check (CI fails here if code is not formatted)
cd contracts/bounty_escrow/contracts/escrow
cargo fmt --check --all
# If it fails, fix with:
cargo fmt --all

# 2. Build for WASM (same target as CI)
cargo build --release --target wasm32v1-none

# 3. All tests
cargo test --verbose --lib

# 4. Invariant checker CI tests
cargo test --verbose --lib invariant_checker_ci

# 5. Stellar contract build (requires Stellar CLI)
stellar contract build --verbose
```

On Windows, `stellar contract build` may require Stellar CLI installed. Steps 1–4 are enough to catch most CI failures.

## Fixture Hardening CI Steps

These steps specifically validate test fixture stability and gas budget enforcement regression guards:

```bash
# Run all gas budget enforcement tests (fixture hardening)
cargo test test_gas_budget -- --nocapture

# Run all gas profiling tests (deterministic measurement verification)
cargo test gas_profile -- --nocapture --test-threads=1

# Run the consolidated scaling summary
cargo test gas_profile_scaling_summary -- --nocapture

# Verify advisory status production-gap tests
cargo test test_advisory_status -- --nocapture

# Run is_any_cap_configured unit tests
cargo test test_is_any_cap_configured -- --nocapture
```

See `FIXTURE_HARDENING.md` for the full test fixture hardening strategy and regression surface documentation.
