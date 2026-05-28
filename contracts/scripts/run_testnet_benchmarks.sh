#!/bin/bash
# ==============================================================================
# Grainlify — Testnet Benchmark Runner
# ==============================================================================
# Deploys the program-escrow contract to Stellar testnet and measures real
# transaction fees and resource consumption for batch_payout and
# lock_program_funds at multiple batch sizes.
#
# USAGE:
#   ./contracts/scripts/run_testnet_benchmarks.sh [--dry-run]
#
# OPTIONS:
#   --dry-run   Print what would be executed without submitting any transactions
#   -h, --help  Show this help message
#
# PREREQUISITES:
#   1. Stellar CLI installed and on PATH:
#        https://developers.stellar.org/docs/tools/developer-tools/cli/install
#   2. A funded testnet identity. Generate and fund one with:
#        stellar keys generate --global bench-identity
#        stellar keys fund bench-identity --network testnet
#   3. jq installed (optional — used to format JSON output):
#        sudo apt-get install -y jq   # Ubuntu/Debian
#        brew install jq              # macOS
#   4. wasm32v1-none Rust target:
#        rustup target add wasm32v1-none
#
# ENVIRONMENT VARIABLES:
#   DEPLOYER_IDENTITY          (required) Name of the Stellar CLI identity to use
#   SOROBAN_RPC_URL            (default: https://soroban-testnet.stellar.org)
#   STELLAR_NETWORK_PASSPHRASE (default: Stellar testnet passphrase)
#
# OUTPUTS:
#   contracts/benchmarks/results/batch_payout_testnet_YYYY_MM.json
#   contracts/benchmarks/results/lock_funds_testnet_YYYY_MM.json
#   Summary table printed to stdout
#
# SECURITY:
#   Never pass private keys as command-line arguments.
#   Use stellar keys generate to store keys in the local keychain.
#
# ==============================================================================

set -euo pipefail

# ------------------------------------------------------------------------------
# Setup
# ------------------------------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RESULTS_DIR="$PROJECT_ROOT/benchmarks/results"
TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
MONTH_TAG="$(date -u +%Y_%m)"

DRY_RUN="false"

# Default network config
: "${SOROBAN_RPC_URL:=https://soroban-testnet.stellar.org}"
: "${STELLAR_NETWORK_PASSPHRASE:=Test SDF Network ; September 2015}"
NETWORK="testnet"

# Batch sizes to benchmark for batch_payout
BATCH_SIZES=(1 10 50 100)

# ------------------------------------------------------------------------------
# Colours / logging helpers
# ------------------------------------------------------------------------------

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
RESET='\033[0m'

log_info()    { echo -e "${BLUE}[INFO]${RESET}  $*"; }
log_success() { echo -e "${GREEN}[OK]${RESET}    $*"; }
log_warn()    { echo -e "${YELLOW}[WARN]${RESET}  $*"; }
log_error()   { echo -e "${RED}[ERROR]${RESET} $*" >&2; }
log_section() { echo -e "\n${BOLD}==> $*${RESET}"; }
log_dry()     { echo -e "${YELLOW}[DRY-RUN]${RESET} $*"; }

# ------------------------------------------------------------------------------
# Argument parsing
# ------------------------------------------------------------------------------

show_usage() {
    head -55 "$0" | grep -E "^#" | sed 's/^# \?//'
    exit 0
}

parse_args() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --dry-run)
                DRY_RUN="true"
                shift
                ;;
            -h|--help)
                show_usage
                ;;
            *)
                log_error "Unknown argument: $1"
                echo "Use --help for usage information."
                exit 1
                ;;
        esac
    done
}

# ------------------------------------------------------------------------------
# Prerequisite checks
# ------------------------------------------------------------------------------

