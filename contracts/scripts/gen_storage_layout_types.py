#!/usr/bin/env python3
"""Regenerate the ``type_layouts`` fixture of a contract storage-layout snapshot.

Usage:
  python3 contracts/scripts/gen_storage_layout_types.py [snapshot.json ...]

Defaults to ``contracts/storage-layout/program-escrow.json``. The snapshot must
already declare ``type_layout_source`` (the Rust file whose ``#[contracttype]``
layouts are pinned) and should declare ``type_layout_goldens`` to link the
fixture to the serialization golden file.
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


def regenerate(snapshot_path: Path) -> int:
    snapshot = json.loads(snapshot_path.read_text())
    source_rel = snapshot.get("type_layout_source")
    if not source_rel:
        raise SystemExit(f"{snapshot_path}: missing type_layout_source")

    types = parse_contracttype_types(ROOT / source_rel)

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
        print(f"{target.relative_to(ROOT)}: wrote {count} type layouts")


if __name__ == "__main__":
    main()
