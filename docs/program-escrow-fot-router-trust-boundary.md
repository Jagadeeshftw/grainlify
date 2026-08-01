# Program Escrow FoT Router Trust Boundary

## Overview

The fee-on-transfer (FoT) router is an **external, caller-configured contract** that
`program-escrow` queries on every payout to translate an intended *net* recipient
amount into the gross transfer amount required after any fee-on-transfer
deduction. Because that translation is performed by code outside the escrow
program, it is a trust boundary that must be explicitly bounded.

A compromised, buggy, or malicious router can return an arbitrarily large gross
amount. Without an upper-bound check, the escrow contract would debit that
amount from `remaining_balance` and transfer it out, draining the program for
far more than the intended payout plus any plausible FoT fee.

## Upper-Bound Sanity Check

`apply_fot_router` enforces the invariant:

```text
gross_quote <= net_amount * max_fot_multiplier_bps / 10_000
```

- `net_amount` is the amount the recipient is intended to receive.
- `max_fot_multiplier_bps` is an admin-configured ceiling expressed in basis
  points over `10_000` (e.g. `15_000` allows a gross quote up to `1.5x` net).
- `BASIS_POINTS` is `10_000` (`100%`).

If a router quote exceeds the bound, the payout aborts with the typed error
`ContractError::FotRouterQuoteExceeded` and **no tokens are transferred**.

## Configuring the Bound

The bound is set alongside the existing slippage parameter when the admin
configures the router:

```rust
contract.set_fot_router(
    &router_address,
    &100,      // slippage_bps (1%)
    &15_000,   // max_fot_multiplier_bps (1.5x net)
);
```

- `slippage_bps` is validated to be `<= 500` (5%).
- `max_fot_multiplier_bps` must be between `10_000` (`1x`) and `100_000`
  (`10x`).

The default value used by the contract tests is `15_000` (`1.5x`). A more
conservative value is recommended for production deployments.

## Why a Configurable Cap?

- **Token-specific FoT rates vary**: Some tokens have modest fees (1-10%), while
  exotic tokens may require higher multipliers.
- **Admin control over trade-offs**: A lower cap limits blast radius; a higher
  cap supports tokens with larger fees.
- **Upper ceiling on the ceiling**: The `100_000` (`10x`) hard cap prevents a
  compromised admin from setting a multiplier so high that the sanity check is
  rendered meaningless.

## Interaction with Slippage

The sanity check is applied to the **raw gross quote** returned by the router,
**before** the configured slippage is added. This ensures that a malicious router
cannot first pass the bound check and then inflate the final transfer amount via
an excessive slippage buffer.

The final transfer amount sent to the token contract is therefore:

```text
gross = router_quote(token, net_amount)
assert gross <= net_amount * max_fot_multiplier_bps / 10_000
adjusted = gross * (10_000 + slippage_bps) / 10_000
```

## Security Assumptions

- The program admin is trusted to set a sensible `max_fot_multiplier_bps`.
- The router contract is still trusted to provide an *accurate* gross amount,
  but its ability to drain the program is now capped.
- The escrow contract does not call the router unless a `FotRouter` config has
  been explicitly set.

## Test Coverage

The `test_fot_routing.rs` suite includes:

- `test_malicious_router_inflated_quote_rejected`: a router returning `100x`
  the intended net is rejected before any token transfer occurs.
- `test_fot_router_in_program_data`: the configured `max_fot_multiplier_bps` is
  persisted alongside `router_contract` and `slippage_bps`.

Existing tests for normal FoT routing, slippage, batch payouts, protocol fees,
and balance checks continue to exercise the bounded path under safe routers.
