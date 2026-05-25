# Grainlify Smart Contract External Audit Checklist

Source inventory date: 2026-05-25
Scope: `contracts/program-escrow/`, `contracts/bounty_escrow/`, and `contracts/grainlify-core/`

This checklist prepares Grainlify for an external review by making the audit surface explicit before auditors start line-by-line work. It is intentionally operational: every row should be converted into audit tasks, test coverage checks, and remediation tickets.

## Audit Readiness Gates

| Gate | Required state before audit kickoff | Current status |
| --- | --- | --- |
| Entrypoint inventory | All public contract methods are listed with expected authorization | Ready for audit intake |
| Error-code registry | `errors.rs` codes are mapped to the operations that can surface them | Ready for audit intake |
| Known gaps | Pre-audit remediation items are tracked with owners/status | Open items listed below |
| Test command | `cargo test -p program-escrow` is the minimum validation command for this scope | Documented and covered by checklist test |
| Threat model | Reentrancy, oracle manipulation, fee drain, authorization, and upgrade risks are captured | Ready for audit intake |

## Contract Surface And Access Controls

### `contracts/program-escrow`

Primary financial flows are program creation, fund locking, payouts, scheduled releases, claims, fee handling, circuit breakers, and emergency withdrawal.

| Area | Entrypoints | Expected access control | Associated errors from `contracts/program-escrow/src/errors.rs` |
| --- | --- | --- | --- |
| Program initialization and metadata | `init_program`, `initialize_program`, `init_program_with_metadata`, `batch_initialize_programs`, `publish_program`, `program_exists`, `program_exists_by_id`, `get_program_info`, `get_program_info_v2`, `get_program_metadata`, `update_program_metadata`, `update_program_metadata_by`, `get_program_analytics`, `get_program_reputation`, `get_program_count`, `list_programs`, `get_program_aggregate_stats` | Creation validates uniqueness and token allowlist; publish requires the configured payout key; metadata updates require owner/admin/delegate authorization | `InvalidProgramId(6)`, `ProgramNotFound(7)`, `ProgramAlreadyExists(8)`, `ProgramArchived(9)`, `ProgramInitFailed(100)`, `NoAuthorizedPayoutKey(101)`, `MetadataUpdateFailed(104)`, `TokenNotAllowed(1100)` |
| Funding | `lock_program_funds`, `lock_program_funds_v2`, `batch_lock`, `batch_release`, `get_remaining_balance` | Locking is blocked by pause/read-only state and validates positive amounts; v2 flows bind operations to a program id | `InvalidAmount(2)`, `Paused(3)`, `MaintenanceMode(4)`, `ReadOnlyMode(5)`, `InsufficientBalance(10)`, `Overflow(11)`, `FundLockFailed(200)`, `FundReleaseFailed(201)`, `LockFeeExceedsAmount(204)`, `TokenTransferFailed(203)` |
| Payouts and receipts | `single_payout`, `single_payout_by`, `single_payout_v2`, `single_payout_idempotent`, `single_payout_idempotent_by`, `batch_payout`, `batch_payout_by`, `batch_payout_v2`, `batch_payout_with_receipt`, `batch_payout_idempotent`, `batch_payout_idempotent_by`, `get_batch_receipt`, `get_batch_receipt_by_batch_id`, `get_batch_payout_schema_version`, `is_payout_processed`, `get_idempotency_key_status` | Default payout paths require the program payout key; explicit `*_by` variants must validate the supplied caller; batch paths enforce deterministic order, size, duplicate, threshold, spend-limit, circuit-breaker, and idempotency checks | `Unauthorized(1)`, `InvalidAmount(2)`, `InsufficientBalance(10)`, `DuplicateEntry(15)`, `PayoutFailed(300)`, `BatchPayoutFailed(301)`, `InvalidBatchSize(302)`, `DuplicateRecipients(303)`, `BatchAmountsMismatch(304)`, `PayoutFeeExceedsAmount(205)`, `CircuitBreakerOpen(800)`, `ThresholdBreached(900)`, `SpendLimitExceeded(904)`, `BatchNotFound(1000)` |
| Release schedules | `create_program_release_schedule`, `create_prog_release_schedule_by`, `trigger_program_releases`, `trigger_program_releases_by`, `get_release_schedules`, `get_program_release_schedule`, `get_program_release_schedules`, `get_all_prog_release_schedules`, `get_pending_schedules`, `get_pending_program_schedules`, `get_due_schedules`, `get_due_program_schedules`, `get_total_scheduled_amount`, `release_program_schedule_manual`, `release_prog_schedule_manual_by`, `release_prog_schedule_automatic`, `get_program_release_history`, `query_schedules_by_recipient`, `query_releases_by_recipient` | Schedule creation/release paths are payout-key or delegated release operations; automatic triggers must be deterministic and reentrancy-safe | `ScheduleNotFound(400)`, `ScheduleAlreadyReleased(401)`, `ScheduleNotDue(402)`, `ScheduleCreationFailed(403)`, `InvalidScheduleParams(404)`, `ScheduleReleaseFailed(405)`, `MaxSchedulesExceeded(406)`, `ReleaseTriggerFailed(905)`, `NoSchedulesDue(906)`, `DeterminismViolation(907)` |
| Claim periods | `create_pending_claim`, `execute_claim`, `cancel_claim`, `get_claim`, `set_claim_window`, `get_claim_window` | Creating a claim is a release-path authorization decision; execution requires recipient authorization; cancellation requires admin authorization and follows refund pause state | `ClaimNotFound(500)`, `ClaimAlreadyExecuted(501)`, `ClaimExpired(502)`, `ClaimCreationFailed(503)`, `ClaimExecutionFailed(504)`, `ClaimCancellationFailed(505)`, `InvalidClaimWindow(506)` |
| Roles and administration | `initialize_contract`, `set_admin`, `get_admin`, `get_program_admin`, `propose_admin`, `accept_admin`, `cancel_admin_rotation`, `set_program_delegate`, `revoke_program_delegate`, `propose_controller`, `accept_controller`, `cancel_controller_rotation`, `rotate_payout_key`, `get_rotation_nonce`, `get_role_management_schema_version` | Admin rotations are two-step; current/proposed roles must authorize; delegate permissions are bitmask-constrained; payout-key rotation requires owner/admin/payout-key authorization depending on path | `Unauthorized(1)`, `InvalidAddress(13)`, `InvalidState(14)`, `DelegateNotSet(102)`, `DelegatePermissionsInsufficient(103)`, `AdminRotationInProgress(1200)`, `NoAdminRotationInProgress(1201)`, `InvalidAdminRotationState(1202)`, `ControllerRotationInProgress(1203)`, `NoControllerRotationInProgress(1204)`, `InvalidControllerRotationState(1205)`, `RoleTransitionExpired(1206)`, `InvalidRoleProposal(1207)`, `RoleRotationNotAllowed(1208)` |
| Pausing, maintenance, and emergency | `set_paused`, `get_pause_flags`, `get_pause_schema_version`, `set_maintenance_mode`, `is_maintenance_mode`, `set_read_only_mode`, `is_read_only`, `emergency_withdraw` | Admin-only for mode changes and emergency withdraw; emergency withdraw requires lock pause before transfer | `Paused(3)`, `MaintenanceMode(4)`, `ReadOnlyMode(5)`, `EmergencyWithdrawFailed(206)`, `Unauthorized(1)` |
| Fee, spend limit, split, and allowlist controls | `get_fee_config`, `update_fee_config`, `set_program_spend_threshold`, `get_program_spend_threshold`, `get_spend_limit_schema_version`, `set_program_spending_limit`, `get_program_spending_limit`, `get_program_spending_state`, `set_split_config`, `get_split_config`, `disable_split_config`, `execute_split_payout`, `preview_split`, `add_allowed_token`, `remove_allowed_token`, `is_token_allowed`, `get_allowed_tokens`, `get_allowlist_schema_version`, `set_whitelist` | Admin or payout-key depending on program ownership; fee recipients/rates must be bounded; split shares must sum deterministically; token allowlist updates are admin-only | `FeeConfigUpdateFailed(700)`, `InvalidFeeRate(701)`, `FeeRecipientNotSet(702)`, `FeeCollectionFailed(703)`, `InvalidSplitShares(307)`, `SplitPayoutFailed(308)`, `SplitConfigNotSet(305)`, `SplitConfigDisabled(306)`, `TokenAlreadyAllowed(1101)`, `TokenNotInAllowlist(1102)` |
| Circuit breaker, threshold, and rate limiting | `set_circuit_admin`, `get_circuit_admin`, `get_circuit_breaker_status`, `get_circuit_status`, `get_circuit_error_log`, `reset_circuit_breaker`, `configure_circuit_breaker`, `emergency_open_circuit`, `init_threshold_monitoring`, `get_threshold_config`, `get_cb_schema_version`, `update_rate_limit_config`, `get_rate_limit_config` | Circuit-admin/admin authorization; state changes must be monotonic and evented; threshold operations must avoid underflow/overflow and stale windows | `CircuitBreakerOpen(800)`, `CircuitBreakerConfigFailed(801)`, `CircuitBreakerResetFailed(802)`, `CircuitBreakerAdminNotSet(803)`, `ThresholdBreached(900)`, `InvalidThresholdConfig(901)`, `CooldownActive(902)`, `ThresholdWindowNotExpired(903)`, `RateLimitExceeded(18)` |
| Disputes and risk flags | `open_dispute`, `resolve_dispute`, `get_dispute`, `set_program_risk_flags`, `clear_program_risk_flags` | Dispute and risk-flag writers should require admin/payout-key authority; readers are public | `DisputeAlreadyOpen(600)`, `NoActiveDispute(601)`, `DisputeResolutionFailed(602)`, `DisputeOpeningFailed(603)`, `RiskFlagsUpdateFailed(105)` |
| Query and history filters | `query_payouts_by_recipient`, `query_payouts_by_amount`, `query_payouts_by_timestamp`, `get_payouts_by_recipient`, `get_archived_programs`, `get_trigger_schema_version`, `get_idempotency_schema_version` | Public read-only. Must enforce pagination and avoid unbounded storage iteration | `InvalidPaginationLimit(19)`, `PaginationLimitExceedsMax(20)`, `EntryNotFound(16)` |