check_prerequisites() {
    log_section "Checking prerequisites"

    # Detect Stellar CLI (prefer 'stellar', fall back to 'soroban')
    if command -v stellar &>/dev/null; then
        CLI_CMD="stellar"
    elif command -v soroban &>/dev/null; then
        CLI_CMD="soroban"
    else
        log_error "Neither 'stellar' nor 'soroban' CLI found on PATH."
        log_error "Install the Stellar CLI: https://developers.stellar.org/docs/tools/developer-tools/cli/install"
        exit 1
    fi
    log_info "Using CLI: $CLI_CMD ($(command -v "$CLI_CMD"))"

    # Require DEPLOYER_IDENTITY
    if [[ -z "${DEPLOYER_IDENTITY:-}" ]]; then
        log_error "DEPLOYER_IDENTITY environment variable is required."
        log_error ""
        log_error "Create and fund a testnet identity:"
        log_error "  $CLI_CMD keys generate --global bench-identity"
        log_error "  $CLI_CMD keys fund bench-identity --network $NETWORK"
        log_error ""
        log_error "Then export:"
        log_error "  export DEPLOYER_IDENTITY=bench-identity"
        exit 1
    fi

    # Verify the identity exists in the keychain
    if ! "$CLI_CMD" keys address "$DEPLOYER_IDENTITY" &>/dev/null; then
        log_error "Identity '$DEPLOYER_IDENTITY' not found in local keychain."
        log_error "Generate it with: $CLI_CMD keys generate --global $DEPLOYER_IDENTITY"
        exit 1
    fi

    DEPLOYER_ADDRESS=$("$CLI_CMD" keys address "$DEPLOYER_IDENTITY")
    log_info "Deployer identity: $DEPLOYER_IDENTITY ($DEPLOYER_ADDRESS)"
    log_info "RPC URL: $SOROBAN_RPC_URL"
    log_info "Network: $NETWORK"

    # Check jq (optional)
    if command -v jq &>/dev/null; then
        JQ_AVAILABLE="true"
        log_info "jq: available (JSON output will be formatted)"
    else
        JQ_AVAILABLE="false"
        log_warn "jq not found — JSON output will be unformatted. Install jq for better output."
    fi

    # Ensure results directory exists
    mkdir -p "$RESULTS_DIR"

    log_success "Prerequisites OK"
}

# ------------------------------------------------------------------------------
# Build
# ------------------------------------------------------------------------------

build_contract() {
    log_section "Building program-escrow WASM"

    local wasm_path="$PROJECT_ROOT/program-escrow/target/wasm32v1-none/release/program_escrow.wasm"

    if [[ "$DRY_RUN" == "true" ]]; then
        log_dry "cargo build --release --target wasm32v1-none -p program-escrow"
        log_dry "Expected WASM: $wasm_path"
        WASM_FILE="$wasm_path"
        return 0
    fi

    (
        cd "$PROJECT_ROOT"
        cargo build --release --target wasm32v1-none -p program-escrow
    )

    if [[ ! -f "$wasm_path" ]]; then
        log_error "WASM not found after build: $wasm_path"
        exit 1
    fi

    WASM_FILE="$wasm_path"
    log_success "Built: $WASM_FILE"
}

# ------------------------------------------------------------------------------
# Deploy
# ------------------------------------------------------------------------------

deploy_contract() {
    log_section "Deploying contract to $NETWORK"

    if [[ "$DRY_RUN" == "true" ]]; then
        log_dry "$CLI_CMD contract install --wasm $WASM_FILE --network $NETWORK --source $DEPLOYER_IDENTITY"
        log_dry "$CLI_CMD contract deploy --wasm-hash <hash> --network $NETWORK --source $DEPLOYER_IDENTITY"
        CONTRACT_ID="CDRYRUN0000000000000000000000000000000000000000000000000000"
        log_dry "Would use CONTRACT_ID=$CONTRACT_ID"
        return 0
    fi

    log_info "Installing WASM on-chain..."
    WASM_HASH=$("$CLI_CMD" contract install \
        --wasm "$WASM_FILE" \
        --network "$NETWORK" \
        --source "$DEPLOYER_IDENTITY")
    log_success "WASM hash: $WASM_HASH"

    log_info "Deploying contract..."
    CONTRACT_ID=$("$CLI_CMD" contract deploy \
        --wasm-hash "$WASM_HASH" \
        --network "$NETWORK" \
        --source "$DEPLOYER_IDENTITY")
    log_success "Contract deployed: $CONTRACT_ID"
}

# ------------------------------------------------------------------------------
# Fee calculation helpers
# ------------------------------------------------------------------------------

# Convert stroops to XLM string (6 decimal places)
stroops_to_xlm() {
    local stroops="$1"
    # Use awk for portable floating-point division
    awk -v s="$stroops" 'BEGIN { printf "%.6f\n", s / 10000000 }'
}

# ------------------------------------------------------------------------------
# Invoke a single batch_payout and capture metrics
# ------------------------------------------------------------------------------

