# 📘 Patchwork – Backend Architecture & System Flow

Patchwork is an infrastructure-grade platform that connects open-source developers with open-source projects through verifiable contributions, automated rewards, and reputation — within the Stellar ecosystem.

## 1. High-Level System Architecture

```mermaid
flowchart LR

    FE[Frontend - Next.js]

    API[API Gateway - Go/Fiber]

    AUTH[Auth Service]

    PROJ[Project Service]

    BOUNTY[Bounty Service]

    SUB[Submission Service]

    CHAT[Chat Service]

    BUS[NATS Event Bus]

    WORKERS[Async Workers]

    DB[(PostgreSQL)]

    REDIS[(Redis)]

    IPFS[(IPFS)]

    GITHUB[GitHub API/Webhooks]

    CHAIN["Blockchain RPCs<br>Stellar (Soroban)"]

    FE --> API

    API --> AUTH

    API --> PROJ

    API --> BOUNTY

    API --> SUB

    API --> CHAT

    PROJ --> DB

    BOUNTY --> DB

    SUB --> DB

    CHAT --> DB

    AUTH --> DB

    API --> BUS

    BUS --> WORKERS

    WORKERS --> DB

    WORKERS --> IPFS

    WORKERS --> CHAIN

    GITHUB --> API

    API --> REDIS
```

## 2. Core Backend Tech Stack

### Language & Runtime

Go 1.22+

### HTTP Layer

Fiber (fasthttp based)

### Database

PostgreSQL

Driver: pgx (no ORM)

### Cache & Rate Control

Redis

### Eventing / Async

NATS

### Blockchain

Stellar RPC (Soroban / Horizon)

### Storage

IPFS (verification proofs, metadata)

### Auth

Wallet signature auth

GitHub OAuth

JWT (short-lived)

## 3. Core Features (Backend-Owned)

### 🔐 Authentication

Wallet login (Stellar)

GitHub OAuth linking

Role-based access (maintainer / contributor / admin)

### 📦 Project Registration

Register OSS project

Verify GitHub repo ownership

Enable GitHub webhooks

Chain-agnostic project identity

### 🧵 Issue & PR Sync

Auto-sync GitHub issues

Track PR lifecycle

Deduplicate webhook events

Rate-limit safe GitHub fetchers

### 💰 Bounties & Grants

Create bounties on issues

Lock funds in on-chain escrow

Stellar-native rewards

Deadlines & milestones

### 🔀 Submissions & Verification

Auto PR submission tracking

Merge verification

Issue ↔ PR ↔ bounty matching

Proof generation (JSON → IPFS)

### 💸 Automated Payouts

Async payout execution

Retry-safe chain transactions

Escrow release (Soroban)

### 🧠 Reputation System

Contribution scoring

On-chain / off-chain hybrid

Leaderboards

PatchQuest snapshots

### 🏆 PatchQuest (Monthly Hackathon)

Time-boxed contribution cycles

Ranking by verified impact

Reward pools

Community visibility

## 4. End-to-End Contribution Flow

```mermaid
sequenceDiagram

    participant Dev as Developer

    participant FE as Frontend

    participant API as API (Go)

    participant GH as GitHub

    participant BUS as NATS

    participant VER as Verifier Worker

    participant PAY as Payout Worker

    participant CH as Blockchain

    Dev->>FE: Connect wallet + GitHub

    FE->>API: Auth request

    API->>API: Verify signature

    GH-->>API: PR merged webhook

    API->>BUS: PR_MERGED event

    API-->>GH: 200 OK

    BUS->>VER: Verify PR

    VER->>GH: Fetch PR/Commits

    VER->>VER: Generate proof

    VER->>BUS: VERIFIED_SUBMISSION

    BUS->>PAY: Execute payout

    PAY->>CH: Send tx

    CH-->>PAY: Tx confirmed

    PAY->>API: Update DB
```

## 5. Async Processing Model (Critical)

```mermaid
flowchart TD

    API_REQ[HTTP Request]

    FAST_RESP[Immediate Response]

    EVENT[NATS Event]

    WORKER[Worker Pool]

    EXT[External Systems]

    API_REQ --> FAST_RESP

    API_REQ --> EVENT

    EVENT --> WORKER

    WORKER --> EXT

    WORKER --> EVENT
```

### Rule

❌ No GitHub / RPC / heavy logic in HTTP path

✅ Everything slow is async

## 6. Chat System (Dev ↔ Maintainer)

### Purpose

Allow contributors to talk directly with maintainers

Scoped to project or bounty

Auditable & moderation-friendly

### Chat Architecture

```mermaid
flowchart LR

    DEV[Developer]

    MAIN[Maintainer]

    FE[Frontend]

    API[Chat API]

    WS[WebSocket Gateway]

    DB[(Postgres)]

    REDIS[(Redis Pub/Sub)]

    DEV --> FE

    MAIN --> FE

    FE --> WS

    WS --> API

    API --> DB

    API --> REDIS

    REDIS --> WS
```

### Chat Features (Backend)

Project-scoped rooms

Bounty-specific threads

Wallet-verified identity

Read receipts

Rate limiting

Moderation flags

Optional message hashing (future on-chain anchoring)

### Chat Data Model

chat_rooms

- id

- project_id

- bounty_id (nullable)

chat_messages

- id

- room_id

- sender_wallet

- content

- created_at

## 7. Database Schema (Core Tables)

```mermaid
erDiagram

    USERS ||--o{ PROJECTS : owns

    PROJECTS ||--o{ BOUNTIES : has

    BOUNTIES ||--o{ SUBMISSIONS : receives

    SUBMISSIONS ||--|| PAYOUTS : triggers

    PROJECTS ||--o{ CHAT_ROOMS : has

    CHAT_ROOMS ||--o{ CHAT_MESSAGES : contains
```

## 8. Security & Trust Guarantees

Webhook signature verification

Idempotent event processing

Replay protection

Chain tx confirmation checks

Audit logs

KYC/AML hooks (admin only)

## 9. Why This Backend Is Correct

Low latency (1–5ms API)

High concurrency (goroutines)

Fault-tolerant

Chain-agnostic

Infra-grade

Foundation-friendly

This backend can support:

Stellar Hackathon

Soroban escrow payouts

Long-term grants

Millions of PR events

## 10. Next Extensions (Planned)

AI PR reviewer

AI contributor matching

Milestone-based grants

DAO voting integration

Cross-chain identity

zk-proof-based verification


