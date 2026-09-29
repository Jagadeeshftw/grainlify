#!/usr/bin/env bash
# ==============================================================================
# run_contract_coverage.sh — measure line coverage for every contract crate.
# ==============================================================================
# For each contract crate registered in .github/coverage-thresholds.json this
# runs the crate's own test suite under LLVM source-based coverage and records
# how many instrumentable lines the tests actually reach.
#
# The crate registry (.github/coverage-thresholds.json) is the single source of
# truth: this script reads it to know *what* to measure, and
# check-coverage-thresholds.sh reads the same file to know *what floor* each
# measurement must clear. Adding a contract crate therefore means adding one
# entry there, not editing three places.
#
# Output (default: coverage/):
#   coverage/<crate>/lcov.info   machine-readable LCOV
#   coverage/<crate>/coverage.json  machine-readable summary (codecov format)
#   coverage/<crate>/html/index.html  browsable line-by-line report
#   coverage/<crate>/summary.txt  the per-file text table
#   coverage/<crate>/run.log     full test log for the crate
#   coverage/coverage-summary.json  aggregated line coverage, consumed by the gate
#
# This script MEASURES; it deliberately does not decide pass/fail. Keeping the
# two apart means one run always produces the full picture plus every artifact,
# and check-coverage-thresholds.sh then renders the verdict. That is also why a
# crate that fails to build cannot take the rest of the report down with it.
#
# USAGE:
#   ./scripts/run_contract_coverage.sh                 # every registered crate
#   ./scripts/run_contract_coverage.sh bounty-escrow   # one crate (repeatable)
#
# ENVIRONMENT:
#   COVERAGE_OUT_DIR   where reports land           (default: <repo>/coverage)
#   COVERAGE_CLEAN     1 = wipe prior profiles first (default: 1, see below)
#   COVERAGE_MERGE     1 = keep other crates' results (default: 1, see below)
#
# WHY COVERAGE_CLEAN DEFAULTS TO 1:
#   LLVM coverage profiles accumulate. If a stale .profdata from an earlier run
#   survives, lines hit by a test that has since been deleted still count as
#   covered, and the gate would happily pass a suite that no longer exists.
#   That is exactly the regression this gate exists to catch, so the default is
#   a clean measurement. Only the profile data is discarded, not the compiled
#   instrumented objects, so this stays cheap. Set COVERAGE_CLEAN=0 to skip it.
#
# WHY --ignore-run-fail:
#   Coverage measurement and test correctness are separate concerns. Red tests
#   are the job of the `build-test` job in contracts-ci.yml. Here we need the
#   profile data from every test that *does* run, so a pre-existing unrelated
#   failure cannot stop the report from being produced. The number of failing
#   tests is captured in run.log and surfaced in the CI job summary so it is
#   never hidden.
# ==============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CONFIG_FILE="${COVERAGE_CONFIG:-$REPO_ROOT/.github/coverage-thresholds.json}"
OUT_DIR="${COVERAGE_OUT_DIR:-$REPO_ROOT/coverage}"
COVERAGE_CLEAN="${COVERAGE_CLEAN:-1}"

RESULTS_FILE="$(mktemp)"
trap 'rm -f "$RESULTS_FILE"' EXIT

# ── preconditions ──────────────────────────────────────────────────────────
if [[ ! -f "$CONFIG_FILE" ]]; then
  echo "::error::Crate registry not found: $CONFIG_FILE"
  exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "::error::jq is required but not installed."
  exit 1
fi

if ! cargo llvm-cov --version >/dev/null 2>&1; then
  cat >&2 <<'EOF'
::error::cargo-llvm-cov is not available.

Install it, then add the llvm-tools-preview component it needs:
    rustup component add llvm-tools-preview
    cargo install cargo-llvm-cov
EOF
  exit 1
fi

mkdir -p "$OUT_DIR"

# ── colours (disabled in CI for cleaner logs) ──────────────────────────────
if [[ -t 1 ]]; then
  GREEN='\033[0;32m'; RED='\033[0;31m'; YELLOW='\033[0;33m'; BOLD='\033[1m'; RESET='\033[0m'
else
  GREEN=''; RED=''; YELLOW=''; BOLD=''; RESET=''
fi

