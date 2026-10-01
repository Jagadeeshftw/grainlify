#!/usr/bin/env python3
"""Regenerate the ``type_layouts`` fixture of a contract storage-layout snapshot.

Usage:
  python3 contracts/scripts/gen_storage_layout_types.py [snapshot.json ...]

Defaults to ``contracts/storage-layout/program-escrow.json``. The snapshot must
already declare ``type_layout_source`` (or ``type_layout_sources``) — the Rust
file(s) whose ``#[contracttype]`` layouts are pinned — and should declare
``type_layout_goldens`` to link the fixture to the serialization golden file.

Regenerating soroban workspace snapshots (documented command for #1953):
  python3 contracts/scripts/gen_storage_layout_types.py \\
    contracts/storage-layout/soroban-escrow.json \\
    contracts/storage-layout/soroban-program-escrow.json \\
    contracts/storage-layout/soroban-stream.json
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

from storage_layout_parse import (
    load_serialization_goldens,
    parse_contracttype_types,
    sha256_hex,
)

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_SNAPSHOT = ROOT / "contracts" / "storage-layout" / "program-escrow.json"


def _source_paths(snapshot: dict) -> list[Path]:
    sources = snapshot.get("type_layout_sources")
    if sources is None:
        single = snapshot.get("type_layout_source")
        sources = [single] if single else []
    return [ROOT / rel for rel in sources]


def regenerate(snapshot_path: Path) -> int:
    snapshot = json.loads(snapshot_path.read_text())
    source_paths = _source_paths(snapshot)
    if not source_paths and snapshot.get("type_layouts"):
        raise SystemExit(
            f"{snapshot_path}: type_layouts present but no type_layout_source(s)"
        )

    types: list[dict] = []
    seen: set[str] = set()
    for source in source_paths:
        if not source.is_file():
            raise SystemExit(f"{snapshot_path}: missing type layout source: {source}")
        for entry in parse_contracttype_types(source):
            if entry["type"] in seen:
                raise SystemExit(
                    f"{snapshot_path}: duplicate contracttype {entry['type']} across sources"
                )
            seen.add(entry["type"])
            types.append(entry)

    goldens_rel = snapshot.get("type_layout_goldens")
    goldens = load_serialization_goldens(ROOT / goldens_rel) if goldens_rel else {}
    for entry in types:
        golden = goldens.get(entry["type"])
        if golden:
            entry["xdr_sha256"] = sha256_hex(golden)

    snapshot["type_layouts"] = types
    snapshot_path.write_text(json.dumps(snapshot, indent=2) + "\n")
    return len(types)


def main() -> None:
    targets = [Path(arg) for arg in sys.argv[1:]] or [DEFAULT_SNAPSHOT]
    for target in targets:
        count = regenerate(target)
        print(f"{target}: wrote {count} type layouts")


if __name__ == "__main__":
    main()
