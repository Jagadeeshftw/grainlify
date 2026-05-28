# Full Event Schema Reference

This document is the canonical reference for every event emitted across all three
Grainlify smart contracts:

| Contract | Crate path |
|----------|-----------|
| **program-escrow** | `contracts/program-escrow/src/lib.rs` |
| **bounty-escrow** | `contracts/bounty_escrow/contracts/escrow/src/` |
| **grainlify-core** | `contracts/grainlify-core/src/lib.rs` |

**Audience:** indexer authors, SDK developers, monitoring engineers.

---

## Conventions

### EVENT_VERSION_V2 Envelope

Every event payload struct carries a `version: u32` field set to **`2`**.
This constant is defined as `EVENT_VERSION_V2 = 2` in each crate.

```
topics : (category_symbol [, secondary_key])
data   : <EventStruct { version: 2, ... }>
```

Indexers **must** check `version` before decoding field values.
Unknown versions must be treated as forward-compatible (new optional fields may be added
without a version bump; breaking layout changes require an increment).

### XDR Encoding

All event data structs are encoded as Soroban `ScMap` (XDR map type `0x00000011`).
Fields are ordered **alphabetically** by field name within the map.
Field names are encoded as `ScSymbol` (XDR type `0x0000000f`).

**Topic symbols** are limited to ≤ 9 bytes. Longer values are rejected by the Soroban host.

### Security Invariants (all contracts)

1. Events are emitted **after** all state mutations and token transfers (CEI pattern).
2. No PII, private keys, or sensitive secrets are ever included in event payloads.
3. Amounts and addresses in events reflect **settled** on-chain state.

---

## 1. program-escrow

Source: [`contracts/program-escrow/src/lib.rs`](../../contracts/program-escrow/src/lib.rs)
Errors: [`contracts/program-escrow/src/errors.rs`](../../contracts/program-escrow/src/errors.rs)

### 1.1 ProgramInitialized

Emitted once when `init_program` / `initialize_program` succeeds.

```
topics : (PrgInit)
data   : ProgramInitializedEvent {
    version:              u32,     // EVENT_VERSION_V2
    program_id:           String,
    authorized_payout_key: Address,
    token_address:        Address,
    total_funds:          i128,
}
```

*Error paths:* `ContractError::ProgramAlreadyExists (8)`, `ContractError::TokenNotAllowed (1100)`

---

### 1.2 FundsLocked

Emitted when `lock_program_funds` succeeds.

```
topics : (FndsLock)
data   : FundsLockedEvent {
    version:           u32,
    program_id:        String,
    amount:            i128,   // gross amount credited
    remaining_balance: i128,   // balance after this lock
}
```

*Error paths:* `ContractError::Paused (3)`, `ContractError::InvalidAmount (2)`, `ContractError::FundLockFailed (200)`

---

### 1.3 BatchFundsLocked

Emitted once per `batch_lock` call (summary, not per-program).

```
topics : (BatLck)
data   : BatchFundsLocked {
    count:        u32,
    total_amount: i128,
    timestamp:    u64,
}
```

*Error paths:* `BatchError::InvalidBatchSizeProgram (403)`, `BatchError::DuplicateProgramId (402)`, `BatchError::FundsPaused (407)`

---

### 1.4 BatchFundsReleased

Emitted once per `batch_release` call.

```
topics : (BatRel)
data   : BatchFundsReleased {
    count:        u32,
    total_amount: i128,
    timestamp:    u64,
}
```

---

### 1.5 BatchPayout

Emitted per `batch_payout` / `batch_payout_by` call.

```
topics : (BatchPay)
data   : BatchPayoutEvent {
    version:          u32,
    program_id:       String,
    recipient_count:  u32,
    total_amount:     i128,
    remaining_balance: i128,
}
```

*Error paths:* `BatchPayoutError::Unauthorized (3103)`, `BatchPayoutError::InsufficientBalance (3109)`, `BatchPayoutError::SpendLimitExceeded (3108)`, `BatchPayoutError::CircuitBreakerOpen (3110)`

---

### 1.6 Payout

Emitted per `single_payout` / `single_payout_by` call.

```
topics : (Payout)
data   : PayoutEvent {
    version:           u32,
    program_id:        String,
    recipient:         Address,
    amount:            i128,
    remaining_balance: i128,
}
```

---

### 1.7 ReleaseScheduled

Emitted when `schedule_program_release` adds a new schedule.

