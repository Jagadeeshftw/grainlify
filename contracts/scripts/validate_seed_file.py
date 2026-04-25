#!/usr/bin/env python3
"""
Seed + manifest validation harness (dependency-free).

Why this exists:
- The repo already provides JSON Schema validation via AJV (see validate-manifests.{sh,js}).
- This script adds a lightweight, dependency-free sanity layer that validates:
  1) All `contracts/*-manifest.json` files have required top-level fields and sane values.
  2) Any deployment seed files under `contracts/**/deployments/*.json` follow a consistent shape.

This script is intentionally conservative about what it *fails* on:
- It validates structure and basic types.
- It does NOT attempt to validate Soroban StrKey formats or cross-check Rust entrypoint names,
  because those checks are environment- and refactor-sensitive and would create noisy failures.
"""

from __future__ import annotations

import json
import re
from pathlib import Path


CONTRACTS_DIR = Path(__file__).resolve().parents[1]
SCHEMA_PATH = CONTRACTS_DIR / "contract-manifest-schema.json"

_SEMVER_RE = re.compile(r"^\d+\.\d+\.\d+$")
_ISO_Z_RE = re.compile(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$")
_VALID_AUTH = {"admin", "signer", "any", "capability", "multisig",
                 "authorized_payout_key", "proposed_admin", "controller_or_admin",
                 "proposed_controller"}
_VALID_NETWORKS = {"testnet", "mainnet", "futurenet", "local"}
_VALID_DEPLOYMENT_STATUS = {"deployed", "upgraded", "rolled_back", "failed"}

# Snapshot/rollback guardrail: manifest behaviors that must be present when
# the contract exposes snapshot entrypoints.
_SNAPSHOT_ENTRYPOINTS = {"create_config_snapshot", "restore_config_snapshot"}
_REQUIRED_SNAPSHOT_SECURITY_FEATURES = {
    "read-only mode",  # substring match — case-insensitive
}


def _load_json(path: Path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as e:
        raise SystemExit(f"ERROR: missing file: {path}") from e
    except json.JSONDecodeError as e:
        raise SystemExit(f"ERROR: invalid JSON in {path}: {e}") from e


def _find_manifests() -> list[Path]:
    return sorted(CONTRACTS_DIR.glob("*-manifest.json"))


def _basic_manifest_checks(manifest: dict, path: Path) -> list[str]:
    errors: list[str] = []

    required = ["contract_name", "contract_purpose", "version", "entrypoints", "configuration", "behaviors"]
    for key in required:
        if key not in manifest:
            errors.append(f"{path.name}: missing required field `{key}`")

    version = manifest.get("version") or {}
    for k in ("current", "schema"):
        v = version.get(k)
        if not isinstance(v, str) or not _SEMVER_RE.match(v):
            errors.append(f"{path.name}: version.{k} must be semver (x.y.z); got {v!r}")

    entrypoints = manifest.get("entrypoints") or {}
    for k in ("public", "view"):
        if k not in entrypoints:
            errors.append(f"{path.name}: entrypoints.{k} missing")
        elif not isinstance(entrypoints.get(k), list):
            errors.append(f"{path.name}: entrypoints.{k} must be an array")

    def walk(obj):
        if isinstance(obj, dict):
            if "authorization" in obj:
                yield obj["authorization"]
            for v in obj.values():
                yield from walk(v)
        elif isinstance(obj, list):
            for v in obj:
                yield from walk(v)

    for auth in walk(manifest):
        if auth is None:
            continue
        if auth not in _VALID_AUTH:
            errors.append(f"{path.name}: invalid authorization value {auth!r}")

    # Snapshot/rollback guardrail: if the manifest exposes snapshot entrypoints,
    # it must document at least one read-only-mode security feature so reviewers
    # know mutations are blocked during incidents.
    all_entrypoint_names: set[str] = set()
    for section in (manifest.get("entrypoints") or {}).values():
        if isinstance(section, list):
            for ep in section:
                if isinstance(ep, dict) and isinstance(ep.get("name"), str):
                    all_entrypoint_names.add(ep["name"])

    if _SNAPSHOT_ENTRYPOINTS & all_entrypoint_names:
        security_features: list = (
            (manifest.get("behaviors") or {}).get("security_features") or []
        )
        feature_text = " ".join(f.lower() for f in security_features if isinstance(f, str))
        for required in _REQUIRED_SNAPSHOT_SECURITY_FEATURES:
            if required not in feature_text:
                errors.append(
                    f"{path.name}: manifest exposes snapshot entrypoints but "
                    f"behaviors.security_features does not mention {required!r} — "
                    f"add a note confirming snapshot mutations are blocked in read-only mode"
                )

    return errors


def _find_deployment_seed_files() -> list[Path]:
    # Intentionally broad: teams can keep multiple seed files per contract + network.
    return sorted(CONTRACTS_DIR.glob("**/deployments/*.json"))


def _deployment_seed_checks(data: dict, path: Path) -> list[str]:
    errors: list[str] = []

    deployments = data.get("deployments")
    if not isinstance(deployments, list):
        errors.append(f"{path}: missing/invalid `deployments` array")
        return errors

    meta = data.get("metadata")
    if meta is None:
        errors.append(f"{path}: missing `metadata` object")
        meta = {}
    if not isinstance(meta, dict):
        errors.append(f"{path}: `metadata` must be an object")
        meta = {}

    created = meta.get("created")
    if created is not None and (not isinstance(created, str) or not _ISO_Z_RE.match(created)):
        errors.append(f"{path}: metadata.created must be ISO-8601 Zulu like 2026-01-01T00:00:00Z; got {created!r}")

    version = meta.get("version")
    if version is not None and not isinstance(version, str):
        errors.append(f"{path}: metadata.version must be a string; got {type(version).__name__}")

    required_fields = (
        "contract_id",
        "wasm_hash",
        "contract_name",
        "network",
        "deployer",
        "deployed_at",
        "status",
    )

    for i, d in enumerate(deployments):
        if not isinstance(d, dict):
            errors.append(f"{path}: deployments[{i}] must be an object; got {type(d).__name__}")
            continue

        for k in required_fields:
            if k not in d:
                errors.append(f"{path}: deployments[{i}] missing `{k}`")

        network = d.get("network")
        if isinstance(network, str) and network not in _VALID_NETWORKS:
            errors.append(f"{path}: deployments[{i}].network must be one of {sorted(_VALID_NETWORKS)}; got {network!r}")

        status = d.get("status")
        if isinstance(status, str) and status not in _VALID_DEPLOYMENT_STATUS:
            errors.append(
                f"{path}: deployments[{i}].status must be one of {sorted(_VALID_DEPLOYMENT_STATUS)}; got {status!r}"
            )

        deployed_at = d.get("deployed_at")
        if deployed_at is not None and (not isinstance(deployed_at, str) or not _ISO_Z_RE.match(deployed_at)):
            errors.append(
                f"{path}: deployments[{i}].deployed_at must be ISO-8601 Zulu like 2026-01-01T00:00:00Z; got {deployed_at!r}"
            )

        # Basic type sanity (do not enforce StrKey formatting here).
        for k in ("contract_id", "wasm_hash", "contract_name", "deployer"):
            v = d.get(k)
            if v is not None and not isinstance(v, str):
                errors.append(f"{path}: deployments[{i}].{k} must be a string; got {type(v).__name__}")
            if isinstance(v, str) and not v.strip():
                errors.append(f"{path}: deployments[{i}].{k} must be non-empty")

    return errors


def _error_registry_checks(manifest: dict, path: Path) -> list[str]:
    """Validate the optional error_registry section of a manifest.

    Checks:
    - error_registry.codes is an array of objects
    - Each entry has a positive integer `code` and a non-empty string `name`
    - No two entries share the same numeric code (within this manifest)
    - If `ranges` are declared, every range has min <= max and min >= 1
    """
    errors: list[str] = []
    reg = manifest.get("error_registry")
    if reg is None:
        return errors  # section is optional
    if not isinstance(reg, dict):
        errors.append(f"{path.name}: error_registry must be an object; got {type(reg).__name__}")
        return errors

    codes = reg.get("codes", [])
    if not isinstance(codes, list):
        errors.append(f"{path.name}: error_registry.codes must be an array")
        return errors

    seen: dict[int, str] = {}
    for i, entry in enumerate(codes):
        if not isinstance(entry, dict):
            errors.append(f"{path.name}: error_registry.codes[{i}] must be an object")
            continue

        code = entry.get("code")
        name = entry.get("name", f"<entry {i}>")

        if not isinstance(code, int) or isinstance(code, bool) or code < 1:
            errors.append(
                f"{path.name}: error_registry.codes[{i}].code must be a positive integer; got {code!r}"
            )
            continue

        if not isinstance(name, str) or not name.strip():
            errors.append(f"{path.name}: error_registry.codes[{i}].name must be a non-empty string")

        if code in seen:
            errors.append(
                f"{path.name}: duplicate error code {code} — "
                f"used by both '{seen[code]}' and '{name}'"
            )
        else:
            seen[code] = name

    ranges = reg.get("ranges", [])
    if not isinstance(ranges, list):
        errors.append(f"{path.name}: error_registry.ranges must be an array")
        return errors

    for j, rng in enumerate(ranges):
        if not isinstance(rng, dict):
            errors.append(f"{path.name}: error_registry.ranges[{j}] must be an object")
            continue
        rmin = rng.get("min")
        rmax = rng.get("max")
        rname = rng.get("name", f"<range {j}>")
        if not isinstance(rmin, int) or isinstance(rmin, bool) or rmin < 1:
            errors.append(f"{path.name}: error_registry.ranges[{j}] ({rname!r}) min must be an integer >= 1")
        if not isinstance(rmax, int) or isinstance(rmax, bool):
            errors.append(f"{path.name}: error_registry.ranges[{j}] ({rname!r}) max must be an integer")
        if isinstance(rmin, int) and isinstance(rmax, int) and rmin > rmax:
            errors.append(
                f"{path.name}: error_registry.ranges[{j}] ({rname!r}) min ({rmin}) > max ({rmax})"
            )

    return errors


def _cross_manifest_error_code_warnings(
    manifests_data: "list[tuple[Path, dict]]",
) -> list[str]:
    """Return warning strings for error codes that appear in more than one manifest.

    Cross-contract duplicates are *warnings*, not errors: Soroban contracts have
    independent error namespaces, so the same numeric value in two contracts is
    legal.  However, documenting the overlap helps SDK authors and auditors.
    """
    # code -> list of (contract_name, variant_name)
    global_map: dict[int, list[tuple[str, str]]] = {}
    for path, manifest in manifests_data:
        contract_name = manifest.get("contract_name") or path.stem
        reg = manifest.get("error_registry") or {}
        for entry in reg.get("codes") or []:
            if not isinstance(entry, dict):
                continue
            code = entry.get("code")
            name = entry.get("name", "?")
            if not isinstance(code, int) or isinstance(code, bool) or code < 1:
                continue
            global_map.setdefault(code, []).append((contract_name, name))

    warnings = []
    for code, usages in sorted(global_map.items()):
        if len(usages) > 1:
            detail = ", ".join(f"{c}::{n}" for c, n in usages)
            warnings.append(
                f"WARN: error code {code} appears in multiple contracts: {detail}"
            )
    return warnings


def main() -> int:
    # Existence + JSON validity check only (AJV is the schema enforcer).
    _load_json(SCHEMA_PATH)

    manifests = _find_manifests()
    if not manifests:
        print("WARNING: no *-manifest.json files found.")
        return 0

    all_errors: list[str] = []
    valid_manifests: list[tuple[Path, dict]] = []

    for path in manifests:
        manifest = _load_json(path)
        if not isinstance(manifest, dict):
            all_errors.append(f"{path.name}: root must be an object; got {type(manifest).__name__}")
            continue
        all_errors.extend(_basic_manifest_checks(manifest, path))
        all_errors.extend(_error_registry_checks(manifest, path))
        valid_manifests.append((path, manifest))

    seed_files = _find_deployment_seed_files()
    for seed_path in seed_files:
        seed = _load_json(seed_path)
        if not isinstance(seed, dict):
            all_errors.append(f"{seed_path}: root must be an object; got {type(seed).__name__}")
            continue
        all_errors.extend(_deployment_seed_checks(seed, seed_path))

    if all_errors:
        for e in all_errors:
            print(f"ERROR: {e}")
        return 1

    # Cross-manifest warnings are printed but do not fail the script.
    for w in _cross_manifest_error_code_warnings(valid_manifests):
        print(w)

    print(
        f"OK: validated {len(manifests)} manifest(s), "
        f"{len(seed_files)} deployment seed file(s), "
        f"and error code registry uniqueness."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

