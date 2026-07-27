# Bounty Escrow — External Audit Checklist

This checklist is intended for external auditors reviewing the
`contracts/bounty_escrow/contracts/escrow` smart contract. Each item describes
a known behaviour, platform constraint, or potential confusion point that
auditors should explicitly verify or rule out.

---

## 1. Gas Budget — Advisory-Only Caps Outside `testutils`

**Priority: HIGH — must not be misread as an enforcement guarantee**

### Finding

`GasBudgetConfig` caps (CPU instructions and memory bytes per operation) are
**stored in instance storage** and **enforced only under the `testutils` build
feature**. On the live Stellar network (production WASM) the `env.budget()` API
is unavailable and no runtime measurement occurs.

### Impact

- `GasBudgetConfig::enforce = true` has **no runtime effect in production**.
- `Error::GasBudgetExceeded` is never returned from production deployments.
- A deployment with non-zero caps and `enforce = true` is **functionally
  uncapped** from a resource-consumption perspective on mainnet.

### How to verify

```
// On-chain query (Soroban RPC / Stellar CLI):
BountyEscrowContract::get_gas_budget_advisory_status()
```

The returned `GasBudgetAdvisoryStatus.caps_enforced_in_production` field is a
**compile-time constant** (`false` in production WASM, `true` under testutils).
It cannot be spoofed by contract logic.

When `caps_configured = true` and `caps_enforced_in_production = false`, the
deployment is in advisory-only mode: caps are documented but not enforced.

The `"gas_adv"` event in the on-chain event stream (emitted by
`get_gas_budget_advisory_status` whenever caps are configured) is a secondary
indicator for indexer/monitoring verification.

### Indirect mitigations in production

| Control | Effect |
|---------|--------|
| `MAX_BATCH_SIZE` (default: 20, configurable via `set_batch_size_caps`) | Hard-caps items per batch call, bounding worst-case CPU by construction |
| Soroban network-level limits (~100B CPU, ~40 MB memory per tx) | Absolute hard caps enforced by validators, independent of contract logic |
| Per-bounty amount validation | Bounds token-math complexity in the common failure path |

### References

- `src/gas_budget.rs` — full module doc with enforcement matrix
- `docs/security/gas-budget-production-gap.md` — operator and auditor guide
- `GAS_TESTS.md` — profiling workflow

---

## 2. Fee-on-Transfer Token Risk

**Priority: HIGH**

### Finding

Tokens that silently deduct a fee on transfer can cause the INV-2 invariant
(aggregate escrow balance == token.balance(contract)) to mismatch, breaking the
accounting of all locked bounties.

### Mitigation

INV-2 is checked at the end of every `lock_funds` and `publish` call.
A violated invariant causes a panic, which atomically rolls back all state
mutations in the transaction — the depositor's tokens are not lost.

### How to verify

- Review `src/multitoken_invariants.rs` `assert_after_lock`.
- Run `test_fee_on_transfer.rs` — specifically `test_full_fee_drain_detected_by_inv2_on_lock`.

### References

- `docs/security/fee-on-transfer-tokens.md` — full analysis
- `src/test_fee_on_transfer.rs` — test suite

---

## 3. Reentrancy Guard

**Priority: HIGH**

### Finding

All state-mutating entry points (lock, release, refund, partial_release, batch
variants) are protected by a reentrancy guard stored at `DataKey::ReentrancyGuard`.

### How to verify

- Review `src/reentrancy_guard.rs`.
- Confirm `reentrancy_guard::acquire` is called at the start and
  `reentrancy_guard::release` at the end of each protected function.
- Run `test_reentrancy_guard.rs`.

---

## 4. Admin Two-Step Rotation Timelock

**Priority: MEDIUM**

### Finding

Admin key rotation uses a two-step propose/accept pattern with a configurable
timelock delay (default enforced; `timelock_delay > 0` required).

### How to verify

- Review `src/lib.rs` `propose_admin` / `accept_admin`.
- Confirm `DataKey::PendingAdminTransition` is cleared on both accept and cancel.
- Run `test_admin_rotation.rs`.

---

## 5. Batch Operation Atomicity

**Priority: MEDIUM**

### Finding

`batch_lock_funds` and `batch_release_funds` are **strictly atomic**:
all items are validated before any state mutation. A single item failure
reverts the entire batch.

### Caveat

Batch size is bounded by `MAX_BATCH_SIZE` (default 20, configurable via
`set_batch_size_caps`). Auditors should confirm this limit cannot be set to 0
or above the hard-coded maximum.

### How to verify

- Review `src/lib.rs` batch logic (`validate_batch_lock_items`,
  `validate_batch_release_items`).
- Review `set_batch_size_caps` for `InvalidBatchSizeCap` guard.
- Run `test_batch_failure_mode.rs`.

---

## 6. `INV-2` Bypass Flag (`InvOff`)

**Priority: HIGH — must be absent in production**

### Finding

`DataKey::InvOff` is a storage key that disables the INV-2 invariant check
when set to `true`. It exists solely to support adversarial-state unit tests.

### How to verify

Confirm `DataKey::InvOff` is **not set** (`false` or absent) in any production
deployment. The recommended verification:

```
// Off-chain (Stellar CLI):
contract storage read --key InvOff
# Expected: empty / not present
```

---

## 7. Maintenance Mode and Pause Flags

**Priority: LOW (operational risk only)**

### Finding

The admin can pause individual operation categories (lock, release, refund)
or enable maintenance mode (blocks all state-mutating calls). These flags
must not be left enabled in a production deployment unintentionally.

### How to verify

- Call `get_maintenance_status()` and `get_pause_status()` on the deployed
  contract.
- Confirm both return the expected state for the intended deployment mode.

---

## Checklist Summary

| # | Item | Status |
|---|------|--------|
| 1 | Gas budget caps advisory-only in production | ⚠ Advisory |
| 2 | Fee-on-transfer token risk | ✅ Mitigated via INV-2 |
| 3 | Reentrancy guard on all mutating entry points | ✅ Implemented |
| 4 | Admin rotation timelock | ✅ Implemented |
| 5 | Batch operation atomicity and size cap | ✅ Implemented |
| 6 | INV-2 bypass flag absent in production | 🔍 Verify deployment |
| 7 | Maintenance mode / pause flags | 🔍 Verify deployment |

Legend: ✅ Implemented and tested  ⚠ Known gap, documented  🔍 Requires deployment verification
