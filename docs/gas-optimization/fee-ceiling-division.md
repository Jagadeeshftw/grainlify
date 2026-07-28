# Fee Ceiling Division

FeeConfig percentage fees use ceiling division:

```text
fee = ceil(amount * rate_bps / 10000)
net = amount - fee
```

This prevents fractional fee dust from being silently lost for odd amounts such as `1001` at `100` bps, where the fee is `11` and the net payout is `990`.

Security notes:

- `fee + net == amount` for every successful payout.
- Checked arithmetic rejects overflow instead of wrapping.
- Fees remain capped by the payout amount through `combined_fee_amount`.
