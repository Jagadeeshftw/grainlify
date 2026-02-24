#!/bin/bash
# ==============================================================================
# Grainlify - Smart Contract Deployment Script
# ==============================================================================
# Deploys a Soroban smart contract to the specified network.
#
# USAGE:
#   ./scripts/deploy.sh <wasm_file> [options]
#
# ARGUMENTS:
#   <wasm_file>     Path to the compiled .wasm contract file
#
# OPTIONS:
#   -n, --network   Network to deploy to (testnet|mainnet) [default: testnet]
#   -c, --config    Path to configuration file [default: scripts/config/<network>.env]
#   -i, --identity  Deployer identity name [default: from config]
#   -N, --name      Contract name for registry [default: derived from filename]
#   --init          Initialize contract after deployment (calls init function)
#   --init-args     Arguments for init function (legacy, prefer -- passthrough)
#   --dry-run       Simulate deployment without executing
#   -v, --verbose   Enable verbose output
#   -h, --help      Show this help message
#   --              Everything after this is passed to the init function
#
# EXAMPLES:
#   # Deploy escrow contract to testnet
#   ./scripts/deploy.sh soroban/target/wasm32-unknown-unknown/release/escrow.wasm
#
#   # Deploy to mainnet with specific identity
#   ./scripts/deploy.sh contract.wasm -n mainnet -i mainnet-deployer
#
#   # Deploy and initialize with arguments (RECOMMENDED: use -- passthrough)
#   ./scripts/deploy.sh contract.wasm -- --admin GABC... --fee 100
#
#   # Deploy and initialize with multiple arguments
#   ./scripts/deploy.sh contract.wasm -- --admin GABC... --token CDEF... --oracle GHIJ...
#
#   # Legacy init-args syntax (still supported)
#   ./scripts/deploy.sh contract.wasm --init --init-args '--admin GABC...'
#
#   # Dry run to see what would happen
#   ./scripts/deploy.sh contract.wasm --dry-run
#
# ENVIRONMENT VARIABLES:
#   SOROBAN_RPC_URL           RPC endpoint URL
#   SOROBAN_NETWORK           Network name (testnet/mainnet)
#   SOROBAN_NETWORK_PASSPHRASE Network passphrase
#   DEPLOYER_IDENTITY         Identity name for signing
#   DEPLOYMENT_LOG            Path to deployment registry JSON
#
# SECURITY:
#   - Never pass private keys as command line arguments
#   - Use stored identities (stellar keys generate) or environment variables
#   - Mainnet deployments require explicit confirmation
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

