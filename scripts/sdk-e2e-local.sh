#!/usr/bin/env bash
# Reproduce the SDK examples end-to-end job locally.
#
# Starts an ephemeral Stellar quickstart container, waits for RPC and friendbot,
# builds and deploys program_escrow.wasm, funds a throwaway key, writes the
# GRAINLIFY_SDK_E2E_* environment to a private file, and runs the e2e test.
#
# By default the container is removed when the script exits. Use
# KEEP_QUICKSTART=1 (or --keep-running) to leave it running and keep the
# environment file for subsequent test runs.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

QUICKSTART_IMAGE="${QUICKSTART_IMAGE:-stellar/quickstart:v695-b1655.1-latest}"
QUICKSTART_PORT="${QUICKSTART_PORT:-8000}"
CONTAINER_NAME="${QUICKSTART_CONTAINER_NAME:-grainlify-sdk-e2e-$$}"
KEY_NAME="${GRAINLIFY_SDK_E2E_KEY_NAME:-grainlify-e2e-$$}"
TIMEOUT="${GRAINLIFY_QUICKSTART_TIMEOUT:-300}"
KEEP_QUICKSTART="${KEEP_QUICKSTART:-0}"
RUN_E2E_TEST="${RUN_E2E_TEST:-1}"
ENV_FILE="${GRAINLIFY_SDK_E2E_ENV_FILE:-${TMPDIR:-/tmp}/grainlify-sdk-e2e.env}"

RPC_URL="http://localhost:${QUICKSTART_PORT}/rpc"
FRIENDBOT_URL="http://localhost:${QUICKSTART_PORT}/friendbot"
NETWORK_PASSPHRASE="Standalone Network ; February 2017"
WASM_PATH="$ROOT_DIR/contracts/program-escrow/target/wasm32-unknown-unknown/release/program_escrow.wasm"

usage() {
  cat <<'EOF'
Usage: scripts/sdk-e2e-local.sh [options]

Start a local Stellar quickstart, deploy ProgramEscrow, and run the SDK e2e test.

Options:
  --keep-running  Leave the quickstart container running after the test
  --no-test       Deploy only; do not run the Jest e2e test
  -h, --help      Show this help

Environment variables:
  QUICKSTART_IMAGE              Image to run (default: stellar/quickstart:v695-b1655.1-latest)
  QUICKSTART_PORT               Host port to expose (default: 8000)
  QUICKSTART_CONTAINER_NAME     Container name (default: grainlify-sdk-e2e-<pid>)
  GRAINLIFY_QUICKSTART_TIMEOUT  Readiness timeout in seconds (default: 300)
  KEEP_QUICKSTART=1             Same as --keep-running
  RUN_E2E_TEST=0                Same as --no-test
  GRAINLIFY_SDK_E2E_ENV_FILE    Where to write the generated env file
EOF
}

die() {
  printf 'ERROR: %s\n' "$*" >&2
  exit 1
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --keep-running)
      KEEP_QUICKSTART=1
      shift
      ;;
    --no-test)
      RUN_E2E_TEST=0
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      die "unknown argument: $1 (use --help)"
      ;;
  esac
done

for command_name in docker cargo curl npm; do
  command -v "$command_name" >/dev/null 2>&1 ||
    die "$command_name is required but was not found on PATH"
done

if ! command -v stellar >/dev/null 2>&1; then
  die "the Stellar CLI is required (install with: cargo install --locked stellar-cli --version 23.0.1)"
fi

docker info >/dev/null 2>&1 ||
  die "the Docker daemon is not reachable; start Docker and try again"

if command -v rustup >/dev/null 2>&1; then
  if ! rustup target list --installed | grep -qx wasm32-unknown-unknown; then
    die "the Rust wasm target is missing (run: rustup target add wasm32-unknown-unknown)"
  fi
fi

if [ ! -d "$ROOT_DIR/contracts/sdk/node_modules" ]; then
  die "SDK dependencies are missing (run: cd contracts/sdk && npm ci)"
fi

printf 'Starting %s as %s...\n' "$QUICKSTART_IMAGE" "$CONTAINER_NAME"
docker run --rm --name "$CONTAINER_NAME" -d \
  -p "${QUICKSTART_PORT}:8000" \
  "$QUICKSTART_IMAGE" --local --enable rpc,core,horizon >/dev/null

