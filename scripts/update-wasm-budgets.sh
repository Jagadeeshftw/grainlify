#!/usr/bin/env bash
set -euo pipefail

# update-wasm-budgets.sh — Refresh baseline_bytes in .github/wasm-budgets.json
# from the currently built artifacts. Run this after an intentional size change,
# then review and commit the diff.
#
# The tolerance and the absolute ceiling are global policy (tolerance_percent /
# absolute_ceiling_bytes) and are not touched here, so growth is still gated
# relative to the refreshed baseline.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${WASM_BUDGETS_ROOT:-$(cd "$SCRIPT_DIR/.." && pwd)}"
BUDGET_FILE="${WASM_BUDGETS_FILE:-${REPO_ROOT}/.github/wasm-budgets.json}"

if [[ ! -f "$BUDGET_FILE" ]]; then
  echo "::error::Budget file not found: $BUDGET_FILE"
  exit 1
fi

if ! command -v jq &>/dev/null; then
  echo "::error::jq is required but not installed."
  exit 1
fi

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Updating wasm size baselines from current build outputs"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

tmp_file=$(mktemp)
cp "$BUDGET_FILE" "$tmp_file"

contracts=$(jq -r '.budgets | keys[]' "$BUDGET_FILE")

for contract in $contracts; do
  base=".budgets.\"$contract\""
  package=$(jq -r "${base}.package" "$BUDGET_FILE")
  target_dir=$(jq -r "${base}.target_dir" "$BUDGET_FILE")
  target=$(jq -r "${base}.target // \"wasm32-unknown-unknown\"" "$BUDGET_FILE")
  artifact=$(jq -r "${base}.artifact // empty" "$BUDGET_FILE")
  old_baseline=$(jq -r "${base}.baseline_bytes" "$BUDGET_FILE")

  if [[ -n "$artifact" ]]; then
    wasm_name="$artifact"
  else
    wasm_name="${package//-/_}.wasm"
  fi
  wasm_path="${REPO_ROOT}/${target_dir}/target/${target}/release/${wasm_name}"

  if [[ ! -f "$wasm_path" ]]; then
    echo "⚠  $contract: wasm not found at $wasm_path — skipping"
    continue
  fi

  actual_bytes=$(stat --printf="%s" "$wasm_path" 2>/dev/null || stat -f%z "$wasm_path" 2>/dev/null)
  # Record the measured size rounded up to the nearest 1 KB. Tolerance and the
  # absolute ceiling are applied on top by check-wasm-budgets.sh.
  new_baseline=$(( (actual_bytes + 1023) / 1024 * 1024 ))

  if [[ "$new_baseline" -ne "$old_baseline" ]]; then
    echo "  $contract: ${old_baseline} → ${new_baseline} bytes (actual: ${actual_bytes})"
    jq "${base}.baseline_bytes = $new_baseline" "$tmp_file" > "${tmp_file}.new" && mv "${tmp_file}.new" "$tmp_file"
  else
    echo "  $contract: unchanged (${old_baseline} bytes, actual: ${actual_bytes})"
  fi
done

cp "$tmp_file" "$BUDGET_FILE"
rm -f "$tmp_file"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Budget file updated: $BUDGET_FILE"
echo "  Review the diff and commit it with the rationale in your PR."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
