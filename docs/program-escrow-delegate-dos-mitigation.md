# Delegate DoS Mitigation in Program Escrow

## Vulnerability Description
Prior to this mitigation, the `update_program_metadata_by` function in the `program-escrow` contract allowed delegates with the `DELEGATE_PERMISSION_UPDATE_META` permission to repeatedly rewrite `ProgramMetadata.custom_fields` without any restrictions. A malicious or compromised delegate could exploit this to spam metadata updates with excessively large payloads, inflating storage costs and TTL-extension fees for the program owner. This served as a griefing vector since `DELEGATE_PERMISSION_UPDATE_META` is intended as a low-privilege role without the ability to release or refund funds.

## Mitigations

To protect the program owner from storage-bloat griefing, two mechanisms have been implemented:

### 1. Per-Program Rate Limiting for Delegates
Delegates are now subject to a minimum-interval rate limit on metadata updates. A new `DataKey` (`DelegateMetadataRateLimit`) tracks the timestamp of the last metadata update by any delegate for a given program. If a delegate attempts to update the metadata before the interval has elapsed, the transaction will revert with the error `"Delegate metadata update rate limit exceeded"`.

- **Interval:** 60 seconds (defined by `DELEGATE_METADATA_UPDATE_INTERVAL`).
- **Scope:** Applies specifically to delegate-invoked calls. The admin (program owner) bypasses this rate limit and can update metadata freely.

### 2. Hard Cap on Custom Fields
To prevent unbounded storage growth even within the rate limit, the size of `ProgramMetadata.custom_fields` is strictly capped. This cap applies universally to all callers, including the admin, to enforce predictable storage bounds.

- **Maximum Fields:** 10 (defined by `MAX_PROGRAM_METADATA_CUSTOM_FIELDS`).
- If an update attempts to set more than 10 custom fields, the transaction will revert with the error `"Metadata custom fields exceed limit"`.

## Testing and Verification
A dedicated test suite (`delegate_metadata_dos_tests.rs`) has been added to verify these mitigations:
- `test_delegate_rate_limit_engages`: Asserts that a delegate cannot spam updates within the 60-second window, but can successfully update after the interval elapses.
- `test_admin_bypasses_rate_limit`: Confirms that the admin is not restricted by the delegate rate limit.
- `test_custom_fields_size_cap`: Ensures that any attempt to exceed 10 custom fields fails predictably.
