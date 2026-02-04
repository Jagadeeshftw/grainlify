---
description: High-level explanation of how Grainlify works end-to-end.
---

# How Grainlify Works

Grainlify connects **ecosystems**, **projects**, and **contributors** through a simple flow:

1. Ecosystems lock grant funds into **on‑chain escrows**.
2. Maintainers turn GitHub issues into **funded bounties** backed by those escrows.
3. Contributors work via normal GitHub PRs.
4. Grainlify tracks contributions off‑chain and, when rules are satisfied, **triggers on‑chain payouts** directly to contributor wallets.

---

## 1. Funding Programs and Pools

- Ecosystem teams create **programs** (hackathons, grant rounds, continuous pools).
- For each program, they:
  - deploy or connect to a **program escrow contract**,
  - send tokens (e.g., XLM, USDC) to that escrow.
- The escrow holds funds on‑chain until payouts are triggered.

Grainlify never takes custody; it only orchestrates when and how escrows release funds.

---

## 2. Registering Projects and Bounties

- Maintainers log in with **GitHub** and register their repositories.
- They install the **Grainlify GitHub App** to enable webhooks and permissions.
- Inside Grainlify, maintainers:
  - mark issues as **funded bounties**,
  - assign bounty amounts or point values,
  - optionally lock funds for specific bounties from a program pool.

These funded issues are visible to contributors as clear, scoped opportunities.

---

## 3. Contributors Work via GitHub

- Contributors:
  - sign in with GitHub,
  - complete **KYC** (once, off‑chain) if they want payouts,
  - link a payout **wallet** (e.g., Stellar address).
- They then:
  - pick funded issues,
  - open PRs that reference those issues,
  - iterate with maintainers until PRs are merged.

All work happens using familiar GitHub workflows.

---

## 4. Off‑Chain Tracking and Scoring

Behind the scenes, Grainlify’s backend:

- listens to **GitHub webhooks** for issues and PRs,
- mirrors that data into its PostgreSQL database,
- links PRs to funded bounties and contributors,
- applies program rules and scoring (e.g., points, deadlines, multiple winners),
- checks **KYC status** and wallet binding for each contributor.

This entire step is off‑chain to keep the system flexible, efficient, and privacy‑preserving.

---

## 5. Payouts from Escrow

When bounties or programs reach a payout condition:

- Grainlify computes **who should be paid and how much**, based on:
  - merged PRs,
  - bounty configuration,
  - KYC eligibility,
  - available escrow balances.
- The backend sends **payout instructions** to the relevant smart contract(s).
- The **escrow contracts**:
  - verify authorization,
  - transfer tokens directly from escrow to contributor wallets,
  - emit events for each payout.

Contributors see payouts in:

- their on‑chain wallet,
- Grainlify’s **payout history** and analytics.

---

## 6. Analytics and Reporting

Grainlify provides views for:

- **Ecosystems**
  - program‑level and ecosystem‑level stats,
  - funds locked vs. funds paid,
  - contribution and payout timelines.
- **Maintainers**
  - per‑project analytics,
  - active contributors and bounties,
  - completed work and paid rewards.
- **Contributors**
  - personal contribution history,
  - earnings over time,
  - breakdown by project and ecosystem.

This closes the loop from **budget → work → payouts → impact**.

For detailed technical internals, see **Architecture Overview** and **Smart Contracts**.

