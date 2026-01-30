#!/bin/bash
# ==============================================================================
# Grainlify - Deployment Verification Script
# ==============================================================================
# Verifies that a deployed contract is healthy and responsive.
#
# This script performs basic health checks on a deployed contract:
#   - Attempts to read contract state (version, admin, or custom field)
#   - Reports HEALTHY or UNRESPONSIVE status
#   - Optionally outputs detailed contract information
#
# USAGE:
#   ./scripts/verify-deployment.sh <contract_id> [options]
#
# ARGUMENTS:
#   <contract_id>       The deployed contract ID (C... format)
#
# OPTIONS:
#   -n, --network       Network (testnet|mainnet) [default: testnet]
#   -s, --source        Source identity for reading [default: from config]
#   -c, --config        Path to configuration file
#   -f, --function      Function to call for verification [default: get_version]
#   --check-admin       Verify admin address matches expected value
#   --expected-admin    Expected admin address for --check-admin
#   --json              Output result as JSON
#   -v, --verbose       Enable verbose output
#   -h, --help          Show this help message
#
# EXAMPLES:
#   # Basic health check
#   ./scripts/verify-deployment.sh CABC123...
#
#   # Check on mainnet with JSON output
#   ./scripts/verify-deployment.sh CABC123... -n mainnet --json
#
#   # Verify admin matches expected
#   ./scripts/verify-deployment.sh CABC123... --check-admin --expected-admin GABC...
#
#   # Use custom verification function
#   ./scripts/verify-deployment.sh CABC123... -f get_balance
#
# EXIT CODES:
#   0 - Contract is healthy
#   1 - Contract is unresponsive or verification failed
#   2 - Invalid arguments or configuration error
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
NETWORK="testnet"
SOURCE_IDENTITY=""
CONFIG_FILE=""
VERIFY_FUNCTION="get_version"
CHECK_ADMIN="false"
EXPECTED_ADMIN=""
OUTPUT_JSON="false"
VERBOSE="false"

# ------------------------------------------------------------------------------
# Usage
# ------------------------------------------------------------------------------

show_usage() {
    head -50 "$0" | grep -E "^#" | sed 's/^# \?//'
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
            -f|--function)
                VERIFY_FUNCTION="$2"
                shift 2
                ;;
            --check-admin)
                CHECK_ADMIN="true"
                shift
                ;;
            --expected-admin)
                EXPECTED_ADMIN="$2"
                shift 2
                ;;
            --json)
                OUTPUT_JSON="true"
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
                exit 2
                ;;
            *)
                if [[ -z "$CONTRACT_ID" ]]; then
                    CONTRACT_ID="$1"
                else
                    log_error "Unexpected argument: $1"
                    exit 2
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
    if [[ "$OUTPUT_JSON" != "true" ]]; then
        log_section "Validating Inputs"
    fi

    # Check contract ID
    if [[ -z "$CONTRACT_ID" ]]; then
        log_error "No contract ID specified"
        echo "Usage: $0 <contract_id> [options]"
        exit 2
    fi

    # Basic format validation
    if [[ ! "$CONTRACT_ID" =~ ^C[A-Z0-9]{55}$ ]]; then
        log_warn "Contract ID format may be invalid: $CONTRACT_ID"
    fi

    if [[ "$OUTPUT_JSON" != "true" ]]; then
        log_info "Contract ID: $CONTRACT_ID"
        log_success "Inputs validated"
    fi
}

# ------------------------------------------------------------------------------
# Configuration
# ------------------------------------------------------------------------------

load_verify_config() {
    if [[ "$OUTPUT_JSON" != "true" ]]; then
        log_section "Loading Configuration"
    fi

    # Set default config
    if [[ -z "$CONFIG_FILE" ]]; then
        CONFIG_FILE="$SCRIPT_DIR/config/${NETWORK}.env"
    fi

    # Load config if exists
    if [[ -f "$CONFIG_FILE" ]]; then
        load_config "$CONFIG_FILE"
    fi

    # Override with command line
    if [[ -n "$SOURCE_IDENTITY" ]]; then
        export DEPLOYER_IDENTITY="$SOURCE_IDENTITY"
    fi

    # Set defaults
    : "${SOROBAN_RPC_URL:=https://soroban-testnet.stellar.org}"
    : "${SOROBAN_NETWORK:=$NETWORK}"
    : "${DEPLOYER_IDENTITY:=default}"
    : "${CLI_TIMEOUT:=30}"

    export SOROBAN_RPC_URL
    export SOROBAN_NETWORK

    if [[ "$OUTPUT_JSON" != "true" ]]; then
        log_info "Network: $SOROBAN_NETWORK"
        log_success "Configuration loaded"
    fi
}

# ------------------------------------------------------------------------------
# Verification Functions
# ------------------------------------------------------------------------------

