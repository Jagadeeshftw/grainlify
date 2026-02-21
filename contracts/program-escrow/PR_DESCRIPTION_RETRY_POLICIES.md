# Add Tests for Retry Policies (Aggressive vs Conservative)

Closes #388

## Summary

This PR adds comprehensive tests for comparing aggressive and conservative retry policies in the error recovery module. The tests validate delay sequences, max retry behavior, and policy characteristics.

## Policy Comparison Table

| Property           | Default     | Aggressive | Conservative |
|--------------------|-------------|------------|--------------|
| `max_attempts`     | 3           | 5          | 2            |
| `initial_delay_ms` | 100         | 50         | 200          |
| `max_delay_ms`     | 5000        | 3000       | 10000        |
| `backoff_multiplier` | 2         | 2          | 3            |
| `jitter_percent`   | 20          | 15         | 25           |

## Use Case Recommendations

- **Aggressive**: Best for critical operations where quick recovery is needed
  - More retry attempts (5)
  - Shorter initial delay (50ms)
  - Lower max delay cap (3000ms)
  - Lower jitter (15%) for more predictable timing

- **Conservative**: Best for rate-limit-aware scenarios
  - Fewer retry attempts (2) to avoid overwhelming services
  - Longer initial delay (200ms) to allow recovery
  - Higher max delay cap (10000ms)
  - Higher jitter (25%) to prevent thundering herd

- **Default**: Middle ground for general use cases

## Tests Added

### Policy Comparison Tests
- `test_aggressive_has_more_attempts_than_conservative` - Verifies aggressive has 5 attempts vs 2 for conservative
- `test_aggressive_has_shorter_initial_delay_than_conservative` - Verifies 50ms vs 200ms initial delay
- `test_conservative_has_higher_max_delay_cap` - Verifies 10000ms vs 3000ms max delay
- `test_conservative_has_higher_backoff_multiplier` - Verifies 3x vs 2x multiplier
- `test_policy_comparison_table` - Comprehensive comparison of all three policies

### Delay Sequence Tests
- `test_delay_sequence_aggressive_vs_conservative` - Compares delay progression between policies
- `test_aggressive_reaches_max_delay_faster` - Verifies aggressive hits cap sooner
- `test_conservative_allows_longer_delays` - Verifies conservative allows longer delays

### Max Retry Behavior Tests
- `test_aggressive_max_retries_behavior` - Validates 5 attempts with bounded total delay
- `test_conservative_max_retries_behavior` - Validates 2 attempts with significant delays

### Scenario-Based Tests
- `test_aggressive_policy_quick_recovery_scenario` - Validates total delay under 2 seconds
- `test_conservative_policy_rate_limit_aware_scenario` - Validates rate-limit-friendly behavior
- `test_all_policies_respect_delay_bounds` - Ensures all policies stay within bounds
- `test_default_is_middle_ground` - Confirms default is between aggressive and conservative

### Jitter Tests
- `test_jitter_application_differs_between_policies` - Verifies jitter percentages differ

## Test Results

All 59 tests pass:
- 35 error recovery tests (including 20 new retry policy tests)
- 24 existing contract tests

## Files Modified

- `contracts/program-escrow/src/lib.rs` - Added module declarations for error_recovery and retry_executor
- `contracts/program-escrow/src/error_recovery_tests.rs` - Added 20 new tests for retry policy comparison