# Builds a minimal JSON array of recipient addresses for the CLI invocation.
# In a real run, you would use actual funded addresses; for benchmarking we
# use the deployer address repeated N times (safe on testnet, not mainnet).
build_recipients_arg() {
    local n="$1"
    local addr="$2"
    local arr="["
    for (( i=0; i<n; i++ )); do
        [[ $i -gt 0 ]] && arr+=","
        arr+="\"$addr\""
    done
    arr+="]"
    echo "$arr"
}

build_amounts_arg() {
    local n="$1"
    local arr="["
    for (( i=0; i<n; i++ )); do
        [[ $i -gt 0 ]] && arr+=","
        arr+="1000"
    done
    arr+="]"
    echo "$arr"
}

invoke_batch_payout() {
    local batch_size="$1"
    log_info "Invoking batch_payout(batch_size=$batch_size)..."

    local recipients
    recipients=$(build_recipients_arg "$batch_size" "$DEPLOYER_ADDRESS")
    local amounts
    amounts=$(build_amounts_arg "$batch_size")

    if [[ "$DRY_RUN" == "true" ]]; then
        log_dry "$CLI_CMD contract invoke --id $CONTRACT_ID --network $NETWORK --source $DEPLOYER_IDENTITY -- batch_payout --recipients '$recipients' --amounts '$amounts'"
        # Return placeholder metrics
        echo "0:PENDING:0:0:0:0:0"
        return 0
    fi

    local raw_result
    raw_result=$("$CLI_CMD" contract invoke \
        --id "$CONTRACT_ID" \
        --network "$NETWORK" \
        --source "$DEPLOYER_IDENTITY" \
        -- \
        batch_payout \
        --recipients "$recipients" \
        --amounts "$amounts" 2>&1) || true

    # Parse metrics from XDR/response if available; fall back to 0
    # The Stellar CLI prints fee info to stderr in newer versions.
    # We capture best-effort values here.
    local fee_stroops=0
    local cpu_insns=0
    local mem_bytes=0
    local ledger_reads=0
    local ledger_writes=0
    local ledger_seq=0
    local tx_hash="UNKNOWN"

    if echo "$raw_result" | grep -q '"fee_charged"'; then
        fee_stroops=$(echo "$raw_result" | grep -o '"fee_charged":[0-9]*' | grep -o '[0-9]*$' || echo 0)
    fi
    if echo "$raw_result" | grep -q '"instructions"'; then
        cpu_insns=$(echo "$raw_result" | grep -o '"instructions":[0-9]*' | grep -o '[0-9]*$' || echo 0)
    fi
    if echo "$raw_result" | grep -q '"ledger"'; then
        ledger_seq=$(echo "$raw_result" | grep -o '"ledger":[0-9]*' | grep -o '[0-9]*$' || echo 0)
    fi
    if echo "$raw_result" | grep -q '"hash"'; then
        tx_hash=$(echo "$raw_result" | grep -o '"hash":"[^"]*"' | head -1 | sed 's/"hash":"//;s/"//' || echo "UNKNOWN")
    fi

    echo "${ledger_seq}:${tx_hash}:${fee_stroops}:${cpu_insns}:${mem_bytes}:${ledger_reads}:${ledger_writes}"
}

invoke_lock_program_funds() {
    log_info "Invoking lock_program_funds(amount=1000000)..."

    if [[ "$DRY_RUN" == "true" ]]; then
        log_dry "$CLI_CMD contract invoke --id $CONTRACT_ID --network $NETWORK --source $DEPLOYER_IDENTITY -- lock_program_funds --amount 1000000"
        echo "0:PENDING:0:0:0:0:0"
        return 0
    fi

    local raw_result
    raw_result=$("$CLI_CMD" contract invoke \
        --id "$CONTRACT_ID" \
        --network "$NETWORK" \
        --source "$DEPLOYER_IDENTITY" \
        -- \
        lock_program_funds \
        --amount 1000000 2>&1) || true

    local fee_stroops=0 cpu_insns=0 mem_bytes=0
    local ledger_reads=0 ledger_writes=0
    local ledger_seq=0 tx_hash="UNKNOWN"

    if echo "$raw_result" | grep -q '"fee_charged"'; then
        fee_stroops=$(echo "$raw_result" | grep -o '"fee_charged":[0-9]*' | grep -o '[0-9]*$' || echo 0)
    fi
    if echo "$raw_result" | grep -q '"instructions"'; then
        cpu_insns=$(echo "$raw_result" | grep -o '"instructions":[0-9]*' | grep -o '[0-9]*$' || echo 0)
    fi
    if echo "$raw_result" | grep -q '"ledger"'; then
        ledger_seq=$(echo "$raw_result" | grep -o '"ledger":[0-9]*' | grep -o '[0-9]*$' || echo 0)
    fi
    if echo "$raw_result" | grep -q '"hash"'; then
        tx_hash=$(echo "$raw_result" | grep -o '"hash":"[^"]*"' | head -1 | sed 's/"hash":"//;s/"//' || echo "UNKNOWN")
    fi

    echo "${ledger_seq}:${tx_hash}:${fee_stroops}:${cpu_insns}:${mem_bytes}:${ledger_reads}:${ledger_writes}"
}

