# grainlify-core Module Boundaries

The contract root remains the discoverable API facade. Version migration hooks
live in `src/migration.rs`, and event schema compatibility lives in
`src/event_compatibility.rs`. Both modules are private implementation details;
entrypoint names, storage keys, event names, feature flags, and public errors
remain owned by the facade and are unchanged.

The migration module is intentionally dependency-light: it receives only the
Soroban environment and does not depend on contract entrypoint types. This
keeps future governance, storage, and migration extraction from introducing a
cycle.
