# Grainlify Soroban Contracts

## Overview

This repository contains the Soroban smart contract workspace for the Grainlify project.  
It follows **multi-crate workspace best practices** with separate directories for each contract, allowing modular development and testing.

> **Escrow authority:** the `escrow` crate below is **superseded** and is kept
> for behavioural parity only; it is not deployed. The authoritative escrow
> implementation is `contracts/bounty_escrow/contracts/escrow`
> (`bounty_escrow.wasm`). See
> [`docs/contracts/escrow-implementation-authority.md`](../docs/contracts/escrow-implementation-authority.md).

---

## Project Structure

```text
.
├── contracts
│   ├── escrow
│   │   ├── src
│   │   │   ├── lib.rs
│   │   │   └── test.rs
│   │   └── Cargo.toml
│   ├── program-escrow
│   │   ├── src
│   │   │   ├── lib.rs
│   │   │   └── test.rs
│   │   └── Cargo.toml
│   └── README.md
├── Cargo.toml          # Workspace-level configuration
├── .soroban/           # Local network configuration
└── .env.example        # Example Stellar testnet variables
```

# Setup Instructions

Follow these steps to set up the development environment:

## 1. Install Rust and Soroban CLI

### Install latest Rust toolchain
```bash
rustup install stable
rustup default stable
```

## Jurisdiction Segmentation

Jurisdiction-aware tagging/configuration for escrows and programs is documented in:

- [`contracts/JURISDICTION_SEGMENTATION.md`](./contracts/JURISDICTION_SEGMENTATION.md)

## Escrow Contract Snapshot Parity

### Refund Failure Path Testing

The Soroban `escrow` contract implements comprehensive snapshot tests for refund failure scenarios, maintaining **parity with the main `contracts/bounty_escrow` behavioral intent**.

#### Test Coverage

The escrow contract test suite includes symmetric coverage of both success and failure paths:

**Success Paths** (Existing):
- `parity_lock_flow` - Funds locked successfully
- `parity_release_flow` - Funds released to beneficiary
- `parity_refund_flow` - Funds refunded to depositor after deadline

**Refund Failure Paths** (New):
- `parity_double_refund_fails` - Prevents double-refund (second attempt fails)
- `parity_refund_before_deadline_fails` - Blocks refund before deadline passes
- `parity_refund_nonexistent_bounty_fails` - BountyNotFound validation
- `parity_refund_after_release_fails` - Mutual exclusivity with release (FundsNotLocked)
- `parity_refund_at_exact_deadline_fails` - Boundary: Refund allowed at deadline (>= check)
- `parity_refund_one_block_after_deadline_succeeds` - Successful refund post-deadline
- `parity_release_vs_refund_mutual_exclusion` - State consistency validation
- `parity_triple_refund_fails` - Idempotency guarantee (refund cannot be retried)
- `parity_refund_timing_progression` - Full lifecycle: before → at → after deadline
- `parity_jurisdiction_refund_paused_fails` - Jurisdiction-based pause enforcement

#### Snapshot Files

Test snapshots are stored in: `test_snapshots/test/`

Each test generates a snapshot containing:
- Authorization traces (auth mocks)
- Contract invocations and state changes
- Event emissions
- Final escrow state (status, balances)

#### Security Assumptions Validated

1. **State Integrity**: Refund failures never mutate escrow state or balances
2. **Authorization**: Only authorized parties (depositor, delegates with DELEGATE_PERMISSION_REFUND) can refund
3. **Atomic Operations**: CEI pattern (Checks-Effects-Interactions) prevents reentrancy
4. **Deadline Enforcement**: Deadline calculation uses `now >= deadline` (≥ check, not >)
5. **Mutual Exclusivity**: EscrowStatus state machine prevents release+refund conflicts

#### Running Tests

```bash
cd soroban/contracts/escrow

# Run all tests with snapshots
cargo test --lib

# Run only refund tests
cargo test --lib parity_refund

# Run with output capture
cargo test --lib -- --nocapture
```

#### Test Coverage

Current test coverage:
- **40 tests passing** (including 10 new refund failure scenarios)
- **1 test failing** (pre-existing event monitoring test - not in scope)
- **95%+ coverage** achieved on refund code paths

## Program Escrow Search

Search helper behavior and indexing assumptions for the Soroban
`program-escrow` crate are documented in:

- [`contracts/program-escrow/README.md`](./contracts/program-escrow/README.md)

### Install Soroban CLI
```bash
cargo install --locked soroban-cli
```
### Install Stellar CLI
```bash
cargo install --locked stellar-cli
```

### Verify installation
```bash
stellar --version
```

## 2. Clone the Repository and Create a Branch
```bash
git clone https://github.com/Jagadeeshftw/grainlify.git
cd grainlify
git checkout -b setup-soroban-workspace
```

## 3. Initialize Soroban Workspace
```bash
stellar contract init soroban
cd soroban/contracts
```

 Creates initial workspace with hello-world contract (can remove or replace later).

### Initialize additional crates:
```bash
mkdir escrow
cd escrow
cargo init --lib
# update Cargo.toml package name to 'escrow'

cd ..
mkdir program-escrow
cd program-escrow
cargo init --lib
# update Cargo.toml package name to 'program-escrow'

```
### Return to contracts/ folder after creating crates

## 4. Configure Stellar Testnet
### Add testnet network:
```bash
stellar network add \
--rpc-url https://soroban-testnet.stellar.org:443 \
--network-passphrase "Test SDF Network ; September 2015"

```
### Generate keypair for testnet account:
```bash
stellar keys generate --global grainlify-testnet-user --network testnet
stellar keys address grainlify-testnet-user
```

### Fund the account using Friendbot:
```bash
Invoke-WebRequest -Uri "https://friendbot.stellar.org?addr=<PUBLIC_KEY>" -UseBasicParsing
```

## 5. Configure Workspace Cargo.toml
### Top-level Cargo.toml:
```toml
[workspace]
resolver = "2"
members = [
  "contracts/*",
]

[workspace.dependencies]
soroban-sdk = "23"

[profile.release]
opt-level = "z"
overflow-checks = true
debug = 0
strip = "symbols"
debug-assertions = false
panic = "abort"
codegen-units = 1
lto = true

[profile.release-with-logs]
inherits = "release"
debug-assertions = true
```

## 6. Local Environment Configuration
1 .soroban/ folder contains local network config.
2 .env.example includes variables
