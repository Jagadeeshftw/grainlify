---
description: High-level architecture and technical overview of Grainlify.
---

# Architecture Overview

This document explains how Grainlify is built: the main components, data flows, and technical choices behind the product.

---

## High‑Level Architecture

At a high level, Grainlify consists of:

- a **frontend web application** (for all user personas),
- a **backend API and worker layer**,
- **smart contracts** (escrow vaults on Stellar/Soroban),
- **external integrations** (GitHub, KYC provider, blockchain nodes),
- a **PostgreSQL database** for off‑chain state.

Grainlify follows a hybrid **off‑chain intelligence + on‑chain escrow** model:

- All rich logic (scoring, KYC, analytics, GitHub sync) lives off‑chain.
- Smart contracts are focused and minimal, acting primarily as **non‑custodial vaults**.

---

## Frontend

The frontend is a **React + TypeScript** application built with:

- **React 18+**
- **Vite** for bundling and development
- **TypeScript** for type safety
- **Tailwind CSS** and UI libraries (e.g., Radix UI, MUI icons) for styling and components
- **React Router** for client‑side routing
- **React Hook Form** for forms
- **Recharts** and other visualization libraries for analytics

### Responsibilities

- Authentication flows (GitHub OAuth, session management).
- Role‑based user experiences:
  - contributor dashboards,
  - maintainer dashboards,
  - ecosystem dashboards,
  - admin views.
- Surfacing:
  - projects, programs, issues, and bounties,
  - contribution histories and leaderboards,
  - analytics and reporting.
- Interacting with:
  - backend REST/JSON APIs,
  - wallet and KYC UX flows (where applicable).

Frontend is generally deployable as a static app (e.g., Vercel/Netlify) configured to point to the backend API domain.

---

## Backend

The backend is a **Go (Golang)** service which exposes HTTP APIs and background workers.

### Core Technologies

- **Go** for API and workers.
- **Fiber** (fasthttp‑based framework) or a similar HTTP framework.
- **PostgreSQL** via the `pgx` driver (SQL‑first, no heavy ORM).
- **golang‑migrate** or equivalent for database migrations.
- **NATS** (optional) or another event bus for async processing.
- **JWT** for stateless authentication.

### Responsibilities

- **Authentication & Authorization**
  - GitHub OAuth login.
  - Admin login where applicable.
  - JWT issuance and verification.
  - Role‑based authorization checks.

- **GitHub Integration**
  - GitHub OAuth App and App installations.
  - Webhook endpoints for:
    - issues,
    - pull requests,
    - repository events.
  - GitHub REST API usage (with rate limiting).

- **Program, Project, and Bounty Management**
  - CRUD for ecosystems, programs, and projects.
  - Mapping projects to GitHub repos.
  - Storing bounty metadata and point values.
  - Tracking funding allocations and escrow relations.

- **Contribution Tracking**
  - Persisting issues and PRs mirrored from GitHub.
  - Linking PRs to funded issues.
  - Computing contribution metrics and points.
  - Producing analytics and leaderboards.

- **KYC & Compliance**
  - Managing KYC sessions with the provider (e.g., Didit).
  - Storing KYC status (without sensitive PII).
  - Enforcing payout eligibility rules.

- **Wallet & Payout Orchestration**
  - Linking user accounts to wallets.
  - Preparing payout instructions based on scoring.
  - Interacting with smart contracts to release escrows.

- **Background Processing**
  - Handling webhooks asynchronously.
  - Retrying failed tasks (e.g., GitHub calls, payout attempts).
  - Running periodic sync jobs.

---

## Database Layer

Grainlify uses **PostgreSQL** as the primary database.

Example types of data stored:

- **Users & roles**
  - GitHub ID, username, basic profile.
  - Roles (contributor, maintainer, admin).
  - Wallet addresses (off‑chain).

- **Ecosystems & Programs**
  - Ecosystem identifiers (e.g., Stellar).
  - Programs (hackathons, grant rounds, continuous pools).
  - Program metadata and settings.

