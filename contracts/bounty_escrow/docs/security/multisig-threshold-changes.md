# Multisig Threshold Changes — Security Analysis

## Overview

The Bounty Escrow contract supports a **multisig approval flow** for high-value
releases. When a bounty amount exceeds the high-value threshold, `release_funds`
places the release in a timelock queue (`QueuedRelease`). Before the queued
release can be executed via `execute_queued_release`, a set of authorized
multisig signers must approve the release by calling `approve_large_release`.

## Approval Flow

```
Admin calls release_funds(bounty_id, contributor)
  └─ amount >= high_value_config.threshold?  →  QueuedRelease stored
  └─ amount < threshold?                      →  Released immediately

Multisig signers call approve_large_release(bounty_id, contributor, approver)
  └─ approver must be in MultisigConfig.signers
  └─ approval stored in ReleaseApproval(bounty_id)

Anyone calls execute_queued_release(bounty_id)
  └─ timelock elapsed?
  └─ multisig threshold met? (approvals >= required_signatures)
  └─ escrow state checks → token transfer
```

## Semantics: Live-Threshold

The contract enforces **live-threshold semantics**: the `required_signatures`
value at execution time (when `execute_queued_release` is called) governs
whether the release proceeds, not the value at the time approvals were
collected.

### Why live-threshold?

Live-threshold was chosen over snapshot-at-queue-time for the following reasons:

1. **Admin flexibility** — The admin can respond to changing security
   requirements by adjusting the threshold without needing to cancel and
   re-queue a release.

2. **Simplicity of state** — No need to snapshot the threshold into each
   `QueuedRelease` entry or `ReleaseApproval` record, keeping storage and
   upgrade paths simpler.

3. **Consistent failure mode** — A threshold raise always blocks pending
   releases, which is the conservative default. The admin can always lower
   again or collect more approvals.

### Implications

| Action | Effect on queued release |
|--------|--------------------------|
| Lower `required_signatures` | May make a blocked release executable |
| Raise `required_signatures` | May block a previously-eligible release |
| Change signer set | Existing approvals remain valid (matched by address) |

## Security Considerations

### Raising the threshold after approvals are collected

If the admin raises `required_signatures` after signers have already approved a
release, those approvals are **insufficient** and `execute_queued_release`
returns `Error::Unauthorized`. This is by design: it prevents a compromised
subset of signers from authorizing a release before the admin can react.

### Lowering the threshold after approvals are collected

If the admin lowers `required_signatures`, a previously-blocked release may
become executable. This is safe because the admin is already trusted to
configure the multisig parameters and could simply re-queue the release with
the same outcome.

### Threshold of zero

When `required_signatures == 0` (the default), the multisig check is skipped
entirely. This ensures backward compatibility with contracts that have not
configured multisig.

### Replay of approvals across signer set changes

Changing the signer set (`MultisigConfig.signers`) does not invalidate existing
approvals. An approver address that is removed from the signer set but has
already approved will still count toward the threshold. Administrators should
clear pending approvals when rotating signers.

## Test Coverage

All tests are in `contracts/escrow/src/test_state_verification.rs`.

| Test | Verifies |
|------|----------|
| `test_multisig_lower_threshold_makes_release_executable` | Lowering `required_signatures` from 3 to 2 makes a queued release executable |
| `test_multisig_raise_threshold_blocks_release` | Raising `required_signatures` from 3 to 4 blocks a previously-queued release |
| `test_multisig_threshold_not_met_still_blocks` | Fewer approvals than required still blocks execution |
| `test_multisig_threshold_met_allows_release` | Sufficient approvals allows execution |
| `test_multisig_config_persistence` | `update_multisig_config` and `get_multisig_config` round-trip correctly |

## References

- `contracts/escrow/src/lib.rs` — `update_multisig_config`, `approve_large_release`,
  `execute_queued_release`, `get_multisig_config`
- `contracts/escrow/src/lib.rs` — `MultisigConfig`, `ReleaseApproval` structs
- `contracts/escrow/src/test_state_verification.rs` — test suite