```
topics : (RelSched, program_id)
data   : ReleaseScheduledEvent {
    version:           u32,
    program_id:        String,
    schedule_id:       u64,
    recipient:         Address,
    amount:            i128,
    release_timestamp: u64,
}
```

---

### 1.8 ScheduleReleased

Emitted when a scheduled release is executed.

```
topics : (SchRel, program_id)
data   : ScheduleReleasedEvent {
    version:     u32,
    program_id:  String,
    schedule_id: u64,
    recipient:   Address,
    amount:      i128,
    released_at: u64,
    released_by: Address,
}
```

---

### 1.9 ProgramDelegateSet

Emitted when `set_program_delegate` succeeds.

```
topics : (PrgDlgS, program_id)
data   : ProgramDelegateSetEvent {
    version:     u32,
    program_id:  String,
    delegate:    Address,
    permissions: u32,    // DELEGATE_PERMISSION_RELEASE=1 | REFUND=2 | UPDATE_META=4
    updated_by:  Address,
    timestamp:   u64,
}
```

---

### 1.10 ProgramDelegateRevoked

Emitted by both `revoke_program_delegate` (normal path) and
`emergency_revoke_delegate` (admin fast-path).

```
topics : (PrgDlgR, program_id)
data   : ProgramDelegateRevokedEvent {
    version:     u32,
    program_id:  String,
    delegate:    Address,  // address whose permissions were zeroed
    revoked_by:  Address,
    timestamp:   u64,
    emergency:   bool,     // true  = called via emergency_revoke_delegate
                           // false = called via revoke_program_delegate
}
```

**Security note for indexers:** Monitor for `emergency: true` — it indicates the admin
has determined a delegate key is compromised. Alert on any such event.

*Error paths:* `ContractError::Unauthorized (1)` — caller is neither admin nor payout key.

---

### 1.11 ProgramRiskFlagsUpdated

```
topics : (pr_risk, program_id)
data   : ProgramRiskFlagsUpdated {
    version:        u32,
    program_id:     String,
    previous_flags: u32,
    new_flags:      u32,
    admin:          Address,
    timestamp:      u64,
}
```

Risk flag bits: `HIGH_RISK=1`, `UNDER_REVIEW=2`, `RESTRICTED=4`, `DEPRECATED=8`.

---

### 1.12 ProgramMetadataUpdated

```
topics : (PrgMeta, program_id)
data   : ProgramMetadataUpdatedEvent {
    version:    u32,
    program_id: String,
    updated_by: Address,
    timestamp:  u64,
}
```

---

### 1.13 PauseStateChanged (v1)

```
topics : (PauseSt, operation_symbol)
data   : PauseStateChanged {
    operation:  Symbol,   // "lock" | "release" | "refund"
    paused:     bool,
    admin:      Address,
    reason:     Option<String>,
    timestamp:  u64,
    receipt_id: u64,
}
```

---

### 1.14 PauseStateChangedV2

Emitted alongside v1 for every `set_paused` call. Adds `previous_paused` for full transition logging.

```
topics : (PauseStV2, operation_symbol)
data   : PauseStateChangedV2 {
    version:        u32,
    operation:      Symbol,
    previous_paused: bool,
    paused:         bool,
    admin:          Address,
    reason:         Option<String>,
    timestamp:      u64,
    receipt_id:     u64,
}
```

---

### 1.15 MaintenanceModeChanged

```
topics : (MaintSt)
data   : MaintenanceModeChanged {
    enabled:   bool,
    admin:     Address,
    timestamp: u64,
}
```

---

### 1.16 ReadOnlyModeChanged

```
topics : (ROModeChg)
data   : ReadOnlyModeChanged {
    enabled:   bool,
    admin:     Address,
    timestamp: u64,
    reason:    Option<String>,
}
```

---

### 1.17 AdminProposed / AdminAccepted / AdminRotationCancelled

Two-step admin rotation events (see `propose_admin`, `accept_admin`, `cancel_admin_rotation`).

```
topics : (AdmProp)
data   : AdminProposedEvent { version, proposed_by, proposed_admin, timestamp }

topics : (AdmAcc)
data   : AdminAcceptedEvent { version, previous_admin, new_admin, timestamp }

topics : (AdmCanc)
data   : AdminRotationCancelledEvent { version, cancelled_by, timestamp }
```

---

### 1.18 ControllerProposed / ControllerAccepted / ControllerRotationCancelled