# Crates named on the command line, else every registered crate.
SELECTED=("$@")
in_selection() {
  [[ ${#SELECTED[@]} -eq 0 ]] && return 0
  local candidate="$1" pick
  for pick in "${SELECTED[@]}"; do
    [[ "$pick" == "$candidate" ]] && return 0
  done
  return 1
}

echo "${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  contract coverage measurement"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

while IFS=$'\t' read -r crate manifest status reason; do
  [[ -z "$crate" ]] && continue
  in_selection "$crate" || continue

  manifest_path="$REPO_ROOT/$manifest"
  crate_dir="$(dirname "$manifest_path")"
  crate_out="$OUT_DIR/$crate"

  if [[ "$status" == "blocked" ]]; then
    echo -e "\n${YELLOW}━━━ $crate: skipped (not measurable)${RESET}"
    echo "    ${reason:-no reason recorded}"
    jq -n --arg c "$crate" --arg m "$manifest" --arg r "${reason:-}" \
      '{crate:$c, manifest:$m, status:"blocked", reason:$r,
        lines_pct:null, covered_lines:null, total_lines:null}' >> "$RESULTS_FILE"
    continue
  fi

  if [[ ! -f "$manifest_path" ]]; then
    echo -e "\n${RED}━━━ $crate: FAILED — manifest not found: $manifest${RESET}"
    jq -n --arg c "$crate" --arg m "$manifest" \
      '{crate:$c, manifest:$m, status:"error",
        error:("manifest not found: "+$m), reason:"",
        lines_pct:null, covered_lines:null, total_lines:null}' >> "$RESULTS_FILE"
    continue
  fi

  mkdir -p "$crate_out"
  echo -e "\n${BOLD}━━━ $crate${RESET}  (${manifest})"

  # Wipe stale profiles so a deleted test suite cannot keep its lines "covered".
  # Only the .profdata files are removed, not the instrumented build output, so
  # the expensive part of the run still comes from the Cargo cache.
  if [[ "$COVERAGE_CLEAN" == "1" ]]; then
    if [[ -d "$crate_dir/target/llvm-cov-target" ]]; then
      find "$crate_dir/target/llvm-cov-target" -name '*.profdata' -delete 2>/dev/null || true
    else
      (cd "$crate_dir" && cargo llvm-cov clean --workspace >/dev/null 2>&1) || true
    fi
  fi

  # One instrumented test run. --ignore-run-fail keeps a pre-existing red test
  # from suppressing the report; --lcov gives us the canonical machine format.
  run_log="$crate_out/run.log"
  (cd "$crate_dir" && cargo llvm-cov \
      --workspace \
      --lcov \
      --ignore-run-fail \
      --output-path "$crate_out/lcov.info") > "$run_log" 2>&1
  run_status=$?

  if [[ ! -s "$crate_out/lcov.info" ]]; then
    echo -e "${RED}    ✗ coverage not produced (cargo llvm-cov exit ${run_status})${RESET}"
    echo "    last lines of $run_log:"
    tail -n 15 "$run_log" 2>/dev/null | sed 's/^/      /'
    jq -n --arg c "$crate" --arg m "$manifest" --arg e "cargo llvm-cov exited ${run_status} without producing a report" \
      '{crate:$c, manifest:$m, status:"error", error:$e, reason:"",
        lines_pct:null, covered_lines:null, total_lines:null}' >> "$RESULTS_FILE"
    continue
  fi

  # Everything below reuses the profile data from that single run, so the tests
  # are not re-executed for each report format.
  (cd "$crate_dir" && cargo llvm-cov report --summary-only) \
    > "$crate_out/summary.txt" 2>&1
  (cd "$crate_dir" && cargo llvm-cov report --json \
      --output-path "$crate_out/coverage.json") >/dev/null 2>&1
  # llvm-cov always creates its own `html/` inside the directory it is given,
  # so the parent is passed to land the report at <crate>/html/index.html.
  (cd "$crate_dir" && cargo llvm-cov report --html \
      --output-dir "$crate_out") >/dev/null 2>&1

  # totals live at .data[0].totals.lines; `covered` is hit lines, `count` total.
  if [[ -s "$crate_out/coverage.json" ]]; then
    totals="$(jq -r '[.data[0].totals.lines.count, .data[0].totals.lines.covered,
                     .data[0].totals.lines.percent] | @tsv' "$crate_out/coverage.json" 2>/dev/null)"
  else
    totals=""
  fi

  if [[ -z "$totals" ]]; then
    echo -e "${RED}    ✗ could not read line totals from coverage.json${RESET}"
    jq -n --arg c "$crate" --arg m "$manifest" \
      '{crate:$c, manifest:$m, status:"error",
        error:"coverage.json missing or unparseable", reason:"",
        lines_pct:null, covered_lines:null, total_lines:null}' >> "$RESULTS_FILE"
    continue
  fi

  total_lines="$(cut -f1 <<< "$totals")"
  covered_lines="$(cut -f2 <<< "$totals")"
  lines_pct="$(cut -f3 <<< "$totals")"

  # Count tests from every `test result:` line — a binary whose suite is red
  # still reports how much of it passed, and those numbers belong in the summary
  # so a pre-existing failure is visible in the artifact rather than hidden.
  failing="$(grep -cE '^test result: FAILED' "$run_log" 2>/dev/null || true)"
  passed="$(awk '/^test result:/ {
                   for (i = 1; i <= NF; i++) if ($i == "passed;") { gsub(/;/, "", $(i - 1)); print $(i - 1) }
                 }' "$run_log" 2>/dev/null | paste -sd+ - | bc 2>/dev/null || true)"
  failed="$(awk '/^test result:/ {
                   for (i = 1; i <= NF; i++) if ($i == "failed;") { gsub(/;/, "", $(i - 1)); print $(i - 1) }
                 }' "$run_log" 2>/dev/null | paste -sd+ - | bc 2>/dev/null || true)"
  [[ -z "$failing" ]] && failing=0
  [[ -z "$passed" ]] && passed=0
  [[ -z "$failed" ]] && failed=0

  echo -e "${GREEN}    ✓ lines ${lines_pct}%  (${covered_lines}/${total_lines})${RESET}"
  echo "    tests: ${passed} passed, ${failed} failed (pre-existing failures do not block the report)"

  jq -n --arg c "$crate" --arg m "$manifest" \
        --argjson pct "$lines_pct" \
        --argjson cov "$covered_lines" \
        --argjson tot "$total_lines" \
        --argjson passed "$passed" \
        --argjson failed "$failed" \
      '{crate:$c, manifest:$m, status:"measured", reason:"",
        lines_pct:$pct, covered_lines:$cov, total_lines:$tot,
        tests_passed:$passed, tests_failed:$failed}' >> "$RESULTS_FILE"