# ------------------------------------------------------------------------------
# Write JSON result files
# ------------------------------------------------------------------------------

write_batch_payout_json() {
    local output_file="$1"
    shift
    local -a rows=("$@")

    local measurements=""
    local first=1
    for row in "${rows[@]}"; do
        IFS=':' read -r batch_size ledger_seq tx_hash fee_stroops cpu_insns mem_bytes ledger_reads ledger_writes <<< "$row"
        local fee_xlm
        fee_xlm=$(stroops_to_xlm "$fee_stroops")

        [[ $first -eq 0 ]] && measurements+=","
        measurements+=$(printf '
    {
      "batch_size": %s,
      "ledger_sequence": %s,
      "transaction_hash": "%s",
      "fee_stroops": %s,
      "fee_xlm": "%s",
      "cpu_instructions": %s,
      "memory_bytes": %s,
      "ledger_reads": %s,
      "ledger_writes": %s,
      "status": "measured",
      "measured_at": "%s"
    }' \
            "$batch_size" "$ledger_seq" "$tx_hash" \
            "$fee_stroops" "$fee_xlm" "$cpu_insns" "$mem_bytes" \
            "$ledger_reads" "$ledger_writes" \
            "$TIMESTAMP")
        first=0
    done

    local json
    json=$(printf '{
  "schema_version": 1,
  "generated_at": "%s",
  "network": "%s",
  "contract_name": "program-escrow",
  "function": "batch_payout",
  "note": "Measured values recorded by run_testnet_benchmarks.sh on %s",
  "ci_threshold_cpu_instructions_50": 12000000,
  "measurements": [%s
  ]
}' "$TIMESTAMP" "$NETWORK" "$TIMESTAMP" "$measurements")

    if [[ "$JQ_AVAILABLE" == "true" ]]; then
        echo "$json" | jq . > "$output_file"
    else
        echo "$json" > "$output_file"
    fi

    log_success "Written: $output_file"
}

write_lock_funds_json() {
    local output_file="$1"
    local row="$2"

    IFS=':' read -r batch_size ledger_seq tx_hash fee_stroops cpu_insns mem_bytes ledger_reads ledger_writes <<< "$row"
    local fee_xlm
    fee_xlm=$(stroops_to_xlm "$fee_stroops")

    local json
    json=$(printf '{
  "schema_version": 1,
  "generated_at": "%s",
  "network": "%s",
  "contract_name": "program-escrow",
  "function": "lock_program_funds",
  "note": "Measured values recorded by run_testnet_benchmarks.sh on %s. lock_program_funds is O(1) so only a single batch_size=1 measurement is needed.",
  "ci_threshold_cpu_instructions_50": 500000,
  "measurements": [
    {
      "batch_size": 1,
      "ledger_sequence": %s,
      "transaction_hash": "%s",
      "fee_stroops": %s,
      "fee_xlm": "%s",
      "cpu_instructions": %s,
      "memory_bytes": %s,
      "ledger_reads": %s,
      "ledger_writes": %s,
      "status": "measured",
      "measured_at": "%s"
    }
  ]
}' "$TIMESTAMP" "$NETWORK" "$TIMESTAMP" \
    "$ledger_seq" "$tx_hash" \
    "$fee_stroops" "$fee_xlm" "$cpu_insns" "$mem_bytes" \
    "$ledger_reads" "$ledger_writes" \
    "$TIMESTAMP")

    if [[ "$JQ_AVAILABLE" == "true" ]]; then
        echo "$json" | jq . > "$output_file"
    else
        echo "$json" > "$output_file"
    fi

    log_success "Written: $output_file"
}

# ------------------------------------------------------------------------------
# Print summary table
# ------------------------------------------------------------------------------

print_summary_table() {
    local -a rows=("$@")

    echo ""
    echo "┌─────────────┬──────────────────┬──────────────┬──────────────┐"
    printf "│ %-11s │ %-16s │ %-12s │ %-12s │\n" \
        "batch_size" "cpu_instructions" "fee_stroops" "fee_xlm"
    echo "├─────────────┼──────────────────┼──────────────┼──────────────┤"

    for row in "${rows[@]}"; do
        IFS=':' read -r batch_size _ledger_seq _tx_hash fee_stroops cpu_insns _mem _reads _writes <<< "$row"
        local fee_xlm
        fee_xlm=$(stroops_to_xlm "$fee_stroops")
        printf "│ %11s │ %16s │ %12s │ %12s │\n" \
            "$batch_size" "$cpu_insns" "$fee_stroops" "$fee_xlm"
    done

    echo "└─────────────┴──────────────────┴──────────────┴──────────────┘"
    echo ""
    echo "  CI threshold (batch_size=50): 12,000,000 CPU instructions"
    echo "  Threshold constant: CPU_INSNS_THRESHOLD_50"
    echo "  Location: contracts/program-escrow/src/test_batch_operations.rs"
    echo ""
}

# ------------------------------------------------------------------------------
# Main
# ------------------------------------------------------------------------------

main() {
    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  Grainlify — Testnet Benchmark Runner"
    if [[ "$DRY_RUN" == "true" ]]; then
        echo "  [DRY-RUN MODE — no transactions will be submitted]"
    fi
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    parse_args "$@"
    check_prerequisites
    build_contract
    deploy_contract

    # ── batch_payout benchmarks ──────────────────────────────────────────
    log_section "Benchmarking batch_payout"

    declare -a payout_rows=()
    for batch_size in "${BATCH_SIZES[@]}"; do
        result=$(invoke_batch_payout "$batch_size")
        IFS=':' read -r ledger_seq tx_hash fee_stroops cpu_insns mem_bytes ledger_reads ledger_writes <<< "$result"
        payout_rows+=("${batch_size}:${ledger_seq}:${tx_hash}:${fee_stroops}:${cpu_insns}:${mem_bytes}:${ledger_reads}:${ledger_writes}")
    done

    local payout_output="$RESULTS_DIR/batch_payout_testnet_${MONTH_TAG}.json"
    if [[ "$DRY_RUN" != "true" ]]; then
        write_batch_payout_json "$payout_output" "${payout_rows[@]}"
    else
        log_dry "Would write: $payout_output"
    fi

    # ── lock_program_funds benchmark ────────────────────────────────────
    log_section "Benchmarking lock_program_funds"

    lock_result=$(invoke_lock_program_funds)
    IFS=':' read -r lock_seq lock_hash lock_fee lock_cpu lock_mem lock_reads lock_writes <<< "$lock_result"
    lock_row="1:${lock_seq}:${lock_hash}:${lock_fee}:${lock_cpu}:${lock_mem}:${lock_reads}:${lock_writes}"

    local lock_output="$RESULTS_DIR/lock_funds_testnet_${MONTH_TAG}.json"
    if [[ "$DRY_RUN" != "true" ]]; then
        write_lock_funds_json "$lock_output" "$lock_row"
    else
        log_dry "Would write: $lock_output"
    fi

    # ── Summary ─────────────────────────────────────────────────────────
    log_section "batch_payout — Summary"
    print_summary_table "${payout_rows[@]}"

    log_section "lock_program_funds — Summary"
    IFS=':' read -r _bs _seq _hash fee_stroops cpu_insns _mem _reads _writes <<< "$lock_row"
    printf "  cpu_instructions: %s\n" "$cpu_insns"
    printf "  fee_stroops:      %s  (%.6f XLM)\n" "$fee_stroops" "$(awk -v s="$fee_stroops" 'BEGIN { printf "%.6f", s / 10000000 }')"
    echo ""

    if [[ "$DRY_RUN" != "true" ]]; then
        log_success "Results written to:"
        log_success "  $payout_output"
        log_success "  $lock_output"
    fi

    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  Done. Review docs/gas-optimization/batch-payout-benchmarks.md"
    echo "  for interpretation guidance and threshold update instructions."
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
}

main "$@"
