#!/usr/bin/env bash
# ==============================================================================
# check-coverage-thresholds.sh — the coverage gate.
# ==============================================================================
# Compares what scripts/run_contract_coverage.sh just measured against the
# per-crate floors recorded in .github/coverage-thresholds.json, and fails if
# any crate came in below its floor.
#
# The floors are set to the level that was actually measured when the gate was
# introduced, so the gate is green on day one and turns red the moment coverage
# is lost. There is no "current coverage" free pass: to land a change that
# legitimately lowers coverage you must edit the floor in the same PR and
# justify it in the description, which puts the drop in front of a reviewer.
#
# A crate can only go quiet in one way: by being listed with
# "status": "blocked" plus a written reason. That is deliberate. It makes the
# set of unmeasured crates visible and reviewable instead of something a
# contributor can widen by accident, and it means a crate that was measurable
# and stops being measurable fails here rather than quietly dropping out of the
# gate. Crates are only ever expected to move blocked -> measured, never the
# other way.
#
# EXIT CODES:
#   0  every measured crate cleared its floor
#   1  a crate fell below its floor, lost its measurement, or the summary is
#      missing/stale
# ==============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CONFIG_FILE="${COVERAGE_CONFIG:-$REPO_ROOT/.github/coverage-thresholds.json}"
OUT_DIR="${COVERAGE_OUT_DIR:-$REPO_ROOT/coverage}"
SUMMARY_FILE="${COVERAGE_SUMMARY:-$OUT_DIR/coverage-summary.json}"

# ── preconditions ──────────────────────────────────────────────────────────
if [[ ! -f "$CONFIG_FILE" ]]; then
  echo "::error::Crate registry not found: $CONFIG_FILE"
  exit 1
fi

if [[ ! -f "$SUMMARY_FILE" ]]; then
  echo "::error::No coverage measurement found at $SUMMARY_FILE"
  echo "::error::Run ./scripts/run_contract_coverage.sh first."
  exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "::error::jq is required but not installed."
  exit 1
fi

# ── colours (disabled in CI for cleaner logs) ──────────────────────────────
if [[ -t 1 ]]; then
  GREEN='\033[0;32m'; RED='\033[0;31m'; YELLOW='\033[0;33m'; BOLD='\033[1m'; RESET='\033[0m'
else
  GREEN=''; RED=''; YELLOW=''; BOLD=''; RESET=''
fi

echo "${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  contract coverage gate"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

fail=0
enforced=0
passed_count=0
blocked_count=0

