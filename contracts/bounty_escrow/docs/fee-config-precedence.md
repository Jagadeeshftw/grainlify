# Fee Config Precedence: Global `fee_enabled` vs Per-Token `TokenFeeConfig`

**Contract:** `contracts/bounty_escrow/contracts/escrow/src/lib.rs`
**Resolved in:** `BountyEscrowContract::resolve_fee_config`
**Tests:** `contracts/bounty_escrow/contracts/escrow/src/test_multi_token_fees.rs`

## The question

The escrow contract has two layers of fee configuration:

1. A **global `FeeConfig`** (`update_fee_config` / `get_fee_config`) with a
   `fee_enabled` flag, and
2. an optional **per-token `TokenFeeConfig`** override
   (`set_token_fee_config` / `get_token_fee_config`) which also carries its
   own `fee_enabled` flag.

What happens when the global flag is `false` but a token has an active
override with `fee_enabled = true`? Before this change the per-token config
replaced the global config wholesale, so the override silently **re-enabled**
fees for that token even while the global flag was off — meaning there was no
way to halt all fee collection without clearing every per-token config.

## Locked-in behavior

The global `fee_enabled` flag is a **master kill-switch**. The effective flag
for a token is the logical AND of both layers:

```
effective_enabled = global.fee_enabled AND token.fee_enabled   (override present)
effective_enabled = global.fee_enabled                          (no override)
```

Resolution order in `resolve_fee_config`:

1. **Global `FeeConfig.fee_enabled`** — when `false`, no fee is charged for
   *any* token, regardless of per-token overrides.
2. **`TokenFeeConfig(token)`** — when present, its `lock_fee_rate`,
   `release_fee_rate`, fixed fees, and `fee_recipient` override the global
   values for that token. Its `fee_enabled` flag can only *further restrict*
   (disable fees for that one token); it can never re-enable fees while the
   global switch is off.
3. **Global `FeeConfig`** — the fallback when no per-token override exists.

### Truth table (per-token override present)

| `global.fee_enabled` | `token.fee_enabled` | Fee charged? | Rates used |
|----------------------|---------------------|--------------|------------|
| `true`               | `true`              | Yes          | per-token  |
| `true`               | `false`             | No           | —          |
| `false`              | `true`              | **No**       | —          |
| `false`              | `false`             | No           | —          |

Without an override, fees follow the global config directly (`true` → global
rates, `false` → none).

Note that the storage **default** for the global config is
`fee_enabled = false`, so per-token configs charge nothing until an admin
explicitly enables fees globally via `update_fee_config`.

## Why AND (security rationale)

- **Single-operation emergency stop.** An incident responder can halt all fee
  collection with one admin call (`update_fee_config(..., fee_enabled =
  Some(false))`) instead of enumerating and clearing every token override —
  fewer transactions, no risk of missing one.
- **No privilege escalation through overrides.** A per-token config is a
  *narrowing* mechanism. If it could out-vote the global switch, setting an
  override would silently widen fee collection contrary to the operator's
  most recent global decision.
- **Toggle-safe.** Disabling and re-enabling the global switch preserves all
  per-token configs untouched; they resume charging as soon as the switch is
  back on (covered by `test_kill_switch_toggle_rearms_token_config`).

## Regression tests

`test_multi_token_fees.rs` locks this behavior in:

- `test_fee_enabled_precedence_truth_table` — asserts the full four-row truth
  table above; a future change that reverses precedence flips the
  `(false, true)` row and fails this test.
- `test_global_kill_switch_suppresses_enabled_token_config_on_lock` /
  `..._on_release` / `..._token_fixed_fee` — kill-switch off with an active,
  enabled per-token override (percentage and fixed fees, both fee paths):
  zero fee, full principal preserved.
- `test_global_enabled_no_token_override_charges_global_fee` — the inverse
  case from the issue: global `fee_enabled = true` with no override charges
  the global rate.
- `test_lock_no_fee_when_token_disabled` — the per-token flag still restricts
  when the global switch is on.
