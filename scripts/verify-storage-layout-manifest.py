#!/usr/bin/env python3
"""Validate that every declared deployable contract has a reviewed layout snapshot."""
import json
from pathlib import Path

root = Path(__file__).resolve().parents[1]
manifest_path = root / "contracts/storage-layout-manifest.json"
manifest = json.loads(manifest_path.read_text())
assert manifest["format"] == 1
entries = manifest["contracts"]
assert entries, "storage layout manifest must not be empty"

for entry in entries:
    cargo = root / entry["manifest"]
    snapshot = root / entry["snapshot"]
    assert cargo.is_file(), f"missing Cargo manifest: {entry['manifest']}"
    assert snapshot.is_file(), f"missing storage snapshot: {entry['snapshot']}"
    data = json.loads(snapshot.read_text())
    assert data["format"] == 1
    assert data["package"] == entry["package"]
    assert isinstance(data["persistent_keys"], list)
    names = [key["name"] for key in data["persistent_keys"]]
    assert len(names) == len(set(names)), f"duplicate storage key in {entry['package']}"
    for key in data["persistent_keys"]:
        assert set(key) == {"name", "type"}, f"storage keys must pin name and type: {entry['package']}"

print(f"validated {len(entries)} deployable contract storage snapshots")
