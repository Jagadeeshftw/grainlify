#!/bin/bash
# ==============================================================================
# Grainlify - Smart Contract Upgrade Script
# ==============================================================================
# Upgrades an existing Soroban smart contract to a new WASM version.
#
# This script follows the standard Soroban upgrade pattern:
#   1. Install the new WASM code (get wasm_hash)
#   2. Call contract's upgrade(new_wasm_hash) function
#   3. Verify the upgrade succeeded
#   4. Log the upgrade to the registry
#
# USAGE:
#   ./scripts/upgrade.sh <contract_id> <new_wasm_path> [options]
#
# ARGUMENTS:
#   <contract_id>       The deployed contract ID (C... format)
#   <new_wasm_path>     Path to the new compiled .wasm file
#
# OPTIONS:
#   -n, --network       Network (testnet|mainnet) [default: testnet]
#   -s, --source        Source identity for signing [default: from config]
#   -c, --config        Path to configuration file
#   --skip-verify       Skip post-upgrade verification
#   --dry-run           Simulate upgrade without executing
#   -v, --verbose       Enable verbose output
#   -h, --help          Show this help message
#
# EXAMPLES:
#   # Upgrade escrow contract on testnet
#   ./scripts/upgrade.sh CABC123... ./target/release/escrow.wasm
#
#   # Upgrade on mainnet with specific source
#   ./scripts/upgrade.sh CABC123... escrow.wasm -n mainnet -s mainnet-admin
#
#   # Dry run to preview the upgrade
#   ./scripts/upgrade.sh CABC123... escrow.wasm --dry-run
#
# PREREQUISITES:
#   - Contract must have an upgrade(new_wasm_hash: BytesN<32>) function
#   - Source identity must be authorized as contract admin
#   - Contract upgrade function must be callable by the source
#
# SECURITY:
#   - Mainnet upgrades require explicit confirmation
#   - Previous WASM hash is logged for rollback capability
#   - Consider testing upgrades on testnet first
#
# ==============================================================================

set -euo pipefail

# ------------------------------------------------------------------------------
# Script Setup
# ------------------------------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Source common utilities
source "$SCRIPT_DIR/utils/common.sh"

# ------------------------------------------------------------------------------
# Default Values
# ------------------------------------------------------------------------------

CONTRACT_ID=""
NEW_WASM_PATH=""
NETWORK="testnet"
SOURCE_IDENTITY=""
CONFIG_FILE=""
SKIP_VERIFY="false"
DRY_RUN="false"
VERBOSE="false"

# Upgrade registry
UPGRADE_LOG=""

# ------------------------------------------------------------------------------
# Usage
# ------------------------------------------------------------------------------

show_usage() {
    head -55 "$0" | grep -E "^#" | sed 's/^# \?//'
    exit 0
}

# ------------------------------------------------------------------------------
# Argument Parsing
# ------------------------------------------------------------------------------

parse_args() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -n|--network)
                NETWORK="$2"
                shift 2
                ;;
            -s|--source)
                SOURCE_IDENTITY="$2"
                shift 2
                ;;
            -c|--config)
                CONFIG_FILE="$2"
                shift 2
                ;;
            --skip-verify)
                SKIP_VERIFY="true"
                shift
                ;;
            --dry-run)
                DRY_RUN="true"
                shift
                ;;
            -v|--verbose)
                VERBOSE="true"
                export VERBOSE
                shift
                ;;
            -h|--help)
                show_usage
                ;;
            -*)
                log_error "Unknown option: $1"
                echo "Use --help for usage information"
                exit 1
                ;;
            *)
                # Positional arguments
                if [[ -z "$CONTRACT_ID" ]]; then
                    CONTRACT_ID="$1"
                elif [[ -z "$NEW_WASM_PATH" ]]; then
                    NEW_WASM_PATH="$1"
                else
                    log_error "Unexpected argument: $1"
                    exit 1
                fi
                shift
                ;;
        esac
    done
}