Program-escrow batch-specific errors:

`NotInitialized(3100)`, `Paused(3101)`, `DisputeOpen(3102)`, `Unauthorized(3103)`, `LengthMismatch(3104)`, `EmptyBatch(3105)`, `ZeroAmount(3106)`, `AmountOverflow(3107)`, `SpendLimitExceeded(3108)`, `InsufficientBalance(3109)`, `CircuitBreakerOpen(3110)`, `DuplicateRecipient(3111)`, `FeeConsumesAmount(3112)`.

### `contracts/bounty_escrow`

| Area | Entrypoints | Expected access control | Associated errors |
| --- | --- | --- | --- |
| Payout wrapper | `payout` | Must be reviewed before production use: the current wrapper gates behavior on `gradual_payout_rollout` but does not show an explicit caller authorization check in `contracts/bounty_escrow/src/lib.rs` | No local `errors.rs` exists in this folder. Audit should either add a contract error enum or map failures to the shared registry in `contracts/grainlify-core/src/errors.rs` |
| Internal payout branches | `execute_new_payout_logic`, `execute_legacy_payout` | Referenced by `payout`; audit should confirm implementation location, transfer semantics, and authorization | Open remediation item because these helpers are referenced by the wrapper and must be proven in build/test scope |

### `contracts/grainlify-core`

