# Epoch-Boundary Snapshotting for Scheduled Releases

## Overview
The Program Escrow contract supports scheduled automated releases for recipients over time. Historically, `trigger_program_releases` evaluated the live `RELEASE_HISTORY` and schedule registry at call time. 

If a schedule was edited (e.g. by a program administrator) between when an off-chain caller decides to trigger releases and when the transaction lands on-chain, the payout could execute against a different recipient set than intended.

To resolve this, the contract introduces an **epoch-boundary snapshot mechanism** that freezes the currently-due schedule entries into an immutable batch reference prior to execution.

## Lifecycle
1. **Create Snapshot**: An authorized caller invokes `create_epoch_snapshot` or `create_epoch_snapshot_by`. The contract evaluates the live registry, identifies all due schedules (`now >= release_timestamp`), and clones them into an `EpochSnapshot` stored under a unique `epoch_id`.
2. **Execute Trigger**: The caller invokes `trigger_program_releases` (or `trigger_program_releases_by`), passing the `epoch_id`.
3. **Frozen Execution**: The contract executes the token transfers using the `recipient` and `amount` from the **frozen snapshot** instead of the live registry.
4. **State Updates**: After execution, the global `RELEASE_HISTORY` and live `SCHEDULES` registry are updated. The corresponding schedule in the live registry is marked as `released = true`.

## Security Assumptions
- **Immutability**: Once a snapshot is created, its recipient set and amounts are immutable. Even if an admin edits the schedule in the live registry, any trigger using the `epoch_id` will execute with the frozen values.
- **Single Execution**: A schedule that is marked `released = true` in the live registry will not be executed again, even if it is part of another snapshot.
- **Fallback**: If `trigger_program_releases` is called with `epoch_id = None`, the contract falls back to evaluating the live registry at call time.

## Relationship to RELEASE_HISTORY
The `RELEASE_HISTORY` and `PROGRAM_DATA.payout_history` are appended with the actual executed recipient and amount from the frozen snapshot (if provided), ensuring audit logs accurately reflect the destination of funds.
