#!/usr/bin/env python3
"""
Generate a SHA-256 manifest for wasm artifacts produced by cargo builds.

Usage: python scripts/generate_manifest.py --workspace-root <path> --out <outdir>

Behavior:
- Requires to be run in a git repository (uses short commit id for artifact names).
- Scans for wasm files under <workspace-root>/**/target/wasm32-unknown-unknown/release/*.wasm
- For each wasm: determine crate name by searching upward for Cargo.toml package.name or using parent directory name
- Copies artifacts into OUTDIR as <crate>-<rev>.wasm (overwrites if present)
- Writes OUTDIR/sha256-manifest.txt with lines: <sha256>  <filename>

"""
from __future__ import annotations
import argparse
import hashlib
import os
import shutil
import subprocess
import sys
from pathlib import Path
import re


def run(cmd, cwd=None):
    try:
        out = subprocess.check_output(cmd, cwd=cwd, shell=False)
        return out.decode().strip()
    except Exception as e:
        raise RuntimeError(f"Command failed: {cmd!r}: {e}")


def git_short_rev(repo_root: Path) -> str:
    try:
        out = subprocess.check_output(["git", "rev-parse", "--short", "HEAD"], cwd=str(repo_root))
        return out.decode().strip()
    except Exception:
        raise RuntimeError("Not a git repository or git not available. A git commit is required for deterministic artifact names.")


def find_cargo_package_name(start: Path) -> str | None:
    cur = start
    for _ in range(6):
        toml = cur / "Cargo.toml"
        if toml.exists():
            # read simple package.name
            text = toml.read_text(encoding="utf-8")
            m = re.search(r"(?m)^\s*name\s*=\s*\"([^\"]+)\"", text)
            if m:
                return m.group(1)
        if cur.parent == cur:
            break
        cur = cur.parent
    return None


def sha256_of_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


def main(argv=None) -> int:
    p = argparse.ArgumentParser()
    p.add_argument("--workspace-root", default=".", help="Workspace root to scan for wasm artifacts")
    p.add_argument("--out", default="artifacts", help="Output directory to copy artifacts and write manifest")
    p.add_argument("--manifest-name", default="sha256-manifest.txt", help="Manifest filename to write inside out dir")
    args = p.parse_args(argv)

    workspace_root = Path(args.workspace_root).resolve()
    out_dir = Path(args.out).resolve()

    rev = git_short_rev(workspace_root)

    wasm_glob = list(workspace_root.glob("**/target/wasm32-unknown-unknown/release/*.wasm"))
    if not wasm_glob:
        print("No wasm artifacts found under", workspace_root, file=sys.stderr)
        return 2

    out_dir.mkdir(parents=True, exist_ok=True)

    manifest_lines = []
    for wasm in sorted(wasm_glob):
        # crate name detection
        crate_name = find_cargo_package_name(wasm.parent)
        if crate_name is None:
            crate_name = wasm.parent.name
        dest_name = f"{crate_name}-{rev}.wasm"
        dest_path = out_dir / dest_name
        shutil.copy2(wasm, dest_path)
        sha = sha256_of_file(dest_path)
        manifest_lines.append(f"{sha}  {dest_name}")
        print(f"Added {dest_name} (sha256={sha})")

    manifest_path = out_dir / args.manifest_name
    manifest_path.write_text("\n".join(manifest_lines) + "\n", encoding="utf-8")
    print(f"Wrote manifest: {manifest_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
