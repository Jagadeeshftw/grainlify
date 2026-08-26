#!/usr/bin/env bash
set -euo pipefail

# update-wasm-budgets.sh — Rebuild all contracts and update the budget file
# with the current sizes (+ 5 % headroom).  Intended to be run locally before
# committing an intentional size increase.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BUDGET_FILE="${REPO_ROOT}/.github/wasm-budgets.json"

if [[ ! -f "$BUDGET_FILE" ]]; then
  echo "::error::Budget file not found: $BUDGET_FILE"
  exit 1
fi

if ! command -v jq &>/dev/null; then
  echo "::error::jq is required but not installed."
  exit 1
fi

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Updating wasm size budgets from current build outputs"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Create a temp file for the updated budgets
tmp_file=$(mktemp)
cp "$BUDGET_FILE" "$tmp_file"

contracts=$(jq -r '.budgets | keys[]' "$BUDGET_FILE")

for contract in $contracts; do
  package=$(jq -r ".budgets.\"$contract\".package" "$BUDGET_FILE")
  target_dir=$(jq -r ".budgets.\"$contract\".target_dir" "$BUDGET_FILE")
  old_budget=$(jq -r ".budgets.\"$contract\".budget_bytes" "$BUDGET_FILE")

  wasm_name="${package//-/_}"
  wasm_path="${REPO_ROOT}/${target_dir}/target/wasm32-unknown-unknown/release/${wasm_name}.wasm"

  if [[ ! -f "$wasm_path" ]]; then
    echo "⚠  $contract: wasm not found at $wasm_path — skipping"
    continue
  fi

  actual_bytes=$(stat --printf="%s" "$wasm_path" 2>/dev/null || stat -f%z "$wasm_path" 2>/dev/null)
  # Add 5 % headroom, minimum 1 KB
  new_budget=$(( actual_bytes + actual_bytes / 20 + 1024 ))
  # Round up to nearest 1 KB
  new_budget=$(( (new_budget + 1023) / 1024 * 1024 ))

  delta=$((new_budget - old_budget))

  if [[ $new_budget -ne $old_budget ]]; then
    echo "  $contract: ${old_budget} → ${new_budget} bytes (actual: ${actual_bytes}, delta: +${delta})"
    jq ".budgets.\"$contract\".budget_bytes = $new_budget" "$tmp_file" > "${tmp_file}.new" && mv "${tmp_file}.new" "$tmp_file"
  else
    echo "  $contract: unchanged (${old_budget} bytes, actual: ${actual_bytes})"
  fi
done

cp "$tmp_file" "$BUDGET_FILE"
rm -f "$tmp_file"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Budget file updated: $BUDGET_FILE"
echo "  Review the diff and commit."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