- **Projects & Repositories**
  - Project metadata.
  - Linked GitHub repository info.
  - Ecosystem associations.

- **Issues & Pull Requests**
  - Mirrored issue and PR data.
  - Status transitions.
  - Linkage to bounties and programs.

- **Bounties & Funding**
  - Bounty configurations (amounts, points).
  - Escrow relationships and identifiers.
  - Accounting for allocations and balances (off‑chain view).

- **KYC & Compliance**
  - KYC statuses and session references.
  - No sensitive PII is stored—only status and identifiers.

- **Events & Jobs**
  - Webhook logs.
  - Background job queues (if persisted).

Database migrations are version‑controlled and applied via tooling in CI/CD or at startup.

---

## Smart Contracts (On‑Chain)

Grainlify uses smart contracts on **Stellar (Soroban)**, written in **Rust**, to manage escrows and payouts.

### Design Principles

- **Minimal logic** – contracts hold funds and perform authorized transfers.
- **Non‑custodial** – funds are held on behalf of programs/projects and transferred directly to contributor wallets.
- **Configurable** – contracts can represent:
  - program‑level escrows,
  - bounty‑level escrows,
  - vesting schedules (where implemented).

### Common Responsibilities

- Accepting deposits (funding from ecosystems or sponsors).
- Tracking balances per:
  - program,
  - bounty,
  - or configured key.
- Authorizing payout keys:
  - typically a backend‑controlled key (or multisig) that can trigger payouts per rules.
- Executing payouts:
  - transfers to contributor wallets based on instructions.
- Emitting events for:
  - deposits,
  - payouts,
  - relevant state changes.

Business logic (who gets how much and when) is determined **off‑chain** by the backend, then translated into simple contract calls.

For more detail, see `smart-contracts.md`.

---

## External Integrations

### GitHub

- Used for:
  - authentication,
  - project registration,
  - contribution tracking (issues & PRs).
- Components:
  - GitHub OAuth for user login.
  - GitHub App for:
    - granular repo permissions,
    - webhook delivery,
    - installation‑level auth.

### KYC Provider (e.g., Didit)

- Used for:
  - user identity verification,
  - regulatory compliance around payouts.
- Grainlify:
  - redirects contributors to the provider’s flow,
  - stores only KYC status and references,
  - never puts PII on‑chain.

### Blockchain / RPC

- Stellar / Soroban RPC nodes for:
  - interacting with escrow contracts,
  - reading balances and events.
- Horizon or equivalent APIs for:
  - addressing,
  - transaction lookups (if used).

### Event Bus (e.g., NATS)

- Optional but recommended for:
  - high‑volume webhook processing,
  - decoupling ingestion from business logic,
  - retry and resilience patterns.

---

## Data Flow: Hackathon Example

1. **Program Funding**
   - Ecosystem sends tokens to a **program escrow contract**.
   - Backend records the escrow metadata and balance.

2. **Contribution**
   - Contributor submits PRs against funded projects on GitHub.
   - GitHub sends webhooks to the backend.

3. **Off‑Chain Processing**
   - Backend validates events and associates PRs with funded issues.
   - Points/scores and KYC eligibility are checked.

4. **Payout Decision**
   - At program end (or incrementally), backend decides payout allocations.
   - It constructs a set of payout instructions (who, how much).

5. **On‑Chain Execution**
   - Backend calls the escrow contract with payout instructions.
   - Contract performs token transfers to contributor wallets.

6. **Reporting**
   - Backend updates internal records and analytics.
   - Ecosystem and contributors can see on‑chain and off‑chain views.

---

## Security & Reliability Considerations

- **JWT‑based authentication** with short‑lived tokens.
- **Webhook signature verification** for GitHub.
- **Rate limiting** for GitHub and external APIs.
- **Encrypted secrets** and secure storage of sensitive configuration.
- **Idempotent processing** for webhooks and payout instructions.
- **Separation of concerns**:
  - contracts only manage funds,
  - backend manages identities, scoring, and compliance.

For more, see `security-and-compliance.md`.

