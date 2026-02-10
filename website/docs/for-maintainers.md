---
description: Guide for project maintainers using Grainlify to fund and reward contributors.
---

# Guide for Project Maintainers

This guide is for **project maintainers** who want to:

- register their GitHub repositories in Grainlify,
- create funded issues and bounties,
- lock funds into escrow, and
- automatically pay contributors for merged work.

---

## What You Get as a Maintainer

With Grainlify, you can:

- Turn your GitHub issues into **funded bounties**.
- Use **escrow‑backed budgets** so contributors know funds are real.
- Let contributors work via normal GitHub workflows (issues and PRs).
- Automate payouts once work is merged and validated.
- Track **project‑level analytics**:
  - number of contributors,
  - issues resolved,
  - payouts made.

---

## Prerequisites

- A GitHub account with maintainer or admin access to the repository.
- Participation in an ecosystem program **or** access to a funding pool.
- A basic understanding of:
  - **issues** and **pull requests** on GitHub,
  - your project’s priorities and roadmap.

---

## Step‑by‑Step: From Project to Paid Contributions

### Step 1 – Sign In and Register Your Project

1. Log in to Grainlify using **GitHub OAuth**.
2. Navigate to the **Maintainer Dashboard**.
3. Click **“Register Project”** (or equivalent).
4. Select or enter your GitHub repository.
5. Provide project metadata:
   - description,
   - logo (optional),
   - links (docs, website, etc.).
6. Confirm registration.

Your project will now appear in the Grainlify ecosystem and can participate in programs and bounties.

---

### Step 2 – Install the GitHub App

To track contributions and automate workflows, Grainlify uses a **GitHub App**.

1. From your project’s settings or prompt in the UI, start the **GitHub App installation**.
2. Choose:
   - to install on **all repositories**, or
   - **only the relevant repositories** you want to manage via Grainlify.
3. Grant required permissions:
   - read/write access to issues and pull requests,
   - webhook delivery for relevant events.

Once installed, Grainlify will start receiving:

- issue events (created, updated, closed),
- pull request events (opened, merged, closed),
- other relevant GitHub activity.

---

### Step 3 – Connect to a Program or Funding Source

There are two primary ways your project can be funded:

1. **Program‑based funding** – your project participates in an ecosystem program (hackathon, grant round) which has a **program escrow**.
2. **Direct or continuous funding** – your project or organization directly locks funds into an escrow contract for ongoing bounties.

In the Grainlify UI:

- Associate your project with one or more programs.
- Or configure a **project‑level escrow** or funding source if supported by your deployment.

---

### Step 4 – Create Funded Issues / Bounties

Now you can turn your roadmap into funded work:

1. Go to your project’s **Issues** view in Grainlify.
2. Select existing GitHub issues or create new ones.
3. For funded issues, specify:
   - **bounty amount** (e.g., 200 XLM),
   - optional **points** or scoring weight,
   - label or category if needed,
   - clear, detailed description and acceptance criteria.
4. Mark the issue as a **funded bounty** in Grainlify.

Depending on your configuration:

- You can either fund bounties from a **shared project pool**, or
- Lock funds into dedicated **per‑issue escrows**.

---

### Step 5 – Lock Funds into Escrow

To signal seriousness and give contributors confidence:

1. Navigate to the **Funding** or **Escrow** section for your project or bounty.
2. Choose:
   - which bounty or pool you’re funding,
   - which asset (e.g., XLM, USDC) and amount.
3. Follow the on‑screen instructions to send funds to the **escrow contract address**.

Once confirmed:

- The UI will show:
  - **total funded** amount,
  - remaining balance,
  - and which issues are backed by escrow.

Contributors can now see that **real funds** back your bounties.

---

### Step 6 – Manage Contributors and PRs

Contributors will:

- browse funded issues,
- apply or start work,
- open PRs referencing those issues.

In your maintainer workflow:

- Use GitHub as usual to:
  - review PRs,
  - request changes,
  - merge approved work.
- Grainlify will:
  - receive webhook events when PRs are opened/updated/merged,
  - associate PRs with funded issues,
  - mark contributions as **eligible for payout** once merged and validated.

You can use the Grainlify UI to:

- see which issues have active contributors,
- track PR status and associated bounties,
- view a project‑level **activity feed** and **leaderboard**.

---

### Step 7 – Trigger or Approve Payouts

Depending on program rules and implementation:

- Payouts can be:
  - **fully automatic** upon certain conditions (e.g., merged PR + KYC‑verified contributor),
  - or require a **maintainer or ecosystem approval step** before escrow release.

In practice:

1. Grainlify computes the payout allocation off‑chain (points, rules, eligibility).
2. The backend sends a **payout instruction** to the on‑chain escrow contract.
3. The escrow contract transfers funds **directly** to contributor wallets on‑chain.

You do **not** need to:

- manage manual multisig payouts,
- export spreadsheets,
- coordinate off‑platform transactions.

---

## Project Analytics

As a maintainer, you can access project‑level analytics such as:

- Number of:
  - funded issues,
  - completed bounties,
  - active contributors.
- Total:
  - funds allocated,
  - funds paid out,
  - remaining budget.
- Contribution activity over time:
  - PRs per week/month,
  - merged vs. closed without merge.

These insights help you:

- understand which areas of your project attract contributors,
- plan future funding,
- report outcomes to your ecosystem or sponsors.

---

## Best Practices for Maintainers

- **Write clear issue descriptions** with:
  - scope,
  - deliverables,
  - acceptance criteria.
- **Price bounties realistically**:
  - consider complexity, urgency, and impact.
- **Communicate expectations**:
  - coding standards,
  - review process,
  - timelines.
- **Be responsive on GitHub**:
  - acknowledge contributors quickly,
  - review PRs regularly,
  - provide constructive feedback.
- **Use analytics** to:
  - identify high‑impact contributors,
  - refine what you fund next,
  - share outcomes with your ecosystem.

