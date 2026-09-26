# Fee Model

This document outlines how fees are calculated, who pays them, bounds, and rounding behavior in the grainlify escrow contracts (`program-escrow` and `bounty-escrow`).

## Fee Computation

Fees can be configured as a rate in basis points (1 bp = 0.01%) or as a fixed token amount. There are two primary operations where fees may be applied:

1. **Lock Fees (`lock_fee_rate`, `lock_fixed_fee`)**: Evaluated when funds are deposited (locked) into the escrow.
2. **Payout/Release Fees (`payout_fee_rate`, `release_fee_rate`, `payout_fixed_fee`, `release_fixed_fee`)**: Evaluated when funds are distributed to recipients.

The total fee for an operation is computed as:
```text
total_fee = (amount * fee_rate_bps / 10000) + fixed_fee
```
*(Note: fees are only applied if `fee_enabled` is true, and are subject to specific logic defined in `combined_fee_amount`).*

## Who Bears the Fee

* **Lock Fees**: Borne by the depositor (or the program funds). The lock fee is deducted from the gross amount deposited, meaning the escrow is credited with the net amount (`gross_amount - fee`).
* **Payout Fees**: Borne by the recipient. The payout fee is deducted from the gross payout amount, meaning the recipient receives the net amount (`gross_payout - fee`).

In both cases, the payer bears the fee (depositor pays lock fees from their total, recipient pays payout fees from their payout).

## Fee Bounds

* **Maximum Rate**: The maximum allowed fee rate (`MAX_FEE_RATE`) is strictly enforced as 5,000 basis points (50%). An administrative attempt to set a rate higher than this will fail.
* **Never Exceed Amount**: The calculated fee is guaranteed never to exceed the principal amount from which it is derived. An invariant `assert!(fee <= amount)` is strictly enforced in the math helpers.
* **Fee Config Changes**: Updating the fee configuration is bounded by `MAX_FEE_RATE`. When changed by an admin, the `FeeConfigUpdatedEvent` or `FeeConfigUpdated` event is emitted containing the updated parameters.

## Rounding Direction

* All fee rate calculations use **floor (round-down)** rounding.
* For example, if the fee calculation results in a fractional token unit, the fraction is discarded, and the fee collected is smaller.
* This means the protocol never overcharges — any remainder stays with the payer (depositor or recipient) rather than being collected as a fee. The invariant `fee + net == gross` always holds perfectly.
