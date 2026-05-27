# Circuit Breaker Documentation

## Overview

The Program Escrow contract implements a three-state circuit breaker pattern to protect against cascading failures during token transfers and external calls. This document describes the circuit breaker functionality, including the new automatic timeout handling implemented in Issue #1254.

## Circuit States

The circuit breaker operates in three distinct states:

```
[Closed] ──(failure_threshold failures)──> [Open]
    ^                                         │
    │                                         │
    │ (success_threshold successes)           │ (recovery_window timeout)
    │                                         │
    │                                         ▼
[HalfOpen] <────────────────────────────────────
         ^
         │ (admin reset)
         │
    [Open]
```

### Closed State
- **Normal operation**: All requests pass through normally
- **Failure handling**: Consecutive failures are tracked
- **Transition**: Moves to Open when `failure_threshold` consecutive failures occur

### Open State  
- **Protective mode**: All requests are immediately rejected
- **Purpose**: Prevents cascading failures by failing fast
- **Automatic transition**: After `recovery_window` seconds, automatically transitions to HalfOpen
- **Manual transition**: Admin can manually reset to HalfOpen state

### HalfOpen State
- **Trial period**: Allows limited requests to test system recovery
- **Success handling**: Successful operations are counted
- **Automatic closure**: After `success_threshold` successes, automatically transitions to Closed
- **Failure handling**: Any failure immediately reopens the circuit (returns to Open)

## Configuration Parameters

### Core Thresholds
- **`failure_threshold`**: Number of consecutive failures required to open the circuit (default: 3)
- **`success_threshold`**: Number of consecutive successes in HalfOpen required to close the circuit (default: 1)
- **`max_error_log`**: Maximum number of error entries to retain in the log (default: 10)

### Timeout Handling (New in Issue #1254)
- **`recovery_window`**: Time in seconds after which an Open circuit automatically transitions to HalfOpen (default: 300 seconds / 5 minutes)

## Automatic Timeout Behavior

### Open → HalfOpen Transition
When the circuit is in the Open state:
1. The `opened_at` timestamp is recorded when the circuit opens
2. On each operation attempt, the system checks if `current_time >= opened_at + recovery_window`
3. If the recovery window has elapsed, the circuit automatically transitions to HalfOpen
4. A `cb_timeout` event is emitted with reason `auto_half`

### HalfOpen → Closed Transition  
When the circuit is in the HalfOpen state:
1. Each successful operation increments the success counter
2. When `success_count >= success_threshold`, the circuit automatically closes
3. The circuit returns to normal Closed operation
4. All counters are reset

### Failure in HalfOpen
If an operation fails while in HalfOpen state:
1. The circuit immediately reopens (transitions to Open)
2. A new `opened_at` timestamp is recorded
3. The recovery window timer restarts
4. Failure and success counters are reset

## API Reference

### Configuration Functions

#### `configure_circuit_breaker`
```rust
pub fn configure_circuit_breaker(
    env: Env,
    caller: Address,
    failure_threshold: u32,
    success_threshold: u32, 
    max_error_log: u32,
    recovery_window: u64,
)
```

Configures the circuit breaker parameters. Only the circuit admin can call this function.

**Parameters:**
- `caller`: Address of the caller (must be circuit admin)
- `failure_threshold`: Number of failures to trigger Open state
- `success_threshold`: Number of successes to close from HalfOpen
- `max_error_log`: Maximum error log entries to retain
- `recovery_window`: Automatic recovery timeout in seconds

### Status Functions

#### `get_circuit_breaker_status`
```rust
pub fn get_circuit_breaker_status(env: Env) -> CircuitBreakerStatus
```

Returns the current circuit breaker status including:
- Current state (Closed/Open/HalfOpen)
- Failure and success counts
- Timestamps (last failure, opened at)
- Configuration values (thresholds, recovery window)

### Administrative Functions

#### `reset_circuit_breaker`
```rust
pub fn reset_circuit_breaker(env: Env, admin: Address)
```

Manually resets the circuit breaker state:
- Open → HalfOpen: Initiates trial period
- HalfOpen/Closed → Closed: Forces circuit closed

#### `set_circuit_admin`
```rust
pub fn set_circuit_admin(env: Env, new_admin: Address, caller: Option<Address>)
```

Sets or updates the circuit breaker admin address.

## Events

The circuit breaker emits the following events for monitoring and auditing:

