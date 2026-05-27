# Error Code Registry Implementation

## Overview

This document describes the enhanced error code registry system implemented for the Grainlify smart contract repository. The system provides compile-time uniqueness checks, cross-contract validation, and comprehensive documentation for error codes.

## Features

### 1. Compile-Time Uniqueness Checks
- **Build-time validation**: Error code duplicates are caught during compilation, not at runtime
- **Const assertions**: Uses `const` functions to ensure uniqueness without runtime overhead
- **Automatic failure**: Any duplicate error codes cause immediate build failure

### 2. Cross-Contract Validation
- **Conflict detection**: Identifies error code conflicts across different contracts
- **Configurable resolution**: Supports error, warning, or ignore modes for conflicts
- **Exclusion lists**: Allows specific contracts to be excluded from validation

### 3. Range-Based Organization
- **Standardized ranges**: Error codes follow established conventions:
  - 1-99: Common errors
  - 100-199: Governance errors
  - 200-299: Escrow errors
  - 300-399: Identity/KYC errors
  - 400-499: Program escrow errors
  - 1000+: System errors
- **Range validation**: Ensures all error codes are within appropriate ranges
- **Category lookup**: Provides functions to determine the category of any error code

### 4. Enhanced Schema Validation
- **JSON Schema**: Strict validation rules for contract manifests
- **Required fields**: Ensures all necessary error registry information is present
- **Format validation**: Enforces PascalCase naming and proper descriptions
- **Cross-validation**: Validates consistency between codes and declared ranges

## Implementation Details

### Core Components

#### 1. Error Registry Module (`error_registry.rs`)
```rust
// Main registry with compile-time uniqueness check
pub const GRAINLIFY_CORE_REGISTRY: &[RegistryEntry] = &[
    (1, "AlreadyInitialized"),
    (2, "NotInitialized"),
    // ... more entries
];

// Compile-time uniqueness assertion
const _UNIQUENESS_CHECK: () = {
    if has_duplicate_codes(GRAINLIFY_CORE_REGISTRY) {
        panic!("Duplicate error code detected");
    }
};
```

#### 2. Cross-Contract Validation Functions
```rust
// Check for conflicts with another contract's registry
pub const fn no_cross_contract_conflicts(other_registry: &[RegistryEntry]) -> bool

// Validate that error codes follow range conventions
pub const fn validate_ranges() -> bool

// Get the category for a specific error code
pub const fn get_range_category(code: u32) -> &'static str
```

#### 3. Enhanced Manifest Schema
The `contract-manifest-schema.json` includes:
- Required `range` field for each error code
- Cross-contract validation configuration
- Strict format validation for names and descriptions
- Range boundary validation

#### 4. Validation Script (`validate_seed_file.py`)
Enhanced with:
- Range compliance checking
- PascalCase name validation
- Cross-validation between codes and ranges
- Detailed error reporting

### Usage Guidelines

#### Adding New Error Codes

1. **Add to ContractError enum**:
```rust
#[contracterror]
pub enum ContractError {
    // Existing errors...
    NewError = 108, // Choose appropriate number within range
}
```

2. **Update registry**:
```rust
pub const GRAINLIFY_CORE_REGISTRY: &[RegistryEntry] = &[
    // Existing entries...
    (108, "NewError"),
];
```

3. **Update manifest**:
```json
{
  "error_registry": {
    "codes": [
      {
        "code": 108,
        "name": "NewError",
        "description": "Detailed description of when this error occurs",
        "range": "governance",
        "added_in": "2.3.0"
      }
    ]
  }
}
```

4. **Run validation**:
```bash
python3 scripts/validate_seed_file.py
```

#### Range Allocation Guidelines

- **Common (1-99)**: Initialization, authorization, validation errors
- **Governance (100-199)**: Upgrade, proposal, voting, snapshot errors
- **Escrow (200-299)**: Bounty, payment, batch operation errors
- **Identity (300-399)**: KYC, signature, claim verification errors
- **Program Escrow (400-499)**: Program-specific errors
- **System (1000+)**: Circuit breaker, critical system errors

## Testing

### Comprehensive Test Coverage

The implementation includes extensive tests covering:
- Registry uniqueness and completeness
- Range validation and categorization
- Cross-contract conflict detection
- Name format conventions
- Edge cases and boundary conditions

### Test Categories

1. **Structure Tests**: Registry format and organization
2. **Uniqueness Tests**: No duplicate codes within contract
3. **Range Tests**: Proper range allocation and boundaries
4. **Integration Tests**: Cross-contract validation
5. **Convention Tests**: Naming and format standards

## Security Considerations

### Compile-Time Guarantees
- **No runtime bypass**: Compile-time checks cannot be disabled
- **Zero overhead**: Const functions have no runtime cost
- **Immediate feedback**: Errors caught before deployment

### Cross-Contract Safety
- **Namespace isolation**: While contracts have independent error namespaces, avoiding conflicts improves developer experience
- **SDK generation**: Clean error code separation enables better SDK generation
- **Audit clarity**: Distinct error codes simplify security audits

## Migration Guide

### For Existing Contracts

1. **Update manifests**: Add required `range` field to all error codes
2. **Fix ranges**: Ensure range boundaries follow conventions (1-99, 100-199, etc.)
3. **Add validation**: Include cross-contract validation configuration
4. **Run tests**: Execute validation script to verify compliance

### For New Contracts

1. **Choose ranges**: Select appropriate range based on contract type
2. **Follow conventions**: Use PascalCase names and descriptive text
3. **Include from start**: Add error registry section to initial manifest
4. **Validate early**: Run validation script during development

## Performance Impact

### Build Time
- **Minimal overhead**: Const functions execute during compilation
- **Fast validation**: Simple loops with early exit on conflicts
- **Parallelizable**: Validation can run alongside other build steps

### Runtime
- **Zero cost**: All validation happens at compile time
- **No storage**: Registry data doesn't consume contract storage
- **Optional lookup**: Runtime functions provided for debugging only

## Future Enhancements

### Planned Improvements
1. **Automatic code allocation**: Tool to suggest next available error code
2. **Cross-repository validation**: Global registry across all Grainlify contracts
3. **Documentation generation**: Auto-generate error code documentation
4. **IDE integration**: VS Code extension for error code validation

### Extension Points
- **Custom ranges**: Support for contract-specific range definitions
- **Validation rules**: Pluggable validation rule system
- **Reporting formats**: Multiple output formats for validation results

## Conclusion

The enhanced error code registry system provides:
- **Robust validation**: Compile-time uniqueness with comprehensive checks
- **Developer experience**: Clear guidelines and helpful error messages
- **Maintainability**: Organized, well-documented error code management
- **Security**: Early detection of potential issues
- **Scalability**: Support for growing contract ecosystems

This implementation establishes a foundation for reliable error code management across the Grainlify smart contract repository while maintaining performance and security standards.