cleanup() {
  if [ "$KEEP_QUICKSTART" = "1" ]; then
    printf '\nQuickstart container %s is still running.\n' "$CONTAINER_NAME" >&2
    printf 'Generated environment: %s\n' "$ENV_FILE" >&2
    printf 'Stop the container with: docker stop %s\n' "$CONTAINER_NAME" >&2
  else
    docker stop "$CONTAINER_NAME" >/dev/null 2>&1 || true
    rm -f "$ENV_FILE"
  fi
}
trap cleanup EXIT

bash "$ROOT_DIR/scripts/wait-for-quickstart.sh" \
  --rpc-url "$RPC_URL" \
  --friendbot-url "$FRIENDBOT_URL" \
  --timeout "$TIMEOUT" \
  --container "$CONTAINER_NAME"

printf 'Building program_escrow.wasm...\n'
cargo build \
  --manifest-path "$ROOT_DIR/contracts/program-escrow/Cargo.toml" \
  --target wasm32-unknown-unknown \
  --release
[ -f "$WASM_PATH" ] ||
  die "expected wasm artifact was not produced at $WASM_PATH"

printf 'Generating and funding key %s...\n' "$KEY_NAME"
stellar keys generate "$KEY_NAME"
secret="$(stellar keys secret "$KEY_NAME")"
public_key="$(stellar keys public-key "$KEY_NAME")"
[ -n "$secret" ] || die "failed to read the generated secret key"
[ -n "$public_key" ] || die "failed to read the generated public key"

funded=0
for _ in $(seq 1 15); do
  if curl -fsS "${FRIENDBOT_URL}?addr=${public_key}" >/dev/null 2>&1; then
    funded=1
    break
  fi
  sleep 2
done
[ "$funded" -eq 1 ] || die "friendbot did not fund $public_key"

printf 'Deploying ProgramEscrow...\n'
if ! deploy_output="$(
  stellar contract deploy \
    --wasm "$WASM_PATH" \
    --source-account "$KEY_NAME" \
    --rpc-url "$RPC_URL" \
    --network-passphrase "$NETWORK_PASSPHRASE" 2>&1
)"; then
  printf '%s\n' "$deploy_output" >&2
  die "contract deployment failed"
fi
printf '%s\n' "$deploy_output"

contract_id="$(
  printf '%s\n' "$deploy_output" |
    grep -Eo 'C[A-Z2-7]{55}' |
    tail -n 1 || true
)"
[ -n "$contract_id" ] || die "could not read a contract id from the deploy output"

token_address="$(
  stellar contract id asset \
    --asset native \
    --rpc-url "$RPC_URL" \
    --network-passphrase "$NETWORK_PASSPHRASE"
)"
[ -n "$token_address" ] || die "could not resolve the native asset contract address"

umask 077
{
  printf 'export GRAINLIFY_SDK_E2E_CONTRACT_ID=%q\n' "$contract_id"
  printf 'export GRAINLIFY_SDK_E2E_TOKEN_ADDRESS=%q\n' "$token_address"
  printf 'export GRAINLIFY_SDK_E2E_RPC_URL=%q\n' "$RPC_URL"
  printf 'export GRAINLIFY_SDK_E2E_NETWORK_PASSPHRASE=%q\n' "$NETWORK_PASSPHRASE"
  printf 'export GRAINLIFY_SDK_E2E_SECRET=%q\n' "$secret"
  printf 'export GRAINLIFY_SDK_E2E_REQUIRED=1\n'
} >"$ENV_FILE"

printf 'Contract: %s\n' "$contract_id"
printf 'Token:    %s\n' "$token_address"
printf 'Environment written to %s (mode 600).\n' "$ENV_FILE"

if [ "$RUN_E2E_TEST" = "1" ]; then
  printf 'Running the SDK e2e test...\n'
  (
    cd "$ROOT_DIR/contracts/sdk"
    # shellcheck source=/dev/null
    source "$ENV_FILE"
    npm test -- --runInBand src/__tests__/examples.e2e.test.ts
  )
else
  printf 'Skipping the e2e test (RUN_E2E_TEST=0).\n'
fi
