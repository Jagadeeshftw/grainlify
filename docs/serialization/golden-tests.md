# Golden Serialization Tests

Golden serialization tests ensure that our on-chain data structures (XDR) remain
consistent across contract upgrades. These tests capture the byte-level representation
of key structs and assert that changes to the codebase do not unintentionally break
on-chain compatibility.

## Testing Strategy

- **Fixtures**: Stored in `contracts/program-escrow/src/serialization_goldens.rs` as XDR hex strings.
- **Coverage**: Covers all variants of core structs, including:
  - Default/zero values
  - Maximum/edge values (max `i128`, max length strings)
  - Optional field states (`Some`/`None`)
- **Verification**: Tests in `contracts/program-escrow/src/test_serialization_compatibility.rs` compare live XDR serialization against these stored fixtures.

## Adding New Fixtures

If you add a new storage type or modify an existing one:
1. Update `contracts/program-escrow/src/lib.rs` with the new structure.
2. Run the serialization compatibility test with a generation flag (e.g., `GENERATE_GOLDENS=1 cargo test`).
3. Verify the generated output in `serialization_goldens.rs` before committing.
4. Ensure the new fixture follows the established format.
