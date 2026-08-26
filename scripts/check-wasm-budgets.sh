#!/usr/bin/env bash
set -euo pipefail

# check-wasm-budgets.sh — Compare built .wasm sizes against the checked-in
# budget file (.github/wasm-budgets.json).  Fails if any contract exceeds its
# per-contract budget.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BUDGET_FILE="${REPO_ROOT}/.github/wasm-budgets.json"

if [[ ! -f "$BUDGET_FILE" ]]; then
  echo "::error::Budget file not found: $BUDGET_FILE"
  exit 1
fi

# Require jq (available on all GH Actions runners)
if ! command -v jq &>/dev/null; then
  echo "::error::jq is required but not installed."
  exit 1
fi

# ── colours (disabled in CI for cleaner logs) ────────────────────────────
if [[ -t 1 ]]; then
  GREEN='\033[0;32m'; RED='\033[0;31m'; YELLOW='\033[0;33m'; RESET='\033[0m'
else
  GREEN=''; RED=''; YELLOW=''; RESET=''
fi

# ── helpers ──────────────────────────────────────────────────────────────
fail=0
printed_header=0

print_table_header() {
  if [[ $printed_header -eq 0 ]]; then
    printf "\n%-28s %12s %12s %10s %s\n" "Contract" "Actual" "Budget" "Delta" "Status"
    printf "%-28s %12s %12s %10s %s\n" "────────────────────────────" "────────────" "────────────" "──────────" "──────────"
    printed_header=1
  fi
}

# ── iterate over budgets ────────────────────────────────────────────────
contracts=$(jq -r '.budgets | keys[]' "$BUDGET_FILE")
contract_count=$(echo "$contracts" | wc -l)
checked=0
passed=0

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  wasm size budget check  ($contract_count contracts)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

for contract in $contracts; do
  budget_bytes=$(jq -r ".budgets.\"$contract\".budget_bytes" "$BUDGET_FILE")
  package=$(jq -r ".budgets.\"$contract\".package" "$BUDGET_FILE")
  target_dir=$(jq -r ".budgets.\"$contract\".target_dir" "$BUDGET_FILE")
  description=$(jq -r ".budgets.\"$contract\".description" "$BUDGET_FILE")

  # Expected wasm file name (underscores, not hyphens — Rust convention)
  wasm_name="${package//-/_}"
  wasm_path="${REPO_ROOT}/${target_dir}/target/wasm32-unknown-unknown/release/${wasm_name}.wasm"

  if [[ ! -f "$wasm_path" ]]; then
    echo "::warning::wasm not found for '$contract' at $wasm_path — skipping (build first)"
    continue
  fi

  actual_bytes=$(stat --printf="%s" "$wasm_path" 2>/dev/null || stat -f%z "$wasm_path" 2>/dev/null)
  delta=$((actual_bytes - budget_bytes))

  checked=$((checked + 1))

  if [[ $actual_bytes -le $budget_bytes ]]; then
    status="${GREEN}PASS${RESET}"
    passed=$((passed + 1))
  else
    status="${RED}FAIL${RESET}"
    fail=1
  fi

  print_table_header
  printf "%-28s %10s B %10s B %+10d B  %b\n" \
    "$contract" \
    "$actual_bytes" \
    "$budget_bytes" \
    "$delta" \
    "$status"
done

echo ""

# ── summary ─────────────────────────────────────────────────────────────
if [[ $checked -eq 0 ]]; then
  echo "::warning::No wasm files found.  Run 'cargo build --target wasm32-unknown-unknown --release' first."
  exit 0
fi

echo "──────────────────────────────────────────────────────────────────────"
echo "  ${passed}/${checked} contracts within budget"

if [[ $fail -ne 0 ]]; then
  echo ""
  echo -e "${RED}  ✗ One or more contracts exceeded their wasm size budget.${RESET}"
  echo "  To intentionally increase a budget, update .github/wasm-budgets.json"
  echo "  and include the rationale in your PR description."
  echo ""
  exit 1
else
  echo -e "${GREEN}  ✓ All contracts within budget.${RESET}"
  echo ""
  exit 0
fi
