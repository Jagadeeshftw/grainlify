#!/bin/bash
set -e

# Configuration
NETWORK="testnet"
CONTRACT_DIR="contracts/grainlify-core"
SRC_FILE="$CONTRACT_DIR/src/lib.rs"
SOURCE="demo_user"

echo "=== Grainlify Contract Upgrade Demo ==="

# 1. Build V1
echo "[1/9] Building V1..."
cargo build --target wasm32-unknown-unknown --release --manifest-path "$CONTRACT_DIR/Cargo.toml"
WASM_V1="$CONTRACT_DIR/target/wasm32-unknown-unknown/release/grainlify_core.wasm"

# 2. Setup Identity
echo "[2/9] Setting up Identity..."
soroban keys generate "$SOURCE" --network "$NETWORK" --overwrite || true
soroban keys fund "$SOURCE" --network "$NETWORK"

# 3. Deploy V1
echo "[3/9] Deploying V1..."
ID=$(soroban contract deploy --wasm "$WASM_V1" --source "$SOURCE" --network "$NETWORK")
echo "Contract Deployed: $ID"

# 4. Initialize V1
echo "[4/9] Initializing V1..."
ADMIN_ADDR=$(soroban keys address "$SOURCE")
soroban contract invoke --id "$ID" --source "$SOURCE" --network "$NETWORK" --send=yes -- init --admin "$ADMIN_ADDR"

# 5. Check Version
echo "[5/9] Checking Version (Expect: 1)..."
VER=$(soroban contract invoke --id "$ID" --source "$SOURCE" --network "$NETWORK" -- get_version)
echo "Current Version: $VER"

if [[ "$VER" != *"1"* ]]; then
    echo "Error: Expected version 1, got $VER"
    exit 1
fi

# 6. Modify Code to V2
echo "[6/9] Modifying code to Version 2..."
# Change get_version to return hardcoded 2
sed -i 's/env.storage().instance().get(&DataKey::Version).unwrap_or(0)/2/' "$SRC_FILE"

# 7. Build V2
echo "[7/9] Building V2..."
cargo build --target wasm32-unknown-unknown --release --manifest-path "$CONTRACT_DIR/Cargo.toml"
WASM_V2="$CONTRACT_DIR/target/wasm32-unknown-unknown/release/grainlify_core.wasm"

# 8. Upgrade
echo "[8/9] Upgrading Contract..."
./scripts/upgrade_contract.sh "$ID" "$WASM_V2" "$NETWORK" "$SOURCE"

# 9. Check Version
echo "[9/9] Checking Version (Expect: 2)..."
VER=$(soroban contract invoke --id "$ID" --source "$SOURCE" --network "$NETWORK" -- get_version)
echo "Current Version: $VER"

if [[ "$VER" != *"2"* ]]; then
    echo "Error: Expected version 2, got $VER"
    # Cleanup before exit
    sed -i 's/2/env.storage().instance().get(\&DataKey::Version).unwrap_or(0)/' "$SRC_FILE"
    exit 1
fi

# Cleanup
echo "=== Demo Successful ==="
echo "Restoring source code..."
sed -i 's/2/env.storage().instance().get(\&DataKey::Version).unwrap_or(0)/' "$SRC_FILE"
