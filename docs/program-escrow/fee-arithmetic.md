# Fee Arithmetic in Program Escrow

## Overview

The Program Escrow contract uses precise fee arithmetic to manage prize pools and fee collection. Ensuring no value is lost or created out of thin air is critical for security and financial integrity.

## Rounding Policy

The contract employs **floor (round-down)** rounding for all fee calculations. This ensures that the protocol never overcharges the user.

Any remainder from basis-point division (dust) remains with the payer as part of the net amount, adhering to the fundamental invariant:

```
fee + net_amount == gross_amount
```

## Implementation

Fee calculation is performed in `contracts/program-escrow/src/token_math.rs`:

```rust
pub fn calculate_fee(amount: i128, fee_rate: i128) -> i128 {
    amount
        .checked_mul(fee_rate)
        .and_then(|x| x.checked_div(BASIS_POINTS))
        .unwrap_or(0)
}
```

## Security Invariants

1. **Conservation of Value**: The sum of `fee` and `net_amount` must always equal the `gross_amount` input.
2. **No Over-charge**: The fee must never exceed the calculated share based on basis points.
3. **No Overflow**: Calculations must be safe against integer overflow, panicking if they occur.
4. **Dust Control**: For equal splits, any rounding remainder (dust) is accounted for in the `net_amount`, ensuring no token loss.