### State Transition Events
- **`cb_open`**: Circuit opened (manual or automatic)
- **`cb_half`**: Circuit moved to HalfOpen (manual reset)
- **`cb_close`**: Circuit closed (automatic after successes)
- **`cb_timeout`**: Automatic timeout transition (new in #1254)

### Operational Events  
- **`cb_reject`**: Operation rejected due to open circuit
- **`cb_fail`**: Operation failed and recorded
- **`cb_cfg`**: Configuration changed
- **`cb_reset`**: Manual admin reset performed
- **`cb_adm`**: Admin address changed

### Timeout Event Details
The new `cb_timeout` event includes:
- **Topic 0**: `"circuit"`
- **Topic 1**: `"cb_timeout"`  
- **Data 0**: Transition reason (`"auto_half"`)
- **Data 1**: Timestamp of transition

## Security Considerations

### Authorization
- Only the designated circuit admin can modify configuration or manually reset
- Admin address can only be changed by the current admin
- All administrative operations require proper authentication

### Timestamp Security
- Uses ledger timestamps for reliable timeout calculations
- `opened_at` timestamp is always set when transitioning to Open
- Timeout calculations are resistant to timestamp manipulation

### Invariant Verification
The system maintains several critical invariants:
- Open state must have non-zero `opened_at` timestamp
- Closed state must have zero `opened_at` timestamp  
- HalfOpen state success count must be less than threshold
- Failure count in Closed state must be less than failure threshold

## Usage Examples

### Basic Configuration
```rust
// Configure with 5-minute recovery window
client.configure_circuit_breaker(
    &admin,
    &3u32,    // failure_threshold
    &1u32,    // success_threshold  
    &10u32,   // max_error_log
    &300u64   // recovery_window (5 minutes)
);
```

### Monitoring Circuit State
```rust
let status = client.get_circuit_breaker_status();
match status.state {
    CircuitState::Closed => {
        // Normal operation
    },
    CircuitState::Open => {
        let time_until_halfopen = status.opened_at + status.recovery_window - current_time;
        // Circuit will auto-recover in time_until_halfopen seconds
    },
    CircuitState::HalfOpen => {
        // Trial period - monitor for success/failure
    }
}
```

### Emergency Manual Reset
```rust
// Admin can manually transition Open → HalfOpen
client.reset_circuit_breaker(&admin);

// Or force close from any state  
client.reset_circuit_breaker(&admin); // Open → HalfOpen
client.reset_circuit_breaker(&admin); // HalfOpen → Closed
```

## Best Practices

### Recovery Window Configuration
- **Short windows (1-5 minutes)**: For transient network issues
- **Medium windows (5-30 minutes)**: For service degradation scenarios  
- **Long windows (30+ minutes)**: For planned maintenance or major outages
- **Zero window**: Immediate recovery attempts (use with caution)

### Threshold Tuning
- **Low failure threshold (1-3)**: Sensitive to failures, fast protection
- **High failure threshold (5-10)**: Tolerant of occasional failures
- **Success threshold**: Usually 1-3, higher values provide more confidence

### Monitoring and Alerting
- Monitor `cb_open` events for circuit trips
- Alert on `cb_timeout` events for automatic recoveries
- Track failure patterns in error logs
- Monitor recovery success rates in HalfOpen state

### Testing Recommendations
- Test timeout behavior with various recovery windows
- Verify manual reset functionality
- Test failure scenarios in HalfOpen state
- Validate event emission for monitoring systems

## Migration Notes

### Upgrading from Previous Versions
The new timeout functionality is backward compatible:
- Existing circuits will use the default recovery window (300 seconds)
- Manual reset behavior remains unchanged
- All existing events continue to be emitted

### Configuration Migration
When upgrading, update configuration calls to include the recovery window:
```rust
// Old format (will cause compilation error)
client.configure_circuit_breaker(&admin, &3u32, &1u32, &10u32);

// New format (required)
client.configure_circuit_breaker(&admin, &3u32, &1u32, &10u32, &300u64);
```

## Troubleshooting

### Common Issues

#### Circuit Stuck in Open State
- **Cause**: Very long recovery window or system clock issues
- **Solution**: Check recovery window configuration, use manual reset if needed

#### Frequent Open/HalfOpen Cycling  
- **Cause**: Failure threshold too low or underlying issues not resolved
- **Solution**: Increase failure threshold or fix underlying problems

#### Timeout Not Working
- **Cause**: Operations not being attempted, or recovery window too long
- **Solution**: Verify operations are calling `check_and_allow()`, check recovery window

### Diagnostic Commands
```rust
// Check current status
let status = client.get_circuit_breaker_status();

// Verify invariants (internal function)
let valid = error_recovery::verify_circuit_invariants(&env);

// Check error log
let errors = error_recovery::get_error_log(&env);
```