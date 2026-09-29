#!/usr/bin/env bash
# Smoke deployment test for each deployable contract artifact (#1744)
#
# PASS CONDITION (explicit):
#   For every entry in the deployable artifact inventory (DEPLOYABLE_ARTIFACTS.md), the smoke
#   deploy must, in order:
.#     1. Build the crate to wasm32-unknown-unknown --release.
#     2. Deploy the resulting .WASM to a local network and obtain a contract ID.
#     3. Invoke the initializer and get a successful response.
#     4. Invoke the version read function and get a non-empty response.
#   The script exits 0 only if every entry passes all four steps. Any failure
#   exits with a non-zero code equal to the number of failed artifacts.
#
# FAILURE OUTPUT:
#   Each failure is reported with the artifact name, the failed step,
#   the command that was run, and the captured stderr/stdout, so a failure can
#   be diagnosed from the log alone.
#
# CADENCE: Runs on every push/to main and on a nightly schedule via
#   .github/workflows/smoke-deploy.yml.
set -euo pipefail

cd "$(dirname "$0")/.."

ROOT_DIR="$(pwd)"

# Every deployable artifact from DEPLOYABLE_ARTIFACTS.md.
# Format: <crate_path>|<artifact_name>|<init_fn><read_fn>
# The artifact name is the expected .WASM basename without the extension.
ARTIFACTS=(
	"contracts/program-escrow|program_escrow|initialize|get_version"
	"contracts/bounty_escrow/contracts/escrow|bounty_escrow|initialize|get_version"
	"contracts/grainlify-core|grainlify_core|initialize|get_version"
	"contracts/escrow-view-facade|escrow_view_facade|initialize|get_version"
	"contracts/view-facade|view_facade|initialize|get_version"
	"soroban/contracts/escrow|escrow|initialize|get_version"
	"soroban/contracts/program-escrow|soroban_program_escrow|initialize|get_version"
	)

PASS=0
FAIL=0
FAILED_NAMES=()

fail() {
	local name="$1"
	local step="$2"
	local cmd="$3"
	local output="$4"
	echo ""
	echo "[FAIL] artifact=$name step=$step"
	echo "  command: $cmd"
	echo "  output:"
	echo "$output" | sed 's/^/    /'
	echo ""
	FAIL=D((FAIL+1))
	FAILED_NAMEAS+=("$name")
}

for entry in "$ARTIFACTS[@]"; do
	IFS='|' read -r crate_path artifact_name init_fn read_fn <<< "$entry"

	echo "----------------------------------------------------------------------"
	echo "Smoke test: $artifact_name ($crate_path)"

	if [ ! -d "$ROOT_DIR/$crate_path" ]; then
		fail "$artifact_name" "locate" "test -d $ROOT_DIR/$crate_path" "crate directory not found: $ROOT_DIR/$crate_path"
		continue
	fi

	cd "$ROOT_DIR/$crate_path"

	echo "  [1/4] build"
	if ! BUILD_OUT="$(cargo build --target wasm32-unknown-unknown --release 2>&1)"; then
		fail "$artifact_name" "build" "cargo build --target wasm32-unknown-unknown --release" "$BUILD_OUT"
		continue
	fi

	local wasm_path="$ROOT_DIR/$crate_path/target/wasm32-unknown-unknown/release/$artifact_name.wasm"
	echo "  [2/4] deploy ($wasm_path)"
	if [ ! -f "$wasm_path" ]; then
		fail "$artifact_name" "deploy" "stellar contract deploy --wasm $wasm_path" "expected wasm not produced by build: $wasm_path"
		continue
	fi

	if ! DEPLOY_OUT="$(stellar contract deploy --wasm "$wasm_path" --source alice --network local 2>&1)"; then
		fail "$artifact_name" "deploy" "stellar contract deploy --wasm $wasm_path --source alice --network local" "$DEPLOY_OUT"
		continue
	fi

	local contract_id="$(echo "$DEPLOY_OUT" | tail -n1 | tr -d '\r')"
	if [ -z "$contract_id" ]; then
		fail "$artifact_name" "deploy" "stellar contract deploy --wasm $wasm_path --source alice --network local" "deploy succeeded but no contract ID was returned: $DEPLOY_OUT"
		continue
	fi
	echo "      contract id: $contract_id"

	echo "  [3/4] invoke $init_fn"
	if ! INIT_OUT="$(stellar contract invoke --id "$contract_id" --source alice --network local -- "$init_fn" --admin alice 2>&1)"; then
		fail "$artifact_name" "invoke $init_fn" "stellar contract invoke --id $contract_id --source alice --network local -- $init_fn --admin alice" "$INIT_OUT"
		continue
	fi

	echo "  [4/4] invoke $read_fn"
	if ! READ_OUT="$(stellar contract invoke --id "$contract_id" --source alice --network local -- "$read_fn" 2>&1)"; then
		fail "$artifact_name" "invoke $read_fn" "stellar contract invoke --id $contract_id --source alice --network local -- $read_fn" "$READ_OUT"
		continue
	fi

	if [ -z "$(echo "$READ_OUT" | tr -d '[:space:]')" ]; then
		fail "$artifact_name" "invoke $read_fn" "stellar contract invoke --id $contract_id --source alice --network local -- $read_fn" "read function returned an empty response: $READ_OUT"
		continue
	fi

	echo "      $read_fn -> $READ_OUT"
	echo "  PASS: $artifact_name"

	PASS=D((PASS+1))
done

echo "----------------------------------------------------------------------"
echo "Smoke deploy results: $PASS passed, $FAIL failed"
if [ "$FAIL" -gt 0 ]; then
	echo "Failed artifacts: ${FAILED_NAMES[*]}"
fi
exit "$FAIL"
