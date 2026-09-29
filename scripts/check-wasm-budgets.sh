#!/usr/bin/env bash
set -euo pipefail

# check-wasm-budgets.sh — Compare built .wasm sizes against the checked-in
# budget file (.github/wasm-budgets.json). Fails when either:
#
#   1. an artifact grows beyond its recorded baseline plus the stated
#      tolerance ("growth"), or
#   2. an artifact crosses the absolute on-chain deployment ceiling
#      ("ceiling"), i.e. Stellar's contractMaxSizeBytes.
#
# The two gates are separate because they answer different questions: the
# baseline catches regression as a contract evolves, the ceiling catches a
# wasm that can no longer be uploaded at all.
#
# Env overrides (used by the regression tests):
#   WASM_BUDGETS_FILE  path to the budget JSON (default <root>/.github/wasm-budgets.json)
#   WASM_BUDGETS_ROOT  repo root used to resolve target_dir (default script parent)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${WASM_BUDGETS_ROOT:-$(cd "$SCRIPT_DIR/.." && pwd)}"
BUDGET_FILE="${WASM_BUDGETS_FILE:-${REPO_ROOT}/.github/wasm-budgets.json}"

if [[ ! -f "$BUDGET_FILE" ]]; then
  echo "::error::Budget file not found: $BUDGET_FILE"
  exit 1
fi

# Require jq (available on all GH Actions runners)
if ! command -v jq &>/dev/null; then
  echo "::error::jq is required but not installed."
  exit 1
fi

# ── colours (disabled when not a tty, e.g. CI logs) ───────────────────────
if [[ -t 1 ]]; then
  GREEN='\033[0;32m'; RED='\033[0;31m'; YELLOW='\033[0;33m'; RESET='\033[0m'
else
  GREEN=''; RED=''; YELLOW=''; RESET=''
fi

# ── global gates ──────────────────────────────────────────────────────────
absolute_ceiling=$(jq -r '.absolute_ceiling_bytes // empty' "$BUDGET_FILE")
tolerance_percent=$(jq -r '.tolerance_percent // 0' "$BUDGET_FILE")

if [[ -n "$absolute_ceiling" && ! "$absolute_ceiling" =~ ^[0-9]+$ ]]; then
  echo "::error::absolute_ceiling_bytes must be an integer, got: $absolute_ceiling"
  exit 1
fi
if [[ ! "$tolerance_percent" =~ ^[0-9]+$ ]]; then
  echo "::error::tolerance_percent must be an integer, got: $tolerance_percent"
  exit 1
fi

# ── helpers ───────────────────────────────────────────────────────────────
fail=0
printed_header=0

print_table_header() {
  if [[ $printed_header -eq 0 ]]; then
    printf "\n%-26s %14s %14s %14s %12s %s\n" \
      "Contract" "Actual" "Baseline" "Growth limit" "Ceiling" "Status"
    printf "%-26s %14s %14s %14s %12s %s\n" \
      "──────────────────────────" "──────────────" "──────────────" "──────────────" "────────────" "──────────"
    printed_header=1
  fi
}

fmt_ceiling() {
  if [[ -n "$absolute_ceiling" ]]; then echo "$absolute_ceiling"; else echo "—"; fi
}

# ── iterate over budgets ────────────────────────────────────────────────
contracts=$(jq -r '.budgets | keys[]' "$BUDGET_FILE")
contract_count=$(printf '%s\n' "$contracts" | wc -l | tr -d ' ')
checked=0
passed=0
skipped=0

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  wasm size budget check  ($contract_count artifacts, tolerance ${tolerance_percent}%, ceiling ${absolute_ceiling:-none})"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

for contract in $contracts; do
  base=".budgets.\"$contract\""
  baseline_bytes=$(jq -r "${base}.baseline_bytes" "$BUDGET_FILE")
  package=$(jq -r "${base}.package" "$BUDGET_FILE")
  target_dir=$(jq -r "${base}.target_dir" "$BUDGET_FILE")
  target=$(jq -r "${base}.target // \"wasm32-unknown-unknown\"" "$BUDGET_FILE")
  artifact=$(jq -r "${base}.artifact // empty" "$BUDGET_FILE")

  if [[ ! "$baseline_bytes" =~ ^[0-9]+$ ]]; then
    echo "::error::$contract: baseline_bytes must be an integer, got: $baseline_bytes"
    fail=1
    continue
  fi

  # Expected wasm file name: explicit artifact, else package name underscored.
  if [[ -n "$artifact" ]]; then
    wasm_name="$artifact"
  else
    wasm_name="${package//-/_}.wasm"
  fi
  wasm_path="${REPO_ROOT}/${target_dir}/target/${target}/release/${wasm_name}"

  if [[ ! -f "$wasm_path" ]]; then
    echo "::warning::wasm not found for '$contract' at $wasm_path — skipping (build first)"
    skipped=$((skipped + 1))
    continue
  fi

  actual_bytes=$(stat --printf="%s" "$wasm_path" 2>/dev/null || stat -f%z "$wasm_path" 2>/dev/null)
  growth_limit=$(( baseline_bytes + baseline_bytes * tolerance_percent / 100 ))

  # The effective limit is the tighter of the growth limit and the ceiling.
  limit=$growth_limit
  if [[ -n "$absolute_ceiling" && "$absolute_ceiling" -lt "$limit" ]]; then
    limit=$absolute_ceiling
  fi

  if [[ -n "$absolute_ceiling" && "$actual_bytes" -gt "$absolute_ceiling" ]]; then
    status="${RED}FAIL (ceiling)${RESET}"
    reason="exceeds absolute deployment ceiling ${absolute_ceiling} B"
    fail=1
  elif [[ "$actual_bytes" -gt "$growth_limit" ]]; then
    status="${RED}FAIL (growth)${RESET}"
    reason="exceeds baseline+tolerance ${growth_limit} B"
    fail=1
  else
    status="${GREEN}PASS${RESET}"
    reason=""
    passed=$((passed + 1))
  fi

  checked=$((checked + 1))
  print_table_header
  printf "%-26s %12s B %12s B %12s B %10s B  %b" \
    "$contract" "$actual_bytes" "$baseline_bytes" "$growth_limit" "$(fmt_ceiling)" "$status"
  if [[ -n "$reason" ]]; then printf "  (%s)" "$reason"; fi
  printf "\n"
done

echo ""

# ── summary ─────────────────────────────────────────────────────────────
# Configuration errors (e.g. a malformed baseline) are hard failures even when
# nothing was built, so check `fail` before the "nothing to do" early exit.
if [[ $fail -ne 0 ]]; then
  echo "──────────────────────────────────────────────────────────────────────"
  echo "  ${passed}/${checked} built artifacts within budget (${skipped} not built)"
  echo ""
  echo -e "${RED}  ✗ One or more deployable wasm artifacts are over budget.${RESET}"
  echo "  To intentionally raise a baseline, update .github/wasm-budgets.json"
  echo "  and include the rationale in your PR description. Crossing the"
  echo "  absolute deployment ceiling is never acceptable: shrink the contract."
  echo ""
  exit 1
fi

if [[ $checked -eq 0 ]]; then
  echo "::warning::No wasm files found. Run the release builds for every workspace first."
  exit 0
fi

echo "──────────────────────────────────────────────────────────────────────"
echo "  ${passed}/${checked} built artifacts within budget (${skipped} not built)"
echo -e "${GREEN}  ✓ All built deployable wasm artifacts within budget.${RESET}"
echo ""
exit 0
