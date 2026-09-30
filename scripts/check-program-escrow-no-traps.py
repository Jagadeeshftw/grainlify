#!/usr/bin/env python3
"""Fail CI when deployable program-escrow source introduces unwrap()/panic! traps.

Scans every deployable `.rs` file under BOTH program-escrow implementations:

  • contracts/program-escrow/src/   (Anchor / Solana workspace)
  • soroban/contracts/program-escrow/src/   (Soroban workspace)

and reports every `.unwrap()` or `panic!` site outside `#[cfg(test)]` modules
(test-only chaos hooks remain exempt).  Typed failures must use
`panic_with_error!` / `Result` + `ContractError`.

A newly introduced trap in *either* tree now fails CI.

Usage:
  python3 scripts/check-program-escrow-no-traps.py
  python3 scripts/check-program-escrow-no-traps.py \\
      --fixture scripts/tests/fixtures/program_escrow_unwrap_trap.rs
  python3 scripts/check-program-escrow-no-traps.py \\
      --fixture scripts/tests/fixtures/soroban_program_escrow_unwrap_trap.rs \\
      --expect-fail

Exit codes:
  0 — clean (or fixture correctly detected traps when --expect-fail is set)
  1 — traps found in target (or fixture did not fail when expected)
"""
from __future__ import annotations
import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# Both deployable source trees are scanned by default.
DEFAULT_TARGETS: list[Path] = [
    ROOT / "contracts" / "program-escrow" / "src",
    ROOT / "soroban" / "contracts" / "program-escrow" / "src",
]

TRAP_UNWRAP = re.compile(r"\.unwrap\s*\(")
TRAP_PANIC  = re.compile(r"(?<![A-Za-z0-9_])panic!\s*\(")


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


def scan_tree(src_dir: Path) -> list[tuple[Path, int, str, str]]:
    """Scan all non-test .rs files under *src_dir* and return (file, line, kind, snippet)."""
    hits: list[tuple[Path, int, str, str]] = []
    if not src_dir.is_dir():
        # Not an error — the tree may not exist in every checkout.
        return hits
    for rs_file in sorted(src_dir.rglob("*.rs")):
        # Skip files whose name starts with "test" — test helper modules live
        # under src/ in some crates and must remain exempt.
        if rs_file.name.startswith("test") or "test_" in rs_file.name:
            continue
        for line_no, kind, snippet in find_traps(rs_file.read_text()):
            hits.append((rs_file, line_no, kind, snippet))
    return hits


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "path",
        nargs="?",
        type=Path,
        default=None,
        help=(
            "Single Rust source file to scan (default: all .rs files under "
            "contracts/program-escrow/src/ AND soroban/contracts/program-escrow/src/)"
        ),
    )
    ap.add_argument(
        "--fixture",
        type=Path,
        help="Scan this fixture instead of the default targets",
    )
    ap.add_argument(
        "--expect-fail",
        action="store_true",
        help="Invert exit status: succeed only when traps are detected (fixture validation)",
    )
    args = ap.parse_args()

    # ── Single-file mode (backward-compat / fixture) ─────────────────────────
    if args.fixture or args.path:
        target: Path = args.fixture if args.fixture else args.path
        if not target.is_file():
            print(f"error: file not found: {target}", file=sys.stderr)
            return 1
        hits_raw = find_traps(target.read_text())
        if hits_raw:
            print(f"FOUND {len(hits_raw)} unwrap()/panic! trap(s) in {target}:")
            for line_no, kind, snippet in hits_raw:
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

    # ── Multi-tree mode (default, covers both workspaces) ────────────────────
    all_hits: list[tuple[Path, int, str, str]] = []
    missing_trees: list[Path] = []

    for src_dir in DEFAULT_TARGETS:
        if not src_dir.is_dir():
            missing_trees.append(src_dir)
            continue
        tree_hits = scan_tree(src_dir)
        all_hits.extend(tree_hits)

    if missing_trees:
        for t in missing_trees:
            print(f"warning: source tree not found, skipping: {t}", file=sys.stderr)

    # Require at least one tree to exist so a misconfigured checkout fails loudly.
    trees_present = [t for t in DEFAULT_TARGETS if t.is_dir()]
    if not trees_present:
        print(
            "error: neither program-escrow source tree was found. "
            "Expected at least one of:\n"
            + "\n".join(f"  {t}" for t in DEFAULT_TARGETS),
            file=sys.stderr,
        )
        return 1

    if all_hits:
        print(f"FOUND {len(all_hits)} unwrap()/panic! trap(s) across both program-escrow trees:")
        for rs_file, line_no, kind, snippet in all_hits:
            rel = rs_file.relative_to(ROOT)
            print(f"  {rel}  L{line_no}: [{kind}] {snippet}")
        print(
            "\nDeployable source must return typed ContractError via panic_with_error! "
            "or Result — see contracts/program-escrow/ERROR_CODES.md",
            file=sys.stderr,
        )
        return 1

    scanned = [str(t.relative_to(ROOT)) for t in trees_present]
    print(f"OK: no unwrap()/panic! traps in deployable source (scanned: {', '.join(scanned)})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
