# Program Escrow Storage Layout

## Overview
The `program-escrow` contract utilizes two parallel storage models for storing `ProgramData`:
1. **Legacy Singleton Storage**: `PROGRAM_DATA` key. This was the original storage model where the contract only managed a single program.
2. **Multi-Program Registry Storage**: `DataKey::Program(program_id)` key. This was introduced later to support multiple programs within the same contract instance.

## Why Both Exist
The legacy singleton storage (`PROGRAM_DATA`) is retained for backward compatibility. Many older indexers and downstream clients may still depend on the `get_program_info` function, which explicitly queries the `PROGRAM_DATA` key.

To support multiple programs without breaking existing clients, the contract maintains both models. When a program is initialized via `initialize_program`, its data is stored in the multi-program registry (`DataKey::Program(id)`) and is also dual-written to `PROGRAM_DATA` (making it the "active" or "primary" program from the legacy perspective).

## Interactions and Synchronization
Initialization and payout paths (like `initialize_program`, `lock_program_funds`, `single_payout_internal`, and `batch_payout_internal`) explicitly synchronize both storage locations. Whenever a program's state is mutated, the contract checks if the mutated program matches the one stored in `PROGRAM_DATA`. If it does, the updated `ProgramData` is written to both locations to prevent desynchronization.

## Migration Strategy
- `get_program_info` has been marked as deprecated.
- All new integrations and internal contract logic should prefer `get_program_info_v2(program_id)` to explicitly query a specific program.
- Eventually, once downstream clients have migrated to the multi-program registry accessors, the `PROGRAM_DATA` singleton dual-writes can be phased out in a future upgrade.
