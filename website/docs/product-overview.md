---
description: High-level product overview of Grainlify – what it is, who it is for, and how it works.
---

# Product Overview

## What Is Grainlify?

**Grainlify is a grant execution layer that converts ecosystem funding into automated, verifiable payments for open‑source contributions.**

It sits **between ecosystems and contributors**, ensuring that:

1. **Ecosystems** fund on‑chain **programs** (hackathons, grant rounds, always‑on bounties).
2. **Programs** allocate budgets to **projects**.
3. **Projects** allocate funds to **contributors** via issues and pull requests.
4. All payouts are:
   - **escrow‑backed**
   - **rule‑based**
   - **driven by real GitHub activity**
   - **verifiable on‑chain**
   - **non‑custodial** (the backend never holds funds).

Grainlify does **not** decide who should be funded. It guarantees **how funding is executed** once a program, project, or ecosystem decides to run a grant or bounty.

---

## The Problem It Solves

Open‑source ecosystems regularly allocate large **grant budgets** but struggle with:

- **Manual execution** – payouts are handled by humans via spreadsheets and multisigs.
- **Subjective or delayed rewards** – contributors must trust maintainers to pay fairly.
- **Lack of verifiable proof** – ecosystems cannot easily answer “what did we actually pay for?”
- **Operational drag** – hackathons and grants require heavy admin work to settle payments.

As a result:

- Contributors are often paid late or not at all.
- Ecosystems cannot audit how grant funds converted into real work.
- Good coordination platforms exist, but **grant execution remains largely manual**.

Grainlify turns this into a **rule‑driven, automated, and auditable** execution layer.

---

## Core Idea in One Sentence

> **Grainlify turns ecosystem grants into automatic, verifiable, non‑custodial payouts for open‑source contributions.**

---

## Who Grainlify Is For

- **Ecosystem teams / foundations / DAOs**
  - Want verifiable execution of grants and hackathons.
  - Need non‑custodial, compliant payout rails.
  - Care about transparent reporting and impact.

- **Project maintainers**
  - Want to run bounties and attract contributors.
  - Don’t want to manually manage spreadsheets or distribution.
  - Need predictable budgets and automated payments.

- **Contributors**
  - Want to work via normal GitHub flows.
  - Want predictable, automatic payouts once work is merged.
  - Need to satisfy KYC once and then get paid directly to their wallet.

- **Operators / platform engineers**
  - Run the infrastructure, integrate with existing systems, and ensure uptime.

---

## Two Core Execution Modes

Grainlify supports two complementary execution modes:

### 1. Time‑Boxed Programs (Hackathons and Grant Rounds)

- Ecosystem locks a **program budget** into an on‑chain escrow contract.
- The program has a **start/end date** and rules for rewards.
- Projects participate, contributors submit work on GitHub.
- At the end of the program, the backend computes scores and **triggers payouts** from escrow.

Use cases:

- Seasonal hackathons.
- Multi‑week grant rounds.
- Thematic funding campaigns (“Q1 Governance Tools”, “DevEx Sprint”).

### 2. Continuous Contributions (Always‑On Bounties)

- Projects can set up **always‑on bounties** on their GitHub issues.
- Funds are **locked in escrow** per bounty or per project.
- When a PR that resolves a bounty is merged and verified, payout is **automatically released**.

Use cases:

- Long‑running OSS maintenance.
- Continuous contributor programs.
- Opportunistic bounties on important issues.

---

## High‑Level Features

### Grant & Program Management

- Program escrows with **on‑chain locked funds**.
- Time‑boxed hackathons and grant rounds.
- Continuous bounties and always‑on programs.
- Ecosystem‑level analytics and reporting.

### Project & Repository Management

- Project registration via **GitHub repositories**.
- GitHub App integration for **webhooks and permissions**.
- Project metadata (description, logo, links).
- Per‑project analytics and contributor stats.

### Issues, PRs, and Bounties

- Real‑time sync of **GitHub issues and pull requests**.
- Point‑based scoring for issues/bounties.
- Status tracking across the full lifecycle:
  - open → in progress → merged/closed → paid.
- Escrow‑backed bounties linked directly to GitHub work.

### Contributor Experience

- GitHub‑based login (no new identity needed).
- Public profiles with contribution history and stats.
- Contribution calendar and activity feed.
- Automatic payouts to linked wallets after KYC.

### Maintainer Experience

- Dashboards to:
  - register projects,
  - create bounties,
  - manage funds,
  - see analytics and contributor leaderboards.

### Ecosystem Experience

- Multi‑ecosystem support.
- Program‑level and ecosystem‑level reporting.
- Verifiable proof that “we paid for real work”.

---

## Execution Model – What Lives Where

### Off‑Chain (Backend)

Stored and processed **off‑chain**:

- Projects and repositories.
- GitHub issues, PRs, and activity.
- Point assignments and scoring.
- Contributor rankings and analytics.
- KYC status and wallet linking.
- Payout calculations and eligibility.

Reasons:

- Keep **logic flexible** and easy to evolve.
- Maintain **performance** and **low cost**.
- Preserve **privacy** (no PII or GitHub metadata on‑chain).

### On‑Chain (Smart Contracts)

Stored and enforced **on‑chain**:

- Program and bounty escrows.
- Locked balances and remaining funds.
- Authorized payout keys.
- Final payouts to recipient wallets.

No identities, points, or GitHub data are stored on‑chain—only what is needed to **securely move funds**.

---

## What Grainlify Is (and Is Not)

### Grainlify Is

- **Grant execution infrastructure** for open‑source ecosystems.
- **Payout automation layer** for GitHub‑based contributions.
- **Non‑custodial escrow system** for grant funds and bounties.
- **Coordination and analytics layer** for programs, projects, and contributors.

### Grainlify Is Not

- A DAO or governance platform.
- A general‑purpose marketplace.
- A GitHub replacement.
- A custodial wallet or exchange.

---

## Key Differentiators

- **Escrow‑first** – funds are locked before work starts; contributors know the pool exists.
- **Automation over trust** – payouts follow rules encoded off‑chain and backed on‑chain.
- **Minimal on‑chain logic** – smart contracts act as secure vaults; intelligence stays off‑chain.
- **Non‑custodial** – the backend never holds funds; contracts pay contributors directly.
- **Continuous funding** – supports both events (hackathons) and continuous bounties.
- **Verifiable impact** – ecosystems can point to on‑chain transfers mapped to real GitHub work.

---

## One‑Line Vision

Grainlify’s long‑term vision is to become the **default execution layer for open‑source grants**, so ecosystems can confidently say:

> “We verifiably paid for real work.”

