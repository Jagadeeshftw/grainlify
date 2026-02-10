---
description: Key concepts in Grainlify – programs, projects, escrows, and bounties.
---

# Key Concepts: Programs & Bounties

This page explains the core concepts you’ll see throughout Grainlify: **ecosystems, programs, projects, escrows, bounties, and contributors**.

---

## Ecosystem

An **ecosystem** is a funding source such as a foundation, DAO, or protocol team.

- Provides the **capital** for programs and pools.
- Locks funds into **on‑chain escrows**.
- Owns the high‑level goals and rules for how funds should be used.

Examples: a Stellar foundation program, a protocol’s ecosystem fund, or a DAO treasury.

---

## Program

A **program** is a structured funding initiative, such as:

- a hackathon,
- a themed grant round,
- a continuous funding pool.

Programs:

- have a **budget** locked in one or more escrows,
- define a **time window** (for time‑boxed events),
- group together **projects** and **bounties**,
- provide a unit for reporting and impact analysis.

---

## Project

A **project** is a GitHub repository registered into Grainlify.

- Owned and managed by **maintainers**.
- Linked to:
  - one or more **programs**,
  - one or more **ecosystems**.
- Contains **issues** that can be turned into funded bounties.

Projects are the bridge between high‑level funding and concrete technical work.

---

## Escrow

An **escrow** is an on‑chain smart contract that safely holds funds for:

- an entire **program**, or
- a specific set of **bounties**.

Escrows are:

- **non‑custodial** – Grainlify never holds funds directly.
- controlled by clearly defined **payout keys**.
- the source of truth for how much budget remains available.

Payouts always move **from escrow → contributor wallet**.

---

## Bounty

A **bounty** is a funded issue or task within a project.

- Defined by a maintainer on top of a **GitHub issue**.
- Includes:
  - a **bounty amount** or a set of **points**,
  - eligibility rules (e.g., deadlines, multiple winners),
  - a link to the backing **escrow** or program budget.

Lifecycle of a bounty:

1. **Open** – visible to contributors as a funded opportunity.
2. **In progress** – one or more contributors are actively working.
3. **Completed** – one or more PRs are merged and validated.
4. **Paid** – Grainlify triggers payouts from escrow to contributors.

---

## Contributor

A **contributor** is a GitHub user who:

- logs into Grainlify with GitHub,
- completes **KYC** (for payouts),
- links a payout wallet,
- works on funded issues via PRs.

Contributors have:

- a **profile** with contribution history,
- analytics such as calendars and stats,
- a payout history associated with their wallets.

---

## Roles vs. Concepts

- **Roles** (ecosystem, maintainer, contributor, admin) describe **who** is using Grainlify.
- **Concepts** (programs, projects, escrows, bounties) describe **what** those users are operating on.

For role details, see **User Roles**. For how all of this ties together end‑to‑end, see **How it Works**.

