#!/bin/bash
# ==============================================================================
# Grainlify - Contract Rollback Script
# ==============================================================================
# Rolls back a Soroban smart contract to a previous WASM version.
#
# This script performs a rollback by calling the contract's upgrade function
# with a previously known WASM hash. It is essentially an "upgrade" to an
# older version.
#
# !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
# !!                                                                         !!
# !!                       *** CRITICAL WARNING ***                          !!
# !!                                                                         !!
# !!   ROLLBACK ONLY REVERTS THE CONTRACT CODE (LOGIC).                      !!
# !!   IT DOES NOT REVERT CONTRACT STATE (DATA).                             !!
# !!                                                                         !!
# !!   If the new version modified data structures, rolling back the code    !!
# !!   may cause data incompatibility issues. Manual data migration may      !!
# !!   be required after rollback.                                           !!
# !!                                                                         !!
# !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
#
# USAGE:
#   ./scripts/rollback.sh <contract_id> <previous_wasm_hash> [options]
#
# ARGUMENTS:
#   <contract_id>         The deployed contract ID (C... format)
#   <previous_wasm_hash>  The WASM hash to rollback to (from upgrade log)
#
# OPTIONS:
#   -n, --network         Network (testnet|mainnet) [default: testnet]
#   -s, --source          Source identity for signing [default: from config]
#   -c, --config          Path to configuration file
#   --force               Skip confirmation prompts (dangerous!)
#   --dry-run             Simulate rollback without executing
#   -v, --verbose         Enable verbose output
#   -h, --help            Show this help message
#
# EXAMPLES:
#   # Rollback to previous version
#   ./scripts/rollback.sh CABC123... 7a8b9c0d...
#
#   # Find previous WASM hash from upgrade log
#   cat deployments/upgrades.json | jq '.upgrades[-1].old_wasm_hash'
#
#   # Rollback on mainnet (requires confirmation)
#   ./scripts/rollback.sh CABC123... 7a8b9c0d... -n mainnet
#
# HOW TO FIND PREVIOUS WASM HASH:
#   1. Check deployments/upgrades.json for old_wasm_hash values
#   2. Check deployments/<network>.json for historical deployments
#   3. If you have the old WASM file, install it to get the hash:
#      stellar contract install --wasm old_contract.wasm
#
# PREREQUISITES:
#   - The previous_wasm_hash must already be installed on the network
#   - Source identity must be authorized as contract admin
#   - Contract must have upgrade(new_wasm_hash) function
#
# POST-ROLLBACK CHECKLIST:
#   [ ] Verify contract is responsive (./contracts/scripts/verify-deployment.sh)
#   [ ] Check if data migration is needed
#   [ ] Test critical contract functions
#   [ ] Update documentation/status
#   [ ] Notify relevant stakeholders
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
PREVIOUS_WASM_HASH=""
NETWORK="testnet"
SOURCE_IDENTITY=""
CONFIG_FILE=""
FORCE="false"
DRY_RUN="false"
VERBOSE="false"

ROLLBACK_LOG=""

# ------------------------------------------------------------------------------
# Usage
# ------------------------------------------------------------------------------

show_usage() {
    head -70 "$0" | grep -E "^#" | sed 's/^# \?//'
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
            --force)
                FORCE="true"
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
                exit 1
                ;;
            *)
                if [[ -z "$CONTRACT_ID" ]]; then
                    CONTRACT_ID="$1"
                elif [[ -z "$PREVIOUS_WASM_HASH" ]]; then
                    PREVIOUS_WASM_HASH="$1"
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
        echo "Usage: $0 <contract_id> <previous_wasm_hash> [options]"
        exit 1
    fi

    if [[ ! "$CONTRACT_ID" =~ ^C[A-Z0-9]{55}$ ]]; then
        log_warn "Contract ID format may be invalid: $CONTRACT_ID"
    fi

    log_info "Contract ID: $CONTRACT_ID"

    # Check WASM hash
    if [[ -z "$PREVIOUS_WASM_HASH" ]]; then
        log_error "No previous WASM hash specified"
        echo ""
        echo "To find previous WASM hashes, check:"
        echo "  - deployments/upgrades.json (old_wasm_hash field)"
        echo "  - deployments/${NETWORK}.json (wasm_hash field)"
        exit 1
    fi

    # Basic WASM hash format validation (64 hex chars)
    if [[ ! "$PREVIOUS_WASM_HASH" =~ ^[a-f0-9]{64}$ ]]; then
        log_warn "WASM hash format may be invalid: $PREVIOUS_WASM_HASH"
        log_warn "Expected: 64 lowercase hexadecimal characters"
    fi

    log_info "Target WASM hash: $PREVIOUS_WASM_HASH"

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

