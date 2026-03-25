# Program Escrow Search Notes

## Search indexing assumptions

`program-escrow` search helpers are implemented with a persisted `ProgramIndex`
vector instead of direct storage scans.

- every successful registration appends its `program_id` to `ProgramIndex`
- `program_id` is the stable cursor value returned to clients
- `get_programs` walks the index in order and applies filters to loaded records
- missing records are skipped defensively during reads
- `limit` is clamped to `MAX_PAGE_SIZE` to keep query work bounded

## Review and security notes

- search helpers are read-only and do not mutate contract state
- the implementation avoids hidden full-storage scans by relying on the index
- cursor pagination keeps result windows predictable for wallets, dashboards,
  and indexers
- the current implementation assumes registrations are append-only for
  discoverability; if deletions are introduced later, the index maintenance
  rules should be updated alongside the query documentation and tests
