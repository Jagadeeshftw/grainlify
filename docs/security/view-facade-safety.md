# View Facade Cross-Contract Call Safety

## Overview

This document is the security audit for cross-contract call safety in
`contracts/escrow-view-facade/` and `contracts/view-facade/` (issue #1288).

Both facades act as **read-only aggregation layers**. They must never:
- Call auth-gated (state-mutating) functions on underlying contracts
- Forward or escalate caller auth to underlying contracts
- Modify state in any contract other than their own registry

---

## Audit: `escrow-view-facade`

### Cross-contract calls made

| Function called on underlying | Type | Auth required? | State mutated? |
|-------------------------------|------|----------------|----------------|
| `try_get_escrow_info(id)` | view | ❌ No | ❌ No |
| `try_get_metadata(id)` | view | ❌ No | ❌ No |
| `try_get_pause_flags()` | view | ❌ No | ❌ No |
| `try_query_escrows_by_depositor(user, offset, limit)` | view | ❌ No | ❌ No |

**All calls use the `try_` prefix** — they return `Result` instead of panicking,
so a missing or erroring underlying contract causes graceful degradation
(`None` / empty vec) rather than a trap.

### Auth forwarding analysis

The facade does **not** call `require_auth()` on behalf of the caller at any
point. The facade's own entrypoints (`get_escrow_summary`, `get_escrow_summaries`,
`get_user_portfolio`) take no `caller: Address` parameter and perform no auth
checks — any address can call them.

### State mutation analysis

The facade writes **no state** to the underlying escrow contract. It only reads
from instance storage of the escrow contract via view functions.

The facade itself has **no instance storage** — it is a pure pass-through.

### Security verdict: ✅ SAFE

---

## Audit: `view-facade`

### Cross-contract calls made

`ViewFacade` makes **no cross-contract calls**. It only reads and writes its
own instance storage (`DataKey::Admin`, `DataKey::Registry`).

### Auth model

| Entrypoint | Auth required | Notes |
|------------|---------------|-------|
| `init(admin)` | None (first-caller) | Admin stored immutably; double-init rejected |
| `register(addr, kind, ver)` | Admin | `admin.require_auth()` enforced |
| `deregister(addr)` | Admin | `admin.require_auth()` enforced |
| `list_contracts(offset, limit)` | None | Pure read |
| `list_contracts_all()` | None | Pure read |
| `contract_count()` | None | Pure read |
| `get_contract(addr)` | None | Pure read |
| `get_admin()` | None | Pure read |

### State mutation analysis

- `register` and `deregister` write only to `DataKey::Registry` in the
  facade's own instance storage. They do not touch any external contract.
- All view functions are pure reads with no side effects.

### Security verdict: ✅ SAFE

---

## Security Assumptions

1. **Admin key security** — the admin address is immutable after `init`. A
   compromised admin key can modify the registry but cannot affect underlying
   escrow contracts.

2. **No fund custody** — neither facade holds tokens or transfers funds.

3. **Bounded registry** — `ViewFacade` enforces `MAX_REGISTRY_SIZE = 1000`
   to prevent storage exhaustion attacks.

4. **try_ pattern** — `EscrowViewFacade` uses `try_` calls so a malicious or
   broken underlying contract cannot cause the facade to trap.

5. **No auth escalation** — calling a view function on either facade does not
   grant the caller any elevated permissions on the underlying contracts.

---

## Test Coverage (issue #1288)

### `escrow-view-facade/src/test_cross_contract_safety.rs`

| Test | Property verified |
|------|-------------------|
| `test_get_escrow_summary_is_read_only` | No mutating functions called |
| `test_get_escrow_summaries_batch_is_read_only` | Batch also read-only |
| `test_get_user_portfolio_is_read_only` | Portfolio also read-only |
| `test_unprivileged_caller_can_query_facade` | No auth required to query |
| `test_two_different_callers_get_identical_results` | No per-caller state |
| `test_facade_does_not_require_caller_auth` | No auth check on caller |
| `test_missing_escrow_returns_none` | Graceful degradation |
| `test_batch_with_missing_escrows_returns_empty_vec` | Graceful degradation |
| `test_user_portfolio_with_missing_contract_returns_empty` | Graceful degradation |
| `test_escrow_summary_fields_match_underlying_data` | Correct data mapping |
| `test_paused_contract_reflected_in_summary` | Pause state read correctly |
| `test_batch_and_single_return_consistent_data` | Batch/single consistency |
| `test_empty_batch_returns_empty_vec` | Edge case: empty input |

### `view-facade/src/test_cross_contract_safety.rs`

| Test | Property verified |
|------|-------------------|
| `test_list_contracts_requires_no_auth` | View is public |
| `test_get_contract_requires_no_auth` | View is public |
| `test_contract_count_requires_no_auth` | View is public |
| `test_list_contracts_all_requires_no_auth` | View is public |
| `test_get_admin_requires_no_auth` | View is public |
| `test_register_requires_admin_auth` | Mutation is admin-gated |
| `test_deregister_requires_admin_auth` | Mutation is admin-gated |
| `test_view_call_does_not_grant_register_access` | No auth escalation |
| `test_double_init_is_rejected` | Admin immutability |
| `test_admin_cannot_be_replaced_after_init` | Admin immutability |
| `test_registry_state_consistent_across_reads` | Registry isolation |
| `test_unprivileged_caller_sees_same_registry_as_admin` | No per-caller state |
| `test_paginated_list_requires_no_auth` | Pagination is public |
| `test_invalid_pagination_returns_error_not_panic` | Graceful error handling |
| `test_registry_full_error_is_returned_not_panic` | Bounded storage |
| `test_deregister_nonexistent_is_noop` | Idempotent deregister |
| `test_get_contract_returns_none_for_unknown_address` | Safe miss handling |
| `test_get_contract_returns_correct_entry_after_register` | Correct data |
