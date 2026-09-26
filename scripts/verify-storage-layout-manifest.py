#!/usr/bin/env python3
"""Validate that every declared deployable contract has a reviewed layout snapshot.

For each snapshot that declares a ``type_layout_source``, this also asserts that
the ``type_layouts`` fixture covers every ``#[contracttype]`` type declared in
that source file and that each recorded field encoding still matches the source.
Adding, removing, reordering or retyping a field therefore fails CI, as does
introducing a new ``#[contracttype]`` type without recording its layout.
"""
import json
import sys
from pathlib import Path

root = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(root / "contracts" / "scripts"))

from storage_layout_parse import (  # noqa: E402
    load_serialization_goldens,
    parse_contracttype_types,
    sha256_hex,
)

manifest_path = root / "contracts/storage-layout-manifest.json"
manifest = json.loads(manifest_path.read_text())
assert manifest["format"] == 1
entries = manifest["contracts"]
assert entries, "storage layout manifest must not be empty"


def verify_type_layouts(package, snapshot_path, data):
    """Re-derive contracttype encodings from source and compare to the fixture."""
    source_rel = data.get("type_layout_source")
    if not source_rel:
        return 0

    assert data.get("type_layouts"), (
        f"{package}: type_layout_source requires a non-empty type_layouts fixture"
    )
    source = root / source_rel
    assert source.is_file(), f"{package}: missing type layout source: {source_rel}"

    parsed = {t["type"]: t for t in parse_contracttype_types(source)}
    declared = {t["type"]: t for t in data["type_layouts"]}
    assert len(declared) == len(data["type_layouts"]), f"{package}: duplicate type in type_layouts"

    missing = sorted(set(parsed) - set(declared))
    assert not missing, (
        f"{package}: contracttype(s) missing from {snapshot_path.name} type_layouts: {missing}. "
        "Record the type (and bump the storage schema version if it is persisted)."
    )
    stale = sorted(set(declared) - set(parsed))
    assert not stale, f"{package}: type_layouts entries have no matching contracttype: {stale}"

    for name, entry in declared.items():
        want = parsed[name]
        assert entry["kind"] == want["kind"], (
            f"{package}: {name} kind changed ({entry['kind']} != {want['kind']})"
        )
        assert entry["encoding"] == want["encoding"], (
            f"{package}: encoding changed for {name}: {entry['encoding']} != {want['encoding']}. "
            "A field addition/removal/reorder/retype must bump the schema version and add a migration note."
        )

    # Every value type written under a persistent key must be covered by the fixture.
    for key in data.get("persistent_keys", []):
        key_type = key["type"]
        if key_type in parsed:
            assert key_type in declared, (
                f"{package}: persistent key {key['name']} value type {key_type} is not in type_layouts"
            )

    goldens_rel = data.get("type_layout_goldens")
    if goldens_rel:
        goldens_path = root / goldens_rel
        assert goldens_path.is_file(), f"{package}: missing golden file: {goldens_rel}"
        goldens = load_serialization_goldens(goldens_path)
        for name, entry in declared.items():
            golden = goldens.get(name)
            if golden is None:
                continue
            assert "xdr_sha256" in entry, (
                f"{package}: {name} has a serialization golden but no xdr_sha256"
            )
            assert entry["xdr_sha256"] == sha256_hex(golden), (
                f"{package}: xdr_sha256 is stale for {name}; "
                "regenerate with contracts/scripts/gen_storage_layout_types.py"
            )
    return len(declared)


checked = 0
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
    checked += verify_type_layouts(entry["package"], snapshot, data)

print(f"validated {len(entries)} deployable contract storage snapshots")
print(f"validated {checked} pinned type encodings across all snapshots")