Core coordinates admin, governance, upgrades, snapshots, registry, monitoring, and shared helper modules.

| Area | Entrypoints | Expected access control | Associated errors |
| --- | --- | --- | --- |
| Admin and initialization | `init_admin`, `init`, `init_with_network`, `init_governance`, `get_admin`, `get_chain_id`, `get_network_id`, `get_network_info`, `get_version`, `get_version_semver_string`, `get_version_numeric_encoded`, `require_min_version`, `set_version` | Initialization and version mutation require admin/signers; read methods are public | `AlreadyInitialized(1)`, `NotInitialized(2)`, `NotAdmin(3)`, shared `ALREADY_INITIALIZED(1)`, `NOT_INITIALIZED(2)`, `UNAUTHORIZED(3)` |
| Upgrade and migration | `propose_upgrade`, `approve_upgrade`, `cancel_upgrade`, `execute_upgrade`, `upgrade`, `get_upgrade_proposal`, `get_timelock_delay`, `set_timelock_delay`, `get_timelock_status`, `commit_migration`, `migrate`, `get_migration_state`, `get_previous_version`, `can_execute` | Multisig signer or admin authorization; timelocks and migration hash commitments must be enforced before WASM upgrade/migration | `ThresholdNotMet(101)`, `ProposalNotFound(102)`, `MigrationCommitmentNotFound(103)`, `MigrationHashMismatch(104)`, `TimelockDelayTooHigh(105)`, shared `THRESHOLD_NOT_MET(101)`, `PROPOSAL_NOT_FOUND(102)`, `UPGRADE_SAFETY_CHECK_FAILED(220)` |
| Config snapshots and rollback | `create_config_snapshot`, `list_config_snapshots`, `get_config_snapshot`, `get_latest_config_snapshot`, `get_snapshot_count`, `compare_snapshots`, `restore_config_snapshot`, `confirm_admin_restore`, `get_rollback_info`, `get_config_change_delay`, `set_config_change_delay`, `propose_config_snapshot_restore`, `get_config_change_proposal`, `get_config_change_status`, `cancel_config_change`, `execute_config_snapshot_restore` | Admin-only for writes and restore proposals; restoring admin requires explicit confirmation by proposed admin | `SnapshotRestoreAdminPending(106)`, `SnapshotPruned(107)`, shared `INVALID_STATE(13)` |
| Registry and liveness | `register_deployed_contract`, `deregister_deployed_contract`, `get_deployed_contract`, `deployed_contract_count`, `list_deployed_contracts`, `liveness_watchdog`, `ping_watchdog`, `get_liveness_schema_version`, `verify_storage_layout` | Admin-only registry writes; watchdog ping is admin-only; read checks are public | shared `INVALID_ADDRESS(15)`, `MAINTENANCE_MODE(9)`, `PAUSED(10)` |
| Monitoring | `track_operation`, `emit_performance`, `health_check`, `get_analytics`, `get_state_snapshot`, `get_performance_stats`, `check_invariants`, `verify_invariants`, `is_strict_mode`, `is_read_only`, `set_read_only_mode` | Mutating state toggles require admin; telemetry reads are public | shared `INVALID_STATE(13)`, `CONTRACT_DEPRECATED(8)` |
| Multisig helper module | `init`, `propose`, `approve`, `can_execute`, `mark_executed`, `cancel`, `pause`, `unpause`, `is_contract_paused`, `is_state_inconsistent`, `get_config_opt`, `is_cancelled`, `is_expired`, `get_proposal_opt`, `set_config`, `clear_config` | Signers authorize proposal, approval, cancellation, pause, and unpause | `MultiSigError` plus shared governance errors |
| Governance helper module | `init_governance_state`, `create_proposal`, `cast_vote`, `finalize_proposal`, `execute_proposal`, `get_config` | Admin initializes; proposers/voters authorize proposal creation and voting; execution requires approved status and delay | shared `PROPOSAL_NOT_FOUND(102)`, `PROPOSAL_NOT_ACTIVE(107)`, `ALREADY_VOTED(111)`, `PROPOSAL_NOT_APPROVED(112)`, `EXECUTION_DELAY_NOT_MET(113)`, `PROPOSAL_EXPIRED(114)` |
| Commit-reveal, nonce, and asset helpers | `create_commitment`, `verify_reveal`, `get_nonce`, `get_nonce_with_domain`, `validate_and_increment_nonce`, `validate_and_increment_nonce_with_domain`, `normalize_asset_id`, `validate_asset_id`, `token_client`, `balance`, `transfer_exact` | Reveal requires revealer authorization; nonce increments must be signer/domain bound; asset transfers rely on token contract authorization semantics | shared `INVALID_SIGNATURE(301)`, `CLAIM_EXPIRED(302)`, `INVALID_ASSET_ID(15)`, `INVALID_AMOUNT(4)`, `INSUFFICIENT_FUNDS(5)` |

