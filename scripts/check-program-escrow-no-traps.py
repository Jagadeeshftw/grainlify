#!/usr/bin/env python3
"""Fail CI when deployable program-escrow source introduces unwrap()/panic! traps.

Scans `contracts/program-escrow/src/lib.rs` and reports every `.unwrap()` or
`panic!` site outside `#[cfg(test)]` modules (test-only chaos hooks remain
exempt). Typed failures must use `panic_with_error!` / `Result` + `ContractError`.

Usage:
  python3 scripts/check-program-escrow-no-traps.py
  python3 scripts/check-program-escrow-no-traps.py --fixture scripts/tests/fixtures/program_escrow_unwrap_trap.rs

Exit codes:
  0 — clean (or fixture correctly detected traps when --fixture is used with --expect-fail)
  1 — traps found in target (or fixture did not fail when expected)
"""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_TARGET = ROOT / "contracts" / "program-escrow" / "src" / "lib.rs"

TRAP_UNWRAP = re.compile(r"\.unwrap\s*\(")
TRAP_PANIC = re.compile(r"(?<![A-Za-z0-9_])panic!\s*\(")


def cfg_test_exempt_lines(lines: list[str]) -> set[int]:
    """Return 0-based line indices belonging to #[cfg(test)] modules."""
    exempt: set[int] = set()
    i = 0
    n = len(lines)
    while i < n:
        if "#[cfg(test)]" in lines[i]:
            for j in range(i, min(i + 8, n)):
                if re.match(r"\s*(pub\s+)?mod\s+\w+", lines[j]):
                    start = j
                    while start < n and "{" not in lines[start]:
                        start += 1
                    if start >= n:
                        break
                    depth = 0
                    for k in range(start, n):
                        depth += lines[k].count("{") - lines[k].count("}")
                        for t in range(i, k + 1):
                            exempt.add(t)
                        if depth == 0 and k > start:
                            break
                    break
        i += 1
    return exempt


def find_traps(source: str) -> list[tuple[int, str, str]]:
    lines = source.splitlines()
    exempt = cfg_test_exempt_lines(lines)
    hits: list[tuple[int, str, str]] = []
    for idx, line in enumerate(lines):
        if idx in exempt:
            continue
        stripped = line.strip()
        if stripped.startswith("//"):
            continue
        if TRAP_UNWRAP.search(line):
            hits.append((idx + 1, "unwrap", stripped[:160]))
        if TRAP_PANIC.search(line) and "panic_with_error" not in line:
            hits.append((idx + 1, "panic", stripped[:160]))
    return hits


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "path",
        nargs="?",
        type=Path,
        default=DEFAULT_TARGET,
        help="Rust source to scan (default: program-escrow lib.rs)",
    )
    ap.add_argument(
        "--fixture",
        type=Path,
        help="Scan this fixture instead of the default target",
    )
    ap.add_argument(
        "--expect-fail",
        action="store_true",
        help="Invert exit status: succeed only when traps are detected (fixture validation)",
    )
    args = ap.parse_args()
    target: Path = args.fixture if args.fixture else args.path
    if not target.is_file():
        print(f"error: file not found: {target}", file=sys.stderr)
        return 1
    hits = find_traps(target.read_text())
    if hits:
        print(f"FOUND {len(hits)} unwrap()/panic! trap(s) in {target}:")
        for line_no, kind, snippet in hits:
            print(f"  L{line_no}: [{kind}] {snippet}")
        if args.expect_fail:
            print("OK: fixture correctly failed the no-trap check.")
            return 0
        print(
            "Deployable source must return typed ContractError via panic_with_error! "
            "or Result — see contracts/program-escrow/ERROR_CODES.md",
            file=sys.stderr,
        )
        return 1
    if args.expect_fail:
        print("error: expected traps in fixture but found none", file=sys.stderr)
        return 1
    print(f"OK: no unwrap()/panic! traps in deployable source ({target})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