Two-step payout-key rotation events (see `propose_controller`, `accept_controller`,
`cancel_controller_rotation`).

```
topics : (CtrlProp, program_id)
data   : ControllerProposedEvent { version, program_id, proposed_by, proposed_controller, timestamp }

topics : (CtrlAcc, program_id)
data   : ControllerAcceptedEvent { version, program_id, previous_controller, new_controller, timestamp }

topics : (CtrlCanc, program_id)
data   : ControllerRotationCancelledEvent { version, program_id, cancelled_by, timestamp }
```

---

### 1.19 DisputeOpened / DisputeResolved

```
topics : (DspOpen, program_id)
data   : DisputeOpenedEvent { version, program_id, raised_by, reason, opened_at }

topics : (DspRslv, program_id)
data   : DisputeResolvedEvent { version, program_id, resolved_by, resolution_notes, resolved_at }
```

*Error paths:* `ContractError::DisputeAlreadyOpen (600)`, `ContractError::NoActiveDispute (601)`

---

### 1.20 SpendLimitSet / SpendLimitExceeded / SpendLimitSchemaVersionSet

```
topics : (SpLimSet, program_id)
data   : SpendLimitSetEvent {
    version, program_id, previous_threshold, new_threshold, set_by, timestamp
}

topics : (SpLimExc, program_id)
data   : SpendLimitExceededEvent {
    version, program_id, requested_amount, threshold, timestamp
}

topics : (SpLimSch)
data   : SpendLimitSchemaVersionSet { version, schema_version, timestamp }
```

---

### 1.21 IdempotencyKeyUsed (first use and retry)

Both first-use and retry events use the same topic symbol. Distinguish via payload type.

```
topics : (IdempUsed, idempotency_key)
data   : IdempotencyKeyUsedEvent {
    version, idempotency_key, operation_type, program_id,
    total_amount, recipient_count, executor, executed_at
}

-- OR (on retry) --

data   : IdempotencyKeyRetryEvent {
    version, idempotency_key, original_success, original_executed_at,
    original_executor, retry_attempt_at, retry_by
}
```

---

### 1.22 TokenAllowlistUpdated / TokenRejected / TokenAllowlistSchemaVersionSet

```
topics : (TkAllow)
data   : TokenAllowlistUpdatedEvent { version, token, added, updated_by, timestamp }

topics : (TkReject)
data   : TokenRejectedEvent { version, token, program_id, timestamp }

topics : (TkAlSch)
data   : TokenAllowlistSchemaVersionSet { version, schema_version, timestamp }
```

---

### 1.23 FeeCollected / FeeRecipientUpdated

```
topics : (FeeCol, operation_symbol)
data   : FeeCollectedEvent { version, operation, fee_amount, fee_rate_bps, fee_fixed, recipient, timestamp }

topics : ("fee_recipient_updated")
data   : FeeRecipientUpdatedEvent { version, old_recipient, new_recipient, updated_by, timestamp }
```

---

## 2. bounty-escrow

Source: [`contracts/bounty_escrow/contracts/escrow/src/events.rs`](../../contracts/bounty_escrow/contracts/escrow/src/events.rs)
Errors: [`contracts/bounty_escrow/contracts/escrow/src/lib.rs`](../../contracts/bounty_escrow/contracts/escrow/src/lib.rs) — `Error` enum

### 2.1 BountyEscrowInitialized

Emitted once per contract deployment.

```
topics : ("init")
data   : BountyEscrowInitialized { version, admin, token, timestamp }
```

*Error path:* `Error::AlreadyInitialized (1)`

---

### 2.2 Admin Rotation Events

Four-event sequence covering two-step admin rotation with optional timelock.

```
topics : ("adm_prop")
data   : (old_admin: Address, new_admin: Address)

topics : ("admin_tx")
data   : (old_admin: Address, new_admin: Address)

topics : ("adm_cncl2")
data   : (admin: Address)

topics : ("adm_prop2")     [AdminRotationProposed, structured payload]
data   : AdminRotationProposed { version, current_admin, proposed_admin, timelock_until, timestamp }

topics : ("adm_acc2")
data   : AdminRotationAccepted { version, previous_admin, new_admin, timestamp }

topics : ("adm_can2")
data   : AdminRotationCancelled { version, admin, timestamp }

topics : ("adm_tlk")       [timelock update]
data   : AdminRotationTimelockUpdated { version, previous_duration, new_duration, updated_by, timestamp }
```