done < <(jq -r '.crates | to_entries[] |
           [.key, .value.manifest, (.value.status // "measured"),
            (.value.blocked_reason // "")] | @tsv' "$CONFIG_FILE")

# ── aggregate ──────────────────────────────────────────────────────────────
# Merge rather than overwrite. Re-measuring one crate after a change
# (`run_contract_coverage.sh bounty-escrow`) must not erase the other crates'
# results, or the gate would report them as missing for a reason that has
# nothing to do with their coverage. Newer results win on a crate collision.
SUMMARY_FILE="$OUT_DIR/coverage-summary.json"
NEW_CRATES="$(mktemp)"
trap 'rm -f "$RESULTS_FILE" "$NEW_CRATES"' EXIT

jq -s '.' "$RESULTS_FILE" > "$NEW_CRATES"

OLD_CRATES="$(mktemp)"
if [[ -f "$SUMMARY_FILE" ]] && [[ "${COVERAGE_MERGE:-1}" == "1" ]]; then
  # `.[0].crates` — the stored summary is the whole {generated_by, crates}
  # object, so it has to be unwrapped to just the crate list before merging.
  jq -s '.[0].crates // []' "$SUMMARY_FILE" > "$OLD_CRATES"
else
  echo '[]' > "$OLD_CRATES"
fi

jq -n --slurpfile old "$OLD_CRATES" --slurpfile new "$NEW_CRATES" '
  ($old[0] | map({(.crate): .}) | add // {}) as $o
  | ($new[0] | map({(.crate): .}) | add // {}) as $n
  | { generated_by: "scripts/run_contract_coverage.sh",
      crates: ($o + $n | [.[]] | sort_by(.crate)) }' > "$SUMMARY_FILE"
rm -f "$OLD_CRATES"

measured=$(jq '[.crates[] | select(.status == "measured")] | length' "$SUMMARY_FILE")
blocked=$(jq '[.crates[] | select(.status == "blocked")] | length' "$SUMMARY_FILE")
errored=$(jq '[.crates[] | select(.status == "error")] | length' "$SUMMARY_FILE")

echo ""
echo "──────────────────────────────────────────────────────────────────────"
printf "  %-22s %10s %14s  %s\n" "Crate" "Lines" "Covered" "Status"
printf "  %-22s %10s %14s  %s\n" "──────────────────────" "──────────" "──────────────" "──────"
jq -r '.crates[] |
       [.crate,
        (if .total_lines == null then "-" else (.total_lines | tostring) end),
        (if .lines_pct == null then "-" else ((.lines_pct * 100 | round) / 100 | tostring) + "%" end),
        (if .status == "measured" then "measured"
         elif .status == "blocked" then "not measurable"
         else "error" end)] | @tsv' "$SUMMARY_FILE" |
while IFS=$'\t' read -r name lines cover state; do
  case "$state" in
    measured) printf "  %-22s %10s %14s  %bPASS%b\n" "$name" "$lines" "$cover" "$GREEN" "$RESET" ;;
    error)    printf "  %-22s %10s %14s  %bERROR%b\n" "$name" "$lines" "$cover" "$RED" "$RESET" ;;
    *)        printf "  %-22s %10s %14s  %bSKIP%b\n" "$name" "$lines" "$cover" "$YELLOW" "$RESET" ;;
  esac
done
echo "──────────────────────────────────────────────────────────────────────"
printf "  %d measured, %d not measurable, %d errored\n" "$measured" "$blocked" "$errored"
echo ""
echo "  reports:  $OUT_DIR"
echo "  summary:  $SUMMARY_FILE"
echo ""

exit 0
