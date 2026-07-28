# Program Escrow Status Guards

`single_payout` and `batch_payout` call `require_active_program` immediately after loading `ProgramData`. Draft programs fail with `ERR_PROGRAM_NOT_ACTIVE` (`107`) before authorization, balance checks, fee math, or token transfers can process.

Security notes:

- Draft programs must call `publish_program()` before payouts.
- Legacy programs already stored as `Active` continue through the same payout path.
- The guard is read-only and does not change storage layout.
