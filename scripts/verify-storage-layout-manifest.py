#!/usr/bin/env python3
"""Validate that every declared deployable contract has a reviewed layout snapshot.

For each snapshot that declares a ``type_layout_source`` (or
``type_layout_sources``), this also asserts that the ``type_layouts`` fixture
covers every ``#[contracttype]`` type declared in those source files and that
each recorded field encoding still matches the source. Adding, removing,
reordering or retyping a field therefore fails CI, as does introducing a new
``#[contracttype]`` type without recording its layout.

``--require-soroban`` additionally asserts that every ``#[contracttype]`` type
found under ``soroban/contracts/*/src`` is present in a soroban workspace
snapshot declared in the manifest (#1953).

Regenerate soroban snapshots after intentional layout changes with:
  python3 contracts/scripts/gen_storage_layout_types.py \\
    contracts/storage-layout/soroban-escrow.json \\
    contracts/storage-layout/soroban-program-escrow.json \\
    contracts/storage-layout/soroban-stream.json
"""
from __future__ import annotations

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

REQUIRE_SOROBAN = "--require-soroban" in sys.argv[1:]


def source_rel_list(data: dict) -> list[str]:
    sources = data.get("type_layout_sources")
    if sources is None:
        single = data.get("type_layout_source")
        return [single] if single else []
    return list(sources)


def verify_type_layouts(package, snapshot_path, data):
    """Re-derive contracttype encodings from source and compare to the fixture."""
    sources_rel = source_rel_list(data)
    if not sources_rel:
        return 0

    assert data.get("type_layouts"), (
        f"{package}: type_layout_source requires a non-empty type_layouts fixture"
    )

    parsed: dict[str, dict] = {}
    for source_rel in sources_rel:
        source = root / source_rel
        assert source.is_file(), f"{package}: missing type layout source: {source_rel}"
        for t in parse_contracttype_types(source):
            assert t["type"] not in parsed, (
                f"{package}: duplicate contracttype {t['type']} across sources"
            )
            parsed[t["type"]] = t

    declared = {t["type"]: t for t in data["type_layouts"]}
    assert len(declared) == len(data["type_layouts"]), f"{package}: duplicate type in type_layouts"

    missing = sorted(set(parsed) - set(declared))
    assert not missing, (
        f"{package}: contracttype(s) missing from {snapshot_path.name} type_layouts: {missing}. "
        "Record the type (and bump the storage schema version if it is persisted) by running "
        "contracts/scripts/gen_storage_layout_types.py."
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

if REQUIRE_SOROBAN:
    soroban_entries = [e for e in entries if e["snapshot"].startswith("contracts/storage-layout/soroban-")]
    assert soroban_entries, (
        "--require-soroban: no soroban workspace snapshots declared in "
        "contracts/storage-layout-manifest.json"
    )
    covered: set[str] = set()
    for entry in soroban_entries:
        data = json.loads((root / entry["snapshot"]).read_text())
        for t in data.get("type_layouts", []):
            covered.add(t["type"])
        for source_rel in source_rel_list(data):
            source = root / source_rel
            assert source.is_file(), f"{entry['package']}: missing soroban source {source_rel}"
            for t in parse_contracttype_types(source):
                assert t["type"] in covered or t["type"] in {x["type"] for x in data.get("type_layouts", [])}, (
                    f"{entry['package']}: soroban contracttype {t['type']} is not in any "
                    "soroban storage-layout snapshot"
                )

    # Every #[contracttype] under soroban/contracts must appear in a soroban snapshot.
    soroban_root = root / "soroban" / "contracts"
    if soroban_root.is_dir():
        snapshot_types: set[str] = set()
        for entry in soroban_entries:
            data = json.loads((root / entry["snapshot"]).read_text())
            snapshot_types.update(t["type"] for t in data.get("type_layouts", []))
        for contract_dir in sorted(p for p in soroban_root.iterdir() if p.is_dir()):
            src_dir = contract_dir / "src"
            if not src_dir.is_dir():
                continue
            for rs in sorted(src_dir.glob("*.rs")):
                for t in parse_contracttype_types(rs):
                    assert t["type"] in snapshot_types, (
                        f"soroban contracttype {t['type']} in "
                        f"{rs.relative_to(root)} is not covered by any soroban snapshot. "
                        "Add/regenerate with contracts/scripts/gen_storage_layout_types.py "
                        "and update contracts/storage-layout-manifest.json."
                    )

print(f"validated {len(entries)} deployable contract storage snapshots")
print(f"validated {checked} pinned type encodings across all snapshots")
if REQUIRE_SOROBAN:
    print("validated soroban workspace contracttype coverage")
