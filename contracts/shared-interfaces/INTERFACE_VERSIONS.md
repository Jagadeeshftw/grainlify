# Shared Interfaces Version Documentation

This document describes the supported interface versions for Grainlify smart contracts.

## Current Interface Version

**Version 1.0.0**

- **Major**: 1
- **Minor**: 0
- **Patch**: 0

## Interface Stability Guarantees

### Major Version (1.x.x)

The major version indicates the core interface structure. Breaking changes require a major version bump.

**Stable Interfaces:**
- `BountyEscrowTrait` - Standard interface for bounty escrow contracts
- `ProgramEscrowTrait` - Standard interface for program escrow contracts
- `Pausable` - Pause functionality for operations
- `AdminManaged` - Admin management interface
- `Versioned` - Version tracking interface

### Minor Version (x.0.x)

Minor versions add new functionality while maintaining backward compatibility.

**Current Features:**
- All core traits defined
- Common error codes (100-199)
- Escrow status enumeration
- Compile-time version checking macro

### Patch Version (x.x.0)

Patch versions include documentation updates and internal improvements.

## Supported Contracts

| Contract | Interface Version | Notes |
|----------|------------------|-------|
| `BountyEscrowContract` | 1.0.0 | Implements `BountyEscrowTrait` |
| `ProgramEscrowContract` | 1.0.0 | Implements `ProgramEscrowTrait` |
| `GrainlifyContract` | 1.0.0 | Implements `Versioned`, `AdminManaged` |

## Error Code Ranges

| Range | Category |
|-------|----------|
| 100-199 | Common errors (shared across all contracts) |
| 200-299 | Bounty escrow specific errors |
| 300-399 | Program escrow specific errors |
| 400-499 | Governance specific errors |

## Breaking Change Policy

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

## Testing Interface Compatibility

All contracts should include tests that verify interface compatibility:

```rust
#[cfg(test)]
mod interface_compat_tests {
    use shared_interfaces::{BountyEscrowTrait, Versioned};
    
    #[test]
    fn test_interface_version() {
        crate::assert_interface_version!(1, 0, 0);
    }
    
    #[test]
    fn test_trait_implementation() {
        // Verify contract implements required traits
    }
}
```

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2024-02-21 | Initial interface definitions |

## Migration Guide

When upgrading contracts to support new interface versions:

1. **Check version compatibility** using `assert_interface_version!` macro
2. **Implement new trait methods** for minor version updates
3. **Update error handling** for new error codes
4. **Run compatibility tests** before deployment

## Contact

For questions about interface compatibility, open an issue on the repository.
