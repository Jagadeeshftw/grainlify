#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/eb_apply_env.sh <eb_environment_name> <path_to_env_file>

Example:
  ./scripts/eb_apply_env.sh grainlify-api-1 backend/.env
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

ENV_NAME="${1:-}"
ENV_FILE="${2:-}"

if [[ -z "$ENV_NAME" || -z "$ENV_FILE" ]]; then
  usage
  exit 2
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$REPO_ROOT"

if [[ ! -d ".elasticbeanstalk" ]]; then
  echo "error: .elasticbeanstalk/ not found. Run this script from inside an EB-initialized repo." >&2
  exit 1
fi

if [[ ! -f "$ENV_FILE" ]]; then
  echo "error: env file not found: $ENV_FILE" >&2
  exit 1
fi

if ! command -v eb >/dev/null 2>&1; then
  echo "error: eb CLI not found. Install/enable the Elastic Beanstalk CLI first." >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "error: python3 not found (needed to parse .env reliably)." >&2
  exit 1
fi

echo "Using EB environment: $ENV_NAME"
eb use "$ENV_NAME" >/dev/null

echo "Applying env vars from: $ENV_FILE"

# Parse dotenv -> NUL-separated KEY=VALUE pairs; then pass as distinct args to eb setenv.
# This avoids breaking on spaces in values.
python3 - "$ENV_FILE" <<'PY' | xargs -0 eb setenv
import sys

path = sys.argv[1]

def strip_quotes(v: str) -> str:
    v = v.strip()
    if len(v) >= 2 and v[0] == v[-1] and v[0] in ("'", '"'):
        return v[1:-1]
    return v

def strip_inline_comment(v: str) -> str:
    """
    Strip inline comments like:
      KEY=value # comment
    BUT keep hashes inside quoted strings:
      KEY="value # keep"
    """
    s = v.strip()
    if not s:
        return s
    # If value starts with a quote, keep as-is (strip_quotes handles it)
    if s[0] in ("'", '"'):
        return s
    # Remove anything after a # (common dotenv comment style)
    if "#" in s:
        s = s.split("#", 1)[0].rstrip()
    return s

with open(path, "r", encoding="utf-8") as f:
    for raw in f:
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("export "):
            line = line[len("export "):].lstrip()
        if "=" not in line:
            continue
        key, value = line.split("=", 1)
        key = key.strip()
        if not key:
            continue
        value = strip_inline_comment(value)
        value = strip_quotes(value)
        # Output as NUL-delimited so xargs -0 treats each KEY=VALUE as one argument.
        sys.stdout.write(f"{key}={value}\0")
PY

echo "Done."


