# Bounty Escrow Contract

## Batch Operations & Failure Semantics

The escrow contract supports atomically locking or releasing multiple bounties in a single transaction via `batch_lock_funds` and `batch_release_funds`.

### All-or-nothing atomicity

A batch either succeeds completely or reverts entirely — no partial state is written. If any item fails validation (wrong amount, duplicate ID, bounty already exists, etc.) the transaction panics and every sibling item is rolled back.

### Processing order

Items are sorted by ascending `bounty_id` before execution, ensuring deterministic ordering regardless of input order. This makes replay and debugging reproducible across runs.

### Limits

| Constant | Value | Error when exceeded |
|---|---|---|
| `MAX_BATCH_SIZE` | 20 | `InvalidBatchSize` |
| Duplicate `bounty_id` within one batch | — | `DuplicateBountyId` |

### Common failure causes

| Error | Triggered when |
|---|---|
| `NotInitialized` | Contract `init()` has not been called |
| `FundsPaused` | Lock/release operations are paused by admin |
| `ContractDeprecated` | Contract has been deprecated (lock only) |
| `InvalidBatchSize` | Batch is empty or exceeds `MAX_BATCH_SIZE` |
| `DuplicateBountyId` | Same `bounty_id` appears more than once in the batch |
| `InvalidAmount` | An `amount` field is `<= 0` or outside the configured `AmountPolicy` |
| `BountyExists` | A `bounty_id` in `batch_lock_funds` is already in storage |
| `BountyNotFound` | A `bounty_id` in `batch_release_funds` does not exist |
| `FundsNotLocked` | Escrow for that ID exists but is not in `Locked` status |

For full details including the CEI pattern breakdown, reentrancy guarantees, and security assumptions, see [contracts/escrow/BATCH_SEMANTIC.md](contracts/escrow/BATCH_SEMANTIC.md).

---

## Dry-Run Simulation API

The escrow contract provides read-only dry-run entrypoints for previewing operations without mutating state. See [contracts/escrow/DRY_RUN_API.md](contracts/escrow/DRY_RUN_API.md) for full documentation.

- **dry_run_lock** – Simulate lock without transfers
- **dry_run_release** – Simulate release without transfers
- **dry_run_refund** – Simulate refund without transfers

All return `SimulationResult` with success/error_code/amount/resulting_status/remaining_amount. No authorization required.

## Participant filters

The bounty escrow contract supports mutually exclusive participant filtering for new locks:

- `Disabled` lets any depositor lock funds.
- `BlocklistOnly` rejects blocklisted depositors with `ParticipantBlocked`.
- `AllowlistOnly` accepts only allowlisted depositors and rejects others with `ParticipantNotAllowed`.

Mode changes emit `ParticipantFilterModeChanged`, and allowlist entries continue to bypass anti-abuse cooldown checks even when filtering is disabled. See [contracts/escrow/PARTICIPANT_FILTER.md](contracts/escrow/PARTICIPANT_FILTER.md) for the full behavior matrix and edge cases.

## Metadata Constraints

The escrow metadata API enforces validation rules for human-readable tags like
`bounty_type`. See `contracts/escrow/METADATA_CONSTRAINTS.md` for the current
limits and guidance.

---

## Risk flags (bounty metadata)

Per-bounty risk signaling uses a `u32` bitfield on escrow metadata (`RISK_FLAG_*` constants in `lib.rs`). Admin-only entrypoints `set_escrow_risk_flags` and `clear_escrow_risk_flags` persist flags and emit `RiskFlagsUpdated` (versioned payload with `previous_flags`, `new_flags`, `admin`, `timestamp`) for indexers. Flags are informational on-chain; policy enforcement is expected off-chain. Tests: `contracts/escrow/src/test_risk_flags.rs`.

## Treasury routing

The escrow contract supports optional multi-region treasury routing for collected
fees. Admins configure weighted `TreasuryDestination` entries through
`set_treasury_distributions`, then enable routing with `distribution_enabled =
true`.

- Lock and release fees continue to use the existing fee rates.
- When routing is disabled, the full fee is sent to the configured
  `fee_recipient`.
- When routing is enabled, the fee is split proportionally across treasury
  destinations by weight.
- Rounding remains deterministic: any remainder is assigned to the final
  destination in config order so the distributed total always matches the
  collected fee exactly.
- Invalid enabled configurations are rejected if there are no destinations or
  if any destination has zero weight.

Tests: `contracts/escrow/src/test_multi_region_treasury.rs`.

## Boundary limits