# Check if contract responds to a function call
check_contract_responsive() {
    local cli_cmd
    cli_cmd=$(get_cli_command)

    local result=""
    local status="UNRESPONSIVE"
    local error_msg=""

    if [[ "$OUTPUT_JSON" != "true" ]]; then
        log_info "Checking contract responsiveness..."
        log_info "Calling function: $VERIFY_FUNCTION"
    fi

    # Try to invoke the verification function
    if result=$(run_with_timeout "$CLI_TIMEOUT" \
        $cli_cmd contract invoke \
        --id "$CONTRACT_ID" \
        --network "$SOROBAN_NETWORK" \
        --source "$DEPLOYER_IDENTITY" \
        -- \
        "$VERIFY_FUNCTION" 2>&1); then
        status="HEALTHY"
    else
        error_msg="$result"
        # Check if it's a "function not found" vs "contract unreachable"
        if echo "$result" | grep -qi "not found\|does not exist\|no such"; then
            error_msg="Function '$VERIFY_FUNCTION' not found in contract"
        fi
    fi

    echo "$status|$result|$error_msg"
}

# Check admin address if requested
check_admin_address() {
    local cli_cmd
    cli_cmd=$(get_cli_command)

    local admin_result=""

    # Try common admin getter functions
    for func in "get_admin" "admin" "owner" "get_owner"; do
        if admin_result=$(run_with_timeout "$CLI_TIMEOUT" \
            $cli_cmd contract invoke \
            --id "$CONTRACT_ID" \
            --network "$SOROBAN_NETWORK" \
            --source "$DEPLOYER_IDENTITY" \
            -- \
            "$func" 2>&1); then
            echo "$admin_result"
            return 0
        fi
    done

    echo ""
    return 1
}

# ------------------------------------------------------------------------------
# Output Functions
# ------------------------------------------------------------------------------

output_result() {
    local status="$1"
    local function_result="$2"
    local error_msg="$3"
    local admin_match="$4"

    if [[ "$OUTPUT_JSON" == "true" ]]; then
        # JSON output
        jq -n \
            --arg contract_id "$CONTRACT_ID" \
            --arg network "$SOROBAN_NETWORK" \
            --arg status "$status" \
            --arg function "$VERIFY_FUNCTION" \
            --arg result "$function_result" \
            --arg error "$error_msg" \
            --arg admin_check "$admin_match" \
            --arg timestamp "$(get_timestamp)" \
            '{
                contract_id: $contract_id,
                network: $network,
                status: $status,
                verification: {
                    function: $function,
                    result: $result,
                    error: (if $error == "" then null else $error end)
                },
                admin_check: (if $admin_check == "" then null else $admin_check end),
                verified_at: $timestamp
            }'
    else
        # Human-readable output
        log_section "Verification Result"
        echo ""

        if [[ "$status" == "HEALTHY" ]]; then
            echo -e "  Status:          ${GREEN}$status${NC}"
        else
            echo -e "  Status:          ${RED}$status${NC}"
        fi

        echo "  Contract ID:     $CONTRACT_ID"
        echo "  Network:         $SOROBAN_NETWORK"
        echo "  Function:        $VERIFY_FUNCTION"

        if [[ -n "$function_result" && "$status" == "HEALTHY" ]]; then
            echo "  Result:          $function_result"
        fi

        if [[ -n "$error_msg" ]]; then
            echo "  Error:           $error_msg"
        fi

        if [[ -n "$admin_match" ]]; then
            echo "  Admin Check:     $admin_match"
        fi

        echo ""
    fi
}

# ------------------------------------------------------------------------------
# Main Verification
# ------------------------------------------------------------------------------

perform_verification() {
    if [[ "$OUTPUT_JSON" != "true" ]]; then
        log_section "Performing Verification"
    fi

    # Check dependencies (quietly for JSON mode)
    if [[ "$OUTPUT_JSON" != "true" ]]; then
        check_dependencies
    else
        get_cli_command > /dev/null
        check_jq_installed 2>/dev/null
    fi

    # Main responsiveness check
    local check_result
    check_result=$(check_contract_responsive)

    local status
    local function_result
    local error_msg
    IFS='|' read -r status function_result error_msg <<< "$check_result"

    # Admin check if requested
    local admin_match=""
    if [[ "$CHECK_ADMIN" == "true" ]]; then
        if [[ "$OUTPUT_JSON" != "true" ]]; then
            log_info "Checking admin address..."
        fi

        local current_admin
        current_admin=$(check_admin_address)

        if [[ -n "$current_admin" ]]; then
            if [[ -n "$EXPECTED_ADMIN" ]]; then
                if [[ "$current_admin" == *"$EXPECTED_ADMIN"* ]]; then
                    admin_match="MATCH ($current_admin)"
                else
                    admin_match="MISMATCH (expected: $EXPECTED_ADMIN, got: $current_admin)"
                    status="UNHEALTHY"
                fi
            else
                admin_match="$current_admin"
            fi
        else
            admin_match="Unable to retrieve admin"
        fi
    fi

    # Output result
    output_result "$status" "$function_result" "$error_msg" "$admin_match"

    # Exit with appropriate code
    if [[ "$status" == "HEALTHY" ]]; then
        if [[ "$OUTPUT_JSON" != "true" ]]; then
            log_success "Contract verification passed"
        fi
        return 0
    else
        if [[ "$OUTPUT_JSON" != "true" ]]; then
            log_error "Contract verification failed"
        fi
        return 1
    fi
}

# ------------------------------------------------------------------------------
# Main
# ------------------------------------------------------------------------------

main() {
    parse_args "$@"
    validate_inputs
    load_verify_config
    perform_verification
}

# Run main if executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