## Shared Error Registry

`contracts/grainlify-core/src/errors.rs` is the cross-contract source for unique numeric codes:

| Range | Codes |
| --- | --- |
| Common | `ALREADY_INITIALIZED(1)`, `NOT_INITIALIZED(2)`, `UNAUTHORIZED(3)`, `INVALID_AMOUNT(4)`, `INSUFFICIENT_FUNDS(5)`, `DEADLINE_NOT_PASSED(6)`, `INVALID_DEADLINE(7)`, `CONTRACT_DEPRECATED(8)`, `MAINTENANCE_MODE(9)`, `PAUSED(10)`, `OVERFLOW(11)`, `UNDERFLOW(12)`, `INVALID_STATE(13)`, `NOT_PAUSED(14)`, `INVALID_ASSET_ID(15)` |
| Governance | `THRESHOLD_NOT_MET(101)`, `PROPOSAL_NOT_FOUND(102)`, `INVALID_THRESHOLD(103)`, `THRESHOLD_TOO_LOW(104)`, `INSUFFICIENT_STAKE(105)`, `PROPOSALS_NOT_FOUND(106)`, `PROPOSAL_NOT_ACTIVE(107)`, `VOTING_NOT_STARTED(108)`, `VOTING_ENDED(109)`, `VOTING_STILL_ACTIVE(110)`, `ALREADY_VOTED(111)`, `PROPOSAL_NOT_APPROVED(112)`, `EXECUTION_DELAY_NOT_MET(113)`, `PROPOSAL_EXPIRED(114)` |
| Escrow | `BOUNTY_EXISTS(201)`, `BOUNTY_NOT_FOUND(202)`, `FUNDS_NOT_LOCKED(203)`, `INVALID_FEE_RATE(204)`, `FEE_RECIPIENT_NOT_SET(205)`, `INVALID_BATCH_SIZE(206)`, `BATCH_SIZE_MISMATCH(207)`, `DUPLICATE_BOUNTY_ID(208)`, `REFUND_NOT_APPROVED(209)`, `AMOUNT_BELOW_MINIMUM(210)`, `AMOUNT_ABOVE_MAXIMUM(211)`, `CLAIM_PENDING(212)`, `TICKET_NOT_FOUND(213)`, `TICKET_ALREADY_USED(214)`, `TICKET_EXPIRED(215)`, `PARTICIPANT_BLOCKED(216)`, `PARTICIPANT_NOT_ALLOWED(217)`, `NOT_ANONYMOUS_ESCROW(218)`, `INVALID_SELECTION_INPUT(219)`, `UPGRADE_SAFETY_CHECK_FAILED(220)`, `BOUNTY_ALREADY_INITIALIZED(221)`, `ANON_REFUND_REQUIRED(222)`, `ANON_RESOLVER_NOT_SET(223)`, `NOT_ANON_VARIANT(224)`, `USE_INFO_V2_FOR_ANON(225)`, `INVALID_LABEL(226)`, `TOO_MANY_LABELS(227)`, `LABEL_NOT_ALLOWED(228)` |
| Identity and KYC | `INVALID_SIGNATURE(301)`, `CLAIM_EXPIRED(302)`, `UNAUTHORIZED_ISSUER(303)`, `INVALID_CLAIM_FORMAT(304)`, `TRANSACTION_EXCEEDS_LIMIT(305)`, `INVALID_RISK_SCORE(306)`, `INVALID_TIER(307)` |
| Program escrow shared | `PROGRAM_ALREADY_EXISTS(401)`, `DUPLICATE_PROGRAM_ID(402)`, `INVALID_BATCH_SIZE_PROGRAM(403)`, `PROGRAM_NOT_FOUND(404)`, `SCHEDULE_NOT_FOUND(405)`, `ALREADY_RELEASED(406)`, `FUNDS_PAUSED(407)`, `DUPLICATE_SCHEDULE_ID(408)` |
| Circuit breaker | `CIRCUIT_OPEN(1001)` |

