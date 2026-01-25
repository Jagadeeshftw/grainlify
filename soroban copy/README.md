# Grainlify Soroban Contracts

## Overview

This repository contains the Soroban smart contract workspace for the Grainlify project.  
It follows **multi-crate workspace best practices** with separate directories for each contract, allowing modular development and testing.

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

