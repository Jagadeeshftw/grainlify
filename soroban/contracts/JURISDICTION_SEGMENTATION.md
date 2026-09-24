# Jurisdiction Segmentation (Program + Escrow)

This document describes how to use optional jurisdiction tags/configuration without splitting contract codebases.

## Goals

- Keep one deployable contract per domain (`soroban-program-escrow`, `escrow`)
- Support segment-specific policy (KYC, limits, pause behavior)
- Preserve backward compatibility for existing generic flows

## Program Contract

### Config type

`ProgramJurisdictionConfig`:

- `tag: Option<String>`: human-readable segment label (`"EU-only"`, `"US-only"`, etc.)
- `requires_kyc: bool`: require KYC attestation input on registration
- `max_funding: Option<i128>`: per-jurisdiction funding cap
- `registration_paused: bool`: block new registrations for that segment

### Entry points

- Existing: `register_program(...)` and `batch_register_programs(...)`
  - Behavior unchanged
  - Stored jurisdiction is `None` (generic)
- New:
  - `register_program_with_jurisdiction(...)`
  - `batch_register_programs_with_jurisdiction(...)`

### Enforcement rules

- If `registration_paused == true`, registration fails (`JurisdictionPaused`)
- If `max_funding` is set and exceeded, registration fails (`JurisdictionFundingLimitExceeded`)
- If `requires_kyc == true` and `kyc_attested != Some(true)`, registration fails (`JurisdictionKycRequired`)

### Events

Program registration emits `prg_reg` events with:

- jurisdiction tag
- KYC requirement flag
- max funding
- paused flag

## Escrow Contract

### Config type

`EscrowJurisdictionConfig`:

- `tag: Option<String>`
- `requires_kyc: bool`
- `enforce_identity_limits: bool`
- `lock_paused: bool`
- `release_paused: bool`
- `refund_paused: bool`
- `max_lock_amount: Option<i128>`

### Entry points

- Existing: `lock_funds(...)`
  - Behavior unchanged
  - Stores `jurisdiction: None`
- New: `lock_funds_with_jurisdiction(...)`
  - Stores policy on the escrow record for lifecycle enforcement

Read path:

- `get_escrow_jurisdiction(bounty_id)`

### Enforcement rules

- `lock_funds_with_jurisdiction`:
  - `lock_paused` blocks lock
  - optional `max_lock_amount` cap
  - optional KYC requirement (`is_claim_valid`)
  - optional identity-limit enforcement (`enforce_identity_limits`)
- `release_funds`:
  - uses escrow-stored jurisdiction config
  - `release_paused` and `requires_kyc` enforced
  - limit checks run only when `enforce_identity_limits == true`
- `refund`:
  - uses escrow-stored jurisdiction config
  - `refund_paused` and `requires_kyc` enforced

### Events

Escrow lifecycle emits `juris` events (`lock`, `release`, `refund`) containing:

- jurisdiction tag
- KYC/limit flags
- operation pause flags
- max lock amount

## Generic vs Tagged Integration Pattern

1. Use existing entrypoints for generic markets (`jurisdiction = None`).
2. Use new jurisdiction-aware entrypoints for regulated segments.
3. Keep off-chain policy mapping from region/compliance domain to the corresponding config.
4. Index `prg_reg` and `juris` events for audit/compliance reporting.

## Test Coverage Added

- Program:
  - tagged registration success
  - KYC-required registration rejection
  - max-funding and paused-segment rejection
  - batch registration with mixed generic/tagged programs
- Escrow:
  - generic escrow (`None`) behavior
  - tagged escrow with limit override behavior
  - tagged pause enforcement
  - jurisdiction event emission checks
