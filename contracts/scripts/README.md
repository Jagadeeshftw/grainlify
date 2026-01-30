# Grainlify Deployment Scripts

This directory contains automation scripts for deploying, upgrading, verifying, and managing Soroban smart contracts on the Stellar network.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [Scripts Reference](#scripts-reference)
  - [deploy.sh](#deploysh)
  - [upgrade.sh](#upgradesh)
  - [verify-deployment.sh](#verify-deploymentsh)
  - [rollback.sh](#rollbacksh)
- [Network Safety](#network-safety)
- [Deployment Registry](#deployment-registry)
- [Troubleshooting](#troubleshooting)

---

## Prerequisites

### Required Tools

| Tool | Version | Installation |
|------|---------|--------------|
| **Stellar CLI** | Latest | `cargo install --locked stellar-cli` |
| **jq** | 1.6+ | `apt install jq` or `brew install jq` |
| **Rust** | Stable | [rustup.rs](https://rustup.rs) |

### Verify Installation

```bash
# Check Stellar CLI
stellar --version

# Check jq
jq --version

# Check Rust and WASM target
rustup target list --installed | grep wasm32
```

### Install WASM Target (if missing)

```bash
rustup target add wasm32-unknown-unknown
```

---

## Quick Start

```bash
# 1. Create a deployer identity
stellar keys generate --global grainlify-deployer

# 2. Fund the account (testnet only)
stellar keys fund grainlify-deployer --network testnet

# 3. Build contracts
cd soroban && cargo build --release --target wasm32-unknown-unknown

# 4. Deploy
cd .. && ./contracts/scripts/deploy.sh soroban/target/wasm32-unknown-unknown/release/escrow.wasm
```

---

## Configuration

### Environment Files

Configuration is stored in `contracts/scripts/config/`:

| File | Purpose |
|------|---------|
| `testnet.env` | Stellar Testnet configuration |
| `mainnet.env` | Stellar Mainnet configuration (production) |

### Setting Up Testnet

1. **Copy and customize the config:**

```bash
# The default testnet.env works out of the box
# Optionally create a local override:
cp contracts/scripts/config/testnet.env contracts/scripts/config/testnet.env.local
```

2. **Edit key values:**

```bash
# contracts/scripts/config/testnet.env

# Network endpoint
SOROBAN_RPC_URL="https://soroban-testnet.stellar.org"
SOROBAN_NETWORK="testnet"

# Your deployer identity (create with: stellar keys generate)
DEPLOYER_IDENTITY="grainlify-deployer"

# Safety settings
REQUIRE_CONFIRMATION="true"
DRY_RUN="false"
```

3. **Create and fund your identity:**

```bash
# Generate a new keypair
stellar keys generate --global grainlify-deployer

# View the public address
stellar keys address grainlify-deployer

# Fund with testnet XLM (via Friendbot)
stellar keys fund grainlify-deployer --network testnet
```

### Setting Up Mainnet

> **Warning:** Mainnet deployments use real XLM and are irreversible.

1. **Create a separate mainnet identity:**

```bash
# NEVER reuse testnet keys for mainnet
stellar keys generate --global grainlify-mainnet-deployer
```

2. **Fund with real XLM:**
   - Transfer XLM from an exchange or existing wallet
   - Minimum ~10 XLM recommended for deployments

3. **Update mainnet config:**

```bash
# contracts/scripts/config/mainnet.env
DEPLOYER_IDENTITY="grainlify-mainnet-deployer"
```

---

## Scripts Reference

### deploy.sh

Deploys a new smart contract to the network.

#### Usage

```bash
./contracts/scripts/deploy.sh <wasm_file> [options]
```

#### Options

| Option | Description | Default |
|--------|-------------|---------|
| `-n, --network` | Target network | `testnet` |
| `-i, --identity` | Deployer identity | from config |
| `-N, --name` | Contract name for registry | filename |
| `--init` | Call init() after deploy | false |
| `--init-args` | Arguments for init | - |
| `--dry-run` | Simulate only | false |
| `-v, --verbose` | Detailed output | false |

#### Examples

```bash
# Deploy escrow contract to testnet
./contracts/scripts/deploy.sh soroban/target/wasm32-unknown-unknown/release/escrow.wasm

# Deploy with custom name
./contracts/scripts/deploy.sh escrow.wasm -N bounty-escrow-v1

# Deploy and initialize with admin
./contracts/scripts/deploy.sh escrow.wasm --init --init-args '--admin GABC...'

# Deploy to mainnet (requires confirmation)
./contracts/scripts/deploy.sh escrow.wasm -n mainnet -i mainnet-deployer

# Dry run (see what would happen)
./contracts/scripts/deploy.sh escrow.wasm --dry-run --verbose
```

#### Output

On success, the script:
1. Prints the new **Contract ID**
2. Records the deployment in `contracts/deployments/<network>.json`

```
Contract ID: CABC123DEF456...
```

---

### upgrade.sh

Upgrades an existing contract to a new WASM version.

#### Usage

```bash
./contracts/scripts/upgrade.sh <contract_id> <new_wasm_path> [options]
```

#### Options

| Option | Description | Default |
|--------|-------------|---------|
| `-n, --network` | Target network | `testnet` |
| `-s, --source` | Signing identity | from config |
| `--skip-verify` | Skip post-upgrade check | false |
| `--dry-run` | Simulate only | false |

#### Examples

```bash
# Upgrade contract on testnet
./contracts/scripts/upgrade.sh CABC123... ./target/release/escrow.wasm

# Upgrade on mainnet with specific identity
./contracts/scripts/upgrade.sh CABC123... escrow.wasm -n mainnet -s admin-key

# Upgrade without verification
./contracts/scripts/upgrade.sh CABC123... escrow.wasm --skip-verify

# Preview upgrade (dry run)
./contracts/scripts/upgrade.sh CABC123... escrow.wasm --dry-run
```

#### How It Works

1. **Installs** the new WASM code (gets `wasm_hash`)
2. **Invokes** the contract's `upgrade(new_wasm_hash)` function
3. **Verifies** the contract responds after upgrade
4. **Logs** the upgrade to `contracts/deployments/upgrades.json`

#### Prerequisites

- Contract must have an `upgrade(new_wasm_hash: BytesN<32>)` function
- Signing identity must be the contract admin

---

### verify-deployment.sh

Checks if a deployed contract is healthy and responsive.

#### Usage

```bash
./contracts/scripts/verify-deployment.sh <contract_id> [options]
```

#### Options

| Option | Description | Default |
|--------|-------------|---------|
| `-n, --network` | Target network | `testnet` |
| `-f, --function` | Function to call | `get_version` |
| `--check-admin` | Verify admin address | false |
| `--expected-admin` | Expected admin value | - |
| `--json` | Output as JSON | false |

#### Examples

```bash
# Basic health check
./contracts/scripts/verify-deployment.sh CABC123...

# Check on mainnet
./contracts/scripts/verify-deployment.sh CABC123... -n mainnet

# Use custom verification function
./contracts/scripts/verify-deployment.sh CABC123... -f get_balance

# Verify admin matches expected
./contracts/scripts/verify-deployment.sh CABC123... --check-admin --expected-admin GABC...

# JSON output (for CI/CD pipelines)
./contracts/scripts/verify-deployment.sh CABC123... --json
```

#### Output

**Human-readable:**
```
Status:          HEALTHY
Contract ID:     CABC123...
Network:         testnet
Function:        get_version
Result:          1
```

**JSON mode (`--json`):**
```json
{
  "contract_id": "CABC123...",
  "network": "testnet",
  "status": "HEALTHY",
  "verification": {
    "function": "get_version",
    "result": "1"
  },
  "verified_at": "2024-01-15T10:30:00Z"
}
```

#### Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Contract is HEALTHY |
| `1` | Contract is UNRESPONSIVE |
| `2` | Invalid arguments |

#### Use in CI/CD

```bash
# In a GitHub Action or script
if ./contracts/scripts/verify-deployment.sh "$CONTRACT_ID" --json; then
  echo "Deployment verified"
else
  echo "Deployment failed verification"
  exit 1
fi
```

---

### rollback.sh

Reverts a contract to a previous WASM version.

#### Usage

```bash
./contracts/scripts/rollback.sh <contract_id> <previous_wasm_hash> [options]
```

#### Options

| Option | Description | Default |
|--------|-------------|---------|
| `-n, --network` | Target network | `testnet` |
| `-s, --source` | Signing identity | from config |
| `--force` | Skip confirmations | false |
| `--dry-run` | Simulate only | false |

#### Finding Previous WASM Hash

```bash
# From upgrade log (most recent)
cat contracts/deployments/upgrades.json | jq -r '.upgrades[-1].old_wasm_hash'

# List all upgrades for a contract
cat contracts/deployments/upgrades.json | jq '.upgrades[] | select(.contract_id == "CABC123...")'

# From deployment log
cat contracts/deployments/testnet.json | jq -r '.deployments[] | select(.contract_name == "escrow") | .wasm_hash'
```

#### Examples

```bash
# Rollback to previous version
./contracts/scripts/rollback.sh CABC123... 7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b

# Rollback on mainnet (double confirmation required)
./contracts/scripts/rollback.sh CABC123... 7a8b9c0d... -n mainnet

# Force rollback (skip prompts - dangerous!)
./contracts/scripts/rollback.sh CABC123... 7a8b9c0d... --force

# Dry run
./contracts/scripts/rollback.sh CABC123... 7a8b9c0d... --dry-run
```

#### Critical Warning

```
╔══════════════════════════════════════════════════════════════════╗
║                                                                  ║
║                    ⚠️  CRITICAL WARNING ⚠️                         ║
║                                                                  ║
║   ROLLBACK ONLY REVERTS CONTRACT CODE (LOGIC).                   ║
║   CONTRACT STATE (DATA) IS NOT REVERTED.                         ║
║                                                                  ║
║   If the upgraded version modified data structures or storage    ║
║   keys, rolling back may cause DATA INCOMPATIBILITY.             ║
║                                                                  ║
║   MANUAL DATA MIGRATION MAY BE REQUIRED AFTER ROLLBACK.          ║
║                                                                  ║
╚══════════════════════════════════════════════════════════════════╝
```

#### Post-Rollback Checklist

After a rollback, always:

- [ ] Run `verify-deployment.sh` to confirm responsiveness
- [ ] Check if data migration is needed
- [ ] Test critical contract functions manually
- [ ] Review contract state for inconsistencies
- [ ] Notify team members

---

## Network Safety

### Testnet vs Mainnet Comparison

| Aspect | Testnet | Mainnet |
|--------|---------|---------|
| **XLM** | Free (Friendbot) | Real money |
| **Confirmation** | Single prompt | Double confirmation |
| **Risk** | Low (test data) | High (production) |
| **Rollback** | Safe to experiment | Requires caution |

### Safety Features

1. **Confirmation Prompts**
   - Testnet: Single confirmation for destructive operations
   - Mainnet: Double confirmation required

2. **Dry Run Mode**
   - All scripts support `--dry-run` to preview actions
   - No state changes are made

3. **Separate Identities**
   - Use different keys for testnet and mainnet
   - Never reuse testnet keys in production

4. **Registry Logging**
   - All operations are logged with timestamps
   - Enables audit trail and rollback capability

### Mainnet Deployment Checklist

Before deploying to mainnet:

- [ ] Contract tested thoroughly on testnet
- [ ] Code reviewed and/or audited
- [ ] Admin addresses verified correct
- [ ] Sufficient XLM balance for deployment
- [ ] Backup of deployment keys exists
- [ ] Team notified of deployment window
- [ ] Rollback plan documented

---

## Deployment Registry

All deployments and operations are logged in `contracts/deployments/`:

| File | Contents |
|------|----------|
| `testnet.json` | Testnet deployment records |
| `mainnet.json` | Mainnet deployment records |
| `upgrades.json` | All upgrade operations |
| `rollbacks.json` | All rollback operations |

### Registry Format

```json
{
  "deployments": [
    {
      "contract_id": "CABC123...",
      "wasm_hash": "7a8b9c0d...",
      "contract_name": "escrow",
      "network": "testnet",
      "deployer": "grainlify-deployer",
      "deployed_at": "2024-01-15T10:30:00Z",
      "status": "deployed"
    }
  ],
  "metadata": {
    "created": "2024-01-15T10:00:00Z",
    "version": "1.0"
  }
}
```

### Querying the Registry

```bash
# List all deployed contracts
cat contracts/deployments/testnet.json | jq '.deployments[].contract_name'

# Get contract ID by name
cat contracts/deployments/testnet.json | jq -r '.deployments[] | select(.contract_name == "escrow") | .contract_id'

# View upgrade history
cat contracts/deployments/upgrades.json | jq '.upgrades[] | {contract: .contract_name, from: .old_wasm_hash, to: .new_wasm_hash, date: .upgraded_at}'
```

---

## Troubleshooting

### Common Errors

#### "Identity not found"

```
[ERROR] Identity not found: grainlify-deployer
```

**Solution:**
```bash
# Create the identity
stellar keys generate --global grainlify-deployer

# Fund it (testnet)
stellar keys fund grainlify-deployer --network testnet
```

#### "Insufficient balance"

```
[ERROR] Transaction failed: insufficient balance
```

**Solution:**
```bash
# Check balance
stellar keys address grainlify-deployer
# Then check balance on network explorer or:
stellar contract invoke --id <native_token> -- balance --id <your_address>

# Fund more XLM (testnet)
stellar keys fund grainlify-deployer --network testnet
```

#### "Source is not contract admin"

```
[ERROR] Upgrade invocation failed
Possible causes:
  - Source identity is not the contract admin
```

**Solution:**
- Verify you're using the correct identity that deployed/owns the contract
- Check the contract's admin address matches your identity

#### "WASM hash not installed"

```
[ERROR] Rollback failed!
  - WASM hash not installed on network
```

**Solution:**
```bash
# Install the old WASM file first
stellar contract install --wasm old_contract.wasm --network testnet

# Then retry rollback with the returned hash
./contracts/scripts/rollback.sh CABC123... <returned_hash>
```

#### "Network unreachable"

```
[ERROR] Cannot reach network: https://soroban-testnet.stellar.org
```

**Solution:**
- Check your internet connection
- Verify the RPC URL in your config
- Try again (network may be temporarily unavailable)

### Getting Help

```bash
# View script help
./contracts/scripts/deploy.sh --help
./contracts/scripts/upgrade.sh --help
./contracts/scripts/verify-deployment.sh --help
./contracts/scripts/rollback.sh --help

# Enable verbose mode for debugging
./contracts/scripts/deploy.sh contract.wasm --verbose
```

---

## Directory Structure

```
contracts/
├── scripts/
│   ├── config/
│   │   ├── testnet.env           # Testnet configuration
│   │   └── mainnet.env           # Mainnet configuration
│   ├── utils/
│   │   └── common.sh             # Shared utility functions
│   ├── deploy.sh                 # Deploy new contracts
│   ├── upgrade.sh                # Upgrade existing contracts
│   ├── verify-deployment.sh      # Health check contracts
│   ├── rollback.sh               # Revert to previous version
│   ├── upgrade_contract.sh       # Legacy upgrade script
│   ├── demo_upgrade.sh           # Upgrade demonstration
│   └── README.md                 # This file
└── deployments/
    ├── .gitkeep
    ├── testnet.json              # Testnet deployment log
    ├── mainnet.json              # Mainnet deployment log
    ├── upgrades.json             # Upgrade history
    └── rollbacks.json            # Rollback history
```

---

## Contributing

When modifying these scripts:

1. Always source `utils/common.sh` for shared functions
2. Support standard flags (`-n`, `-s`, `--dry-run`, `-v`, `-h`)
3. Add mainnet confirmation prompts for destructive operations
4. Log all operations to the appropriate registry
5. Update this README with new functionality

---

## License

Part of the Grainlify project. See repository root for license information.
