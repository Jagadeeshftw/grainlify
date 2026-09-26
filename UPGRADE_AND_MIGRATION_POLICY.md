# Grainlify Upgrade and Migration Policy

This document defines the rules and requirements for authorizing, executing, and reverting upgrades and migrations in the Grainlify smart contracts. It serves as the source of truth for safe contract evolution and aligns with the existing test suites.

## 1. Authorization Requirements
- **Admin Access:** Upgrades and migrations can only be proposed and initiated by the designated contract `Admin`. Any attempt by unauthorized entities will be rejected.
- **Sufficient Approvals (Multisig):** Executing a proposed upgrade requires sufficient cryptographic approvals (multisig) from the designated signers or governance board. 
- **Registered Proposal:** An upgrade must correspond to a valid, pre-registered commitment or proposal. Upgrades lacking an active proposal, or those referencing non-existent/expired proposals, will be rejected.
- **Contract State:** The contract must be fully initialized before any upgrades can be performed. Upgrades cannot be executed when the system is paused.
- **Single-Use Proposals:** Proposals are single-use. Once an upgrade is executed, the commitment is consumed, and replay protection prevents it from being executed again.

## 2. Reversibility and Rollback Conditions
Migrations and upgrades are reversible under the following conditions:
- **Automatic Storage Rollback:** If an upgrade transaction fails, panics, or violates an invariant (e.g., incorrect hash, uncommitted migration, expired commitment), the Soroban VM automatically rolls back all storage changes. The contract version and state remain entirely unchanged, preventing partial migrations.
- **Manual Rollback (Downgrade):** After a successful upgrade, if a critical defect is identified, the system supports a manual rollback path. The `Admin` can restore the previous state by explicitly calling `set_version` and reverting to the tracked `PreviousVersion`.

## 3. Policy to Test Mapping
This policy aligns directly with the `grainlify-core` test suites (`e2e-upgrade-tests.yml`, `upgrade_rollback_tests.rs`, and `test_migration_replay.rs`):

| Policy Clause | Mapped Test Cases |
| --- | --- |
| Admin Access required | `test_upgrade_rejects_non_admin`, `init_admin_sets_version_and_admin` |
| Contract Initialization required | `test_upgrade_rejects_uninitialized_contract`, `init_admin_double_init_panics` |
| Multisig/Approvals required | `test_execute_upgrade_with_sufficient_approvals`, `test_execute_upgrade_insufficient_approvals` |
| Proposal Registration required | `test_execute_upgrade_nonexistent_proposal`, `commit_migration_stores_commitment` |
| Upgrade blocked if paused | `test_execute_upgrade_when_paused` |
| Upgrade blocked if state inconsistent | `test_execute_upgrade_when_state_inconsistent` |
| Version Tracking / Monotonicity | `test_execute_upgrade_version_tracking` |
| Proposals are single-use | `test_execute_upgrade_already_executed`, `migrate_consumes_commitment_replay_protection` |
| Automatic Rollback (Failed execution) | `migrate_with_wrong_hash_panics`, `migrate_without_prior_commit_panics` |
| Manual Rollback (set_version) | `test_rollback_version_tracking_via_set_version` |

## 4. Contracts Lacking a Tested Rollback Path
While `grainlify-core` implements comprehensive upgrade and rollback testing, the following contracts do not currently have a defined or tested rollback path. They must be brought into alignment with this policy prior to executing any production upgrades on them:
- `bounty_escrow`
- `escrow-view-facade`
- `program-escrow`
- `view-facade`
