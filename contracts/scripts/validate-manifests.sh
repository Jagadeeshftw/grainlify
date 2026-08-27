#!/usr/bin/env bash

# Backward-compatible entrypoint. Rules live in validate-manifests.js.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec node "$SCRIPT_DIR/validate-manifests.js" "$@"
