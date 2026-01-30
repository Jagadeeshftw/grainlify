#!/bin/bash
# ==============================================================================
# Grainlify - Common Utility Functions
# ==============================================================================
# Shared functions for deployment scripts.
# Source this file in other scripts: source "$(dirname "$0")/utils/common.sh"
#
# Functions provided:
#   - log_info, log_warn, log_error, log_success, log_debug
#   - check_soroban_cli_installed
#   - check_stellar_cli_installed
#   - load_config
#   - require_env_var
#   - confirm_action
#   - retry_command
#   - append_to_registry
#   - get_timestamp
# ==============================================================================

set -euo pipefail

# ------------------------------------------------------------------------------
# Color Codes (disable if not in terminal)
# ------------------------------------------------------------------------------
if [[ -t 1 ]]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    BLUE='\033[0;34m'
    CYAN='\033[0;36m'
    NC='\033[0m' # No Color
    BOLD='\033[1m'
else
    RED=''
    GREEN=''
    YELLOW=''
    BLUE=''
    CYAN=''
    NC=''
    BOLD=''
fi

# ------------------------------------------------------------------------------
# Logging Functions
# ------------------------------------------------------------------------------

# Get ISO 8601 timestamp
get_timestamp() {
    date -u +"%Y-%m-%dT%H:%M:%SZ"
}

log_info() {
    echo -e "${BLUE}[INFO]${NC} $(get_timestamp) $*"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $(get_timestamp) $*" >&2
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $(get_timestamp) $*" >&2
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $(get_timestamp) $*"
}

log_debug() {
    if [[ "${VERBOSE:-false}" == "true" ]]; then
        echo -e "${CYAN}[DEBUG]${NC} $(get_timestamp) $*" >&2
    fi
}

