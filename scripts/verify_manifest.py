#!/usr/bin/env python3
"""
Verify artifacts listed in a SHA-256 manifest.

Manifest format: lines of "<sha256>  <filename>" (two spaces between sha and name like sha256sum)
The filename is resolved relative to the manifest directory.

Exit codes:
 - 0: all OK
 - 1: one or more mismatches or missing files
 - 2: manifest missing or malformed
"""
from __future__ import annotations
import argparse
import hashlib
import sys
from pathlib import Path


def sha256_of_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


def verify_manifest(manifest_path: Path) -> int:
    if not manifest_path.exists():
        print(f"Manifest not found: {manifest_path}", file=sys.stderr)
        return 2
    base = manifest_path.parent
    bad = 0
    with manifest_path.open("r", encoding="utf-8") as f:
        for lineno, raw in enumerate(f, start=1):
            s = raw.strip()
            if not s:
                continue
            parts = s.split(None, 1)
            if len(parts) != 2:
                print(f"Malformed manifest line {lineno}: {raw!r}", file=sys.stderr)
                return 2
            expected, name = parts
            # allow leading spaces in filename if split collapsed; name may start with spaces removed
            path = base / name
            if not path.exists():
                print(f"MISSING: {name} (expected sha {expected})")
                bad += 1
                continue
            actual = sha256_of_file(path)
            if actual.lower() != expected.lower():
                print(f"MISMATCH: {name}\n  expected: {expected}\n  actual:   {actual}")
                bad += 1
            else:
                print(f"OK: {name}")
    if bad:
        print(f"Verification failed: {bad} problem(s)")
        return 1
    print("All checksums match")
    return 0


def main(argv=None) -> int:
    p = argparse.ArgumentParser()
    p.add_argument("manifest", nargs="?", default="artifacts/sha256-manifest.txt", help="Path to manifest file")
    args = p.parse_args(argv)
    return verify_manifest(Path(args.manifest))


if __name__ == "__main__":
    raise SystemExit(main())
