# Event Schema

This repository currently supports two event payload generations:

- `v1` (legacy): unversioned payloads (required fields only)
- `v2` (current): payload map containing a `version: 2` field plus required fields

## Required Compatibility Fields

Indexers and SDK consumers must be able to parse these required fields across versions:

- `version` (optional in v1, required in v2+)
- `amount` (when event type includes value transfer semantics)

Additional fields are considered additive and should be ignored by forward-compatible parsers.

