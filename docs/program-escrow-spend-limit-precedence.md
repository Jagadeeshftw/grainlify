# Program Escrow Spend Limit Precedence

This document clarifies the precedence between the contract-wide global rate limits (`RateLimitConfig`) and the per-program spend thresholds.

## Precedence Resolution

1. **Per-Program Thresholds:** Programs can define a maximum spend threshold via `set_program_spend_threshold`. This is strictly enforced during payouts (both single and batch) via `enforce_spend_threshold`. If a payout exceeds this limit, it is rejected with a `SpendLimitExceeded` error.
2. **Global RateLimitConfig:** The global `RateLimitConfig` (set via `update_rate_limit_config`) tracks operational parameters like `max_operations` and `window_size`. However, this config is currently **not** strictly enforced as a blocking limit on payouts (e.g., batch sizes or cumulative payout volumes are not bounded by `max_operations` during the `batch_payout` execution). 

**Conclusion:** The per-program threshold is the effective enforced limit for payout amounts. Since the global `RateLimitConfig` does not enforce hard caps on payout execution (either by batch size or amount), the per-program settings implicitly act as the most restrictive (and only) enforced limits. The per-program spend threshold always overrides the global rate limit.
