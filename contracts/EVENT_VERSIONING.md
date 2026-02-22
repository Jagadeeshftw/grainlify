# Event Versioning

## Rules

1. Legacy `v1` payloads may be unversioned.
2. Current `v2` payloads must include `version: 2`.
3. Newer payload versions (`v3+`) must preserve required fields used by indexers and SDK clients.
4. Parsers should:
   - default missing `version` to `1`
   - reject payloads missing required compatibility fields
   - ignore unknown additive fields

## Compatibility Guarantee

For event consumers in this repository:

- Backward compatibility: SDK parsing tests cover legacy/unversioned payloads.
- Forward compatibility: SDK parsing tests cover newer version tags with additive fields.
- Contract emission correctness: contract tests assert emitted payloads include `version: 2` tags on current emitters.

