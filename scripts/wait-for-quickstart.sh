#!/usr/bin/env bash
# Wait until a local Stellar quickstart container is actually ready.
#
# Readiness means both:
#   * Soroban RPC answers JSON-RPC `getHealth` with `"status":"healthy"`.
#   * The friendbot endpoint answers HTTP (2xx, or the 400 it returns when
#     the required address is missing).
#
# On timeout the script prints the last responses and, when a container name is
# supplied, the container's log tail, then exits non-zero.
set -euo pipefail

RPC_URL="${GRAINLIFY_QUICKSTART_RPC_URL:-http://localhost:8000/rpc}"
FRIENDBOT_URL="${GRAINLIFY_QUICKSTART_FRIENDBOT_URL:-http://localhost:8000/friendbot}"
TIMEOUT="${GRAINLIFY_QUICKSTART_TIMEOUT:-180}"
POLL_INTERVAL="${GRAINLIFY_QUICKSTART_POLL_INTERVAL:-2}"
CONTAINER="${GRAINLIFY_QUICKSTART_CONTAINER:-}"

usage() {
  cat <<'EOF'
Usage: scripts/wait-for-quickstart.sh [options]

Wait until Soroban RPC and friendbot are ready.

Options:
  --rpc-url URL        Soroban RPC endpoint (default: http://localhost:8000/rpc)
  --friendbot-url URL  Friendbot endpoint (default: http://localhost:8000/friendbot)
  --timeout SECONDS    Total wait budget (default: 180)
  --poll-interval SECS Delay between probes (default: 2)
  --container NAME     Docker container to log on timeout
  -h, --help           Show this help

Environment variables mirror the options with a GRAINLIFY_QUICKSTART_ prefix,
for example GRAINLIFY_QUICKSTART_TIMEOUT=300.
EOF
}

die() {
  printf 'ERROR: %s\n' "$*" >&2
  exit 1
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --rpc-url)
      [ "$#" -ge 2 ] || die "--rpc-url requires a value"
      RPC_URL="$2"
      shift 2
      ;;
    --friendbot-url)
      [ "$#" -ge 2 ] || die "--friendbot-url requires a value"
      FRIENDBOT_URL="$2"
      shift 2
      ;;
    --timeout)
      [ "$#" -ge 2 ] || die "--timeout requires a value"
      TIMEOUT="$2"
      shift 2
      ;;
    --poll-interval)
      [ "$#" -ge 2 ] || die "--poll-interval requires a value"
      POLL_INTERVAL="$2"
      shift 2
      ;;
    --container)
      [ "$#" -ge 2 ] || die "--container requires a value"
      CONTAINER="$2"
      shift 2
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

command -v curl >/dev/null 2>&1 || die "curl is required"

case "$TIMEOUT" in
  '' | *[!0-9]*) die "--timeout must be a whole number of seconds" ;;
esac
[ "$TIMEOUT" -gt 0 ] || die "--timeout must be greater than zero"
if ! [[ "$POLL_INTERVAL" =~ ^[0-9]+([.][0-9]+)?$ ]]; then
  die "--poll-interval must be a positive number of seconds"
fi

printf 'Waiting up to %ss for quickstart (RPC: %s)...\n' "$TIMEOUT" "$RPC_URL"

start=$SECONDS
attempt=0
last_health='(no response)'
last_friendbot='(no response)'

while [ $((SECONDS - start)) -lt "$TIMEOUT" ]; do
  attempt=$((attempt + 1))

  health="$(
    curl -sS --max-time 5 -H 'Content-Type: application/json' \
      -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' \
      "$RPC_URL" 2>&1 || true
  )"
  friendbot_code="$(
    curl -sS --max-time 5 -o /dev/null -w '%{http_code}' \
      "$FRIENDBOT_URL" 2>/dev/null || true
  )"

  [ -n "$health" ] || health='(no response)'
  [ -n "$friendbot_code" ] || friendbot_code='000'
  last_health="$health"
  last_friendbot="$friendbot_code"

  rpc_ready=0
  if printf '%s' "$health" |
    grep -Eq '"status"[[:space:]]*:[[:space:]]*"healthy"'; then
    rpc_ready=1
  fi

  friendbot_ready=0
  case "$friendbot_code" in
    2?? | 400) friendbot_ready=1 ;;
  esac

  if [ "$rpc_ready" -eq 1 ] && [ "$friendbot_ready" -eq 1 ]; then
    printf 'Quickstart is ready after %ss (RPC healthy, friendbot HTTP %s).\n' \
      "$((SECONDS - start))" "$friendbot_code"
    exit 0
  fi

  if [ $((attempt % 5)) -eq 1 ]; then
    printf '  still waiting (%ss): rpc=%s friendbot=%s\n' \
      "$((SECONDS - start))" "$health" "$friendbot_code"
  fi

  sleep "$POLL_INTERVAL"
done

printf 'ERROR: quickstart did not become ready within %ss.\n' "$TIMEOUT" >&2
printf '  last RPC getHealth response: %s\n' "$last_health" >&2
printf '  last friendbot HTTP status: %s\n' "$last_friendbot" >&2

if [ -n "$CONTAINER" ] && command -v docker >/dev/null 2>&1; then
  printf '\nLast 200 log lines from container %s:\n' "$CONTAINER" >&2
  docker logs --tail 200 "$CONTAINER" >&2 || true
fi

exit 1