*Error paths:* `Error::AdminRotationAlreadyPending (47)`, `Error::AdminRotationNotPending (48)`,
`Error::AdminRotationTimelockActive (49)`, `Error::InvalidAdminRotationTarget (51)`

---

### 2.3 FundsLocked

```
topics : ("lock", bounty_id: u64)
data   : FundsLocked { version, bounty_id, depositor, amount, deadline, timestamp }
```

*Error paths:* `Error::InvalidAmount (13)`, `Error::InvalidDeadline (14)`, `Error::FundsPaused (18)`,
`Error::AmountBelowMinimum (19)`, `Error::AmountAboveMaximum (20)`, `Error::ParticipantBlocked (35)`

---

### 2.4 FundsReleased

```
topics : ("release", bounty_id: u64)
data   : FundsReleased {
    version, bounty_id, recipient, amount, fee_amount, net_amount, timestamp
}
```

---

### 2.5 EscrowPublished

```
topics : ("publish", bounty_id: u64)
data   : EscrowPublished { version, bounty_id, timestamp }
```

---

### 2.6 FundsRefunded

```
topics : ("refund", bounty_id: u64)
data   : FundsRefunded { version, bounty_id, recipient, amount, timestamp }
```

*Error paths:* `Error::RefundNotApproved (17)`, `Error::ClaimPending (22)`

---

### 2.7 RefundApprovalSet / RefundApprovalConsumed

```
topics : ("ref_appr", bounty_id: u64)
data   : RefundApprovalSet { version, bounty_id, approved_by, timestamp }

topics : ("ref_cons", bounty_id: u64)
data   : RefundApprovalConsumed { version, bounty_id, consumed_by, timestamp }
```

---

### 2.8 FeeCollected / FeeConfigUpdated / FeeRoutingUpdated / FeeRouted

```
topics : ("fee", bounty_id: u64)
data   : FeeCollected { version, bounty_id, amount, rate_bps, recipient, timestamp }

topics : ("feecfg")
data   : FeeConfigUpdated { version, lock_fee_rate, release_fee_rate, fee_recipient, timestamp }

topics : ("feeroute")
data   : FeeRoutingUpdated { version, routes, timestamp }

topics : ("feertd", bounty_id: u64)
data   : FeeRouted { version, bounty_id, recipient, amount, timestamp }
```

---

### 2.9 BatchFundsLocked / BatchFundsReleased

```
topics : ("batchlck")
data   : BatchFundsLocked { version, count, total_amount, timestamp }

topics : ("batchrel")
data   : BatchFundsReleased { version, count, total_amount, timestamp }
```

---

### 2.10 BatchSizeCapsUpdated / MaxBatchSizeUpdated

```
topics : ("batchcap")
data   : BatchSizeCapsUpdated { version, new_lock_cap, new_release_cap, updated_by, timestamp }

topics : ("maxbatch")
data   : MaxBatchSizeUpdated { version, previous_max, new_max, updated_by, timestamp }
```

---

### 2.11 Escrow Lifecycle — Archived / Expired / CleanedUp

```
topics : ("archive", bounty_id: u64)
data   : (timestamp: u64)

topics : ("expire", bounty_id: u64)
data   : EscrowExpired { version, bounty_id, depositor, amount, timestamp }

topics : ("cleanup", bounty_id: u64)
data   : EscrowCleanedUp { version, bounty_id, timestamp }
```

---

### 2.12 Claim Tickets — TicketIssued / TicketClaimed

```
topics : ("ticket", bounty_id: u64)
data   : TicketIssued { version, bounty_id, ticket_hash, recipient, amount, timestamp }

topics : ("tclaim", bounty_id: u64)
data   : TicketClaimed { version, bounty_id, ticket_hash, recipient, amount, timestamp }
```

*Error paths:* `Error::TicketNotFound (23)`, `Error::TicketAlreadyUsed (24)`, `Error::TicketExpired (25)`

---

### 2.13 Capability Tokens — Issued / Used / Revoked

```
topics : ("cap_iss", bounty_id: u64)
data   : CapabilityIssued { version, cap_id, bounty_id, holder, action, max_amount, expiry, uses_remaining, timestamp }

topics : ("cap_use", bounty_id: u64)
data   : CapabilityUsed { ... }

topics : ("cap_rev", bounty_id: u64)
data   : CapabilityRevoked { version, cap_id, bounty_id, revoked_by, timestamp }
```

