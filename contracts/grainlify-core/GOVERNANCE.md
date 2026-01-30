# Grainlify Governance System

## Overview

The Grainlify governance system enables decentralized decision-making for contract upgrades through a proposal and voting mechanism. This system replaces the traditional admin-only upgrade path with a community-driven process.

## Key Parameters

- **Voting Period:** Duration during which votes can be cast (e.g., 7 days).
- **Execution Delay:** Time-lock period after a proposal is approved before it can be executed (e.g., 2 days).
- **Quorum:** Minimum percentage of total possible votes that must be cast for a proposal to be valid (e.g., 50%).
- **Approval Threshold:** Minimum percentage of "For" votes (excluding abstentions) required for approval (e.g., 66.67%).
- **Proposal Expiration:** Proposals expire if not executed within a certain timeframe after the execution window opens.

## Governance Flow

1. **Proposal Creation**
   - Any address with the minimum required stake can propose a contract upgrade.
   - The proposal includes the new WASM hash and a description (symbol).
   - Voting starts immediately upon creation.

2. **Voting Period**
   - Eligible voters can cast their votes (`For`, `Against`, or `Abstain`).
   - Voting power is determined by the configured scheme:
     - `OnePersonOneVote`: Every address has equal power (1).
     - `TokenWeighted`: Power is proportional to token balance (integration required).
   - **Security:** Each address can only vote once per proposal.

3. **Finalization**
   - After the voting period ends, anyone can trigger the `finalize_proposal` function.
   - The system checks if the quorum and approval threshold requirements are met.
   - The proposal status is updated to `Approved` or `Rejected`.

4. **Execution**
   - Approved proposals enter a time-lock period (execution delay).
   - Once the delay has passed, anyone can call `execute_proposal`.
   - The contract's WASM is automatically updated to the proposed hash.
   - **Audit:** All executions are recorded and emitted as events.

5. **Expiration**
   - Proposals that are not executed within 7 days after the execution window opens are marked as `Expired` and can no longer be executed.

## Security Features

- **Double-Voting Prevention:** Robust checks ensure each address votes only once.
- **Time-locked Upgrades:** The execution delay provides a safety buffer for stakeholders to react to approved changes.
- **Minimum Stake Requirement:** Prevents spam proposals by requiring a significant commitment from the proposer.
- **Immutable Logic:** Proposals cannot be modified once created.

## TODO / Future Enhancements

- [ ] Integrate with a native Soroban token for precise `TokenWeighted` voting power.
- [ ] Implement a dynamic quorum based on historical participation.
- [ ] Add a formal "veto" mechanism for high-stakes upgrades.

---
*Grainlify Governance - Empowering Decentralized Evolution*