# Emit one row per registered crate, joined against the measurement.
# `. as $cfg` is captured first: inside to_entries the context is the entry
# itself, so the registry object has to be held by name to stay reachable.
#
# Fields are joined with US (0x1f) rather than tab on purpose. Tab is IFS
# whitespace, so `read` would collapse the runs of empty fields a crate with no
# floor or no measurement produces and shift every later column.
mapfile -t ROWS < <(jq -r --slurpfile summary "$SUMMARY_FILE" '
  . as $cfg
  | $cfg.crates | to_entries[]
  | .key as $name
  | .value as $entry
  | ($summary[0].crates | map(select(.crate == $name)) | .[0]) as $m
  | [
      $name,
      ($entry.status // "measured"),
      (($entry.min_lines_pct // "") | tostring),
      (($m.status // "missing")),
      (($m.lines_pct // "") | tostring),
      (($m.error // "") | tostring)
    ] | join("")' "$CONFIG_FILE")

printf "  %-22s %10s %10s  %s\n" "Crate" "Floor" "Measured" "Result"
printf "  %-22s %10s %10s  %s\n" "──────────────────────" "──────────" "──────────" "──────────"

for row in "${ROWS[@]}"; do
  IFS=$'\x1f' read -r name declared_status floor measured_status measured error <<< "$row"
  [[ -z "$name" ]] && continue

  # Render the measurement to 2dp once; the raw float is unreadable in a table.
  if [[ -n "$measured" ]]; then
    measured_disp="$(awk -v v="$measured" 'BEGIN{printf "%.2f", v}')"
  else
    measured_disp='-'
  fi
  floor_disp="${floor:+${floor}%}"

  case "$measured_status" in
    measured)
      if [[ -z "$floor" || "$floor" == "null" ]]; then
        echo -e "  $(printf '%-22s' "$name") $(printf '%10s' '-') $(printf '%10s' "$measured_disp")  ${RED}NO FLOOR${RESET}"
        echo -e "      ${RED}crate is measurable but has no min_lines_pct — record one${RESET}"
        fail=1
        continue
      fi

      # Compare in integer hundredths so 87.0 vs 87.000001 can't flip the gate.
      measured_x100="$(awk -v v="$measured" 'BEGIN{printf "%d", v*100 + 0.5}')"
      floor_x100="$(awk -v v="$floor" 'BEGIN{printf "%d", v*100 + 0.5}')"

      if (( measured_x100 < floor_x100 )); then
        delta="$(awk -v m="$measured" -v f="$floor" 'BEGIN{printf "%+.2f", m-f}')"
        echo -e "  $(printf '%-22s' "$name") $(printf '%10s' "$floor_disp") $(printf '%10s' "$measured_disp")  ${RED}FAIL${RESET}  ${delta} pts below floor"
        echo -e "      ${RED}::error::${name} line coverage ${measured_disp}% is below the recorded floor ${floor}%.${RESET}"
        echo "      Restore the removed test(s), or if the drop is intended, lower"
        echo "      min_lines_pct for $name in .github/coverage-thresholds.json and"
        echo "      say why in the PR description."
        fail=1
      else
        headroom="$(awk -v m="$measured" -v f="$floor" 'BEGIN{printf "%+.2f", m-f}')"
        echo -e "  $(printf '%-22s' "$name") $(printf '%10s' "$floor_disp") $(printf '%10s' "$measured_disp")  ${GREEN}PASS${RESET}  ${headroom} pts"
        enforced=$((enforced + 1))
        passed_count=$((passed_count + 1))
      fi
      ;;

    blocked)
      # Expected and recorded. Surface it, but do not fail on it.
      echo -e "  $(printf '%-22s' "$name") $(printf '%10s' '-') $(printf '%10s' '-')  ${YELLOW}SKIP${RESET}  declared not measurable"
      blocked_count=$((blocked_count + 1))
      ;;

    error)
      echo -e "  $(printf '%-22s' "$name") $(printf '%10s' '-') $(printf '%10s' '-')  ${RED}ERROR${RESET}"
      echo -e "      ${RED}::error::${name} is declared ${declared_status} with a floor but coverage could not be measured: ${error}${RESET}"
      fail=1
      ;;

    missing|*)
      echo -e "  $(printf '%-22s' "$name") $(printf '%10s' '-') $(printf '%10s' '-')  ${RED}MISSING${RESET}"
      if [[ "$declared_status" == "blocked" ]]; then
        # A blocked crate that suddenly measures is good news, not a failure:
        # someone fixed the blocker without re-baselining. Point it out.
        echo -e "      ${YELLOW}declared blocked, but a measurement exists — re-baseline or set status to measured${RESET}"
        fail=1
      else
        echo -e "      ${RED}::error::${name} has a recorded floor but produced no measurement.${RESET}"
        echo "      A crate cannot silently drop out of the gate. If it is genuinely"
        echo "      unbuildable, set \"status\": \"blocked\" with a reason in"
        echo "      .github/coverage-thresholds.json so the loss is reviewed."
        fail=1
      fi
      ;;
  esac
done

echo "──────────────────────────────────────────────────────────────────────"
printf "  %d/%d measured crates at or above floor" "$passed_count" "$enforced"
echo ""
echo ""

if (( fail != 0 )); then
  echo -e "${RED}  ✗ Coverage gate failed.${RESET}"
  echo ""
  exit 1
fi

if (( blocked_count > 0 )); then
  echo -e "${GREEN}  ✓ Coverage gate passed.${RESET} $blocked_count crate(s) are declared not measurable;"
  echo "    see \"status\" / \"blocked_reason\" in .github/coverage-thresholds.json."
else
  echo -e "${GREEN}  ✓ Coverage gate passed.${RESET}"
fi
echo ""
exit 0
