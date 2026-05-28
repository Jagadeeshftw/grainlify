#!/usr/bin/env python3
"""Validate committed program-escrow benchmark snapshots against CI thresholds."""

from __future__ import annotations

import json
import sys
from pathlib import Path


def fail(message: str) -> int:
    print(f"benchmark gate failed: {message}")
    return 1


def load_json(path: Path) -> dict:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def main() -> int:
    if len(sys.argv) != 3:
        return fail("usage: check_program_escrow_benchmark.py <baseline.json> <thresholds.json>")

    baseline_path = Path(sys.argv[1])
    thresholds_path = Path(sys.argv[2])

    if not baseline_path.exists():
        return fail(f"missing baseline snapshot: {baseline_path}")
    if not thresholds_path.exists():
        return fail(f"missing threshold file: {thresholds_path}")

    baseline = load_json(baseline_path)
    thresholds = load_json(thresholds_path)

    if baseline.get("status") != "complete":
        return fail("baseline snapshot status must be 'complete'")

    gate = thresholds.get("gates", {}).get("batch_payout", {}).get("50")
    if not gate:
        return fail("missing batch_payout/50 threshold")

    max_fee = gate.get("max_actual_fee_charged_stroops")
    if not isinstance(max_fee, int):
        return fail("threshold max_actual_fee_charged_stroops must be an integer")

    matches = [
        record
        for record in baseline.get("results", [])
        if record.get("operation") == "batch_payout" and record.get("batch_size") == 50
    ]
    if not matches:
        return fail("baseline snapshot is missing a batch_payout result for batch_size=50")

    record = matches[0]
    actual_fee = record.get("actual_fee_charged_stroops")
    if not isinstance(actual_fee, int):
        return fail("actual_fee_charged_stroops must be an integer")

    cpu_instructions = record.get("cpu_instructions")
    if not isinstance(cpu_instructions, int):
        return fail("cpu_instructions must be an integer")

    inclusion_ledger = record.get("inclusion_ledger")
    if not isinstance(inclusion_ledger, int):
        return fail("inclusion_ledger must be an integer")

    if actual_fee > max_fee:
        return fail(
            f"batch_payout(50) actual fee {actual_fee} exceeds threshold {max_fee} at ledger {inclusion_ledger}"
        )

    print(
        "benchmark gate passed: "
        f"batch_payout(50) fee={actual_fee} threshold={max_fee} cpu={cpu_instructions} ledger={inclusion_ledger}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
