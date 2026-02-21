# test: add tests for cross-contract interface compatibility

Closes #71

## Summary

This PR adds shared interfaces and comprehensive tests for cross-contract interface compatibility, ensuring that all Grainlify escrow contracts maintain stable ABIs and catch breaking changes early.

## Changes

### New Files
- `contracts/shared-interfaces/` - New crate for shared interface definitions
  - `src/lib.rs` - Core trait definitions and version constants
  - `src/interface_tests.rs` - Comprehensive test suite (19 tests)
  - `INTERFACE_VERSIONS.md` - Version documentation

### Defined Interfaces
| Trait | Purpose |
|-------|---------|
| `BountyEscrowTrait` | Standard interface for bounty escrow contracts |
| `ProgramEscrowTrait` | Standard interface for program escrow contracts |
| `Pausable` | Granular pause functionality |
| `AdminManaged` | Admin management operations |
| `Versioned` | Version tracking and compatibility |

### Test Coverage
- ✅ Interface version stability tests
- ✅ Trait bounds verification
- ✅ Error code stability tests
- ✅ Escrow status enumeration tests
- ✅ Cross-contract interaction through traits
- ✅ Compile-time interface version checking
- ✅ Breaking change detection documentation

## Interface Version

**Current Version: 1.0.0**

The interface follows semantic versioning:
- **MAJOR**: Breaking changes to function signatures
- **MINOR**: New functions added (backward compatible)
- **PATCH**: Documentation or internal changes

## Breaking Change Policy

Documented criteria for what constitutes a breaking vs non-breaking change to help maintain interface stability across versions.

### Breaking Changes (require major version bump)
1. Removing a function from a trait
2. Changing a function's return type
3. Changing a function's parameter types
4. Changing a function's parameter order
5. Adding a required parameter (not `Option`)
6. Changing error code values
7. Removing an error code variant

### Non-Breaking Changes (minor version bump)
1. Adding a new function to a trait
2. Adding a new error code variant
3. Adding a new status variant
4. Adding an optional parameter

## How to Test

```bash
cd contracts/shared-interfaces
cargo test
```

All 19 tests should pass.

## Checklist
- [x] Fork the repo and create a branch
- [x] Implement shared interfaces/traits
- [x] Add interface compatibility tests
- [x] Add compile-time interface checks
- [x] Document supported interface versions
- [x] Run tests and commit
