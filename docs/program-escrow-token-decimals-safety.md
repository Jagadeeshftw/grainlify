# Program Escrow token-decimals safety

Program Escrow always transfers and stores token amounts as raw `i128` units.
Decimals are display metadata: indexers and user interfaces divide a raw amount
by `10^decimals` to produce a human-readable amount.

## Configuration policy

Use `add_allowed_token_with_decimals(token, decimals)` to allowlist a token and
set its display scale. The first configured value is immutable. Repeating the
call with that same value is idempotent; a different value is rejected with
`Token decimals are immutable`.

This is deliberately stricter than permitting an admin correction after a
payout. Changing a scale can make a historical raw payout appear to be ten,
one hundred, or more times larger or smaller to downstream consumers. There is
no acknowledgement flag that can make that reinterpretation safe. Migrate to a
new token address (and configure its decimals) instead.

Legacy entries added with `add_allowed_token` have no configured decimal value;
`get_token_decimals` returns `None` for them. Configure them once with
`add_allowed_token_with_decimals` before exposing their amounts to consumers.

## Live token sanity check

When available, the contract calls the token's standard `decimals()` view at
configuration time. If the reported value differs from the supplied value, it
emits `TkDecMis` (`TokenDecimalsMismatchEvent`) containing both values. This is
a warning rather than a rejection: custom tokens may intentionally use an
application-specific accounting scale, and non-standard token contracts may not
provide the view at all.

The contract does not read decimals live for every payout or display. A live
view can be unavailable or change after a token upgrade, whereas completed
payouts need a stable historical interpretation. The configured value is
therefore an explicit, immutable admin attestation, with the live view used
only as a monitoring check.

## Operational checks

- Monitor `TkDecMis` events and investigate every mismatch before publishing
  amounts to users.
- Persist the configured decimals alongside every indexed payout.
- Treat a request to change configured decimals as a token migration, not a
  metadata correction.
- The regression suite covers both mismatch event emission and rejection of a
  reconfiguration after a successful payout.
