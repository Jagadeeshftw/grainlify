---
description: Security, KYC, and compliance model for Grainlify in production.
---

# Security and Compliance

This document describes how Grainlify approaches **security**, **KYC**, and **compliance** in a production environment.

---

## Security Model Overview

Grainlify is designed with a few core principles:

- **Non‑custodial by default**
  - Grant funds are locked into **on‑chain escrows**, not held by the backend.
  - Payouts move **directly** from escrows to contributor wallets.
- **Minimal on‑chain data**
  - On‑chain contracts store only what is necessary to secure funds and execute payouts.
  - No PII, GitHub data, or KYC details appear on‑chain.
- **Off‑chain intelligence**
  - All complex logic (scoring, KYC status, contribution history) stays off‑chain.
  - This keeps the chain clean and flexible while making it easier to evolve logic.

---

## Authentication and Authorization

### Authentication

- **GitHub OAuth**
  - Primary method for contributors and maintainers.
  - Ties Grainlify accounts to GitHub identities (username, ID).
- **Admin Authentication**
  - Admins may have additional credential flows (e.g., password, SSO).

### Authorization

- Role‑based access control:
  - Contributor, Maintainer, Admin (and Ecosystem/Operator views).
- Enforcement at API layer:
  - each endpoint checks roles and permissions,
  - sensitive operations (e.g., admin actions, payout triggers) have stricter checks.

---

## Token and Secret Handling

- **JWT tokens**
  - Used to represent authenticated sessions.
  - Short‑lived where possible; refresh or re‑auth via GitHub when needed.
- **Encryption**
  - Sensitive fields (tokens, provider secrets, etc.) are stored encrypted where appropriate.
  - Keys loaded from secure environment configuration.
- **Secrets management**
  - Deployment should use a secure secrets store (e.g., environment variables managed by a vault or platform services).
  - Avoid hard‑coding secrets in code or configuration files.

---

## Webhook Security

Grainlify relies heavily on **GitHub webhooks**.

- **Signature verification**
  - Each incoming webhook is validated using the shared secret between the GitHub App and backend.
  - Unverified or malformed requests are rejected.
- **Idempotency**
  - Event processing is designed to be idempotent so that retries or duplicate events do not corrupt state.
- **Rate limiting and backpressure**
  - Webhook endpoints can be fronted by rate limiters or queues.
  - Background workers process events from queues to keep ingestion robust.

---

## KYC and Compliance

Grainlify integrates with a third‑party **KYC provider** (e.g., Didit) to verify contributor identities for payouts.

### Approach

- **Off‑chain KYC**
  - Contributors complete KYC entirely within the provider’s flow.
  - The provider returns:
    - session identifiers,
    - verification status,
    - webhook notifications for updates.
- **Minimal data in Grainlify**
  - Grainlify stores:
    - user identifier,
    - KYC status enum (`not_started`, `pending`, `in_review`, `verified`, `rejected`, `expired`),
    - references (IDs) to the KYC session.
  - Grainlify does **not** store raw documents, photos, or sensitive PII.

### Payout Eligibility

The backend enforces KYC as part of payout logic:

- Only users with **`verified`** KYC status are eligible to receive payouts.
- When computing payout instructions:
  - contributions from non‑verified users are either held or excluded (depending on program rules).
- This ensures:
  - compliance with ecosystem or jurisdictional requirements,
  - no personal data is pushed on‑chain.

---

## Data Privacy

### On‑Chain

- Only:
  - escrow balances,
  - payout amounts,
  - recipient wallet addresses,
  - minimal program identifiers (if any)
  are stored on‑chain.
- No personal identity data, GitHub usernames, or KYC details.

### Off‑Chain

- PostgreSQL stores:
  - user accounts (GitHub IDs, usernames),
  - wallets,
  - project and program metadata,
  - mirrored GitHub issues and PRs,
  - KYC status and session references.
- Operational logs and metrics should follow best practices for:
  - log retention,
  - access control,
  - incident response.

Deployers of Grainlify should:

- comply with relevant data protection laws (e.g., GDPR where applicable),
- update privacy policies to reflect:
  - what user data is stored,
  - for how long,
  - for what purpose.

---

## Operational Security

Recommended practices for running Grainlify in production include:

- **Network security**
  - Restrict backend and database access with firewalls and private networking.
  - Use TLS for all external communication.
- **Access control**
  - Use SSO and least‑privilege accounts for operators.
  - Separate staging and production environments.
- **Logging and monitoring**
  - Monitor:
    - error rates,
    - latency,
    - background job queues,
    - webhook failures.
  - Set up alerts for:
    - unusual traffic spikes,
    - repeated authentication failures,
    - abnormal payout patterns.
- **Backups and recovery**
  - Regular database backups.
  - Documented and tested restore procedures.
- **Key management**
  - Store payout keys securely (HSMs, KMS, or hardware wallets where appropriate).
  - Rotate keys periodically and on incident.

---

## Threat Model Highlights

Some key threats and mitigations:

- **Compromised backend**
  - Mitigation:
    - minimal authority on‑chain (contracts require specific keys),
    - strong key management (limit who/what can sign payouts),
    - monitoring payout volumes and patterns.
- **Webhook spoofing**
  - Mitigation:
    - strict signature validation,
    - HTTPS only,
    - IP allowlists if appropriate.
- **Abusive contributors**
  - Mitigation:
    - KYC requirements for payouts,
    - contribution review and acceptance controlled by maintainers,
    - program rules to cap per‑user rewards if needed.
- **Ecosystem misconfiguration**
  - Mitigation:
    - clear program creation flows,
    - admin review steps for large budgets,
    - test environments for dry runs.

---

## Summary

- Grainlify is **non‑custodial**, using smart contracts as vaults for program and bounty funds.
- All complex and sensitive logic (KYC, scoring, analytics) remains **off‑chain**.
- Payouts are:
  - rule‑based,
  - executed on‑chain,
  - auditable without exposing personal data.

Operators should combine these architectural safeguards with solid operational security and compliance processes for a robust production deployment.

