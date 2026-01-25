# 📘 Grainlify — Grant Execution Infrastructure for Open Source

## 1. Problem Statement

Open-source ecosystems regularly allocate **grant funding** to support critical projects and contributors.

However, today’s grant execution model has major gaps:

- Grant money is often managed **off-chain**
- Project maintainers manually distribute rewards
- Contributor payments are delayed or subjective
- Ecosystems lack verifiable proof of how grants convert into real work
- Contributors must trust platforms or maintainers to be paid fairly

Platforms like OnlyDust solve **grant discovery and coordination**, but **grant execution and payout automation remain largely manual**.

---

## 2. Our Core Idea (One Sentence)

> **Grainlify is a grant execution layer that converts ecosystem funding into automated, verifiable payments for open-source contributions.**

---

## 3. High-Level Concept

Grainlify sits **between ecosystems and contributors**, ensuring that:

1. Ecosystems fund **programs** (hackathons, grant rounds)
2. Programs fund **projects**
3. Projects fund **contributors**
4. All payouts are:
   - escrow-backed
   - rule-based
   - automated on real GitHub work
   - verifiable on-chain

---

## 4. Two Core Execution Modes

Grainlify supports both **time-boxed programs** and **continuous contributions**.

This enables **continuous OSS funding**, not just events.

---

## 5. System Roles

### Ecosystem (Stellar)

- Provide grant capital
- Define program scope and duration
- Gain transparent, verifiable impact data

### Grainlify Platform

- Operates grant programs and hackathons
- Tracks contributions and scoring
- Enforces execution rules
- Automates payouts
- Handles compliance off-chain

### Project Maintainers

- Register projects
- Create issues and bounties
- Define point values
- Lock funds into escrow

### Contributors

- Work normally on GitHub
- Submit PRs
- Earn points or rewards
- Get paid automatically after verification

---

## 6. What Lives Where (Very Important)

### Off-Chain (Backend)

Stored and processed off-chain:

- projects and repositories
- GitHub issues and PRs (via webhooks / API)
- point assignments and scoring
- contributor rankings
- KYC status
- payout calculation

This keeps the system:

- flexible
- efficient
- privacy-safe

### On-Chain (Smart Contracts)

Stored on-chain **only**:

- program or hackathon escrow
- total funds locked
- remaining balance
- authorized payout key
- final payouts to wallets

No identities, points, or GitHub data are stored on-chain.

---

## 7. End-to-End Flow (Hackathon Example)

### Step 1 — Program Funding

- Ecosystem locks funds into an on-chain escrow  
  Example: 50,000 XLM for “Stellar Q1 OSS Program”

### Step 2 — Hackathon Runs

- Projects participate
- Contributors work on GitHub
- Points are awarded off-chain

### Step 3 — Scoring & Eligibility

- Backend computes final scores
- Only **KYC-approved contributors** are eligible for payout

### Step 4 — Automated Payout

- Backend triggers escrow contract
- Funds are sent **directly** to contributor wallets
- Backend never holds funds

### Hackathon Flow (Diagram)

```mermaid
flowchart TB
    Ecosystem["Ecosystem / Foundation"]
    Escrow["On-chain Escrow<br/>Stellar (Soroban) Smart Contract"]
    Backend["Grainlify Backend"]
    GitHub["GitHub<br/>Issues & PRs"]
    Contributors["Contributors"]
    Wallets["Contributor Wallets"]

    Ecosystem -->|"Lock Prize Pool"| Escrow
    Contributors -->|"Work & PRs"| GitHub
    GitHub -->|"Webhooks"| Backend
    Backend -->|"Score & KYC Check"| Backend
    Backend -->|"Trigger Payout"| Escrow
    Escrow -->|"Direct Transfer"| Wallets
```

### Detailed Sequence Flow

```mermaid
sequenceDiagram
    participant E as Ecosystem
    participant S as Stellar (Soroban) Escrow Contract
    participant B as Grainlify Backend
    participant G as GitHub
    participant C as Contributor
    participant W as Contributor Wallet

    E->>S: Lock Program Funds
    C->>G: Submit PRs
    G-->>B: PR Merged Webhook
    B->>B: Award Points (Off-chain)
    B->>S: Trigger Payouts
    S->>W: Transfer XLM
```

---

## 8. KYC & Compliance Model

KYC is handled entirely off-chain.

- Contributor identity and status are never stored on-chain
- Only wallets linked to approved KYC records are included in payouts

This ensures:

- privacy
- regulatory flexibility
- minimal on-chain complexity

---

## 9. Why This Model Works

### For Ecosystems

- Funds locked before work begins
- Clear proof of grant usage
- Reduced misuse risk

### For Maintainers

- No manual reward handling
- Budget clarity
- Lower admin overhead

### For Contributors

- Predictable rewards
- Guaranteed payouts
- Merit-based scoring

---

## 10. Key Differentiation

| Feature | Traditional Grant Platforms | Grainlify |
|---|---:|---:|
| Grant discovery | ✅ | ✅ |
| Hackathons | ✅ | ✅ |
| Always-on bounties | ❌ | ✅ |
| On-chain escrow | ❌ | ✅ |
| Point-based allocation | ❌ | ✅ |
| Automated payouts | ❌ | ✅ |
| Non-custodial payments | ❌ | ✅ |

---

## 11. Design Principles

- **Escrow-first**: funds locked before work begins
- **Automation over trust**: payouts follow rules
- **Minimal on-chain logic**: contracts act as vaults
- **Off-chain intelligence**: scoring and verification stay flexible
- **Ecosystem-agnostic**: same execution model across chains

---

## 12. What Grainlify Is (and Is Not)

### Grainlify Is

- a grant execution infrastructure
- a payout automation layer
- a coordination system for OSS funding

### Grainlify Is Not

- a DAO
- a marketplace
- a GitHub replacement
- a custodial payment platform

---

## 13. Vision

Grainlify’s long-term vision is to become the default execution layer for open-source grants, enabling ecosystems to move from:

> “We funded projects.”

to:

> “We verifiably paid for real work.”

---

## 14. One-Line Summary

> **Grainlify turns ecosystem grants into automatic, verifiable payments for open-source contributions.**
