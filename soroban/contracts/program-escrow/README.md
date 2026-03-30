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

## Label semantics

- programs can store up to 10 labels
- each label must be 1..=32 characters
- duplicate labels are collapsed while preserving first-seen order
- admins can switch labels between open mode and a restricted allowlist
- labels are queryable through `get_programs_by_label`
- label creation and updates emit dedicated events for indexers

## Error codes

All public entrypoints return `Result<T, Error>` with stable, machine-readable
error codes. Clients should use these codes for programmatic error handling.

### Error code table

| Code | Name | Description | Client Action |
|------|------|-------------|---------------|
| 1 | `AlreadyInitialized` | Contract has already been initialized | None required; contract is ready for use |
| 2 | `NotInitialized` | Contract has not been initialized | Call `init()` with admin and token addresses first |
| 3 | `ProgramExists` | Program with this ID already exists | Use a different `program_id` or query existing programs |
| 4 | `ProgramNotFound` | Program with this ID does not exist | Verify the `program_id` is correct and that the program has been registered |
| 5 | `Unauthorized` | Caller is not authorized to perform this action | Ensure the correct address is signing the transaction |
| 6 | `InvalidBatchSize` | Invalid batch size (0 or >20) | Ensure batch contains 1-20 items (inclusive) |
| 7 | `DuplicateProgramId` | Duplicate program ID within a single batch | Ensure all `program_id` values in the batch are unique |
| 8 | `InvalidAmount` | Invalid funding amount (zero or negative) | Provide a positive funding amount |
| 9 | `InvalidName` | Invalid program name (empty) | Provide a non-empty program name |
| 10 | `ContractDeprecated` | Contract is deprecated and no longer accepts new registrations | Check deprecation status before registration; migrate to target contract if specified |
| 11 | `JurisdictionKycRequired` | KYC attestation required by jurisdiction | Provide valid KYC attestation before registration |
| 12 | `JurisdictionFundingLimitExceeded` | Funding exceeds jurisdiction limit | Reduce funding amount or update jurisdiction limits |
| 13 | `JurisdictionPaused` | Registration paused for this jurisdiction | Wait for jurisdiction to resume registration or use a different jurisdiction |
| 14 | `InvalidLabel` | Invalid label format (empty or >32 chars) | Ensure labels are 1-32 characters in length |
| 15 | `TooManyLabels` | Too many labels (>10) | Reduce the number of labels to 10 or fewer |
| 16 | `LabelNotAllowed` | Label not in allowed list | Use an allowed label or request admin to update the label configuration |

### Error handling best practices

1. **Always check error codes**: Use the error code for programmatic handling,
   not the error message string.
2. **Log errors safely**: Error messages never expose sensitive data (keys,
   balances, internal state), so they are safe to log.
3. **Handle jurisdiction errors**: Jurisdiction errors (11-13) indicate
   compliance issues that may require user action.
4. **Batch error atomicity**: Batch operations fail atomically; if any item
   fails, no items are registered.
5. **Deprecation checks**: Check deprecation status before attempting
   registration to provide better user experience.

### Security notes

- Error messages never expose sensitive data (keys, balances, internal state)
- Error codes are deterministic and stable for client-side discrimination
- All errors are safe to log and display to end users
- Error codes are stable across contract versions