| Area | Behavior |
|------|----------|
| Amount policy | Optional inclusive `[min_amount, max_amount]` via `set_amount_policy`; violations return `AmountBelowMinimum` / `AmountAboveMaximum`. `min > max` panics. |
| Fees | Rates are in basis points; cap is `MAX_FEE_RATE` (5000 = 50%). |
| Batch size | `MAX_BATCH_SIZE` (20) items per `batch_lock_funds` / `batch_release_funds`. |
| Deadlines | `u64::MAX` is stored as-is (no-expiry style); past deadlines are allowed at lock time. |
| Pagination | `get_escrow_ids_by_status` with `limit == 0` returns an empty list. |

Tests: `contracts/escrow/src/test_boundary_edge_cases.rs` (and `test_batch_failure_modes.rs` for batch edges).

## Status transitions and expiry vs dispute

Valid payout paths are mutually exclusive: admin `release_funds` moves escrow to `Released`; refund moves to `Refunded` / `PartiallyRefunded`. A second release or refund on a terminal state fails with `FundsNotLocked` (or related errors). When a pending claim exists (`authorize_claim`), `refund` returns `ClaimPending` even if the bounty deadline has passed—admin must `cancel_pending_claim` (or the beneficiary `claim`s) before expiry-based refund applies. Tests: `contracts/escrow/src/test_lifecycle.rs`, `test_expiration_and_dispute.rs`, `test_status_transitions.rs`.

---

## Timelock Policy

The escrow contract implements an optional timelock mechanism for sensitive admin actions to protect against compromised admin keys. When enabled, admin actions must go through a propose-and-execute flow with a configurable delay.

### Admin Rotation with Timelock

The contract features a **two-step admin rotation system** with configurable timelock for secure governance transitions. See [ADMIN_ROTATION_TIMELOCK.md](contracts/escrow/ADMIN_ROTATION_TIMELOCK.md) for complete documentation including:
- Proposal and acceptance flow
- Timelock configuration (1 hour to 30 days)
- Security invariants and authorization
- Event specifications
- Upgrade safety considerations

### Two-Step Flow

1. **Propose** - Admin calls `propose_admin_action` with the desired action and parameters
2. **Execute** - After the delay elapses, anyone can call `execute_after_delay` to apply the change

### ASCII Timeline

```
T=0        T=0+delay         T=0+delay+ε
|  PROPOSE  |  EXECUTABLE    |  EXECUTE
|-----------|----------------|---------->
             ↑ earliest execution point
```

### Configuration

| Parameter | Minimum | Default | Maximum | Description |
|-----------|---------|---------|---------|-------------|
| `delay` | 3,600 seconds (1 hour) | 86,400 seconds (24 hours) | 2,592,000 seconds (30 days) | Time before action becomes executable |
| `is_enabled` | false | false | true | Whether timelock is active |

### Protected Actions

When timelock is enabled, these admin functions are blocked and must use the propose flow:
- `update_fee_config` → `ActionType::ChangeFeeRecipient`
- `set_paused` → `ActionType::SetPaused`
- `set_deprecated` → `ActionType::EnableKillSwitch` / `DisableKillSwitch`
- `set_maintenance_mode` → `ActionType::SetMaintenanceMode`

### Emergency Cancellation

Admin can cancel any pending action at any time before execution using `cancel_admin_action`.

### Public Visibility

All pending actions are publicly visible via:
- `get_pending_actions()` - Returns all pending actions ordered by proposal time
- `get_action(action_id)` - Returns details of a specific action

### Design Notes

- `configure_timelock` bypasses the timelock (bootstrap problem)
- When timelock is disabled, all existing admin functions work normally
- Execution is permissionless - any address can execute after the delay
- Tests: `contracts/escrow/src/test_timelock.rs`

---

# Soroban Project

## Project Structure

This repository uses the recommended structure for a Soroban project:
```text
.
├── contracts
│   └── hello_world
│       ├── src
│       │   ├── lib.rs
│       │   └── test.rs
│       └── Cargo.toml
├── Cargo.toml
└── README.md
```

- New Soroban contracts can be put in `contracts`, each in their own directory. There is already a `hello_world` contract in there to get you started.
- If you initialized this project with any other example contracts via `--with-example`, those contracts will be in the `contracts` directory as well.
- Contracts should have their own `Cargo.toml` files that rely on the top-level `Cargo.toml` workspace for their dependencies.
- Frontend libraries can be added to the top-level directory as well. If you initialized this project with a frontend template via `--frontend-template` you will have those files already included.
