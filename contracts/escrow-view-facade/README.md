# Escrow View Facade

## Deployment status and crate relationships

No network deployment or contract ID is recorded for this facade in the repository. It is a read-only query contract and does not custody escrow funds.

- Depends on: [program-escrow](../program-escrow/README.md) through Cargo. At runtime it queries the [bounty-escrow](../bounty_escrow/contracts/escrow/README.md) and program-escrow ABIs through local client bindings; bounty-escrow is not a Cargo path dependency here.
- Depended on by: no other in-repository Cargo package declares a dependency on escrow-view-facade.
- Unlike [view-facade](../view-facade/README.md), this facade builds bounty summaries and user portfolios rather than maintaining the general contract registry.

The Escrow View Facade is a read-optimized smart contract for the Grainlify ecosystem. Its goal is to aggregate views over the `BountyEscrow` core module, allowing for single-call queries while avoiding large frontend multi-call round trips.

## Architecture & Responsibilities

The Facade operates strictly as a read-only Layer-2 projection.
It does not hold any funds or mutate the underlying target contract. Integrators query the Facade, providing the instance of `BountyEscrow`, and receive structured snapshots containing amount, depositor info, pause flags, and metadata.

## Contract Endpoints

Integrators interacting with the contract have access to the following queries:

### `get_escrow_summary(env: Env, escrow_contract: Address, bounty_id: u64) -> Option<EscrowSummary>`
Fetches all relevant data associated with a specific bounty ID. Gracefully returns `None` without trapping if the ID is missing. Includes core storage mapped alongside the `EscrowMetadata`.

### `get_escrow_summaries(env: Env, escrow_contract: Address, bounty_ids: Vec<u64>) -> Vec<EscrowSummary>`
Returns a batched slice of valid summaries. Any missing IDs from the provided input array are omitted. 

### `get_user_portfolio(env: Env, escrow_contract: Address, user: Address) -> UserPortfolio`
Traverses and filters the aggregate data to compile a subset of `EscrowSummary` elements linked uniquely to a specific `soroban_sdk::Address`. Currently retrieves components where the queried user is listed directly as the depositor.

## Integration Details

Ensure the `BountyEscrow` contract address passed to the view facade has been fully initialized and active on the network.
WASM optimization bindings are hardcoded locally to mirror `BountyEscrow` definitions inside `src/bounty_escrow_bindings.rs`. Ensure to replicate schema updates if the base escrow schema undergoes major migrations.