## Threat Model

### Reentrancy

Risk areas:

- Token transfers in `lock_program_funds`, payout, split payout, claim execution, release triggers, and emergency withdrawal.
- State updates that occur before/after token client calls.
- Batch operations where one recipient or token behavior can affect later recipients.

Audit checks:

- Confirm every state-mutating transfer path either uses `reentrancy_guard` or updates accounting in an order that cannot be abused by callbacks.
- Confirm batch payout failure modes are atomic or explicitly recorded as partial recovery states.
- Confirm idempotency records are written in deterministic order and cannot be reused across conflicting recipients/amounts.

### Oracle Manipulation

The scoped contracts do not show direct price-oracle reads in the entrypoint inventory. The realistic oracle-like inputs are off-chain winner selection, governance decisions, metadata, risk flags, and timing.

Audit checks:

- Treat `authorized_payout_key`, admins, signers, and off-chain services as privileged oracle inputs.
- Validate no payout amount depends on mutable off-chain metadata without an authenticated on-chain decision.
- Ensure commit-reveal and pseudo-random selection helpers cannot be biased by late seeds, candidate ordering, or repeated reveals.

### Fee Drain

Risk areas:

- `update_fee_config`, fixed-plus-basis-point fee calculation, payout/lock fee collection, split payout, and emergency withdrawal.
- Fee-on-transfer assets where actual received amount differs from requested amount.

