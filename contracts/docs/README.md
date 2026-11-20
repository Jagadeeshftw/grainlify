# Contracts Documentation Index

This directory serves as the centralized, categorized index for all documentation under . Use this guide to quickly locate authoritative specifications for architecture, security, errors, events, fees, storage, and testing.

---

## 📚 Architecture & Overview

| Document | Description |
| :--- | :--- |
| [ARCHITECTURE.md](../ARCHITECTURE.md) | Core contract architecture, lifecycle models, and high-level component diagrams. |
| [Build_and_Validation_Guide.md](../Build_and_Validation_Guide.md) | Compilation, build targets, and contract validation runbooks. |
| [IMPLEMENTATION_SUMMARY.md](../IMPLEMENTATION_SUMMARY.md) | Summary of contract implementation phases and status. |
| [README.md](../README.md) | Top-level contracts directory overview and repository entrypoint. |
| [VERSIONS.md](../VERSIONS.md) | Contract protocol versioning and release matrix. |
| [SDK_COMPATIBILITY.md](../SDK_COMPATIBILITY.md) | Soroban SDK compatibility guidelines and supported versions. |

## 📚 Security, Role Separation & Invariants

| Document | Description |
| :--- | :--- |
| [SECURITY_NOTES.md](../SECURITY_NOTES.md) | Security model, threat vectors, and audit mitigation notes. |
| [ROLE_SEPARATION_SECURITY_NOTES.md](../ROLE_SEPARATION_SECURITY_NOTES.md) | Access control, administrative separation, and privilege boundaries. |
| [DELEGATE_PERMISSION_MATRIX.md](../DELEGATE_PERMISSION_MATRIX.md) | Granular delegation rights, operator roles, and execution limits. |
| [REENTRANCY_GUARD_ANALYSIS.md](../REENTRANCY_GUARD_ANALYSIS.md) | Cross-contract reentrancy risks and invocation safety invariants. |
| [INVARIANT_ENFORCEMENT.md](../INVARIANT_ENFORCEMENT.md) | Mathematical invariants and state-transition constraints. |
| [INVARIANT_ENTRYPOINTS.md](../INVARIANT_ENTRYPOINTS.md) | Public entrypoints enforcing runtime invariant validation. |
| [RISK_FLAGS.md](../RISK_FLAGS.md) | Risk flagging engine, anomaly heuristics, and circuit breakers. |
| [STRICT_MODE.md](../STRICT_MODE.md) | Strict validation rules for protocol compliance and execution safety. |

## 📚 Errors & Recovery

| Document | Description |
| :--- | :--- |
| [ERRORS.md](../ERRORS.md) | Canonical protocol error codes, panic conditions, and error classification. |
| [ERROR_REGISTRY_IMPLEMENTATION.md](../ERROR_REGISTRY_IMPLEMENTATION.md) | Implementation specification for the centralized error registry. |
| [CONFIGURATION_SNAPSHOTS_ROLLBACK.md](../CONFIGURATION_SNAPSHOTS_ROLLBACK.md) | Configuration rollback mechanism and state snapshots for disaster recovery. |

## 📚 Events & Telemetry

| Document | Description |
| :--- | :--- |
| [EVENT_SCHEMA.md](../EVENT_SCHEMA.md) | Schema definitions for contract events emitted across lifecycle transitions. |
| [EVENT_VERSIONING.md](../EVENT_VERSIONING.md) | Event schema evolution and backward compatibility standards. |

## 📚 Fees & Treasury

| Document | Description |
| :--- | :--- |
| [FEE_MECHANISM.md](../FEE_MECHANISM.md) | Fee structures, calculation models, and deduction policies. |
| [INSURANCE_POOL_HOOKS.md](../INSURANCE_POOL_HOOKS.md) | Integration hooks for insurance pool reserves and claim funding. |
| [MULTI_REGION_TREASURY_DISTRIBUTION.md](../MULTI_REGION_TREASURY_DISTRIBUTION.md) | Treasury splits, multi-region routing, and payout settlements. |

## 📚 Storage & State Management

| Document | Description |
| :--- | :--- |
| [STORAGE_KEY_AUDIT.md](../STORAGE_KEY_AUDIT.md) | Comprehensive audit of Soroban instance and persistent storage keys. |
| [STORAGE_KEY_IMPLEMENTATION_COMPLETE.md](../STORAGE_KEY_IMPLEMENTATION_COMPLETE.md) | Finalized storage layout, TTL boundaries, and key packing. |
| [PROGRAM_ESCROW_PAGINATION_SECURITY.md](../PROGRAM_ESCROW_PAGINATION_SECURITY.md) | Pagination safety and unbounded scan protections in escrow storage. |
| [NONCE_STRATEGY.md](../NONCE_STRATEGY.md) | Anti-replay nonce generation, invalidation, and session tracking. |
| [ASSET_ID_STRATEGY.md](../ASSET_ID_STRATEGY.md) | Cross-chain and Soroban asset identifier representation standards. |

## 📚 Queries & Payloads

| Document | Description |
| :--- | :--- |
| [QUERY_DOCUMENTATION.md](../QUERY_DOCUMENTATION.md) | Detailed read-only query interfaces, filter semantics, and RPC schemas. |
| [QUERY_QUICK_REFERENCE.md](../QUERY_QUICK_REFERENCE.md) | Quick cheatsheet for client querying and contract inspectors. |
| [METADATA_SAMPLE_PAYLOADS.md](../METADATA_SAMPLE_PAYLOADS.md) | Sample payloads for metadata serialization and escrow attachments. |
| [NETWORK_CONFIGURATION_IMPLEMENTATION.md](../NETWORK_CONFIGURATION_IMPLEMENTATION.md) | Network profiles, RPC endpoints, and chain configuration details. |

## 📚 Testing & Compatibility

| Document | Description |
| :--- | :--- |
| [ADMIN_TESTS_SUMMARY.md](../ADMIN_TESTS_SUMMARY.md) | Summary of admin permission and role-transition unit/integration tests. |
| [COMMIT_REVEAL_INTEGRATION.md](../COMMIT_REVEAL_INTEGRATION.md) | Commit-reveal scheme testing and integration test vectors. |
| [Idempotency_Implementation_Summary.md](../Idempotency_Implementation_Summary.md) | Idempotent transaction submission and duplicate rejection test review. |
| [SERIALIZATION_COMPATIBILITY_TESTS.md](../SERIALIZATION_COMPATIBILITY_TESTS.md) | Cross-version XDR serialization and deserialization test suite. |