load_rollback_config() {
    log_section "Loading Configuration"

    # Save command-line flags before loading config (CLI takes precedence)
    local cli_dry_run="$DRY_RUN"
    local cli_verbose="$VERBOSE"
    local cli_force="$FORCE"

    if [[ -z "$CONFIG_FILE" ]]; then
        CONFIG_FILE="$SCRIPT_DIR/config/${NETWORK}.env"
    fi

    if [[ -f "$CONFIG_FILE" ]]; then
        load_config "$CONFIG_FILE"
    else
        log_warn "Config file not found: $CONFIG_FILE"
    fi

    # Restore command-line flags (they take precedence over config)
    [[ "$cli_dry_run" == "true" ]] && DRY_RUN="true"
    [[ "$cli_verbose" == "true" ]] && VERBOSE="true" && export VERBOSE
    [[ "$cli_force" == "true" ]] && FORCE="true"

    if [[ -n "$SOURCE_IDENTITY" ]]; then
        export DEPLOYER_IDENTITY="$SOURCE_IDENTITY"
    fi

    : "${SOROBAN_RPC_URL:=https://soroban-testnet.stellar.org}"
    : "${SOROBAN_NETWORK:=$NETWORK}"
    : "${DEPLOYER_IDENTITY:=default}"
    : "${CLI_TIMEOUT:=120}"

    ROLLBACK_LOG="${PROJECT_ROOT}/deployments/rollbacks.json"

    export SOROBAN_RPC_URL
    export SOROBAN_NETWORK

    log_info "RPC URL: $SOROBAN_RPC_URL"
    log_info "Network: $SOROBAN_NETWORK"
    log_info "Source: $DEPLOYER_IDENTITY"

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

    check_network_connectivity

    log_success "Pre-flight checks passed"
}

# ------------------------------------------------------------------------------
# Rollback Registry
# ------------------------------------------------------------------------------

init_rollback_registry() {
    local registry_dir
    registry_dir=$(dirname "$ROLLBACK_LOG")

    mkdir -p "$registry_dir"

    if [[ ! -f "$ROLLBACK_LOG" ]]; then
        log_info "Initializing rollback registry: $ROLLBACK_LOG"
        echo '{"rollbacks": [], "metadata": {"created": "'"$(get_timestamp)"'", "version": "1.0"}}' | jq '.' > "$ROLLBACK_LOG"
    fi
}

record_rollback() {
    local contract_id="$1"
    local target_wasm_hash="$2"
    local reason="${3:-manual rollback}"

    init_rollback_registry

    local timestamp
    timestamp=$(get_timestamp)

    local record
    record=$(jq -n \
        --arg contract_id "$contract_id" \
        --arg target_hash "$target_wasm_hash" \
        --arg network "$SOROBAN_NETWORK" \
        --arg source "$DEPLOYER_IDENTITY" \
        --arg reason "$reason" \
        --arg timestamp "$timestamp" \
        '{
            contract_id: $contract_id,
            rolled_back_to: $target_hash,
            network: $network,
            executed_by: $source,
            reason: $reason,
            executed_at: $timestamp,
            status: "completed",
            data_migration_required: "REVIEW NEEDED"
        }')

    local temp_file
    temp_file=$(mktemp)
    jq --argjson record "$record" '.rollbacks += [$record]' "$ROLLBACK_LOG" > "$temp_file"
    mv "$temp_file" "$ROLLBACK_LOG"

    log_success "Rollback recorded to registry"
}

# ------------------------------------------------------------------------------
# Rollback Execution
# ------------------------------------------------------------------------------

