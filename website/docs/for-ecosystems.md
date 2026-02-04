---
description: Guide for ecosystem teams and foundations using Grainlify.
---

# Guide for Ecosystem Teams

This guide is for **ecosystem teams, foundations, DAOs, and protocol organizations** that want to run grant programs, hackathons, or continuous funding using Grainlify.

---

## What You Get as an Ecosystem

With Grainlify, you can:

- Lock **grant budgets** into on‑chain escrows.
- Run **time‑boxed programs** (hackathons, grant rounds).
- Support **continuous contributions** via always‑on bounties.
- Ensure payouts are:
  - **automatic** (rule‑based, triggered by real GitHub work),
  - **non‑custodial** (funds move directly from escrow to wallets),
  - **verifiable** (on‑chain payouts mapped to contributions).
- Access **analytics** on:
  - funds locked and funds paid,
  - active projects and contributors,
  - GitHub activity and ecosystem health.

---

## Concepts You Need to Know

- **Program** – a funding initiative (hackathon, grant round, long‑running pool).
- **Project** – a GitHub repository registered into a program or ecosystem.
- **Escrow** – an on‑chain contract holding funds for a program or set of bounties.
- **Bounty** – a funded issue or task within a project.
- **Contributor** – a GitHub user who submits PRs and gets paid after verification.

---

## End‑to‑End Flow (Ecosystem Perspective)

1. **Plan the program**
   - Define objectives, scope, and duration.
   - Decide budget, eligible ecosystems, and any rules you want enforced.
2. **Fund the program escrow**
   - Lock the budget into a **program escrow contract** on Stellar (Soroban).
   - Obtain a transaction or contract reference for internal records.
3. **Onboard projects and maintainers**
   - Invite maintainers to register projects and install the GitHub App.
   - Optionally define eligibility criteria (e.g., specific repos or tags).
4. **Program runs**
   - Contributors work through GitHub issues and PRs.
   - Grainlify tracks activity, points, and eligibility off‑chain.
5. **Scoring & verification**
   - Grainlify’s backend computes final scores based on rules and activity.
   - Only **KYC‑verified contributors** are included in payouts.
6. **Payout execution**
   - Backend triggers payouts from program escrow to contributors’ wallets.
   - Transfers are recorded on‑chain, mapping budget to real work.
7. **Reporting**
   - View analytics dashboards or export data for internal reporting.

---

## Setting Up Your First Program

> The exact UI names may differ slightly based on your deployment and branding, but the workflow remains consistent.

### Step 1 – Create a Program

- In the ecosystem dashboard:
  - Click **“Create Program”** (or similar).
  - Enter:
    - Program name (e.g., “Q1 Open Source Grants”).
    - Description and objectives.
    - Start and end dates (if time‑boxed).
    - Ecosystem or chain context (e.g., Stellar).
  - Save the program.

### Step 2 – Fund the Program Escrow

- From the program detail page:
  - Obtain the escrow **contract address** or funding instructions.
  - Use your organization’s wallet or treasury setup to:
    - Send the intended budget (e.g., XLM, USDC) to the program escrow contract.
  - Once the transaction confirms, the program will show:
    - **Total funds locked.**
    - **Available balance.**

> Grainlify does not take custody of your funds. They live in the escrow contract under your program.

### Step 3 – Invite Projects and Maintainers

- Share a public link or onboarding guide for maintainers.
- Encourage maintainers to:
  - Register their GitHub projects into the program.
  - Install the GitHub App to enable real‑time tracking.
  - Create funded issues/bounties with clear scope.

Grainlify will then start tracking contributions across these projects.

---

## Working with Time‑Boxed Programs

For **hackathons or grant rounds**:

- Define:
  - A **submission window** (when contributions count).
  - Any rules around:
    - minimum PR quality,
    - review processes,
    - maximum payouts per contributor.
- During the program:
  - Monitor dashboards for:
    - active contributors,
    - number of funded issues solved,
    - remaining budget.
- At the end:
  - Confirm final scoring and eligibility.
  - Trigger final payouts from the program escrow.

The result is a fully auditable trail from **initial budget → contributions → payouts**.

---

## Working with Continuous Funding

For **always‑on bounties**:

- You can choose to:
  - Allocate a dedicated pool into an escrow that backs long‑running bounties.
  - Let maintainers continuously create and retire bounties.
- Contributors:
  - Pick issues at their pace.
  - Submit PRs.
  - Receive payouts whenever their work is validated and merged.

This model suits:

- Core protocol maintenance.
- Long‑term infrastructure projects.
- Ongoing tooling or documentation efforts.

---

## Monitoring and Reporting

Depending on your deployment, the ecosystem dashboard will provide:

- **Funds Overview**
  - Total funds locked in escrows.
  - Funds paid out.
  - Remaining balances.
- **Activity Overview**
  - Number of active projects.
  - Number of active contributors.
  - Issues and PRs processed.
- **Impact Metrics**
  - Contributions over time (e.g., weekly).
  - Distribution of payouts across projects and contributors.

You can typically:

- Filter by program, project, time range, or ecosystem.
- Export data for:
  - internal reporting,
  - community transparency,
  - compliance and audit.

---

## Compliance and KYC

Grainlify integrates with a KYC provider (e.g., **Didit**) to ensure that payouts are compliant without putting PII on‑chain.

- Contributors complete KYC **off‑chain**.
- Grainlify stores KYC status and links it to wallets.
- Smart contracts only receive instructions to pay wallets that are KYC‑approved.
- No personal data or GitHub history is stored on‑chain.

For details, see `security-and-compliance.md`.

---

## Best Practices for Ecosystems

- **Lock funds early** so contributors and maintainers have confidence.
- **Be explicit about rules** and expectations in program descriptions.
- **Encourage maintainers** to write clear, well‑scoped funded issues.
- **Treat Grainlify as a neutral execution layer**:
  - You define programs, scope, and intent.
  - Grainlify ensures fair, rule‑based execution and payouts.
- **Use analytics** after each program to:
  - refine future grant strategies,
  - highlight success stories,
  - share verifiable impact with your community.

