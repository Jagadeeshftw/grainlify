---
description: Overview of how KYC and personal data are handled in Grainlify.
---

# KYC Privacy Notice

This page explains, at a high level, how **Know Your Customer (KYC)** data is handled when you use Grainlify.

> This is **not legal advice**. For production use, your legal team should review and adapt this text to match your actual providers, jurisdictions, and policies.

---

## Why We Use KYC

Grainlify enables ecosystems to send **on‑chain payouts** to contributors around the world. In many jurisdictions and for many funding sources, compliance rules require that we:

- know who we are paying,
- prevent misuse of grant funding,
- satisfy AML/CTF and related requirements.

To do this, we integrate with a third‑party **KYC provider** (for example, Didit).

---

## What the KYC Provider Collects

When you start KYC from Grainlify, you are redirected to the provider’s flow. There, the provider may collect information such as:

- your legal name and date of birth,
- your address,
- identity documents (passport, ID card, etc.),
- selfie or liveness checks,
- any additional information required by law.

This information is collected and processed **by the KYC provider**, not by Grainlify directly.

---

## What Grainlify Stores

Grainlify does **not** store your raw KYC documents.

Instead, Grainlify stores only:

- your Grainlify and GitHub identifiers,
- a reference to the KYC session with the provider,
- your KYC **status**, for example:
  - `not_started`
  - `pending`
  - `in_review`
  - `verified`
  - `rejected`
  - `expired`

We use this status to determine whether your account is **eligible for payouts** from grant programs and bounties.

---

## How KYC Affects Payouts

- Only contributors with a **verified** KYC status are eligible to receive payouts.
- When Grainlify computes payout instructions, contributions from non‑verified users may:
  - be held until verification is complete, or
  - be excluded, depending on program rules.
- Smart contracts on‑chain only see:
  - wallet addresses,
  - assets and amounts to transfer.

No personal identity data or KYC details are ever written to the blockchain.

---

## Data Sharing

Grainlify may share:

- limited identifiers and status information with:
  - ecosystem partners funding programs,
  - compliance or audit partners where required by law.

We do **not** share your underlying KYC documents; those remain with the provider, subject to their own privacy policy.

---

## Your Rights

Depending on your jurisdiction, you may have rights related to:

- accessing your personal data,
- correcting inaccurate information,
- requesting deletion where appropriate.

To exercise these rights:

- follow the instructions in the KYC provider’s own privacy policy, and
- contact the Grainlify operator using the contact information provided in your deployment (e.g., a dedicated privacy email).

---

## More Information

For a deeper technical description of how KYC, data, and security are handled in Grainlify, see **Security and Compliance**.

