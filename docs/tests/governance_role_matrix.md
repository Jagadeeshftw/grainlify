# Governance Module — Role-Based Access Control (RBAC) Matrix

## Role Definitions

| Role          | Symbol    | Purpose                                                                 |
|---------------|-----------|-------------------------------------------------------------------------|
| **Admin**     | `ADMIN`   | General administration: role assignment, admin rotation, initialization. Super-user privilege that can grant/revoke all other roles. |
| **Emergency** | `EMERG`   | Emergency pause/unpause controls. Halts contract operations during incidents without touching configuration or upgrades. |
| **Upgrade**   | `UPGR`    | Authorizes and executes WASM upgrades. Runs the `execute_proposal` path after governance approvals. |
| **Config**    | `CFG`     | Updates protocol parameters (timelock delays, voting periods, quorum thresholds) without touching admin keys or upgrade logic. |

> **Security Principle:** Separation of duties — no single role combines upgrade + config + emergency powers.
> Admin can delegate, but `Admin` rotation itself is a two-step action (current Admin proposes + new Admin confirms).

---

## Mutating Entrypoints → Required Role

The table below lists every state-mutating entrypoint exposed by `GovernanceContract` in `contracts/grainlify-core/src/governance.rs` and the RBAC role that **must** be verified (via `require_auth` + role check) **before** any storage write occurs.

| # | Entrypoint / Method                     | Required Role   | Auth Target         | State Mutations Performed                                                                 | Event Symbol (Auditable)                  |
|---|------------------------------------------|-----------------|---------------------|-------------------------------------------------------------------------------------------|-------------------------------------------|
| 1 | `init_governance_state`                 | Admin (seed)    | `admin` argument    | Writes `GOVERNANCE_CONFIG`, `PROPOSAL_COUNT`, stores initial Admin/Emergency/Upgrade/Config role holders, pause flag. | `gov_init`                                |
| 2 | `rotate_admin`                           | Admin           | `current_admin`     | Writes `PENDING_ADMIN` rotation record (two-step transfer).                               | `adm_rot_p` (AdminRotationProposed)       |
| 3 | `confirm_admin_rotation`                 | New Admin       | `new_admin`         | Consumes pending rotation; atomically replaces Admin role holder.                         | `adm_rot_c` (AdminRotationConfirmed)      |
| 4 | `set_emergency_role`                    | Admin           | `admin` argument    | Grants or revokes the Emergency role to/from an address.                                  | `emg_role` (EmergencyRoleSet)             |
| 5 | `set_upgrade_role`                      | Admin           | `admin` argument    | Grants or revokes the Upgrade role to/from an address.                                    | `upg_role` (UpgradeRoleSet)               |
| 6 | `set_config_role`                       | Admin           | `admin` argument    | Grants or revokes the Config role to/from an address.                                     | `cfg_role` (ConfigRoleSet)                |
| 7 | `set_security_council`                  | Admin           | `admin` argument    | Writes `SECURITY_COUNCIL` storage key (veto authority).                                   | `sec_cncl` (SecurityCouncilSet)           |
| 8 | `emergency_pause`                        | Emergency       | `caller` argument   | Sets `EMERGENCY_PAUSED` flag → all mutating ops blocked (proposal create/vote/finalize/execute). | `emg_pause` (EmergencyPaused)         |
| 9 | `emergency_unpause`                      | Emergency       | `caller` argument   | Clears `EMERGENCY_PAUSED` flag → normal operation restored.                               | `emg_unp` (EmergencyUnpaused)             |
|10 | `update_governance_config`               | Config          | `caller` argument   | Updates mutable fields in `GOVERNANCE_CONFIG` (voting period, quorum, approval threshold, min stake, execution delay). | `gov_cfgup` (GovernanceConfigUpdated)     |
|11 | `create_proposal`                        | Proposer (stake)| `proposer` argument | Stake transfer + writes `PROPOSALS` + `PROPOSAL_COUNT`. Stake-gated not RBAC-gated.       | `gov_prop` (ProposalCreated)              |
|12 | `cast_vote`                              | Voter           | `voter` argument    | Writes `VOTES`, updates `PROPOSALS` vote tallies. One-person or token-weighted.           | `gov_vote` (VoteCast)                     |
|13 | `finalize_proposal`                      | Public          | — (no auth)         | Updates proposal status to Approved/Rejected, refunds stake. Permissionless caller.       | `gov_final` (ProposalFinalized)           |
|14 | `execute_proposal`                       | Upgrade         | `executor` argument | Calls `update_current_contract_wasm`, sets proposal to Executed.                          | `gov_exec` (ProposalExecuted)             |
|15 | `veto_proposal`                          | Security Council| `security_council`  | Sets proposal status to Vetoed during execution timelock window.                          | `gov_veto` (ProposalVetoed)               |

---

## Read-Only Entrypoints (No Auth / No Role Required)

These methods only read storage and never mutate state — they are intentionally unguarded.

| Method                  | Returns                      |
|-------------------------|------------------------------|
| `get_config`            | `GovernanceConfig`           |
| `get_security_council`  | `Address`                    |
| `get_role_holder`       | `Option<Address>` per role   |
| `is_emergency_paused`   | `bool` (emergency pause)     |
| `get_pending_admin_rotation` | `Option<PendingAdminRotation>` |

---

## Authorization Ordering Guarantee

For every mutating entrypoint the following **strict ordering** is enforced inside `governance.rs`:

```
1. require_auth() on the principal address
2. Validate role ownership / capability (returns Err on mismatch)
3. Validate inputs (zero-address checks, range checks, not-self address)
4. Check emergency pause status (if entrypoint is pause-gated)
5. ←— NO STORAGE WRITES BEFORE THIS LINE —←
6. Perform state mutations
7. Emit auditable event
```

Any step failure before #6 leaves storage untouched — this is the TOCTOU-safe invariant.

---

## Validity / Expiry Semantics

| Capability              | Expiration Model                                | Test Case Reference                    |
|-------------------------|-------------------------------------------------|----------------------------------------|
| Admin / Emergency / Upgrade / Config roles | No time expiry; explicitly transferred/revoked by Admin | `test_role_revocation_*`          |
| Pending admin rotation  | `DEFAULT_ADMIN_ROTATION_TTL` (24h) ledger-timestamp based | `test_expired_admin_rotation`    |
| Proposal vote window    | `voting_end` ledger timestamp                   | `voting period tests`                  |
| Timelock execute window | `voting_end + execution_delay`                  | `timelock / veto tests`                |

---

## Invalid / Zero-Address Guard Matrix

Every role-assignment entrypoint (`rotate_admin`, `set_*_role`, `set_security_council`, `init_governance_state`) validates the target address is **not** the zero/uninitialized address and **not** equal to the current contract's own address **before** any storage write.

| Entrypoint               | Zero-Address Check Before State Write? | Panic Error Code          |
|--------------------------|----------------------------------------|---------------------------|
| `init_governance_state`  | Yes — `admin` (all 4 initial roles seeded from admin) | `InvalidRoleHolder`       |
| `rotate_admin`           | Yes — `new_admin`                      | `InvalidRoleHolder`       |
| `set_emergency_role`     | Yes — `new_holder`                     | `InvalidRoleHolder`       |
| `set_upgrade_role`       | Yes — `new_holder`                     | `InvalidRoleHolder`       |
| `set_config_role`        | Yes — `new_holder`                     | `InvalidRoleHolder`       |
| `set_security_council`   | Yes — `security_council`               | `InvalidRoleHolder`       |