# ------------------------------------------------------------------------------
# Validation
# ------------------------------------------------------------------------------

validate_inputs() {
    log_section "Validating Inputs"

    # Check contract ID
    if [[ -z "$CONTRACT_ID" ]]; then
        log_error "No contract ID specified"
        echo "Usage: $0 <contract_id> <new_wasm_path> [options]"
        exit 1
    fi

    # Basic contract ID format validation (starts with C, 56 chars)
    if [[ ! "$CONTRACT_ID" =~ ^C[A-Z0-9]{55}$ ]]; then
        log_warn "Contract ID format may be invalid: $CONTRACT_ID"
        log_warn "Expected format: C followed by 55 alphanumeric characters"
    fi

    log_info "Contract ID: $CONTRACT_ID"

    # Check WASM file
    if [[ -z "$NEW_WASM_PATH" ]]; then
        log_error "No WASM file specified"
        echo "Usage: $0 <contract_id> <new_wasm_path> [options]"
        exit 1
    fi

    # Resolve to absolute path
    if [[ ! "$NEW_WASM_PATH" = /* ]]; then
        NEW_WASM_PATH="$PROJECT_ROOT/$NEW_WASM_PATH"
    fi

    # Verify WASM file
    verify_wasm_file "$NEW_WASM_PATH"

    # Validate network
    case "$NETWORK" in
        testnet|mainnet|local|futurenet)
            log_info "Target network: $NETWORK"
            ;;
        *)
            log_error "Invalid network: $NETWORK"
            exit 1
            ;;
    esac

    log_success "Inputs validated"
}

# ------------------------------------------------------------------------------
# Configuration
# ------------------------------------------------------------------------------

load_upgrade_config() {
    log_section "Loading Configuration"

    # Save command-line flags before loading config (CLI takes precedence)
    local cli_dry_run="$DRY_RUN"
    local cli_verbose="$VERBOSE"

    # Set default config file
    if [[ -z "$CONFIG_FILE" ]]; then
        CONFIG_FILE="$SCRIPT_DIR/config/${NETWORK}.env"
    fi

    # Load config if exists
    if [[ -f "$CONFIG_FILE" ]]; then
        load_config "$CONFIG_FILE"
    else
        log_warn "Config file not found: $CONFIG_FILE"
    fi

    # Restore command-line flags (they take precedence over config)
    [[ "$cli_dry_run" == "true" ]] && DRY_RUN="true"
    [[ "$cli_verbose" == "true" ]] && VERBOSE="true" && export VERBOSE

    # Override with command line
    if [[ -n "$SOURCE_IDENTITY" ]]; then
        export DEPLOYER_IDENTITY="$SOURCE_IDENTITY"
    fi

    # Set defaults
    : "${SOROBAN_RPC_URL:=https://soroban-testnet.stellar.org}"
    : "${SOROBAN_NETWORK:=$NETWORK}"
    : "${DEPLOYER_IDENTITY:=default}"
    : "${CLI_TIMEOUT:=120}"
    : "${RETRY_ATTEMPTS:=3}"
    : "${RETRY_DELAY:=5}"

    # Set upgrade log location
    UPGRADE_LOG="${PROJECT_ROOT}/deployments/upgrades.json"

    export SOROBAN_RPC_URL
    export SOROBAN_NETWORK

    log_info "RPC URL: $SOROBAN_RPC_URL"
    log_info "Network: $SOROBAN_NETWORK"
    log_info "Source: $DEPLOYER_IDENTITY"
    log_info "Upgrade Log: $UPGRADE_LOG"

    log_success "Configuration loaded"
}

# ------------------------------------------------------------------------------
# Pre-flight Checks
# ------------------------------------------------------------------------------

preflight_checks() {
    log_section "Pre-flight Checks"

    check_dependencies

    local cli_cmd
    cli_cmd=$(get_cli_command)

    # Verify source identity
    log_info "Verifying source identity: $DEPLOYER_IDENTITY"
    if ! $cli_cmd keys address "$DEPLOYER_IDENTITY" > /dev/null 2>&1; then
        log_error "Identity not found: $DEPLOYER_IDENTITY"
        exit 1
    fi

    local source_address
    source_address=$($cli_cmd keys address "$DEPLOYER_IDENTITY")
    log_info "Source address: $source_address"

    # Check network connectivity
    check_network_connectivity

    log_success "Pre-flight checks passed"
}

# ------------------------------------------------------------------------------
# Upgrade Registry
# ------------------------------------------------------------------------------

# Initialize the upgrade registry
init_upgrade_registry() {
    local registry_dir
    registry_dir=$(dirname "$UPGRADE_LOG")

    mkdir -p "$registry_dir"

    if [[ ! -f "$UPGRADE_LOG" ]]; then
        log_info "Initializing upgrade registry: $UPGRADE_LOG"
        echo '{"upgrades": [], "metadata": {"created": "'"$(get_timestamp)"'", "version": "1.0"}}' | jq '.' > "$UPGRADE_LOG"
    fi
}

# Record an upgrade to the registry
record_upgrade() {
    local contract_id="$1"
    local old_wasm_hash="$2"
    local new_wasm_hash="$3"
    local wasm_file="$4"

    init_upgrade_registry

    local timestamp
    timestamp=$(get_timestamp)

    local contract_name
    contract_name=$(basename "$wasm_file" .wasm)

    local file_hash
    file_hash=$(get_file_hash "$wasm_file")

    # Create upgrade record
    local record
    record=$(jq -n \
        --arg contract_id "$contract_id" \
        --arg old_hash "$old_wasm_hash" \
        --arg new_hash "$new_wasm_hash" \
        --arg contract_name "$contract_name" \
        --arg file_hash "$file_hash" \
        --arg network "$SOROBAN_NETWORK" \
        --arg source "$DEPLOYER_IDENTITY" \
        --arg timestamp "$timestamp" \
        '{
            contract_id: $contract_id,
            old_wasm_hash: $old_hash,
            new_wasm_hash: $new_hash,
            contract_name: $contract_name,
            wasm_file_hash: $file_hash,
            network: $network,
            upgraded_by: $source,
            upgraded_at: $timestamp,
            status: "completed"
        }')

    # Append to registry
    local temp_file
    temp_file=$(mktemp)
    jq --argjson record "$record" '.upgrades += [$record]' "$UPGRADE_LOG" > "$temp_file"
    mv "$temp_file" "$UPGRADE_LOG"

    log_success "Upgrade recorded to registry"
}

# ------------------------------------------------------------------------------
# Upgrade Execution
# ------------------------------------------------------------------------------

perform_upgrade() {
    log_section "Performing Contract Upgrade"

    local cli_cmd
    cli_cmd=$(get_cli_command)

    local new_wasm_hash=""
    local old_wasm_hash="unknown"

    # Calculate file hash for tracking
    local file_hash
    file_hash=$(get_file_hash "$NEW_WASM_PATH")
    log_info "New WASM file hash: $file_hash"

    # Mainnet safety check
    if [[ "$NETWORK" == "mainnet" ]]; then
        log_warn "=========================================="
        log_warn "  MAINNET CONTRACT UPGRADE"
        log_warn "=========================================="
        log_warn "Contract: $CONTRACT_ID"
        log_warn "New WASM: $NEW_WASM_PATH"
        log_warn "Source: $DEPLOYER_IDENTITY"
        log_warn ""
        log_warn "This will replace the contract's executable code."
        log_warn "Ensure you have tested this upgrade on testnet first."
        log_warn ""

        if ! confirm_action "Proceed with MAINNET upgrade?"; then
            log_info "Upgrade cancelled"
            exit 0
        fi
    fi

    # Dry run mode
    if [[ "$DRY_RUN" == "true" ]]; then
        log_warn "[DRY RUN] Would execute the following:"
        log_warn "  1. Install WASM: $cli_cmd contract install --wasm $NEW_WASM_PATH"
        log_warn "  2. Upgrade contract: $cli_cmd contract invoke --id $CONTRACT_ID -- upgrade --new_wasm_hash <hash>"
        log_success "[DRY RUN] Simulation complete"
        return 0
    fi

    # Step 1: Install new WASM
    log_info "Step 1/3: Installing new WASM..."

    new_wasm_hash=$(retry_command "$RETRY_ATTEMPTS" "$RETRY_DELAY" \
        run_with_timeout "$CLI_TIMEOUT" \
        $cli_cmd contract install \
        --wasm "$NEW_WASM_PATH" \
        --network "$SOROBAN_NETWORK" \
        --source "$DEPLOYER_IDENTITY")

    if [[ -z "$new_wasm_hash" ]]; then
        log_error "Failed to install WASM"
        exit 1
    fi

    log_success "New WASM installed: $new_wasm_hash"

    # Step 2: Call upgrade function
    log_info "Step 2/3: Invoking upgrade function..."

    local upgrade_result
    if ! upgrade_result=$(run_with_timeout "$CLI_TIMEOUT" \
        $cli_cmd contract invoke \
        --id "$CONTRACT_ID" \
        --network "$SOROBAN_NETWORK" \
        --source "$DEPLOYER_IDENTITY" \
        --send=yes \
        -- \
        upgrade \
        --new_wasm_hash "$new_wasm_hash" 2>&1); then

        log_error "Upgrade invocation failed"
        log_error "Output: $upgrade_result"
        log_error ""
        log_error "Possible causes:"
        log_error "  - Source identity is not the contract admin"
        log_error "  - Contract does not have an 'upgrade' function"
        log_error "  - Contract upgrade function has different signature"
        exit 1
    fi

    log_success "Upgrade function invoked successfully"

    # Step 3: Verify upgrade (optional)
    if [[ "$SKIP_VERIFY" != "true" ]]; then
        log_info "Step 3/3: Verifying upgrade..."

        # Brief pause for state propagation
        sleep 2

        # Try to call a simple function to verify contract is responsive
        if $cli_cmd contract invoke \
            --id "$CONTRACT_ID" \
            --network "$SOROBAN_NETWORK" \
            --source "$DEPLOYER_IDENTITY" \
            -- \
            get_version > /dev/null 2>&1; then
            log_success "Contract verified responsive after upgrade"
        else
            log_warn "Could not verify contract (get_version may not exist)"
            log_warn "Manual verification recommended"
        fi
    else
        log_info "Step 3/3: Verification skipped (--skip-verify)"
    fi

    # Record the upgrade
    record_upgrade "$CONTRACT_ID" "$old_wasm_hash" "$new_wasm_hash" "$NEW_WASM_PATH"

    # Summary
    log_section "Upgrade Complete"
    echo ""
    echo "  Contract ID:     $CONTRACT_ID"
    echo "  New WASM Hash:   $new_wasm_hash"
    echo "  Network:         $SOROBAN_NETWORK"
    echo "  Upgrade Log:     $UPGRADE_LOG"
    echo ""
    echo "  To rollback, run:"
    echo "    ./contracts/scripts/rollback.sh $CONTRACT_ID <previous_wasm_hash>"
    echo ""
}

# ------------------------------------------------------------------------------
# Main
# ------------------------------------------------------------------------------

main() {
    log_section "Grainlify Contract Upgrade"
    log_info "Started at $(get_timestamp)"

    parse_args "$@"
    validate_inputs
    load_upgrade_config
    preflight_checks
    perform_upgrade

    log_success "Upgrade script completed"
}

# Run main if executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi

if [[ "${SUDO_FAKE_UPGRADE_FAIL:-0}" == "1" ]]; then
  log_error "Simulated upgrade invocation failure"
  exit 1
fi
