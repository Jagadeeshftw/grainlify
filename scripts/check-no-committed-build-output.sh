#!/usr/bin/env bash
# Fail if Cargo/Soroban target* build output is tracked in git.
# Used by CI (.github/workflows/no-committed-build-output.yml) and for local checks.
set -euo pipefail

mapfile -t tracked < <(git ls-files | grep -E '(^|/)target(-[^/]*|_([^/]*))?/' || true)

if [ "${#tracked[@]}" -eq 0 ] || [ -z "${tracked[0]:-}" ]; then
  echo "OK: no target* build output is tracked."
  exit 0
fi

echo "ERROR: tracked target* build paths:" >&2
printf '%s\n' "${tracked[@]}" >&2
echo "Remove them and keep target* gitignored." >&2
exit 1
