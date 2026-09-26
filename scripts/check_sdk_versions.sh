#!/usr/bin/env bash
# Fail on mixed or drifted soroban-sdk versions (issues #1743, #1848).
# Pin table below is the source of truth; keep contracts/SDK_COMPATIBILITY.md in sync.
# The script ALWAYS prints the cross-tree divergence report so dual majors cannot
# pass silently.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Repository target (single destination major). Trees not on this pin are owned exceptions.
TARGET_SDK="23.4.1"
EXCEPTION_OWNER="@Jagadeeshftw"

# manifest-root|expected resolved soroban-sdk version|status
MANIFEST_PINS=(
  "contracts|21.7.7|exception"
  "contracts/bounty_escrow|21.7.7|exception"
  "contracts/grainlify-core|21.7.7|exception"
  "contracts/program-escrow|21.7.7|exception"
  "contracts/view-facade|21.7.7|exception"
  "contracts/escrow-view-facade|21.7.7|exception"
  "soroban|23.4.1|on-target"
)

echo "==> SDK divergence report (issue #1848)"
echo "    repository target soroban-sdk: ${TARGET_SDK}"
echo "    exception owner: ${EXCEPTION_OWNER}"
behind=0
on_target=0
for entry in "${MANIFEST_PINS[@]}"; do
  IFS='|' read -r root expected status <<< "$entry"
  if [[ "$expected" == "$TARGET_SDK" ]]; then
    echo "    ON TARGET  ${root}: soroban-sdk ${expected}"
    on_target=$((on_target + 1))
  else
    echo "    EXCEPTION  ${root}: soroban-sdk ${expected} (behind target ${TARGET_SDK}; owner ${EXCEPTION_OWNER})"
    behind=$((behind + 1))
  fi
done
echo "    summary: ${on_target} on target, ${behind} owned exception(s); two majors are intentional until contracts/ migrates"
echo

fail=0

echo "==> resolved-graph check (one soroban-sdk version per lockfile)"
for entry in "${MANIFEST_PINS[@]}"; do
  IFS='|' read -r root expected status <<< "$entry"
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
    echo "ok   $root: soroban-sdk $versions ($status)"
  fi
done

echo "==> manifest check (every soroban-sdk requirement is an exact allowed pin)"
# Only =21.7.7 and =23.4.1 are allowed — a third version fails CI.
# grep -r skips the contracts/escrow dir symlink, so bounty-escrow is not double-counted
while IFS= read -r line; do
  case "$line" in
    *"workspace = true"*) continue ;;
    *'"=21.7.7"'* | *'"=23.4.1"'*) continue ;;
    *)
      echo "FAIL non-exact or unknown soroban-sdk pin (third version?): $line"
      fail=1
      ;;
  esac
done < <(grep -rn --include=Cargo.toml 'soroban-sdk *=' "$ROOT_DIR/contracts" "$ROOT_DIR/soroban")

echo "==> policy doc mentions target, owner, and both pins"
DOC="$ROOT_DIR/contracts/SDK_COMPATIBILITY.md"
for needle in "Target \`soroban-sdk\`" "$EXCEPTION_OWNER" "=23.4.1" "=21.7.7" "Owned exception"; do
  if ! grep -q "$needle" "$DOC"; then
    echo "FAIL contracts/SDK_COMPATIBILITY.md missing required policy text: $needle"
    fail=1
  fi
done

if [[ "$fail" -ne 0 ]]; then
  echo "SDK version check FAILED"
  exit 1
fi
echo "SDK version check passed (divergence reported; no silent dual-major)"
