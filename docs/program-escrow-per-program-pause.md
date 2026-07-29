# Per-Program Pause Flags

The `program-escrow` contract supports a **Per-Program Pause Override** in addition to the existing global pause flags. This allows the contract admin to surgically pause operations for a single misbehaving or disputed program without impacting the entire contract instance.

## Architecture

Pause state is evaluated using a **layered precedence** model:
1. **Global Pause Flags:** Checked first. If an operation is paused globally, it is blocked for all programs.
2. **Per-Program Pause Flags:** Checked second. If an operation is not paused globally, the contract checks the program-specific pause flags. If the operation is paused for that specific program, it is blocked.

**Precedence Rule:** An operation is blocked if **either** the global flag **or** the per-program flag is set.

## API Additions

The API for program-specific pause mirrors the global API:

*   `set_program_paused(env: Env, program_id: String, lock: Option<bool>, release: Option<bool>, refund: Option<bool>, reason: Option<String>, unpause_at: Option<u64>)`
    *   **Admin-only** operation.
    *   Targets a specific `program_id`.
    *   Supports the same TTL-based auto-unpause logic as the global flags.

*   `get_program_pause_flags(env: Env, program_id: String) -> PauseFlags`
    *   Returns the `PauseFlags` structure for the given `program_id`.
    *   If no flags have been explicitly set for the program, defaults to fully unpaused (all flags `false`).

## TTL Auto-Unpause Behavior

Both global and per-program pause flags support `unpause_at` (Time-To-Live). The TTL evaluations are processed independently:
*   If a global pause expires, the operation may still be blocked if the program-specific pause is active and has not expired.
*   If a program-specific pause expires, the operation may still be blocked if the global pause is active and has not expired.

## Internal Storage

Per-program pause flags are stored in instance storage under a new `DataKey::ProgramPauseFlags(String)` variant, ensuring backwards compatibility and schema separation from the global `DataKey::PauseFlags`.
