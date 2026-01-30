#!/bin/bash
set -e

# Usage: ./upgrade_contract.sh <CONTRACT_ID> <WASM_FILE> <NETWORK> <SOURCE_IDENTITY>
# Example: ./upgrade_contract.sh C... contracts/grainlify-core/target/wasm32-unknown-unknown/release/grainlify_core.wasm testnet demo_user

CONTRACT_ID=$1
WASM_FILE=$2
NETWORK=${3:-testnet}
SOURCE=${4:-default}

if [ -z "$CONTRACT_ID" ] || [ -z "$WASM_FILE" ]; then
    echo "Usage: $0 <CONTRACT_ID> <WASM_FILE> [NETWORK] [SOURCE_IDENTITY]"
    exit 1
fi

echo "Uploading WASM..."
# Use 'upload' instead of 'install' as per deprecation warning
WASM_HASH=$(soroban contract upload --wasm "$WASM_FILE" --network "$NETWORK" --source "$SOURCE")
echo "WASM Hash: $WASM_HASH"

echo "Upgrading contract..."
soroban contract invoke \
    --id "$CONTRACT_ID" \
    --network "$NETWORK" \
    --source "$SOURCE" \
    --send=yes \
    -- \
    upgrade \
    --new_wasm_hash "$WASM_HASH"

echo "Upgrade complete."