Audit checks:

- Ensure all fee rates are capped by `MAX_FEE_RATE` or stricter config.
- Ensure fixed fees plus percentage fees cannot consume the whole payout unless that is intentionally rejected.
- Validate fee recipient cannot be unset on a fee-enabled path.
- Confirm `remaining_balance`, token balance, and fee-collected counters remain consistent after every fee path.

### Authorization And Role Drift

Risk areas:

- Two-step admin/controller rotations.
- Delegate permissions.
- Explicit `*_by` methods that accept a caller parameter.
- Emergency and maintenance paths.

Audit checks:

- Verify every caller-supplied address is authenticated before it is treated as authority.
- Ensure old keys cannot act after rotation acceptance.
- Ensure pending role transitions expire deterministically.
- Confirm read-only and maintenance mode block all intended mutating entrypoints.

### Storage, Pagination, And Upgrade Safety

Risk areas:

- Large vectors for payout history, release schedules, registry entries, snapshots, and batch recovery records.
- Storage schema versions and backwards-compatible query methods.
- Upgrade and migration pathways.

Audit checks:

- Verify all list/query entrypoints enforce stable pagination or bounded iteration.
- Confirm schema version keys are initialized and readable after init.
- Confirm migration commitments are one-time-use and cannot replay across target versions.

## Pre-Audit Remediation Tracker

| ID | Item | Impact | Status | Notes |
| --- | --- | --- | --- | --- |
| PRE-AUDIT-001 | Draft state incomplete | Draft programs may have inconsistent release/payout behavior if all mutating entrypoints do not reject or handle draft state consistently | Open | Audit `ProgramStatus::Draft` branches across initialization, scheduling, claim, and payout paths |
| PRE-AUDIT-002 | Missing test imports | Tests can silently drift from the contract surface if new modules are not imported in `lib.rs` test configuration | Open | The checklist test added with this file protects the required audit document; follow-up should repair and re-enable currently broken modules such as `test_circuit_breaker_enforcement.rs`, then verify all `test_*.rs` modules are wired into the package |
| PRE-AUDIT-003 | `bounty_escrow` payout helper resolution | The wrapper references `execute_new_payout_logic` and `execute_legacy_payout`; auditors need the implementation or an explicit removal plan | Open | Confirm build scope and add explicit errors/auth checks before production use |
| PRE-AUDIT-004 | Access-control review for `*_by` entrypoints | Caller-parameter methods are higher risk because authority is data-driven | Open | Auditor should trace each `*_by` path to `require_auth` and delegate checks |
| PRE-AUDIT-005 | Fee-drain invariant suite | Fee configs span multiple payout paths | Open | Add invariant tests that fee plus payout never exceeds funded balance and fee recipient is always valid |

## Minimum Test Plan

Run before audit handoff:

```bash
cargo test -p program-escrow
```

Recommended follow-up matrix:

- `program-escrow`: full cargo test, targeted payout/schedule/claim/fee/circuit/role tests, fuzz-like boundary tests for batch sizes and fee math.
- `grainlify-core`: governance, multisig, migration, config snapshot, registry, liveness, and strict-mode tests.
- `bounty_escrow`: build/test once helper resolution and authorization are clarified.
