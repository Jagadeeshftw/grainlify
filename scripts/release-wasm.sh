#!/usr/bin/env bash
# Build wasm artifacts from a clean target and generate a sha256 manifest.
# Usage: ./scripts/release-wasm.sh [--workspace-root PATH] [--out PATH]
set -euo pipefail

WORKSPACE_ROOT="."
OUTDIR="artifacts"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --workspace-root) WORKSPACE_ROOT="$2"; shift 2;;
    --out) OUTDIR="$2"; shift 2;;
    *) echo "Unknown arg: $1"; exit 2;;
  esac
done

# Prefer workspace Cargo.toml in current dir, or grainlify/ subdir
if [[ -f "$WORKSPACE_ROOT/Cargo.toml" ]]; then
  MANIFEST_PATH="$WORKSPACE_ROOT/Cargo.toml"
elif [[ -f "$WORKSPACE_ROOT/grainlify/Cargo.toml" ]]; then
  MANIFEST_PATH="$WORKSPACE_ROOT/grainlify/Cargo.toml"
else
  echo "Could not find workspace Cargo.toml in $WORKSPACE_ROOT or $WORKSPACE_ROOT/grainlify" >&2
  exit 2
fi

# Require git
if ! git -C "$WORKSPACE_ROOT" rev-parse --short HEAD >/dev/null 2>&1; then
  echo "Workspace is not a git repository or git not available" >&2
  exit 2
fi

echo "Cleaning workspace targets..."
( cd "$WORKSPACE_ROOT" && cargo clean )

echo "Building wasm artifacts (release)"
( cd "$WORKSPACE_ROOT" && cargo build --manifest-path "$MANIFEST_PATH" --workspace --target wasm32-unknown-unknown --release )

echo "Generating manifest and copying artifacts to $OUTDIR"
python3 "$WORKSPACE_ROOT/scripts/generate_manifest.py" --workspace-root "$WORKSPACE_ROOT" --out "$OUTDIR" --manifest-name sha256-manifest.txt

echo "Done. Manifest at $OUTDIR/sha256-manifest.txt"