NETWORK="testnet"
CONFIG_FILE=""
IDENTITY=""
CONTRACT_NAME=""
WASM_FILE=""
DO_INIT="false"
INIT_ARGS=""
EXTRA_ARGS=()  # Passthrough arguments after --
DRY_RUN="false"
VERBOSE="false"

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
            -c|--config)
                CONFIG_FILE="$2"
                shift 2
                ;;
            -i|--identity)
                IDENTITY="$2"
                shift 2
                ;;
            -N|--name)
                CONTRACT_NAME="$2"
                shift 2
                ;;
            --init)
                DO_INIT="true"
                shift
                ;;
            --init-args)
                INIT_ARGS="$2"
                shift 2
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
            --)
                # Everything after -- is passed through to the init function
                shift
                EXTRA_ARGS=("$@")
                DO_INIT="true"  # Implies --init when using passthrough
                break
                ;;
            -*)
                log_error "Unknown option: $1"
                echo "Use --help for usage information"
                exit 1
                ;;
            *)
                if [[ -z "$WASM_FILE" ]]; then
                    WASM_FILE="$1"
                else
                    log_error "Unexpected argument: $1"
                    log_error "Hint: Use -- to pass arguments to the contract init function"
                    log_error "Example: $0 contract.wasm -- --admin G... --amount 100"
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

    # Check WASM file is provided
    if [[ -z "$WASM_FILE" ]]; then
        log_error "No WASM file specified"
        echo "Usage: $0 <wasm_file> [options]"
        echo "Use --help for more information"
        exit 1
    fi

    # Resolve to absolute path
    if [[ ! "$WASM_FILE" = /* ]]; then
        WASM_FILE="$PROJECT_ROOT/$WASM_FILE"
    fi

    # Verify WASM file
    verify_wasm_file "$WASM_FILE"

    # Derive contract name from filename if not provided
    if [[ -z "$CONTRACT_NAME" ]]; then
        CONTRACT_NAME=$(basename "$WASM_FILE" .wasm)
        log_debug "Derived contract name: $CONTRACT_NAME"
    fi

    # Validate network
    case "$NETWORK" in
        testnet|mainnet|local|futurenet)
            log_info "Target network: $NETWORK"
            ;;
        *)
            log_error "Invalid network: $NETWORK"
            log_error "Valid options: testnet, mainnet, local, futurenet"
            exit 1
            ;;
    esac

    # Set default config file if not provided
    if [[ -z "$CONFIG_FILE" ]]; then
        CONFIG_FILE="$SCRIPT_DIR/config/${NETWORK}.env"
    fi

    log_success "Inputs validated"
}

# ------------------------------------------------------------------------------
# Configuration
# ------------------------------------------------------------------------------

load_deployment_config() {
    log_section "Loading Configuration"

    # Save command-line flags before loading config (CLI takes precedence)
    local cli_dry_run="$DRY_RUN"
    local cli_verbose="$VERBOSE"

    # Load config file if it exists
    if [[ -f "$CONFIG_FILE" ]]; then
        load_config "$CONFIG_FILE"
    else
        log_warn "Config file not found: $CONFIG_FILE"
        log_warn "Using environment variables and defaults"
    fi

    # Restore command-line flags (they take precedence over config)
    [[ "$cli_dry_run" == "true" ]] && DRY_RUN="true"
    [[ "$cli_verbose" == "true" ]] && VERBOSE="true" && export VERBOSE

    # Override with command line arguments
    if [[ -n "$IDENTITY" ]]; then
        export DEPLOYER_IDENTITY="$IDENTITY"
    fi

    # Set defaults for required variables
    : "${SOROBAN_RPC_URL:=https://soroban-testnet.stellar.org}"
    : "${SOROBAN_NETWORK:=$NETWORK}"
    : "${DEPLOYER_IDENTITY:=default}"
    : "${DEPLOYMENT_LOG:=$PROJECT_ROOT/deployments/${NETWORK}.json}"
    : "${CLI_TIMEOUT:=120}"
    : "${RETRY_ATTEMPTS:=3}"
    : "${RETRY_DELAY:=5}"

    # Export for CLI
    export SOROBAN_RPC_URL
    export SOROBAN_NETWORK

    log_info "RPC URL: $SOROBAN_RPC_URL"
    log_info "Network: $SOROBAN_NETWORK"
    log_info "Deployer: $DEPLOYER_IDENTITY"
    log_info "Registry: $DEPLOYMENT_LOG"

    log_success "Configuration loaded"
}

# ------------------------------------------------------------------------------
# Pre-flight Checks
# ------------------------------------------------------------------------------

preflight_checks() {
    log_section "Pre-flight Checks"

    # Check dependencies
    check_dependencies

    # Check network connectivity
    check_network_connectivity

    # Verify identity exists
    local cli_cmd
    cli_cmd=$(get_cli_command)

    log_info "Verifying deployer identity: $DEPLOYER_IDENTITY"

    if ! $cli_cmd keys address "$DEPLOYER_IDENTITY" > /dev/null 2>&1; then
        log_error "Identity not found: $DEPLOYER_IDENTITY"
        log_error "Create with: $cli_cmd keys generate --global $DEPLOYER_IDENTITY"
        log_error "Fund with:   $cli_cmd keys fund $DEPLOYER_IDENTITY --network testnet"
        exit 1
    fi

    local deployer_address
    deployer_address=$($cli_cmd keys address "$DEPLOYER_IDENTITY")
    log_info "Deployer address: $deployer_address"

    log_success "Pre-flight checks passed"
}

# ------------------------------------------------------------------------------
# Deployment
# ------------------------------------------------------------------------------

deploy_contract() {
    log_section "Deploying Contract"

    local cli_cmd
    cli_cmd=$(get_cli_command)

    local wasm_hash
    local contract_id

    # Calculate WASM hash for tracking
    local file_hash
    file_hash=$(get_file_hash "$WASM_FILE")
    log_info "WASM file hash: $file_hash"

    # Mainnet safety check
    if [[ "$NETWORK" == "mainnet" ]]; then
        log_warn "=========================================="
        log_warn "  MAINNET DEPLOYMENT"
        log_warn "=========================================="
        log_warn "Contract: $CONTRACT_NAME"
        log_warn "WASM: $WASM_FILE"
        log_warn "Deployer: $DEPLOYER_IDENTITY"
        log_warn ""

        if ! confirm_action "Deploy to MAINNET? This action cannot be undone."; then
            log_info "Deployment cancelled"
            exit 0
        fi
    fi

    # Dry run mode
    if [[ "$DRY_RUN" == "true" ]]; then
        log_warn "[DRY RUN] Would execute the following:"
        log_warn "  1. Install WASM: $cli_cmd contract install --wasm $WASM_FILE --network $SOROBAN_NETWORK --source $DEPLOYER_IDENTITY"
        log_warn "  2. Deploy contract: $cli_cmd contract deploy --wasm-hash <hash> --network $SOROBAN_NETWORK --source $DEPLOYER_IDENTITY"
        if [[ "$DO_INIT" == "true" ]]; then
            local display_args="${INIT_ARGS:-}"
            if [[ ${#EXTRA_ARGS[@]} -gt 0 ]]; then
                display_args="${display_args:+$display_args }${EXTRA_ARGS[*]}"
            fi
            log_warn "  3. Initialize: $cli_cmd contract invoke --id <contract_id> -- init ${display_args:-<no args>}"
        fi
        log_success "[DRY RUN] Simulation complete"
        return 0
    fi

    # Step 1: Install WASM (upload and get hash)
    log_info "Step 1/2: Installing WASM..."

    wasm_hash=$(retry_command "$RETRY_ATTEMPTS" "$RETRY_DELAY" \
        run_with_timeout "$CLI_TIMEOUT" \
        $cli_cmd contract install \
        --wasm "$WASM_FILE" \
        --network "$SOROBAN_NETWORK" \
        --source "$DEPLOYER_IDENTITY")

    if [[ -z "$wasm_hash" ]]; then
        log_error "Failed to install WASM"
        exit 1
    fi

    log_success "WASM installed: $wasm_hash"

    # Step 2: Deploy contract
    log_info "Step 2/2: Deploying contract..."

    contract_id=$(retry_command "$RETRY_ATTEMPTS" "$RETRY_DELAY" \
        run_with_timeout "$CLI_TIMEOUT" \
        $cli_cmd contract deploy \
        --wasm-hash "$wasm_hash" \
        --network "$SOROBAN_NETWORK" \
        --source "$DEPLOYER_IDENTITY")

    if [[ -z "$contract_id" ]]; then
        log_error "Failed to deploy contract"
        exit 1
    fi

    log_success "Contract deployed: $contract_id"

    # Step 3: Initialize if requested
    if [[ "$DO_INIT" == "true" ]]; then
        log_info "Step 3: Initializing contract..."

        # Build init arguments array
        local -a init_args=()

        # Add legacy --init-args if provided (for backwards compatibility)
        if [[ -n "$INIT_ARGS" ]]; then
            log_debug "Init args (legacy): $INIT_ARGS"
            # shellcheck disable=SC2206
            init_args+=($INIT_ARGS)
        fi

        # Add passthrough arguments (preferred method)
        if [[ ${#EXTRA_ARGS[@]} -gt 0 ]]; then
            log_debug "Init args (passthrough): ${EXTRA_ARGS[*]}"
            init_args+=("${EXTRA_ARGS[@]}")
        fi

        log_debug "Final init command args: ${init_args[*]:-<none>}"

        # Execute init with proper argument handling (no eval needed)
        if ! $cli_cmd contract invoke \
            --id "$contract_id" \
            --network "$SOROBAN_NETWORK" \
            --source "$DEPLOYER_IDENTITY" \
            -- \
            init \
            "${init_args[@]+"${init_args[@]}"}"; then
            log_warn "Initialization failed or not supported"
            log_warn "You may need to initialize the contract manually"
        else
            log_success "Contract initialized"
        fi
    fi

    # Record deployment
    log_info "Recording deployment..."
    append_to_registry "$DEPLOYMENT_LOG" "$contract_id" "$wasm_hash" "$CONTRACT_NAME"

    # Summary
    log_section "Deployment Complete"
    echo ""
    echo "  Contract Name:  $CONTRACT_NAME"
    echo "  Contract ID:    $contract_id"
    echo "  WASM Hash:      $wasm_hash"
    echo "  Network:        $SOROBAN_NETWORK"
    echo "  Registry:       $DEPLOYMENT_LOG"
    echo ""

    # Output contract ID for scripting
    echo "$contract_id"
}

# ------------------------------------------------------------------------------
# Main
# ------------------------------------------------------------------------------

main() {
    log_section "Grainlify Contract Deployment"
    log_info "Started at $(get_timestamp)"

    parse_args "$@"
    validate_inputs
    load_deployment_config
    preflight_checks
    deploy_contract

    log_success "Deployment script completed"
}

# Run main if executed directly (not sourced)
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi

if [[ "${SUDO_FAKE_INSTALL_FAIL:-0}" == "1" ]]; then
  log_error "Simulated install failure"
  exit 1
fi
