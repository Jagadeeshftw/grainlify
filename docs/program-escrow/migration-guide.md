# Draft-to-Active Migration Guide

## Lifecycle
Draft -> publish_program() -> Active -> complete_program() -> Completed

## Storage Changes
- ProgramStatus::Draft added as tag 0
- Active shifts tag 0 to 1
- Completed shifts tag 1 to 2
- Cancelled shifts tag 2 to 3
- New field: published_at Option<u64>

## Migration Steps
1. Backup: near view <CONTRACT> list_programs
2. Deploy v2 WASM
3. Call migrate_v1_to_v2
4. Verify: near view <CONTRACT> list_programs_by_status Draft
5. Remove migration method and redeploy

## Security
- Deposits blocked on Draft
- Only owner can publish
- Migration is idempotent