# Print a section header
log_section() {
    echo ""
    echo -e "${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BOLD}  $*${NC}"
    echo -e "${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

# ------------------------------------------------------------------------------
# Dependency Checks
# ------------------------------------------------------------------------------

# Check if soroban CLI is installed (legacy command)
check_soroban_cli_installed() {
    if command -v soroban &> /dev/null; then
        local version
        version=$(soroban --version 2>/dev/null || echo "unknown")
        log_debug "Found soroban CLI: $version"
        return 0
    fi
    return 1
}

# Check if stellar CLI is installed (current command)
check_stellar_cli_installed() {
    if command -v stellar &> /dev/null; then
        local version
        version=$(stellar --version 2>/dev/null || echo "unknown")
        log_debug "Found stellar CLI: $version"
        return 0
    fi
    return 1
}

# Get the correct CLI command (stellar preferred, soroban fallback)
get_cli_command() {
    if check_stellar_cli_installed; then
        echo "stellar"
    elif check_soroban_cli_installed; then
        echo "soroban"
    else
        log_error "Neither 'stellar' nor 'soroban' CLI found."
        log_error "Install with: cargo install --locked stellar-cli"
        log_error "Or visit: https://developers.stellar.org/docs/tools/developer-tools"
        exit 1
    fi
}

# Check if jq is installed (for JSON manipulation)
check_jq_installed() {
    if ! command -v jq &> /dev/null; then
        log_error "'jq' is required but not installed."
        log_error "Install with: apt-get install jq (Debian/Ubuntu) or brew install jq (macOS)"
        exit 1
    fi
    log_debug "Found jq: $(jq --version)"
}

# Verify all required dependencies
check_dependencies() {
    log_info "Checking dependencies..."

    local cli_cmd
    cli_cmd=$(get_cli_command)
    log_info "Using CLI: $cli_cmd"

    check_jq_installed

    log_success "All dependencies satisfied"
}

# ------------------------------------------------------------------------------
# Configuration Loading
# ------------------------------------------------------------------------------

# Load configuration from an env file
# Usage: load_config "path/to/config.env"
load_config() {
    local config_file="${1:-}"

    if [[ -z "$config_file" ]]; then
        log_error "No configuration file specified"
        return 1
    fi

    if [[ ! -f "$config_file" ]]; then
        log_error "Configuration file not found: $config_file"
        return 1
    fi

    log_info "Loading configuration from: $config_file"

    # Export variables from the config file
    # This handles comments and empty lines safely
    while IFS='=' read -r key value || [[ -n "$key" ]]; do
        # Skip comments and empty lines
        [[ -z "$key" || "$key" =~ ^[[:space:]]*# ]] && continue

        # Remove leading/trailing whitespace from key
        key=$(echo "$key" | xargs)

        # Skip if key is empty after trimming
        [[ -z "$key" ]] && continue

        # Remove surrounding quotes from value if present
        value="${value%\"}"
        value="${value#\"}"
        value="${value%\'}"
        value="${value#\'}"

        # Export the variable
        export "$key=$value"
        log_debug "Loaded: $key"
    done < "$config_file"

    log_success "Configuration loaded"
}

# Require that an environment variable is set
# Usage: require_env_var "VAR_NAME" "description"
require_env_var() {
    local var_name="$1"
    local description="${2:-$var_name}"

    if [[ -z "${!var_name:-}" ]]; then
        log_error "Required environment variable not set: $var_name"
        log_error "Description: $description"
        return 1
    fi

    log_debug "Verified: $var_name is set"
}

# ------------------------------------------------------------------------------
# User Interaction
# ------------------------------------------------------------------------------

# Prompt user for confirmation
# Usage: confirm_action "Are you sure you want to deploy?"
confirm_action() {
    local prompt="${1:-Continue?}"

    # Skip confirmation if REQUIRE_CONFIRMATION is false
    if [[ "${REQUIRE_CONFIRMATION:-true}" != "true" ]]; then
        log_debug "Confirmation skipped (REQUIRE_CONFIRMATION=false)"
        return 0
    fi

    # Skip confirmation in non-interactive mode
    if [[ ! -t 0 ]]; then
        log_warn "Non-interactive mode: assuming 'yes'"
        return 0
    fi

    echo -e -n "${YELLOW}$prompt [y/N]: ${NC}"
    read -r response

    case "$response" in
        [yY][eE][sS]|[yY])
            return 0
            ;;
        *)
            log_info "Operation cancelled by user"
            return 1
            ;;
    esac
}

# ------------------------------------------------------------------------------
# Command Execution
# ------------------------------------------------------------------------------

# Retry a command with exponential backoff
# Usage: retry_command 3 5 "command" "arg1" "arg2"
retry_command() {
    local max_attempts="${1:-3}"
    local delay="${2:-5}"
    shift 2

    local attempt=1

    while [[ $attempt -le $max_attempts ]]; do
        log_debug "Attempt $attempt of $max_attempts: $*"

        if "$@"; then
            return 0
        fi

        if [[ $attempt -lt $max_attempts ]]; then
            log_warn "Command failed, retrying in ${delay}s..."
            sleep "$delay"
            # Exponential backoff
            delay=$((delay * 2))
        fi

        ((attempt++))
    done

    log_error "Command failed after $max_attempts attempts: $*"
    return 1
}

# Execute a command with timeout
# Usage: run_with_timeout 120 "command" "arg1"
run_with_timeout() {
    local timeout_seconds="${1:-120}"
    shift

    if command -v timeout &> /dev/null; then
        timeout "$timeout_seconds" "$@"
    else
        # Fallback for systems without timeout command
        "$@"
    fi
}

# ------------------------------------------------------------------------------
# Deployment Registry
# ------------------------------------------------------------------------------

# Initialize the deployment registry if it doesn't exist
# Usage: init_registry "deployments/testnet.json"
init_registry() {
    local registry_file="${1:-deployments/registry.json}"
    local registry_dir
    registry_dir=$(dirname "$registry_file")

    # Create directory if needed
    mkdir -p "$registry_dir"

    # Create empty registry if it doesn't exist
    if [[ ! -f "$registry_file" ]]; then
        log_info "Initializing deployment registry: $registry_file"
        echo '{"deployments": [], "metadata": {"created": "'"$(get_timestamp)"'", "version": "1.0"}}' | jq '.' > "$registry_file"
    fi

    log_debug "Registry ready: $registry_file"
}

# Append a deployment record to the registry
# Usage: append_to_registry "deployments/testnet.json" "contract_id" "wasm_hash" "contract_name"
append_to_registry() {
    local registry_file="$1"
    local contract_id="$2"
    local wasm_hash="$3"
    local contract_name="${4:-unknown}"
    local network="${SOROBAN_NETWORK:-unknown}"
    local deployer="${DEPLOYER_IDENTITY:-unknown}"

    init_registry "$registry_file"

    local timestamp
    timestamp=$(get_timestamp)

    # Create the new deployment record
    local new_record
    new_record=$(jq -n \
        --arg id "$contract_id" \
        --arg hash "$wasm_hash" \
        --arg name "$contract_name" \
        --arg network "$network" \
        --arg deployer "$deployer" \
        --arg timestamp "$timestamp" \
        '{
            contract_id: $id,
            wasm_hash: $hash,
            contract_name: $name,
            network: $network,
            deployer: $deployer,
            deployed_at: $timestamp,
            status: "deployed"
        }')

    # Append to the registry
    local temp_file
    temp_file=$(mktemp)
    jq --argjson record "$new_record" '.deployments += [$record]' "$registry_file" > "$temp_file"
    mv "$temp_file" "$registry_file"

    log_success "Recorded deployment: $contract_name ($contract_id)"
}

# Get the latest deployment for a contract name
# Usage: get_latest_deployment "deployments/testnet.json" "escrow"
get_latest_deployment() {
    local registry_file="$1"
    local contract_name="$2"

    if [[ ! -f "$registry_file" ]]; then
        return 1
    fi

    jq -r --arg name "$contract_name" '
        .deployments
        | map(select(.contract_name == $name))
        | sort_by(.deployed_at)
        | last
        | .contract_id // empty
    ' "$registry_file"
}

# ------------------------------------------------------------------------------
# WASM Utilities
# ------------------------------------------------------------------------------

# Verify a WASM file exists and is valid
# Usage: verify_wasm_file "path/to/contract.wasm"
verify_wasm_file() {
    local wasm_file="$1"

    if [[ ! -f "$wasm_file" ]]; then
        log_error "WASM file not found: $wasm_file"
        return 1
    fi

    # Check file is not empty
    if [[ ! -s "$wasm_file" ]]; then
        log_error "WASM file is empty: $wasm_file"
        return 1
    fi

    # Check for WASM magic bytes (optional, basic validation)
    local magic_bytes
    magic_bytes=$(xxd -l 4 -p "$wasm_file" 2>/dev/null || echo "")
    if [[ "$magic_bytes" != "0061736d" ]]; then
        log_warn "File may not be a valid WASM binary: $wasm_file"
        log_warn "Expected magic bytes: 0061736d, got: $magic_bytes"
    fi

    local file_size
    file_size=$(stat -f%z "$wasm_file" 2>/dev/null || stat -c%s "$wasm_file" 2>/dev/null || echo "unknown")
    log_info "WASM file verified: $wasm_file ($file_size bytes)"

    return 0
}

# Calculate SHA256 hash of a file
# Usage: get_file_hash "path/to/file"
get_file_hash() {
    local file="$1"

    if command -v sha256sum &> /dev/null; then
        sha256sum "$file" | cut -d' ' -f1
    elif command -v shasum &> /dev/null; then
        shasum -a 256 "$file" | cut -d' ' -f1
    else
        log_warn "No SHA256 tool found, skipping hash calculation"
        echo "unknown"
    fi
}

# ------------------------------------------------------------------------------
# Network Utilities
# ------------------------------------------------------------------------------

# Check if the network is reachable
# Usage: check_network_connectivity
check_network_connectivity() {
    local rpc_url="${SOROBAN_RPC_URL:-}"

    if [[ -z "$rpc_url" ]]; then
        log_warn "SOROBAN_RPC_URL not set, skipping connectivity check"
        return 0
    fi

    log_info "Checking network connectivity: $rpc_url"

    if command -v curl &> /dev/null; then
        if curl -s --connect-timeout 10 "$rpc_url" > /dev/null 2>&1; then
            log_success "Network is reachable"
            return 0
        else
            log_error "Cannot reach network: $rpc_url"
            return 1
        fi
    else
        log_warn "curl not found, skipping connectivity check"
        return 0
    fi
}

# Fund an account on testnet using Friendbot
# Usage: fund_testnet_account "public_key"
fund_testnet_account() {
    local public_key="$1"
    local friendbot_url="${FRIENDBOT_URL:-https://friendbot.stellar.org}"

    log_info "Funding account via Friendbot: $public_key"

    local response
    response=$(curl -s "${friendbot_url}?addr=${public_key}")

    if echo "$response" | jq -e '.successful // .id' > /dev/null 2>&1; then
        log_success "Account funded successfully"
        return 0
    else
        log_warn "Friendbot response: $response"
        log_warn "Account may already be funded or funding failed"
        return 0  # Don't fail - account might already be funded
    fi
}

# ------------------------------------------------------------------------------
# Script Initialization
# ------------------------------------------------------------------------------

# Get the directory of the calling script
get_script_dir() {
    local source="${BASH_SOURCE[1]:-${BASH_SOURCE[0]}}"
    local dir

    while [[ -L "$source" ]]; do
        dir=$(cd -P "$(dirname "$source")" && pwd)
        source=$(readlink "$source")
        [[ $source != /* ]] && source="$dir/$source"
    done

    cd -P "$(dirname "$source")" && pwd
}

# Get the project root directory
get_project_root() {
    local script_dir
    script_dir=$(get_script_dir)

    # Navigate up from scripts/ to project root
    cd "$script_dir/.." && pwd
}

log_debug "common.sh loaded successfully"