*Error paths:* `Error::CapabilityNotFound (26)`, `Error::CapabilityExpired (27)`, `Error::CapabilityRevoked (28)`,
`Error::CapabilityActionMismatch (29)`, `Error::CapabilityUsesExhausted (31)`

---

### 2.14 Participant Filtering Events

```
topics : ("prtfltr")
data   : ParticipantFilterModeChanged { version, mode, updated_by, timestamp }

topics : ("prtfent")
data   : ParticipantFilterEntryUpdated { version, address, added, mode, updated_by, timestamp }
```

---

### 2.15 EscrowFrozen / AddressUnfrozen

```
topics : ("frzesc", bounty_id: u64)
data   : FreezeRecord { ... }

topics : ("unfrzes", bounty_id: u64)
data   : (timestamp: u64)

topics : ("frzaddr", address: Address)
data   : FreezeRecord { ... }

topics : ("unfrzad", address: Address)
data   : (timestamp: u64)
```

---

### 2.16 Maintenance Mode

```
topics : ("maint")
data   : MaintenanceModeChanged { version, enabled, updated_by, timestamp }

topics : ("maintv2")
data   : MaintenanceModeChangedV2 { version, enabled, previous, updated_by, reason, timestamp }
```

---

### 2.17 Risk Flags

```
topics : ("rflag", bounty_id: u64)
data   : RiskFlagsUpdated {
    version, bounty_id, previous_flags, new_flags, updated_by, timestamp
}
```

Risk bit constants: `RISK_FLAG_HIGH_RISK=1`, `RISK_FLAG_UNDER_REVIEW=2`,
`RISK_FLAG_RESTRICTED=4`, `RISK_FLAG_DEPRECATED=8`.

---

### 2.18 EmergencyWithdraw

```
topics : ("emg_wd", bounty_id: u64)
data   : EmergencyWithdrawEvent { version, admin, bounty_id, amount, reason, timestamp }
```

---

### 2.19 Pause State

```
topics : ("pause", operation: Symbol)
data   : PauseStateChanged { operation, paused, admin, reason, timestamp, receipt_id }
```

---

### 2.20 High-Value Release Timelock Events

```
topics : ("hvq", bounty_id: u64)    [queued]
topics : ("hvx", bounty_id: u64)    [executed]
topics : ("hvcx", bounty_id: u64)   [cancelled]
```

---

### 2.21 OracleConfigUpdated

```
topics : ("oracle")
data   : OracleConfigUpdated { version, oracle_address, enabled, updated_by, timestamp }
```

---

### 2.22 Recurring Lock Events

```
topics : ("rec_lck")
data   : RecurringLockCreated { version, lock_id, depositor, amount, interval, timestamp }

topics : ("rec_exe")
data   : RecurringLockExecuted { version, lock_id, bounty_id, amount, timestamp }

topics : ("rec_cxl")
data   : RecurringLockCancelled { version, lock_id, cancelled_by, timestamp }
```

---

### 2.23 Deterministic Anonymous Selection

```
topics : ("det_sel", bounty_id: u64)
data   : DeterministicSelectionDerived { version, bounty_id, seed, selected, timestamp }
```

---

### 2.24 Notification Preferences

```
topics : ("notifprf")
data   : NotificationPreferencesUpdated { version, address, flags, updated_by, timestamp }
```

Flags: `NOTIFY_ON_LOCK=1`, `NOTIFY_ON_RELEASE=2`, `NOTIFY_ON_DISPUTE=4`, `NOTIFY_ON_EXPIRATION=8`

---

### 2.25 MetadataUpdated

```
topics : ("metadata", bounty_id: u64)
data   : MetadataUpdated { version, bounty_id, updated_by, timestamp }
```

---

### 2.26 Reentrancy Attempt Blocked

```
topics : ("reent_blk")
data   : ReentrancyAttemptBlocked { version, caller, timestamp }
```

*Error path:* implicit panic — reentrancy guard reverts the transaction.

---

## 3. grainlify-core

Source: [`contracts/grainlify-core/src/lib.rs`](../../contracts/grainlify-core/src/lib.rs)
Errors: [`contracts/grainlify-core/src/errors.rs`](../../contracts/grainlify-core/src/errors.rs)

### 3.1 Contract Initialized (admin_init)

```
topics : ("adm_init")
data   : (admin: Address, timestamp: u64)
```

---

### 3.2 Build Info

Emitted during `initialize` to record the compiled wasm build metadata.

