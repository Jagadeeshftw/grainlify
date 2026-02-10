---
description: On-chain smart contract design for Grainlify escrows and payouts.
---

# Smart Contracts

Grainlify uses **Stellar (Soroban)** smart contracts, written in **Rust**, to provide secure, non‑custodial escrow for grant funds and bounties.

This document describes:

- what the contracts do,
- how they relate to the off‑chain system,
- what guarantees they provide and their limitations.

---

## Design Philosophy

The smart contracts are designed to be:

- **Minimal** – only the logic necessary to secure and move funds lives on‑chain.
- **Non‑custodial** – Grainlify never has full custody; contracts hold funds and release them under rules.
- **Composable** – contracts can be combined to model:
  - program‑level escrows,
  - per‑bounty escrows,
  - vesting schedules (where implemented).
- **Ecosystem‑agnostic** – the same execution model can be reused across chains, even if initial implementation focuses on Stellar.

All rich business logic (scoring, KYC, analytics) runs **off‑chain** in the backend.

---

## Contract Types

Exact interfaces may evolve, but the core contract types are:

### 1. Program Escrow Contract

Represents a **program‑level funding pool**, e.g.:

- “Stellar Q1 OSS Program”
- “OSW Hackathon Prize Pool”
- “Continuous Funding Pool for Core Dev”

**Key responsibilities:**

- Accept deposits from an ecosystem or sponsor.
- Track:
  - total funds locked,
  - remaining balance.
- Authorize one or more **payout keys**:
  - keys controlled by Grainlify backend or separate governance logic.
- Execute payouts:
  - send tokens directly to contributor wallets,
  - log events for each payout.

This contract does **not** know:

- what GitHub issues or PRs exist,
- how many points a contributor has,
- anything about KYC,
- per‑project or per‑bounty details.

Those are handled off‑chain.

---

### 2. Bounty Escrow Contract (Optional / Variant)

In some deployments, you may also have **per‑bounty escrows**, where each funded issue has its own mini‑vault.

**Key responsibilities:**

- Accept deposits specifically for one bounty.
- Track whether bounty is:
  - open,
  - completed,
  - refunded (if unused).
- Allow a payout key to:
  - pay a winning contributor,
  - or refund unused funds to the original funder after conditions.

This pattern is useful when:

- individual bounties are funded by different parties,
- you want stronger isolation between bounties,
- you need clear on‑chain attribution per issue.

---

### 3. Vesting or Program Schedule Contracts (Where Implemented)

For certain programs, funds may:

- **vest over time** to projects,
- or be **released in tranches** based on milestones.

Vesting contracts can:

- hold a larger pool,
- expose methods to:
  - release a portion at fixed intervals,
  - enforce cliffs or schedules.

These may be used in addition to program escrows, depending on grant design.

---

## Off‑Chain vs On‑Chain Responsibilities

### On‑Chain (Contracts)

- Maintain balances of escrowed funds.
- Enforce basic invariants and authorization:
  - only authorized keys can trigger payouts,
  - no transfers beyond available balances.
- Trigger token transfers:
  - from escrow to recipient wallets.
- Emit events:
  - for deposits,
  - for payouts,
  - for configuration changes.

### Off‑Chain (Backend)

- Understand:
  - who users are (GitHub identity, KYC status),
  - what contributions they made (issues, PRs),
  - how points and rewards are computed.
- Decide:
  - which contributors should receive payouts,
  - how much each should receive,
  - from which escrow(s) payouts should come.
- Construct and sign:
  - on‑chain calls to escrow contracts.

This separation keeps contracts **simple, auditable, and safe**, while still enabling complex funding strategies off‑chain.

---

## Payout Flow (On‑Chain Perspective)

1. **Funding**
   - Ecosystem or sponsor sends tokens to the escrow contract.
   - Contract updates internal accounting and emits a “funded” event.

2. **Off‑Chain Computation**
   - Backend regularly:
     - syncs GitHub events,
     - computes scores,
     - checks KYC and eligibility.

3. **Payout Instruction**
   - Backend prepares a transaction:
     - listing recipients,
     - specifying amounts and assets,
     - referencing the relevant escrow.

4. **Contract Execution**
   - Contract verifies:
     - caller is an authorized payout key,
     - balances are sufficient.
   - Then performs transfers to recipients.

5. **Event Emission**
   - Contract emits events indicating:
     - who was paid,
     - how much,
     - which program or bounty it related to.

6. **Off‑Chain Reconciliation**
   - Backend records payouts in PostgreSQL.
   - Frontend dashboards update to reflect on‑chain state.

---

## Security Model

- Contracts are designed to:
  - only accept authority from specific keys,
  - never allow arbitrary withdrawals,
  - prevent overspending beyond balances.
- Payout keys are:
  - controlled by Grainlify backend and/or governance mechanisms,
  - ideally stored with best practices (HSMs, key management services, hardware wallets, etc., depending on deployment).
- No KYC or GitHub data is stored on‑chain:
  - only addresses and amounts,
  - preserving privacy and limiting regulatory surface on‑chain.

For end‑to‑end security, see `security-and-compliance.md`.

---

## Limitations and Non‑Goals

- Contracts **do not**:
  - track GitHub issues or PRs,
  - run complex scoring logic,
  - store or verify KYC data,
  - manage identities beyond wallet addresses.

They intentionally remain focused on:

- **holding funds**,
- **authorizing withdrawals**, and
- **executing token transfers**.

This keeps upgrade and audit surface small and makes the system easier to reason about.

