#!/usr/bin/env bash
# Fail on mixed or drifted soroban-sdk versions (issue #1743).
# Pin table below is the source of truth; keep contracts/SDK_COMPATIBILITY.md in sync.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# manifest-root|expected resolved soroban-sdk version
MANIFEST_PINS=(
  "contracts|21.7.7"
  "contracts/bounty_escrow|21.7.7"
  "contracts/grainlify-core|21.7.7"
  "contracts/program-escrow|21.7.7"
  "contracts/view-facade|21.7.7"
  "contracts/escrow-view-facade|21.7.7"
  "soroban|23.4.1"
)

fail=0

echo "==> resolved-graph check (one soroban-sdk version per lockfile)"
for entry in "${MANIFEST_PINS[@]}"; do
  root="${entry%%|*}"
  expected="${entry##*|}"
  manifest="$ROOT_DIR/$root/Cargo.toml"

  if [[ ! -f "$ROOT_DIR/$root/Cargo.lock" ]]; then
    echo "FAIL $root: no committed Cargo.lock, the gate needs --locked resolution"
    fail=1
    continue
  fi

  versions="$(cargo tree --manifest-path "$manifest" --locked --prefix none 2>/dev/null \
    | { grep -E '^soroban-sdk v' || true; } | awk '{print $2}' | sed 's/^v//' | sort -u)" || {
    echo "FAIL $root: cargo tree --locked failed (lockfile out of sync with manifest?)"
    fail=1
    continue
  }

  count="$(printf '%s' "$versions" | grep -c . || true)"
  if [[ "$count" -ne 1 ]]; then
    echo "FAIL $root: expected exactly one soroban-sdk version, found $count:"
    printf '%s\n' "$versions" | sed 's/^/       /'
    fail=1
  elif [[ "$versions" != "$expected" ]]; then
    echo "FAIL $root: resolves soroban-sdk $versions, pin table says $expected"
    fail=1
  else
    echo "ok   $root: soroban-sdk $versions"
  fi
done

echo "==> manifest check (every soroban-sdk requirement is an exact pin)"
# grep -r skips the contracts/escrow dir symlink, so bounty-escrow is not double-counted
while IFS= read -r line; do
  case "$line" in
    *"workspace = true"*) continue ;;
    *'"=21.7.7"'* | *'"=23.4.1"'*) continue ;;
    *)
      echo "FAIL non-exact or unknown soroban-sdk pin: $line"
      fail=1
      ;;
  esac
done < <(grep -rn --include=Cargo.toml 'soroban-sdk *=' "$ROOT_DIR/contracts" "$ROOT_DIR/soroban")

if [[ "$fail" -ne 0 ]]; then
  echo "SDK version check FAILED"
  exit 1
fi
echo "SDK version check passed"
