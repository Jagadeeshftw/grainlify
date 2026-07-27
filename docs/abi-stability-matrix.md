# Cross-Contract ABI Stability Matrix

> **Canonical reference** for the public surface area of all five Grainlify Soroban contracts.
> Every integrator, facade binding author, and contract upgrader **must** consult this document
> before changing a public function signature, adding or removing a struct field, or reordering
> enum variants.
>
> Last updated: 2026-07-27  
> Covers contracts at commit referenced in `contracts/VERSIONS.md`.

---

## Table of Contents

1. [Stability Classifications](#1-stability-classifications)
2. [Breaking vs Additive Changes](#2-breaking-vs-additive-changes)
3. [Synchronization-Risk Types](#3-synchronization-risk-types)
4. [Contract: program-escrow](#4-contract-program-escrow)
5. [Contract: bounty-escrow](#5-contract-bounty-escrow)
6. [Contract: grainlify-core](#6-contract-grainlify-core)
7. [Contract: view-facade](#7-contract-view-facade)
8. [Contract: escrow-view-facade](#8-contract-escrow-view-facade)
9. [Cross-Contract Dependency Graph](#9-cross-contract-dependency-graph)
10. [Upgrade Checklist](#10-upgrade-checklist)

---

## 1. Stability Classifications

Each public entry-point and exported type carries one of three labels:

| Label | Meaning |
|---|---|
| **STABLE** | Frozen interface. Breaking changes require a semver-major bump, a migration path, and updates to all facade bindings that mirror the type. Announce changes at least one release cycle in advance. |
| **EVOLVING** | Under active development. Signature may change across minor releases. Downstream callers should pin to a specific contract hash and re-test on upgrade. |
| **INTERNAL** | Exposed `pub` for Rust crate-level visibility or test harnesses only. Not part of the on-chain ABI from an integrator's perspective; may change without notice. |

---

## 2. Breaking vs Additive Changes

### 2.1 Breaking Changes (require coordinated update across all affected bindings)

- Removing a `pub fn` entrypoint
- Renaming a `pub fn` entrypoint
- Changing parameter types, order, or count of any `pub fn`
- Changing the return type of any `pub fn`
- Removing a field from a `#[contracttype]` struct
- Reordering fields in a `#[contracttype]` struct (XDR serialization is field-order-dependent)
- Removing a variant from a `#[contracttype]` enum
- Reordering variants in a `#[contracttype]` enum
- Changing a field type in a `#[contracttype]` struct

### 2.2 Additive Changes (non-breaking, but require facade-binding review)

- Adding a new `pub fn` entrypoint
- Adding a new `#[contracttype]` struct or enum (new storage key is required)
- Appending a new field to a `#[contracttype]` struct **with a corresponding schema-version bump** so old and new storage layouts can coexist
- Appending a new variant to a `#[contracttype]` enum (only safe if all match arms in all callers use a wildcard/catch-all)
- Adding a new optional `Option<T>` field to a struct that is versioned

### 2.3 Facade Binding Synchronization Rule

Any type listed in [§3 Synchronization-Risk Types](#3-synchronization-risk-types) that changes in
its canonical contract **must** be updated in every facade binding file that copies it **in the
same PR**. Partial updates will cause silent XDR decode errors at runtime — there is no compile-time
guard between contracts on Soroban.

---

## 3. Synchronization-Risk Types

These types are defined in a canonical contract **and** duplicated verbatim in one or more facade
binding files. Any field addition, removal, reorder, or type change in the canonical definition
is a **breaking synchronization risk**.

| Type | Canonical Source | Duplicated In | Risk Level |
|---|---|---|---|
| `PayoutRecord` | `program-escrow/src/lib.rs` | `view-facade/src/lib.rs` (local copy) | 🔴 HIGH — different field sets; view-facade omits `payout_type` |
| `ProgramDelegateInfo` | `program-escrow/src/lib.rs` | `escrow-view-facade/src/program_escrow_bindings.rs` | 🔴 HIGH — exact mirror; field reorder will silently corrupt |
| `EscrowStatus` (enum) | `bounty-escrow/contracts/escrow/src/lib.rs` | `escrow-view-facade/src/bounty_escrow_bindings.rs`, `escrow-view-facade/src/lib.rs` (re-exported as local `EscrowStatus`) | 🔴 HIGH — variant order must match exactly |
| `EscrowMetadata` | `bounty-escrow/contracts/escrow/src/lib.rs` | `escrow-view-facade/src/bounty_escrow_bindings.rs` | 🔴 HIGH — binding uses subset of fields |
| `PauseFlags` | `bounty-escrow/contracts/escrow/src/lib.rs` | `escrow-view-facade/src/bounty_escrow_bindings.rs` | 🔴 HIGH — binding copies all fields; `pause_reason` is `Option<String>` |
| `Escrow` | `bounty-escrow/contracts/escrow/src/lib.rs` | `escrow-view-facade/src/bounty_escrow_bindings.rs` | 🟡 MEDIUM — binding omits optional fields present in canonical |
| `ProgramMetadata` / `ProgramMetadataField` | `program-escrow/src/lib.rs` | Not yet in any binding — risk emerges if facades add metadata queries | 🟢 LOW (today) — becomes HIGH if view-facade adds metadata endpoint |

### 3.1 Detailed Field Diff: PayoutRecord

```
Canonical (program-escrow)          view-facade local copy
─────────────────────────────       ───────────────────────
pub recipient: Address              pub recipient: Address        ✓
pub amount: i128                    pub amount: i128              ✓
pub timestamp: u64                  pub timestamp: u64            ✓
pub payout_type: PayoutType         (MISSING)                     ⚠ DRIFT
```

The view-facade's `PayoutRecord` is a **subset** of the canonical struct. Any consumer receiving
the facade's `PayoutRecord` will not have `payout_type`. If the canonical struct reorders or removes
`timestamp`, XDR decoding in the facade silently produces wrong values.

### 3.2 Detailed Field Diff: EscrowStatus

```
Canonical (bounty-escrow)           bounty_escrow_bindings.rs     escrow-view-facade local
─────────────────────────           ─────────────────────────     ────────────────────────
Locked          = 0                 Locked          = 0           Locked          = 0
Released        = 1                 Released        = 1           Released        = 1
Refunded        = 2                 Refunded        = 2           Refunded        = 2
PartiallyRefunded = 3               PartiallyRefunded = 3         PartiallyRefunded = 3
```

Currently in sync. Adding a new variant to canonical (e.g. `Disputed`) is an **additive** change
only if the facade match arms include a catch-all. Currently they do not — they exhaustively match
all four variants. Adding a fifth variant to the canonical enum is therefore **breaking** for both
facade copies until they are updated simultaneously.

### 3.3 Detailed Field Diff: ProgramDelegateInfo

```
Canonical (program-escrow)          program_escrow_bindings.rs
─────────────────────────           ──────────────────────────
pub program_id: String              pub program_id: String        ✓
pub delegate: Option<Address>       pub delegate: Option<Address> ✓
pub permissions: u32                pub permissions: u32          ✓
```

Currently in sync. Any field addition to the canonical struct must be mirrored in the binding
within the same PR, or the `query_all_delegates` call will return garbage for the new field.

---

## 4. Contract: program-escrow

**Crate:** `contracts/program-escrow`  
**Contract struct:** `ProgramEscrowContract`  
**Purpose:** Manages hackathon and grant prize pools — fund locking, batch/single payouts, release schedules, and delegate authorization.

### 4.1 Key Exported Types

| Type | Stability | Notes |
|---|---|---|
| `ProgramData` | STABLE | Core program state; field reorder is breaking. `circuit_breaker_threshold` field has an unresolved merge conflict between `Option<u8>` and `Option<u32>` — must be resolved before next deployment. |
| `PayoutRecord` | STABLE | Mirrored in view-facade (with drift — see §3.1). |
| `ProgramMetadata` / `ProgramMetadataField` | EVOLVING | `custom_fields` vector; length now capped at `MAX_CUSTOM_FIELDS=20`. |
| `ProgramDelegateInfo` | STABLE | Mirrored in escrow-view-facade binding (see §3.3). |
| `PauseFlags` | STABLE | All four fields are load-bearing; `pause_reason` is `Option<String>`. |
| `ProgramStatus` (enum) | STABLE | `Draft → Active → Drained`; variant removal is breaking. |
| `FeeConfig` | EVOLVING | Fee configuration; `fee_waivers` bitmask may gain new bits. |
| `RateLimitConfig` | INTERNAL | Global rate-limit for anti-abuse; not exposed to downstream integrators. |
| `DelegateMetaRateLimitState` | INTERNAL | Per-program rolling window counter; opaque to callers. |
| `SplitConfig` / `BeneficiarySplit` | EVOLVING | Payout-split configuration; schema version tracked. |
| `BatchReceipt` | EVOLVING | Idempotency receipts; schema version tracked. |
| `ClaimRecord` / `ClaimStatus` | EVOLVING | Pending-claim workflow; under active development. |
| `DisputeRecord` / `DisputeState` | EVOLVING | Dispute resolution; single active dispute per contract. |
| `AllowedTokenEntry` | EVOLVING | Token allowlist with decimals; V2 schema. |
| `ProgramReleaseSchedule` / `ProgramReleaseHistory` | STABLE | Release schedule system; adding fields requires schema bump. |
| `AnonymousResolver` | EVOLVING | Privacy feature; may gain additional fields. |

### 4.2 Public Entry-Points

#### Initialization & Program Lifecycle

| Function | Signature | Stability | Notes |
|---|---|---|---|
| `initialize_contract` | `(env: Env, admin: Address)` | STABLE | One-time init; idempotent guard. |
| `init_program` | `(env, program_id, authorized_payout_key, token_address, creator, initial_liquidity, reference_hash) -> ProgramData` | STABLE | Creates a program in Draft state. |
| `init_program_with_metadata` | `(env, program_id, authorized_payout_key, token_address, organizer, metadata) -> ProgramData` | STABLE | Init with optional metadata. |
| `initialize_program` | `(env, ...) -> ProgramData` | INTERNAL | Alias/internal variant; do not rely on externally. |
| `batch_initialize_programs` | `(env, items: Vec<...>) -> Result<u32, BatchError>` | EVOLVING | Batch init; error semantics may change. |
| `publish_program` | `(env, program_id, caller) -> ProgramData` | STABLE | Transitions Draft → Active. |
| `lock_program_funds` | `(env, amount: i128) -> ProgramData` | STABLE | Legacy single-program lock. |
| `lock_program_funds_v2` | `(env, program_id, amount) -> ProgramData` | STABLE | Multi-program lock; preferred path. |
| `batch_lock` | `(env, items: Vec<LockItem>) -> Result<u32, BatchError>` | EVOLVING | Batch lock; may gain idempotency key. |
| `archive_program` | `(env, program_id)` | STABLE | Admin-only soft delete. |
| `get_archived_programs` | `(env) -> Vec<String>` | STABLE | Returns archived program IDs. |

#### Payout Operations

| Function | Signature | Stability | Notes |
|---|---|---|---|
| `single_payout` | `(env, program_id, recipient, amount) -> ProgramData` | STABLE | Single transfer; circuit-breaker checked. |
| `single_payout_by` | `(env, program_id, caller, recipient, amount) -> ProgramData` | STABLE | Caller-explicit variant. |
| `single_payout_v2` | `(env, program_id, caller, recipient, amount, ...) -> ProgramData` | EVOLVING | V2 with extended options. |
| `single_payout_idempotent` | `(env, program_id, recipient, amount, idempotency_key) -> ProgramData` | EVOLVING | Idempotency-keyed single payout. |
| `single_payout_idempotent_by` | `(env, ..., caller, ...) -> ProgramData` | EVOLVING | Caller-explicit idempotent. |
| `batch_payout` | `(env, recipients: Vec<Address>, amounts: Vec<i128>) -> ProgramData` | STABLE | Legacy batch; circuit-breaker checked. |
| `batch_payout_by` | `(env, program_id, caller, recipients, amounts) -> ProgramData` | STABLE | Multi-program batch. |
| `batch_payout_v2` | `(env, ...) -> ProgramData` | EVOLVING | V2 batch with extended options. |
| `batch_payout_idempotent` | `(env, ..., idempotency_key) -> ProgramData` | EVOLVING | Idempotency-keyed batch. |
| `batch_payout_idempotent_by` | `(env, ...) -> ProgramData` | EVOLVING | Caller-explicit idempotent batch. |
| `batch_payout_with_receipt` | `(env, ...) -> ProgramData` | EVOLVING | Returns receipt ID. |
| `batch_release` | `(env, items: Vec<ReleaseItem>) -> Result<u32, BatchError>` | EVOLVING | Batch release variant. |
| `execute_split_payout` | `(env, program_id, caller, amount) -> SplitPayoutResult` | EVOLVING | Splits amount per `SplitConfig`. |
| `preview_split` | `(env, program_id, amount) -> SplitPayoutResult` | EVOLVING | Dry-run; no state change. |

#### Release Schedules

| Function | Signature | Stability | Notes |
|---|---|---|---|
| `create_program_release_schedule` | `(env, program_id, recipient, amount, release_at)` | STABLE | Creates a time-locked schedule. |
| `create_prog_release_schedule_by` | `(env, program_id, caller, ...)` | STABLE | Caller-explicit variant. |
| `trigger_program_releases` | `(env) -> u32` | STABLE | Processes all due schedules. |
| `trigger_program_releases_by` | `(env, caller) -> u32` | STABLE | Caller-explicit trigger. |
| `get_program_release_schedules` | `(env) -> Vec<ProgramReleaseSchedule>` | STABLE | Legacy single-program query. |
| `get_release_schedules` | `(env) -> Vec<ProgramReleaseSchedule>` | STABLE | Alias. |
| `get_program_release_history` | `(env) -> Vec<ProgramReleaseHistory>` | STABLE | History of triggered releases. |

#### Delegate & Access Control

| Function | Signature | Stability | Notes |
|---|---|---|---|
| `set_program_delegate` | `(env, program_id, caller, delegate, permissions: u32) -> ProgramData` | STABLE | Sets delegate with permission bitmask. |
| `revoke_program_delegate` | `(env, program_id, caller) -> ProgramData` | STABLE | Clears delegate. |
| `emergency_revoke_delegate` | `(env, program_id) -> ProgramData` | STABLE | Admin emergency revoke. |
| `propose_controller` | `(env, program_id, caller, proposed) -> Result<ProgramData, ContractError>` | EVOLVING | Two-step controller rotation step 1. |
| `accept_controller` | `(env, program_id) -> Result<ProgramData, ContractError>` | EVOLVING | Two-step controller rotation step 2. |
| `cancel_controller_rotation` | `(env, program_id, caller) -> Result<ProgramData, ContractError>` | EVOLVING | Cancel pending rotation. |
| `rotate_payout_key` | `(env, program_id, caller, new_key, nonce) -> ProgramData` | EVOLVING | Nonce-protected key rotation. |
| `get_rotation_nonce` | `(env, program_id) -> u64` | STABLE | Read rotation nonce. |
| `propose_admin` | `(env, proposed_admin) -> Result<(), ContractError>` | STABLE | Two-step admin rotation step 1. |
| `accept_admin` | `(env) -> Result<(), ContractError>` | STABLE | Two-step admin rotation step 2. |
| `cancel_admin_rotation` | `(env) -> Result<(), ContractError>` | STABLE | Cancel pending admin rotation. |

#### Metadata & Risk

| Function | Signature | Stability | Notes |
|---|---|---|---|
| `update_program_metadata` | `(env, program_id, caller, metadata: ProgramMetadata) -> ProgramData` | STABLE | Delegate-invoked calls are rate-limited (≤10/hr). `custom_fields` capped at 20 entries. |
| `update_program_metadata_by` | `(env, program_id, caller, metadata) -> ProgramData` | STABLE | Alias; same rate-limit applies. |
| `get_program_metadata` | `(env, program_id) -> Option<ProgramMetadata>` | STABLE | Read metadata. |
| `set_program_risk_flags` | `(env, program_id, flags: u32) -> ProgramData` | STABLE | Admin-only; bitmask. |
| `clear_program_risk_flags` | `(env, program_id, flags: u32) -> ProgramData` | STABLE | Admin-only; clears specified bits. |

#### Pause, Emergency & Maintenance

| Function | Signature | Stability | Notes |
|---|---|---|---|
| `set_paused` | `(env, lock_paused, release_paused, refund_paused, reason) -> PauseFlags` | STABLE | Granular pause flags. |
| `emergency_withdraw` | `(env, target: Address)` | STABLE | Admin-only; requires lock_paused=true. |
| `set_maintenance_mode` | `(env, enabled: bool)` | STABLE | Blocks lock only. |
| `is_maintenance_mode` | `(env) -> bool` | STABLE | Read maintenance flag. |
| `set_read_only_mode` | `(env, enabled, reason)` | STABLE | Blocks all writes. |
| `is_read_only` | `(env) -> bool` | STABLE | |

#### Fee Configuration

| Function | Signature | Stability | Notes |
|---|---|---|---|
| `get_fee_config` | `(env) -> FeeConfig` | STABLE | |
| `update_fee_config` | `(env, ...)` | STABLE | Admin-only. |
| `set_program_spending_limit` | `(env, program_id, ...)` | EVOLVING | Per-program spend cap. |
| `get_program_spending_limit` | `(env, program_id) -> i128` | EVOLVING | |
| `set_program_spend_threshold` | `(env, program_id, threshold_amount)` | EVOLVING | Alert threshold (separate from hard cap). |
| `update_rate_limit_config` | `(env, window_size, max_operations, cooldown_period)` | INTERNAL | Global anti-abuse rate limit; not for integrators. |
| `get_rate_limit_config` | `(env) -> RateLimitConfig` | INTERNAL | |

#### Token Allowlist

| Function | Signature | Stability | Notes |
|---|---|---|---|
| `add_allowed_token` | `(env, token: Address)` | STABLE | Adds to V1 allowlist. |
| `add_allowed_token_with_decimals` | `(env, token, decimals: u32)` | STABLE | V2 allowlist with decimal normalization. |
| `remove_allowed_token` | `(env, token: Address)` | STABLE | |
| `is_token_allowed` | `(env, token: Address) -> bool` | STABLE | |
| `get_allowed_tokens` | `(env) -> Vec<Address>` | STABLE | V1 list. |
| `get_allowed_tokens_with_decimals` | `(env) -> Vec<AllowedTokenEntry>` | STABLE | V2 list. |

#### Split Configuration

| Function | Signature | Stability | Notes |
|---|---|---|---|
| `set_split_config` | `(env, program_id, config: SplitConfig)` | EVOLVING | |
| `get_split_config` | `(env, program_id) -> Option<SplitConfig>` | EVOLVING | |
| `disable_split_config` | `(env, program_id)` | EVOLVING | |

#### Circuit Breaker

| Function | Signature | Stability | Notes |
|---|---|---|---|
| `set_circuit_admin` | `(env, new_admin, caller)` | INTERNAL | Circuit breaker admin management. |
| `get_circuit_admin` | `(env) -> Option<Address>` | INTERNAL | |
| `get_circuit_breaker_status` | `(env) -> CircuitBreakerStatus` | INTERNAL | |
| `reset_circuit_breaker` | `(env, caller)` | INTERNAL | |
| `configure_circuit_breaker` | `(env, ...)` | INTERNAL | |
| `emergency_open_circuit` | `(env, admin)` | INTERNAL | |
| `set_program_cb_threshold` | `(env, program_id, threshold)` | EVOLVING | Per-program circuit breaker override. |

#### Query & Analytics

| Function | Signature | Stability | Notes |
|---|---|---|---|
| `get_program_info` | `(env) -> ProgramData` | STABLE | Legacy single-program read. |
| `get_program_info_v2` | `(env, program_id) -> ProgramData` | STABLE | Multi-program read. |
| `get_remaining_balance` | `(env) -> i128` | STABLE | |
| `get_analytics` | `(env) -> Analytics` | EVOLVING | |
| `get_program_analytics` | `(env) -> Analytics` | EVOLVING | |
| `query_payouts_by_recipient` | `(env, program_id, recipient, offset, limit) -> Vec<PayoutRecord>` | STABLE | Paginated payout index. |
| `query_recipient_history` | `(env, program_id, recipient) -> Vec<PayoutRecord>` | STABLE | Full history; used by view-facade. |
| `query_payouts_by_amount` | `(env, program_id, min, max, offset, limit) -> Vec<PayoutRecord>` | EVOLVING | |
| `get_batch_receipt` | `(env, receipt_id) -> Option<BatchReceipt>` | EVOLVING | |
| `is_payout_processed` | `(env, idempotency_key) -> bool` | STABLE | |
| `get_program_metadata` | `(env, program_id) -> Option<ProgramMetadata>` | STABLE | |
| `program_exists` | `(env) -> bool` | STABLE | Legacy. |
| `program_exists_by_id` | `(env, program_id) -> bool` | STABLE | |

---

## 5. Contract: bounty-escrow

**Crate:** `contracts/bounty_escrow/contracts/escrow`  
**Contract struct:** `BountyEscrowContract`  
**Purpose:** Manages individual bounty escrows — fund locking per bounty, contributor release, refund workflows, capability-based authorization, and multi-token support.

### 5.1 Key Exported Types

| Type | Stability | Notes |
|---|---|---|
| `Escrow` | STABLE | Core bounty state. Mirrored in `bounty_escrow_bindings.rs`. Field reorder is breaking. |
| `EscrowStatus` (enum) | STABLE | 4 variants; exhaustive matches in facade — adding variant is breaking until facades updated. |
| `EscrowMetadata` | STABLE | Mirrored in `bounty_escrow_bindings.rs` (partial copy). |
| `PauseFlags` | STABLE | Mirrored in `bounty_escrow_bindings.rs`. `pause_reason: Option<String>` is load-bearing. |
| `EscrowWithId` | EVOLVING | Wrapper used by `query_escrows_by_depositor`; mirrored in binding. |
| `FeeConfig` | EVOLVING | Fee configuration; separate from program-escrow's `FeeConfig` — different type. |
| `ClaimRecord` / `ClaimTicket` | EVOLVING | Claim workflow types. |
| `Capability` / `CapabilityAction` | EVOLVING | Capability-based auth; may gain new action variants. |
| `SimulationResult` | EVOLVING | Dry-run result. |
| `ParticipantFilterMode` | EVOLVING | Whitelist/blocklist mode enum. |
| `AdminRotationStatus` / `AdminRotationConfig` | STABLE | Two-step admin rotation state. |
| `RefundEligibilityView` | EVOLVING | Refund eligibility check result. |
| `FreezeRecord` | EVOLVING | Per-escrow and per-address freeze state. |
| `TreasuryDestination` / `PerBountyFeeRouting` | EVOLVING | Multi-region treasury routing. |
| `AnonymousParty` (enum) | EVOLVING | `Address(Address)` or `Commitment(BytesN<32>)`; mirrored in binding. |

### 5.2 Public Entry-Points

#### Initialization

| Function | Signature | Stability | Notes |
|---|---|---|---|
| `init` | `(env, admin: Address, token: Address) -> Result<(), Error>` | STABLE | One-time init. |
| `init_with_network` | `(env, admin, token, chain_id, network_id) -> Result<(), Error>` | EVOLVING | Network-aware init. |

#### Fund Locking

| Function | Signature | Stability | Notes |
|---|---|---|---|
| `lock_funds` | `(env, bounty_id, depositor, amount, deadline, ...) -> Result<Escrow, Error>` | STABLE | Core lock operation. |
| `lock_funds_anonymous` | `(env, bounty_id, commitment, amount, deadline, ...) -> Result<AnonymousEscrow, Error>` | EVOLVING | Privacy-preserving lock. |
| `dry_run_lock` | `(env, bounty_id, ...) -> SimulationResult` | EVOLVING | Dry-run; no state change. |
| `publish` | `(env, bounty_id) -> Result<(), Error>` | STABLE | Makes escrow visible. |

#### Release & Refund

| Function | Signature | Stability | Notes |
|---|---|---|---|
| `release_funds` | `(env, bounty_id, contributor) -> Result<(), Error>` | STABLE | Standard release to contributor. |
| `release_with_conversion` | `(env, bounty_id, contributor, ...) -> Result<(), Error>` | EVOLVING | Release with AMM conversion. |
| `release_with_capability` | `(env, bounty_id, contributor, capability_id) -> Result<(), Error>` | EVOLVING | Capability-gated release. |
| `partial_release` | `(env, bounty_id, contributor, amount) -> Result<(), Error>` | EVOLVING | Partial amount release. |
| `dry_run_release` | `(env, bounty_id, contributor) -> SimulationResult` | EVOLVING | Dry-run release. |
| `refund` | `(env, bounty_id) -> Result<(), Error>` | STABLE | Standard refund to depositor. |
| `dry_run_refund` | `(env, bounty_id) -> SimulationResult` | EVOLVING | Dry-run refund. |
| `approve_refund` | `(env, bounty_id, ...) -> Result<(), Error>` | EVOLVING | Admin-approved refund path. |
| `approve_large_release` | `(env, bounty_id, ...) -> Result<(), Error>` | EVOLVING | Multisig-approved large release. |
| `archive_escrow` | `(env, bounty_id) -> Result<(), Error>` | STABLE | Soft-delete a settled escrow. |

#### Claim Workflow

| Function | Signature | Stability | Notes |
|---|---|---|---|
| `authorize_claim` | `(env, bounty_id, contributor, amount, ...) -> Result<(), Error>` | EVOLVING | Creates a pending claim. |
| `claim` | `(env, bounty_id) -> Result<(), Error>` | EVOLVING | Contributor executes claim. |
| `claim_with_capability` | `(env, bounty_id, capability_id) -> Result<(), Error>` | EVOLVING | Capability-gated claim. |
| `cancel_pending_claim` | `(env, bounty_id) -> Result<(), Error>` | EVOLVING | Admin cancels pending claim. |
| `get_pending_claim` | `(env, bounty_id) -> Result<ClaimRecord, Error>` | EVOLVING | Read pending claim. |

#### Capability System

| Function | Signature | Stability | Notes |
|---|---|---|---|
| `issue_capability` | `(env, bounty_id, action, ...) -> Result<BytesN<32>, Error>` | EVOLVING | Issues a one-time capability token. |
| `revoke_capability` | `(env, capability_id) -> Result<(), Error>` | EVOLVING | Admin revokes a capability. |
| `get_capability` | `(env, capability_id) -> Result<Capability, Error>` | EVOLVING | Read capability state. |

#### Participant Filtering

| Function | Signature | Stability | Notes |
|---|---|---|---|
| `set_whitelist` | `(env, address, whitelisted) -> Result<(), Error>` | STABLE | |
| `set_whitelist_entry` | `(env, address, whitelisted) -> Result<(), Error>` | STABLE | Paginated variant. |
| `set_blocklist` | `(env, address, blocked) -> Result<(), Error>` | STABLE | |
| `set_blocklist_entry` | `(env, address, blocked) -> Result<(), Error>` | STABLE | |
| `set_filter_mode` | `(env, mode: ParticipantFilterMode) -> Result<(), Error>` | EVOLVING | |
| `get_filter_mode` | `(env) -> ParticipantFilterMode` | EVOLVING | |
| `query_whitelist` | `(env, offset, limit) -> ParticipantListPage` | EVOLVING | Paginated whitelist read. |
| `query_blocklist` | `(env, offset, limit) -> ParticipantListPage` | EVOLVING | |

#### Pause, Freeze & Emergency

| Function | Signature | Stability | Notes |
|---|---|---|---|
| `set_paused` | `(env, lock_paused, release_paused, refund_paused, reason) -> PauseFlags` | STABLE | Mirrored in binding. |
| `emergency_withdraw` | `(env, target) -> Result<(), Error>` | STABLE | Admin-only; requires lock_paused. |
| `freeze_escrow` | `(env, bounty_id, reason) -> Result<(), Error>` | EVOLVING | Freeze a specific escrow. |
| `unfreeze_escrow` | `(env, bounty_id) -> Result<(), Error>` | EVOLVING | |
| `freeze_address` | `(env, address, reason)` | EVOLVING | Freeze a participant address. |
| `unfreeze_address` | `(env, address) -> Result<(), Error>` | EVOLVING | |

#### Admin Rotation

| Function | Signature | Stability | Notes |
|---|---|---|---|
| `propose_admin` | `(env, new_admin)` | STABLE | Step 1 of two-step rotation. |
| `accept_admin` | `(env)` | STABLE | Step 2 — new admin accepts. |
| `cancel_admin_transfer` | `(env)` | STABLE | Cancel pending rotation. |
| `propose_admin_rotation` | `(env, new_admin) -> Result<u64, Error>` | EVOLVING | Timelock-enforced variant. |
| `accept_admin_rotation` | `(env) -> Result<Address, Error>` | EVOLVING | |
| `cancel_admin_rotation` | `(env) -> Result<(), Error>` | EVOLVING | |
| `set_rotation_timelock_duration` | `(env, duration) -> Result<(), Error>` | EVOLVING | |

#### Queries

| Function | Signature | Stability | Notes |
|---|---|---|---|
| `get_escrow_info` | `(env, bounty_id) -> Result<Escrow, Error>` | STABLE | Used by escrow-view-facade binding. |
| `get_metadata` | `(env, bounty_id) -> EscrowMetadata` | STABLE | Used by escrow-view-facade binding. |
| `get_pause_flags` | `(env) -> PauseFlags` | STABLE | Used by escrow-view-facade binding. |
| `get_balance` | `(env) -> i128` | STABLE | Contract-level token balance. |
| `get_archived_escrows` | `(env) -> Vec<u64>` | STABLE | |
| `get_fee_config` | `(env) -> FeeConfig` | EVOLVING | |
| `get_refund_eligibility` | `(env, bounty_id) -> ...` | EVOLVING | |
| `get_refund_eligibility_view` | `(env, bounty_id) -> RefundEligibilityView` | EVOLVING | |
| `get_admin_rotation_status` | `(env) -> Option<AdminRotationStatus>` | EVOLVING | |

---

## 6. Contract: grainlify-core

**Crate:** `contracts/grainlify-core`  
**Contract struct:** `GrainlifyContract`  
**Purpose:** Contract upgrade management with timelocked proposals, version tracking, governance, config snapshots/rollback, contract registry, and liveness watchdog.

### 6.1 Key Exported Types

| Type | Stability | Notes |
|---|---|---|
| `ContractError` | STABLE | Error discriminants; removing or renumbering variants is breaking. |
| `CoreConfigSnapshot` | EVOLVING | Snapshot of on-chain config; fields may grow with new governance features. |
| `SnapshotDiff` | EVOLVING | Result of `compare_snapshots`; format may change. |
| `RollbackInfo` | EVOLVING | State before last rollback. |
| `MigrationState` | EVOLVING | Tracks in-progress migration. |
| `UpgradeProposalRecord` | EVOLVING | Governance upgrade proposal. |
| `DeployedContract` | EVOLVING | Registry entry; `ContractKind` enum may gain new variants. |
| `ContractKind` (enum) | EVOLVING | Adding a variant is additive only if consumers handle unknown kinds. |
| `LivenessStatus` | EVOLVING | Watchdog output. |
| `GovernanceConfig` | EVOLVING | Multisig/governance parameters. |

### 6.2 Public Entry-Points

#### Initialization

| Function | Signature | Stability | Notes |
|---|---|---|---|
| `init_admin` | `(env, admin: Address)` | STABLE | Single-admin init path. |
| `init` | `(env, signers: Vec<Address>, threshold: u32)` | STABLE | Multisig init path. |
| `init_with_network` | `(env, admin, chain_id, network_id)` | EVOLVING | Network-aware init. |
| `init_governance` | `(env, admin, config: GovernanceConfig)` | EVOLVING | Governance init path. |

#### Upgrade Management

| Function | Signature | Stability | Notes |
|---|---|---|---|
| `propose_upgrade` | `(env, proposer, wasm_hash, expiry) -> u64` | STABLE | Returns proposal_id. |
| `approve_upgrade` | `(env, proposal_id, signer)` | STABLE | Multisig approval. |
| `cancel_upgrade` | `(env, proposal_id, canceller)` | STABLE | |
| `execute_upgrade` | `(env, proposal_id)` | STABLE | Executes after timelock. |
| `upgrade` | `(env, new_wasm_hash: BytesN<32>)` | STABLE | Direct upgrade (admin-only, no timelock). |
| `get_upgrade_proposal` | `(env, proposal_id) -> Option<UpgradeProposalRecord>` | STABLE | |
| `get_timelock_delay` | `(env) -> u64` | STABLE | Default 24 h; min 1 h; max 30 d. |
| `set_timelock_delay` | `(env, delay_seconds)` | STABLE | Admin-only; bounded 1 h–30 d. |
| `get_timelock_status` | `(env, proposal_id) -> Option<u64>` | STABLE | Seconds remaining. |
| `can_execute` | `(env, proposal_id) -> bool` | STABLE | |

#### Migration

| Function | Signature | Stability | Notes |
|---|---|---|---|
| `commit_migration` | `(env, target_version, hash, expires_at)` | STABLE | Hash commitment step. |
| `migrate` | `(env, target_version, migration_hash)` | STABLE | Executes migration after commitment. |
| `get_migration_state` | `(env) -> Option<MigrationState>` | STABLE | |
| `get_previous_version` | `(env) -> Option<u32>` | STABLE | |

#### Config Snapshots & Rollback

| Function | Signature | Stability | Notes |
|---|---|---|---|
| `create_config_snapshot` | `(env) -> u64` | EVOLVING | Returns snapshot_id. |
| `list_config_snapshots` | `(env) -> Vec<CoreConfigSnapshot>` | EVOLVING | |
| `get_config_snapshot` | `(env, snapshot_id) -> Option<CoreConfigSnapshot>` | EVOLVING | |
| `get_latest_config_snapshot` | `(env) -> Option<CoreConfigSnapshot>` | EVOLVING | |
| `compare_snapshots` | `(env, from_id, to_id) -> SnapshotDiff` | EVOLVING | |
| `restore_config_snapshot` | `(env, snapshot_id)` | EVOLVING | Admin-only. |
| `propose_config_snapshot_restore` | `(env, snapshot_id) -> u64` | EVOLVING | Timelocked restore. |
| `execute_config_snapshot_restore` | `(env, proposal_id)` | EVOLVING | |
| `cancel_config_change` | `(env, proposal_id)` | EVOLVING | |
| `confirm_admin_restore` | `(env, snapshot_id)` | EVOLVING | Two-step admin confirmation. |
| `get_rollback_info` | `(env) -> RollbackInfo` | EVOLVING | |

#### Version & Read-Only

| Function | Signature | Stability | Notes |
|---|---|---|---|
| `get_version` | `(env) -> u32` | STABLE | Current numeric version. |
| `get_version_semver_string` | `(env) -> String` | EVOLVING | Human-readable semver. |
| `get_version_numeric_encoded` | `(env) -> u32` | EVOLVING | Packed major/minor/patch. |
| `set_version` | `(env, new_version: u32)` | STABLE | Admin-only. |
| `require_min_version` | `(env, min_numeric)` | EVOLVING | Panics if version < minimum. |
| `is_read_only` | `(env) -> bool` | STABLE | |
| `set_read_only_mode` | `(env, enabled)` | STABLE | Admin-only. |
| `verify_storage_layout` | `(env) -> bool` | INTERNAL | Storage schema validation; test use. |

#### Contract Registry

| Function | Signature | Stability | Notes |
|---|---|---|---|
| `register_deployed_contract` | `(env, address, kind, version, ...)` | EVOLVING | Registers a contract in the core registry. |
| `deregister_deployed_contract` | `(env, address)` | EVOLVING | |
| `get_deployed_contract` | `(env, address) -> Option<DeployedContract>` | EVOLVING | |
| `deployed_contract_count` | `(env) -> u32` | EVOLVING | |
| `list_deployed_contracts` | `(env, offset, limit) -> Vec<DeployedContract>` | EVOLVING | |

#### Liveness & Pause

| Function | Signature | Stability | Notes |
|---|---|---|---|
| `pause` | `(env, signer)` | STABLE | |
| `unpause` | `(env, signer)` | STABLE | |
| `is_paused` | `(env) -> bool` | STABLE | |
| `ping_watchdog` | `(env)` | EVOLVING | Keeps liveness watchdog alive. |
| `liveness_watchdog` | `(env) -> LivenessStatus` | EVOLVING | Returns current liveness state. |

#### Governance

| Function | Signature | Stability | Notes |
|---|---|---|---|
| `init_governance` | `(env, admin, config)` | EVOLVING | |
| `propose_upgrade` | `(env, proposer, wasm_hash, expiry) -> u64` | STABLE | |
| `approve_upgrade` | `(env, proposal_id, signer)` | STABLE | |

#### Admin

| Function | Signature | Stability | Notes |
|---|---|---|---|
| `get_admin` | `(env) -> Option<Address>` | STABLE | |
| `get_chain_id` | `(env) -> Option<String>` | EVOLVING | |
| `get_network_id` | `(env) -> Option<String>` | EVOLVING | |
| `get_network_info` | `(env) -> (Option<String>, Option<String>)` | EVOLVING | |

---

## 7. Contract: view-facade

**Crate:** `contracts/view-facade`  
**Contract struct:** `ViewFacade`  
**Purpose:** Read-only registry of Grainlify contract deployments; aggregates cross-contract queries for dashboards, indexers, and wallets.

### 7.1 Key Exported Types

| Type | Stability | Notes |
|---|---|---|
| `PayoutRecord` | STABLE | **⚠ DRIFT** — local copy omits `payout_type` field present in canonical `program-escrow`. See §3.1. |
| `RegisteredContract` | STABLE | Registry entry; `ContractKind` enum variant addition is additive. |
| `ContractKind` (enum) | STABLE | `BountyEscrow`, `ProgramEscrow`, `SorobanEscrow`, `GrainlifyCore`. Adding variant is additive only. |
| `FacadeError` | STABLE | 4 error codes; removing or renumbering is breaking. |

### 7.2 Public Entry-Points

| Function | Signature | Stability | Notes |
|---|---|---|---|
| `init` | `(env, admin: Address) -> Result<(), FacadeError>` | STABLE | One-time init; stores admin and empty registry. |
| `get_admin` | `(env) -> Option<Address>` | STABLE | |
| `register` | `(env, address, kind, version, ...) -> Result<(), FacadeError>` | STABLE | Upsert semantics (duplicate = update). |
| `deregister` | `(env, address) -> Result<(), FacadeError>` | STABLE | Admin-only removal. |
| `list_contracts` | `(env, offset: u32, limit: u32) -> Vec<RegisteredContract>` | STABLE | Paginated registry. |
| `list_contracts_all` | `(env) -> Vec<RegisteredContract>` | STABLE | Bounded by `MAX_REGISTRY_SIZE=1000`. |
| `contract_count` | `(env) -> u32` | STABLE | |
| `get_contract` | `(env, address) -> Option<RegisteredContract>` | STABLE | Single-contract lookup. |
| `query_recipient_history` | `(env, escrow_contract, program_id, recipient) -> Vec<PayoutRecord>` | STABLE | Cross-contract call; returns local `PayoutRecord` (drift from canonical). |

---

## 8. Contract: escrow-view-facade

**Crate:** `contracts/escrow-view-facade`  
**Contract struct:** `EscrowViewFacade`  
**Purpose:** Read-only aggregation of bounty-escrow data for frontend consumption; also proxies delegate queries to program-escrow.

### 8.1 Key Exported Types

| Type | Stability | Notes |
|---|---|---|
| `EscrowStatus` (enum) | STABLE | **Local copy** of `bounty-escrow::EscrowStatus`. Must stay in sync with canonical (see §3.2). |
| `EscrowSummary` | STABLE | Aggregated view of a single bounty; used by `get_escrow_summary`. |
| `UserPortfolio` | EVOLVING | `as_beneficiary` field is always empty today — placeholder for future use. |

### 8.2 Binding Files

| Binding File | Mirrors | Status |
|---|---|---|
| `src/bounty_escrow_bindings.rs` | `bounty-escrow`: `EscrowStatus`, `EscrowMetadata`, `PauseFlags`, `Escrow`, `EscrowWithId`, `AnonymousParty` | 🔴 Must update in same PR as canonical |
| `src/program_escrow_bindings.rs` | `program-escrow`: `ProgramDelegateInfo` | 🔴 Must update in same PR as canonical |

### 8.3 Public Entry-Points

| Function | Signature | Stability | Notes |
|---|---|---|---|
| `get_escrow_summary` | `(env, escrow_contract: Address, bounty_id: u64) -> Option<EscrowSummary>` | STABLE | Returns `None` instead of trapping. |
| `get_escrow_summaries` | `(env, escrow_contract, bounty_ids: Vec<u64>) -> Vec<EscrowSummary>` | STABLE | Batch; missing bounties omitted. |
| `get_user_portfolio` | `(env, escrow_contract, user: Address) -> UserPortfolio` | EVOLVING | `as_beneficiary` currently always empty. |
| `query_all_delegates` | `(env, program_contract: Address, program_id: String) -> Vec<ProgramDelegateInfo>` | STABLE | Returns empty vec on error rather than trapping. |

---

## 9. Cross-Contract Dependency Graph

```
                    ┌─────────────────────┐
                    │   grainlify-core    │
                    │  (upgrade mgmt,     │
                    │   registry)         │
                    └──────────┬──────────┘
                               │ (no direct calls; contracts
                               │  register themselves here)
          ┌────────────────────┼────────────────────┐
          │                    │                    │
          ▼                    ▼                    ▼
  ┌──────────────┐    ┌──────────────┐    ┌──────────────────┐
  │ program-     │    │ bounty-      │    │ (future)         │
  │ escrow       │    │ escrow       │    │                  │
  └──────┬───────┘    └──────┬───────┘    └──────────────────┘
         │                   │
         │  query_recipient_  │  get_escrow_info
         │  history           │  get_metadata
         ▼                   │  get_pause_flags
  ┌──────────────┐           │  query_escrows_by_depositor
  │ view-facade  │           │
  └──────────────┘           ▼
                    ┌──────────────────────┐
                    │ escrow-view-facade   │
                    │  also calls          │
                    │  program-escrow:     │
                    │  query_all_delegates │
                    └──────────────────────┘
```

**Binding dependency table:**

| Caller | Callee | Binding File | Functions Called |
|---|---|---|---|
| `view-facade` | `program-escrow` | inline `ProgramEscrowTrait` | `query_recipient_history` |
| `escrow-view-facade` | `bounty-escrow` | `bounty_escrow_bindings.rs` | `get_escrow_info`, `get_metadata`, `get_pause_flags`, `query_escrows_by_depositor` |
| `escrow-view-facade` | `program-escrow` | `program_escrow_bindings.rs` | `query_all_delegates` |

---

## 10. Upgrade Checklist

Use this checklist whenever modifying a type listed in §3 or a function used across contract
boundaries (§9).

### Before making a change

- [ ] Identify all entries in §3 that reference the type or function being changed.
- [ ] Identify all binding files from §9 that call the function or mirror the type.
- [ ] Determine if the change is **breaking** or **additive** per §2.

### For breaking changes

- [ ] Bump the storage schema version constant for affected contracts.
- [ ] Update the binding file in the **same PR** as the canonical contract change.
- [ ] Update the local copy in any facade that re-declares the type (e.g. `escrow-view-facade`'s `EscrowStatus`, `view-facade`'s `PayoutRecord`).
- [ ] Update this matrix document to reflect the new signature and sync status.
- [ ] Add or update a serialization golden test to catch future regressions.
- [ ] Run `cargo test -p program-escrow`, `cargo test -p grainlify-core`, and `cargo test -p bounty-escrow` — all must pass.

### For additive changes

- [ ] If a new field is appended to a `#[contracttype]` struct, verify that old storage entries are still readable (add a deserialization test with the old layout).
- [ ] If a new enum variant is added to a shared enum, update all exhaustive match arms in facade bindings.
- [ ] Update this matrix document.

### Known Outstanding Issues

| Issue | Location | Risk |
|---|---|---|
| `PayoutRecord` field drift — `payout_type` missing in view-facade | `view-facade/src/lib.rs` | 🔴 HIGH — returned data is incomplete |
| `ProgramData.circuit_breaker_threshold` has unresolved merge conflict (`Option<u8>` vs `Option<u32>`) | `program-escrow/src/lib.rs` line ~743 | 🔴 CRITICAL — must resolve before deployment |
| `UserPortfolio.as_beneficiary` is always empty | `escrow-view-facade/src/lib.rs` | 🟡 MEDIUM — documented placeholder |

---

*This document is automatically linked from each contract's crate-level doc comment.*  
*See: `program-escrow/src/lib.rs`, `bounty-escrow/.../lib.rs`, `grainlify-core/src/lib.rs`,*  
*`view-facade/src/lib.rs`, `escrow-view-facade/src/lib.rs`.*