perform_rollback() {
    log_section "Contract Rollback"

    local cli_cmd
    cli_cmd=$(get_cli_command)

    # Display critical warning
    echo ""
    echo -e "${RED}╔══════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${RED}║                                                                  ║${NC}"
    echo -e "${RED}║                    ⚠️  CRITICAL WARNING ⚠️                         ║${NC}"
    echo -e "${RED}║                                                                  ║${NC}"
    echo -e "${RED}║   ROLLBACK ONLY REVERTS CONTRACT CODE (LOGIC).                   ║${NC}"
    echo -e "${RED}║   CONTRACT STATE (DATA) IS NOT REVERTED.                         ║${NC}"
    echo -e "${RED}║                                                                  ║${NC}"
    echo -e "${RED}║   If the upgraded version modified data structures or storage    ║${NC}"
    echo -e "${RED}║   keys, rolling back may cause DATA INCOMPATIBILITY.             ║${NC}"
    echo -e "${RED}║                                                                  ║${NC}"
    echo -e "${RED}║   MANUAL DATA MIGRATION MAY BE REQUIRED AFTER ROLLBACK.          ║${NC}"
    echo -e "${RED}║                                                                  ║${NC}"
    echo -e "${RED}╚══════════════════════════════════════════════════════════════════╝${NC}"
    echo ""

    # Show rollback details
    echo "  Rollback Details:"
    echo "  ─────────────────"
    echo "  Contract ID:      $CONTRACT_ID"
    echo "  Target WASM Hash: $PREVIOUS_WASM_HASH"
    echo "  Network:          $SOROBAN_NETWORK"
    echo "  Source:           $DEPLOYER_IDENTITY"
    echo ""

    # Mainnet extra warning
    if [[ "$NETWORK" == "mainnet" ]]; then
        echo -e "${YELLOW}  ⚠️  This is a MAINNET rollback. Real funds may be affected.${NC}"
        echo ""
    fi

    # Confirmation (unless --force)
    if [[ "$FORCE" != "true" ]]; then
        if ! confirm_action "Do you understand the risks and want to proceed with rollback?"; then
            log_info "Rollback cancelled"
            exit 0
        fi

        # Double confirmation for mainnet
        if [[ "$NETWORK" == "mainnet" ]]; then
            echo ""
            if ! confirm_action "FINAL CONFIRMATION: Execute MAINNET rollback?"; then
                log_info "Rollback cancelled"
                exit 0
            fi
        fi
    else
        log_warn "Confirmation skipped (--force flag)"
    fi

    # Dry run mode
    if [[ "$DRY_RUN" == "true" ]]; then
        log_warn "[DRY RUN] Would execute the following:"
        log_warn "  $cli_cmd contract invoke --id $CONTRACT_ID -- upgrade --new_wasm_hash $PREVIOUS_WASM_HASH"
        log_success "[DRY RUN] Simulation complete"
        return 0
    fi

    # Execute rollback (call upgrade with old hash)
    log_info "Executing rollback..."

    local rollback_result
    if ! rollback_result=$(run_with_timeout "$CLI_TIMEOUT" \
        $cli_cmd contract invoke \
        --id "$CONTRACT_ID" \
        --network "$SOROBAN_NETWORK" \
        --source "$DEPLOYER_IDENTITY" \
        --send=yes \
        -- \
        upgrade \
        --new_wasm_hash "$PREVIOUS_WASM_HASH" 2>&1); then

        log_error "Rollback failed!"
        log_error "Output: $rollback_result"
        log_error ""
        log_error "Possible causes:"
        log_error "  - WASM hash not installed on network"
        log_error "  - Source is not contract admin"
        log_error "  - Network issues"
        log_error ""
        log_error "If the WASM hash is not installed, you need the original .wasm file:"
        log_error "  stellar contract install --wasm old_contract.wasm --network $NETWORK"
        exit 1
    fi

    log_success "Rollback executed successfully"

    # Record the rollback
    record_rollback "$CONTRACT_ID" "$PREVIOUS_WASM_HASH"

    # Post-rollback summary
    log_section "Rollback Complete"
    echo ""
    echo "  Contract ID:      $CONTRACT_ID"
    echo "  Now running:      $PREVIOUS_WASM_HASH"
    echo "  Network:          $SOROBAN_NETWORK"
    echo "  Rollback Log:     $ROLLBACK_LOG"
    echo ""
    echo -e "  ${YELLOW}⚠️  POST-ROLLBACK CHECKLIST:${NC}"
    echo "  ────────────────────────────"
    echo "  [ ] Run verification: ./contracts/scripts/verify-deployment.sh $CONTRACT_ID"
    echo "  [ ] Check if data migration is needed"
    echo "  [ ] Test critical contract functions"
    echo "  [ ] Review contract state for inconsistencies"
    echo "  [ ] Update team/documentation"
    echo ""
}

# ------------------------------------------------------------------------------
# Main
# ------------------------------------------------------------------------------

main() {
    log_section "Grainlify Contract Rollback"
    log_info "Started at $(get_timestamp)"

    parse_args "$@"
    validate_inputs
    load_rollback_config
    preflight_checks
    perform_rollback

    log_success "Rollback script completed"
}

# Run main if executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
