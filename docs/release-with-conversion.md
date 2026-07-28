# Stellar Path-Payment Auto-Conversion at Release Time

This document details the design, security assumptions, and usage of the atomic path-payment auto-conversion feature for foreign-currency payouts.

## Feature Overview

Bounty contributors may prefer to receive payouts in a different asset than the one used to fund the bounty. To facilitate this atomically and trustlessly, the contract implements a `release_with_conversion` entrypoint.

When triggered, the contract calls the configured Stellar AMM / DEX router, swaps the locked asset to the recipient's destination asset along a specified path, validates that the conversion rate satisfies the slippage threshold, and transfers the converted asset directly to the recipient wallet.

## Contract Entrypoints

### `release_with_conversion`
Triggers the release transition, swapping the locked source asset to the recipient's preferred currency atomically.

* **Signature**:
  ```rust
  pub fn release_with_conversion(
      env: Env,
      bounty_id: u64,
      contributor: Address,
      dest_asset: Address,
      path: Vec<Address>,
      max_slippage_bps: u32,
  ) -> Result<(), Error>
  ```
* **Parameters**:
  - `bounty_id`: The identifier of the escrow to release.
  - `contributor`: The contributor's wallet address to receive the converted asset.
  - `dest_asset`: The target asset address to convert to.
  - `path`: The list of intermediary token addresses representing the swap route (must begin with the source asset and end with `dest_asset`).
  - `max_slippage_bps`: Maximum allowed slippage in basis points ($1 \text{ bp} = 0.01\%$).
* **Access Control**: Admin only.
* **Transitions**: Escrow status goes from `Locked` to `Released`.

### `set_router`
Configures the DEX/AMM router address utilized for swapping.

* **Signature**:
  ```rust
  pub fn set_router(env: Env, router: Address) -> Result<(), Error>
  ```
* **Access Control**: Admin only.

### `get_router`
Returns the currently configured router address, if any.

* **Signature**:
  ```rust
  pub fn get_router(env: Env) -> Option<Address>
  ```

## Slippage Validation

The contract protects contributors from high price volatility and front-running by validating the swap rate:
1. It queries the configured router for a price quote (`get_amounts_out`) on the given `path` for the net payout amount.
2. It calculates the minimum expected output using `max_slippage_bps`:
   $$\text{min\_amount\_out} = \text{expected\_out} \times \frac{10000 - \text{max\_slippage\_bps}}{10000}$$
3. During the swap execution, if the actual received amount is less than `min_amount_out`, the transaction reverts with `Error::SlippageExceeded`.

## Event Schema

Emits a `ReleasedWithConversion` event upon successful conversion and transfer:

```rust
#[contracttype]
pub struct ReleasedWithConversion {
    pub escrow_id: u64,
    pub src_asset: Address,
    pub dest_asset: Address,
    pub rate: i128, // Scaled by 1,000,000 (PPM)
}
```

## Security Assumptions and Safeguards

1. **Reentrancy Guard**: The entrypoint acquires the reentrancy lock at the beginning of logic and releases it at the end to prevent callbacks during external token operations.
2. **CEI Pattern (Checks-Effects-Interactions)**: Escrow state and remaining balance are updated in storage *before* calling external approve, swap, or transfer operations.
3. **Approval Safety**: Token approvals given to the router contract are strictly limited to the `net_payout` amount and configured with a short expiration deadline.
