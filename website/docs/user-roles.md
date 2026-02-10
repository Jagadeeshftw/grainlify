---
description: User roles in Grainlify – Ecosystem, Maintainer, Contributor, and Admin.
---

# User Roles

Grainlify is built around four primary roles:

- **Ecosystem**
- **Project Maintainer**
- **Contributor**
- **Admin**

Each role sees a different part of the product and has different permissions.

---

## Ecosystem

**Who:** Foundations, DAOs, protocol teams, or organizations that allocate grant capital.

**Goal:** Turn ecosystem funds into verifiable, automated payouts for real open‑source work.

### Capabilities

- Create and manage **programs**:
  - Hackathons.
  - Grant rounds.
  - Continuous funding streams.
- Lock **program budgets** into on‑chain escrows.
- Define:
  - Program scope and dates.
  - Eligible ecosystems or project types.
  - High‑level rules (e.g., scoring guidelines, prize structures).
- View program‑level and ecosystem‑level analytics:
  - Total funds locked.
  - Funds paid out.
  - Number of projects and contributors.
  - Contribution activity over time.

### Typical Journey

1. Decide to run an OSS funding initiative.
2. Create a program and **lock funds in escrow**.
3. Onboard projects and maintainers.
4. Let maintainers and contributors work via GitHub.
5. At program milestones or at the end, trigger or approve payouts.
6. Export reports or view dashboards for impact analysis.

---

## Project Maintainer

**Who:** Owners or core maintainers of GitHub repositories participating in programs or bounties.

**Goal:** Attract and reward contributors without manually managing payout logistics.

### Capabilities

- Register projects by connecting GitHub repositories.
- Install and manage the **GitHub App** integration.
- Create and manage:
  - **Issues** with point values or bounty metadata.
  - **Bounties** linked to specific issues.
  - Allocated budgets for their projects (from program escrows).
- Lock funds for specific bounties into escrow.
- Review applications (if using an “apply to work” model).
- Monitor contributor activity and project analytics.

### Typical Journey

1. Log in with **GitHub OAuth**.
2. Register a project and connect the GitHub repo.
3. Install the Grainlify GitHub App for webhooks and permissions.
4. Create or tag issues as funded bounties, assign point values.
5. Optionally lock funds into escrow per issue or per project pool.
6. Review PRs, merge valid contributions.
7. Once merged and validated, let the system trigger payouts from escrow.

---

## Contributor

**Who:** Developers and community members who work on GitHub issues and pull requests.

**Goal:** Earn predictable, automatic rewards for meaningful contributions to open‑source projects.

### Capabilities

- Authenticate with **GitHub** to create a Grainlify account.
- Browse:
  - Programs and hackathons.
  - Funded projects and issues.
  - Their own contribution history.
- Link one or more wallets (e.g., Stellar addresses) for payouts.
- Complete **KYC** (via the integrated KYC provider) once to unlock payouts.
- Work on GitHub as usual:
  - Pick funded issues.
  - Open pull requests.
  - Get merged and verified.
- Receive **automatic payouts** from escrow contracts to their wallet.

### Typical Journey

1. Log in with GitHub.
2. Complete KYC (if required for payouts).
3. Link a payout wallet.
4. Browse funded issues or program opportunities.
5. Open PRs referencing those issues.
6. Once merged and verified, receive payouts directly to their wallet.

---

## Admin

**Who:** Operators of the Grainlify platform (internal team or delegated administrators).

**Goal:** Safely operate and monitor the platform, enforce policies, and support users.

### Capabilities

- View platform‑wide dashboards:
  - Ecosystems, programs, and projects.
  - User and wallet metrics.
  - KYC status distributions.
- Approve or reject:
  - Project registrations (if moderation is enabled).
  - Special ecosystem integrations.
- Manage system configuration:
  - Ecosystem entries (e.g., Stellar, other chains).
  - Feature flags and environment‑specific settings (where supported by code).
- Monitor:
  - Webhook health.
  - Background workers.
  - Event processing and error logs (via external observability tools in production).

### Typical Journey

1. Log in with admin credentials.
2. Check health dashboards and queues.
3. Review new project or ecosystem requests.
4. Assist in troubleshooting integrations (GitHub, KYC, wallets).
5. Ensure safe rollout of new features and migrations.

---

## Role Summary

- **Ecosystem** – funds programs and wants verifiable, automated grant execution.
- **Maintainer** – registers projects, creates bounties, and manages contributions.
- **Contributor** – does the work on GitHub and gets paid automatically.
- **Admin** – operates the platform and ensures system health and compliance.

Use the persona‑specific guides:

- `for-ecosystems.md`
- `for-maintainers.md`
- `for-contributors.md`

for detailed, step‑by‑step flows for each role.