```
topics : ("init", "build")
data   : BuildInfoEvent { version, git_commit, build_timestamp, wasm_hash, timestamp }
```

---

### 3.3 Upgrade (WASM upgrade)

```
topics : ("upgrade", "wasm")
data   : UpgradeEvent { version, new_wasm_hash, upgraded_by, timestamp }
```

---

### 3.4 ReadOnlyModeChanged

```
topics : ("ROModeChg")
data   : ReadOnlyModeEvent { enabled, admin, timestamp, reason }
```

---

### 3.5 Config Change Timelock

```
topics : ("timelock", "dly_chg")         [delay updated]
topics : ("cfg_tmlk", "dly_chg")         [config change — delay]
topics : ("cfg_tmlk", "propose")         [proposal created]
topics : ("cfg_tmlk", "cancel")          [proposal cancelled]
topics : ("cfg_tmlk", "exec")            [proposal executed]
```

---

### 3.6 Config Snapshot / Rollback

```
topics : ("cfg_snap", "create")          [snapshot created]
topics : ("cfg_snap", "restore")         [snapshot restored / rollback]
topics : ("cfg_snap", "adm_pnd")         [admin snapshot pending]
topics : ("cfg_snap", "adm_conf")        [admin snapshot confirmed]
```

---

### 3.7 Migration Events

```
topics : ("migrate", "commit")
data   : MigrationEvent { version, migration_hash, committed_by, timestamp }

topics : ("migrate", "done")
data   : MigrationCommittedEvent { version, migration_hash, executed_by, timestamp }
```

---

### 3.8 Liveness Watchdog

```
topics : ("watchdog", "ping")
data   : (operator: Address, timestamp: u64)
```

---

### 3.9 Strict Mode Invariant Failure

```
topics : ("strict", "inv_fail")
data   : InvariantReport { ... }
```

---

### 3.10 Monitoring Metrics

These are internal observability events — not guaranteed to be stable across upgrades.

```
topics : ("metric", "op")
data   : OperationMetric { operation, caller, timestamp, success }

topics : ("metric", "perf")
data   : PerformanceMetric { function, duration, timestamp }
```

---

## Cross-Contract Error Code Reference

| Domain | Code range | Contract |
|--------|-----------|----------|
| General | 1–99 | program-escrow |
| Program management | 100–199 | program-escrow |
| Fund operations | 200–299 | program-escrow |
| Payout | 300–399 | program-escrow |
| Schedule | 400–499 | program-escrow |
| Claim | 500–599 | program-escrow |
| Dispute | 600–699 | program-escrow |
| Fee | 700–799 | program-escrow |
| Circuit breaker | 800–899 | program-escrow |
| Threshold / spend limit | 900–999 | program-escrow |
| Batch recovery | 1000–1099 | program-escrow |
| Token allowlist | 1100–1199 | program-escrow |
| Role management | 1200–1299 | program-escrow |
| Batch payout | 3100–3199 | program-escrow |
| Bounty escrow (`Error`) | 1–54 | bounty-escrow |
| Core (`ContractError`) | 1–N | grainlify-core |

---

## Indexer Checklist

- [ ] Filter by topic[0] (category symbol) for efficient event retrieval.
- [ ] Always validate `version == 2` before decoding payload fields.
- [ ] For `ProgramDelegateRevokedEvent`, check `emergency: bool` and alert on `true`.
- [ ] Treat unknown `version` values as forward-compatible (do not drop events).
- [ ] Correlate `program_id` across program-escrow events to reconstruct program state.
- [ ] For bounty-escrow events that carry `bounty_id` as topic[1], use it for efficient per-bounty filtering.
- [ ] Schema version markers (e.g. `SpLimSch`, `TkAlSch`) are emitted once on init — use them to detect deployment schema.

---

## See Also

- [PR_EVENT_SCHEMA_AUDIT.md](../../PR_EVENT_SCHEMA_AUDIT.md) — prior audit of EVENT_VERSION_V2 fields
- [docs/program-escrow/delegate-authorization.md](../program-escrow/delegate-authorization.md) — delegate RBAC including emergency revocation
- [contracts/program-escrow/src/errors.rs](../../contracts/program-escrow/src/errors.rs) — full error code list
- [contracts/bounty_escrow/contracts/escrow/src/events.rs](../../contracts/bounty_escrow/contracts/escrow/src/events.rs) — bounty-escrow event definitions
